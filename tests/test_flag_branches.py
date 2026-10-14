# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""A flag setter followed by a conditional jump, compiled into one block (recentering step 5: the jump branches on the
setter's NZCV). The class runs judge each instruction alone and the lockstep compiles one instruction a block, so neither
reaches the pair; this does. One program runs every case: two values loaded, one of ten setters, one of the sixteen
conditions, and whether the jump was taken stored as a byte. The expected byte comes from the SDM's flag rules."""

from __future__ import annotations

import random
import struct

import pytest

MASK = {8: 0xFF, 32: 0xFFFFFFFF, 64: (1 << 64) - 1}
# (name, encoding, bits, operation): the setter reads eax/ebx (al/bl, rax/rbx)
SETTERS = [
    ("cmp eax, ebx", "39d8", 32, "sub"), ("sub eax, ebx", "29d8", 32, "sub"), ("add eax, ebx", "01d8", 32, "add"),
    ("test eax, ebx", "85d8", 32, "and"), ("and eax, ebx", "21d8", 32, "and"), ("inc eax", "ffc0", 32, "inc"),
    ("dec eax", "ffc8", 32, "dec"), ("cmp al, bl", "38d8", 8, "sub"), ("cmp rax, rbx", "4839d8", 64, "sub"),
    ("add rax, rbx", "4801d8", 64, "add"),
]
CONDITIONS = ["o", "no", "b", "nb", "z", "nz", "be", "nbe", "s", "ns", "p", "np", "l", "nl", "le", "nle"]


def flags(op: str, a: int, b: int, bits: int) -> dict[str, int]:
    m, sign = MASK[bits], 1 << (bits - 1)
    a, b = a & m, b & m
    if op in ("add", "inc"):
        b = 1 if op == "inc" else b
        r = (a + b) & m
        f = {"C": int(a + b > m), "O": int(bool(~(a ^ b) & (a ^ r) & sign))}
    elif op in ("sub", "dec"):
        b = 1 if op == "dec" else b
        r = (a - b) & m
        f = {"C": int(a < b), "O": int(bool((a ^ b) & (a ^ r) & sign))}
    else:
        r = a & b
        f = {"C": 0, "O": 0}
    f.update(Z=int(r == 0), S=int(bool(r & sign)), P=int(bin(r & 0xFF).count("1") % 2 == 0))
    return f


def taken(cc: str, f: dict[str, int]) -> int:
    neg, key = cc.startswith("n"), cc[1:] if cc.startswith("n") else cc
    v = {"o": f["O"], "b": f["C"], "z": f["Z"], "be": f["C"] | f["Z"], "s": f["S"], "p": f["P"],
         "l": f["S"] ^ f["O"], "le": f["Z"] | (f["S"] ^ f["O"])}[key]
    return v ^ int(neg)


def load(reg: int, value: int, bits: int) -> bytes:
    """mov eax/ebx, imm32 or mov rax/rbx, imm64."""
    return (b"\x48" + bytes([0xB8 + reg]) + struct.pack("<Q", value)) if bits == 64 else \
        bytes([0xB8 + reg]) + struct.pack("<I", value & 0xFFFFFFFF)


@pytest.mark.hvf
def test_a_jump_after_a_setter_takes_the_branch_the_flags_say() -> None:
    from w2m import vm_backend
    from w2m.loader import Loaded, Memory
    rng = random.Random(68)
    edges = [0, 1, 2, 0x7F, 0x80, 0xFF, 0x7FFFFFFF, 0x80000000, 0xFFFFFFFF, (1 << 63) - 1, 1 << 63, (1 << 64) - 1]
    cases = []
    for name, enc, bits, op in SETTERS:
        for cc in CONDITIONS:
            if op in ("inc", "dec") and cc in ("b", "nb", "be", "nbe"):
                continue                                         # CF is kept, so undefined at the program's start
            for _ in range(6):
                cases.append((name, enc, bits, op, cc, rng.choice(edges + [rng.getrandbits(64)]),
                              rng.choice(edges + [rng.getrandbits(64)])))
    code_va, data_va, stack_va = 0x2000_0000, 0x3000_0000, 0x4000_0000
    out = bytearray()
    for i, (_, enc, bits, _, cc, a, b) in enumerate(cases):
        body = load(0, a, bits) + load(3, b, bits) + bytes.fromhex(enc)
        jcc_at = len(body)
        body += b"\x00\x00"                                      # jcc rel8, filled below
        not_taken = b"\xc6\x04\x25" + struct.pack("<I", data_va + i) + b"\x00"   # mov byte [data + i], 0
        body += not_taken + b"\xeb" + bytes([len(not_taken)])     # then jmp over the taken store
        body += b"\xc6\x04\x25" + struct.pack("<I", data_va + i) + b"\x01"     # mov byte [data + i], 1
        body = body[:jcc_at] + bytes([0x70 + CONDITIONS.index(cc), len(not_taken) + 2]) + body[jcc_at + 2:]
        out += body
    out += b"\xc6\x04\x25\x00\x00\x00\x00\x00"                       # mov byte [0], 0: the end
    mem = Memory()
    mem.map(code_va, (len(out) + 0xFFF) & ~0xFFF, "code").data[:len(out)] = out
    mem.map(data_va, 0x10000, "data")
    mem.map(stack_va, 0x10000, "stack")
    vm = vm_backend.VM(Loaded(None, mem, code_va, {}, stack_va + 0x10000))   # type: ignore[arg-type]
    try:
        r = vm.run(max_steps=1 << 30, timeout_ms=60_000)
        got = bytes(vm.mem.read(data_va, len(cases)))
    finally:
        vm.close()
    assert r.stop.startswith("UnhandledException 0xc0000005"), r.stop
    wrong = [(c[0], c[4], hex(c[5]), hex(c[6]), got[i]) for i, c in enumerate(cases)
             if got[i] != taken(c[4], flags(c[3], c[5], c[6], c[2]))]
    assert not wrong, (len(wrong), wrong[:8])
