"""Sprint 030 grading surface: the managed-VM back end. The runner-built skeleton.exe runs inside
a Hypervisor.framework VM as AArch64 compiled from lifted operations; lockstep comparison with
the model runtime after every x86 instruction finds no difference; planted compiler faults are
found at a named x86 address, and random operand states reach faults the program path does
not."""

from __future__ import annotations

import pytest

from test_runtime import _skeleton, _verdict
from w2m import a64_encoder as a64
from w2m import vm_backend
from w2m.loader import load
from w2m.runtime import Runtime
from w2m.vm_backend import VM, lockstep, state_differential

pytestmark = pytest.mark.hvf


def test_skeleton_runs_in_vm_and_conforms() -> None:
    model = Runtime(load(_skeleton())).run()
    vm = VM(load(_skeleton()))
    try:
        r = vm.run()
    finally:
        vm.close()
    assert (r.stop, r.exit_code) == ("ExitProcess", 0)
    assert (r.instructions_executed, r.distinct_addresses) == (model.instructions_executed,
                                                               model.distinct_addresses)
    assert r.calls == model.calls and r.stdout == model.stdout
    assert _verdict(r.stdout) == "conforms"


def test_lockstep_finds_no_difference() -> None:
    result = lockstep(_skeleton())
    assert result.differences == []
    assert result.compared == result.model.instructions_executed + len(result.model.calls)
    assert result.vm.stop == "ExitProcess" and result.vm.stdout == result.model.stdout


def test_planted_parity_fault_is_found(monkeypatch: pytest.MonkeyPatch) -> None:
    """PF compiled with the opposite sense: lockstep names the first instruction that sets it."""
    orig = a64.cset

    def wrong(rd: int, cond: str) -> int:
        return orig(rd, "ne" if rd == vm_backend.REG_PF and cond == "eq" else cond)

    monkeypatch.setattr(a64, "cset", wrong)
    result = lockstep(_skeleton(), limit=0)
    assert result.differences and "PF vm" in result.differences[0]
    assert "x86 0x14000" in result.differences[0]


def test_state_differential_finds_no_difference() -> None:
    result = state_differential(_skeleton())
    assert (result.instructions, result.cases, result.differences) == (217, 217 * 40, [])


def test_planted_carry_fault_passes_lockstep_and_fails_state_differential(
        monkeypatch: pytest.MonkeyPatch) -> None:
    """SUB's carry (unsigned borrow) compiled as signed less-than. Every SUB and CMP on the
    skeleton's path has operands of equal sign, where the two agree, so the full lockstep run
    passes; random operand states find it."""
    orig = a64.cset
    monkeypatch.setattr(a64, "cset",
                        lambda rd, cond: orig(rd, "lt" if cond == "lo" else cond))
    assert lockstep(_skeleton()).differences == []
    found = state_differential(_skeleton(), limit=0).differences
    assert found and "CF vm" in found[0]


def test_planted_store_width_fault_is_found(monkeypatch: pytest.MonkeyPatch) -> None:
    """32-bit stores compiled as 64-bit. The skeleton's 32-bit stores are RIP-relative into
    .data; the bytes compared around each effective address catch the extra four."""
    orig = a64.str_
    monkeypatch.setattr(a64, "str_",
                        lambda size, rt, rn, offset=0: orig(8 if size == 4 else size, rt, rn, offset))
    found = state_differential(_skeleton(), limit=0).differences
    assert found and "differs" in found[0]


def test_unmapped_store_is_an_unhandled_access_violation() -> None:
    """Sprint 040: the guest runtime builds the exception record and CONTEXT a Windows handler
    would get; with no handler the process ends with the exception code, as on Windows."""
    data = bytearray(_skeleton())
    ld = load(bytes(data))
    vm = VM(ld)
    try:
        vm.mem.write(ld.entry, bytes.fromhex("8804250000000000"))   # mov [0x0], al
        r = vm.run()
        exc = vm.exception()
    finally:
        vm.close()
    assert r.stop == f"UnhandledException 0xc0000005 at x86 0x{ld.entry:x}"
    assert r.exit_code == 0xC0000005
    assert exc is not None
    assert (exc["code"], exc["address"], exc["parameters"], exc["info0"], exc["info1"]) == (
        0xC0000005, ld.entry, 2, 1, 0)
    assert exc["rip"] == ld.entry and exc["r4"] == ld.stack_top - 0x100 - 8
    assert exc["eflags"] & 0x202 == 0x202 and exc["mxcsr"] == exc["fltsave_mxcsr"] == 0x1F80
    assert exc["fltsave_control_word"] == 0x27F and exc["seg_cs"] == 0x33


def test_unliftable_instruction_stops_the_vm() -> None:
    ld = load(_skeleton())
    vm = VM(ld)
    try:
        vm.mem.write(ld.entry, bytes.fromhex("0f0b"))               # UD2: no lifting rule
        r = vm.run()
    finally:
        vm.close()
    assert r.stop.startswith("LiftError") and r.exit_code is None
