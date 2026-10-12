# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""x87.py — the x87 FPU on bit patterns: the 80-bit format, the register stack and the instructions the game reaches
(Sprint 065).

The classes are the census of 2026-10-12: mono's JIT code uses FLD, FSTP, FMUL, FSQRT, FSIN, FCOS, FADD, FWAIT and
FISTTP; UnityPlayer.dll uses FIADD, FISTP, FIDIVR, FLDCW, FNCLEX and FNSTSW. Every result is computed exactly
(rational arithmetic) and rounded once, as x86fp does for SSE. Sources:

- Intel SDM Vol 1 ch. 8: the register stack (8.1.2: TOP, a push decrements it), the status word (8.1.3: IE DE ZE OE
  UE PE bits 0-5, SF 6, ES 7, C0 8, C1 9, C2 10, TOP 11-13, C3 14, B 15), the control word (8.1.5: masks bits 0-5,
  PC bits 8-9 with 00 single, 10 double, 11 extended; RC bits 10-11), the tag word (8.1.7; FXSAVE's abridged form,
  Vol 2 FXSAVE: one bit a physical register, 1 for not empty), the formats (8.2: the explicit integer bit; unnormals,
  pseudo-denormals, pseudo-NaNs and pseudo-infinities are invalid operands, Table 8-3), stack faults (8.5.1.1:
  IE and SF, C1 1 on overflow and 0 on underflow; masked, the destination gets the real indefinite), precision
  control (8.1.5.2: only FADD, FIADD, FSUB, FISUB, FMUL, FIMUL, FDIV, FIDIV, FDIVR, FIDIVR and FSQRT round to PC).
- Vol 1 4.8.3.7 and Table 4-7: the QNaN floating-point indefinite (sign 1, exponent all ones, 0xC000000000000000);
  for two NaN operands the one with the larger significand, quieted; Table 8-10's integer indefinite.
- Vol 2 each instruction's page: its operands, C1 (set when the result was rounded up, 0 on stack underflow), the
  exceptions it raises; FSIN and FCOS leave ST(0) and set C2 when |x| >= 2^63.
