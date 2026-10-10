"""Sprint 040 grading surface: the guest state in the VM back end, judged on Windows's records.

Sprint 039's single-instruction cases run in the VM; for faults, the guest runtime's exception
record and CONTEXT are compared with what Windows gave its vectored handler on Intel CPUs; for
completed cases, GPRs, flags with lazy AF, DF and memory. Three planted faults must be caught by
those records alone: POP [m] moving RSP before its store (the commit rule), AF computed from the
wrong operands (lazy AF), and the reconstruction table mapping a host PC one byte off.
"""

from __future__ import annotations

import sys
from pathlib import Path

import pytest

from test_semantics_differential import _built_039
from w2m import lift as lift_module
from w2m import a64_encoder as a64
from w2m import vm_backend
from w2m.encgen import cases as encgen_cases
from w2m.lift import commit_order, lift

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))
import semantics_differential as sd  # noqa: E402
import vm_oracle  # noqa: E402

INTEL = ("38011862284", "38011858047")                 # Xeon Platinum 8573C, 8370C


def test_every_liftable_encoding_obeys_the_commit_rule() -> None:
    lifted = 0
    for c in encgen_cases():
        try:
            ins = lift(c.raw + bytes(16), 0x1000)
        except Exception:                               # noqa: BLE001 — not liftable: not this test's subject
            continue
        lifted += 1
        assert commit_order(ins) is None, commit_order(ins)
    assert lifted > 8000


def test_commit_order_names_a_register_write_before_a_store() -> None:
    ins = lift_module.Lifted(0x1000, 1, "PUSH", (
        ("ld", 1, ("r", 4, 64, False)), ("imm", 2, 8), ("alu", 3, "sub", 1, 2, 64),
        ("st", ("r", 4, 64, False), 3), ("st", ("m", 3, 8), 1)))
    assert commit_order(ins) == "PUSH at 0x1000: st at operation 3 before memory access 4"


def _faults(labels: set[str] | None = None) -> tuple[list[sd.Case], list[sd.Result]]:
    """Every fault case of the Xeon 8573C run (store/probe-runs/38011862284/result-1.faults.bin)."""
    built = _built_039()
    found = vm_oracle.read_fault_results(ROOT / "store" / "probe-runs" / INTEL[0] / "result-1.faults.bin")
    keep = [i for i in sorted(found) if labels is None or built[i].label in labels]
    return [built[i] for i in keep], [found[i] for i in keep]


@pytest.mark.hvf
def test_vm_builds_windows_context_for_every_fault_case() -> None:
    """Every 039 fault, SSE included since Sprint 041: CONTEXT with XMM and MXCSR as Windows gave it."""
    cases, results = _faults()
    assert len(cases) == 5485
    classes, example, skipped, judged, faults = vm_oracle.run_cases(cases, results)
    assert classes == {}, [example[k] for k in classes][:5]
    assert not skipped and judged == faults == 5485


def _subset(run: str, labels: set[str] | None = None, faults_only: bool = False
            ) -> tuple[list[sd.Case], list[sd.Result]]:
    built = list(_built_039()[::10])
    results = sd.read_results(ROOT / "store" / "probe-runs" / run / "result-1.every10.bin")
    pairs = [(c, r) for c, r in zip(built, results, strict=True)
             if (labels is None or c.label in labels) and (not faults_only or r.marker == 2)]
    return [c for c, _ in pairs], [r for _, r in pairs]


@pytest.mark.hvf
@pytest.mark.parametrize("run", INTEL)
def test_vm_agrees_with_windows_on_intel(run: str) -> None:
    cases, results = _subset(run)
    classes, example, _skipped, judged, faults = vm_oracle.run_cases(cases, results)
    assert classes == {}, [example[k] for k in classes][:5]
    assert judged > 2000 and faults > 150


@pytest.mark.hvf
def test_planted_pop_order_is_caught(monkeypatch: pytest.MonkeyPatch) -> None:
    """POP [m] writing RSP before its store, the order the lifter had until this sprint: a fault
    on the store leaves RSP moved, and Windows's CONTEXT shows RSP from before the instruction.
    (A PUSH that moves RSP first is the same defect, but no Windows record can show it: PUSH
    faults here are loads of a memory source, and a fault on the stack store itself cannot be
    dispatched on Windows, so Sprint 039 never placed one.)"""
    real_lift = lift

    def rsp_first(code: bytes, address: int) -> lift_module.Lifted:
        ins = real_lift(code, address)
        ops = list(ins.ops)
        if ins.iclass == "POP" and ops[-2][0] == "st" and ops[-2][1][0] == "m":
            ops[-2], ops[-1] = ops[-1], ops[-2]
        return lift_module.Lifted(ins.address, ins.length, ins.iclass, tuple(ops))
    monkeypatch.setattr(vm_backend, "lift", rsp_first)
    cases, results = _faults({"POP"})
    classes, _, _, judged, faults = vm_oracle.run_cases(cases, results)
    assert faults == judged > 0
    assert any(k.endswith("fault r4") for k in classes), classes


