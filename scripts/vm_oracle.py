"""vm_oracle.py — Sprint 039's single-instruction cases run in the managed VM and judged against
Windows on Intel (Sprint 040).

Each case (scripts/semantics_differential.py format 2: code bytes, the 16 GPRs, RFLAGS, memory
windows, the data page's last 32 bytes before an unmapped page) runs in one VM, one instruction
per block: the driver writes the case's code, memory and registers at a dispatch miss and reads
the state at the next one. A fault goes the way a Windows fault goes in w2m: the EL1 vector saves
the registers, the guest runtime maps the host PC to the instruction through the reconstruction
table, fills the thread's state block and builds the EXCEPTION_RECORD and CONTEXT; with no handler
the exception is unhandled, and the driver reads both records and goes on.

The reference is the Windows result file of an Intel run (release semantics-039, sha256 in the
run's results.sha256): for a fault, the exception code and the CONTEXT Windows gave its vectored
handler (RIP, GPRs, EFlags); for a completed case, the GPRs, RFLAGS and the path taken; for both,
the three memory windows. Compared: GPRs, RIP or path, CF PF AF ZF SF OF where w2m defines them,
DF, IF and bit 1 at a fault, XMM0-15 and MXCSR (Sprint 041), and the windows. Cases whose
instruction the VM cannot compile are counted by class, not judged.

Usage: uv run python scripts/vm_oracle.py <Intel run id> [--record] [--runs runs] [--limit N]
"""

from __future__ import annotations

import argparse
import hashlib
import json
import struct
import subprocess
import sys
import time
from collections import Counter
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))
sys.path.insert(0, str(ROOT / "src"))

import semantics_differential as sd  # noqa: E402

from w2m.lift import LiftError, lift  # noqa: E402
from w2m.loader import Loaded, Memory  # noqa: E402
from w2m.vm_backend import (FLAG_REG, VM, Assembler, CompileError, StopRun, compile_insn,  # noqa: E402
                            x86_flags)
from w2m.x86dec import DecodeError  # noqa: E402
from w2m.x86dec_tables import SPEC_VERSION  # noqa: E402

RELEASES = {"039": "semantics-039", "041": "semantics-041", "042": "semantics-042", "044": "semantics-044", "044b": "semantics-044b", "044c": "semantics-044c"}
EFLAGS_FAULT_MASK = 0x1 | 0x2 | 0x4 | 0x10 | 0x40 | 0x80 | 0x200 | 0x400 | 0x800   # CF 1 PF AF ZF SF IF DF OF


def result_file(run: str, case_set: str = "039") -> Path:
    """The run's first full result file, from its case set's release, checked against results.sha256."""
    out = ROOT / "build" / "probe-runs" / run / "result-1.bin"
    want = next(line.split()[0].lower() for line in (ROOT / "store" / "probe-runs" / run / "results.sha256")
                .read_text().splitlines() if line.strip().endswith("result-1.bin"))
    if not out.exists() or hashlib.sha256(out.read_bytes()).hexdigest() != want:
        out.parent.mkdir(parents=True, exist_ok=True)
        subprocess.run(["gh", "release", "download", RELEASES[case_set], "-p", f"{run}-result-1.bin", "-O", str(out),
                        "--clobber"], check=True, cwd=ROOT)
        got = hashlib.sha256(out.read_bytes()).hexdigest()
        if got != want:
            raise SystemExit(f"{out}: sha256 {got}, results.sha256 says {want}")
    return out


def read_fault_results(path: Path) -> dict[int, sd.Result]:
    """A fault extract (store/probe-runs/<run>/result-1.faults.bin): case index -> Windows result,
    for every case that faulted on the CPU."""
    raw = path.read_bytes()
    assert raw[:4] == b"W2MF"
    count = struct.unpack_from("<I", raw, 4)[0]
    out = {}
    step = 4 + sd.RESULT_SIZE
    for n in range(count):
        p = 8 + n * step
        index = struct.unpack_from("<I", raw, p)[0]
        tmp = raw[p + 4:p + step]
        out[index] = _result(tmp)
    return out


