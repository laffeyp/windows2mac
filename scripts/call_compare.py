# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""call_compare.py — w2m's calls judged against the game's on Windows, call by call (Sprint 060).

The Windows run (tools/jit_capture/call_trace.h) and the frontier (tools/guest_runtime/call_trace.c) record the same
thing: for each function an image calls, its first 32 calls, each with its eight integer arguments, its return value
and the last error after it. Each (image, function) pair's calls are compared in order, as far as both go (w2m's run
stops earlier than the 180-second Windows run). Each value is judged by the relation its type in
specs/signatures.json allows, because pointers, handles and thread ids differ between any two runs:

  value    equal in the type's bytes (a 32-bit argument's upper half is whatever the caller left there)
  pointer  null on both or on neither; an argument below 0x10000 (an atom, a resource id) equal
  handle   null on both or on neither; a pseudo-handle or predefined key (top bit set) equal
  float    not compared: an XMM register, which neither trace records
  void     the return of a function with none: not compared

A last error is compared where the metadata marks the function as setting one (SupportsLastError) and the return
says it failed (a BOOL of FALSE, a null handle or pointer, INVALID_HANDLE_VALUE): a function that succeeds leaves the
last error as it was, which belongs to the call before. A function the metadata does not describe is compared on its
return only. A pair w2m calls and Windows never does is a divergence of its own.

Usage: uv run python scripts/call_compare.py <w2m calls.tsv> <game-run-dir> [--signatures specs/signatures.json]
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))
SIGNATURES = ROOT / "specs" / "signatures.json"
CONTRACT = ROOT / "contracts" / "windows.game.calls" / "0.1.0.json"
INVALID_HANDLE_VALUE = (1 << 64) - 1

Call = tuple[int, int, str, str, str, tuple[int, ...], int, int]   # seq, tid, image, dll, function, a0..a7, ret, error


@dataclass(frozen=True)
class Divergence:
    image: str
    function: str
    index: int                  # the call's place among this pair's calls (0: the first)
    field: str                  # "a0".."a7", "ret", "error", "called" (w2m calls it, Windows never does) or
                                # "missing" (Windows makes the call, the w2m thread paired with it does not)
    windows: int | None
    w2m: int | None
    windows_seq: int | None     # the Windows call's sequence number: the order the divergences are reported in

    def text(self) -> str:
        w = "none" if self.windows is None else f"0x{self.windows:x}"
        m = "none" if self.w2m is None else f"0x{self.w2m:x}"
        return f"{self.image}!{self.function} call {self.index} {self.field}: Windows {w}, w2m {m}"


def read_tsv(path: Path) -> list[Call]:
    return read_tsv_text(path.read_text())


def read_tsv_text(text: str) -> list[Call]:
    """A trace in call_trace.h's text (its line 286): seq, tid and the last error in decimal, the arguments and the
    return in hex without a prefix."""
    out = []
    for line in text.splitlines():
        if not line:
            continue
        f = line.split("\t")
        out.append((int(f[0]), int(f[1]), f[2], f[3], f[4], tuple(int(x, 16) for x in f[5:13]), int(f[13], 16),
                    int(f[14])))
    return out


def write_tsv(calls: list[Call]) -> str:
    return "".join(f"{s}\t{t}\t{i}\t{d}\t{fn}\t" + "\t".join(f"{a:x}" for a in args) + f"\t{r:x}\t{e}\n"
                   for s, t, i, d, fn, args, r, e in calls)


def rendering(fn: str, args: tuple[int, ...], signatures: dict[str, Any]) -> str:
    """GUEST_CALL_RETURNED's canonical argument text (vocabulary 0.4 note): 'Function(a1,a2,...)', integers in decimal,
    a non-null pointer or ordinary handle by its parameter's name in the metadata (its value differs between runs).
    A function the metadata does not describe shows its four register arguments in decimal."""
    sig = signatures.get(fn)
    if sig is None:
        return f"{fn}({','.join(str(a) for a in args[:4])})"
    out = []
    for p, a in zip(sig["params"], args):
        kind, size = p["kind"], p.get("bytes")
        if kind == "value":
            out.append(str(a & ((1 << (8 * size)) - 1) if size else a))
        elif kind == "pointer" and a >= 0x10000:
            out.append(p["name"])
        elif kind == "handle" and a and not a >> 63:
            out.append(p["name"])
        elif kind == "float":
            out.append("float")
        else:
            out.append(str(a))
    return f"{fn}({','.join(out)})"


