// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! The x87 FPU on bit patterns (Sprint 175), carried function for function from `src/w2m/x87.py` (Sprint 065): the
//! 80-bit format, the register stack and the 17 classes the game reaches. Every result is computed exactly and
//! rounded once; `x87.py`'s `fractions.Fraction` is `num-rational`'s `BigRational` here, reduced as Fraction is. The
//! sources are `x87.py`'s docstring's: Intel SDM Vol 1 ch. 8 and 4.8.3.7, Vol 2 each class's page, and the 66-bit pi
//! FSIN and FCOS reduce by.
//!
//! An exception the control word does not mask has no rows (Windows starts each thread with 0x27F) and is refused by
//! name, as the Python's `X87Unmasked` is.

use crate::model::Fault;
use num_bigint::BigInt;
use num_integer::Integer;
use num_rational::BigRational;
use num_traits::{One, Signed, ToPrimitive, Zero};
use std::sync::OnceLock;

type Q = BigRational;

pub const IE: u16 = 1;
pub const DE: u16 = 2;
pub const ZE: u16 = 4;
pub const OE: u16 = 8;
pub const UE: u16 = 16;
pub const PE: u16 = 32;
pub const SF: u16 = 64;
pub const ES: u16 = 128;
pub const C1: u16 = 1 << 9;
pub const C2: u16 = 1 << 10;
pub const B: u16 = 1 << 15;
const TOP_SHIFT: u32 = 11;
const EXP_MASK: u32 = 0x7FFF;
const BIAS: i64 = 16383;
/// the smallest normal exponent
const EMIN: i64 = 1 - BIAS;
/// the explicit integer bit
const J: u128 = 1 << 63;
const M64: u128 = u64::MAX as u128;
pub const INDEFINITE: u128 = 1 << 79 | (EXP_MASK as u128) << 64 | 0xC000_0000_0000_0000;
/// oracle: the Xeon's cases decide (x87.py 44)
const TINY_BEFORE_ROUNDING: bool = false;

/// An IEEE memory format the x87 loads and stores: m32fp, m64fp.
#[derive(Clone, Copy)]
struct Fmt {
    bits: u32,
    ebits: u32,
    mbits: u32,
}

impl Fmt {
    fn bias(self) -> i64 {
        (1 << (self.ebits - 1)) - 1
    }
}

const M32F: Fmt = Fmt {
    bits: 32,
    ebits: 8,
    mbits: 23,
};
const M64F: Fmt = Fmt {
    bits: 64,
    ebits: 11,
    mbits: 52,
};

fn unmasked(why: String) -> Fault {
    Fault::Refused(why)
}

// ---- exact numbers ----------------------------------------------------------------------------------------------

fn int(n: i64) -> Q {
    Q::from_integer(BigInt::from(n))
}

fn big(n: u128) -> Q {
    Q::from_integer(BigInt::from(n))
}

/// Fraction(2) ** e
fn pow2(e: i64) -> Q {
    if e >= 0 {
        Q::from_integer(BigInt::one() << e as usize)
    } else {
        Q::new(BigInt::one(), BigInt::one() << (-e) as usize)
    }
}

/// the exponent of a > 0: 2^e <= a < 2^(e+1) (x87.py 134-136)
fn exponent(a: &Q) -> i64 {
    let mut e = a.numer().bits() as i64 - a.denom().bits() as i64;
    if pow2(e) > *a {
        e -= 1;
    }
    e
}

/// Python's `round()` of a Fraction: to the nearest integer, a half to the even one
fn round_half_even(v: &Q) -> BigInt {
    let (floor, rem) = v.numer().div_mod_floor(v.denom());
    let twice: BigInt = &rem * 2u32;
    match twice.cmp(v.denom()) {
        std::cmp::Ordering::Less => floor,
        std::cmp::Ordering::Greater => floor + 1,
        std::cmp::Ordering::Equal => {
            if floor.is_even() {
                floor
            } else {
                floor + 1
            }
        }
    }
}

/// int(r) of r >= 0 that the caller knows to be whole
fn whole(r: &Q) -> u128 {
    r.to_integer().to_u128().expect("an 80-bit significand")
}

// ---- the 80-bit format --------------------------------------------------------------------------------------------

