# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""x86fp.py — SSE floating-point semantics on bit patterns, under MXCSR (Sprint 039).

The reference the model runs for scalar and packed single- and double-precision instructions.
Every result is computed exactly (rational arithmetic) and rounded once, in the mode MXCSR.RC
selects, so no host floating-point behaviour leaks in. Sources:

- IEEE 754-2019 for the formats and correctly rounded results.
- Intel SDM Vol 1 §10.2.3 (MXCSR: IE DE ZE OE UE PE flags, bits 0-5; DAZ bit 6; masks bits 7-12;
  RC bits 13-14; FTZ bit 15), §11.5 (SIMD floating-point exceptions), Table 4-7 (NaN operands:
  for SSE the first source's NaN is returned, quieted; a QNaN result with no NaN operand is the
  "real indefinite", sign 1 and only the quiet bit set).
- Underflow: x86 detects tininess after rounding (glibc sysdeps/x86/tininess.h,
  TININESS_AFTER_ROUNDING 1); a masked underflow sets UE only when the result is also inexact.
- Vol 2 instruction pages for each instruction's exception list and special cases (MIN/MAX
  return the second source on a NaN or two zeros, an SNaN there unchanged; CVTT*2SI return the
  integer indefinite 1 << (w-1) on NaN or overflow).

Behaviour the SDM leaves to the reader and the CPU oracle decides (Sprint 039, scripts/
semantics_differential.py) is marked "oracle" where it is chosen.
"""

from __future__ import annotations

import math
from dataclasses import dataclass
from fractions import Fraction

IE, DE, ZE, OE, UE, PE = 1, 2, 4, 8, 16, 32
DAZ, FTZ = 1 << 6, 1 << 15
MASK_SHIFT = 7
MXCSR_DEFAULT = 0x1F80
MXCSR_WRITABLE = 0xFFFF


class SimdFault(Exception):
    """An unmasked SIMD floating-point exception: #XM; the destination is not written."""

    def __init__(self, flags: int) -> None:
        super().__init__(f"unmasked SIMD floating-point exception, flags 0x{flags:x}")
        self.flags = flags


@dataclass(frozen=True)
class Fmt:
    bits: int
    ebits: int
    mbits: int

    @property
    def bias(self) -> int:
        return (1 << (self.ebits - 1)) - 1

    @property
    def emin(self) -> int:
        return 1 - self.bias

    @property
    def emax(self) -> int:
        return self.bias

    @property
    def quiet(self) -> int:
        return 1 << (self.mbits - 1)

    @property
    def emask(self) -> int:
        return (1 << self.ebits) - 1

    @property
    def mmask(self) -> int:
        return (1 << self.mbits) - 1

    @property
    def sign(self) -> int:
        return 1 << (self.bits - 1)

    @property
    def indefinite(self) -> int:
        return self.sign | self.emask << self.mbits | self.quiet

    @property
    def max_finite(self) -> Fraction:
        return Fraction(2 ** (self.mbits + 1) - 1) * Fraction(2) ** (self.emax - self.mbits)


F32 = Fmt(32, 8, 23)
F64 = Fmt(64, 11, 52)


def fields(f: Fmt, x: int) -> tuple[int, int, int]:
    return x >> (f.bits - 1) & 1, x >> f.mbits & f.emask, x & f.mmask


def is_nan(f: Fmt, x: int) -> bool:
    _, e, m = fields(f, x)
    return e == f.emask and m != 0


def is_snan(f: Fmt, x: int) -> bool:
    return is_nan(f, x) and not x & f.quiet


def is_denormal(f: Fmt, x: int) -> bool:
    _, e, m = fields(f, x)
    return e == 0 and m != 0


def is_inf(f: Fmt, x: int) -> bool:
    _, e, m = fields(f, x)
    return e == f.emask and m == 0


def is_zero(f: Fmt, x: int) -> bool:
    return x & (f.sign - 1) == 0


def value(f: Fmt, x: int) -> Fraction:
    """The exact value of a finite pattern."""
    s, e, m = fields(f, x)
    v = Fraction(m, 1 << f.mbits) * Fraction(2) ** f.emin if e == 0 else \
        Fraction((1 << f.mbits) | m, 1 << f.mbits) * Fraction(2) ** (e - f.bias)
    return -v if s else v


def _floor_log2(v: Fraction) -> int:
    e = v.numerator.bit_length() - v.denominator.bit_length()
    if Fraction(2) ** e > v:
        e -= 1
    elif Fraction(2) ** (e + 1) <= v:
        e += 1
    return e