@pytest.mark.hvf
def test_planted_lazy_af_operand_is_caught(monkeypatch: pytest.MonkeyPatch) -> None:
    """ADD/SUB storing a in place of r for lazy AF: AF then reads bit 4 of b alone."""
    original = vm_backend._set_flags

    def wrong_af(a: vm_backend.Assembler, kind: str, ra: int, rb: int, rr: int, bits: int) -> None:
        original(a, kind, ra, rb, rr, bits)
        if kind in ("add", "sub", "inc", "dec"):
            a.emit(a64.str_(8, ra, 28, vm_backend.AF_OPERANDS + 16))
    monkeypatch.setattr(vm_backend, "_set_flags", wrong_af)
    cases, results = _subset(INTEL[0], {"ADD", "SUB"})
    classes, _, _, _, _ = vm_oracle.run_cases(cases, results)
    assert any(k.endswith(" AF") for k in classes), classes


@pytest.mark.hvf
def test_planted_reconstruction_offset_is_caught(monkeypatch: pytest.MonkeyPatch) -> None:
    """The table mapping each host PC to the x86 address one byte on: every fault's RIP moves."""
    original = vm_backend.VM._recon_append

    def off_by_one(self: vm_backend.VM, marks: list[tuple[int, int]]) -> None:
        original(self, [(h, g + 1 if g else g) for h, g in marks])
    monkeypatch.setattr(vm_backend.VM, "_recon_append", off_by_one)
    cases, results = _faults({"MOV", "ADD"})
    classes, _, _, judged, _ = vm_oracle.run_cases(cases, results)
    assert judged > 0 and any(k.endswith("fault rip") for k in classes), classes


@pytest.mark.hvf
def _store_loop() -> tuple[int, int, list[int], set[int], int, str]:
    """ADD [m], 1; PUSH; POP; DEC; JNZ, `iterations` times, with the host interrupting the vCPU
    every millisecond. Returns (iterations, count, stops, block starts, end rip, stop)."""
    from w2m.loader import Loaded, Memory

    iterations = 300_000_000
    code_va, data_va, stack_va = 0x2000_0000, 0x3000_0000, 0x4000_0000
    loop = (bytes.fromhex("48830425") + data_va.to_bytes(4, "little") + b"\x01"   # add qword [data], 1
            + bytes.fromhex("51 59 ffc9"))                                       # push rcx; pop rcx; dec ecx
    body = bytes.fromhex("b9") + iterations.to_bytes(4, "little") + loop
    jnz = bytes([0x75, (-(len(loop) + 2)) & 0xFF])
    code = body + jnz + bytes.fromhex("c6042500000000 00")                       # mov byte [0], 0: the end
    mem = Memory()
    mem.map(code_va, 0x1000, "code").data[:len(code)] = code
    mem.map(data_va, 0x1000, "data")
    mem.map(stack_va, 0x10000, "stack")
    vm = vm_backend.VM(Loaded(None, mem, code_va, {}, stack_va + 0x10000))   # type: ignore[arg-type]
    stops: list[int] = []
    try:
        r = vm.run(max_steps=1 << 40, timeout_ms=60_000, interrupt_every_ms=1,
                   on_boundary=lambda v, x, rip: stops.append(rip))
        count = int.from_bytes(vm.mem.read(data_va, 8), "little")
        starts = set(vm.blocks)
    finally:
        vm.close()
    return iterations, count, stops, starts, code_va + len(body) + len(jnz), r.stop


@pytest.mark.hvf
def test_forward_rule_stops_only_at_boundaries_and_repeats_no_store() -> None:
    """Each asynchronous stop runs on to the next block entry: the loop's count is exact, and
    every stop is at a translated block's first x86 instruction."""
    iterations, count, stops, starts, end, stop = _store_loop()
    assert stop == f"UnhandledException 0xc0000005 at x86 0x{end:x}", stop
    assert count == iterations
    assert len(stops) >= 100, len(stops)
    assert set(stops) <= starts, sorted(set(stops) - starts)[:5]


