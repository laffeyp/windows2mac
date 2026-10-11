# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""integer_model.py — the integer classes the model computes whole (Sprint 050, set 050).

A rule emits ("x86", ic, dst, srcs, bits) (lift.py); the model (runtime.py) calls `execute` with the values of srcs,
and the VM runs the instruction as one model step (hvc #11). Each class follows its Intel SDM Vol 2 page:

  SHLD, SHRD   dst, src, count: the count masked to 5 bits (6 with REX.W); 0 changes nothing; CF the last bit
               shifted out of dst, OF for a count of 1, SF ZF PF by the result, AF undefined
  RCR          dst, count: rotate right through CF; the masked count taken mod 9 (8 bits) or 17 (16 bits); OF for
               a masked count of 1 (MSB(dst) XOR CF before the rotation); only CF and OF change
  STC CLC CMC  CF set, cleared, complemented; no other flag
  SAHF         SF ZF AF PF CF from AH bits 7, 6, 4, 2, 0
  PUSHFQ       RFLAGS to the stack; a flag the model holds undefined pushes as 0, IF (bit 9) and bit 1 as 1
  POPFQ        CF PF AF ZF SF DF OF from the stack; TF, AC, NT or ID set stops the run (not in the rows)
  POPCNT       the set bits of src; ZF src was 0, CF OF SF AF PF 0
  LOOP LOOPE LOOPNE JRCXZ  RCX - 1 (no flags), then the branch on RCX != 0 (and ZF, or not ZF); JRCXZ on RCX = 0
  LODSB STOSB MOVSB MOVSD SCASB SCASD CMPSB CMPSD  one element through RSI and RDI, stepped by its size, down when
               DF is set; SCAS and CMPS set the flags of AL/EAX - [RDI] and [RSI] - [RDI]
"""

from __future__ import annotations

from typing import Any

from w2m.lift import RAX, RCX, RDI, RSI, RSP, Op

M64 = (1 << 64) - 1
FLAG_BITS = {"C": 0, "P": 2, "A": 4, "Z": 6, "S": 7, "O": 11}
DF_BIT, IF_BIT = 10, 9
STRINGS = {"LODSB": ("lods", 1), "STOSB": ("stos", 1), "MOVSB": ("movs", 1), "MOVSD": ("movs", 4),
           "SCASB": ("scas", 1), "SCASD": ("scas", 4), "CMPSB": ("cmps", 1), "CMPSD": ("cmps", 4)}


def _mask(bits: int) -> int:
    return (1 << bits) - 1


def _parity(v: int) -> bool:
    return bin(v & 0xFF).count("1") % 2 == 0


def execute(rt: Any, ic: str, dst: Op | None, vals: list[int], bits: int) -> int | None:
    """Run `ic`; returns a branch target when the instruction branches, else None."""
    th, f = rt.th, rt.th.flags
    m, sign = _mask(bits), 1 << (bits - 1)
    if ic in ("SHLD", "SHRD"):
        a, b, count = vals[0] & m, vals[1] & m, vals[2] & (0x3F if bits == 64 else 0x1F)
        if count == 0:
            return None
        if ic == "SHLD":                                        # the high half of (dst:src) << count
            r = ((a << bits | b) << count >> bits) & m if count <= bits else 0
            cf = bool(a >> (bits - count) & 1) if count <= bits else None
        else:                                                   # the low half of (src:dst) >> count
            r = ((b << bits | a) >> count) & m if count <= bits else 0
            cf = bool(a >> (count - 1) & 1) if count <= bits else None
        rt._store(dst, r)
        f.update(C=cf, Z=r == 0, S=bool(r & sign), P=_parity(r), A=None,
                 O=(bool(r & sign) != bool(a & sign)) if count == 1 else None)
        return None
    if ic == "RCR":
        a, masked = vals[0] & m, vals[1] & (0x3F if bits == 64 else 0x1F)
        count = masked % 9 if bits == 8 else masked % 17 if bits == 16 else masked
        if masked == 0:
            return None
        carry = int(rt._flag("C"))
        of = (bool(a & sign) != bool(carry)) if masked == 1 else None
        wide = a | carry << bits                                # bits + 1 wide: CF above the operand
        for _ in range(count):
            wide = (wide >> 1) | (wide & 1) << bits
        rt._store(dst, wide & m)
        f.update(C=bool(wide >> bits & 1), O=of)
        return None
    if ic in ("STC", "CLC", "CMC"):
        f["C"] = True if ic == "STC" else False if ic == "CLC" else not rt._flag("C")
        return None
    if ic == "SAHF":
        ah = th.regs[RAX] >> 8 & 0xFF
        f.update(S=bool(ah & 0x80), Z=bool(ah & 0x40), A=bool(ah & 0x10), P=bool(ah & 0x04), C=bool(ah & 0x01))
        return None
    if ic == "PUSHFQ":
        v = 1 << 1 | 1 << IF_BIT | int(th.df) << DF_BIT
        v |= sum(1 << bit for flag, bit in FLAG_BITS.items() if f[flag])
        sp = (th.regs[RSP] - 8) & M64
        rt.mem.write(sp, v.to_bytes(8, "little"))
        th.regs[RSP] = sp
        return None
    if ic == "POPFQ":
        sp = th.regs[RSP]
        v = int.from_bytes(rt.mem.read(sp, 8), "little")
        if v & (1 << 8 | 1 << 18 | 1 << 14 | 1 << 21):
            from w2m.kernel32 import Unimplemented
            raise Unimplemented(f"POPFQ of 0x{v:x}: TF, AC, NT or ID set, not in the rows")
        f.update({flag: bool(v >> bit & 1) for flag, bit in FLAG_BITS.items()})
        th.df = bool(v >> DF_BIT & 1)
        th.regs[RSP] = (sp + 8) & M64
        return None
    if ic == "POPCNT":
        v = vals[0] & m
        rt._store(dst, bin(v).count("1"))
        f.update(Z=v == 0, C=False, O=False, S=False, A=False, P=False)
        return None
    if ic in ("LOOP", "LOOPE", "LOOPNE"):
        rcx = (th.regs[RCX] - 1) & M64
        th.regs[RCX] = rcx
        taken = rcx != 0 and (ic == "LOOP" or rt._flag("Z") == (ic == "LOOPE"))
        return vals[0] if taken else None
    if ic == "JRCXZ":
        return vals[0] if th.regs[RCX] == 0 else None
    if ic in STRINGS:
        kind, size = STRINGS[ic]
        step = -size if th.df else size
        mask = _mask(8 * size)
        regs = th.regs
        if kind == "lods":
            v = int.from_bytes(rt.mem.read(regs[RSI], size), "little")
            regs[RAX] = regs[RAX] & ~mask & M64 | v if size < 4 else v
            regs[RSI] = (regs[RSI] + step) & M64
        elif kind == "stos":
            rt.mem.write(regs[RDI], (regs[RAX] & mask).to_bytes(size, "little"))
            regs[RDI] = (regs[RDI] + step) & M64
        elif kind == "movs":
            rt.mem.write(regs[RDI], rt.mem.read(regs[RSI], size))
            regs[RSI] = (regs[RSI] + step) & M64
            regs[RDI] = (regs[RDI] + step) & M64
        else:
            if kind == "scas":
                a = regs[RAX] & mask
            else:
                a = int.from_bytes(rt.mem.read(regs[RSI], size), "little")
            b = int.from_bytes(rt.mem.read(regs[RDI], size), "little")
            r = (a - b) & mask
            w = 8 * size
            f.update(C=a < b, Z=r == 0, S=bool(r >> (w - 1) & 1), P=_parity(r), A=bool((a ^ b ^ r) & 0x10),
                     O=bool((a ^ b) & (a ^ r) >> (w - 1) & 1))
            if kind == "cmps":
                regs[RSI] = (regs[RSI] + step) & M64
            regs[RDI] = (regs[RDI] + step) & M64
        return None
    raise ValueError(f"integer_model: {ic}")