- FSIN and FCOS reduce the argument with pi to 66 bits (Intel's documentation as Bruce Dawson measured it, "Intel
  Underestimates Error Bounds by 1.3 quintillion", 2014-10-09): the result is the sine or cosine of x reduced by
  that pi, rounded to 64 bits.

An unmasked exception has no rows (Windows starts every thread with the control word 0x27F, all masked) and raises
X87Unmasked, which the run reports by name.
"""

from __future__ import annotations

from collections.abc import Callable
from dataclasses import dataclass, field
from fractions import Fraction

IE, DE, ZE, OE, UE, PE, SF, ES = 1, 2, 4, 8, 16, 32, 64, 128
C0, C1, C2, C3, B = 1 << 8, 1 << 9, 1 << 10, 1 << 14, 1 << 15
TOP_SHIFT = 11
EXP_MASK = 0x7FFF
BIAS = 16383
EMIN = 1 - BIAS                                    # the smallest normal exponent
J = 1 << 63                                        # the explicit integer bit
INDEFINITE = 1 << 79 | EXP_MASK << 64 | 0xC000_0000_0000_0000
CW_DEFAULT = 0x037F                                 # FNINIT's; Windows starts threads with 0x027F (double precision)
TINY_BEFORE_ROUNDING = False                        # oracle: the Xeon's cases decide


class X87Unmasked(Exception):
    """An x87 exception the control word does not mask: no row covers it."""


@dataclass
class Fmt:
    """An IEEE memory format the x87 loads and stores: m32fp, m64fp."""
    bits: int
    ebits: int
    mbits: int

    @property
    def bias(self) -> int:
        return (1 << (self.ebits - 1)) - 1


M32, M64 = Fmt(32, 8, 23), Fmt(64, 11, 52)


# ---- the 80-bit format --------------------------------------------------------------------------------------------

def parts(x: int) -> tuple[int, int, int]:
    return x >> 79 & 1, x >> 64 & EXP_MASK, x & ((1 << 64) - 1)


def kind(x: int) -> str:
    """zero, denormal, pseudo_denormal, normal, unnormal, inf, nan, invalid (pseudo-NaN, pseudo-infinity)."""
    _, e, m = parts(x)
    if e == 0:
        if m == 0:
            return "zero"
        return "pseudo_denormal" if m & J else "denormal"
    if e == EXP_MASK:
        if not m & J:
            return "invalid"
        return "inf" if m == J else "nan"
    return "normal" if m & J else "unnormal"


def is_nan(x: int) -> bool:
    return kind(x) == "nan"


def is_snan(x: int) -> bool:
    return is_nan(x) and not x >> 62 & 1


def quiet(x: int) -> int:
    return x | 1 << 62


def value(x: int) -> Fraction:
    """The exact value of a finite pattern (zero, denormal, pseudo-denormal, normal)."""
    s, e, m = parts(x)
    v = Fraction(m) * Fraction(2) ** ((e if e else 1) - BIAS - 63)
    return -v if s else v


def make(sign: int, e: int, m: int) -> int:
    return sign << 79 | e << 64 | m


def infinity(sign: int) -> int:
    return make(sign, EXP_MASK, J)


def _round_int(v: Fraction, rc: int, sign: int) -> int:
    """v >= 0 to an integer in rounding mode rc (0 nearest even, 1 down, 2 up, 3 toward zero), sign the value's."""
    n, r = divmod(v.numerator, v.denominator)
    rem = Fraction(r, v.denominator)
    if rem == 0:
        return n
    if rc == 0:
        return n + (1 if rem > Fraction(1, 2) or (rem == Fraction(1, 2) and n & 1) else 0)
    if rc == 3:
        return n
    up = (rc == 2 and not sign) or (rc == 1 and sign)
    return n + 1 if up else n


def round80(v: Fraction, precision: int, rc: int, sign_of_zero: int = 0) -> tuple[int, int]:
    """v rounded to `precision` significand bits in the 80-bit exponent range: (pattern, flags with C1 for a
    rounded-up magnitude)."""
    if v == 0:
        return make(sign_of_zero, 0, 0), 0
    sign = 1 if v < 0 else 0
    a = -v if sign else v
    e = a.numerator.bit_length() - a.denominator.bit_length()
    if Fraction(2) ** e > a:
        e -= 1
    tiny = e < EMIN
    q = max(e, EMIN) - (precision - 1)                 # the weight of the last significand bit kept
    scaled = a / Fraction(2) ** q
    n = _round_int(scaled, rc, sign)
    flags = 0
    exact = Fraction(n) == scaled
    if not exact:
        flags |= PE
        if Fraction(n) > scaled:
            flags |= C1
    if n >> precision:                                  # rounding carried into a new bit
        n >>= 1
        q += 1
    r = Fraction(n) * Fraction(2) ** q
    re = r.numerator.bit_length() - r.denominator.bit_length() if r else EMIN
    if r and Fraction(2) ** re > r:
        re -= 1
    if r and re > BIAS:                                # overflow (Vol 1 4.9.1.4, masked)
        flags |= OE | PE
        to_inf = rc == 0 or (rc == 2 and not sign) or (rc == 1 and sign)
        if to_inf:
            return infinity(sign), flags | C1
        return make(sign, EXP_MASK - 1, ((1 << precision) - 1) << (64 - precision)), flags & ~C1
    tiny_after = r == 0 or re < EMIN                   # a nonzero value rounded to zero is tiny (set 065: UE)
    if (tiny if TINY_BEFORE_ROUNDING else tiny_after) and flags & PE:
        flags |= UE
    if r == 0:
        return make(sign, 0, 0), flags
    if re < EMIN:                                       # a denormal: biased exponent 0, weight 2^(EMIN-63)
        m = int(r / Fraction(2) ** (EMIN - 63))
        return make(sign, 0, m), flags
    m = int(r / Fraction(2) ** (re - 63))
    return make(sign, re + BIAS, m), flags


def from_ieee(f: Fmt, x: int) -> tuple[int, int]:
    """m32fp or m64fp to 80 bits, exactly: (pattern, flags). An SNaN is quieted with IE, a denormal sets DE."""
    s = x >> (f.bits - 1) & 1
    e = x >> f.mbits & ((1 << f.ebits) - 1)
    m = x & ((1 << f.mbits) - 1)
    if e == (1 << f.ebits) - 1:
        if m == 0:
            return infinity(s), 0
        payload = m << (63 - f.mbits)
        flags = 0 if m >> (f.mbits - 1) else IE
        return make(s, EXP_MASK, J | payload | 1 << 62), flags
    if e == 0 and m == 0:
        return make(s, 0, 0), 0
    v = Fraction(m if e == 0 else m | 1 << f.mbits) * Fraction(2) ** ((e if e else 1) - f.bias - f.mbits)
    r, _ = round80(-v if s else v, 64, 0)
    return r, DE if e == 0 else 0


def to_ieee(f: Fmt, x: int, rc: int) -> tuple[int, int]:
    """80 bits to m32fp or m64fp in mode rc: (pattern, flags with C1 for a rounded-up magnitude)."""
    s, e, m = parts(x)
    k = kind(x)
    emax_bits = (1 << f.ebits) - 1
    if k == "invalid" or k == "unnormal":
        return _ieee_indefinite(f), IE
    if k == "inf":
        return s << (f.bits - 1) | emax_bits << f.mbits, 0
    if k == "nan":
        flags = 0 if m >> 62 & 1 else IE
        payload = (m >> (63 - f.mbits)) & ((1 << f.mbits) - 1) | 1 << (f.mbits - 1)
        return s << (f.bits - 1) | emax_bits << f.mbits | payload, flags
    flags = 0                                      # a denormal source raises no DE in a store (set 065, Xeon 8573C)
    if k == "zero":
        return s << (f.bits - 1), 0
    v = value(x)
    a = -v if s else v
    emin = 1 - f.bias
    ex = a.numerator.bit_length() - a.denominator.bit_length()
    if Fraction(2) ** ex > a:
        ex -= 1
    q = max(ex, emin) - f.mbits
    scaled = a / Fraction(2) ** q
    n = _round_int(scaled, rc, s)
    if Fraction(n) != scaled:
        flags |= PE
        if Fraction(n) > scaled:
            flags |= C1
    if n >> (f.mbits + 1):
        n >>= 1
        q += 1
    r = Fraction(n) * Fraction(2) ** q
    if r == 0:
        if flags & PE:
            flags |= UE
        return s << (f.bits - 1), flags
    re = r.numerator.bit_length() - r.denominator.bit_length()
    if Fraction(2) ** re > r:
        re -= 1
    if re > f.bias:
        flags |= OE | PE
        to_inf = rc == 0 or (rc == 2 and not s) or (rc == 1 and s)
        if to_inf:
            return s << (f.bits - 1) | emax_bits << f.mbits, flags | C1
        return s << (f.bits - 1) | (emax_bits - 1) << f.mbits | ((1 << f.mbits) - 1), flags & ~C1
    tiny = (ex < emin) if TINY_BEFORE_ROUNDING else (re < emin)
    if tiny and flags & PE:
        flags |= UE
    if re < emin:
        return s << (f.bits - 1) | int(r / Fraction(2) ** (emin - f.mbits)), flags
    return s << (f.bits - 1) | (re + f.bias) << f.mbits | int(r / Fraction(2) ** (re - f.mbits)) & ((1 << f.mbits) - 1), flags


def _ieee_indefinite(f: Fmt) -> int:
    return 1 << (f.bits - 1) | ((1 << f.ebits) - 1) << f.mbits | 1 << (f.mbits - 1)


def to_int(x: int, bits: int, rc: int, truncate: bool) -> tuple[int, int]:
    """FIST(T)P: (the integer as an unsigned `bits` pattern, flags). NaN, infinity, an invalid pattern or a value out
    of range gives the integer indefinite 1 << (bits-1) with IE (Vol 1 Table 8-10)."""
    indefinite = 1 << (bits - 1)
    k = kind(x)
    if k in ("nan", "inf", "invalid", "unnormal"):
        return indefinite, IE
    flags = 0                                      # a denormal source raises no DE in FIST(T)P (set 065, Xeon 8573C)
    if k == "zero":
        return 0, flags
    v = value(x)
    s = 1 if v < 0 else 0
    a = -v if s else v
    n = _round_int(a, 3 if truncate else rc, s)
    r = -n if s else n
    if not -(1 << (bits - 1)) <= r < 1 << (bits - 1):
        return indefinite, IE
    if Fraction(n) != a:
        flags |= PE
        if Fraction(n) > a:
            flags |= C1
    return r & ((1 << bits) - 1), flags


def from_int(n: int, bits: int) -> int:
    """A signed integer of `bits` bits, exactly (every int64 fits 64 bits of significand)."""
    v = n - (1 << bits) if n >> (bits - 1) & 1 else n
    r, _ = round80(Fraction(v), 64, 0)
    return r


# ---- arithmetic ---------------------------------------------------------------------------------------------------

def _nan_result(a: int, b: int | None) -> tuple[int, int]:
    """Table 4-7 for x87: an SNaN sets IE; two NaNs give the larger significand (the first on a tie), quieted."""
    flags = IE if is_snan(a) or (b is not None and is_snan(b)) else 0
    nans = [x for x in (a, b) if x is not None and is_nan(x)]
    if len(nans) == 2:
        x, y = nans
        pick = x if (x & ((1 << 64) - 1)) >= (y & ((1 << 64) - 1)) else y
    else:
        pick = nans[0]
    return quiet(pick), flags


def _operand_flags(*xs: int) -> tuple[bool, int]:
    """(invalid, flags): an unsupported pattern is invalid; a denormal operand sets DE."""
    flags = 0
    for x in xs:
        k = kind(x)
        if k in ("invalid", "unnormal"):
            return True, IE
        if k in ("denormal", "pseudo_denormal"):
            flags |= DE
    return False, flags


def add(a: int, b: int, precision: int, rc: int, negate_b: bool = False) -> tuple[int, int]:
    if negate_b:
        b ^= 1 << 79
    if _unsupported(a, b):
        return INDEFINITE, IE
    if is_nan(a) or is_nan(b):
        return _nan_result(a, b)
    bad, flags = _operand_flags(a, b)
    if bad:
        return INDEFINITE, IE
    ka, kb = kind(a), kind(b)
    if ka == "inf" or kb == "inf":
        if ka == kb == "inf" and a >> 79 != b >> 79:
            return INDEFINITE, IE | flags
        return (a if ka == "inf" else b), flags
    v = value(a) + value(b)
    if v == 0:
        sa, sb = a >> 79, b >> 79
        sign = sa if sa == sb else (1 if rc == 1 else 0)
        return make(sign, 0, 0), flags
    r, f = round80(v, precision, rc)
    return r, flags | f


def _unsupported(*xs: int) -> bool:
    """An unnormal, pseudo-NaN, pseudo-infinity or other pattern the x87 does not support: the indefinite with IE,
    before any NaN operand is looked at (set 065, Xeon 8573C: FMUL of an unnormal and a QNaN gives the indefinite)."""
    return any(kind(x) in ("invalid", "unnormal") for x in xs)


def mul(a: int, b: int, precision: int, rc: int) -> tuple[int, int]:
    if _unsupported(a, b):
        return INDEFINITE, IE
    if is_nan(a) or is_nan(b):
        return _nan_result(a, b)
    bad, flags = _operand_flags(a, b)
    if bad:
        return INDEFINITE, IE
    ka, kb = kind(a), kind(b)
    sign = (a ^ b) >> 79 & 1
    if ka == "inf" or kb == "inf":
        if ka == "zero" or kb == "zero":
            return INDEFINITE, IE | flags
        return infinity(sign), flags
    if ka == "zero" or kb == "zero":
        return make(sign, 0, 0), flags
    r, f = round80(value(a) * value(b), precision, rc)
    return r, flags | f


def div(a: int, b: int, precision: int, rc: int) -> tuple[int, int]:
    """a / b."""
    if _unsupported(a, b):
        return INDEFINITE, IE
    if is_nan(a) or is_nan(b):
        return _nan_result(a, b)
    bad, flags = _operand_flags(a, b)
    if bad:
        return INDEFINITE, IE
    ka, kb = kind(a), kind(b)
    sign = (a ^ b) >> 79 & 1
    if ka == "inf" and kb == "inf" or ka == "zero" and kb == "zero":
        return INDEFINITE, IE | flags
    if ka == "inf":
        return infinity(sign), flags
    if kb == "inf":
        return make(sign, 0, 0), flags
    if kb == "zero":
        return infinity(sign), ZE | flags
    if ka == "zero":
        return make(sign, 0, 0), flags
    r, f = round80(value(a) / value(b), precision, rc)
    return r, flags | f


def sqrt(a: int, precision: int, rc: int) -> tuple[int, int]:
    if _unsupported(a):
        return INDEFINITE, IE
    if is_nan(a):
        return _nan_result(a, None)
    bad, flags = _operand_flags(a)
    if bad:
        return INDEFINITE, IE
    k = kind(a)
    if k == "zero":
        return a, flags
    if a >> 79:
        return INDEFINITE, IE | flags
    if k == "inf":
        return a, flags
    v = value(a)
    import math
    # exact sqrt to far more bits than any precision, with a sticky bit, then one rounding
    scale = 256
    num, den = v.numerator, v.denominator
    n = num * (1 << (2 * scale)) * den
    root = math.isqrt(n)
    approx = Fraction(root, (1 << scale) * den)
    if root * root != n:
        approx += Fraction(1, (1 << (scale + 1)) * den)         # sticky: strictly between, never on a tie
    r, f = round80(approx, precision, rc)
    return r, flags | f


PI66: Fraction | None = None


def _pi(bits: int) -> Fraction:
    """pi to `bits` fractional bits (Machin's formula), exact enough for the reductions below."""
    getcontext_bits = bits + 16

    def arctan_inv(x: int) -> Fraction:
        total, term, k, x2 = Fraction(0), Fraction(1, x), 0, x * x
        limit = Fraction(1, 1 << getcontext_bits)
        while abs(term) > limit:
            total += term / (2 * k + 1) * (-1) ** k
            term /= x2
            k += 1
        return total
    return 4 * (4 * arctan_inv(5) - arctan_inv(239))


def _pi66() -> Fraction:
    global PI66
    if PI66 is None:
        p = _pi(140)
        PI66 = Fraction(round(p * (1 << 64)), 1 << 64)            # 2 integer bits and 64 fraction bits: 66 bits
    return PI66


def _sin_cos(r: Fraction, bits: int = 160) -> tuple[Fraction, Fraction]:
    """sin and cos of |r| <= pi/4 by their series, each to well past `bits` bits of its own size: sin's terms run
    until they fall below r * 2^-bits (set 065: an argument of 2^-64 needs its r^3/6 term to round as the Xeon does,
    and one of 2^-10681 lies below any fixed limit)."""
    limit = Fraction(1, 1 << bits)
    s, c = Fraction(0), Fraction(0)
    term, k = r, 1
    while term and abs(term) > abs(r) * limit:
        s += term
        term = -term * r * r / ((k + 1) * (k + 2))
        k += 2
    term, k = Fraction(1), 0
    while abs(term) > limit:
        c += term
        term = -term * r * r / ((k + 1) * (k + 2))
        k += 2
    return s, c


def trig(a: int, which: str, rc: int) -> tuple[int | None, int]:
    """FSIN or FCOS of ST(0): (the result, or None for |x| >= 2^63 with C2 set and ST(0) unchanged; flags)."""
    if _unsupported(a):
        return INDEFINITE, IE
    if is_nan(a):
        return _nan_result(a, None)
    bad, flags = _operand_flags(a)
    if bad:
        return INDEFINITE, IE
    k = kind(a)
    if k == "inf":
        return INDEFINITE, IE
    if k == "zero":
        return (a if which == "sin" else make(0, BIAS, J)), flags
    e = a >> 64 & EXP_MASK
    if e < BIAS - 68:
        # |x| < 2^-68: the Xeon 8573C answers x for FSIN and 1 for FCOS in every rounding mode, inexact, C1 clear, and
        # UE when the x it answers is denormal (set 065: every argument from 2^-69 down; from 2^-68 up it rounds
        # sin and cos correctly)
        if which == "sin":                         # a pseudo-denormal comes back normalized, and not tiny
            if kind(a) == "pseudo_denormal":
                return make(a >> 79, 1, a & ((1 << 64) - 1)), flags | PE
            return a, flags | PE | (UE if e == 0 else 0)
        return make(0, BIAS, J), flags | PE
    v = value(a)
    if abs(v) >= Fraction(2) ** 63:
        return None, C2
    half = _pi66() / 2
    n = round(v / half)
    r = v - n * half
    s, c = _sin_cos(r)
    q = n % 4
    if which == "sin":
        out = (s, c, -s, -c)[q]
    else:
        out = (c, -s, -c, s)[q]
    res, f = round80(out, 64, rc)
    return res, flags | f | PE                     # sin and cos of a nonzero number are irrational: always inexact


# ---- the FPU ------------------------------------------------------------------------------------------------------

@dataclass
class Fpu:
    """The register stack: physical registers, TOP, which are empty, and the control and status words."""
    cw: int = CW_DEFAULT
    sw: int = 0
    regs: list[int] = field(default_factory=lambda: [0] * 8)
    empty: list[bool] = field(default_factory=lambda: [True] * 8)

    @property
    def top(self) -> int:
        return self.sw >> TOP_SHIFT & 7

    def _set_top(self, t: int) -> None:
        self.sw = self.sw & ~(7 << TOP_SHIFT) | (t & 7) << TOP_SHIFT

    @property
    def precision(self) -> int:
        return {0: 24, 1: 64, 2: 53, 3: 64}[self.cw >> 8 & 3]       # PC 01 is reserved; it behaves as extended

    @property
    def rc(self) -> int:
        return self.cw >> 10 & 3

    def phys(self, i: int) -> int:
        return (self.top + i) & 7

    def st(self, i: int) -> int | None:
        p = self.phys(i)
        return None if self.empty[p] else self.regs[p]

    def set_st(self, i: int, x: int) -> None:
        p = self.phys(i)
        self.regs[p], self.empty[p] = x, False

    def push(self, x: int) -> None:
        t = (self.top - 1) & 7
        self._set_top(t)
        self.regs[t], self.empty[t] = x, False

    def pop(self) -> None:
        self.empty[self.top] = True
        self._set_top(self.top + 1)

    def raise_flags(self, flags: int, keep: int = 0) -> None:
        """Merge an instruction's flags: the exception flags are sticky; C1 comes from this instruction."""
        self.sw = self.sw & ~C1 | (flags & C1)
        self.sw |= flags & (IE | DE | ZE | OE | UE | PE | SF)
        if self.sw & ~self.cw & 0x3F:
            raise X87Unmasked(f"x87 exception 0x{self.sw & ~self.cw & 0x3F:x} unmasked by control word 0x{self.cw:x}")

    @property
    def abridged_tags(self) -> int:
        return sum(1 << p for p in range(8) if not self.empty[p])

    def set_abridged_tags(self, tags: int) -> None:
        self.empty = [not tags >> p & 1 for p in range(8)]


class StackFault(Exception):
    pass


def _read(fpu: Fpu, i: int) -> int | None:
    """ST(i), or None after a masked stack underflow (IE, SF, C1 0)."""
    x = fpu.st(i)
    if x is None:
        fpu.raise_flags(IE | SF)
    return x


def execute(fpu: Fpu, ic: str, opcode: int, reg: int, rm: int | None, mem_read: Callable[[int], int],
            mem_write: Callable[..., None], ax: list[int]) -> None:
    """One x87 instruction. `rm` is the ST(i) index of a register form, None for a memory form; mem_read(n) and
    mem_write(n, value, offset=0) access the memory operand; ax is [AX] for FNSTSW AX."""
    read, write = mem_read, mem_write
    p, rc = fpu.precision, fpu.rc
    if fpu.sw & ES and ic not in ("FNSTSW", "FNCLEX"):     # a pending unmasked exception: #MF, which has no rows
        raise X87Unmasked(f"{ic} with an unmasked exception pending (status word 0x{fpu.sw:x})")
    if ic == "FWAIT":
        return
    if ic == "FNCLEX":
        fpu.sw &= ~(IE | DE | ZE | OE | UE | PE | SF | ES | B)
        return
    if ic == "FLDCW":
        # bits 7 and 13-15 are reserved and read as 0, bit 6 as 1 (FNSTCW shows 0x027F, 0x037F); a flag the new word
        # unmasks is pending: ES and B set (set 065, Xeon 8573C)
        fpu.cw = read(2) & 0x1F7F | 0x40
        fpu.sw = fpu.sw & ~(ES | B) | (ES | B if fpu.sw & ~fpu.cw & 0x3F else 0)
        return
    if ic == "FNSTSW":                                     # DF E0 stores AX, DD /7 memory
        if opcode == 0xDD:
            write(2, fpu.sw)
        else:
            ax[0] = fpu.sw
        return
    if ic == "FLD":
        if rm is not None:
            src = fpu.st(rm)
            if src is None:
                src, flags = INDEFINITE, IE | SF
            else:
                flags = 0
        elif opcode == 0xDB:
            src, flags = read(10), 0
        else:
            f = M32 if opcode == 0xD9 else M64
            src, flags = from_ieee(f, read(f.bits // 8))
        if fpu.st(7) is not None:                          # the register a push would take is in use: overflow
            fpu.push(INDEFINITE)
            fpu.raise_flags(IE | SF | C1)
            return
        fpu.push(src)
        fpu.raise_flags(flags & ~C1)
        return
    if ic in ("FSTP",):
        x = _read(fpu, 0)
        if x is None:
            x = INDEFINITE
        if rm is not None:
            fpu.set_st(rm, x)
            fpu.raise_flags(0)
        elif opcode == 0xDB:                               # two stores: the significand, then sign and exponent;
            write(8, x & ((1 << 64) - 1))                  # at a page edge the first lands before the second faults
            write(2, x >> 64, 8)                           # (set 065: FSTP m80 across 0x30001000)
            fpu.raise_flags(0)
        else:
            f = M32 if opcode == 0xD9 else M64
            r, flags = to_ieee(f, x, rc)
            write(f.bits // 8, r)
            fpu.raise_flags(flags)
        fpu.pop()
        return
    if ic in ("FISTP", "FISTTP"):
        x = _read(fpu, 0)
        bits = {(0xDF, 3): 16, (0xDB, 3): 32, (0xDF, 7): 64, (0xDF, 1): 16, (0xDB, 1): 32, (0xDD, 1): 64}[(opcode, reg)]
        if x is None:
            n, flags = 1 << (bits - 1), 0
        else:
            n, flags = to_int(x, bits, rc, ic == "FISTTP")
        write(bits // 8, n)
        fpu.raise_flags(flags)
        fpu.pop()
        return
    if ic in ("FADD", "FMUL", "FIADD", "FIDIVR", "FIMUL", "FDIVR"):
        load_flags = 0
        a: int | None
        b: int | None
        if ic in ("FIADD", "FIDIVR", "FIMUL"):
            bits = 32 if opcode == 0xDA else 16
            other = from_int(read(bits // 8), bits)
            dest, a, b = 0, _read(fpu, 0), other
        elif rm is None:
            f = M32 if opcode == 0xD8 else M64
            b, load_flags = from_ieee(f, read(f.bits // 8))     # an SNaN operand is quieted with IE; a denormal DE
            dest, a = 0, _read(fpu, 0)
        elif opcode == 0xD8:                               # ST(0) op= ST(i)
            dest, a, b = 0, _read(fpu, 0), _read(fpu, rm)
        else:                                              # DC: ST(i) op= ST(0)
            dest, a, b = rm, _read(fpu, rm), _read(fpu, 0)
        if a is None or b is None:
            fpu.set_st(dest, INDEFINITE)
            return
        if ic == "FADD" or ic == "FIADD":
            r, flags = add(a, b, p, rc)
        elif ic in ("FMUL", "FIMUL"):
            r, flags = mul(a, b, p, rc)
        elif ic == "FDIVR":                                # the source divided by ST(0) (memory forms; Sprint 065)
            r, flags = div(b, a, p, rc)
        else:                                              # FIDIVR: the integer divided by ST(0)
            r, flags = div(b, a, p, rc)
        if rm is None and ic in ("FADD", "FMUL", "FDIVR"):
            flags |= load_flags & IE                       # an SNaN operand
            if not (is_nan(a) or _unsupported(a)):         # a NaN or unsupported ST(0) decides alone: no DE (set 065)
                flags |= load_flags & DE
        fpu.raise_flags(flags)
        fpu.set_st(dest, r)
        return
    if ic in ("FSQRT", "FSIN", "FCOS"):
        a = _read(fpu, 0)
        if a is None:
            if ic != "FSQRT":
                fpu.sw &= ~C2                              # the argument was in range (set 065)
            fpu.set_st(0, INDEFINITE)
            return
        if ic == "FSQRT":
            r, flags = sqrt(a, p, rc)
            fpu.raise_flags(flags)
            fpu.set_st(0, r)
            return
        res, flags = trig(a, "sin" if ic == "FSIN" else "cos", rc)
        fpu.sw &= ~C2
        if res is None:
            fpu.sw |= C2
            fpu.raise_flags(0)
            return
        fpu.raise_flags(flags)
        fpu.set_st(0, res)
        return
    raise ValueError(f"x87 {ic} is not modelled")
