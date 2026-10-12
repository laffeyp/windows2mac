# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""lift.py — x86-64 instructions to explicit operations (v0.2 §5 machine-semantics IR; Sprint 029).

Each decoded instruction (src/w2m/x86dec.py) with its operands (src/w2m/x86ops.py, equal to
XED's on every compared corpus position) becomes a short list of operations over numbered
temporaries. The operations are target-independent: `runtime.py` executes them on the host
(profile `model`); an AArch64 back end for the Hypervisor.framework VM consumes the same list.

Operations (t* are temporaries, loc is ("r", index, bits, high8) or ("m", t_addr, bytes)):
  ("addr", t, base, index, scale, disp, abits, seg)   effective address (plus the GS base when seg is
                                                "GS": the thread's TEB, Sprint 044); base/index are register
                                                  indices, "rip" resolved at lift time, or None
  ("imm", t, value)        ("ld", t, loc)        ("st", loc, t)
  ("alu", t, op, a, b, bits)      op: add sub and or xor shr mul_lo mul_hi sext32
  ("flags", kind, a, b, r, bits)  kind: add sub logic inc dec shr mul (SDM Vol 2 flag tables)
  ("br", cond, t)                 cond: always or an SDM condition code (Vol 1 B.1)
  ("set", cond, loc)              SETcc
  ("df", 0|1)                     CLD, STD
  ("cpuid",)                      CPUID: EAX, EBX, ECX, EDX from specs/guest_cpuid.json for EAX, ECX
  ("sel", t, c, x, y)             t = x if temporary c is nonzero, else y
  ("cst", c, loc, t)              store t to loc when temporary c is nonzero
  ("cstz", c, loc, t)             store t to loc when temporary c is zero
  ("cmov", cond, loc, t)          CMOVcc: store when the condition holds; a 32-bit register destination
                                  is zero-extended either way (SDM CMOVcc, 64-bit mode)
  alu ops (Sprint 041): shl sar rol ror (count masked as the SDM says), smul_lo smul_hi (signed
  product of `bits`-wide operands, low and high halves)
  alu ops (Sprint 044): sext8 sext16 (sign-extend the low 8 or 16 bits), bswap (byte order, 32 or 64
  bits), sbb (a - b - CF; reads CF, which must be defined); flags kinds sbb (SDM SBB) and bt (SDM BT:
  CF the bit, OF SF AF PF undefined, ZF unchanged)
  ("rep", kind, size)              REP STOS / REP MOVS (SDM; Vol 1 7.3.9): while RCX != 0, store AL/AX/EAX/RAX
                                  (stos) or the element at [RSI] (movs) to [RDI], step RDI (and RSI) by
                                  size, down when DF is set, and decrement RCX; interruptible, so a fault
                                  leaves the registers at the faulting iteration and RIP at the
                                  instruction (the one exception to the commit rule)
  ("mxcsr", "st"|"ld", loc)       STMXCSR / LDMXCSR (SDM): MXCSR to or from 4 bytes of memory; LDMXCSR with
                                  a reserved bit (16-31) set faults before any write (Sprint 044)
  ("fence", kind)                 SFENCE "st": earlier stores before later ones; LFENCE "ld": earlier loads
                                  before later accesses; MFENCE "full": every access (Sprint 047)
  ("fcw", "st", loc)               FNSTCW (Sprint 049): the x87 control word to 2 bytes of memory
  ("x87", ic, opcode, reg, st, mem) one x87 instruction, run whole by the model (src/w2m/x87.py; Sprint 065)
  ("mmx", ic, dst, src, imm)        one MMX-register instruction, run whole by the model (src/w2m/mmx.py; Sprint 065):
                                    dst and src ("mm", n), ("x", n, bits), ("r", idx, bits, hi), ("m", t, bytes),
                                    ("i",) for an immediate count, ("none",)
  ("ud",)                         an instruction the guest profile makes undefined (#UD)
  ("xgetbv",)                     XGETBV (Sprint 049): EDX:EAX = XCR0 (7) for ECX 0, #GP otherwise
  ("vex", base, d, s1, s2, L, align, imm)  a VEX instruction as its legacy-SSE base (Sprint 049; rules/_vex.py): d =
                                  base(s1, s2) on each 128-bit half; VEX.128 zeroes a register destination's bits
                                  255:128; s1 None for two-operand forms
  ("fma", ic, d, s2, s3, L)       VFMADD/VFMSUB/VFNMADD/VFNMSUB 132/213/231 (Sprint 049): one rounding
  ("vzeroupper",)                 VZEROUPPER: every YMM's bits 255:128 to zero
  ("div", bits, t[, signed])      DIV r/m (unsigned), IDIV with signed True (Sprint 046): RDX:RAX (DX:AX, EDX:EAX;
                                  AX for 8 bits) by t; quotient
                                  to RAX/EAX/AX/AL, remainder to RDX/EDX/DX/AH; every flag undefined;
                                  a zero divisor faults STATUS_INTEGER_DIVIDE_BY_ZERO, a quotient too wide
                                  STATUS_INTEGER_OVERFLOW (SDM DIV #DE; ntstatus.h), before any write
  ("sse", iclass, dst, src, align[, imm])  one legacy-SSE instruction (Sprint 039; imm, Sprint 044: PSHUFD's
                                  order, a shift count): dst and src are locs,
                                  ("x", index) for an XMM register; align 16 when a memory operand
                                  must be 16-byte aligned (#GP otherwise); the runtime computes it
                                  with x86fp, after every read and before any write
  ("x86", ic, dst, srcs, bits)    one integer instruction the model computes whole (Sprint 050; integer_model.py):
                                  dst a loc or None, srcs the temporaries it reads (operand values, a branch
                                  target); implicit operands (RSI, RDI, RCX, the stack, the flags) are read by the
                                  model; the VM runs it as one model step (hvc #11)
Semantics follow Intel SDM Vol 2 instruction pages; a 32-bit register write zero-extends to 64
bits and 8/16-bit writes keep the other bits (SDM Vol 1 3.4.1.1).
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Any

from w2m.x86dec import Decoded, decode
from w2m.x86ops import GPR8_LEGACY, GPR8_REX, GPR16, GPR32, GPR64, operands

REG: dict[str, tuple[int, int, bool]] = {}
for _i, _n in enumerate(GPR64):
    REG[_n] = (_i, 64, False)
for _i, _n in enumerate(GPR32):
    REG[_n] = (_i, 32, False)
for _i, _n in enumerate(GPR16):
    REG[_n] = (_i, 16, False)
for _i, _n in enumerate(GPR8_REX):
    REG[_n] = (_i, 8, False)
for _i, _n in enumerate(GPR8_LEGACY[4:]):
    REG[_n] = (_i, 8, True)                  # AH, CH, DH, BH: bits 8-15 of registers 0-3
RSP, RAX, RDX = 4, 0, 2
RCX, RSI, RDI = 1, 6, 7
RBX = 3
# REP STOS and REP MOVS (Sprint 044): iclass -> (kind, element bytes)
REP_STRING = {f"REP_{k}{s}": (k.lower(), n) for k in ("STOS", "MOVS") for s, n in (("B", 1), ("W", 2), ("D", 4), ("Q", 8))}
CC = {"O": "o", "NO": "no", "B": "b", "NB": "nb", "Z": "z", "NZ": "nz", "BE": "be", "NBE": "nbe",
      "S": "s", "NS": "ns", "P": "p", "NP": "np", "L": "l", "NL": "nl", "LE": "le", "NLE": "nle"}
SSE_ALIGNED = {"MOVAPS", "MOVAPD", "XORPS", "XORPD", "ANDPS", "ANDPD"} | {
    f"{op}{t}" for op in ("ADD", "SUB", "MUL", "DIV", "SQRT", "MIN", "MAX") for t in ("PS", "PD")}
SSE = SSE_ALIGNED | {"MOVUPS", "MOVUPD", "MOVSS", "MOVSD_XMM", "MOVD", "MOVQ"} | {
    f"{op}{t}" for op in ("ADD", "SUB", "MUL", "DIV", "SQRT", "MIN", "MAX") for t in ("SS", "SD")} | {
    "CVTTSS2SI", "CVTTSD2SI", "CVTSS2SI", "CVTSD2SI", "CVTSI2SS", "CVTSI2SD", "CVTSS2SD", "CVTSD2SS",
    "UCOMISS", "UCOMISD", "COMISS", "COMISD"}
# Sprint 049: the legacy forms mono's VEX classes rest on; the packed ones need 16-byte alignment, CMPSD's m64 does not
SSE |= {"ORPD", "PCMPEQW", "CVTTPD2DQ", "CVTPD2DQ", "CMPSD_XMM", "HADDPD"}
SSE_ALIGNED |= {"ORPD", "PCMPEQW", "CVTTPD2DQ", "CVTPD2DQ", "HADDPD"}
# Sprint 044: the SSE2 integer block (operands V, W, G, Ib; no MXCSR flags); MOVDQA and MOVNTPS need
# 16-byte alignment (SDM: #GP otherwise)
SSE2_INT = {"MOVDQA", "MOVDQU", "MOVNTPS", "PAND", "POR", "ORPS", "PSUBD", "PSUBQ", "PSRLQ", "PSRLDQ", "PSLLDQ",
            "PUNPCKLBW", "PCMPEQB", "PSHUFD", "PMOVMSKB", "CVTDQ2PD"}
SSE_ALIGNED |= {"MOVDQA", "MOVNTPS", "PAND", "POR", "ORPS", "PSUBD", "PSUBQ", "PSRLQ", "PUNPCKLBW", "PCMPEQB",
                "PSHUFD"}
# Sprint 046: the second SSE block (no MXCSR flags); a legacy-SSE m128 operand needs 16-byte alignment (SDM:
# #GP otherwise); MOVHLPS, MOVLHPS and MOVMSKPS take registers only
SSE2_INT |= {'MOVMSKPS', 'PADDD', 'MOVLHPS', 'PSRAD', 'PSRLD', 'PSLLD', 'UNPCKHPS', 'ANDNPS', 'MOVHLPS', 'PXOR', 'PUNPCKLDQ', 'SHUFPS', 'UNPCKLPS'}
SSE_ALIGNED |= {'PSRAD', 'PSRLD', 'PSLLD', 'UNPCKHPS', 'ANDNPS', 'PUNPCKLDQ', 'PXOR', 'PADDD', 'SHUFPS', 'UNPCKLPS'}
# Sprint 046 (third block): the SSE2 integer classes UnityPlayer.dll reaches most; PINSRW reads 16 bits and MOVHPS
# 8, neither aligned
SSE2_INT |= {'PMULLW', 'PMAXUB', 'PSRLW', 'PCMPEQD', 'PUNPCKHDQ', 'UNPCKLPD', 'PSHUFHW', 'PSHUFLW', 'PADDW', 'PSUBUSB', 'PACKUSWB', 'PADDQ', 'PSLLQ', 'PUNPCKLQDQ', 'PCMPGTD', 'PUNPCKLWD', 'PADDSW', 'PSUBW', 'PADDSB', 'PUNPCKHWD', 'PSUBSB', 'PSRAW', 'PINSRW', 'PANDN', 'MOVHPS', 'PUNPCKHBW'}
SSE_ALIGNED |= {'PMULLW', 'PMAXUB', 'PSRLW', 'PCMPEQD', 'PUNPCKHDQ', 'UNPCKLPD', 'PSHUFHW', 'PSHUFLW', 'PADDW', 'PSUBUSB', 'PACKUSWB', 'PADDQ', 'PSLLQ', 'PUNPCKLQDQ', 'PCMPGTD', 'PUNPCKLWD', 'PADDSW', 'PSUBW', 'PADDSB', 'PUNPCKHWD', 'PSUBSB', 'PSRAW', 'PANDN', 'PUNPCKHBW'}
# Sprint 046 (fifth block): the approximations; the packed forms need 16-byte alignment
SSE2_INT |= {"RCPPS", "RSQRTPS", "RCPSS", "RSQRTSS"}
SSE_ALIGNED |= {"RCPPS", "RSQRTPS"}
# Sprint 047: SSE4.2's PCMPISTRI (ECX and the flags; its m128 needs no alignment, SDM Vol 2A §4.1.1)
SSE2_INT |= {"PCMPISTRI", "PCMPISTRI64"}
SSE2_INT |= {"UNPCKHPD"}
SSE_ALIGNED |= {"UNPCKHPD"}
# Sprint 050 (set 050): the remaining legacy-SSE classes of mono's and UnityPlayer's censuses (2026-10-11), run in the
# model for now (rules/_sse.py). Packed m128 operands need 16-byte alignment (SDM: #GP otherwise); the m64, m32 and m8
# forms (CVTPS2PD, CMPSS, MOVLPS, MOVHPD, PINSRB, PINSRD, PMOVSXDQ, PMOVZXWD) and PEXTRW's register do not
SSE_050 = {"CVTDQ2PS", "CVTPS2DQ", "CVTTPS2DQ", "CVTPS2PD", "CVTPD2PS", "CMPPS", "CMPPD", "CMPSS", "SHUFPD", "MOVLPS",
           "MOVHPD", "PSHUFB", "PALIGNR", "PSADBW", "PACKSSWB", "PACKSSDW", "PMADDUBSW", "PMADDWD", "PEXTRW", "PINSRB",
           "PINSRD", "PUNPCKHQDQ", "PMULHW", "PMULLD", "PADDUSB", "PADDB", "PAVGB", "PABSW", "PCMPGTB", "PCMPGTW",
           "PCMPEQQ", "PMAXUD", "PMAXUW", "PMAXSW", "PMINSD", "PMOVSXDQ", "PMOVZXWD", "PSLLW", "MOVNTDQ"}
SSE |= SSE_050
SSE_ALIGNED |= SSE_050 - {"CVTPS2PD", "CMPSS", "MOVLPS", "MOVHPD", "PINSRB", "PINSRD", "PMOVSXDQ", "PMOVZXWD", "PEXTRW"}
SSE |= SSE2_INT
# Sprint 050: integer classes whose operands are all implicit (flags, the stack, RSI and RDI)
IMPLICIT_050 = {"STC", "CLC", "CMC", "SAHF", "PUSHFQ", "POPFQ", "LEAVE", "PAUSE", "LODSB", "SCASB", "SCASD", "STOSB",
                "MOVSB", "MOVSD", "CMPSB", "CMPSD"}
ALU = {"ADD": "add", "SUB": "sub", "AND": "and", "OR": "or", "XOR": "xor", "CMP": "sub", "TEST": "and"}

Op = tuple[Any, ...]


class LiftError(ValueError):
    """The instruction decodes but has no lifting rule yet; the runtime stops on it."""


@dataclass(frozen=True)
class Lifted:
    address: int
    length: int
    iclass: str
    ops: tuple[Op, ...]


class _OpBuilder:
    def __init__(self) -> None:
        self.ops: list[Op] = []
        self.n = 0

    def t(self) -> int:
        self.n += 1
        return self.n

    def emit(self, *op: object) -> None:
        self.ops.append(op)


def _operand(b: _OpBuilder, s: str, next_ip: int, width_hint: int | None = None) -> tuple[str, object, int]:
    """Parse one xedshim-format operand; return (kind, loc-or-temp, bits)."""
    p = s.split(":")
    if p[0] == "R" and p[1].startswith("XMM"):
        return "loc", ("x", int(p[1][3:])), 128
    if p[0] == "R" and p[1].startswith("YMM"):                 # VEX.256 (Sprint 049): the XMM register is its low half
        return "loc", ("x", int(p[1][3:])), 256
    if p[0] == "R":
        idx, bits, hi = REG[p[1]]
        return "loc", ("r", idx, bits, hi), bits
    if p[0] in ("M", "A"):
        base, index, scale, disp = (p[2], p[3], p[4], p[5]) if p[0] == "M" else (p[1], p[2], p[3], p[4])
        t = b.t()
        abits = 32 if base.startswith("E") or (base == "INVALID" and index.startswith("E")) else 64
        seg = p[1] if p[0] == "M" and p[1] in ("FS", "GS") else None
        if seg == "FS":                    # Windows x64 user code reaches the TEB through GS (Sprint 044)
            raise LiftError(f"FS-relative memory operand {s}: the FS base is not modeled")
        if base in ("RIP", "EIP"):
            b.emit("addr", t, None, None, 1, next_ip + int(disp), abits, seg)
        else:
            b.emit("addr", t, None if base == "INVALID" else REG[base][0],
                   None if index == "INVALID" else REG[index][0], int(scale), int(disp), abits, seg)
        if p[0] == "A":
            return "addr", t, abits
        return "loc", ("m", t, int(p[6])), int(p[6]) * 8
    if p[0] == "I":
        width, raw = int(p[1]) * 8, int(p[2], 16)
        t = b.t()
        target = width_hint or width
        if width < target and raw >> (width - 1) & 1:          # imm sign-extends to operand size
            raw |= ((1 << target) - 1) ^ ((1 << width) - 1)
        b.emit("imm", t, raw)
        return "imm", t, width
    if p[0] == "J":
        t = b.t()
        b.emit("imm", t, (next_ip + int(p[1])) & (1 << 64) - 1)
        return "imm", t, 64
    raise LiftError(f"operand {s}")


def _val(b: _OpBuilder, kind: str, x: object) -> int:
    if kind == "loc":
        t = b.t()
        b.emit("ld", t, x)
        return t
    assert isinstance(x, int)
    return x


def _push(b: _OpBuilder, value_t: int, size: int = 8) -> None:
    """RSP -= size; store `size` bytes at the new RSP (size 2 for a 66-prefixed PUSH)."""
    sp, step, nsp = b.t(), b.t(), b.t()
    b.emit("ld", sp, ("r", RSP, 64, False))
    b.emit("imm", step, size)
    b.emit("alu", nsp, "sub", sp, step, 64)
    b.emit("st", ("m", nsp, size), value_t)          # the store, the only fault point, before RSP moves
    b.emit("st", ("r", RSP, 64, False), nsp)


def _pop(b: _OpBuilder) -> int:
    sp, eight, nsp, v = b.t(), b.t(), b.t(), b.t()
    b.emit("ld", sp, ("r", RSP, 64, False))
    b.emit("ld", v, ("m", sp, 8))
    b.emit("imm", eight, 8)
    b.emit("alu", nsp, "add", sp, eight, 64)
    b.emit("st", ("r", RSP, 64, False), nsp)
    return v


def commit_order(ins: Lifted) -> str | None:
    """None when `ins` obeys the commit rule (specs/guest_state.json 0.2.0, review 2026-10-09 N2):
    no register, flag or DF write before the instruction's last memory access, and at most one
    memory store, after every memory load. A fault then leaves the guest state as it was before
    the instruction, so the reconstruction table maps it to the instruction's start. Otherwise the
    first violation, named."""
    def memory_access(op: Op) -> str | None:
        if op[0] == "ld" and op[2][0] == "m":
            return "load"
        if op[0] == "st" and op[1][0] == "m":
            return "store"
        if op[0] == "set" and op[2][0] == "m":
            return "store"
        if op[0] in ("cst", "cstz") and op[2][0] == "m":
            return "store"
        if op[0] == "sse" and (op[2][0] == "m" or op[3][0] == "m"):
            return "store" if op[2][0] == "m" else "load"
        if op[0] == "mxcsr":
            return "store" if op[1] == "st" else "load"
        if op[0] == "fcw":
            return "store"
        if op[0] == "mmx":                                    # ("mmx", ic, dst, src, imm): Sprint 065
            return "store" if op[2][0] == "m" else "load" if op[3][0] == "m" else None
        if op[0] == "x87":                                    # ("x87", ic, opcode, reg, st, mem): Sprint 065
            if op[5] is None:
                return None
            return "store" if op[1] in ("FSTP", "FISTP", "FISTTP", "FNSTSW") else "load"
        if op[0] == "vexs":                                   # ("vexs", ic, d, s1, s2, L, imm)
            return "store" if op[2][0] == "m" else "load" if op[4][0] == "m" else None
        if op[0] == "vex":                                    # ("vex", base, d, s1, s2, L, align, imm)
            return "store" if op[2][0] == "m" else "load" if op[4][0] == "m" else None
        if op[0] == "fma":                                    # ("fma", ic, d, s2, s3, L)
            return "load" if op[4][0] == "m" else None
        if op[0] == "cx16":                                   # one atomic read-and-write (CMPXCHG16B)
            return "store"
        if op[0] == "x86":                                    # ("x86", ic, dst, sources, bits): the model's (Sprint 050)
            return "store" if op[2] is not None and op[2][0] == "m" else None
        return None

    def effect(op: Op) -> bool:
        return (op[0] == "st" and op[1][0] == "r") or op[0] in ("flags", "df", "cmov", "cpuid", "div") or (
            op[0] in ("set", "cst", "cstz") and op[2][0] == "r") or (op[0] == "sse" and op[2][0] != "m") or (
            op[0] == "mxcsr" and op[1] == "ld") or (op[0] in ("vex", "vexs") and op[2][0] != "m") or op[0] in ("fma", "vzeroupper", "x86")

    last_access = max((i for i, op in enumerate(ins.ops) if memory_access(op)), default=-1)
    if last_access < 0:
        return None
    stores = [i for i, op in enumerate(ins.ops) if memory_access(op) == "store"]
    if len(stores) > 1:
        return f"{ins.iclass} at 0x{ins.address:x}: {len(stores)} memory stores"
    if stores and stores[0] != last_access:
        return f"{ins.iclass} at 0x{ins.address:x}: a memory access after the store"
    for i, op in enumerate(ins.ops[:last_access]):
        if effect(op):
            return f"{ins.iclass} at 0x{ins.address:x}: {op[0]} at operation {i} before memory access {last_access}"
    return None


MMX_ONLY = ("EMMS", "MOVDQ2Q")                   # classes with no SSE form (Sprint 065)


def lift_mmx(b: _OpBuilder, ic: str, ops: tuple[str, ...], nxt: int) -> None:
    """("mmx", ic, dst, src, imm): the operands as src/w2m/mmx.py reads them."""
    out: list[tuple[object, ...]] = []
    imm = 0
    for o in ops:
        if o.startswith("MM:"):
            out.append(("mm", int(o[3:])))
        elif o.startswith("I:"):
            imm = int(o.split(":")[2], 16)
            out.append(("i",))
        else:
            _, loc, _ = _operand(b, o, nxt)
            out.append(loc)                                   # type: ignore[arg-type]
    if ic == "PEXTRW":
        out = out[:2]
    while len(out) < 2:
        out.append(("none",))
    b.emit("mmx", ic, out[0], out[1], imm)


def lift(code: bytes, address: int) -> Lifted:
    """Lift the instruction at the start of `code`, located at guest `address`."""
    d: Decoded = decode(code)
    f = d.fields
    ic = d.iclass
    nxt = address + d.length
    b = _OpBuilder()
    ops = operands(f) if f is not None and f.ops else ()
    declared_none = f is not None and f.ops == "-"       # the spec says the row has no explicit operand (Sprint 065)
    if not ops and not declared_none and ic not in ("NOP", "RET_NEAR", "CLD", "STD", "CPUID", "CDQ", "CDQE", "CQO", "CWDE", "SFENCE", "LFENCE", "MFENCE", "XGETBV", "VZEROUPPER", "VZEROALL",
                              *REP_STRING, *IMPLICIT_050):
        # a row without an operand template (moffs MOV A0-A3, implicit forms): not lifted yet
        raise LiftError(f"no operand template for {ic} at 0x{address:x} ({code[:d.length].hex()})")
    from w2m.rules import Ctx, registry               # the rule for each class (src/w2m/rules, Sprint 041)

    if ic in MMX_ONLY or any(o.startswith("MM:") for o in ops):   # an MMX-register form, whatever its class's SSE rule
        lift_mmx(b, ic, ops, nxt)
        return Lifted(address, d.length, ic, tuple(b.ops))

    found = registry().get(ic)
    if found is None:
        raise LiftError(f"no lifting rule for {ic} at 0x{address:x}")
    found.rule(Ctx(b, ic, f, ops, nxt, address))
    return Lifted(address, d.length, ic, tuple(b.ops))
