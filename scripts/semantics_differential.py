"""semantics_differential.py — the lifter judged by a real x86 CPU, one instruction at a time
(Sprint 037).

  --cases [--out probes/semantics/cases.bin]   write the case file probes/semantics/semantics.c
                                                runs on the windows-2025 runner
  --judge <run dir> [--record]                 compare the runner's result files with
                                                runtime.Runtime on the same cases

Cases: every encgen encoding (spec 0.7.0, seeded) that w2m decodes and lifts, except
control-flow rows, RIP-relative and absolute memory forms (their addresses are not placeable);
plus fixed control-flow encodings whose both paths are observable: Jcc rel8 +14 (all 16
conditions), JMP rel8 +14, CALL rel32 +14, RET to a planted address. Each runs from STATES
random states: 16 GPRs from width and sign boundaries and uniform bits; CF, PF, ZF, SF, OF
random, the other flags 0; random data and stack windows. A memory operand's base register is
set so the effective address falls in the data window (index 0); RSP lies in the stack window.

The reference is the CPU's result file; the model is runtime.Runtime stepping lift.py's
operations once from the same state at the same addresses. Compared: the path (RIP), all 16
GPRs, every flag the model defines after the step (a flag the model marks undefined is not
compared: the SDM leaves it undefined), and both windows.
"""

from __future__ import annotations

import argparse
import asyncio
import hashlib
import random
import struct
import sys
import time
from collections import Counter
from collections.abc import AsyncIterator, Iterator
from dataclasses import dataclass
from pathlib import Path
from typing import Any

import msgspec

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))

from w2m import VOCAB_VERSION, events  # noqa: E402
from w2m.encgen import SPEC, cases as encgen_cases  # noqa: E402
from w2m.kernel32 import M64  # noqa: E402
from w2m.lift import LiftError, Lifted, lift  # noqa: E402
from w2m.loader import Loaded, Memory  # noqa: E402
from w2m.runtime import Runtime  # noqa: E402
from w2m.x86dec import DecodeError, decode  # noqa: E402
from w2m.x86dec_tables import SPEC_VERSION  # noqa: E402

CODE_VA, DATA_VA, STACK_VA = 0x2000_0000, 0x3000_0000, 0x4000_0000
DATA_OFF, STACK_OFF, WINDOW, CODE_BYTES = 0x40, 0x87C0, 64, 32
STACK_SIZE = 0x10000
RET_TARGET = CODE_VA + 0x100
STATES = 4
CONTROL_STATES = 16
FLAG_BITS = {"C": 0, "P": 2, "Z": 6, "S": 7, "O": 11}
CASE_SIZE = 8 + CODE_BYTES + 8 * 16 + 8 + WINDOW + WINDOW
RESULT_SIZE = 8 * 16 + 8 + 8 + 8 + WINDOW + WINDOW
EDGE = (0, 1, 0x7F, 0x80, 0xFF, 0x7FFF, 0x8000, 0xFFFF, 0x7FFF_FFFF, 0x8000_0000, 0xFFFF_FFFF,
        0x7FFF_FFFF_FFFF_FFFF, 0x8000_0000_0000_0000, M64)


@dataclass
class Case:
    code: bytes
    regs: list[int]
    rflags: int
    data: bytes
    stack: bytes
    label: str                    # iclass, for grouping


def _value(rng: random.Random) -> int:
    r = rng.random()
    if r < 0.4:
        v = rng.choice(EDGE)
    elif r < 0.7:
        v = rng.getrandbits(rng.choice((8, 16, 32)))
    else:
        v = rng.getrandbits(64)
    return v ^ (rng.getrandbits(64) & 0xFFFF_FFFF_0000_0000 if rng.random() < 0.2 else 0)


def _state(rng: random.Random) -> tuple[list[int], int, bytes, bytes]:
    regs = [_value(rng) for _ in range(16)]
    regs[4] = STACK_VA + STACK_OFF + 0x20 + 8 * rng.randrange(4)      # push and pop stay inside the window
    rflags = 0x2 | sum(1 << b for b in FLAG_BITS.values() if rng.getrandbits(1))
    return regs, rflags, rng.randbytes(WINDOW), rng.randbytes(WINDOW)