fn parts(x: u128) -> (u32, u32, u128) {
    ((x >> 79 & 1) as u32, (x >> 64) as u32 & EXP_MASK, x & M64)
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Kind {
    Zero,
    Denormal,
    PseudoDenormal,
    Normal,
    Unnormal,
    Inf,
    Nan,
    /// pseudo-NaN, pseudo-infinity
    Invalid,
}

/// `kind` (72-83)
fn kind(x: u128) -> Kind {
    let (_, e, m) = parts(x);
    if e == 0 {
        if m == 0 {
            return Kind::Zero;
        }
        return if m & J != 0 {
            Kind::PseudoDenormal
        } else {
            Kind::Denormal
        };
    }
    if e == EXP_MASK {
        if m & J == 0 {
            return Kind::Invalid;
        }
        return if m == J { Kind::Inf } else { Kind::Nan };
    }
    if m & J != 0 { Kind::Normal } else { Kind::Unnormal }
}

fn is_nan(x: u128) -> bool {
    kind(x) == Kind::Nan
}

fn is_snan(x: u128) -> bool {
    is_nan(x) && x >> 62 & 1 == 0
}

fn quiet(x: u128) -> u128 {
    x | 1 << 62
}

/// `value` (98-102): the exact value of a finite pattern
fn value(x: u128) -> Q {
    let (s, e, m) = parts(x);
    let v = big(m) * pow2(i64::from(if e != 0 { e } else { 1 }) - BIAS - 63);
    if s != 0 { -v } else { v }
}

fn make(sign: u32, e: u32, m: u128) -> u128 {
    u128::from(sign) << 79 | u128::from(e) << 64 | m
}

fn infinity(sign: u32) -> u128 {
    make(sign, EXP_MASK, J)
}

/// `_round_int` (113-124): v >= 0 to an integer in rounding mode rc (0 nearest even, 1 down, 2 up, 3 toward zero),
/// sign the value's
fn round_int(v: &Q, rc: u16, sign: u32) -> BigInt {
    let (n, r) = v.numer().div_rem(v.denom());
    if r.is_zero() {
        return n;
    }
    if rc == 0 {
        let twice: BigInt = &r * 2u32;
        let up = match twice.cmp(v.denom()) {
            std::cmp::Ordering::Greater => true,
            std::cmp::Ordering::Equal => n.is_odd(),
            std::cmp::Ordering::Less => false,
        };
        return if up { n + 1 } else { n };
    }
    if rc == 3 {
        return n;
    }
    let up = (rc == 2 && sign == 0) || (rc == 1 && sign != 0);
    if up { n + 1 } else { n }
}

/// `round80` (127-169): v rounded to `precision` significand bits in the 80-bit exponent range: (pattern, flags with
/// C1 for a rounded-up magnitude)
fn round80(v: &Q, precision: u32, rc: u16, sign_of_zero: u32) -> (u128, u16) {
    if v.is_zero() {
        return (make(sign_of_zero, 0, 0), 0);
    }
    let sign = u32::from(v.is_negative());
    let a = v.abs();
    let e = exponent(&a);
    let tiny = e < EMIN;
    // the weight of the last significand bit kept
    let mut q = e.max(EMIN) - (i64::from(precision) - 1);
    let scaled = &a / pow2(q);
    let mut n = round_int(&scaled, rc, sign);
    let mut flags = 0;
    let n_q = Q::from_integer(n.clone());
    if n_q != scaled {
        flags |= PE;
        if n_q > scaled {
            flags |= C1;
        }
    }
    // rounding carried into a new bit
    if !(&n >> precision as usize).is_zero() {
        n >>= 1usize;
        q += 1;
    }
    let r = Q::from_integer(n) * pow2(q);
    let re = if r.is_zero() { EMIN } else { exponent(&r) };
    if !r.is_zero() && re > BIAS {
        // overflow (Vol 1 4.9.1.4, masked)
        flags |= OE | PE;
        let to_inf = rc == 0 || (rc == 2 && sign == 0) || (rc == 1 && sign != 0);
        if to_inf {
            return (infinity(sign), flags | C1);
        }
        return (
            make(sign, EXP_MASK - 1, ((1u128 << precision) - 1) << (64 - precision)),
            flags & !C1,
        );
    }
    // a nonzero value rounded to zero is tiny (set 065: UE)
    let tiny_after = r.is_zero() || re < EMIN;
    if (if TINY_BEFORE_ROUNDING { tiny } else { tiny_after }) && flags & PE != 0 {
        flags |= UE;
    }
    if r.is_zero() {
        return (make(sign, 0, 0), flags);
    }
    if re < EMIN {
        // a denormal: biased exponent 0, weight 2^(EMIN-63)
        return (make(sign, 0, whole(&(&r / pow2(EMIN - 63)))), flags);
    }
    (make(sign, (re + BIAS) as u32, whole(&(&r / pow2(re - 63)))), flags)
}

/// `from_ieee` (172-187): m32fp or m64fp to 80 bits, exactly. An SNaN is quieted with IE, a denormal sets DE.
fn from_ieee(f: Fmt, x: u64) -> (u128, u16) {
    let s = (x >> (f.bits - 1) & 1) as u32;
    let e = x >> f.mbits & ((1 << f.ebits) - 1);
    let m = x & ((1 << f.mbits) - 1);
    if e == (1 << f.ebits) - 1 {
        if m == 0 {
            return (infinity(s), 0);
        }
        let payload = u128::from(m) << (63 - f.mbits);
        let flags = if m >> (f.mbits - 1) != 0 { 0 } else { IE };
        return (make(s, EXP_MASK, J | payload | 1 << 62), flags);
    }
    if e == 0 && m == 0 {
        return (make(s, 0, 0), 0);
    }
    let mant = if e == 0 { m } else { m | 1 << f.mbits };
    let v = big(u128::from(mant)) * pow2((if e != 0 { e as i64 } else { 1 }) - f.bias() - i64::from(f.mbits));
    let (r, _) = round80(&if s != 0 { -v } else { v }, 64, 0, 0);
    (r, if e == 0 { DE } else { 0 })
}

/// `to_ieee` (190-241): 80 bits to m32fp or m64fp in mode rc: (pattern, flags with C1 for a rounded-up magnitude)
fn to_ieee(f: Fmt, x: u128, rc: u16) -> (u64, u16) {
    let (s, _, m) = parts(x);
    let k = kind(x);
    let emax_bits: u64 = (1 << f.ebits) - 1;
    let sign = u64::from(s) << (f.bits - 1);
    if k == Kind::Invalid || k == Kind::Unnormal {
        return (ieee_indefinite(f), IE);
    }
    if k == Kind::Inf {
        return (sign | emax_bits << f.mbits, 0);
    }
    if k == Kind::Nan {
        let flags = if m >> 62 & 1 != 0 { 0 } else { IE };
        let payload = (m >> (63 - f.mbits)) as u64 & ((1 << f.mbits) - 1) | 1 << (f.mbits - 1);
        return (sign | emax_bits << f.mbits | payload, flags);
    }
    // a denormal source raises no DE in a store (set 065, Xeon 8573C)
    let mut flags = 0;
    if k == Kind::Zero {
        return (sign, 0);
    }
    let a = value(x).abs();
    let emin = 1 - f.bias();
    let ex = exponent(&a);
    let mut q = ex.max(emin) - i64::from(f.mbits);
    let scaled = &a / pow2(q);
    let mut n = round_int(&scaled, rc, s);
    let n_q = Q::from_integer(n.clone());
    if n_q != scaled {
        flags |= PE;
        if n_q > scaled {
            flags |= C1;
        }
    }
    if !(&n >> (f.mbits + 1) as usize).is_zero() {
        n >>= 1usize;
        q += 1;
    }
    let r = Q::from_integer(n) * pow2(q);
    if r.is_zero() {
        if flags & PE != 0 {
            flags |= UE;
        }
        return (sign, flags);
    }
    let re = exponent(&r);
    if re > f.bias() {
        flags |= OE | PE;
        let to_inf = rc == 0 || (rc == 2 && s == 0) || (rc == 1 && s != 0);
        if to_inf {
            return (sign | emax_bits << f.mbits, flags | C1);
        }
        return (sign | (emax_bits - 1) << f.mbits | ((1 << f.mbits) - 1), flags & !C1);
    }
    let tiny = if TINY_BEFORE_ROUNDING { ex < emin } else { re < emin };
    if tiny && flags & PE != 0 {
        flags |= UE;
    }
    if re < emin {
        return (sign | whole(&(&r / pow2(emin - i64::from(f.mbits)))) as u64, flags);
    }
    let mant = whole(&(&r / pow2(re - i64::from(f.mbits)))) as u64 & ((1 << f.mbits) - 1);
    (sign | ((re + f.bias()) as u64) << f.mbits | mant, flags)
}

fn ieee_indefinite(f: Fmt) -> u64 {
    1 << (f.bits - 1) | ((1 << f.ebits) - 1) << f.mbits | 1 << (f.mbits - 1)
}

/// `to_int` (248-269), FIST(T)P: (the integer as an unsigned `bits` pattern, flags). NaN, infinity, an invalid
/// pattern or a value out of range gives the integer indefinite 1 << (bits-1) with IE (Vol 1 Table 8-10).
fn to_int(x: u128, bits: u32, rc: u16, truncate: bool) -> (u64, u16) {
    let indefinite = 1u64 << (bits - 1);
    let k = kind(x);
    if matches!(k, Kind::Nan | Kind::Inf | Kind::Invalid | Kind::Unnormal) {
        return (indefinite, IE);
    }
    // a denormal source raises no DE in FIST(T)P (set 065, Xeon 8573C)
    let mut flags = 0;
    if k == Kind::Zero {
        return (0, flags);
    }
    let v = value(x);
    let s = u32::from(v.is_negative());
    let a = v.abs();
    let n = round_int(&a, if truncate { 3 } else { rc }, s);
    let r = if s != 0 { -n.clone() } else { n.clone() };
    let limit = BigInt::one() << (bits - 1) as usize;
    if !(-limit.clone() <= r && r < limit) {
        return (indefinite, IE);
    }
    let n_q = Q::from_integer(n);
    if n_q != a {
        flags |= PE;
        if n_q > a {
            flags |= C1;
        }
    }
    let r = r.to_i128().expect("in range") as u64;
    (if bits == 64 { r } else { r & ((1 << bits) - 1) }, flags)
}

/// `from_int` (272-276): a signed integer of `bits` bits, exactly
fn from_int(n: u64, bits: u32) -> u128 {
    let v = if n >> (bits - 1) & 1 != 0 {
        i128::from(n) - (1i128 << bits)
    } else {
        i128::from(n)
    };
    round80(&Q::from_integer(BigInt::from(v)), 64, 0, 0).0
}

// ---- arithmetic ---------------------------------------------------------------------------------------------------

/// `_nan_result` (281-290), Table 4-7 for x87: an SNaN sets IE; two NaNs give the larger significand (the first on a
/// tie), quieted
fn nan_result(a: u128, b: Option<u128>) -> (u128, u16) {
    let flags = if is_snan(a) || b.is_some_and(is_snan) { IE } else { 0 };
    let nans: Vec<u128> = [Some(a), b].into_iter().flatten().filter(|x| is_nan(*x)).collect();
    let pick = if let [x, y] = nans[..] {
        if x & M64 >= y & M64 { x } else { y }
    } else {
        nans[0]
    };
    (quiet(pick), flags)
}

/// `_operand_flags` (293-302): (invalid, flags): an unsupported pattern is invalid; a denormal operand sets DE
fn operand_flags(xs: &[u128]) -> (bool, u16) {
    let mut flags = 0;
    for &x in xs {
        match kind(x) {
            Kind::Invalid | Kind::Unnormal => return (true, IE),
            Kind::Denormal | Kind::PseudoDenormal => flags |= DE,
            _ => {}
        }
    }
    (false, flags)
}

/// `_unsupported` (329-332): an unnormal, pseudo-NaN or pseudo-infinity gives the indefinite with IE, before any NaN
/// operand is looked at (set 065, Xeon 8573C)
fn unsupported(xs: &[u128]) -> bool {
    xs.iter().any(|x| matches!(kind(*x), Kind::Invalid | Kind::Unnormal))
}

/// `add` (305-326), with no caller negating b
fn add(a: u128, b: u128, precision: u32, rc: u16) -> (u128, u16) {
    if unsupported(&[a, b]) {
        return (INDEFINITE, IE);
    }
    if is_nan(a) || is_nan(b) {
        return nan_result(a, Some(b));
    }
    let (bad, flags) = operand_flags(&[a, b]);
    if bad {
        return (INDEFINITE, IE);
    }
    let (ka, kb) = (kind(a), kind(b));
    if ka == Kind::Inf || kb == Kind::Inf {
        if ka == Kind::Inf && kb == Kind::Inf && a >> 79 != b >> 79 {
            return (INDEFINITE, IE | flags);
        }
        return (if ka == Kind::Inf { a } else { b }, flags);
    }
    let v = value(a) + value(b);
    if v.is_zero() {
        let (sa, sb) = ((a >> 79) as u32, (b >> 79) as u32);
        let sign = if sa == sb { sa } else { u32::from(rc == 1) };
        return (make(sign, 0, 0), flags);
    }
    let (r, f) = round80(&v, precision, rc, 0);
    (r, flags | f)
}

/// `mul` (335-352)
fn mul(a: u128, b: u128, precision: u32, rc: u16) -> (u128, u16) {
    if unsupported(&[a, b]) {
        return (INDEFINITE, IE);
    }
    if is_nan(a) || is_nan(b) {
        return nan_result(a, Some(b));
    }
    let (bad, flags) = operand_flags(&[a, b]);
    if bad {
        return (INDEFINITE, IE);
    }
    let (ka, kb) = (kind(a), kind(b));
    let sign = ((a ^ b) >> 79 & 1) as u32;
    if ka == Kind::Inf || kb == Kind::Inf {
        if ka == Kind::Zero || kb == Kind::Zero {
            return (INDEFINITE, IE | flags);
        }
        return (infinity(sign), flags);
    }
    if ka == Kind::Zero || kb == Kind::Zero {
        return (make(sign, 0, 0), flags);
    }
    let (r, f) = round80(&(value(a) * value(b)), precision, rc, 0);
    (r, flags | f)
}

/// `div` (355-377): a / b
fn div(a: u128, b: u128, precision: u32, rc: u16) -> (u128, u16) {
    if unsupported(&[a, b]) {
        return (INDEFINITE, IE);
    }
    if is_nan(a) || is_nan(b) {
        return nan_result(a, Some(b));
    }
    let (bad, flags) = operand_flags(&[a, b]);
    if bad {
        return (INDEFINITE, IE);
    }
    let (ka, kb) = (kind(a), kind(b));
    let sign = ((a ^ b) >> 79 & 1) as u32;
    if ka == Kind::Inf && kb == Kind::Inf || ka == Kind::Zero && kb == Kind::Zero {
        return (INDEFINITE, IE | flags);
    }
    if ka == Kind::Inf {
        return (infinity(sign), flags);
    }
    if kb == Kind::Inf {
        return (make(sign, 0, 0), flags);
    }
    if kb == Kind::Zero {
        return (infinity(sign), ZE | flags);
    }
    if ka == Kind::Zero {
        return (make(sign, 0, 0), flags);
    }
    let (r, f) = round80(&(value(a) / value(b)), precision, rc, 0);
    (r, flags | f)
}

/// `sqrt` (380-406)
fn sqrt(a: u128, precision: u32, rc: u16) -> (u128, u16) {
    if unsupported(&[a]) {
        return (INDEFINITE, IE);
    }
    if is_nan(a) {
        return nan_result(a, None);
    }
    let (bad, flags) = operand_flags(&[a]);
    if bad {
        return (INDEFINITE, IE);
    }
    let k = kind(a);
    if k == Kind::Zero {
        return (a, flags);
    }
    if a >> 79 != 0 {
        return (INDEFINITE, IE | flags);
    }
    if k == Kind::Inf {
        return (a, flags);
    }
    let v = value(a);
    // exact sqrt to far more bits than any precision, with a sticky bit, then one rounding
    let scale = 256usize;
    let (num, den) = (v.numer().clone(), v.denom().clone());
    let n = &num * (BigInt::one() << (2 * scale)) * &den;
    let root = n.sqrt();
    let mut approx = Q::new(root.clone(), (BigInt::one() << scale) * &den);
    if &root * &root != n {
        // sticky: strictly between, never on a tie
        approx += Q::new(BigInt::one(), (BigInt::one() << (scale + 1)) * &den);
    }
    let (r, f) = round80(&approx, precision, rc, 0);
    (r, flags | f)
}

/// `_pi` (412-424): pi to `bits` fractional bits (Machin's formula)
fn pi(bits: usize) -> Q {
    let limit = Q::new(BigInt::one(), BigInt::one() << (bits + 16));
    let arctan_inv = |x: i64| {
        let (mut total, mut term, mut k) = (Q::zero(), Q::new(BigInt::one(), BigInt::from(x)), 0i64);
        let x2 = int(x * x);
        while term.abs() > limit {
            let t = &term / int(2 * k + 1);
            if k % 2 == 0 {
                total += t;
            } else {
                total -= t;
            }
            term /= &x2;
            k += 1;
        }
        total
    };
    int(4) * (int(4) * arctan_inv(5) - arctan_inv(239))
}

/// `_pi66` (427-432): 2 integer bits and 64 fraction bits
fn pi66() -> &'static Q {
    static PI66: OnceLock<Q> = OnceLock::new();
    PI66.get_or_init(|| {
        let p = pi(140);
        Q::new(round_half_even(&(p * pow2(64))), BigInt::one() << 64usize)
    })
}

