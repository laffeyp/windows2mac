# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""Flag liveness (recentering step 5, Sprint 068 item L): a setter computes only the flags a later instruction reads.
The class runs judge one instruction per block (lockstep), so they never see a pass across instructions; this test
judges it. Seeded random programs of flag setters (ADD SUB CMP ADC SBB AND OR XOR TEST INC DEC NEG, shifts and rotates
by a constant and by CL, IMUL, BT, STC CLC CMC), readers (Jcc and JMP over one instruction, SETcc, CMOVcc, ADC, SBB,
CMC) and loads, stores and moves between them, in 64-, 32- and 8-bit forms, each program ending with its registers
and PUSHFQ's RFLAGS stored. A reader reads only flags the program has defined (w2m stops on an undefined one; PUSHFQ
pushes it as 0). The same programs run with the pass and without it, and every stored value must be equal; the pass
must also have compiled less code."""

from __future__ import annotations

import random
import struct

import pytest

DATA, RESULTS, CODE = 0x3000_0000, 0x3001_0000, 0x2000_0000
REGS = (0, 2, 3, 5, 6, 7)                       # RAX RDX RBX RBP RSI RDI; RCX holds shift counts
BYTE_REGS = (0, 2, 3)                           # AL DL BL
CC = {"o": "O", "no": "O", "b": "C", "nb": "C", "z": "Z", "nz": "Z", "be": "CZ", "nbe": "CZ", "s": "S", "ns": "S",
      "p": "P", "np": "P", "l": "SO", "nl": "SO", "le": "ZSO", "nle": "ZSO"}
CC_CODE = {name: i for i, name in enumerate(CC)}
ALL = set("CZSOPA")


def rr(reg: int, rm: int) -> int:
    return 0xC0 | (reg & 7) << 3 | (rm & 7)


class Program:
    """One random program; `defined` follows the SDM's flag tables so readers read only defined flags."""

    def __init__(self, rng: random.Random, index: int) -> None:
        self.rng, self.out, self.defined = rng, bytearray(), set(ALL)
        self.results = RESULTS + index * 8 * (len(REGS) + 1)
        for r in REGS:
            self.out += bytes([0x48, 0xC7, 0xC0 | r]) + struct.pack("<i", rng.randint(-2**31, 2**31 - 1))
        self.out += bytes([0x48, 0x39, rr(2, 0)])           # cmp rax, rdx: every flag defined to start

    def setter(self) -> bytes:
        rng = self.rng
        a, b = rng.choice(REGS), rng.choice(REGS)
        kind = rng.choice(["alu", "alu", "alu", "alu32", "alu8", "incdec", "neg", "shift", "shift_cl", "rotate",
                           "imul", "bt", "stc"])
        if kind in ("alu", "alu32", "alu8"):
            names = ["add", "or", "and", "sub", "xor", "cmp", "test"] + (["adc", "sbb"] if "C" in self.defined else [])
            name = rng.choice(names)
            op = {"add": 0x01, "or": 0x09, "adc": 0x11, "sbb": 0x19, "and": 0x21, "sub": 0x29, "xor": 0x31,
                  "cmp": 0x39, "test": 0x85}[name]
            self.defined = set(ALL) if name in ("add", "sub", "cmp", "adc", "sbb") else ALL - {"A"}
            if kind == "alu8":
                return bytes([op - 1, rr(rng.choice(BYTE_REGS), rng.choice(BYTE_REGS))])
            return bytes(([0x48] if kind == "alu" else []) + [op, rr(a, b)])
        if kind == "incdec":
            self.defined |= set("ZSOPA")
            return bytes([0x48, 0xFF, (0xC0 if rng.random() < 0.5 else 0xC8) | a])
        if kind == "neg":
            self.defined = set(ALL)
            return bytes([0x48, 0xF7, 0xD8 | a])
        if kind in ("shift", "rotate", "shift_cl"):
            ext = rng.choice([4, 5, 7] if kind != "rotate" else [0, 1])
            count = rng.choice([0, 1, 1, 2, 5, 63, 64, 65])
            if kind == "shift_cl":
                prefix, body = bytes([0xB9]) + struct.pack("<I", count), bytes([0x48, 0xD3, 0xC0 | ext << 3 | a])
            else:
                prefix, body = b"", bytes([0x48, 0xC1, 0xC0 | ext << 3 | a, count])
            m = count & 63
            if m and kind == "rotate":
                self.defined |= {"C"} | ({"O"} if m == 1 else set())
                self.defined -= set() if m == 1 else {"O"}
            elif m:
                self.defined = set(ALL) - {"A"} - (set() if m == 1 else {"O"})
            return prefix + body
        if kind == "imul":
            self.defined = {"C", "O"}
            return bytes([0x48, 0x0F, 0xAF, rr(a, b)])
        if kind == "bt":
            self.defined = {"C"} | ({"Z"} & self.defined)
            return bytes([0x48, 0x0F, 0xA3, rr(b, a)])
        choice = rng.choice([0xF9, 0xF8] + ([0xF5] if "C" in self.defined else []))   # STC CLC CMC
        self.defined |= {"C"}
        return bytes([choice])

    def reader(self) -> bytes | None:
        rng = self.rng
        conds = [c for c, fl in CC.items() if set(fl) <= self.defined]
        kind = rng.choice(["jcc", "jmp", "setcc", "cmov"])
        if kind in ("jcc", "setcc", "cmov") and not conds:
            return None
        if kind == "setcc":
            return bytes([0x0F, 0x90 + CC_CODE[rng.choice(conds)], 0xC0 | rng.choice(BYTE_REGS)])
        if kind == "cmov":
            return bytes([0x48, 0x0F, 0x40 + CC_CODE[rng.choice(conds)], rr(rng.choice(REGS), rng.choice(REGS))])
        saved = set(self.defined)
        skipped = self.setter() if rng.random() < 0.6 else self.other()
        self.defined &= saved                               # after the jump: what both paths define
        head = bytes([0x70 + CC_CODE[rng.choice(conds)]]) if kind == "jcc" else bytes([0xEB])
        if kind == "jmp":
            self.defined = saved
        return head + bytes([len(skipped)]) + skipped

    def other(self) -> bytes:
        rng, r = self.rng, self.rng.choice(REGS)
        kind = rng.choice(["load", "store", "mov"])
        if kind == "mov":
            return bytes([0x48, 0x89, rr(rng.choice(REGS), r)])
        op = 0x8B if kind == "load" else 0x89
        return bytes([0x48, op, 0x04 | r << 3, 0x25]) + struct.pack("<I", DATA + 8 * rng.randrange(64))

    def build(self, n: int) -> bytes:
        for _ in range(n):
            p = self.rng.random()
            piece = self.setter() if p < 0.45 else (self.reader() if p < 0.8 else self.other())
            self.out += piece or self.other()
        for i, r in enumerate(REGS):
            self.out += bytes([0x48, 0x89, 0x04 | r << 3, 0x25]) + struct.pack("<I", self.results + 8 * i)
        self.out += bytes([0x9C, 0x58, 0x48, 0x89, 0x04, 0x25]) + struct.pack("<I", self.results + 8 * len(REGS))
        return bytes(self.out)