def _place(ins: Lifted, regs: list[int], rng: random.Random) -> bool:
    """Point each memory operand into the data window. False for forms whose address is not
    placeable (RIP-relative and absolute)."""
    for op in ins.ops:
        if op[0] != "addr":
            continue
        _, _, base, index, scale, disp, abits = op
        if base is None:
            return False
        mask = M64 if abits == 64 else 0xFFFF_FFFF
        target = DATA_VA + DATA_OFF + 8 * rng.randrange(6)             # 16-byte operands stay inside
        if index is not None and index != base:
            regs[index] = 0
        if index == base:                                            # [r + r*scale]: solve r*(1+scale)
            target = next((DATA_VA + DATA_OFF + t for t in range(41)
                           if ((DATA_VA + DATA_OFF + t - disp) & mask) % (1 + scale) == 0), None)
            if target is None:
                return False
            regs[base] = ((target - disp) & mask) // (1 + scale)
            continue
        value = (target - disp) & mask
        if abits == 32:
            value |= rng.getrandbits(32) << 32                    # the CPU truncates; so must the model
        regs[base] = value
    return True


def _control_flow() -> Iterator[tuple[bytes, str]]:
    for cc in range(16):
        yield bytes([0x70 + cc, 0x0E]), f"Jcc_{cc:x}"
    yield bytes([0xEB, 0x0E]), "JMP"
    yield bytes([0xE8, 0x0E, 0, 0, 0]), "CALL_NEAR"
    yield bytes([0xC3]), "RET_NEAR"


def build_cases(seed: int = 2026_10_09) -> tuple[list[Case], Counter[str]]:
    rng = random.Random(seed)
    out: list[Case] = []
    skipped: Counter[str] = Counter()
    for c in encgen_cases():
        try:
            d = decode(c.raw)
            ins = lift(c.raw[:d.length] + b"\x90" * 15, CODE_VA)
        except (DecodeError, LiftError, IndexError) as e:
            skipped[f"not lifted: {type(e).__name__}"] += 1
            continue
        if any(op[0] == "br" for op in ins.ops):
            skipped["control flow (fixed encodings instead)"] += 1
            continue
        for _ in range(STATES):
            regs, rflags, data, stack = _state(rng)
            if not _place(ins, regs, rng):
                skipped["address not placeable (RIP-relative or absolute)"] += 1
                break
            out.append(Case(c.raw[:d.length], regs, rflags, data, stack, d.iclass))
    for code, label in _control_flow():
        for _ in range(CONTROL_STATES):
            regs, rflags, data, stack = _state(rng)
            if label == "RET_NEAR":
                off = regs[4] - (STACK_VA + STACK_OFF)
                stack = stack[:off] + RET_TARGET.to_bytes(8, "little") + stack[off + 8:]
            out.append(Case(code, regs, rflags, data, stack, label))
    return out, skipped


def write_cases(cases: list[Case], path: Path) -> None:
    with path.open("wb") as f:
        f.write(b"W2MS" + struct.pack("<II", 1, len(cases)))
        for c in cases:
            f.write(bytes([len(c.code), 0]) + bytes(6) + c.code.ljust(CODE_BYTES, b"\0"))
            f.write(struct.pack("<16QQ", *c.regs, c.rflags) + c.data + c.stack)


def read_cases(path: Path) -> list[Case]:
    raw = path.read_bytes()
    assert raw[:4] == b"W2MS" and struct.unpack_from("<I", raw, 4)[0] == 1
    count = struct.unpack_from("<I", raw, 8)[0]
    out = []
    for n in range(count):
        p = 12 + n * CASE_SIZE
        length = raw[p]
        code = raw[p + 8:p + 8 + length]
        regs = list(struct.unpack_from("<16Q", raw, p + 8 + CODE_BYTES))
        rflags = struct.unpack_from("<Q", raw, p + 8 + CODE_BYTES + 128)[0]
        q = p + 8 + CODE_BYTES + 136
        out.append(Case(code, regs, rflags, raw[q:q + WINDOW], raw[q + WINDOW:q + 2 * WINDOW], ""))
    return out


@dataclass
class Result:
    regs: list[int]
    rflags: int
    marker: int
    fault: int
    data: bytes
    stack: bytes