/// `_sin_cos` (435-451): sin and cos of |r| <= pi/4 by their series, each to well past `bits` bits of its own size
fn sin_cos(r: &Q, bits: usize) -> (Q, Q) {
    let limit = Q::new(BigInt::one(), BigInt::one() << bits);
    let (mut s, mut c) = (Q::zero(), Q::zero());
    let r2 = r * r;
    let sin_limit = r.abs() * &limit;
    let (mut term, mut k) = (r.clone(), 1i64);
    while !term.is_zero() && term.abs() > sin_limit {
        s += &term;
        term = -(&term * &r2) / int((k + 1) * (k + 2));
        k += 2;
    }
    let (mut term, mut k) = (Q::one(), 0i64);
    while term.abs() > limit {
        c += &term;
        term = -(&term * &r2) / int((k + 1) * (k + 2));
        k += 2;
    }
    (s, c)
}

/// `trig` (454-491): FSIN or FCOS of ST(0): (the result, or None for |x| >= 2^63 with C2 set and ST(0) unchanged;
/// flags)
fn trig(a: u128, sin: bool, rc: u16) -> (Option<u128>, u16) {
    if unsupported(&[a]) {
        return (Some(INDEFINITE), IE);
    }
    if is_nan(a) {
        let (r, f) = nan_result(a, None);
        return (Some(r), f);
    }
    let (bad, flags) = operand_flags(&[a]);
    if bad {
        return (Some(INDEFINITE), IE);
    }
    let k = kind(a);
    if k == Kind::Inf {
        return (Some(INDEFINITE), IE);
    }
    if k == Kind::Zero {
        return (Some(if sin { a } else { make(0, BIAS as u32, J) }), flags);
    }
    let e = (a >> 64) as u32 & EXP_MASK;
    if i64::from(e) < BIAS - 68 {
        // |x| < 2^-68: the Xeon 8573C answers x for FSIN and 1 for FCOS in every rounding mode, inexact, C1 clear, and
        // UE when the x it answers is denormal (x87.py 470-477)
        if sin {
            if k == Kind::PseudoDenormal {
                return (Some(make((a >> 79) as u32, 1, a & M64)), flags | PE);
            }
            return (Some(a), flags | PE | if e == 0 { UE } else { 0 });
        }
        return (Some(make(0, BIAS as u32, J)), flags | PE);
    }
    let v = value(a);
    if v.abs() >= pow2(63) {
        return (None, C2);
    }
    let half = pi66() / int(2);
    let n = round_half_even(&(&v / &half));
    let r = &v - Q::from_integer(n.clone()) * &half;
    let (s, c) = sin_cos(&r, 160);
    // Python's n % 4 is floored
    let q = n.mod_floor(&BigInt::from(4)).to_u8().expect("0..4");
    let out = match (sin, q) {
        (true, 0) | (false, 3) => s,
        (true, 1) | (false, 0) => c,
        (true, 2) | (false, 1) => -s,
        _ => -c,
    };
    let (res, f) = round80(&out, 64, rc, 0);
    // sin and cos of a nonzero number are irrational: always inexact
    (Some(res), flags | f | PE)
}