def _round_int(n: Fraction, negative: bool, rc: int) -> int:
    """Round a non-negative rational to an integer; rc 0 nearest-even, 1 toward -inf, 2 toward
    +inf, 3 toward zero (SDM Vol 1 Table 4-8); `negative` is the sign of the value n measures."""
    q, r = divmod(n.numerator, n.denominator)
    if r == 0:
        return q
    if rc == 0:
        twice = 2 * r
        if twice > n.denominator or (twice == n.denominator and q & 1):
            return q + 1
        return q
    if rc == 3 or (rc == 1 and not negative) or (rc == 2 and negative):
        return q
    return q + 1


class Env:
    """MXCSR for one instruction: reads DAZ, FTZ, RC; collects flags."""

    def __init__(self, mxcsr: int) -> None:
        self.mxcsr = mxcsr
        self.flags = 0

    @property
    def rc(self) -> int:
        return self.mxcsr >> 13 & 3

    @property
    def masks(self) -> int:
        return self.mxcsr >> MASK_SHIFT & 0x3F

    def src(self, f: Fmt, x: int, denormal_flag: bool = True) -> int:
        """A source operand after DAZ: a denormal becomes a zero of its sign (no DE); otherwise a
        denormal sets DE when the instruction lists Denormal."""
        if is_denormal(f, x):
            if self.mxcsr & DAZ:
                return x & f.sign
            if denormal_flag:
                self.flags |= DE
        return x

    def round(self, f: Fmt, v: Fraction) -> int:
        """The pattern nearest `v` (nonzero) in mode RC, with OE, UE, PE and FTZ."""
        negative = v < 0
        a = -v if negative else v
        sign = f.sign if negative else 0
        e = _floor_log2(a)
        # tininess after rounding: round to mbits+1 significant bits with unbounded exponent
        unbounded = _round_int(a / Fraction(2) ** (e - f.mbits), negative, self.rc) * Fraction(2) ** (e - f.mbits)
        tiny = unbounded < Fraction(2) ** f.emin
        q = Fraction(2) ** (max(e, f.emin) - f.mbits)
        n = _round_int(a / q, negative, self.rc)
        r = n * q
        inexact = r != a
        # an unmasked overflow or underflow delivers no rounded result; Precision then reports
        # whether rounding to the format's precision with an unbounded exponent was inexact
        # (IEEE 754 trapped exceptions; MULSS and CVTSD2SS, run 38010775429)
        wide_inexact = unbounded != a
        if r > f.max_finite:
            to_inf = self.rc == 0 or (self.rc == 1 and negative) or (self.rc == 2 and not negative)
            self.flags |= OE | (PE if self.masks & OE or wide_inexact else 0)
            return sign | (f.emask << f.mbits if to_inf else (f.emask - 1) << f.mbits | f.mmask)
        if tiny:
            um = self.masks & UE
            if um and self.mxcsr & FTZ:
                self.flags |= UE | PE
                return sign
            if not um:
                self.flags |= UE | (PE if wide_inexact else 0)
                inexact = False
            elif inexact:
                self.flags |= UE
        if inexact:
            self.flags |= PE
        if r == 0:
            return sign
        e2 = _floor_log2(r)
        if e2 < f.emin:
            return sign | int(r / Fraction(2) ** (f.emin - f.mbits))
        mant = int(r / Fraction(2) ** (e2 - f.mbits)) & f.mmask
        return sign | (e2 + f.bias) << f.mbits | mant

    def check(self) -> None:
        """Raise SimdFault for an unmasked exception. An unmasked pre-computation exception (IE,
        DE, ZE) stops the instruction before any result is computed, so only those flags reach
        MXCSR; otherwise every flag does (observed on AMD EPYC 7763, run 38010775429)."""
        pre = self.flags & (IE | DE | ZE)
        if pre & ~self.masks:
            raise SimdFault(pre)
        if self.flags & ~self.masks & 0x3F:
            raise SimdFault(self.flags)


# The Windows exception code a #XM is delivered with: the first unmasked flag of MXCSR after the
# fault, in this order, picks it; Denormal is reported as an invalid operation (ntstatus.h codes;
# order and DE's code observed on windows-2025, run 38010775429, 1,056 #XM faults).
XM_STATUS = ((IE, 0xC000_0090), (ZE, 0xC000_008E), (DE, 0xC000_0090), (OE, 0xC000_0091),
             (UE, 0xC000_0093), (PE, 0xC000_008F))


def xm_status(mxcsr: int) -> int:
    unmasked = mxcsr & ~(mxcsr >> MASK_SHIFT) & 0x3F
    return next(code for flag, code in XM_STATUS if unmasked & flag)