def read_results(path: Path) -> list[Result]:
    raw = path.read_bytes()
    out = []
    for p in range(0, len(raw), RESULT_SIZE):
        regs = list(struct.unpack_from("<16Q", raw, p))
        rflags, marker, fault = struct.unpack_from("<QQQ", raw, p + 128)
        out.append(Result(regs, rflags, marker, fault, raw[p + 152:p + 152 + WINDOW],
                          raw[p + 152 + WINDOW:p + 152 + 2 * WINDOW]))
    return out


def model(c: Case) -> tuple[Runtime | None, str]:
    """runtime.Runtime after one step of `c`, or None with the reason it stopped."""
    mem = Memory()
    code = mem.map(CODE_VA, 0x1000, "code")
    code.data[:len(c.code)] = c.code
    data = mem.map(DATA_VA, 0x1000, "data")
    data.data[DATA_OFF:DATA_OFF + WINDOW] = c.data
    stack = mem.map(STACK_VA, STACK_SIZE, "stack")
    stack.data[STACK_OFF:STACK_OFF + WINDOW] = c.stack
    rt = Runtime(Loaded(None, mem, CODE_VA, {}, STACK_VA + STACK_SIZE))   # type: ignore[arg-type]
    rt.th.regs = list(c.regs)
    rt.th.rip = CODE_VA
    rt.th.flags = {f: bool(c.rflags >> b & 1) for f, b in FLAG_BITS.items()}
    try:
        rt.step()
    except Exception as e:                                       # the model's stop is a result
        return None, f"{type(e).__name__}: {e}"
    return rt, ""


def judge(c: Case, r: Result, label: str) -> list[tuple[str, str]]:
    """(class, detail) for every field where the model differs from the CPU."""
    if r.marker == 2:
        return [(f"{label} cpu fault", f"exception 0x{r.fault:x}")]
    rt, why = model(c)
    if rt is None:
        return [(f"{label} model stopped", why)]
    diffs = []
    paths = {CODE_VA + len(c.code): 0, CODE_VA + len(c.code) + 14: 1, RET_TARGET: 1}
    if paths.get(rt.th.rip) != r.marker:
        diffs.append((f"{label} rip", f"model 0x{rt.th.rip:x}, cpu path {r.marker}"))
    for i in range(16):
        if rt.th.regs[i] != r.regs[i]:
            diffs.append((f"{label} r{i}", f"model 0x{rt.th.regs[i]:x}, cpu 0x{r.regs[i]:x}"))
    for f, b in FLAG_BITS.items():
        mv = rt.th.flags[f]
        if mv is not None and mv != bool(r.rflags >> b & 1):
            diffs.append((f"{label} {f}F", f"model {int(mv)}, cpu {r.rflags >> b & 1}"))
    if rt.mem.read(DATA_VA + DATA_OFF, WINDOW) != r.data:
        diffs.append((f"{label} data", "data window differs"))
    if rt.mem.read(STACK_VA + STACK_OFF, WINDOW) != r.stack:
        diffs.append((f"{label} stack", "stack window differs"))
    return diffs


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--cases", action="store_true")
    ap.add_argument("--out", type=Path, default=ROOT / "probes" / "semantics" / "cases.bin")
    ap.add_argument("--judge", type=Path)
    ap.add_argument("--record", action="store_true")
    ap.add_argument("--runs", type=Path, default=ROOT / "runs")
    a = ap.parse_args(argv)
    if a.cases:
        cs, skipped = build_cases()
        write_cases(cs, a.out)
        labels = Counter(c.label for c in cs)
        print(f"{len(cs)} cases over {len(labels)} iclasses -> {a.out} "
              f"({a.out.stat().st_size} bytes); skipped {dict(skipped)}")
        return 0
    if a.judge:
        return judge_run(a)
    ap.error("--cases or --judge")
    return 2