@pytest.mark.hvf
def test_planted_backward_rule_repeats_stores(monkeypatch: pytest.MonkeyPatch) -> None:
    """Restarting the interrupted x86 instruction instead (mapping the stop backward) re-runs
    half-done work from the middle of an instruction or of the dispatcher: the same checks the
    forward test makes, the count and the final stop, both fail (first run: the guest fetched
    from 0 with the count at 4,691,481 of 300,000,000)."""
    monkeypatch.setattr(vm_backend, "ASYNC_STOP_RULE", "backward")
    iterations, count, stops, _starts, end, stop = _store_loop()
    assert stops
    assert count != iterations or stop != f"UnhandledException 0xc0000005 at x86 0x{end:x}"


@pytest.mark.hvf
def test_cpuid_profile_is_the_same_in_model_vm_and_table() -> None:
    """CPUID in the model and in the VM's guest runtime returns specs/guest_cpuid.json's values
    for every listed leaf; above the maximum basic or extended leaf, the highest basic leaf (SDM);
    the VM logs each read in the service ring."""
    import json

    from w2m import cpuid
    from w2m.loader import Loaded, Memory
    from w2m.runtime import Runtime

    spec = json.loads(cpuid.SPEC.read_text())
    inputs = [(e["leaf"], e["subleaf"] or 0) for e in spec["leaves"]]
    inputs += [(1, 5), (7, 1), (8, 0), (0x40000000, 0), (0x80000009, 0), (0xFFFFFFFF, 0)]
    code_va, stack_va = 0x2000_0000, 0x4000_0000

    def memory() -> Memory:
        mem = Memory()
        mem.map(code_va, 0x1000, "code").data[:2] = b"\x0f\xa2"     # cpuid
        mem.map(stack_va, 0x10000, "stack")
        return mem

    model_out = []
    for eax, ecx in inputs:
        rt = Runtime(Loaded(None, memory(), code_va, {}, stack_va + 0x10000))   # type: ignore[arg-type]
        rt.th.regs[0], rt.th.regs[1] = eax | 0xDEAD_0000_0000_0000, ecx
        rt.step()
        model_out.append(tuple(rt.th.regs[i] for i in (0, 3, 1, 2)))
    vm = vm_backend.VM(Loaded(None, memory(), code_va, {}, stack_va + 0x10000), lockstep=True)   # type: ignore[arg-type]
    vm_out: list[tuple[int, ...]] = []
    pending = list(inputs)

    def drive(v: vm_backend.VM, x: list[int]) -> None:
        if len(vm_out) < len(inputs) and len(pending) < len(inputs):
            vm_out.append(tuple(x[i] for i in (0, 3, 1, 2)))
        if not pending:
            raise vm_backend.StopRun
        eax, ecx = pending.pop(0)
        x[0], x[1], x[17] = eax | 0xDEAD_0000_0000_0000, ecx, code_va
    try:
        vm.run(on_dispatch=drive)
        calls = vm.calls()
    finally:
        vm.close()
    want = [cpuid.lookup(eax, ecx) for eax, ecx in inputs]
    assert model_out == want
    assert vm_out == want
    assert calls.count("Cpuid") == len(inputs)
    assert want[inputs.index((0x40000000, 0))] == cpuid.lookup(7, 0)      # above max basic: highest basic leaf


def _sse_subset(labels: set[str]) -> tuple[list[sd.Case], list[sd.Result]]:
    """Every 039 case of these classes, with the Xeon 8573C's full results (release semantics-039)."""
    built = _built_039()
    results = sd.read_results(vm_oracle.result_file(INTEL[0]))
    keep = [i for i, c in enumerate(built) if c.label in labels]
    return [built[i] for i in keep], [results[i] for i in keep]


@pytest.mark.hvf
def test_planted_missing_indefinite_fixup_is_caught(monkeypatch: pytest.MonkeyPatch) -> None:
    """FCVTZS alone saturates to 0x7FFFFFFF and gives 0 for NaN; x86 gives 0x80000000. Reach: the 37
    register-source CVTT* cases of 039 whose input is NaN, infinite or out of range."""
    monkeypatch.setattr(vm_backend, "CVT_INDEFINITE", False)
    cases, results = _sse_subset({"CVTTSS2SI", "CVTTSD2SI"})
    classes, _, _, _, _ = vm_oracle.run_cases(cases, results)
    assert any(" r" in k and "fault" not in k for k in classes), classes


