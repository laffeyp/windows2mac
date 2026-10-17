# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""The instruction count is the instructions run (recentering step 5): it is the instruction clock's time, and the
step limit. Two ways it went wrong on 2026-10-14: the host computed it from the limit it read before a run while the
slice stub raised the limit in the VM, so each refill went missing; and a model step that leaves its block mid-way
resumes at the next instruction's own block, whose entry counts again what the first block's entry had counted.
Each program ends with a faulting store, three NOPs and a JMP, one block: its entry counts five, none of which runs (a
fault is reported before its instruction executes, Intel SDM Vol 3A 6.5), and the exception path has the host give the
five back (Sprint 098; before, the count held them)."""

from __future__ import annotations

import struct

import pytest

CODE, STACK = 0x2000_0000, 0x4000_0000
N = 200_000


def run(code: bytes, monkeypatch: pytest.MonkeyPatch, refill: bool) -> int:
    from w2m import vm_backend
    from w2m.kernel32 import M64
    from w2m.loader import Loaded, Memory
    from w2m.scheduler import TIME_SLICE
    monkeypatch.setattr(vm_backend, "TRANSLATION_CACHE_DIR", None)
    monkeypatch.setattr(vm_backend.VM, "_set_budget", lambda self: struct.pack_into(   # a time slice, as with threads
        "<Q", self.mm, vm_backend.MONITOR_DATA + 8, min(self._max_steps, struct.unpack_from(
            "<Q", self.mm, vm_backend.MONITOR_DATA)[0] + TIME_SLICE)))
    monkeypatch.setattr(vm_backend.VM, "_slice_until", lambda self, forced: 0 if forced or not refill else M64)
    mem = Memory()
    mem.map(CODE, 0x1000, "code").data[:len(code)] = code
    mem.map(STACK, 0x10000, "stack")
    vm = vm_backend.VM(Loaded(None, mem, CODE, {}, STACK + 0x10000))   # type: ignore[arg-type]
    try:
        r = vm.run(max_steps=1 << 40, timeout_ms=60_000)
    finally:
        vm.close()
    assert r.stop.startswith("UnhandledException 0xc0000005"), r.stop
    return int(r.instructions_executed)


END = bytes.fromhex("c6042500000000 00" "909090" "ebfe")     # mov byte [0], 0 (the end, a fault); nop x3; jmp $
LOOP = bytes.fromhex("b9") + struct.pack("<I", N) + bytes.fromhex("ffc9" "75fc") + END
# mov ecx, N; l: dec ecx; jnz l


@pytest.mark.hvf
def test_refills_in_the_vm_leave_the_count_whole(monkeypatch: pytest.MonkeyPatch) -> None:
    exiting = run(LOOP, monkeypatch, refill=False)            # every slice end leaves the VM
    assert run(LOOP, monkeypatch, refill=True) == exiting      # the stub refills about 20 slices in the VM


STEPPED = (bytes.fromhex("b9") + struct.pack("<I", 1000)       # mov ecx, 1000
           + bytes.fromhex("f9" "4801c0" "4801c0" "ffc9" "75f5")   # l: stc (a model step); add rax, rax x2; dec; jnz l
           + END)


@pytest.mark.hvf
def test_a_model_step_mid_block_counts_the_rest_of_its_block_once(monkeypatch: pytest.MonkeyPatch) -> None:
    exact = 1 + 5 * 1000                                      # the mov, five a turn; the end's block runs none
    assert run(STEPPED, monkeypatch, refill=True) == exact


SPINS = 20_000
SPIN = (bytes.fromhex("b9") + struct.pack("<I", SPINS)         # mov ecx, SPINS
        + bytes.fromhex("f390" "ffc9" "75fa")                  # l: pause; dec ecx; jnz l
        + END)


def slice_ends(code: bytes, monkeypatch: pytest.MonkeyPatch, ready: bool) -> tuple[int, int]:
    """The instructions run and the slice ends (hvc #4 exits) with another thread ready (SLICE_UNTIL 0) or none."""
    from w2m import vm_backend
    from w2m.kernel32 import M64
    from w2m.loader import Loaded, Memory
    from w2m.scheduler import TIME_SLICE
    monkeypatch.setattr(vm_backend, "TRANSLATION_CACHE_DIR", None)
    monkeypatch.setattr(vm_backend.VM, "_set_budget", lambda self: struct.pack_into(
        "<Q", self.mm, vm_backend.MONITOR_DATA + 8, min(self._max_steps, struct.unpack_from(
            "<Q", self.mm, vm_backend.MONITOR_DATA)[0] + TIME_SLICE)))
    monkeypatch.setattr(vm_backend.VM, "_slice_until", lambda self, forced: 0 if forced or ready else M64)
    mem = Memory()
    mem.map(CODE, 0x1000, "code").data[:len(code)] = code
    mem.map(STACK, 0x10000, "stack")
    vm = vm_backend.VM(Loaded(None, mem, CODE, {}, STACK + 0x10000))   # type: ignore[arg-type]
    try:
        r = vm.run(max_steps=1 << 40, timeout_ms=60_000)
        ends = vm.exits[4]
    finally:
        vm.close()
    assert r.stop.startswith("UnhandledException 0xc0000005"), r.stop
    return int(r.instructions_executed), int(ends)


@pytest.mark.hvf
def test_a_pause_yields_the_slice_when_another_thread_is_ready_and_the_count_holds(
        monkeypatch: pytest.MonkeyPatch) -> None:
    """Sprint 096 (KVM's directed yield): a PAUSE with another thread ready ends the slice at the next block's entry,
    the limit moved to the count so no instruction is counted that did not run; with none ready it only counts."""
    exact = 1 + 3 * SPINS                                      # the mov, three a turn; the end's block runs none
    yielded, yield_ends = slice_ends(SPIN, monkeypatch, ready=True)
    alone, alone_ends = slice_ends(SPIN, monkeypatch, ready=False)
    assert yielded == alone == exact
    assert yield_ends >= SPINS - 1                             # about one slice end a PAUSE
    assert alone_ends <= exact // 20_000 + 2                   # the slices of a lone thread


@pytest.mark.hvf
def test_a_fault_counts_only_the_instructions_before_it(monkeypatch: pytest.MonkeyPatch) -> None:
    """Two instructions run in the faulting block before the store; the store and the four after it are given back."""
    code = bytes.fromhex("4801c0" "4801c0") + END              # add rax, rax x2; the end
    assert run(code, monkeypatch, refill=True) == 2