def _quiet(f: Fmt, x: int) -> int:
    return x | f.quiet


def _nan_result(f: Fmt, a: int, b: int | None, env: Env) -> int | None:
    """SDM Vol 1 Table 4-7 for SSE: an SNaN operand sets IE; the first source's NaN wins."""
    nans = [x for x in (a, b) if x is not None and is_nan(f, x)]
    if not nans:
        return None
    if any(is_snan(f, x) for x in nans):
        env.flags |= IE
    return _quiet(f, nans[0])


def arith(op: str, f: Fmt, a: int, b: int, env: Env) -> int:
    """ADD SUB MUL DIV on two patterns (a the first source)."""
    a, b = env.src(f, a), env.src(f, b)
    n = _nan_result(f, a, b, env)
    if n is not None:
        env.flags &= ~DE                                    # a NaN operand outranks Denormal (oracle)
        return n
    sa, sb = bool(a & f.sign), bool(b & f.sign)
    ia, ib = is_inf(f, a), is_inf(f, b)
    za, zb = is_zero(f, a), is_zero(f, b)
    if op in ("add", "sub"):
        if op == "sub":
            sb = not sb
        if ia or ib:
            if ia and ib and sa != sb:
                env.flags = env.flags & ~DE | IE
                return f.indefinite
            negative = sa if ia else sb
            return (f.sign if negative else 0) | f.emask << f.mbits
        va, vb = value(f, a), value(f, b) if op == "add" else -value(f, b)
        v = va + vb
        if v == 0:
            if za and zb and sa == sb:
                return f.sign if sa else 0
            return f.sign if env.rc == 1 else 0                # exact zero sum: -0 only rounding down
        return env.round(f, v)
    if op == "mul":
        s = f.sign if sa != sb else 0
        if ia or ib:
            if za or zb:
                env.flags = env.flags & ~DE | IE
                return f.indefinite
            return s | f.emask << f.mbits
        if za or zb:
            return s
        return env.round(f, value(f, a) * value(f, b))
    if op == "div":
        s = f.sign if sa != sb else 0
        if ia and ib or za and zb:
            env.flags = env.flags & ~DE | IE
            return f.indefinite
        if ia:
            return s | f.emask << f.mbits
        if ib:
            return s
        if zb:
            env.flags |= ZE
            return s | f.emask << f.mbits
        if za:
            return s
        return env.round(f, value(f, a) / value(f, b))
    raise ValueError(op)