def guest_trace(execution_id: str, calls: list[Call], threads: dict[int, tuple[int, int, bool]], dropped: int,
                signatures: dict[str, Any]) -> bytes:
    """The calls as a guest trace (vocabulary 0.7): GUEST_TRACE_OPENED, a GUEST_THREAD_STARTED for each thread
    (parent, start address, created suspended), a GUEST_CALL_RETURNED per call in sequence order, GUEST_TRACE_CLOSED
    with the calls the full trace area left out."""
    import msgspec

    from w2m import VOCAB_VERSION, events
    lines: list[bytes] = [msgspec.json.encode(events.GUEST_TRACE_OPENED(
        execution_id=execution_id, vocab_version=VOCAB_VERSION, profile_id="w2m-vm", trace_format_version="0.1"))]
    seq: dict[int, int] = {}
    for tid in sorted(threads.keys() | {c[1] for c in calls}):
        parent, start, suspended = threads.get(tid, (0, 0, False))
        lines.append(msgspec.json.encode(events.GUEST_THREAD_STARTED(
            thread_id=tid, thread_seq=0, hb_refs=(), parent_thread_id=parent, start_address=start,
            suspended=suspended)))
        seq[tid] = 1
    for c in sorted(calls):
        tid = c[1]
        args = hashlib.sha256(rendering(c[4], c[5], signatures).encode()).hexdigest()
        lines.append(msgspec.json.encode(events.GUEST_CALL_RETURNED(
            thread_id=tid, thread_seq=seq[tid], hb_refs=(), api=f"{c[3]}!{c[4]}", args_sha256="sha256:" + args,
            return_value=c[6] - (1 << 64) if c[6] >> 63 else c[6], last_error=c[7])))
        seq[tid] += 1
    lines.append(msgspec.json.encode(events.GUEST_TRACE_CLOSED(
        line_count=len(lines) + 1, thread_count=len(seq), dropped_count=dropped)))
    return b"".join(line + b"\n" for line in lines)


def failed(kind: str, type_name: str, value: int) -> bool:
    """Whether a return says the call failed, so its last error is the function's to set: a NULL or invalid handle, a
    NULL pointer, a BOOL of FALSE. Other integers carry no failure value the metadata names (WaitForSingleObjectEx's 0
    is WAIT_OBJECT_0; Sprint 064 found the earlier rule, any 4-byte 0, comparing a wait's stale last error)."""
    if kind == "handle":
        return value in (0, INVALID_HANDLE_VALUE)
    if kind == "pointer":
        return value == 0
    if type_name in ("BOOL", "BOOLEAN"):
        return value & 0xFFFFFFFF == 0
    return False


def judge(kind: str, size: int | None, windows: int, w2m: int, argument: bool) -> bool:
    if kind in ("float", "void"):
        return True
    if kind == "value":
        mask = (1 << (8 * size)) - 1 if size else (1 << 64) - 1
        return windows & mask == w2m & mask
    if kind == "pointer":
        if argument and (windows < 0x10000 or w2m < 0x10000):
            return windows == w2m
        return (windows == 0) == (w2m == 0)
    if kind == "handle":
        if windows >> 63 or w2m >> 63:
            return windows == w2m
        return (windows == 0) == (w2m == 0)
    raise ValueError(kind)


# Values the system assigns, which differ between any two runs, each with the relation that holds instead of equality
# (Sprint 060, from the first comparisons): an id must map one to one across the whole stream, a thread id through
# the thread map, and a clock reading is not compared.
ASSIGNED: dict[tuple[str, str], str] = {
    ("GetCurrentThreadId", "ret"): "thread", ("OpenThread", "a2"): "thread", ("GetThreadId", "ret"): "thread",
    ("GetCurrentProcessId", "ret"): "process", ("OpenProcess", "a2"): "process",
    ("TlsAlloc", "ret"): "tls", ("TlsGetValue", "a0"): "tls", ("TlsSetValue", "a0"): "tls", ("TlsFree", "a0"): "tls",
    ("FlsAlloc", "ret"): "fls", ("FlsGetValue", "a0"): "fls", ("FlsSetValue", "a0"): "fls", ("FlsFree", "a0"): "fls",
    ("RegisterClassExW", "ret"): "atom", ("RegisterClassW", "ret"): "atom", ("RegisterClassExA", "ret"): "atom",
    ("CreateWindowExW", "a1"): "atom", ("CreateWindowExA", "a1"): "atom", ("UnregisterClassW", "a0"): "atom",
    ("timeGetTime", "ret"): "clock", ("GetTickCount", "ret"): "clock", ("GetTickCount64", "ret"): "clock"}


