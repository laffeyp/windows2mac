# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""Several vCPUs, each on its own monitor thread, each with its private translation chain (Sprint 104;
docs/design-several-vcpus-2026-10-17.md, design B). While vCPU 0 runs a small x86 program, vCPUs 1-3 each run seven AArch64
words: read MPIDR_EL1, store it at one address on the monitor data page and one in the state block, read both back,
read a heap page the host mapped after the vCPUs started, and stop. The same address holds each vCPU's own value, in
its own page of the memory file; the heap page reads the same on all; each vCPU's ACTLR_EL1 has EnTSO and its
TTBR0_EL1 has CnP clear."""

from __future__ import annotations

import struct
from typing import Any

import pytest

CODE, STACK = 0x2000_0000, 0x4000_0000
PROGRAM = bytes.fromhex("b801000000" "c6042500000000 00" "ebfe")      # mov eax, 1; mov byte [0], 0 (the end); jmp $
# clang -target arm64-apple-macos: mrs x0, mpidr_el1; str x0, [x28, #0xf00]; str x0, [x27]; ldr x1, [x28, #0xf00];
# ldr x2, [x27]; ldr x3, [x26]; hvc #0
WORDS = [0xD53800A0, 0xF9078380, 0xF9000360, 0xF9478381, 0xF9400362, 0xF9400343, 0xD4000002]
MARK = 0x5EED_0F_A11_CAFE
SLOT, STATE_SLOT = 0xF00, 0xF80                 # free on the monitor data page; past the state block's 896 bytes


def run_four(monkeypatch: pytest.MonkeyPatch, shared_monitor_data: bool = False) -> dict[str, Any]:
    from w2m import vm_backend
    from w2m.loader import Loaded, Memory
    monkeypatch.setattr(vm_backend, "TRANSLATION_CACHE_DIR", None)
    if shared_monitor_data:                       # the plant: every vCPU's monitor-data leaf at vCPU 0's page
        pages = vm_backend.VM._vcpu_pages
        monkeypatch.setattr(vm_backend.VM, "_vcpu_pages", lambda self, i: [
            (va, vm_backend.MONITOR_DATA if va == vm_backend.MONITOR_VA + vm_backend.MONITOR_DATA else page)
            for va, page in pages(self, i)])
    mem = Memory()
    mem.map(CODE, 0x1000, "code").data[:len(PROGRAM)] = PROGRAM
    mem.map(STACK, 0x10000, "stack")
    vm = vm_backend.VM(Loaded(None, mem, CODE, {}, STACK + 0x10000))   # type: ignore[arg-type]
    vm.vcpu_count = 4
    heap = vm_backend.HEAP_VA
    vm._map_pages(heap, heap + vm_backend.PAGE, False)
    seen: dict[str, Any] = {}

    def on_dispatch(v: Any, x: list[int]) -> None:
        if seen:
            return
        v._map_pages(heap, heap + vm_backend.PAGE, True)        # after the vCPUs started
        v.write_bytes(heap, MARK.to_bytes(8, "little"))
        at = v.code_cursor + 0x10000                            # code-cache words no block has reached
        v._write_words(at, WORDS)
        regs = [0] * 31
        regs[28] = vm_backend.MONITOR_VA + vm_backend.MONITOR_DATA
        regs[27] = vm_backend.PROCESS_VA + vm_backend.STATE_OFF + STATE_SLOT
        regs[26] = heap
        exits = [c.run(vm_backend.MONITOR_VA + at, vm_backend.CPSR_EL1H, regs, 5_000, None)[0] for c in (u.channel for u in v.vcpus[1:])]
        seen["exits"] = [(e.kind, e.x[0], e.x[1], e.x[2], e.x[3]) for e in exits]
        seen["pages"] = [struct.unpack_from("<Q", v.mm, vm_backend.VCPU_AREA + i * vm_backend.VCPU_STRIDE
                                            + vm_backend.VS_MONITOR_DATA + SLOT)[0] for i in range(1, 4)]
        seen["vcpu0"] = struct.unpack_from("<Q", v.mm, vm_backend.MONITOR_DATA + SLOT)[0]
        seen["readback"] = list(v.vcpu_readback)

    try:
        r = vm.run(max_steps=1 << 20, timeout_ms=20_000, on_dispatch=on_dispatch)
    finally:
        vm.close()
    seen["stop"] = r.stop
    return seen


@pytest.mark.hvf
def test_four_vcpus_hold_their_own_state_at_one_address(monkeypatch: pytest.MonkeyPatch) -> None:
    seen = run_four(monkeypatch)
    assert seen["stop"].startswith("UnhandledException 0xc0000005"), seen["stop"]
    exits = seen["exits"]
    assert [e[0] for e in exits] == ["hvc"] * 3, exits
    assert [e[1] & 0xFF for e in exits] == [1, 2, 3], exits                 # MPIDR_EL1 Aff0 = the vCPU's index
    assert all(e[1] == e[2] == e[3] for e in exits), exits                 # each read back its own, both pages
    assert [e[4] for e in exits] == [MARK] * 3, exits                       # the page mapped after the start
    assert seen["pages"] == [e[1] for e in exits]                           # each in its own page of the file
    assert seen["vcpu0"] not in seen["pages"]
    for actlr, ttbr in seen["readback"]:
        assert actlr & 2, seen["readback"]                                  # EnTSO
        assert not ttbr & 1, seen["readback"]                               # CnP


@pytest.mark.hvf
def test_planted_shared_monitor_page_collapses_the_values(monkeypatch: pytest.MonkeyPatch) -> None:
    """The plant: every vCPU's monitor-data leaf at vCPU 0's page; the private pages never see a store."""
    seen = run_four(monkeypatch, shared_monitor_data=True)
    assert seen["pages"] != [e[1] for e in seen["exits"]]
    assert seen["vcpu0"] == seen["exits"][-1][1]                            # the last vCPU's store is all that is left