// ---- the FPU ------------------------------------------------------------------------------------------------------

/// `Fpu` (496-551): the register stack, physical registers, TOP, which are empty, and the control and status words.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fpu {
    pub cw: u16,
    pub sw: u16,
    pub regs: [u128; 8],
    pub empty: [bool; 8],
}

impl Fpu {
    pub fn new(cw: u16, sw: u16, regs: [u128; 8], abridged_tags: u8) -> Fpu {
        Fpu {
            cw,
            sw,
            regs,
            empty: std::array::from_fn(|p| abridged_tags >> p & 1 == 0),
        }
    }

    fn top(&self) -> usize {
        usize::from(self.sw >> TOP_SHIFT & 7)
    }

    fn set_top(&mut self, t: usize) {
        self.sw = self.sw & !(7 << TOP_SHIFT) | ((t & 7) as u16) << TOP_SHIFT;
    }

    /// PC 01 is reserved; it behaves as extended
    fn precision(&self) -> u32 {
        match self.cw >> 8 & 3 {
            0 => 24,
            2 => 53,
            _ => 64,
        }
    }

    fn rc(&self) -> u16 {
        self.cw >> 10 & 3
    }

    fn phys(&self, i: u8) -> usize {
        (self.top() + usize::from(i)) & 7
    }