# Integers that carry an address, judged as pointers (null on both or on neither). Each is an address by the
# function's documentation on Microsoft Learn: RtlAddGrowableFunctionTable's RangeBase and RangeEnd bound the code the
# table describes; EnumDisplayMonitors' dwData and SetWindowLongPtrW's value at GWLP_WNDPROC, GWLP_HINSTANCE and
# GWLP_USERDATA, and in the window's extra bytes (a non-negative index), are the caller's (a pointer in this game); a window message's lParam is a pointer for the messages
# in POINTER_LPARAM, each page's lParam ("A pointer to a ... structure").
ADDRESS_FIELDS = {("RtlAddGrowableFunctionTable", "a4"), ("RtlAddGrowableFunctionTable", "a5"),
                  ("EnumDisplayMonitors", "a3")}
WINDOW_LONG_ADDRESSES = {(-4) & 0xFFFFFFFF, (-6) & 0xFFFFFFFF, (-21) & 0xFFFFFFFF}
POINTER_LPARAM = {0x01, 0x0C, 0x0D, 0x1A, 0x24, 0x46, 0x47, 0x4A, 0x4E, 0x7C, 0x7D, 0x81, 0x83, 0x2E0}
# A window message's wParam that is a window handle (Microsoft Learn: WM_SETFOCUS "A handle to the window that has lost
# the keyboard focus", WM_KILLFOCUS "A handle to the window that receives the keyboard focus"; either can be NULL):
# judged, with each message function's window (a0), in one map from Windows' windows to w2m's (Sprint 065).
WINDOW_WPARAM = {0x07, 0x08}
MESSAGE_FUNCTIONS = {"DefWindowProcW", "DefWindowProcA", "SendMessageW", "SendMessageA", "CallWindowProcW",
                     "CallWindowProcA", "PostMessageW", "PostMessageA"}


def carries_address(fn: str, field: str, args: tuple[int, ...]) -> bool:
    if (fn, field) in ADDRESS_FIELDS:
        return True
    if fn in ("SetWindowLongPtrW", "SetWindowLongPtrA") and field == "a2":
        index = args[1] & 0xFFFFFFFF
        return index in WINDOW_LONG_ADDRESSES or index < 0x80000000      # a non-negative index: the window's extra bytes
    if fn in MESSAGE_FUNCTIONS and field == "a3":
        return args[1] & 0xFFFFFFFF in POINTER_LPARAM
    return False


MEM_RESERVE = 0x2000


def placed(calls: list[Call]) -> list[Call]:
    """`calls` without the reservations made at an address the caller chose, and the retry each failed one leads to
    (Sprint 065). mono reserves its next code block at the address past its last one (VirtualAlloc(hint, 0x10000,
    MEM_COMMIT | MEM_RESERVE, PAGE_EXECUTE_READWRITE)) and, when the system answers ERROR_INVALID_ADDRESS, asks again
    without the hint. Whether that address is free depends on the address space's layout, which on Windows ASLR
    draws per run (game run 38216081440: the same call failed at seq 2745 and succeeded at 2749), so neither the
    hinted call nor its retry is a value the game decides."""
    out: list[Call] = []
    retry: dict[int, int] = {}                     # thread -> the size a failed hinted reservation asked for
    for c in sorted(calls):
        if c[4] == "VirtualAlloc":
            if c[5][0] and c[5][2] & MEM_RESERVE:
                if c[6] == 0:
                    retry[c[1]] = c[5][1]
                continue
            if c[5][0] == 0 and retry.pop(c[1], None) == c[5][1]:
                continue
        out.append(c)
    return out