def _sqrt_round(f: Fmt, v: Fraction, env: Env) -> int:
    """Correctly rounded square root of a positive rational. Scaled by 4**k so the integer root
    has at least mbits + 3 bits; when the root is not exact the true value lies strictly between
    root and root + 1, and root + 1/2 stands for it (a sticky bit): both round alike."""
    k = f.mbits + 4 - _floor_log2(v) // 2
    scaled = v * Fraction(4) ** k
    root = math.isqrt(scaled.numerator // scaled.denominator)
    exact = scaled.denominator == 1 and root * root == scaled.numerator
    approx = Fraction(root) if exact else Fraction(2 * root + 1, 2)
    return env.round(f, approx / Fraction(2) ** k)


def sqrt(f: Fmt, b: int, env: Env) -> int:
    b = env.src(f, b)
    if is_nan(f, b):
        if is_snan(f, b):
            env.flags |= IE
        env.flags &= ~DE
        return _quiet(f, b)
    if is_zero(f, b):
        return b
    if b & f.sign:
        env.flags = env.flags & ~DE | IE
        return f.indefinite
    if is_inf(f, b):
        return b
    return _sqrt_round(f, value(f, b), env)


def minmax(op: str, f: Fmt, a: int, b: int, env: Env) -> int:
    """MIN/MAX (SDM MINSS/MAXSS Operation): the second source on a NaN in either or on two
    zeros, returned unchanged even when it is an SNaN; IE on any NaN; DE on a denormal."""
    a2, b2 = env.src(f, a), env.src(f, b)
    if is_nan(f, a2) or is_nan(f, b2):
        env.flags = env.flags & ~DE | IE
        return b2
    if is_zero(f, a2) and is_zero(f, b2):
        return b2
    ka, kb = _key(f, a2), _key(f, b2)
    if op == "min":
        return a2 if ka < kb else b2
    return a2 if ka > kb else b2


def _key(f: Fmt, x: int) -> Fraction | float:
    if is_inf(f, x):
        return float("-inf") if x & f.sign else float("inf")
    return value(f, x)


def compare(f: Fmt, a: int, b: int, env: Env, signal_qnan: bool) -> tuple[bool, bool, bool]:
    """(ZF, PF, CF) for COMIS*/UCOMIS*: unordered 111, greater 000, less 001, equal 100."""
    a, b = env.src(f, a), env.src(f, b)
    if is_nan(f, a) or is_nan(f, b):
        if signal_qnan or is_snan(f, a) or is_snan(f, b):
            env.flags = env.flags & ~DE | IE
        else:
            env.flags &= ~DE
        return True, True, True
    ka, kb = _key(f, a), _key(f, b)
    if ka == kb:
        return True, False, False
    return (False, False, False) if ka > kb else (False, False, True)


def to_int(f: Fmt, b: int, bits: int, truncate: bool, env: Env) -> int:
    """CVT(T)S*2SI: the integer indefinite 1 << (bits-1) with IE on NaN or out of range."""
    b = env.src(f, b, denormal_flag=False)
    indefinite = 1 << (bits - 1)
    if is_nan(f, b) or is_inf(f, b):
        env.flags |= IE
        return indefinite
    v = value(f, b)
    neg = v < 0
    n = _round_int(-v if neg else v, neg, 3 if truncate else env.rc)
    r = -n if neg else n
    if not -(1 << (bits - 1)) <= r < 1 << (bits - 1):
        env.flags |= IE
        return indefinite
    if Fraction(r) != v:
        env.flags |= PE
    return r & ((1 << bits) - 1)


def from_int(f: Fmt, x: int, bits: int, env: Env) -> int:
    v = x - (1 << bits) if x >> (bits - 1) & 1 else x
    if v == 0:
        return 0
    return env.round(f, Fraction(v))


def convert(src: Fmt, dst: Fmt, b: int, env: Env) -> int:
    """CVTSS2SD / CVTSD2SS: NaN payloads keep their top bits; SNaN sets IE and is quieted."""
    b = env.src(src, b)
    s = b >> (src.bits - 1) & 1
    sign = dst.sign if s else 0
    if is_nan(src, b):
        if is_snan(src, b):
            env.flags |= IE
        env.flags &= ~DE
        m = b & src.mmask
        m = m << (dst.mbits - src.mbits) if dst.mbits > src.mbits else m >> (src.mbits - dst.mbits)
        return sign | dst.emask << dst.mbits | m | dst.quiet
    if is_inf(src, b):
        return sign | dst.emask << dst.mbits
    if is_zero(src, b):
        return sign
    return env.round(dst, value(src, b))


def fma(f: Fmt, a: int, b: int, c: int, negate_product: bool, negate_addend: bool, env: Env) -> int:
    """(+-a*b) +- c with one rounding (SDM VFMADD*, VFMSUB*, VFNMADD*, VFNMSUB*; Sprint 049). a, b, c are the
    multiplicands and the addend as the form orders them; NaN priority is taken from the instruction's operand
    order by the caller (the first NaN of `nan_order`), an SNaN sets IE; inf*0 and inf - inf are invalid."""
    a, b, c = env.src(f, a), env.src(f, b), env.src(f, c)
    if any(is_nan(f, x) for x in (a, b, c)):
        raise ValueError("fma: NaN operands are resolved by fma_lane")
    sp = bool(a & f.sign) != bool(b & f.sign) != negate_product
    sc = bool(c & f.sign) != negate_addend
    ia, ib, ic = is_inf(f, a), is_inf(f, b), is_inf(f, c)
    za, zb = is_zero(f, a), is_zero(f, b)
    if (ia and zb) or (ib and za):
        env.flags = env.flags & ~DE | IE
        return f.indefinite
    if ia or ib:
        if ic and sc != sp:
            env.flags = env.flags & ~DE | IE
            return f.indefinite
        return (f.sign if sp else 0) | f.emask << f.mbits
    if ic:
        return (f.sign if sc else 0) | f.emask << f.mbits
    product = value(f, a) * value(f, b)
    product = -product if negate_product else product
    addend = -value(f, c) if negate_addend else value(f, c)
    v = product + addend
    if v == 0:
        pz = product == 0
        if pz and is_zero(f, c) and sp == sc:
            return f.sign if sp else 0
        return f.sign if env.rc == 1 else 0                    # exact zero sum: -0 only rounding down
    return env.round(f, v)


def fma_lane(f: Fmt, ops: tuple[int, int, int], mul: tuple[int, int], add: int, negate_product: bool,
             negate_addend: bool, env: Env) -> int:
    """One lane of an FMA form: `ops` are (operand 1, operand 2, operand 3) of the instruction, the destination
    first; `mul` and `add` index them. A NaN operand: the first in operand order, quieted; an SNaN sets IE."""
    srcs = tuple(env.src(f, x) for x in ops)
    nans = [x for x in srcs if is_nan(f, x)]
    if nans:
        if any(is_snan(f, x) for x in nans):
            env.flags |= IE
        a, b = srcs[mul[0]], srcs[mul[1]]
        if (is_inf(f, a) and is_zero(f, b)) or (is_inf(f, b) and is_zero(f, a)):
            env.flags |= IE                                    # inf * 0 is invalid whatever the addend
        env.flags &= ~DE
        return _quiet(f, nans[0])
    return fma(f, srcs[mul[0]], srcs[mul[1]], srcs[add], negate_product, negate_addend, env)


# CMPPS/CMPSD predicates 0-31 (SDM CMPPD, Table 3-1): (relation, true when unordered, signals on QNaN)
CMP_PREDICATES = [("eq", False, False), ("lt", False, True), ("le", False, True), (None, True, False),
                  ("ne", True, False), ("nlt", True, True), ("nle", True, True), (None, False, False),
                  ("eq", True, False), ("nge", True, True), ("ngt", True, True), ("false", False, False),
                  ("ne", False, False), ("ge", False, True), ("gt", False, True), ("true", True, False),
                  ("eq", False, True), ("lt", False, False), ("le", False, False), (None, True, True),
                  ("ne", True, True), ("nlt", True, False), ("nle", True, False), (None, False, True),
                  ("eq", True, True), ("nge", True, False), ("ngt", True, False), ("false", False, True),
                  ("ne", False, True), ("ge", False, False), ("gt", False, False), ("true", True, True)]


def cmp_predicate(f: Fmt, a: int, b: int, imm: int, env: Env) -> bool:
    """CMPSD/CMPPD's predicate `imm` on a, b: (relation, result when unordered, signals on QNaN)."""
    rel, unordered_result, signals = CMP_PREDICATES[imm & 31]
    a, b = env.src(f, a), env.src(f, b)
    if is_nan(f, a) or is_nan(f, b):
        env.flags &= ~DE
        if signals or is_snan(f, a) or is_snan(f, b):
            env.flags |= IE
        if rel is None:                                        # UNORD (3, 19) true, ORD (7, 23) false
            return unordered_result
        return unordered_result if rel not in ("false", "true") else rel == "true"
    if rel is None:
        return not unordered_result
    ka, kb = _key(f, a), _key(f, b)
    return {"eq": ka == kb, "lt": ka < kb, "le": ka <= kb, "ne": ka != kb, "nlt": not ka < kb,
            "nle": not ka <= kb, "nge": not ka >= kb, "ngt": not ka > kb, "ge": ka >= kb, "gt": ka > kb,
            "false": False, "true": True}[rel]


def lanes(x: int, width: int, count: int) -> list[int]:
    return [x >> (width * i) & ((1 << width) - 1) for i in range(count)]


def join(parts: list[int], width: int) -> int:
    return sum(p << (width * i) for i, p in enumerate(parts))


ARITH = {"ADD": "add", "SUB": "sub", "MUL": "mul", "DIV": "div"}


def execute(name: str, dst: int, src: int, mxcsr: int) -> tuple[int, int]:
    """One legacy-SSE arithmetic instruction (ADD/SUB/MUL/DIV/SQRT/MIN/MAX in SS/SD/PS/PD) on a
    128-bit destination and a source (128-bit, or the scalar in its low bits). Returns the new
    destination and MXCSR; raises SimdFault (MXCSR flags already merged into .flags) when an
    exception is unmasked."""
    base, kind = name[:-2], name[-2:]
    f = F32 if kind[1] == "S" else F64
    count = 1 if kind[0] == "S" else 128 // f.bits
    env = Env(mxcsr)
    da, sb = lanes(dst, f.bits, 128 // f.bits), lanes(src, f.bits, 128 // f.bits)
    out = list(da)
    lane_flags = 0
    for i in range(count):
        env.flags = 0
        if base in ARITH:
            out[i] = arith(ARITH[base], f, da[i], sb[i], env)
        elif base == "SQRT":
            out[i] = sqrt(f, sb[i], env)
        elif base in ("MIN", "MAX"):
            out[i] = minmax(base.lower(), f, da[i], sb[i], env)
        else:
            raise ValueError(name)
        lane_flags |= env.flags
    env.flags = lane_flags
    env.check()
    return join(out, f.bits), mxcsr | lane_flags