def _result(rec: bytes) -> sd.Result:
    regs = list(struct.unpack_from("<16Q", rec, 0))
    rflags, marker, fault, fault_rip, mxcsr = struct.unpack_from("<5Q", rec, 128)
    q = 168
    xmm = [int.from_bytes(rec[q + 16 * i:q + 16 * i + 16], "little") for i in range(16)]
    q += 256
    return sd.Result(regs, rflags, marker, fault, rec[q:q + sd.WINDOW], rec[q + sd.WINDOW:q + 2 * sd.WINDOW],
                     fault_rip, mxcsr, xmm, rec[q + 2 * sd.WINDOW:q + 2 * sd.WINDOW + sd.EDGE])


def vm_compiles(code: bytes) -> str | None:
    """None when the VM can compile the case's instruction; otherwise why not."""
    try:
        ins = lift(code + bytes(16), sd.CODE_VA)
        compile_insn(Assembler(0), ins, 0)
    except (LiftError, CompileError, DecodeError) as e:
        return f"{type(e).__name__}: {str(e).split(' at ')[0]}"
    return None


def judge_fault(c: sd.Case, r: sd.Result, exc: dict[str, int], mem: Memory) -> list[tuple[str, str]]:
    diffs = []
    if r.marker != 2:
        return [("vm fault, cpu completed", f"vm 0x{exc['code']:x} at 0x{exc['rip']:x}")]
    if exc["code"] != r.fault:
        diffs.append(("fault code", f"vm 0x{exc['code']:x}, cpu 0x{r.fault:x}"))
    if exc["rip"] != r.fault_rip:
        diffs.append(("fault rip", f"vm 0x{exc['rip']:x}, cpu 0x{r.fault_rip:x}"))
    if exc["address"] != exc["rip"]:
        diffs.append(("fault address", f"ExceptionAddress 0x{exc['address']:x}, Rip 0x{exc['rip']:x}"))
    for i in range(16):
        if exc[f"r{i}"] != r.regs[i]:
            diffs.append((f"fault r{i}", f"vm 0x{exc[f'r{i}']:x}, cpu 0x{r.regs[i]:x}"))
    if (exc["eflags"] ^ r.rflags) & EFLAGS_FAULT_MASK:
        diffs.append(("fault eflags", f"vm 0x{exc['eflags']:x}, cpu 0x{r.rflags:x}"))
    if exc["mxcsr"] != r.mxcsr:
        diffs.append(("fault MXCSR", f"vm 0x{exc['mxcsr']:x}, cpu 0x{r.mxcsr:x} (in 0x{c.mxcsr:x})"))
    for i in range(16):
        if exc[f"xmm{i}"] != r.xmm[i]:
            diffs.append((f"fault xmm{i}", f"vm {exc[f'xmm{i}']:032x}, cpu {r.xmm[i]:032x}"))
    diffs += windows(mem, r)
    return diffs


def judge_done(c: sd.Case, r: sd.Result, vm: VM, x: list[int], mem: Memory) -> list[tuple[str, str]]:
    if r.marker == 2:
        return [("vm completed, cpu fault", f"cpu 0x{r.fault:x}")]
    diffs = []
    paths = {sd.CODE_VA + len(c.code): 0, sd.CODE_VA + len(c.code) + 14: 1, sd.RET_TARGET: 1}
    if paths.get(x[17]) != r.marker:
        diffs.append(("rip", f"vm 0x{x[17]:x}, cpu path {r.marker}"))
    for i in range(16):
        if x[i] != r.regs[i]:
            diffs.append((f"r{i}", f"vm 0x{x[i]:x}, cpu 0x{r.regs[i]:x}"))
    vf = x86_flags(x, vm)
    for f, b in sd.FLAG_BITS.items():
        if vf[f] is not None and vf[f] != bool(r.rflags >> b & 1):
            diffs.append((f"{f}F", f"vm {int(bool(vf[f]))}, cpu {r.rflags >> b & 1}"))
    if vm.df() != bool(r.rflags >> sd.DF_BIT & 1):
        diffs.append(("DF", f"vm {int(vm.df())}, cpu {r.rflags >> sd.DF_BIT & 1}"))
    xmm, _fpcr, _fpsr = vm.vectors()
    mx = vm.mxcsr()
    if mx != r.mxcsr:
        diffs.append(("MXCSR", f"vm 0x{mx:x}, cpu 0x{r.mxcsr:x} (in 0x{c.mxcsr:x})"))
    for i in range(16):
        if xmm[i] != r.xmm[i]:
            diffs.append((f"xmm{i}", f"vm {xmm[i]:032x}, cpu {r.xmm[i]:032x}"))
    diffs += windows(mem, r)
    return diffs