String = tuple[int, int, str]                       # seq, argument, text (<capture>.strings.tsv)


def read_strings(path: Path) -> list[String]:
    """A capture's string arguments: seq and argument in decimal, then the text."""
    out = []
    for line in path.read_text().splitlines() if path.is_file() else []:
        f = line.split("\t", 2)
        if len(f) == 3:
            out.append((int(f[0]), int(f[1]), f[2]))
    return out


def write_strings(strings: list[String]) -> str:
    return "".join(f"{s}\t{a}\t{t}\n" for s, a, t in strings)


def thread_names(calls: list[Call], strings: list[String]) -> dict[int, str]:
    """Each thread's first SetThreadDescription name, given to the thread that makes the call: the game's threads
    name themselves at their start (game run 38216081440; Sprint 064)."""
    by_seq = {c[0]: c for c in calls if c[4] == "SetThreadDescription"}
    names: dict[int, str] = {}
    for seq, arg, text in sorted(strings):
        c = by_seq.get(seq)
        if c is not None and arg == 1:
            names.setdefault(c[1], text)
    return names


POOLED = re.compile(r"^(.+) \d+$")


def pools(names: dict[int, str]) -> dict[int, str]:
    """Each thread of a job pool -> the pool's name (Sprint 065). The game names its workers "Job.Worker 0", "Job.Worker
    1" ... and "Background Job.Worker 0" ...: a name that ends in a number and is shared, but for the number, by two
    threads or more names a pool."""
    stems = {t: m.group(1) for t, n in names.items() if (m := POOLED.match(n))}
    shared = {s for s in stems.values() if list(stems.values()).count(s) > 1}
    return {t: s for t, s in stems.items() if s in shared}


def thread_map(windows: list[Call], w2m: list[Call], signatures: dict[str, Any] | None = None,
               names: tuple[dict[int, str], dict[int, str]] = ({}, {})) -> dict[int, int]:
    """Windows thread id -> w2m thread id. Neither trace links a thread to the CreateThread that made it, and a game's
    workers run one start routine in an order the scheduler picks; the capture keeps a function's first 32 calls over
    all threads, w2m 32 on each, so even the threads' first recorded calls differ. Each Windows thread, in the order it
    first appears, takes the unpaired w2m thread whose calls agree with its own most (per function, in order, each
    value by its relation; Sprint 061), the first to appear on a tie. A divergence every candidate thread shows
    survives any pairing; only interchangeable threads trade places. Threads the game names (SetThreadDescription,
    `names`) pair by name first, in order of appearance where a name repeats (Sprint 064: near-identical workers
    mispaired by agreement once the scheduler's order changed)."""
    def order(calls: list[Call]) -> list[int]:
        seen: list[int] = []
        for c in sorted(calls):
            if c[1] not in seen:
                seen.append(c[1])
        return seen

    def by_function(calls: list[Call]) -> dict[int, dict[tuple[str, str], list[Call]]]:
        out: dict[int, dict[tuple[str, str], list[Call]]] = {}
        for c in sorted(calls):
            out.setdefault(c[1], {}).setdefault((c[2].lower(), c[4]), []).append(c)
        return out

    sig = signatures or {}
    w, m = by_function(windows), by_function(w2m)

    def agreement(tw: int, tm: int) -> int:
        score = 0
        for key, theirs in w[tw].items():
            for a, b in zip(theirs, m[tm].get(key, [])):
                f = sig.get(key[1])
                params = f["params"][:8] if f else []
                values = [(p["kind"], p.get("bytes"), a[5][i], b[5][i]) for i, p in enumerate(params)
                          if (key[1], f"a{i}") not in ASSIGNED and not carries_address(key[1], f"a{i}", a[5])]
                if f and (key[1], "ret") not in ASSIGNED:
                    values.append((f["return"]["kind"], f["return"].get("bytes"), a[6], b[6]))
                score += 1 + sum(judge(k, n, x, y, False) for k, n, x, y in values)
        return score

    free = order(w2m)
    pairs: dict[int, int] = {}
    named_w, named_m = names
    for tw in order(windows):
        name = named_w.get(tw)
        match = next((tm for tm in free if name is not None and named_m.get(tm) == name), None)
        if match is not None:
            pairs[tw] = match
            free.remove(match)
    for tw in order(windows):
        if tw in pairs:
            continue
        if not free:
            break
        best = max(free, key=lambda tm: (agreement(tw, tm), -free.index(tm)))
        pairs[tw] = best
        free.remove(best)
    return pairs


