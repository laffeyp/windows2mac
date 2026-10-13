# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""runtime.py — execute lifted x86-64 operations on the host, with kernel32 serviced in the host
(Sprint 029; profile_kind `model`). One guest thread. The executor runs `lift.py` operations; a
branch to a loader thunk calls the named kernel32 function (src/w2m/kernel32.py), which returns to the
caller as RET would. Since Sprint 030 it is also the reference the managed-VM back end is compared
against, one instruction at a time (`step`).
"""

from __future__ import annotations

import re
import struct

from collections.abc import Callable
from dataclasses import dataclass, field
from typing import Any

from w2m import cpuid
from w2m import approximate, string_compare, x86fp as fp
from w2m import integer_model
from w2m.kernel32 import M64, HandlerFailed, Kernel32, Thread, Unimplemented, monotonic_count
from w2m.lift import RAX, RBX, RCX, RDI, RDX, RSI, SSE2_INT, Lifted, LiftError, Op, lift
from w2m.loader import GuestFault, Loaded

STATUS_INTEGER_DIVIDE_BY_ZERO = 0xC000_0094   # ntstatus.h; DIV #DE with a zero divisor
STATUS_INTEGER_OVERFLOW = 0xC000_0095         # ntstatus.h; DIV #DE with a quotient too wide
STATUS_ILLEGAL_INSTRUCTION = 0xC000_001D      # ntstatus.h; #UD
XCR0 = 7                                      # x87 | SSE | AVX enabled (specs/guest_cpuid.json 0.2.0, leaf 0xD)
# Sprint 049 (set 049b): the legacy forms UnityPlayer's VEX classes rest on (some, such as PERMILPS_VAR and PBLENDD,
# exist only in VEX form; the names are the model's)
SSE_049B = {"PSHUFB", "PALIGNR", "PINSRD", "PINSRQ", "PEXTRQ", "PMOVSXDQ", "ANDNPD", "CVTDQ2PS", "CVTTPS2DQ",
            "CVTPS2PD", "CMPPS_VEX", "ROUNDSD", "PERMILPS_VAR", "PBLENDD"}
SHIFT_BY_COUNT = {"PSRLQ", "PSLLQ", "PSRLD", "PSLLD", "PSRAD", "PSRLW", "PSRAW"}   # VEX.256: one count, both halves
VEX_FP = {"HADDPD", "CVTTPD2DQ", "CVTPD2DQ", "CMPPS_VEX", "CVTDQ2PS", "CVTTPS2DQ", "CVTPS2PD", "ROUNDSD"}  # FP bases beyond ADD..MAX

__all__ = ["Result", "Runtime", "UndefinedFlag", "Unimplemented"]


class UndefinedFlag(RuntimeError):
    """An instruction read a flag the SDM leaves undefined after the instruction that set it."""


@dataclass
class Result:
    exit_code: int | None
    stdout: bytes
    instructions_executed: int
    distinct_addresses: int
    calls: list[str]
    stop: str
    calls_total: dict[str, int] = field(default_factory=dict)   # every call by service (VM: past the ring too)
    calls_dropped: int = 0                                       # calls the ring could not record in order


def _mask(bits: int) -> int:
    return (1 << bits) - 1


def _parity(v: int) -> bool:
    return bin(v & 0xFF).count("1") % 2 == 0


def _signed(v: int, w: int) -> int:
    return v - (1 << w) if v >> (w - 1) else v


# Sprint 046: the third SSE block's lane-by-lane classes: (lane bits, f(dest lane, source lane, bits))
LANE_ARITH: dict[str, tuple[int, Callable[[int, int, int], int]]] = {
    "PADDW": (16, lambda p, q, w: p + q), "PADDQ": (64, lambda p, q, w: p + q), "PSUBW": (16, lambda p, q, w: p - q),
    "PCMPEQD": (32, lambda p, q, w: _mask(w) if p == q else 0),
    "PCMPGTD": (32, lambda p, q, w: _mask(w) if _signed(p, w) > _signed(q, w) else 0),
    "PSUBUSB": (8, lambda p, q, w: max(p - q, 0)), "PMAXUB": (8, lambda p, q, w: max(p, q)),
    "PADDSB": (8, lambda p, q, w: max(-(1 << (w - 1)), min((1 << (w - 1)) - 1, _signed(p, w) + _signed(q, w)))),
    "PADDSW": (16, lambda p, q, w: max(-(1 << (w - 1)), min((1 << (w - 1)) - 1, _signed(p, w) + _signed(q, w)))),
    "PSUBSB": (8, lambda p, q, w: max(-(1 << (w - 1)), min((1 << (w - 1)) - 1, _signed(p, w) - _signed(q, w)))),
    "PMULLW": (16, lambda p, q, w: p * q),
}
# interleaves: (lane bits, from the high half)
LANE_ARITH["PADDB"] = (8, lambda p, q, w: p + q)                                      # Sprint 049
LANE_ARITH["PCMPEQQ"] = (64, lambda p, q, w: _mask(w) if p == q else 0)
LANE_ARITH["PMULUDQ"] = (64, lambda p, q, w: (p & 0xFFFF_FFFF) * (q & 0xFFFF_FFFF))
# Sprint 050 (set 050): the remaining lane-by-lane classes (SDM, each page); PABSW reads only the source
LANE_ARITH["PADDUSB"] = (8, lambda p, q, w: min(p + q, 0xFF))
LANE_ARITH["PAVGB"] = (8, lambda p, q, w: (p + q + 1) >> 1)
LANE_ARITH["PCMPGTB"] = (8, lambda p, q, w: _mask(w) if _signed(p, w) > _signed(q, w) else 0)
LANE_ARITH["PCMPGTW"] = (16, lambda p, q, w: _mask(w) if _signed(p, w) > _signed(q, w) else 0)
LANE_ARITH["PMAXUD"] = (32, lambda p, q, w: max(p, q))
LANE_ARITH["PMAXUW"] = (16, lambda p, q, w: max(p, q))
LANE_ARITH["PMAXSW"] = (16, lambda p, q, w: max(_signed(p, w), _signed(q, w)))
LANE_ARITH["PMINSD"] = (32, lambda p, q, w: min(_signed(p, w), _signed(q, w)))
LANE_ARITH["PMULHW"] = (16, lambda p, q, w: (_signed(p, w) * _signed(q, w)) >> 16)
LANE_ARITH["PMULLD"] = (32, lambda p, q, w: p * q)
LANE_ARITH["PABSW"] = (16, lambda p, q, w: abs(_signed(q, w)))
SSE_050_NEW = {"CVTPS2DQ", "CVTPD2PS", "CMPPS", "CMPPD", "CMPSS", "SHUFPD", "MOVLPS", "MOVHPD", "MOVNTDQ", "PSADBW",
           "PACKSSWB", "PACKSSDW", "PMADDUBSW", "PMADDWD", "PEXTRW", "PINSRB", "PMOVZXWD", "PSLLW", "MPSADBW"}
INTERLEAVE = {"PUNPCKHQDQ": (64, True), "PUNPCKLQDQ": (64, False), "UNPCKLPD": (64, False), "UNPCKHPD": (64, True), "PUNPCKHBW": (8, True), "PUNPCKHDQ": (32, True),
              "PUNPCKHWD": (16, True), "PUNPCKLWD": (16, False)}


class Runtime:
    def __init__(self, loaded: Loaded, tid: int = 4,
                 clock: Callable[[], int] = monotonic_count) -> None:
        self.ld = loaded
        self.mem = loaded.memory
        self.th = Thread(tid)
        self.th.rip = loaded.entry
        self.th.regs[4] = loaded.stack_top - 0x100 - 8           # entry as if called: RSP % 16 == 8
        self.cache: dict[int, Lifted] = {}
        self.kernel = Kernel32(self.mem, clock)
        self.kernel32 = self.kernel.handlers
        self.steps = 0
        self.seen: set[int] = set()
        self.cpuid_reads: list[tuple[int, int]] = []           # (EAX, ECX) per CPUID executed

    @property
    def exit_code(self) -> int | None:
        return self.kernel.exit_code

    @property
    def calls(self) -> list[str]:
        return self.kernel.calls

    # registers -------------------------------------------------------------------------------
    def _rd(self, idx: int, bits: int, hi: bool) -> int:
        v = self.th.regs[idx]
        return (v >> 8) & 0xFF if hi else v & _mask(bits)

    def _wr(self, idx: int, bits: int, hi: bool, v: int) -> None:
        old = self.th.regs[idx]
        if hi:
            self.th.regs[idx] = (old & ~0xFF00) | ((v & 0xFF) << 8)
        elif bits == 32:
            self.th.regs[idx] = v & 0xFFFFFFFF                    # zero-extends (SDM 1 3.4.1.1)
        elif bits == 64:
            self.th.regs[idx] = v & M64
        else:
            self.th.regs[idx] = (old & ~_mask(bits) & M64) | (v & _mask(bits))

    def _x87(self, op: Op, ins: Lifted) -> None:
        """One x87 instruction on the thread's register stack (src/w2m/x87.py; Sprint 065). An exception the control
        word does not mask has no rows and stops the run by name."""
        from w2m import x87
        _, ic, opcode, reg, st, mem = op
        th = self.th
        fpu = x87.Fpu(cw=th.fcw, sw=th.x87_sw, regs=list(th.x87_regs))
        fpu.set_abridged_tags(th.x87_tags)
        address = self._t[mem[1]] if mem is not None else 0

        def read(n: int) -> int:
            return int.from_bytes(self.mem.read(address, n), "little")

        def write(n: int, v: int, offset: int = 0) -> None:
            self.mem.write(address + offset, (v & _mask(8 * n)).to_bytes(n, "little"))

        ax = [th.regs[0] & 0xFFFF]
        try:
            x87.execute(fpu, ic, opcode, reg, st, read, write, ax)
        except x87.X87Unmasked as e:
            raise Unimplemented(f"{ic} at 0x{ins.address:x}: {e}") from None
        th.fcw, th.x87_sw, th.x87_regs, th.x87_tags = fpu.cw, fpu.sw, fpu.regs, fpu.abridged_tags
        if ic == "FNSTSW" and mem is None:
            th.regs[0] = th.regs[0] & ~0xFFFF | ax[0]

    def _mmx(self, op: Op, ins: Lifted) -> None:
        """One MMX-register instruction on the thread's x87 register file (src/w2m/mmx.py; Sprint 065)."""
        from w2m import mmx, x87
        _, ic, dst, src, imm = op
        th = self.th
        fpu = x87.Fpu(cw=th.fcw, sw=th.x87_sw, regs=list(th.x87_regs))
        fpu.set_abridged_tags(th.x87_tags)

        def read(o: tuple[Any, ...]) -> int:
            if o[0] == "x":
                return int(th.xmm[o[1]])
            if o[0] == "r":
                return self._rd(o[1], o[2], o[3])
            return int.from_bytes(self.mem.read(self._t[o[1]], o[2]), "little")

        def write(o: tuple[Any, ...], v: int) -> None:
            if o[0] == "r":
                self._wr(o[1], o[2], o[3], v)
            else:
                self.mem.write(self._t[o[1]], (v & _mask(8 * o[2])).to_bytes(o[2], "little"))

        try:
            mmx.execute(fpu, ic, 0, dst, src, imm, read, write)
        except x87.X87Unmasked as e:
            raise Unimplemented(f"{ic} at 0x{ins.address:x}: {e}") from None
        finally:                                                  # a fault keeps TOP 0 and the tags (set 065b)
            th.x87_sw, th.x87_regs, th.x87_tags = fpu.sw, fpu.regs, fpu.abridged_tags

    def _load(self, loc: Op, t: dict[int, int]) -> int:
        if loc[0] == "r":
            return self._rd(loc[1], loc[2], loc[3])
        return int.from_bytes(self.mem.read(t[loc[1]], loc[2]), "little")

    def _store(self, loc: Op, v: int) -> None:
        if loc[0] == "r":
            self._wr(loc[1], loc[2], loc[3], v)
        else:
            n = loc[2]
            self.mem.write(self._t[loc[1]], (v & _mask(8 * n)).to_bytes(n, "little"))

    # flags -----------------------------------------------------------------------------------
    def _flag(self, f: str) -> bool:
        v = self.th.flags[f]
        if v is None:
            raise UndefinedFlag(f"{f} read while undefined at 0x{self.th.rip:x}")
        return v

    def _cond(self, cc: str) -> bool:
        if cc == "always":
            return True
        neg = cc.startswith("n")                                  # nb, nz, nbe, ...: negations
        base = {"o": lambda: self._flag("O"), "b": lambda: self._flag("C"),
                "z": lambda: self._flag("Z"), "be": lambda: self._flag("C") or self._flag("Z"),
                "s": lambda: self._flag("S"), "p": lambda: self._flag("P"),
                "l": lambda: self._flag("S") != self._flag("O"),
                "le": lambda: self._flag("Z") or self._flag("S") != self._flag("O")}
        key = cc[1:] if neg else cc
        r = base[key]()
        return not r if neg else r

    def _set_flags(self, kind: str, a: int, b: int, r: int, bits: int) -> None:
        m, sign = _mask(bits), 1 << (bits - 1)
        res = r & m
        f = self.th.flags
        if kind == "mul":                                         # SDM MUL: SF ZF AF PF undefined
            f.update(C=res != 0, O=res != 0, Z=None, S=None, P=None, A=None)
            return
        cnt = b & (0x3F if bits == 64 else 0x1F)                  # shift and rotate counts are masked
        if kind in ("shr", "shl", "sar", "rol", "ror") and cnt == 0:
            return                                                # SDM: a zero count affects no flag
        if kind == "shr":
            # SDM SHR: CF undefined when the count >= the operand size (Sprint 041; the model had
            # defined it); OF only for a 1-bit shift; AF undefined
            f.update(C=bool((a >> (cnt - 1)) & 1) if cnt < bits else None, Z=res == 0, S=bool(res & sign),
                     P=_parity(res), O=bool(a & sign) if cnt == 1 else None, A=None)
            return
        if kind == "shl":                                         # SDM SAL/SHL
            c = bool((a >> (bits - cnt)) & 1) if cnt < bits else None
            f.update(C=c, Z=res == 0, S=bool(res & sign), P=_parity(res), A=None,
                     O=(bool(res & sign) != c) if cnt == 1 and c is not None else None)
            return
        if kind == "sar":                                         # SDM SAR: CF defined for every count
            sa = (a & m) - (1 << bits) if a & sign else a & m
            f.update(C=bool((sa >> (cnt - 1)) & 1), Z=res == 0, S=bool(res & sign), P=_parity(res),
                     O=False if cnt == 1 else None, A=None)
            return
        if kind == "rol":                                         # SDM ROL: only CF and OF
            f.update(C=bool(res & 1), O=(bool(res & sign) != bool(res & 1)) if cnt == 1 else None)
            return
        if kind == "ror":                                         # SDM ROR: only CF and OF
            f.update(C=bool(res & sign), O=(bool(res & sign) != bool(res & (sign >> 1))) if cnt == 1 else None)
            return
        if kind == "bt":                                          # SDM BT/BTS/BTR: CF the bit; ZF unchanged
            f.update(C=bool(r & 1), O=None, S=None, A=None, P=None)
            return
        if kind == "bsf":                                         # SDM BSF/BSR: ZF the source was 0
            f.update(Z=(a & m) == 0, C=None, O=None, S=None, A=None, P=None)
            return
        if kind == "adc":                                         # SDM ADC: a + b + CF, with carry
            f.update(Z=res == 0, S=bool(res & sign), P=_parity(res), A=bool((a ^ b ^ res) & 0x10),
                     O=bool(~(a ^ b) & (a ^ res) & sign), C=bool(((a & b) | ((a | b) & ~res)) & sign))
            return
        if kind == "sbb":                                         # SDM SBB: a - b - CF, with borrow
            f.update(Z=res == 0, S=bool(res & sign), P=_parity(res), A=bool((a ^ b ^ res) & 0x10),
                     O=bool((a ^ b) & (a ^ res) & sign), C=bool(((~a & b) | (~(a ^ b) & res)) & sign))
            return
        if kind == "imul":                                        # SDM IMUL: CF=OF on truncation; SF ZF AF PF undefined
            sx = (a & m) - (1 << bits) if a & sign else a & m
            sy = (b & m) - (1 << bits) if b & sign else b & m
            sr = res - (1 << bits) if res & sign else res
            f.update(C=sx * sy != sr, O=sx * sy != sr, Z=None, S=None, P=None, A=None)
            return
        if kind == "andn":                                        # SDM ANDN: SF ZF by the result, CF OF 0, AF PF undefined
            f.update(Z=res == 0, S=bool(res & sign), C=False, O=False, A=None, P=None)
            return
        f.update(Z=res == 0, S=bool(res & sign), P=_parity(res))
        if kind == "logic":
            f.update(C=False, O=False, A=None)                    # SDM AND/OR/XOR/TEST: AF undefined
            return
        f["A"] = bool((a ^ b ^ res) & 0x10)                       # carry out of bit 3 (SDM Vol 1 3.4.3.1)
        if kind in ("add", "inc"):
            o = bool(~(a ^ b) & (a ^ res) & sign)
            f["O"] = o
            if kind == "add":
                f["C"] = (a & m) + (b & m) > m
        elif kind in ("sub", "dec"):
            f["O"] = bool((a ^ b) & (a ^ res) & sign)
            if kind == "sub":
                f["C"] = (a & m) < (b & m)

    def _rep(self, kind: str, size: int) -> None:
        """REP STOS / REP MOVS, one element at a time: a fault leaves RCX, RSI and RDI at the
        faulting iteration (SDM Vol 1 7.3.9; the instruction is interruptible)."""
        regs, step = self.th.regs, -size if self.th.df else size
        while regs[RCX]:
            if kind == "movs":
                value = self.mem.read(regs[RSI], size)
            else:
                value = (regs[RAX] & _mask(8 * size)).to_bytes(size, "little")
            self.mem.write(regs[RDI], value)
            regs[RDI] = (regs[RDI] + step) & M64
            if kind == "movs":
                regs[RSI] = (regs[RSI] + step) & M64
            regs[RCX] -= 1

    def _windows_divide_status(self) -> int:
        """The status Windows gives a #DE (the CPU raises one exception for a zero divisor and for an
        overflowing quotient; Windows tells them apart in software). Its handler re-decodes the
        instruction and reads the divisor: a legacy high-byte register (encodings 4-7 without REX,
        AH CH DH BH) is read as SPL BPL SIL DIL, and a memory operand's address is taken with the full
        64-bit registers, ignoring a 0x67 prefix. Zero or unreadable: STATUS_INTEGER_DIVIDE_BY_ZERO;
        otherwise STATUS_INTEGER_OVERFLOW. Grounds: all 127 DIV #DE results of case set 044 on an
        Intel Xeon 8573C under Server 2022 (run 38039360955) follow this rule; reading the divisor as
        the CPU does mispredicts 27 of them."""
        from w2m.x86dec import decode
        from w2m.x86ops import operands
        from w2m.lift import REG
        d = decode(self.mem.code_at(self.th.rip))
        o = operands(d.fields)[0].split(":") if d.fields is not None and d.fields.ops else []
        seen: int | None = None
        if o and o[0] == "R":
            idx, bits, hi = REG[o[1]]
            seen = self.th.regs[idx + 4] & 0xFF if hi else self.th.regs[idx] & _mask(bits)
        elif o and o[0] == "M":
            base, index, scale, disp, width = o[2], o[3], int(o[4]), int(o[5]), int(o[6])
            addr = disp + (self.th.regs[REG[base][0]] if base not in ("INVALID", "RIP") else 0)
            addr += self.th.regs[REG[index][0]] * scale if index != "INVALID" else 0
            if base == "RIP":
                addr += self.th.rip + d.length
            try:
                seen = int.from_bytes(self.mem.read(addr & M64, width), "little")
            except GuestFault:
                seen = None
        return STATUS_INTEGER_OVERFLOW if seen else STATUS_INTEGER_DIVIDE_BY_ZERO

    def _div(self, bits: int, divisor: int, signed: bool = False) -> None:
        """DIV and IDIV (SDM DIV, IDIV): faults before any write; every flag undefined afterwards. IDIV's
        quotient truncates toward zero and its remainder takes the dividend's sign; a quotient outside
        the signed range (INT_MIN / -1 among them) is #DE, as a zero divisor is."""
        m = _mask(bits)
        divisor &= m
        if bits == 8:
            dividend = self._rd(RAX, 16, False)
        else:
            dividend = self._rd(RDX, bits, False) << bits | self._rd(RAX, bits, False)
        if signed:
            sd = dividend - (1 << 2 * bits) if dividend >> (2 * bits - 1) else dividend
            sv = divisor - (1 << bits) if divisor >> (bits - 1) else divisor
            q = abs(sd) // abs(sv) * (1 if (sd < 0) == (sv < 0) else -1) if sv else 0
            rem = sd - q * sv if sv else 0
            overflow = not -(1 << (bits - 1)) <= q < 1 << (bits - 1)
        else:
            q, rem = divmod(dividend, divisor) if divisor else (0, 0)
            overflow = q > m
        if divisor == 0 or overflow:
            why = "by zero" if divisor == 0 else f"quotient {q:#x} outside {bits} bits"
            raise GuestFault(f"#DE: {'IDIV' if signed else 'DIV'} {why} at 0x{self.th.rip:x}",
                             self._windows_divide_status())
        q &= m
        rem &= m
        if bits == 8:
            self._wr(RAX, 8, False, q)
            self._wr(RAX, 8, True, rem)
        else:
            self._wr(RAX, bits, False, q)
            self._wr(RDX, bits, False, rem)
        self.th.flags.update(dict.fromkeys("CZSOPA"))

    def _cmpxchg16b(self, addr: int) -> None:
        """CMPXCHG16B (SDM): #GP (an access violation on Windows) unless 16-byte aligned; then compare
        RDX:RAX, store RCX:RBX or load the old value, which is also written back; ZF only."""
        if addr % 16:
            raise GuestFault(f"#GP: CMPXCHG16B operand 0x{addr:x} not 16-byte aligned at 0x{self.th.rip:x}")
        old = int.from_bytes(self.mem.read(addr, 16), "little")
        regs = self.th.regs
        equal = old == regs[RDX] << 64 | regs[RAX]
        new = regs[RCX] << 64 | regs[RBX] if equal else old
        self.mem.write(addr, new.to_bytes(16, "little"))
        if not equal:
            regs[RAX], regs[RDX] = old & M64, old >> 64
        self.th.flags["Z"] = equal

    # SSE ---------------------------------------------------------------------------------------
    def _xread(self, loc: Op, nbytes: int) -> int:
        if loc[0] == "x":
            return self.th.xmm[loc[1]] & _mask(8 * nbytes)
        if loc[0] == "r":
            return self._rd(loc[1], loc[2], loc[3])
        return int.from_bytes(self.mem.read(self._t[loc[1]], nbytes), "little")

    def _sse(self, ic: str, dst: Op, src: Op, align: int, imm: int | None = None) -> None:
        """One legacy-SSE instruction: every read and every fault check before any write."""
        th = self.th
        for loc in (dst, src):
            if loc[0] == "m" and align and self._t[loc[1]] % align:
                raise GuestFault(f"#GP: {ic} memory operand at 0x{self._t[loc[1]]:x} not {align}-byte aligned")
        msize = next((loc[2] for loc in (dst, src) if loc[0] == "m"), 16)
        x = th.xmm
        if ic in ("MOVAPS", "MOVAPD", "MOVUPS", "MOVUPD", "MOVDQA", "MOVDQU", "MOVNTPS", "LDDQU"):
            v = self._xread(src, 16)
            self._xwrite(dst, v, 16)
        elif ic in SSE_049B:
            self._sse_049b(ic, dst, src, imm)
        elif ic in SSE_050_NEW:
            self._sse_050(ic, dst, src, imm)
        elif ic in SSE2_INT or ic in LANE_ARITH or ic in INTERLEAVE:
            self._sse2_int(ic, dst, src, imm)
        elif ic in ("MOVSS", "MOVSD_XMM"):
            n = 4 if ic == "MOVSS" else 8
            v = self._xread(src, n)
            if dst[0] == "x" and src[0] == "x":
                x[dst[1]] = x[dst[1]] & ~_mask(8 * n) & _mask(128) | v        # register form merges
            else:
                self._xwrite(dst, v, n)                                       # load zeroes the rest
        elif ic in ("MOVD", "MOVQ"):
            n = 4 if ic == "MOVD" else 8
            if src[0] == "r":
                n = src[2] // 8
            elif dst[0] == "r":
                n = dst[2] // 8
            v = self._xread(src, n)
            self._xwrite(dst, v, n)                                           # to XMM: zero-extended
        elif ic in ("XORPS", "XORPD", "ANDPS", "ANDPD", "ORPD"):
            a, c = x[dst[1]], self._xread(src, 16)
            x[dst[1]] = a ^ c if ic.startswith("XOR") else a | c if ic == "ORPD" else a & c
        elif ic == "PCMPEQW":
            a, c = x[dst[1]], self._xread(src, 16)
            x[dst[1]] = fp.join([0xFFFF if p == q else 0 for p, q in zip(fp.lanes(a, 16, 8), fp.lanes(c, 16, 8))], 16)
        elif ic in ("CVTTPD2DQ", "CVTPD2DQ"):                    # two doubles to two int32; bits 127:64 zeroed
            v, env = self._xread(src, 16), fp.Env(th.mxcsr)
            words = [fp.to_int(fp.F64, d, 32, ic == "CVTTPD2DQ", env) for d in fp.lanes(v, 64, 2)]
            self._fp_check(env)
            x[dst[1]] = fp.join(words, 32)
        elif ic in ("CMPSD_XMM", "CMPSD_XMM_VEX"):             # predicate on the low doubles; all ones or 0
            assert imm is not None
            env = fp.Env(th.mxcsr)
            pred = imm & (31 if ic.endswith("VEX") else 7)       # legacy: imm8[2:0]; VEX: imm8[4:0] (SDM CMPSD)
            hit = fp.cmp_predicate(fp.F64, x[dst[1]] & _mask(64), self._xread(src, 8), pred, env)
            self._fp_check(env)
            x[dst[1]] = x[dst[1]] & ~_mask(64) & _mask(128) | (_mask(64) if hit else 0)
        elif ic == "HADDPD":                                     # dst = (dst.lo + dst.hi, src.lo + src.hi)
            a, c = x[dst[1]], self._xread(src, 16)
            env, flags, sums = fp.Env(th.mxcsr), 0, []
            for p, q in ((a & _mask(64), a >> 64), (c & _mask(64), c >> 64)):
                env.flags = 0                                    # each lane's flags, then all of them (as execute)
                sums.append(fp.arith("add", fp.F64, p, q, env))
                flags |= env.flags
            env.flags = flags
            self._fp_check(env)
            x[dst[1]] = sums[0] | sums[1] << 64
        elif ic[:-2] in ("ADD", "SUB", "MUL", "DIV", "SQRT", "MIN", "MAX"):
            v = self._xread(src, msize if src[0] == "m" else 16)
            if self._defer_fp:                                    # VEX.256: flags of both halves, one check
                out, flags = fp.execute(ic, x[dst[1]], v, th.mxcsr, check=False)
                self._deferred_flags |= flags
            else:
                out, th.mxcsr = self._fp(lambda: fp.execute(ic, x[dst[1]], v, th.mxcsr))
            x[dst[1]] = out
        elif ic in ("CVTTSS2SI", "CVTTSD2SI", "CVTSS2SI", "CVTSD2SI"):
            f = fp.F32 if "SS" in ic else fp.F64
            v = self._xread(src, f.bits // 8)
            env = fp.Env(th.mxcsr)
            r = fp.to_int(f, v, dst[2], ic.startswith("CVTT"), env)
            self._fp_check(env)
            self._wr(dst[1], dst[2], False, r)
        elif ic in ("CVTSI2SS", "CVTSI2SD"):
            f = fp.F32 if ic.endswith("SS") else fp.F64
            bits = src[2] if src[0] == "r" else 8 * src[2]
            v = self._xread(src, bits // 8)
            env = fp.Env(th.mxcsr)
            r = fp.from_int(f, v, bits, env)
            self._fp_check(env)
            x[dst[1]] = x[dst[1]] & ~_mask(f.bits) & _mask(128) | r
        elif ic in ("CVTSS2SD", "CVTSD2SS"):
            s_f, d_f = (fp.F32, fp.F64) if ic == "CVTSS2SD" else (fp.F64, fp.F32)
            v = self._xread(src, s_f.bits // 8)
            env = fp.Env(th.mxcsr)
            r = fp.convert(s_f, d_f, v, env)
            self._fp_check(env)
            x[dst[1]] = x[dst[1]] & ~_mask(d_f.bits) & _mask(128) | r
        elif ic in ("UCOMISS", "UCOMISD", "COMISS", "COMISD"):
            f = fp.F32 if ic.endswith("SS") else fp.F64
            a, c = x[dst[1]] & _mask(f.bits), self._xread(src, f.bits // 8)
            env = fp.Env(th.mxcsr)
            z, p, cf = fp.compare(f, a, c, env, signal_qnan=ic.startswith("COMI"))
            self._fp_check(env)
            th.flags.update(Z=z, P=p, C=cf, O=False, S=False, A=False)
        else:
            raise LiftError(f"sse {ic}")

    def _sse_049b(self, ic: str, dst: Op, src: Op, imm: int | None) -> None:
        """The set-049b legacy forms (Intel SDM Vol 2, each page; Sprint 049), on 128 bits."""
        th, x = self.th, self.th.xmm

        def lanes(v: int, w: int) -> list[int]:
            return [(v >> (w * i)) & _mask(w) for i in range(128 // w)]

        def pack(ls: list[int], w: int) -> int:
            return sum((n & _mask(w)) << (w * i) for i, n in enumerate(ls))

        if ic == "PEXTRQ":                                       # a quadword of the source to r/m64
            assert imm is not None
            self._xwrite(dst, x[src[1]] >> (64 * (imm & 1)) & _mask(64), 8)
            return
        a = x[dst[1]]
        if ic in ("PINSRD", "PINSRQ"):                           # the source into dword imm & 3, qword imm & 1
            assert imm is not None
            w = 32 if ic == "PINSRD" else 64
            lane = imm & (3 if w == 32 else 1)
            v = self._xread(src, w // 8) & _mask(w)
            x[dst[1]] = a & ~(_mask(w) << (w * lane)) & _mask(128) | v << (w * lane)
            return
        if ic == "PMOVSXDQ":                                     # two signed dwords to two qwords
            v = self._xread(src, 8)
            x[dst[1]] = pack([(d - (1 << 32)) if d >> 31 else d for d in (v & _mask(32), v >> 32 & _mask(32))], 64)
            return
        if ic == "CVTPS2PD":                                     # two floats to two doubles
            v, env, flags, out = self._xread(src, 8), fp.Env(th.mxcsr), 0, []
            for f32 in (v & _mask(32), v >> 32):
                env.flags = 0
                out.append(fp.convert(fp.F32, fp.F64, f32, env))
                flags |= env.flags
            env.flags = flags
            self._fp_check(env)
            x[dst[1]] = pack(out, 64)
            return
        if ic == "ROUNDSD":                                      # the low double to an integer value
            assert imm is not None
            v, env = self._xread(src, 8), fp.Env(th.mxcsr)
            x[dst[1]] = a & ~_mask(64) & _mask(128) | fp.round_to_integer(fp.F64, v, imm, env)
            self._fp_check(env)
            return
        v = self._xread(src, 16)
        if ic == "PSHUFB":                                       # byte i: 0 if index bit 7, else a[index & 15]
            ab = lanes(a, 8)
            x[dst[1]] = pack([0 if i & 0x80 else ab[i & 15] for i in lanes(v, 8)], 8)
        elif ic == "PALIGNR":                                    # (dst:src) >> 8*imm, the low 16 bytes
            assert imm is not None
            x[dst[1]] = ((a << 128 | v) >> (8 * imm)) & _mask(128) if imm < 32 else 0
        elif ic == "ANDNPD":
            x[dst[1]] = ~a & v & _mask(128)
        elif ic == "PERMILPS_VAR":                               # dword i = a[v's dword i & 3]
            aw = lanes(a, 32)
            x[dst[1]] = pack([aw[c & 3] for c in lanes(v, 32)], 32)
        elif ic == "PBLENDD":                                    # dword i from the source where imm bit i
            assert imm is not None
            aw, vw = lanes(a, 32), lanes(v, 32)
            x[dst[1]] = pack([vw[i] if imm >> i & 1 else aw[i] for i in range(4)], 32)
        elif ic in ("CVTDQ2PS", "CVTTPS2DQ", "CMPPS_VEX"):
            env, flags, out = fp.Env(th.mxcsr), 0, []
            for i, (p, q) in enumerate(zip(lanes(a, 32), lanes(v, 32))):
                env.flags = 0
                if ic == "CVTDQ2PS":
                    out.append(fp.from_int(fp.F32, q, 32, env))
                elif ic == "CVTTPS2DQ":
                    out.append(fp.to_int(fp.F32, q, 32, True, env))
                else:
                    assert imm is not None
                    out.append(_mask(32) if fp.cmp_predicate(fp.F32, p, q, imm & 31, env) else 0)
                flags |= env.flags
            env.flags = flags
            self._fp_check(env)
            x[dst[1]] = pack(out, 32)
        else:
            raise LiftError(f"sse {ic}")

    def _sse_050(self, ic: str, dst: Op, src: Op, imm: int | None) -> None:
        """The set-050 legacy-SSE classes (Intel SDM Vol 2, each page; Sprint 050), on 128 bits."""
        th, x = self.th, self.th.xmm

        def lanes(v: int, w: int) -> list[int]:
            return [(v >> (w * i)) & _mask(w) for i in range(128 // w)]

        def pack(ls: list[int], w: int) -> int:
            return sum((n & _mask(w)) << (w * i) for i, n in enumerate(ls))

        def saturate(v: int, w: int) -> int:
            return max(-(1 << (w - 1)), min((1 << (w - 1)) - 1, v))

        if ic in ("MOVLPS", "MOVHPD"):                           # one quadword, memory to or from a register's half
            shift = 0 if ic == "MOVLPS" else 64
            if dst[0] == "x":
                q = self._xread(src, 8)
                x[dst[1]] = x[dst[1]] & ~(_mask(64) << shift) & _mask(128) | q << shift
            else:
                self._xwrite(dst, x[src[1]] >> shift & _mask(64), 8)
            return
        if ic == "MOVNTDQ":                                      # a non-temporal 16-byte store
            self._xwrite(dst, x[src[1]], 16)
            return
        if ic == "PEXTRW":                                       # word imm & 7, zero-extended to the register
            assert imm is not None
            self._wr(dst[1], 32, False, x[src[1]] >> (16 * (imm & 7)) & 0xFFFF)
            return
        a = x[dst[1]]
        if ic == "PINSRB":                                       # the source's low byte into byte imm & 15
            assert imm is not None
            byte = self._xread(src, 1) & 0xFF
            lane = imm & 15
            x[dst[1]] = a & ~(0xFF << 8 * lane) & _mask(128) | byte << 8 * lane
            return
        if ic == "PMOVZXWD":                                     # four words zero-extended to dwords
            x[dst[1]] = pack(lanes(self._xread(src, 8), 16)[:4], 32)
            return
        if ic == "CMPSS":                                        # predicate imm8[2:0] on the low floats
            assert imm is not None
            env = fp.Env(th.mxcsr)
            hit = fp.cmp_predicate(fp.F32, a & _mask(32), self._xread(src, 4) & _mask(32), imm & 7, env)
            self._fp_check(env)
            x[dst[1]] = a & ~_mask(32) & _mask(128) | (_mask(32) if hit else 0)
            return
        v = self._xread(src, 16)
        if ic == "PSLLW":                                        # by imm8 or the source's low quadword
            count = imm if imm is not None else v & _mask(64)
            x[dst[1]] = 0 if count > 15 else pack([w << count for w in lanes(a, 16)], 16)
        elif ic == "SHUFPD":
            assert imm is not None
            x[dst[1]] = (a >> (64 * (imm & 1)) & _mask(64)) | (v >> (64 * (imm >> 1 & 1)) & _mask(64)) << 64
        elif ic == "MPSADBW":                                    # Sprint 065: eight sums of |a - v| over 4 bytes, the
            assert imm is not None                               # destination's block sliding a byte each (SDM MPSADBW)
            ab, vb = lanes(a, 8), lanes(v, 8)
            d0, s0 = 4 * (imm >> 2 & 1), 4 * (imm & 3)
            x[dst[1]] = pack([sum(abs(ab[d0 + i + j] - vb[s0 + j]) for j in range(4)) for i in range(8)], 16)
        elif ic == "PSADBW":                                     # per quadword: the sum of |a - v| over its bytes
            ab, vb = lanes(a, 8), lanes(v, 8)
            x[dst[1]] = pack([sum(abs(ab[i] - vb[i]) for i in range(h, h + 8)) for h in (0, 8)], 64)
        elif ic in ("PACKSSWB", "PACKSSDW"):                     # signed, saturated: the destination's half first
            w = 16 if ic == "PACKSSWB" else 32
            x[dst[1]] = pack([saturate(_signed(n, w), w // 2) for n in lanes(a, w) + lanes(v, w)], w // 2)
        elif ic == "PMADDWD":                                    # signed word products, adjacent pairs added
            aw, vw = [_signed(n, 16) for n in lanes(a, 16)], [_signed(n, 16) for n in lanes(v, 16)]
            x[dst[1]] = pack([aw[2 * i] * vw[2 * i] + aw[2 * i + 1] * vw[2 * i + 1] for i in range(4)], 32)
        elif ic == "PMADDUBSW":                                  # unsigned dst bytes by signed src bytes, saturated
            ab, vb = lanes(a, 8), [_signed(n, 8) for n in lanes(v, 8)]
            x[dst[1]] = pack([saturate(ab[2 * i] * vb[2 * i] + ab[2 * i + 1] * vb[2 * i + 1], 16) for i in range(8)], 16)
        elif ic in ("CVTPS2DQ", "CVTPD2PS", "CMPPS", "CMPPD"):
            env, flags, out = fp.Env(th.mxcsr), 0, []
            if ic == "CVTPD2PS":                                 # two doubles to two floats; bits 127:64 zeroed
                for d in lanes(v, 64):
                    env.flags = 0
                    out.append(fp.convert(fp.F64, fp.F32, d, env))
                    flags |= env.flags
                result = pack(out, 32)
            else:
                f, w = (fp.F64, 64) if ic == "CMPPD" else (fp.F32, 32)
                for p, q in zip(lanes(a, w), lanes(v, w)):
                    env.flags = 0
                    if ic == "CVTPS2DQ":
                        out.append(fp.to_int(fp.F32, q, 32, False, env))
                    else:
                        assert imm is not None
                        out.append(_mask(w) if fp.cmp_predicate(f, p, q, imm & 7, env) else 0)
                    flags |= env.flags
                result = pack(out, w)
            env.flags = flags
            self._fp_check(env)
            x[dst[1]] = result
        else:
            raise LiftError(f"sse {ic}")

    def _vex_cross(self, ic: str, d: Op, s1: Op | None, s2: Op, vl: int, imm: int | None) -> None:
        """VEX instructions that move data between the 128-bit halves (Intel SDM Vol 2; Sprint 049), on whole
        256-bit values: VPERMQ, VPERMD, the broadcasts, VINSERT/VEXTRACT F/I128, VCVTPS2PD and VPMOVSXDQ at 256
        bits, VZEROALL."""
        th, x, hi = self.th, self.th.xmm, self.th.ymm_hi
        if ic == "VZEROALL":
            th.xmm, th.ymm_hi = [0] * 16, [0] * 16
            return

        def read(loc: Op, nbytes: int) -> int:
            if loc[0] == "x":
                return (x[loc[1]] | hi[loc[1]] << 128) & _mask(8 * nbytes)
            return int.from_bytes(self.mem.read(self._t[loc[1]], nbytes), "little")

        def lanes(v: int, w: int, n: int) -> list[int]:
            return [(v >> (w * i)) & _mask(w) for i in range(n)]

        def pack(ls: list[int], w: int) -> int:
            return sum((n & _mask(w)) << (w * i) for i, n in enumerate(ls))

        width = 32 if vl else 16
        if ic == "VPERMQ":
            assert imm is not None
            q = lanes(read(s2, 32), 64, 4)
            out = pack([q[(imm >> (2 * i)) & 3] for i in range(4)], 64)
        elif ic == "VPERMD":
            assert s1 is not None
            idx, src = lanes(read(s1, 32), 32, 8), lanes(read(s2, 32), 32, 8)
            out = pack([src[i & 7] for i in idx], 32)
        elif ic in ("VPBROADCASTD", "VBROADCASTSS"):
            out = pack([read(s2, 4)] * (width // 4), 32)
        elif ic == "VPBROADCASTQ":
            out = pack([read(s2, 8)] * (width // 8), 64)
        elif ic == "VBROADCASTF128":
            v = read(s2, 16)
            out = v | v << 128
        elif ic in ("VINSERTF128", "VINSERTI128"):
            assert s1 is not None and imm is not None
            base, part = read(s1, 32), read(s2, 16)
            shift = 128 * (imm & 1)
            out = base & ~(_mask(128) << shift) & _mask(256) | part << shift
        elif ic in ("VEXTRACTF128", "VEXTRACTI128"):
            assert imm is not None
            out = read(s2, 32) >> (128 * (imm & 1)) & _mask(128)
            width = 16
        elif ic == "VCVTPS2PD":
            v, env, flags, res = read(s2, 16), fp.Env(th.mxcsr), 0, []
            for f32 in lanes(v, 32, 4):
                env.flags = 0
                res.append(fp.convert(fp.F32, fp.F64, f32, env))
                flags |= env.flags
            env.flags = flags
            self._fp_check(env)
            out = pack(res, 64)
        elif ic == "VPMOVSXDQ":
            out = pack([(w - (1 << 32)) if w >> 31 else w for w in lanes(read(s2, 16), 32, 4)], 64)
        else:
            raise LiftError(f"vex {ic}")
        if d[0] == "x":
            x[d[1]] = out & _mask(128)
            hi[d[1]] = out >> 128 & _mask(128) if width == 32 else 0
        else:
            self.mem.write(self._t[d[1]], out.to_bytes(width, "little"))

    def _vex(self, base: str, d: Op, s1: Op | None, s2: Op, vl: int, align: int, imm: int | None) -> None:
        """One VEX instruction (Sprint 049) as its legacy-SSE base: d = BASE(s1, s2) on the low 128 bits, the
        same on bits 255:128 for VEX.256; a VEX.128 write to a register zeroes its bits 255:128 (SDM Vol 1
        14.1.1). XMM 16 and 17 are the model's scratch (an operand that aliases the destination, the high
        halves). A memory operand of 32 bytes is read whole before any write."""
        x, hi = self.th.xmm, self.th.ymm_hi
        while len(x) < 18:
            x.append(0)
        for loc in (d, s2):
            if loc[0] == "m" and align and self._t[loc[1]] % align:
                raise GuestFault(f"#GP: V{base} memory operand at 0x{self._t[loc[1]]:x} not {align}-byte aligned")
        src_hi = 0
        if vl:
            if s2[0] == "x":
                src_hi = hi[s2[1]]
            elif s2[0] == "m":
                src_hi = int.from_bytes(self.mem.read(self._t[s2[1]] + 16, 16), "little")
        if vl and base in SHIFT_BY_COUNT and imm is None:             # the count is s2's low quadword, both halves
            src_hi = x[s2[1]] if s2[0] == "x" else int.from_bytes(self.mem.read(self._t[s2[1]], 16), "little")
        first_hi = hi[s1[1]] if vl and s1 is not None and s1[0] == "x" else hi[d[1]] if vl and d[0] == "x" else 0
        if vl and d[0] == "m":                                    # a 256-bit store: all 32 bytes or none
            self.mem.write(self._t[d[1]], x[s2[1]].to_bytes(16, "little") + hi[s2[1]].to_bytes(16, "little"))
            del x[16:]
            return
        fp_both = vl and base[:-2] in ("ADD", "SUB", "MUL", "DIV", "SQRT", "MIN", "MAX") or (vl and base in VEX_FP)
        mxcsr_in = self.th.mxcsr
        if fp_both:                                               # both halves with the real MXCSR, their flags
            self._defer_fp, self._deferred_flags = True, 0        # collected, then one check (an unmasked
                                                                  # exception faults with no lane committed)
        low_src = s2
        if s1 is not None and s1[0] == "x" and d[0] == "x" and s1[1] != d[1]:
            if s2 == ("x", d[1]):
                x[16] = x[d[1]]
                low_src = ("x", 16)
            x[d[1]] = x[s1[1]]
        self._sse(base, d, low_src, 0, imm)
        if base == "PMOVMSKB" and vl:                            # 32 sign bits: the upper half's 16 above
            self._wr(d[1], d[2], False, self._rd(d[1], d[2], False) | sum(((src_hi >> (8 * i + 7)) & 1) << (16 + i)
                                                                          for i in range(16)))
        elif d[0] == "x" and base in ("COMISD", "UCOMISD", "COMISS", "UCOMISS"):
            pass                                                  # flags only: d is a source
        elif d[0] == "x":
            if vl:
                x[16], x[17] = first_hi, src_hi
                self._sse(base, ("x", 16), ("x", 17), 0, imm >> 4 if base == "PBLENDD" and imm is not None else imm)
                hi[d[1]] = x[16]
            else:
                hi[d[1]] = 0
        if fp_both:
            env = fp.Env(mxcsr_in)
            env.flags = self._deferred_flags                      # this instruction's flags, both halves
            self._defer_fp = False
            self.th.mxcsr = mxcsr_in
            self._fp_check(env)
        del x[16:]                                                # the scratch registers go

    def _fma(self, ic: str, d: Op, s2: Op, s3: Op, vl: int) -> None:
        """VFMADD/VFMSUB/VFNMADD/VFNMSUB 132/213/231 in SS/SD/PS/PD (SDM; Sprint 049): one rounding. Operand 1
        is the destination, 2 VEX.vvvv, 3 r/m; 132 is o1*o3 + o2, 213 o2*o1 + o3, 231 o2*o3 + o1."""
        th, x, hi = self.th, self.th.xmm, self.th.ymm_hi
        m = re.fullmatch(r"V(FN?M(?:ADD|SUB))(132|213|231)([SP][SD])", ic)
        if m is None:
            raise LiftError(f"fma {ic}")
        kind, order, form = m.groups()
        neg_p, neg_a = {"FMADD": (False, False), "FMSUB": (False, True), "FNMADD": (True, False),
                        "FNMSUB": (True, True)}[kind]
        mul, add = {"132": ((0, 2), 1), "213": ((1, 0), 2), "231": ((1, 2), 0)}[order]
        f = fp.F32 if form[1] == "S" else fp.F64
        scalar = form[0] == "S"
        nbytes = f.bits // 8 if scalar else 16
        o1, o2 = x[d[1]], x[s2[1]]
        o3 = self._xread(s3, nbytes)
        env = fp.Env(th.mxcsr)
        count = 1 if scalar else 128 // f.bits
        a1, a2, a3 = (fp.lanes(v, f.bits, 128 // f.bits) for v in (o1, o2, o3))
        out = list(a1)
        flags = 0

        def lane(ops: tuple[int, int, int]) -> int:            # each lane's flags, then all of them (as execute)
            nonlocal flags
            env.flags = 0
            r = fp.fma_lane(f, ops, mul, add, neg_p, neg_a, env)
            flags |= env.flags
            return r

        for i in range(count):
            out[i] = lane((a1[i], a2[i], a3[i]))
        new_hi = 0
        if vl and not scalar:
            h3 = hi[s3[1]] if s3[0] == "x" else int.from_bytes(self.mem.read(self._t[s3[1]] + 16, 16), "little")
            b1, b2, b3 = (fp.lanes(v, f.bits, 128 // f.bits) for v in (hi[d[1]], hi[s2[1]], h3))
            new_hi = fp.join([lane((b1[i], b2[i], b3[i])) for i in range(count)], f.bits)
        env.flags = flags
        self._fp_check(env)
        x[d[1]] = fp.join(out, f.bits)
        hi[d[1]] = new_hi

    def _sse2_int(self, ic: str, dst: Op, src: Op, imm: int | None) -> None:
        """The SSE2 integer block (SDM pages of each; Sprint 044): no MXCSR flags."""
        x = self.th.xmm

        def lanes(v: int, w: int) -> list[int]:
            return [(v >> (w * i)) & _mask(w) for i in range(128 // w)]

        def pack(ls: list[int], w: int) -> int:
            return sum((n & _mask(w)) << (w * i) for i, n in enumerate(ls))

        if ic == "PINSRW":                                       # the source's low word into word imm & 7
            assert imm is not None
            word = self._xread(src, 2) & 0xFFFF
            lane = imm & 7
            x[dst[1]] = x[dst[1]] & ~(0xFFFF << 16 * lane) & _mask(128) | word << 16 * lane
            return
        if ic == "MOVHPS":                                       # the high quadword from or to memory
            if dst[0] == "x":
                x[dst[1]] = x[dst[1]] & _mask(64) | self._xread(src, 8) << 64
            else:
                self._xwrite(dst, x[src[1]] >> 64, 8)
            return
        if ic in ("PCMPISTRI", "PCMPISTRI64"):                     # string_compare.py (SDM Vol 2A §4.1)
            assert imm is not None
            a, b = x[dst[1]], self._xread(src, 16)
            res, flags = string_compare.compare(a, b, imm, string_compare.implicit_length(a, imm),
                                                string_compare.implicit_length(b, imm))
            self._wr(RCX, 64 if ic.endswith("64") else 32, False, string_compare.index(res, imm))
            self.th.flags.update(flags)
            return
        if ic in ("RCPPS", "RSQRTPS", "RCPSS", "RSQRTSS"):         # the reference CPU's tables (approximate.py)
            table_fn = approximate.rcp if ic.startswith("RCP") else approximate.rsqrt
            if ic.endswith("PS"):
                x[dst[1]] = pack([table_fn(w) for w in lanes(self._xread(src, 16), 32)], 32)
            else:
                x[dst[1]] = x[dst[1]] & ~_mask(32) & _mask(128) | table_fn(self._xread(src, 4) & _mask(32))
            return
        if ic == "MOVMSKPS":
            v = x[src[1]]
            self._wr(dst[1], dst[2], False, sum(((v >> (32 * i + 31)) & 1) << i for i in range(4)))
            return
        if ic == "PMOVMSKB":
            v = x[src[1]]
            self._wr(dst[1], dst[2], False, sum(((v >> (8 * i + 7)) & 1) << i for i in range(16)))
            return
        if ic == "CVTDQ2PD":
            v = self._xread(src, 8)
            words = [(v >> (32 * i)) & 0xFFFF_FFFF for i in range(2)]
            vals = [w - (1 << 32) if w >> 31 else w for w in words]
            x[dst[1]] = pack([int.from_bytes(struct.pack("<d", float(n)), "little") for n in vals], 64)   # exact
            return
        a = x[dst[1]]
        if ic in ("PSRLDQ", "PSLLDQ"):
            assert imm is not None
            n = min(imm, 16)
            x[dst[1]] = (a >> (8 * n) if ic == "PSRLDQ" else a << (8 * n)) & _mask(128)
            return
        if ic in ("PSRLW", "PSRAW", "PSLLQ"):
            count = imm if imm is not None else self._xread(src, 16) & _mask(64)
            w = 64 if ic == "PSLLQ" else 16
            if ic == "PSRAW":
                signed = [v - (1 << 16) if v >> 15 else v for v in lanes(a, 16)]
                x[dst[1]] = pack([v >> min(count, 15) for v in signed], 16)
            else:
                x[dst[1]] = 0 if count >= w else pack([v >> count if ic == "PSRLW" else v << count for v in lanes(a, w)], w)
            return
        if ic in ("PSRLD", "PSLLD", "PSRAD"):
            count = imm if imm is not None else self._xread(src, 16) & _mask(64)
            if ic == "PSRAD":
                signed = [w - (1 << 32) if w >> 31 else w for w in lanes(a, 32)]
                x[dst[1]] = pack([w >> min(count, 31) for w in signed], 32)
            else:
                x[dst[1]] = 0 if count > 31 else pack([w >> count if ic == "PSRLD" else w << count
                                                       for w in lanes(a, 32)], 32)
            return
        if ic in ("MOVHLPS", "MOVLHPS"):
            v = x[src[1]]
            x[dst[1]] = (a & ~_mask(64) | v >> 64) if ic == "MOVHLPS" else (a & _mask(64) | (v & _mask(64)) << 64)
            return
        if ic == "PSRLQ":
            count = imm if imm is not None else self._xread(src, 16) & _mask(64)
            x[dst[1]] = 0 if count > 63 else pack([lane >> count for lane in lanes(a, 64)], 64)
            return
        v = self._xread(src, 16)
        if ic == "PSHUFD":
            assert imm is not None
            words = lanes(v, 32)
            x[dst[1]] = pack([words[(imm >> (2 * i)) & 3] for i in range(4)], 32)
        elif ic in ("PAND", "POR", "ORPS"):
            x[dst[1]] = a & v if ic == "PAND" else a | v
        elif ic in ("PSUBD", "PSUBQ"):
            w = 32 if ic == "PSUBD" else 64
            x[dst[1]] = pack([p - q for p, q in zip(lanes(a, w), lanes(v, w))], w)
        elif ic == "PUNPCKLBW":
            ab, vb = lanes(a, 8), lanes(v, 8)
            x[dst[1]] = pack([b for i in range(8) for b in (ab[i], vb[i])], 8)
        elif ic == "PCMPEQB":
            x[dst[1]] = pack([0xFF if p == q else 0 for p, q in zip(lanes(a, 8), lanes(v, 8))], 8)
        elif ic == "SHUFPS":
            assert imm is not None
            aw, vw = lanes(a, 32), lanes(v, 32)
            x[dst[1]] = pack([aw[imm & 3], aw[(imm >> 2) & 3], vw[(imm >> 4) & 3], vw[(imm >> 6) & 3]], 32)
        elif ic == "PXOR":
            x[dst[1]] = a ^ v
        elif ic == "ANDNPS":
            x[dst[1]] = ~a & v & _mask(128)
        elif ic in ("UNPCKLPS", "PUNPCKLDQ", "UNPCKHPS"):
            aw, vw, base = lanes(a, 32), lanes(v, 32), 2 if ic == "UNPCKHPS" else 0
            x[dst[1]] = pack([aw[base], vw[base], aw[base + 1], vw[base + 1]], 32)
        elif ic == "PADDD":
            x[dst[1]] = pack([p + q for p, q in zip(lanes(a, 32), lanes(v, 32))], 32)
        elif ic in LANE_ARITH:
            w, f = LANE_ARITH[ic]
            x[dst[1]] = pack([f(p, q, w) for p, q in zip(lanes(a, w), lanes(v, w))], w)
        elif ic in INTERLEAVE:
            w, high = INTERLEAVE[ic]
            al, vl = lanes(a, w), lanes(v, w)
            half = 64 // w
            base = half if high else 0
            x[dst[1]] = pack([e for i in range(half) for e in (al[base + i], vl[base + i])], w)
        elif ic == "PANDN":
            x[dst[1]] = ~a & v & _mask(128)
        elif ic == "PACKUSWB":                                    # signed words to unsigned bytes, saturated
            def sat(word: int) -> int:
                s = word - (1 << 16) if word >> 15 else word
                return max(0, min(255, s))
            x[dst[1]] = pack([sat(word) for word in lanes(a, 16) + lanes(v, 16)], 8)
        elif ic in ("PSHUFLW", "PSHUFHW"):
            assert imm is not None
            words = lanes(v, 16)
            base = 0 if ic == "PSHUFLW" else 4
            moved = [words[base + ((imm >> (2 * i)) & 3)] for i in range(4)]
            x[dst[1]] = pack(moved + words[4:] if ic == "PSHUFLW" else words[:4] + moved, 16)
        else:
            raise LiftError(f"sse {ic}")

    def _xwrite(self, loc: Op, v: int, nbytes: int) -> None:
        if loc[0] == "x":
            self.th.xmm[loc[1]] = v & _mask(8 * nbytes)
        elif loc[0] == "r":
            self._wr(loc[1], loc[2], loc[3], v)
        else:
            self.mem.write(self._t[loc[1]], (v & _mask(8 * nbytes)).to_bytes(nbytes, "little"))

    def _fp(self, run: Callable[[], tuple[int, int]]) -> tuple[int, int]:
        try:
            return run()
        except fp.SimdFault as e:
            self.th.mxcsr |= e.flags                          # flags at an unmasked fault (oracle)
            raise

    _defer_fp = False                                             # VEX.256 halves: collect flags, check once
    _deferred_flags = 0

    def _fp_check(self, env: fp.Env) -> None:
        if self._defer_fp:
            self._deferred_flags |= env.flags
            return
        try:
            env.check()
        except fp.SimdFault as e:
            self.th.mxcsr |= e.flags                          # the flags the fault reports
            raise
        self.th.mxcsr |= env.flags

    # execution -------------------------------------------------------------------------------
    def _exec(self, ins: Lifted) -> None:
        """Run one instruction; on a fault, every register returns to its value before it (the
        instruction's single memory store is all-or-nothing in Memory.write)."""
        th = self.th
        saved = (list(th.regs), dict(th.flags), list(th.xmm), th.mxcsr, th.df)
        saved_hi = list(th.ymm_hi)
        try:
            self._exec_ops(ins)
        except (GuestFault, fp.SimdFault):
            mxcsr = th.mxcsr
            interrupted = list(th.regs)
            th.regs, th.flags, th.xmm, th.mxcsr, th.df = saved
            th.ymm_hi = saved_hi
            self._defer_fp = False
            if any(op[0] == "rep" for op in ins.ops):        # REP keeps its completed iterations (SDM
                th.regs = interrupted                        # Vol 1 7.3.9): RCX, RSI, RDI as at the fault
            th.mxcsr = mxcsr                                  # MXCSR flags a fault sets survive it
            th.rip = ins.address
            raise

    def _exec_ops(self, ins: Lifted) -> None:
        t: dict[int, int] = {}
        self._t = t
        nxt = ins.address + ins.length
        for op in ins.ops:
            k = op[0]
            if k == "imm":
                t[op[1]] = op[2] & M64
            elif k == "addr":
                _, d, base, index, scale, disp, abits, seg = op
                if seg is not None:            # the model has no TEB; the VM serves gs: (Sprint 044)
                    raise HandlerFailed(f"{seg}-relative access at 0x{ins.address:x}: the model has no TEB")
                v = disp + (self.th.regs[base] if base is not None else 0)
                v += self.th.regs[index] * scale if index is not None else 0
                t[d] = v & _mask(abits)
            elif k == "div":
                self._div(op[1], t[op[2]], bool(op[3]) if len(op) > 3 else False)
            elif k == "cx16":
                self._cmpxchg16b(t[op[1]])
            elif k == "rep":
                self._rep(op[1], op[2])
            elif k == "mxcsr":
                if op[1] == "st":
                    self._store(op[2], self.th.mxcsr)
                else:
                    v = self._load(op[2], t)
                    if v & ~0xFFFF:                                       # MXCSR_MASK 0xFFFF: reserved bits #GP
                        raise GuestFault(f"#GP: LDMXCSR 0x{v:x} sets reserved bits at 0x{ins.address:x}")
                    self.th.mxcsr = v
            elif k == "vexs":
                self._vex_cross(op[1], op[2], op[3], op[4], op[5], op[6])
            elif k == "vex":
                self._vex(op[1], op[2], op[3], op[4], op[5], op[6], op[7] if len(op) > 7 else None)
            elif k == "fma":
                self._fma(op[1], op[2], op[3], op[4], op[5])
            elif k == "vzeroupper":
                self.th.ymm_hi = [0] * 16
            elif k == "xgetbv":
                if self.th.regs[1] & 0xFFFF_FFFF:                         # XCR0 is the only one (no XGETBV1)
                    raise GuestFault(f"#GP: XGETBV ECX 0x{self.th.regs[1] & 0xFFFF_FFFF:x} at 0x{ins.address:x}")
                self.th.regs[0], self.th.regs[2] = XCR0, 0
            elif k == "fcw":
                self._store(op[2], self.th.fcw)
            elif k == "mmx":
                self._mmx(op, ins)
            elif k == "x87":
                self._x87(op, ins)
            elif k == "fence":
                pass
            elif k == "ud":
                raise GuestFault(f"#UD: {ins.iclass} at 0x{ins.address:x}", STATUS_ILLEGAL_INSTRUCTION)
            elif k == "ld":
                t[op[1]] = self._load(op[2], t)
            elif k == "st":
                self._store(op[1], t[op[2]])
            elif k == "alu":
                _, d, o, a, b, bits = op
                x, y, m = t[a], t[b], _mask(bits)
                if o == "add":
                    v = x + y
                elif o == "sub":
                    v = x - y
                elif o == "and":
                    v = x & y
                elif o == "or":
                    v = x | y
                elif o == "xor":
                    v = x ^ y
                elif o == "shr":
                    v = (x & m) >> (y & (0x3F if bits == 64 else 0x1F))
                elif o == "mul_lo":
                    v = x * y
                elif o == "mul_hi":
                    v = (x * y) >> 64
                elif o == "sext32":
                    v = x - (1 << 32) if x & 0x80000000 else x
                elif o in ("sext8", "sext16"):
                    w = 8 if o == "sext8" else 16
                    xw = x & _mask(w)
                    v = xw - (1 << w) if xw >> (w - 1) else xw
                elif o == "bswap":
                    v = int.from_bytes((x & m).to_bytes(bits // 8, "little"), "big")
                elif o == "sbb":
                    v = x - y - int(self._flag("C"))
                elif o == "adc":
                    v = x + y + int(self._flag("C"))
                elif o in ("bsf", "bsr"):
                    xm = x & m
                    v = 0 if xm == 0 else ((xm & -xm).bit_length() - 1 if o == "bsf" else xm.bit_length() - 1)
                elif o in ("shl", "sar", "rol", "ror"):
                    c = y & (0x3F if bits == 64 else 0x1F)
                    xm = x & m
                    if o == "shl":
                        v = xm << c
                    elif o == "sar":
                        v = ((xm - (1 << bits)) if xm >> (bits - 1) else xm) >> c
                    else:
                        k = c % bits
                        v = ((xm << k) | (xm >> (bits - k))) if o == "rol" else ((xm >> k) | (xm << (bits - k)))
                elif o in ("smul_lo", "smul_hi"):
                    sx = (x & m) - (1 << bits) if x >> (bits - 1) & 1 else x & m
                    sy = (y & m) - (1 << bits) if y >> (bits - 1) & 1 else y & m
                    v = sx * sy if o == "smul_lo" else (sx * sy) >> bits

                else:
                    raise LiftError(f"alu {o}")
                t[d] = v & m
            elif k == "flags":
                _, kind, a, b, r, bits = op
                self._set_flags(kind, t[a], t[b], t[r], bits)
            elif k == "set":
                self._store(op[2], int(self._cond(op[1])))
            elif k == "df":
                self.th.df = bool(op[1])
            elif k == "sel":                                      # ("sel", t, cond_t, x, y): cond ? x : y
                t[op[1]] = t[op[3]] if t[op[2]] else t[op[4]]
            elif k in ("cst", "cstz"):                            # store when cond is nonzero (cst) or zero (cstz)
                if bool(t[op[1]]) == (k == "cst"):
                    self._store(op[2], t[op[3]])
            elif k == "cmov":                                     # ("cmov", cc, loc, t): SDM CMOVcc
                if self._cond(op[1]):
                    self._store(op[2], t[op[3]])
                elif op[2][0] == "r" and op[2][2] == 32:
                    self.th.regs[op[2][1]] &= 0xFFFF_FFFF         # 64-bit mode: a 32-bit CMOV zero-extends
            elif k == "cpuid":                                # 32-bit results zero-extend (SDM CPUID)
                eax, ecx = self.th.regs[0] & 0xFFFF_FFFF, self.th.regs[1] & 0xFFFF_FFFF
                self.cpuid_reads.append((eax, ecx))
                a, b, c, d = cpuid.lookup(eax, ecx)
                self.th.regs[0], self.th.regs[3], self.th.regs[1], self.th.regs[2] = a, b, c, d
            elif k == "sse":
                self._sse(op[1], op[2], op[3], op[4], op[5] if len(op) > 5 else None)
            elif k == "x86":                                      # ("x86", ic, dst, srcs, bits) (Sprint 050)
                target = integer_model.execute(self, op[1], op[2], [t[v] for v in op[3]], op[4])
                if target is not None:
                    self.th.rip = target
                    return
            elif k == "br":
                if self._cond(op[1]):
                    self.th.rip = t[op[2]]
                    return
            else:
                raise LiftError(f"operation {k}")
        self.th.rip = nxt

    def step(self) -> bool:
        """Execute one x86 instruction, or service one kernel32 call when RIP is a thunk.
        Returns False once the process has exited."""
        th = self.th
        if th.rip in self.ld.thunks:
            ret = self.kernel.call(self.ld.thunks[th.rip], th)
            if ret is None:
                return False
            th.rip = ret
            return True
        ins = self.cache.get(th.rip)
        if ins is None:
            ins = self.cache[th.rip] = lift(self.mem.code_at(th.rip), th.rip)
        self.seen.add(th.rip)
        self._exec(ins)
        self.steps += 1
        return True

    def run(self, max_steps: int = 1_000_000) -> Result:
        stop = "max_steps"
        try:
            while self.steps < max_steps:
                if not self.step():
                    stop = "ExitProcess"
                    break
        except (LiftError, GuestFault, UndefinedFlag, Unimplemented, HandlerFailed, fp.SimdFault) as e:
            stop = f"{type(e).__name__}: {e}"
        return Result(self.exit_code, bytes(self.kernel.out), self.steps, len(self.seen),
                      self.calls, stop)