    fn st(&self, i: u8) -> Option<u128> {
        let p = self.phys(i);
        (!self.empty[p]).then_some(self.regs[p])
    }

    fn set_st(&mut self, i: u8, x: u128) {
        let p = self.phys(i);
        (self.regs[p], self.empty[p]) = (x, false);
    }

    fn push(&mut self, x: u128) {
        let t = (self.top() + 7) & 7;
        self.set_top(t);
        (self.regs[t], self.empty[t]) = (x, false);
    }

    fn pop(&mut self) {
        let t = self.top();
        self.empty[t] = true;
        self.set_top(t + 1);
    }

    /// `raise_flags` (539-544): the exception flags are sticky; C1 comes from this instruction
    fn raise_flags(&mut self, flags: u16) -> Result<(), Fault> {
        self.sw = self.sw & !C1 | (flags & C1);
        self.sw |= flags & (IE | DE | ZE | OE | UE | PE | SF);
        let pending = self.sw & !self.cw & 0x3F;
        if pending != 0 {
            return Err(unmasked(format!(
                "x87 exception 0x{pending:x} unmasked by control word 0x{:x}",
                self.cw
            )));
        }
        Ok(())
    }

    pub fn abridged_tags(&self) -> u8 {
        (0..8).filter(|p| !self.empty[*p]).map(|p| 1u8 << p).sum()
    }

