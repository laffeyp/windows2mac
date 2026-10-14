# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""Memory operands folded into Arm's own forms (recentering step 5): a base and a displacement as LDR/STR's scaled
offset or LDUR/STUR's -256..255, a base and an index scaled by the access size as the register-offset form. The class
runs' cases rarely hold an index scaled by the access size, so this program runs loads and stores of 1, 2, 4 and 8 bytes
over every scale and a spread of displacements: each load leaves its value in a result slot (from a register that held a
sentinel, so the zero-extension shows), each store writes the data region, and both are judged against the same
operations done on a copy of the memory in Python."""

from __future__ import annotations

import struct

import pytest

DATA, RESULTS = 0x3000_0000, 0x3001_0000
SENTINEL = 0x5555_5555_5555_5555


def modrm_sib(reg: int, scale: int | None, disp: int) -> bytes:
    """[rbx + rcx*scale + disp] (scale None: no index) with `reg` in ModRM.reg."""
    mod = 0 if disp == 0 else 1 if -128 <= disp <= 127 else 2
    ss = {None: 0, 1: 0, 2: 1, 4: 2, 8: 3}[scale]
    index = 4 if scale is None else 1                            # 100: no index; 001: rcx
    out = bytes([mod << 6 | reg << 3 | 4, ss << 6 | index << 3 | 3])
    return out + (b"" if mod == 0 else struct.pack("<b", disp) if mod == 1 else struct.pack("<i", disp))


@pytest.mark.hvf
def test_folded_addresses_load_and_store_what_x86_does() -> None:
    from w2m import vm_backend
    from w2m.loader import Loaded, Memory
    memory = bytearray((k * 7 + 3) & 0xFF for k in range(0x1000))
    code, expected = bytearray(), []
    base, index = DATA + 0x400, 5
    code += b"\x48\xbb" + struct.pack("<Q", base) + b"\x48\xb9" + struct.pack("<Q", index)   # mov rbx, rcx
    slot = 0
    for size in (1, 2, 4, 8):
        for scale in (None, 1, 2, 4, 8):
            for disp in (0, 8, -8, 0x28, -0x28, 300):
                at = base + (index * scale if scale else 0) + disp - DATA
                # the load: rax = sentinel, then movzx/mov rax|eax, [..], then mov [results + slot*8], rax
                code += b"\x48\xb8" + struct.pack("<Q", SENTINEL)
                op = {1: b"\x0f\xb6", 2: b"\x0f\xb7", 4: b"\x8b", 8: b"\x48\x8b"}[size]
                code += op + modrm_sib(0, scale, disp)
                code += b"\x48\x89\x04\x25" + struct.pack("<I", RESULTS + 8 * slot)
                expected.append(int.from_bytes(memory[at:at + size], "little"))
                slot += 1
                # the store: rdx = a value, then mov [..], dl|dx|edx|rdx
                value = (0x1122334455667788 + slot * 0x0101010101010101) & ((1 << 64) - 1)
                code += b"\x48\xba" + struct.pack("<Q", value)
                op = {1: b"\x88", 2: b"\x66\x89", 4: b"\x89", 8: b"\x48\x89"}[size]
                code += op + modrm_sib(2, scale, disp)
                memory[at:at + size] = (value & ((1 << 8 * size) - 1)).to_bytes(size, "little")
    code += b"\xc6\x04\x25\x00\x00\x00\x00\x00"                     # mov byte [0], 0: the end
    code_va, stack_va = 0x2000_0000, 0x4000_0000
    mem = Memory()
    mem.map(code_va, (len(code) + 0xFFF) & ~0xFFF, "code").data[:len(code)] = code
    mem.map(DATA, 0x1000, "data").data[:] = bytes((k * 7 + 3) & 0xFF for k in range(0x1000))
    mem.map(RESULTS, 0x1000, "results")
    mem.map(stack_va, 0x10000, "stack")
    vm = vm_backend.VM(Loaded(None, mem, code_va, {}, stack_va + 0x10000))   # type: ignore[arg-type]
    try:
        r = vm.run(max_steps=1 << 30, timeout_ms=60_000)
        got = [int.from_bytes(vm.mem.read(RESULTS + 8 * i, 8), "little") for i in range(slot)]
        data = bytes(vm.mem.read(DATA, 0x1000))
    finally:
        vm.close()
    assert r.stop.startswith("UnhandledException 0xc0000005"), r.stop
    assert got == expected, [(i, hex(g), hex(e)) for i, (g, e) in enumerate(zip(got, expected)) if g != e][:8]
    assert data == bytes(memory)