def run(code: bytes, liveness: bool, count: int, monkeypatch: pytest.MonkeyPatch) -> tuple[list[int], int]:
    """The stored values and the code cache's bytes in use."""
    from w2m import vm_backend
    from w2m.loader import Loaded, Memory
    monkeypatch.setattr(vm_backend, "FLAG_LIVENESS", liveness)
    mem = Memory()
    mem.map(CODE, (len(code) + 0xFFF) & ~0xFFF, "code").data[:len(code)] = code
    mem.map(DATA, 0x1000, "data").data[:] = bytes((k * 37 + 11) & 0xFF for k in range(0x1000))
    mem.map(RESULTS, (count * 8 + 0xFFF) & ~0xFFF, "results")
    mem.map(0x4000_0000, 0x10000, "stack")
    vm = vm_backend.VM(Loaded(None, mem, CODE, {}, 0x4001_0000))   # type: ignore[arg-type]
    try:
        r = vm.run(max_steps=1 << 30, timeout_ms=120_000)
        got = [int.from_bytes(vm.mem.read(RESULTS + 8 * i, 8), "little") for i in range(count)]
        used = vm.code_cursor
    finally:
        vm.close()
    assert r.stop.startswith("UnhandledException 0xc0000005"), r.stop
    return got, used


@pytest.mark.hvf
def test_flag_liveness_keeps_every_flag_a_program_reads(monkeypatch: pytest.MonkeyPatch) -> None:
    rng = random.Random(20261014)
    programs = 300
    code = b"".join(Program(rng, i).build(40) for i in range(programs))
    code += b"\xc6\x04\x25\x00\x00\x00\x00\x00"                    # mov byte [0], 0: the end
    count = programs * (len(REGS) + 1)
    with_pass, bytes_with = run(code, True, count, monkeypatch)
    without, bytes_without = run(code, False, count, monkeypatch)
    differ = [(i // (len(REGS) + 1), i % (len(REGS) + 1), hex(a), hex(b))
              for i, (a, b) in enumerate(zip(with_pass, without)) if a != b]
    assert not differ, differ[:8]
    assert bytes_with < bytes_without, (bytes_with, bytes_without)