    /// `_read` (558-563): ST(i), or None after a masked stack underflow (IE, SF, C1 0)
    fn read(&mut self, i: u8) -> Result<Option<u128>, Fault> {
        let x = self.st(i);
        if x.is_none() {
            self.raise_flags(IE | SF)?;
        }
        Ok(x)
    }
}

/// The memory operand of one x87 instruction: `mem_read(n)` and `mem_write(n, value, offset)` of `execute`.
pub trait X87Memory {
    fn read(&mut self, n: usize) -> Result<u128, Fault>;
    fn write(&mut self, n: usize, v: u128, offset: u64) -> Result<(), Fault>;
}

/// `execute` (566-696): one x87 instruction. `rm` is the ST(i) index of a register form, None for a memory form; `ax`
/// is AX for FNSTSW AX.
pub fn execute(
    fpu: &mut Fpu,
    ic: &str,
    opcode: u8,
    reg: u8,
    rm: Option<u8>,
    mem: &mut dyn X87Memory,
    ax: &mut u16,
) -> Result<(), Fault> {
    let (p, rc) = (fpu.precision(), fpu.rc());
    if fpu.sw & ES != 0 && ic != "FNSTSW" && ic != "FNCLEX" {
        // a pending unmasked exception: #MF, which has no rows
        return Err(unmasked(format!(
            "{ic} with an unmasked exception pending (status word 0x{:x})",
            fpu.sw
        )));
    }
    match ic {
        "FWAIT" => Ok(()),
        "FNCLEX" => {
            fpu.sw &= !(IE | DE | ZE | OE | UE | PE | SF | ES | B);
            Ok(())
        }
        "FLDCW" => {
            // bits 7 and 13-15 are reserved and read as 0, bit 6 as 1; a flag the new word unmasks is pending: ES and
            // B set (set 065, Xeon 8573C)
            fpu.cw = mem.read(2)? as u16 & 0x1F7F | 0x40;
            fpu.sw = fpu.sw & !(ES | B) | if fpu.sw & !fpu.cw & 0x3F != 0 { ES | B } else { 0 };
            Ok(())
        }
        "FNSTSW" => {
            // DF E0 stores AX, DD /7 memory
            if opcode == 0xDD {
                mem.write(2, u128::from(fpu.sw), 0)
            } else {
                *ax = fpu.sw;
                Ok(())
            }
        }
        "FLD" => {
            let (src, flags) = if let Some(i) = rm {
                match fpu.st(i) {
                    None => (INDEFINITE, IE | SF),
                    Some(x) => (x, 0),
                }
            } else if opcode == 0xDB {
                (mem.read(10)?, 0)
            } else {
                let f = if opcode == 0xD9 { M32F } else { M64F };
                from_ieee(f, mem.read(f.bits as usize / 8)? as u64)
            };
            if fpu.st(7).is_some() {
                // the register a push would take is in use: overflow
                fpu.push(INDEFINITE);
                return fpu.raise_flags(IE | SF | C1);
            }
            fpu.push(src);
            fpu.raise_flags(flags & !C1)
        }
        "FSTP" => {
            let x = fpu.read(0)?.unwrap_or(INDEFINITE);
            if let Some(i) = rm {
                fpu.set_st(i, x);
                fpu.raise_flags(0)?;
            } else if opcode == 0xDB {
                // two stores: the significand, then sign and exponent; at a page edge the first lands before the
                // second faults (set 065: FSTP m80 across 0x30001000)
                mem.write(8, x & M64, 0)?;
                mem.write(2, x >> 64, 8)?;
                fpu.raise_flags(0)?;
            } else {
                let f = if opcode == 0xD9 { M32F } else { M64F };
                let (r, flags) = to_ieee(f, x, rc);
                mem.write(f.bits as usize / 8, u128::from(r), 0)?;
                fpu.raise_flags(flags)?;
            }
            fpu.pop();
            Ok(())
        }
        "FISTP" | "FISTTP" => {
            let x = fpu.read(0)?;
            let bits = match (opcode, reg) {
                (0xDF, 3) | (0xDF, 1) => 16,
                (0xDB, 3) | (0xDB, 1) => 32,
                (0xDF, 7) | (0xDD, 1) => 64,
                _ => return Err(Fault::Refused(format!("{ic} with opcode 0x{opcode:x} /{reg}"))),
            };
            let (n, flags) = match x {
                None => (1u64 << (bits - 1), 0),
                Some(x) => to_int(x, bits, rc, ic == "FISTTP"),
            };
            mem.write(bits as usize / 8, u128::from(n), 0)?;
            fpu.raise_flags(flags)?;
            fpu.pop();
            Ok(())
        }
        "FADD" | "FMUL" | "FIADD" | "FIDIVR" | "FIMUL" | "FDIVR" => {
            let mut load_flags = 0;
            let (dest, a, b) = if matches!(ic, "FIADD" | "FIDIVR" | "FIMUL") {
                let bits = if opcode == 0xDA { 32 } else { 16 };
                let other = from_int(mem.read(bits as usize / 8)? as u64, bits);
                (0, fpu.read(0)?, Some(other))
            } else if rm.is_none() {
                let f = if opcode == 0xD8 { M32F } else { M64F };
                // an SNaN operand is quieted with IE; a denormal DE
                let (b, lf) = from_ieee(f, mem.read(f.bits as usize / 8)? as u64);
                load_flags = lf;
                (0, fpu.read(0)?, Some(b))
            } else if opcode == 0xD8 {
                // ST(0) op= ST(i)
                let i = rm.unwrap_or(0);
                let a = fpu.read(0)?;
                (0, a, fpu.read(i)?)
            } else {
                // DC: ST(i) op= ST(0)
                let i = rm.unwrap_or(0);
                let a = fpu.read(i)?;
                (i, a, fpu.read(0)?)
            };
            let (Some(a), Some(b)) = (a, b) else {
                fpu.set_st(dest, INDEFINITE);
                return Ok(());
            };
            let (mut r, mut flags) = match ic {
                "FADD" | "FIADD" => add(a, b, p, rc),
                "FMUL" | "FIMUL" => mul(a, b, p, rc),
                // FDIVR: the source divided by ST(0); FIDIVR: the integer divided by ST(0)
                _ => div(b, a, p, rc),
            };
            if rm.is_none() && matches!(ic, "FADD" | "FMUL" | "FDIVR") {
                // an SNaN operand
                flags |= load_flags & IE;
                // a NaN or unsupported ST(0) decides alone, and a division by zero raises ZE alone: no DE (sets 065,
                // 065c)
                if !(is_nan(a) || unsupported(&[a]) || flags & ZE != 0) {
                    flags |= load_flags & DE;
                }
                if load_flags & IE != 0 && is_nan(b) && is_nan(a) && !is_snan(a) {
                    // an SNaN and a QNaN: the QNaN (Vol 1 Table 4-7, x87; set 065c)
                    r = quiet(a);
                }
            }
            fpu.raise_flags(flags)?;
            fpu.set_st(dest, r);
            Ok(())
        }
        "FSQRT" | "FSIN" | "FCOS" => {
            let Some(a) = fpu.read(0)? else {
                if ic != "FSQRT" {
                    // the argument was in range (set 065)
                    fpu.sw &= !C2;
                }
                fpu.set_st(0, INDEFINITE);
                return Ok(());
            };
            if ic == "FSQRT" {
                let (r, flags) = sqrt(a, p, rc);
                fpu.raise_flags(flags)?;
                fpu.set_st(0, r);
                return Ok(());
            }
            let (res, flags) = trig(a, ic == "FSIN", rc);
            fpu.sw &= !C2;
            let Some(res) = res else {
                fpu.sw |= C2;
                return fpu.raise_flags(0);
            };
            fpu.raise_flags(flags)?;
            fpu.set_st(0, res);
            Ok(())
        }
        _ => Err(Fault::Refused(format!("x87 {ic} is not modelled"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// x87.py 431: pi to 66 bits, 0xC90FDAA22168C235 / 2^62 after rounding at bit 64 of the fraction
    #[test]
    fn pi66_is_pi_rounded_to_64_fraction_bits() {
        let scaled = pi66() * pow2(64);
        assert!(scaled.is_integer());
        assert_eq!(format!("{:x}", scaled.to_integer()), "3243f6a8885a308d3");
    }

    /// 1/3 in extended precision, round to nearest: 0x3FFD AAAA...AAAB with C1 (rounded up)
    #[test]
    fn a_third_rounds_up_to_nearest() {
        let (r, f) = round80(&Q::new(BigInt::from(1), BigInt::from(3)), 64, 0, 0);
        assert_eq!(r, 0x3FFD_AAAA_AAAA_AAAA_AAAB);
        assert_eq!(f, PE | C1);
    }
}
