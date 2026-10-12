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
says it failed (a BOOL of 0, a null handle or pointer, INVALID_HANDLE_VALUE): a function that succeeds leaves the last
error as it was, which belongs to the call before. A function the metadata does not describe is compared on its
return only. A pair w2m calls and Windows never does is a divergence of its own.

Usage: uv run python scripts/call_compare.py <w2m calls.tsv> <game-run-dir> [--signatures specs/signatures.json]
"""

from __future__ import annotations

import argparse
import hashlib
import json
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
    field: str                  # "a0".."a7", "ret", "error", or "called" (w2m calls it, Windows never does)
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


def failed(kind: str, value: int, size: int | None) -> bool:
    if kind == "handle":
        return value in (0, INVALID_HANDLE_VALUE)
    if kind == "pointer":
        return value == 0
    if kind == "value" and size == 4:
        return value & 0xFFFFFFFF == 0          # BOOL FALSE; a count of 0 for the functions that return one
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


def thread_map(windows: list[Call], w2m: list[Call]) -> dict[int, int]:
    """Windows thread id -> w2m thread id, by the order each first makes a traced call: the game starts its threads
    in one order on both, and the first comparison showed each pair's calls the same in shape."""
    def order(calls: list[Call]) -> list[int]:
        seen: list[int] = []
        for c in sorted(calls):
            if c[1] not in seen:
                seen.append(c[1])
        return seen
    return dict(zip(order(windows), order(w2m)))


def compare(windows: list[Call], w2m: list[Call], signatures: dict[str, Any],
            excluded_threads: frozenset[int] = frozenset()) -> tuple[list[Divergence], int]:
    """Every divergence, in the order of the Windows calls they concern, and the count of calls compared. Each
    (label, function)'s calls are compared thread by thread, a Windows thread against the w2m thread the map pairs it
    with, in order, as far as both go: across threads the order is the scheduler's. `excluded_threads` are reference
    threads the game does not start (the capture's own)."""
    windows = [c for c in windows if c[1] not in excluded_threads]
    threads = thread_map(windows, w2m)

    def by_pair(calls: list[Call]) -> dict[tuple[str, str], dict[int, list[Call]]]:
        out: dict[tuple[str, str], dict[int, list[Call]]] = {}
        for c in sorted(calls):
            out.setdefault((c[2].lower(), c[4]), {}).setdefault(c[1], []).append(c)
        return out

    w, m = by_pair(windows), by_pair(w2m)
    found: list[Divergence] = []
    pairs: list[tuple[int, Call, Call]] = []
    for pair, ours in m.items():
        theirs = w.get(pair)
        if theirs is None:
            first = min(c for calls in ours.values() for c in calls)
            found.append(Divergence(first[2], pair[1], 0, "called", None, first[6], None))
            continue
        for tid, calls in theirs.items():
            mine = ours.get(threads.get(tid, -1), [])
            pairs += [(k, a, b) for k, (a, b) in enumerate(zip(calls, mine))]
    maps: dict[str, tuple[dict[int, int], dict[int, int]]] = {}

    def same(space: str, x: int, y: int) -> bool:
        if space == "clock":
            return True
        if space == "thread":
            return threads.get(x & 0xFFFFFFFF) == y & 0xFFFFFFFF
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
            if sig["sets_last_error"] and failed(ret["kind"], a[6], ret.get("bytes")):
                fields.append(("error", "value", 4, a[7], b[7], False))
        for name, kind, size, x, y, argument in fields:
            space = ASSIGNED.get((fn, name))
            ok = same(space, x, y) if space is not None else judge(kind, size, x, y, argument)
            if not ok:
                found.append(Divergence(a[2], fn, k, name, x, y, a[0]))
    found.sort(key=lambda d: (d.windows_seq is None, d.windows_seq or 0, d.function))
    return found, len(pairs)


def excluded(contract: dict[str, Any], run: str) -> frozenset[int]:
    """The reference threads a contract leaves out for a run (its `excluded_threads`)."""
    return frozenset(int(t) for t in contract.get("excluded_threads", {}).get(run, {}))


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("calls", type=Path)
    ap.add_argument("run", type=Path)
    ap.add_argument("--signatures", type=Path, default=SIGNATURES)
    a = ap.parse_args(argv)
    signatures = json.loads(a.signatures.read_text())["functions"]
    found, compared = compare(read_tsv(a.run / "jit_capture.bin.trace.tsv"), read_tsv(a.calls), signatures,
                              excluded(json.loads(CONTRACT.read_text()), a.run.name))
    print(f"{compared} calls compared; {len(found)} divergences")
    for d in found:
        print(f"  seq {d.windows_seq}: {d.text()}")
    return 0 if not found else 1


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