def agreeing(windows: list[Call], w2m: list[Call], signatures: dict[str, Any]) -> int:
    """How many of two threads' calls to one function agree in every value their types let the comparison judge here
    (an assigned id is left to the stream's maps)."""
    count = 0
    for a, b in zip(windows, w2m):
        sig = signatures.get(a[4])
        if sig is None:
            count += a[6] == b[6]
            continue
        values = [(f"a{i}", p["kind"], p.get("bytes"), a[5][i], b[5][i], True) for i, p in enumerate(sig["params"][:8])]
        values.append(("ret", sig["return"]["kind"], sig["return"].get("bytes"), a[6], b[6], False))
        count += all(judge("pointer", None, x, y, arg) if carries_address(a[4], f, a[5]) else judge(k, n, x, y, arg)
                     for f, k, n, x, y, arg in values if (a[4], f) not in ASSIGNED)
    return count


def compare(windows: list[Call], w2m: list[Call], signatures: dict[str, Any],
            excluded_threads: frozenset[int] = frozenset(),
            strings: tuple[list[String], list[String]] = ([], []),
            undecided: frozenset[tuple[str, str]] = frozenset()) -> tuple[list[Divergence], int]:
    """Every divergence, in the order of the Windows calls they concern, and the count of calls compared. Each
    (label, function)'s calls are compared thread by thread, a Windows thread against the w2m thread the map pairs it
    with, in order; a Windows call past the end of its partner's, or to a function w2m never calls, is missing. Across
    threads the order is the scheduler's. `excluded_threads` are reference
    threads the game does not start (the capture's own); `undecided` are (function, field) values two Windows runs of
    the game disagree on, which are not compared (the contract's `undetermined`).

    A job pool's workers (`pools`) take jobs from one queue, and which free worker takes a job is the scheduler's
    choice (frontier 643a6283510c-1791810726153034000: the HeapSize walk Windows' Job.Worker 1 made, w2m's Job.Worker 0
    made, answer for answer). For each function, a pool's Windows workers, in the order of their first call to it,
    take the unpaired w2m worker of the pool whose calls to it agree with theirs most, the worker the thread map pairs
    it with on a tie."""
    windows = placed([c for c in windows if c[1] not in excluded_threads])
    w2m = placed(w2m)
    names = (thread_names(windows, strings[0]), thread_names(w2m, strings[1]))
    threads = thread_map(windows, w2m, signatures, names)
    pool_w, pool_m = pools(names[0]), pools(names[1])

    def by_pair(calls: list[Call]) -> dict[tuple[str, str], dict[int, list[Call]]]:
        out: dict[tuple[str, str], dict[int, list[Call]]] = {}
        for c in sorted(calls):
            out.setdefault((c[2].lower(), c[4]), {}).setdefault(c[1], []).append(c)
        return out

    w, m = by_pair(windows), by_pair(w2m)
    found: list[Divergence] = []
    pairs: list[tuple[int, Call, Call]] = []
    for pair, theirs in w.items():                 # a function Windows calls and w2m never does: each call missing
        if pair not in m:
            found += [Divergence(c[2], pair[1], k, "missing", c[6], None, c[0])
                      for calls in theirs.values() for k, c in enumerate(calls)]
    for pair, ours in m.items():
        theirs = w.get(pair)
        if theirs is None:
            first = min(c for calls in ours.values() for c in calls)
            found.append(Divergence(first[2], pair[1], 0, "called", None, first[6], None))
            continue
        chosen = dict(threads)
        for pool in sorted(set(pool_w.values())):
            free = [t for t in ours if pool_m.get(t) == pool]
            for tw in sorted((t for t in theirs if pool_w.get(t) == pool), key=lambda t: theirs[t][0][0]):
                if not free:
                    break
                best = max(free, key=lambda tm: (agreeing(theirs[tw], ours[tm], signatures), tm == threads.get(tw)))
                chosen[tw] = best
                free.remove(best)
        for tid, calls in theirs.items():
            mine = ours.get(chosen.get(tid, -1), [])
            pairs += [(k, a, b) for k, (a, b) in enumerate(zip(calls, mine))]
            # past the paired thread's last call: Windows makes the call, w2m does not (review of 2026-10-12, B1:
            # 21 calls of frontier 643a6283510c-1791830368398119000 went unjudged here)
            found += [Divergence(c[2], pair[1], k, "missing", c[6], None, c[0])
                      for k, c in enumerate(calls[len(mine):], len(mine))]
    maps: dict[str, tuple[dict[int, int], dict[int, int]]] = {}

    def same(space: str, x: int, y: int) -> bool:
        if space == "clock":
            return True
        if space == "thread":
            x, y = x & 0xFFFFFFFF, y & 0xFFFFFFFF
            if x in pool_w:
                return pool_m.get(y) == pool_w[x]
            return threads.get(x) == y
        forward, backward = maps.setdefault(space, ({}, {}))
        if space == "atom" and (x >= 0x10000 or y >= 0x10000):
            return (x == 0) == (y == 0)            # a class name, not an atom: a pointer
        x, y = x & 0xFFFFFFFF, y & 0xFFFFFFFF
        if forward.setdefault(x, y) != y or backward.setdefault(y, x) != x:
            return False
        return True

    for k, a, b in sorted(pairs, key=lambda p: p[1][0]):
        fn = a[4]
        sig = signatures.get(fn)
        if sig is None:
            fields = [("ret", "value", 8, a[6], b[6], False)]
        else:
            ret = sig["return"]
            fields = [(f"a{i}", p["kind"], p.get("bytes"), a[5][i], b[5][i], True)
                      for i, p in enumerate(sig["params"][:8])]
            fields.append(("ret", ret["kind"], ret.get("bytes"), a[6], b[6], False))
            if sig["sets_last_error"] and failed(ret["kind"], ret["type"], a[6]):
                fields.append(("error", "value", 4, a[7], b[7], False))
        for name, kind, size, x, y, argument in fields:
            if (fn, name) in undecided:
                continue
            space = ASSIGNED.get((fn, name))
            if fn in MESSAGE_FUNCTIONS and (name == "a0" or name == "a2" and a[5][1] & 0xFFFFFFFF in WINDOW_WPARAM):
                space = "window"
            if space is not None:
                ok = same(space, x, y)
            elif carries_address(fn, name, a[5]):
                ok = judge("pointer", None, x, y, argument)
            else:
                ok = judge(kind, size, x, y, argument)
            if not ok:
                found.append(Divergence(a[2], fn, k, name, x, y, a[0]))
    found.sort(key=lambda d: (d.windows_seq is None, d.windows_seq or 0, d.function))
    return found, len(pairs)