def windows(mem: Memory, r: sd.Result) -> list[tuple[str, str]]:
    out = []
    if mem.read(sd.DATA_VA + sd.DATA_OFF, sd.WINDOW) != r.data:
        out.append(("data window", "differs"))
    if mem.read(sd.STACK_VA + sd.STACK_OFF, sd.WINDOW) != r.stack:
        out.append(("stack window", "differs"))
    if mem.read(sd.DATA_VA + sd.EDGE_OFF, sd.EDGE) != r.edge:
        out.append(("edge window", "differs (a store before the fault)"))
    return out


def run_cases(cases: list[sd.Case], results: list[sd.Result], limit: int | None = None,
              slow: list[int] | None = None) -> tuple[Counter[str], dict[str, str], Counter[str], int, int]:
    """Judge every case the VM compiles. Returns (difference classes, an example per class,
    skipped by reason, cases judged, faults judged)."""
    mem = Memory()
    mem.map(sd.CODE_VA, 0x1000, "code")
    mem.map(sd.DATA_VA, 0x1000, "data")
    mem.map(sd.STACK_VA, sd.STACK_SIZE, "stack")
    ld = Loaded(None, mem, sd.CODE_VA, {}, sd.STACK_VA + sd.STACK_SIZE)   # type: ignore[arg-type]
    vm = VM(ld, lockstep=True)
    order: list[int] = []
    skipped: Counter[str] = Counter()
    for i, c in enumerate(cases):
        why = vm_compiles(c.code) or sd.undefined_memory(c)
        if why:
            skipped[f"{c.label}: {why}"] += 1
        else:
            order.append(i)
    if limit is not None:
        order = order[:limit]
    classes: Counter[str] = Counter()
    example: dict[str, str] = {}
    state = {"pos": 0, "current": -1, "faults": 0, "judged": 0}
    zeros_page, zeros_stack = bytes(0x1000), bytes(sd.STACK_SIZE)

    def note(i: int, diffs: list[tuple[str, str]]) -> None:
        c = cases[i]
        state["judged"] += 1
        for k, detail in diffs:
            key = f"{c.label} {k}"
            classes[key] += 1
            example.setdefault(key, f"case {i}: {c.code.hex()} {detail}")

    def start_next(v: VM, x: list[int]) -> None:
        if state["pos"] == len(order):
            raise StopRun
        i = order[state["pos"]]
        state["pos"] += 1
        state["current"] = i
        c = cases[i]
        v.flush_translations()
        v.mem.write(sd.CODE_VA, zeros_page)
        v.mem.write(sd.CODE_VA, c.code)
        v.mem.write(sd.DATA_VA, zeros_page)
        v.mem.write(sd.DATA_VA + sd.DATA_OFF, c.data)
        v.mem.write(sd.DATA_VA + sd.EDGE_OFF, c.edge)
        v.mem.write(sd.STACK_VA, zeros_stack)
        v.mem.write(sd.STACK_VA + sd.STACK_OFF, c.stack)
        x[:16] = c.regs
        x[17] = sd.CODE_VA
        for f in "CZSOP":
            x[FLAG_REG[f]] = c.rflags >> sd.FLAG_BITS[f] & 1
        x[24] = 0
        v.set_af_df(bool(c.rflags >> sd.FLAG_BITS["A"] & 1), bool(c.rflags >> sd.DF_BIT & 1))
        v.set_mxcsr(c.mxcsr, list(c.xmm))

    def on_dispatch(v: VM, x: list[int]) -> None:
        if state["current"] >= 0:
            i = state["current"]
            note(i, judge_done(cases[i], results[i], v, x, v.mem))
            state["current"] = -1
        start_next(v, x)

    def on_exception(v: VM, exc: dict[str, int]) -> bool:
        i = state["current"]
        state["faults"] += 1
        note(i, judge_fault(cases[i], results[i], exc, v.mem))
        state["current"] = -1
        return True

    try:
        r = vm.run(max_steps=1 << 40, timeout_ms=600_000, on_dispatch=on_dispatch, on_exception=on_exception)
        if r.stop != "stopped by on_dispatch":
            raise SystemExit(f"VM stopped at case {state['current']}: {r.stop}")
    finally:
        if slow is not None:
            slow.append(vm.slow_sse)
        vm.close()
    return classes, example, skipped, state["judged"], state["faults"]


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("run")
    ap.add_argument("--set", default="039", choices=sorted(RELEASES))
    ap.add_argument("--record", action="store_true")
    ap.add_argument("--runs", type=Path, default=ROOT / "runs")
    ap.add_argument("--limit", type=int)
    a = ap.parse_args(argv)
    if a.record and a.limit:
        ap.error("--record judges every class; --limit runs a prefix of the compiled cases")
    t0 = time.monotonic()
    store = ROOT / "store" / "probe-runs" / a.run
    seed, class_set, locked, control = sd.CASE_SETS[a.set]
    cases, _ = sd.build_cases(seed, class_set, locked, control)
    built = ROOT / "build" / "probe-runs" / f"semantics-{a.set}-cases.bin"
    built.parent.mkdir(parents=True, exist_ok=True)
    sd.write_cases(cases, built)
    sha = hashlib.sha256(built.read_bytes()).hexdigest()
    ran = (store / "cases.sha256").read_text().strip().split()[0].lower()
    if sha != ran:
        raise SystemExit(f"the cases built here (sha256 {sha}) are not the ones run {a.run} ran ({ran})")
    results = sd.read_results(result_file(a.run, a.set))
    assert len(results) == len(cases), "result count differs from case count"
    cpu = (store / "cpu.txt").read_text().strip()
    profile = (store / "profile.txt").read_text().strip()
    classes, example, skipped, judged, faults = run_cases(cases, results, a.limit)
    elapsed = time.monotonic() - t0
    print(f"cases {len(cases)}; VM compiles {len(cases) - sum(skipped.values())}; judged {judged} "
          f"({faults} faults) against {cpu} ({profile}); difference classes {len(classes)}; {elapsed:.0f} s")
    for k, n in classes.most_common(40):
        print(f"  {n:6} {k}: {example[k]}")
    by_class = Counter(k.split(":")[0] for k in skipped.elements())
    print(f"not compiled by the VM: {sum(skipped.values())} cases in {len(by_class)} classes "
          f"(first: {', '.join(f'{c} {n}' for c, n in by_class.most_common(8))})")
    if a.record:
        print(json.dumps(record(a, f"{cpu}; {profile}", judged, faults, classes, example,
                                [c.label for c in cases],
                                skipped), indent=1))
    return 1 if classes else 0


