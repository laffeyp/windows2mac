# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""x86ops.py — explicit operands of a decoded instruction, resolved from the row's operand
template (SDM Vol 2D Appendix A notation) against the fields x86dec.decode returns (Sprint 022).

Output uses the xedshim operand format (src/w2m/xedref.py) so the two compare directly:
  R:<reg>  M:<seg>:<base>:<index>:<scale>:<disp>:<bytes>  A:<base>:<index>:<scale>:<disp>
  I:<width>:<hex>  J:<disp>
`normalize` maps both sides to the compared form: segment kept only for FS/GS overrides (in 64-bit
mode CS/DS/ES/SS overrides have no effect, SDM Vol 1 3.4.2.1), scale dropped when there is no index.
"""

from __future__ import annotations

from w2m.x86dec import Fields

GPR64 = ["RAX", "RCX", "RDX", "RBX", "RSP", "RBP", "RSI", "RDI"] + [f"R{n}" for n in range(8, 16)]
GPR32 = ["EAX", "ECX", "EDX", "EBX", "ESP", "EBP", "ESI", "EDI"] + [f"R{n}D" for n in range(8, 16)]
GPR16 = ["AX", "CX", "DX", "BX", "SP", "BP", "SI", "DI"] + [f"R{n}W" for n in range(8, 16)]
GPR8_REX = ["AL", "CL", "DL", "BL", "SPL", "BPL", "SIL", "DIL"] + [f"R{n}B" for n in range(8, 16)]
GPR8_LEGACY = ["AL", "CL", "DL", "BL", "AH", "CH", "DH", "BH"]
SIZE = {"b": 8, "w": 16, "d": 32, "q": 64, "dq": 128}
XMM = [f"XMM{n}" for n in range(16)]
YMM = [f"YMM{n}" for n in range(16)]
SEG = {0x64: "FS", 0x65: "GS"}


class OperandError(ValueError):
    """The row has no operand template, or the template uses notation this module lacks."""


def _reg(n: int, bits: int, rex: int) -> str:
    if bits == 8:
        return GPR8_REX[n] if rex else GPR8_LEGACY[n]
    return {64: GPR64, 32: GPR32, 16: GPR16}[bits][n]


def _bits(letter: str, f: Fields) -> int:
    if letter == "z":
        return 16 if f.opsize == 16 else 32
    if letter == "y":                          # d, or q with REX.W (a 66 mandatory prefix does not narrow it)
        return 64 if f.rex >> 3 & 1 else 32
    return f.opsize if letter == "v" else SIZE[letter]


def _mem(f: Fields, nbytes: int | None) -> str:
    assert f.modrm is not None
    mod, rm = f.modrm >> 6, f.modrm & 7
    names = GPR64 if f.asize == 64 else GPR32
    base, index, scale = "INVALID", "INVALID", 1
    if rm == 4:
        assert f.sib is not None
        scale = 1 << (f.sib >> 6)
        ix = ((f.sib >> 3) & 7) | ((f.rex >> 1) & 1) << 3
        if ix != 4:
            index = names[ix]
        if not ((f.sib & 7) == 5 and mod == 0):
            base = names[(f.sib & 7) | (f.rex & 1) << 3]
    elif mod == 0 and rm == 5:
        base = "RIP" if f.asize == 64 else "EIP"
    else:
        base = names[rm | (f.rex & 1) << 3]
    segs = [SEG[p] for p in f.prefixes if p in SEG]
    if nbytes is None:
        return f"A:{base}:{index}:{scale}:{f.disp}"
    return f"M:{segs[-1] if segs else 'INVALID'}:{base}:{index}:{scale}:{f.disp}:{nbytes}"


def operands(f: Fields) -> tuple[str, ...]:
    """The explicit operands, in template order."""
    if not f.ops:
        raise OperandError("row has no operand template")
    out: list[str] = []
    vec = YMM if f.vl else XMM                 # VEX.L (Sprint 049): Vx, Hx, Wx name YMM registers at 256 bits
    if f.ops == "-":                           # no explicit operand (FSIN, FNCLEX, FWAIT, FNSTSW AX; Sprint 065)
        return ()
    for t in f.ops.split(","):
        if t.startswith("Mf"):                 # x87 memory of the template's bytes (Mf2 .. Mf10; Sprint 065)
            assert f.modrm is not None and f.modrm >> 6 != 3
            out.append(_mem(f, int(t[2:])))
            continue
        if t == "STi":                         # ST(i) from ModRM.r/m (Sprint 065)
            assert f.modrm is not None
            out.append(f"ST:{f.modrm & 7}")
            continue
        if t in ("P", "N") or t[0] == "Q":       # MMX (SDM Table A-1; Sprint 065): P from ModRM.reg, N from r/m, Q from
            assert f.modrm is not None          # r/m or memory; REX.R and REX.B do not extend an MMX register
            if t == "P":
                out.append(f"MM:{f.modrm >> 3 & 7}")
            elif f.modrm >> 6 == 3:
                out.append(f"MM:{f.modrm & 7}")
            else:
                out.append(_mem(f, SIZE[t[1:]] // 8))
            continue
        if t == "Vx":                          # XMM or YMM from ModRM.reg, by VEX.L (SDM Table A-1 "x")
            assert f.modrm is not None
            out.append("R:" + vec[((f.modrm >> 3) & 7) | ((f.rex >> 2) & 1) << 3])
        elif t[0] == "B":                      # a general register VEX.vvvv names (BMI: ANDN, MULX, SHLX; Sprint 049)
            assert f.vvvv is not None
            out.append("R:" + _reg(f.vvvv ^ 0xF, _bits(t[1], f), 1))
        elif t in ("H", "Hx"):                 # the register VEX.vvvv names (stored inverted); H alone is XMM
            assert f.vvvv is not None
            out.append("R:" + (vec if t == "Hx" else XMM)[f.vvvv ^ 0xF])
        elif t == "Wx":                        # XMM or YMM from ModRM.r/m, or 16 or 32 bytes of memory
            assert f.modrm is not None
            if f.modrm >> 6 == 3:
                out.append("R:" + vec[(f.modrm & 7) | (f.rex & 1) << 3])
            else:
                out.append(_mem(f, 32 if f.vl else 16))
        elif t == "V":                         # XMM register from ModRM.reg (Sprint 039)
            assert f.modrm is not None
            out.append("R:" + XMM[((f.modrm >> 3) & 7) | ((f.rex >> 2) & 1) << 3])
        elif t == "Edb":                       # r32 or m8 from ModRM.r/m (PINSRB; Sprint 050)
            assert f.modrm is not None
            if f.modrm >> 6 == 3:
                out.append("R:" + _reg((f.modrm & 7) | (f.rex & 1) << 3, 32, f.rex))
            else:
                out.append(_mem(f, 1))
        elif t[0] == "W":                      # XMM register or memory of SIZE[t[1:]] bits from ModRM.r/m
            assert f.modrm is not None
            if f.modrm >> 6 == 3:
                out.append("R:" + XMM[(f.modrm & 7) | (f.rex & 1) << 3])
            else:
                out.append(_mem(f, SIZE[t[1:]] // 8))
        elif t[0] == "E":
            bits = _bits(t[1], f)
            assert f.modrm is not None
            if f.modrm >> 6 == 3:
                out.append("R:" + _reg((f.modrm & 7) | (f.rex & 1) << 3, bits, f.rex))
            else:
                out.append(_mem(f, bits // 8))
        elif t[0] == "G":
            assert f.modrm is not None
            out.append("R:" + _reg(((f.modrm >> 3) & 7) | ((f.rex >> 2) & 1) << 3, _bits(t[1], f), f.rex))
        elif t[0] == "Z":
            out.append("R:" + _reg((f.opcode & 7) | (f.rex & 1) << 3, _bits(t[1], f), f.rex))
        elif t == "M":
            out.append(_mem(f, None))
        elif t == "Mcl":                       # a cache line of memory, 64 bytes (PREFETCHh; XED, Sprint 044)
            out.append(_mem(f, 64))
        elif t == "Mqdq":                      # 8 bytes, 16 with a 64-bit operand size (CMPXCHG8B/16B; Sprint 046)
            out.append(_mem(f, 16 if f.opsize == 64 else 8))
        elif t[0] == "I":
            out.append(f"I:{len(f.imm)}:{int.from_bytes(f.imm, 'little'):x}")
        elif t[0] == "J":
            out.append(f"J:{int.from_bytes(f.imm, 'little', signed=True)}")
        elif t == "1":
            out.append("I:1:1")
        elif t in ("AL", "CL"):
            out.append("R:" + t)
        elif t == "rAX":
            out.append("R:" + _reg(0, f.opsize, f.rex))
        else:
            raise OperandError(f"operand notation {t!r} not handled")
    return tuple(out)


def normalize(op: str) -> str:
    """The compared form of one operand string (either side)."""
    p = op.split(":")
    if p[0] == "M":
        seg = p[1] if p[1] in ("FS", "GS") else "-"
        scale = p[4] if p[3] != "INVALID" else "-"
        return ":".join(["M", seg, p[2], p[3], scale, p[5], p[6]])
    if p[0] == "A":
        return ":".join(["A", p[1], p[2], p[3] if p[2] != "INVALID" else "-", p[4]])
    return op