def excluded(contract: dict[str, Any], run: str) -> frozenset[int]:
    """The reference threads a contract leaves out for a run (its `excluded_threads`)."""
    return frozenset(int(t) for t in contract.get("excluded_threads", {}).get(run, {}))


def undetermined(contract: dict[str, Any]) -> frozenset[tuple[str, str]]:
    """The (function, field) values a contract does not compare: those two Windows runs of the game disagree on (its
    `undetermined`, keyed "Function field", each with its evidence)."""
    return frozenset((k.split(" ")[0], k.split(" ")[1]) for k in contract.get("undetermined", {}))


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("calls", type=Path)
    ap.add_argument("run", type=Path)
    ap.add_argument("--signatures", type=Path, default=SIGNATURES)
    a = ap.parse_args(argv)
    signatures = json.loads(a.signatures.read_text())["functions"]
    contract = json.loads(CONTRACT.read_text())
    found, compared = compare(read_tsv(a.run / "jit_capture.bin.trace.tsv"), read_tsv(a.calls), signatures,
                              excluded(contract, a.run.name),
                              (read_strings(a.run / "jit_capture.bin.strings.tsv"),
                               read_strings(a.calls.with_suffix(".strings.tsv"))), undetermined(contract))
    print(f"{compared} calls compared; {len(found)} divergences")
    for d in found:
        print(f"  seq {d.windows_seq}: {d.text()}")
    return 0 if not found else 1


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
