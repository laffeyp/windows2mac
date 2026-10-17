# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""The 0x67 prefix in 64-bit mode makes a 32-bit effective address from 32-bit registers, R8D-R15D among them
(Intel SDM Vol 1 3.3.7; Sprint 100). Before, lift() took 32 bits only for an E-named register and addressed through
R8D-R15D with all 64 bits of R8-R15. With R8 = 0x1_4000_0100 each load below reads 0x4000_0100, in the stack's
mapping; with 64 bits it reads 0x1_4000_0100, mapped nowhere, and the run faults early, so the instruction count
(the instructions retired before the fault, Sprint 099) tells the rules apart."""

from __future__ import annotations

import pytest

CODE, STACK = 0x2000_0000, 0x4000_0000

LOADS = bytes.fromhex(
    "49b80001004001000000"           # mov r8, 0x1_4000_0100
    "67418b00"                       # mov eax, [r8d]              base R8D
    "67418b1400"                     # mov edx, [r8d + eax]        base R8D, index EAX (EAX = 0, the stack is zero)
    "67428b140500000000"             # mov edx, [r8d*1 + 0]        index R8D, no base
    "c6042500000000 00"              # mov byte [0], 0             the end, a fault
    "ebfe")                          # jmp $


@pytest.mark.hvf
def test_a_0x67_address_through_r8d_is_32_bits() -> None:
    from w2m import vm_backend
    from w2m.loader import Loaded, Memory
    mem = Memory()
    mem.map(CODE, 0x1000, "code").data[:len(LOADS)] = LOADS
    mem.map(STACK, 0x10000, "stack")
    vm = vm_backend.VM(Loaded(None, mem, CODE, {}, STACK + 0x10000))   # type: ignore[arg-type]
    try:
        r = vm.run(max_steps=1 << 20, timeout_ms=10_000)
    finally:
        vm.close()
    assert r.stop.startswith("UnhandledException 0xc0000005"), r.stop
    assert r.instructions_executed == 4                       # the MOV and the three loads; the store faults