def record(a: argparse.Namespace, profile: str, judged: int, faults: int, classes: Counter[str],
           example: dict[str, str], labels: list[str], skipped: Counter[str]) -> dict[str, Any]:
    """One causal differential run (Sprint 043): a unit per instruction class, its candidate the
    class's rule and lowering as the VM runs it; the Intel CPU under Windows is the reference."""
    from w2m import differential
    from w2m.rules import registry

    grouped = differential.by_unit(labels, classes, example)
    not_compiled = Counter(k.split(":")[0] for k in skipped.elements())
    rules = registry()
    whole = "sha256:" + hashlib.sha256("".join(sorted(r.sha256 for r in rules.values())).encode()).hexdigest()

    def judge_unit(unit: str) -> differential.Verdict:
        lines = grouped[unit]
        rule = (f"{sum(1 for x in labels if x == unit)} cases of {unit} in semantics-{a.set} run {a.run}; "
                f"{not_compiled[unit]} not compiled by the VM")
        if not lines:
            return differential.Verdict("conforms", rule_ref=rule)
        return differential.Verdict("discrepancy", lines, rule, "implementation_repair",
                                    "the Intel CPU under Windows is the reference; the VM back end or guest runtime differs")

    return differential.run(differential.Differential(
        name=f"vm-oracle-{a.set}", profile=profile, purpose="conformance",
        contract_id=lambda unit: "w2m.guest_state.fault_context", contract_version="0.2.0", relation="exact",
        implementation=f"the managed VM back end and guest runtime, lifter spec {SPEC_VERSION}",
        implementation_sha256=whole, generator_ref="hand (agent-authored)", reference_ref=a.run,
        target_execution_id=f"vm-oracle-{a.run}", units=sorted(set(labels)), compare=judge_unit,
        baseline={"run": a.run, "judged": judged, "faults": faults},
        unit_sha256=lambda unit: rules[unit].sha256 if unit in rules else whole), a.runs)


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