@pytest.mark.hvf
def test_planted_fpcr_ah_zero_is_caught(monkeypatch: pytest.MonkeyPatch) -> None:
    """FPCR.AH=0: FMIN/FMAX return the NaN instead of the second source. Reach: 117 cases."""
    original = vm_backend.fpcr_for
    monkeypatch.setattr(vm_backend, "fpcr_for", lambda mxcsr: original(mxcsr) & ~2)
    cases, results = _sse_subset({f"{op}{t}" for op in ("MIN", "MAX") for t in ("SS", "SD", "PS", "PD")})
    classes, _, _, _, _ = vm_oracle.run_cases(cases, results)
    assert any("xmm" in k for k in classes), classes


@pytest.mark.hvf
def test_planted_idc_not_mapped_to_de_is_caught(monkeypatch: pytest.MonkeyPatch) -> None:
    """FPSR.IDC not read as MXCSR.DE. Reach: 105 arithmetic cases with a denormal lane, DAZ clear."""
    monkeypatch.setattr(vm_backend, "FPSR_TO_MXCSR", tuple(m for m in vm_backend.FPSR_TO_MXCSR if m[0] != 7))
    cases, results = _sse_subset({f"{op}{t}" for op in ("ADD", "SUB", "MUL", "DIV") for t in ("SS", "SD", "PS", "PD")})
    classes, _, _, _, _ = vm_oracle.run_cases(cases, results)
    assert any(k.endswith("MXCSR") for k in classes), classes


@pytest.mark.hvf
def test_unmasked_exceptions_take_the_slow_path() -> None:
    """MXCSR states that unmask an exception run on x86fp; the count is on the record, and the
    results still agree with the CPU (covered by the full runs above)."""
    cases, results = _sse_subset({"ADDSS", "DIVSD", "CVTTSS2SI"})
    slow: list[int] = []
    classes, _, _, judged, _ = vm_oracle.run_cases(cases, results, slow=slow)
    unmasked = sum(1 for c in cases if c.mxcsr >> 7 & 0x3F != 0x3F)
    assert classes == {} and judged == len(cases)
    assert slow[0] == unmasked > 0


@pytest.mark.hvf
def test_vm_agrees_with_windows_on_every_041_fault() -> None:
    """Case set 041's 2,195 faults (store/probe-runs/38029980267/result-1.faults.bin) in the VM, LOCK
    forms included, against the Xeon Platinum 8573C."""
    seed, classes, locked, control = sd.CASE_SETS["041"]
    built = sd.cached_cases("041")
    found = vm_oracle.read_fault_results(ROOT / "store" / "probe-runs" / "38029980267" / "result-1.faults.bin")
    cases, results = [built[i] for i in sorted(found)], [found[i] for i in sorted(found)]
    diffs, example, skipped, judged, faults = vm_oracle.run_cases(cases, results)
    assert diffs == {}, [example[k] for k in diffs][:5]
    assert not skipped and judged == faults == 2195


@pytest.mark.hvf
def test_xgetbv_is_undefined_under_the_profile() -> None:
    """XGETBV raises #UD while CR4.OSXSAVE is clear (SDM XGETBV), and the guest profile's
    CPUID.1:ECX.OSXSAVE is clear (specs/guest_cpuid.json): the model and the VM both end the process
    with STATUS_ILLEGAL_INSTRUCTION at the instruction (Sprint 044). The runners' CPUs set OSXSAVE, so
    the CPU cannot judge this class; the profile does, as for CPUID."""
    from w2m.loader import GuestFault, Loaded, Memory
    from w2m.runtime import STATUS_ILLEGAL_INSTRUCTION, Runtime
    from w2m.vm_backend import VM

    code_va, stack_va = 0x2000_0000, 0x4000_0000

    def memory() -> Memory:
        mem = Memory()
        mem.map(code_va, 0x1000, "code").data[:3] = b"\x0f\x01\xd0"      # xgetbv
        mem.map(stack_va, 0x10000, "stack")
        return mem

    rt = Runtime(Loaded(None, memory(), code_va, {}, stack_va + 0x10000))   # type: ignore[arg-type]
    with pytest.raises(GuestFault) as e:
        rt.step()
    assert e.value.code == STATUS_ILLEGAL_INSTRUCTION and rt.th.rip == code_va
    vm = VM(Loaded(None, memory(), code_va, {}, stack_va + 0x10000))       # type: ignore[arg-type]
    try:
        r = vm.run(max_steps=100)
        exc = vm.exception()
    finally:
        vm.close()
    assert r.stop.startswith("UnhandledException 0xc000001d at x86 0x20000000"), r.stop
    assert exc is not None and exc["code"] == STATUS_ILLEGAL_INSTRUCTION and exc["rip"] == code_va
