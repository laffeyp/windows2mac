# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""mmx.py — the MMX-register forms UnityPlayer.dll reaches (Sprint 065), on the x87 register file.

Intel SDM Vol 1 §9.5.1 and Table 9-2: MMn is bits 63:0 of the physical x87 register Rn (not the stack-relative ST(n)).
An MMX instruction sets TOP to 0 after it reads its operands and before it writes its result, and every tag to valid
after the write
(a faulting load leaves both; a faulting store has set TOP and kept the tags); one that writes MMn sets Rn's bits 79:64
to all ones. EMMS sets TOP to 0 and marks every register empty (set 065b). An x87 exception pending (FSW.ES) makes any of them #MF, which has no rows: it stops the
run. Each class's lanes follow its Vol 2 page: MOVD, MOVQ, MOVDQ2Q, PADDW, PADDD, PADDSW, PSUBW, PMULLW, PMULHW,
PMADDUBSW, PACKUSWB, PUNPCKL/H BW/WD/DQ (the low forms read 4 bytes of memory), PXOR, PSRAW, PSRAD, PSRLQ, PEXTRW.
"""

from __future__ import annotations

from collections.abc import Callable
from typing import Any

from w2m.x87 import ES, Fpu, X87Unmasked

M64 = (1 << 64) - 1
ONES = 0xFFFF << 64                                  # bits 79:64 of a register an MMX instruction writes


def lanes(v: int, bits: int) -> list[int]:
    return [v >> (i * bits) & ((1 << bits) - 1) for i in range(64 // bits)]


def join(xs: list[int], bits: int) -> int:
    return sum((x & ((1 << bits) - 1)) << (i * bits) for i, x in enumerate(xs))


def signed(x: int, bits: int) -> int:
    return x - (1 << bits) if x >> (bits - 1) & 1 else x


def clamp(v: int, lo: int, hi: int) -> int:
    return lo if v < lo else hi if v > hi else v


def _interleave(a: list[int], b: list[int]) -> list[int]:
    return [x for pair in zip(a, b, strict=True) for x in pair]


def compute(ic: str, d: int, s: int, count: int) -> int:
    """The 64-bit result of a lane operation on destination d and source s (count: a shift's count)."""
    if ic == "PXOR":
        return d ^ s
    if ic in ("PADDW", "PADDD", "PSUBW"):
        bits = 32 if ic == "PADDD" else 16
        sub = ic == "PSUBW"
        return join([x - y if sub else x + y for x, y in zip(lanes(d, bits), lanes(s, bits), strict=True)], bits)
    if ic == "PADDSW":
        return join([clamp(signed(x, 16) + signed(y, 16), -0x8000, 0x7FFF)
                     for x, y in zip(lanes(d, 16), lanes(s, 16), strict=True)], 16)
    if ic in ("PMULLW", "PMULHW"):
        shift = 16 if ic == "PMULHW" else 0
        return join([signed(x, 16) * signed(y, 16) >> shift for x, y in zip(lanes(d, 16), lanes(s, 16), strict=True)], 16)
    if ic == "PMADDUBSW":                            # DEST's bytes unsigned, SRC's signed; adjacent products summed
        db, sb = lanes(d, 8), [signed(x, 8) for x in lanes(s, 8)]
        return join([clamp(db[2 * i] * sb[2 * i] + db[2 * i + 1] * sb[2 * i + 1], -0x8000, 0x7FFF) for i in range(4)], 16)
    if ic == "PACKUSWB":
        return join([clamp(signed(x, 16), 0, 0xFF) for x in lanes(d, 16) + lanes(s, 16)], 8)
    if ic.startswith("PUNPCK"):
        bits = {"BW": 8, "WD": 16, "DQ": 32}[ic[-2:]]
        half = 32 // bits
        a, b = lanes(d, bits), lanes(s, bits)
        lo = ic[6] == "L"
        return join(_interleave(a[:half] if lo else a[half:], b[:half] if lo else b[half:]), bits)
    if ic in ("PSRAW", "PSRAD"):
        bits = 16 if ic == "PSRAW" else 32
        n = min(count, bits - 1)
        return join([signed(x, bits) >> n for x in lanes(d, bits)], bits)
    if ic == "PSRLQ":
        return 0 if count > 63 else d >> count
    raise ValueError(f"MMX {ic} is not modelled")


def execute(fpu: Fpu, ic: str, opcode: int, dst: tuple[Any, ...], src: tuple[Any, ...], imm: int,
            read: Callable[[tuple[Any, ...]], int], write: Callable[[tuple[Any, ...], int], None]) -> None:
    """One MMX instruction. dst and src are ("mm", n), ("x", n) an XMM register, ("r", ...) a general register or
    ("m", bytes) the memory operand; read and write reach the latter three (an MMX register is the Fpu's)."""
    if fpu.sw & ES:
        raise X87Unmasked(f"{ic} with an x87 exception pending (status word 0x{fpu.sw:x})")

    def get(o: tuple[Any, ...]) -> int:
        return fpu.regs[o[1]] & M64 if o[0] == "mm" else read(o)

    def put(o: tuple[Any, ...], v: int) -> None:
        if o[0] == "mm":
            fpu.regs[o[1]] = ONES | v & M64
        else:
            write(o, v)

    def transition() -> None:                        # TOP 0 after the reads, before the write (set 065b, Xeon 8573C:
        fpu.sw &= ~(7 << 11)                         # a faulting load leaves TOP, a faulting store has set it)
        if ic == "EMMS":
            fpu.empty = [True] * 8

    if ic == "EMMS":
        transition()
        return
    if ic in ("MOVD", "MOVQ", "MOVDQ2Q"):
        v = get(src)
        transition()
        put(dst, v & 0xFFFF_FFFF if ic == "MOVD" else v & M64)
    elif ic == "PEXTRW":
        v = get(src) >> (16 * (imm & 3)) & 0xFFFF
        transition()
        put(dst, v)
    else:
        shift = ic in ("PSRAW", "PSRAD", "PSRLQ")
        count = (imm if src[0] == "i" else get(src)) if shift else 0
        v = compute(ic, get(dst), 0 if shift else get(src), count)
        transition()
        put(dst, v)
    fpu.empty = [False] * 8                          # the tags valid once the write is done (a faulting store keeps them)