def judge_run(a: argparse.Namespace) -> int:
    t0 = time.monotonic()
    built, _ = build_cases()
    stored = read_cases(a.out)
    assert [(c.code, c.regs, c.rflags) for c in built] == [(c.code, c.regs, c.rflags) for c in stored], \
        "probes/semantics/cases.bin is not what --cases builds now"
    profile = (a.judge / "profile.txt").read_text().strip()
    result_files = sorted(a.judge.glob("result-*.bin"))
    runs = [read_results(p) for p in result_files]
    assert all(len(r) == len(built) for r in runs), "result count differs from case count"
    unstable = sum(1 for i in range(len(built))
                   if len({(x[i].regs[j], x[i].rflags, x[i].marker) for x in runs for j in range(16)}) >
                   len({(runs[0][i].regs[j], runs[0][i].rflags, runs[0][i].marker) for j in range(16)}))
    classes: Counter[str] = Counter()
    example: dict[str, str] = {}
    for i, (c, r) in enumerate(zip(built, runs[0], strict=True)):
        for k, detail in judge(c, r, c.label):
            classes[k] += 1
            example.setdefault(k, f"case {i}: {c.code.hex()} {detail}")
    print(f"cases {len(built)} over {len({c.label for c in built})} iclasses; runs {len(runs)} on {profile}; "
          f"cases whose CPU result differs between runs {unstable}; classes {len(classes)}; "
          f"{time.monotonic() - t0:.0f} s")
    for k, n in classes.most_common(40):
        print(f"  {n:6} {k}: {example[k]}")
    if a.record:
        record(a, profile, len(built), classes, example, time.monotonic() - t0)
    return 0


def record(a: argparse.Namespace, profile: str, total: int, classes: Counter[str], example: dict[str, str],
           elapsed: float) -> None:
    from substrate.api import Runtime as SubstrateRuntime, read_record, threshold_count

    run_id = f"semantics-{time.time_ns()}"
    target = f"w2m-lift-spec-{SPEC_VERSION}"
    empty = "sha256:" + hashlib.sha256(b"").hexdigest()
    progs = ROOT / "store" / "programs"
    progs.mkdir(parents=True, exist_ok=True)

    async def harness(_inp: Any) -> AsyncIterator[msgspec.Struct]:
        yield events.validate(events.SESSION_INIT(
            vocab_version=VOCAB_VERSION, profile=target, contract_version=SPEC_VERSION,
            harness_version="0.0.1", purpose="conformance"))
        cmp_id = f"cmp-{run_id}"
        yield events.validate(events.COMPARISON_COMPLETED(
            comparison_id=cmp_id, contract_id="x86.semantics", contract_version=SPEC_VERSION,
            relation="exact", reference_ref=profile, target_execution_id=a.judge.name,
            verdict="discrepancy" if classes else "conforms",
            rule_ref=f"{total} single-instruction cases, probes/semantics/cases.bin, {SPEC}"))
        for i, (k, n) in enumerate(classes.most_common(), 1):
            raw = bytes.fromhex(example[k].split()[2])
            digest = hashlib.sha256(raw).hexdigest()
            (progs / f"{digest}.bin").write_bytes(raw)
            did = f"{run_id}-d{i}"
            yield events.validate(events.DISCREPANCY_REPORTED(
                discrepancy_id=did, contract_id="x86.semantics", contract_version=SPEC_VERSION,
                mismatch_class="exact", minimized_program_sha256=f"sha256:{digest}",
                initial_state_sha256=empty, first_divergence=f"{k}: {n} cases; {example[k]}",
                reference_profile_id=profile, destination_profile_id=target, comparison_id=cmp_id))
            fault = k.endswith("cpu fault")
            yield events.validate(events.DISCREPANCY_ROUTED(
                discrepancy_id=did, repair_path="contract_revision" if fault else "implementation_repair",
                reference_conforms=True,
                grounds=("the decode spec calls these bytes an instruction and this CPU faults on them: "
                         "the contract lacks this CPU's rule" if fault else
                         "the runner's CPU executed the instruction; lift.py's operations disagree")))
        yield events.validate(events.SESSION_COMPLETED(
            outcome="completed_with_discrepancies" if classes else "completed",
            discrepancy_count=len(classes), accepted_count=0, elapsed_seconds=round(elapsed, 3)))

    def topology(b: Any) -> None:
        b.baseline(spec_version=SPEC_VERSION, reference=profile, cases=total)
        b.producer_kind("comparator", schemas=list(events.HARNESS_EVENTS), schema_version=1,
                        factory=lambda: harness)
        b.initial("comparator", input=None)
        b.termination(threshold_count("substrate.ProducerCompleted", 1))

    result = asyncio.run(SubstrateRuntime(a.runs / run_id).run(topology))
    kinds = Counter(e["kind"] for e in read_record(result.record_root) if not e["kind"].startswith("substrate."))
    print(f"record {result.record_root}: {dict(kinds)}")


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
