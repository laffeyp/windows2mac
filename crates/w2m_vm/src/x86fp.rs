// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! SSE floating point on bit patterns under MXCSR (Sprint 178), carried function for function from `src/w2m/x86fp.py`
//! (Sprint 039): every result computed exactly and rounded once in MXCSR.RC's mode, `Fraction` as `num-rational`'s
//! `BigRational`, as `x87.rs` carries `x87.py`. The sources are `x86fp.py`'s docstring's: IEEE 754-2019, Intel SDM Vol
//! 1 10.2.3, 11.5 and Table 4-7, glibc's TININESS_AFTER_ROUNDING for x86, and Vol 2's pages. `fma` and `fma_lane` come
//! with the VEX slice.

use num_bigint::BigInt;
use num_integer::Integer;
use num_rational::BigRational;
use num_traits::{One, Signed, ToPrimitive, Zero};

type Q = BigRational;

pub const IE: u32 = 1;
pub const DE: u32 = 2;
pub const ZE: u32 = 4;
pub const OE: u32 = 8;
pub const UE: u32 = 16;
pub const PE: u32 = 32;
pub const DAZ: u32 = 1 << 6;
pub const FTZ: u32 = 1 << 15;
const MASK_SHIFT: u32 = 7;

/// An unmasked SIMD floating-point exception (#XM): the destination is not written; `flags` are those the fault
/// reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SimdFault {
    pub flags: u32,
}

/// An IEEE format: binary32, binary64.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Fmt {
    pub bits: u32,
    pub ebits: u32,
    pub mbits: u32,
}

impl Fmt {
    fn bias(self) -> i64 {
        (1 << (self.ebits - 1)) - 1
    }
    fn emin(self) -> i64 {
        1 - self.bias()
    }
    fn emax(self) -> i64 {
        self.bias()
    }
    fn quiet(self) -> u64 {
        1 << (self.mbits - 1)
    }
    fn emask(self) -> u64 {
        (1 << self.ebits) - 1
    }
    fn mmask(self) -> u64 {
        (1 << self.mbits) - 1
    }
    fn sign(self) -> u64 {
        1 << (self.bits - 1)
    }
    pub fn indefinite(self) -> u64 {
        self.sign() | self.emask() << self.mbits | self.quiet()
    }
    fn max_finite(self) -> Q {
        int((1i128 << (self.mbits + 1)) - 1) * pow2(self.emax() - i64::from(self.mbits))
    }
    pub fn mask(self) -> u64 {
        if self.bits == 64 {
            u64::MAX
        } else {
            (1 << self.bits) - 1
        }
    }
}

pub const F32: Fmt = Fmt {
    bits: 32,
    ebits: 8,
    mbits: 23,
};
pub const F64: Fmt = Fmt {
    bits: 64,
    ebits: 11,
    mbits: 52,
};

fn int(n: i128) -> Q {
    Q::from_integer(BigInt::from(n))
}

fn pow2(e: i64) -> Q {
    if e >= 0 {
        Q::from_integer(BigInt::one() << e as usize)
    } else {
        Q::new(BigInt::one(), BigInt::one() << (-e) as usize)
    }
}

fn fields(f: Fmt, x: u64) -> (u64, u64, u64) {
    (x >> (f.bits - 1) & 1, x >> f.mbits & f.emask(), x & f.mmask())
}

pub fn is_nan(f: Fmt, x: u64) -> bool {
    let (_, e, m) = fields(f, x);
    e == f.emask() && m != 0
}

pub fn is_snan(f: Fmt, x: u64) -> bool {
    is_nan(f, x) && x & f.quiet() == 0
}

fn is_denormal(f: Fmt, x: u64) -> bool {
    let (_, e, m) = fields(f, x);
    e == 0 && m != 0
}

fn is_inf(f: Fmt, x: u64) -> bool {
    let (_, e, m) = fields(f, x);
    e == f.emask() && m == 0
}

fn is_zero(f: Fmt, x: u64) -> bool {
    x & (f.sign() - 1) == 0
}

/// `value` (119-124): the exact value of a finite pattern
fn value(f: Fmt, x: u64) -> Q {
    let (s, e, m) = fields(f, x);
    let v = if e == 0 {
        Q::new(BigInt::from(m), BigInt::one() << f.mbits as usize) * pow2(f.emin())
    } else {
        Q::new(BigInt::from((1u64 << f.mbits) | m), BigInt::one() << f.mbits as usize) * pow2(e as i64 - f.bias())
    };
    if s != 0 { -v } else { v }
}

/// `_floor_log2` (127-133)
fn floor_log2(v: &Q) -> i64 {
    let mut e = v.numer().bits() as i64 - v.denom().bits() as i64;
    if pow2(e) > *v {
        e -= 1;
    } else if pow2(e + 1) <= *v {
        e += 1;
    }
    e
}

/// `_round_int` (136-149): a non-negative rational to an integer; rc 0 nearest-even, 1 toward -inf, 2 toward +inf, 3
/// toward zero; `negative` the sign of the value n measures
fn round_int(n: &Q, negative: bool, rc: u32) -> BigInt {
    let (q, r) = n.numer().div_rem(n.denom());
    if r.is_zero() {
        return q;
    }
    if rc == 0 {
        let twice: BigInt = &r * 2u32;
        if twice > *n.denom() || (twice == *n.denom() && q.is_odd()) {
            return q + 1;
        }
        return q;
    }
    if rc == 3 || (rc == 1 && !negative) || (rc == 2 && negative) {
        return q;
    }
    q + 1
}

/// `Env` (152-226): MXCSR for one instruction: reads DAZ, FTZ, RC; collects flags.
#[derive(Debug, Clone, Copy)]
pub struct Env {
    pub mxcsr: u32,
    pub flags: u32,
}

impl Env {
    pub fn new(mxcsr: u32) -> Env {
        Env { mxcsr, flags: 0 }
    }

    fn rc(&self) -> u32 {
        self.mxcsr >> 13 & 3
    }

    fn masks(&self) -> u32 {
        self.mxcsr >> MASK_SHIFT & 0x3F
    }

    /// `src` (167-175): a source operand after DAZ: a denormal becomes a zero of its sign (no DE); otherwise a denormal
    /// sets DE when the instruction lists Denormal
    fn src(&mut self, f: Fmt, x: u64, denormal_flag: bool) -> u64 {
        if is_denormal(f, x) {
            if self.mxcsr & DAZ != 0 {
                return x & f.sign();
            }
            if denormal_flag {
                self.flags |= DE;
            }
        }
        x
    }

    /// `round` (177-216): the pattern nearest `v` (nonzero) in mode RC, with OE, UE, PE and FTZ
    fn round(&mut self, f: Fmt, v: &Q) -> u64 {
        let negative = v.is_negative();
        let a = v.abs();
        let sign = if negative { f.sign() } else { 0 };
        let e = floor_log2(&a);
        let mb = i64::from(f.mbits);
        // tininess after rounding: round to mbits+1 significant bits with unbounded exponent
        let unbounded = Q::from_integer(round_int(&(&a / pow2(e - mb)), negative, self.rc())) * pow2(e - mb);
        let tiny = unbounded < pow2(f.emin());
        let q = pow2(e.max(f.emin()) - mb);
        let n = round_int(&(&a / &q), negative, self.rc());
        let r = Q::from_integer(n) * &q;
        let mut inexact = r != a;
        // an unmasked overflow or underflow delivers no rounded result; Precision then reports whether rounding to the
        // format's precision with an unbounded exponent was inexact (MULSS and CVTSD2SS, run 38010775429)
        let wide_inexact = unbounded != a;
        if r > f.max_finite() {
            let rc = self.rc();
            let to_inf = rc == 0 || (rc == 1 && negative) || (rc == 2 && !negative);
            self.flags |= OE | if self.masks() & OE != 0 || wide_inexact { PE } else { 0 };
            return sign
                | if to_inf {
                    f.emask() << f.mbits
                } else {
                    (f.emask() - 1) << f.mbits | f.mmask()
                };
        }
        if tiny {
            let um = self.masks() & UE != 0;
            if um && self.mxcsr & FTZ != 0 {
                self.flags |= UE | PE;
                return sign;
            }
            if !um {
                self.flags |= UE | if wide_inexact { PE } else { 0 };
                inexact = false;
            } else if inexact {
                self.flags |= UE;
            }
        }
        if inexact {
            self.flags |= PE;
        }
        if r.is_zero() {
            return sign;
        }
        let e2 = floor_log2(&r);
        let whole = |x: Q| x.to_integer().to_u64().expect("a significand");
        if e2 < f.emin() {
            return sign | whole(&r / pow2(f.emin() - mb));
        }
        let mant = whole(&r / pow2(e2 - mb)) & f.mmask();
        sign | ((e2 + f.bias()) as u64) << f.mbits | mant
    }

    /// `check` (218-226): an unmasked pre-computation exception (IE, DE, ZE) stops the instruction before any result,
    /// so only those flags reach MXCSR; otherwise every flag does (AMD EPYC 7763, run 38010775429)
    pub fn check(&self) -> Result<(), SimdFault> {
        let pre = self.flags & (IE | DE | ZE);
        if pre & !self.masks() != 0 {
            return Err(SimdFault { flags: pre });
        }
        if self.flags & !self.masks() & 0x3F != 0 {
            return Err(SimdFault { flags: self.flags });
        }
        Ok(())
    }
}

/// `XM_STATUS` (232-233): the Windows exception code a #XM is delivered with, the first unmasked flag of MXCSR after
/// the fault in this order; Denormal reported as an invalid operation (windows-2025, run 38010775429)
const XM_STATUS: [(u32, u32); 6] = [
    (IE, 0xC000_0090),
    (ZE, 0xC000_008E),
    (DE, 0xC000_0090),
    (OE, 0xC000_0091),
    (UE, 0xC000_0093),
    (PE, 0xC000_008F),
];

/// `xm_status` (236-238)
pub fn xm_status(mxcsr: u32) -> Option<u32> {
    let unmasked = mxcsr & !(mxcsr >> MASK_SHIFT) & 0x3F;
    XM_STATUS.iter().find(|(f, _)| unmasked & f != 0).map(|(_, c)| *c)
}

fn quiet(f: Fmt, x: u64) -> u64 {
    x | f.quiet()
}

/// `_nan_result` (245-252): an SNaN operand sets IE; the first source's NaN wins
fn nan_result(f: Fmt, a: u64, b: Option<u64>, env: &mut Env) -> Option<u64> {
    let nans: Vec<u64> = [Some(a), b].into_iter().flatten().filter(|x| is_nan(f, *x)).collect();
    let first = *nans.first()?;
    if nans.iter().any(|x| is_snan(f, *x)) {
        env.flags |= IE;
    }
    Some(quiet(f, first))
}

/// `arith` (255-306): ADD SUB MUL DIV on two patterns, a the first source
pub fn arith(op: &str, f: Fmt, a: u64, b: u64, env: &mut Env) -> u64 {
    let (a, b) = (env.src(f, a, true), env.src(f, b, true));
    if let Some(n) = nan_result(f, a, Some(b), env) {
        env.flags &= !DE; // a NaN operand outranks Denormal (oracle)
        return n;
    }
    let (sa, mut sb) = (a & f.sign() != 0, b & f.sign() != 0);
    let (ia, ib) = (is_inf(f, a), is_inf(f, b));
    let (za, zb) = (is_zero(f, a), is_zero(f, b));
    let inf = |neg: bool| (if neg { f.sign() } else { 0 }) | f.emask() << f.mbits;
    match op {
        "add" | "sub" => {
            if op == "sub" {
                sb = !sb;
            }
            if ia || ib {
                if ia && ib && sa != sb {
                    env.flags = env.flags & !DE | IE;
                    return f.indefinite();
                }
                return inf(if ia { sa } else { sb });
            }
            let va = value(f, a);
            let vb = if op == "add" { value(f, b) } else { -value(f, b) };
            let v = va + vb;
            if v.is_zero() {
                if za && zb && sa == sb {
                    return if sa { f.sign() } else { 0 };
                }
                return if env.rc() == 1 { f.sign() } else { 0 }; // exact zero sum: -0 only rounding down
            }
            env.round(f, &v)
        }
        "mul" => {
            let s = if sa != sb { f.sign() } else { 0 };
            if ia || ib {
                if za || zb {
                    env.flags = env.flags & !DE | IE;
                    return f.indefinite();
                }
                return s | f.emask() << f.mbits;
            }
            if za || zb {
                return s;
            }
            env.round(f, &(value(f, a) * value(f, b)))
        }
        _ => {
            // "div"
            let s = if sa != sb { f.sign() } else { 0 };
            if ia && ib || za && zb {
                env.flags = env.flags & !DE | IE;
                return f.indefinite();
            }
            if ia {
                return s | f.emask() << f.mbits;
            }
            if ib {
                return s;
            }
            if zb {
                env.flags |= ZE;
                return s | f.emask() << f.mbits;
            }
            if za {
                return s;
            }
            env.round(f, &(value(f, a) / value(f, b)))
        }
    }
}

/// `_sqrt_round` (309-318): the correctly rounded square root of a positive rational, scaled by 4**k so the integer
/// root has at least mbits + 3 bits; an inexact root stands as root + 1/2 (a sticky bit)
fn sqrt_round(f: Fmt, v: &Q, env: &mut Env) -> u64 {
    // Python's // is floored: _floor_log2(v) // 2
    let k = i64::from(f.mbits) + 4 - Integer::div_floor(&floor_log2(v), &2);
    let scaled = v * pow2(2 * k);
    let root = (scaled.numer() / scaled.denom()).sqrt();
    let exact = scaled.denom().is_one() && &root * &root == *scaled.numer();
    let approx = if exact {
        Q::from_integer(root)
    } else {
        Q::new(root * 2 + 1, BigInt::from(2))
    };
    env.round(f, &(approx / pow2(k)))
}

/// `sqrt` (321-335)
pub fn sqrt(f: Fmt, b: u64, env: &mut Env) -> u64 {
    let b = env.src(f, b, true);
    if is_nan(f, b) {
        if is_snan(f, b) {
            env.flags |= IE;
        }
        env.flags &= !DE;
        return quiet(f, b);
    }
    if is_zero(f, b) {
        return b;
    }
    if b & f.sign() != 0 {
        env.flags = env.flags & !DE | IE;
        return f.indefinite();
    }
    if is_inf(f, b) {
        return b;
    }
    sqrt_round(f, &value(f, b), env)
}

/// `_key` (353-356): a pattern's order, infinities beyond every finite value
#[derive(PartialEq, Eq, PartialOrd, Ord)]
enum Key {
    NegInf,
    Finite(Q),
    PosInf,
}

fn key(f: Fmt, x: u64) -> Key {
    if is_inf(f, x) {
        if x & f.sign() != 0 { Key::NegInf } else { Key::PosInf }
    } else {
        Key::Finite(value(f, x))
    }
}

/// `minmax` (338-350): the second source on a NaN in either or on two zeros, unchanged even when an SNaN; IE on any
/// NaN; DE on a denormal
pub fn minmax(op: &str, f: Fmt, a: u64, b: u64, env: &mut Env) -> u64 {
    let (a2, b2) = (env.src(f, a, true), env.src(f, b, true));
    if is_nan(f, a2) || is_nan(f, b2) {
        env.flags = env.flags & !DE | IE;
        return b2;
    }
    if is_zero(f, a2) && is_zero(f, b2) {
        return b2;
    }
    let (ka, kb) = (key(f, a2), key(f, b2));
    if op == "min" {
        if ka < kb { a2 } else { b2 }
    } else if ka > kb {
        a2
    } else {
        b2
    }
}

/// `compare` (359-371): (ZF, PF, CF) for COMIS*/UCOMIS*: unordered 111, greater 000, less 001, equal 100
pub fn compare(f: Fmt, a: u64, b: u64, env: &mut Env, signal_qnan: bool) -> (bool, bool, bool) {
    let (a, b) = (env.src(f, a, true), env.src(f, b, true));
    if is_nan(f, a) || is_nan(f, b) {
        if signal_qnan || is_snan(f, a) || is_snan(f, b) {
            env.flags = env.flags & !DE | IE;
        } else {
            env.flags &= !DE;
        }
        return (true, true, true);
    }
    let (ka, kb) = (key(f, a), key(f, b));
    if ka == kb {
        return (true, false, false);
    }
    if ka > kb {
        (false, false, false)
    } else {
        (false, false, true)
    }
}

/// `to_int` (374-390): CVT(T)S*2SI: the integer indefinite 1 << (bits-1) with IE on NaN or out of range
pub fn to_int(f: Fmt, b: u64, bits: u32, truncate: bool, env: &mut Env) -> u64 {
    let b = env.src(f, b, false);
    let indefinite = 1u64 << (bits - 1);
    if is_nan(f, b) || is_inf(f, b) {
        env.flags |= IE;
        return indefinite;
    }
    let v = value(f, b);
    let neg = v.is_negative();
    let n = round_int(&v.abs(), neg, if truncate { 3 } else { env.rc() });
    let r = if neg { -n } else { n };
    let limit = BigInt::one() << (bits - 1) as usize;
    if !(-limit.clone() <= r && r < limit) {
        env.flags |= IE;
        return indefinite;
    }
    if Q::from_integer(r.clone()) != v {
        env.flags |= PE;
    }
    let r = r.to_i128().expect("in range") as u64;
    if bits == 64 { r } else { r & ((1 << bits) - 1) }
}

/// `from_int` (393-397)
pub fn from_int(f: Fmt, x: u64, bits: u32, env: &mut Env) -> u64 {
    let x = if bits == 64 { x } else { x & ((1 << bits) - 1) };
    let v = if x >> (bits - 1) & 1 != 0 {
        i128::from(x) - (1i128 << bits)
    } else {
        i128::from(x)
    };
    if v == 0 {
        return 0;
    }
    env.round(f, &int(v))
}

/// `convert` (400-416): CVTSS2SD / CVTSD2SS: NaN payloads keep their top bits; SNaN sets IE and is quieted
pub fn convert(src: Fmt, dst: Fmt, b: u64, env: &mut Env) -> u64 {
    let b = env.src(src, b, true);
    let s = b >> (src.bits - 1) & 1;
    let sign = if s != 0 { dst.sign() } else { 0 };
    if is_nan(src, b) {
        if is_snan(src, b) {
            env.flags |= IE;
        }
        env.flags &= !DE;
        let m = b & src.mmask();
        let m = if dst.mbits > src.mbits {
            m << (dst.mbits - src.mbits)
        } else {
            m >> (src.mbits - dst.mbits)
        };
        return sign | dst.emask() << dst.mbits | m | dst.quiet();
    }
    if is_inf(src, b) {
        return sign | dst.emask() << dst.mbits;
    }
    if is_zero(src, b) {
        return sign;
    }
    env.round(dst, &value(src, b))
}

/// `fma` (419-449): (+-a*b) +- c with one rounding; NaN operands are resolved by `fma_lane` first (Sprint 183).
#[allow(clippy::too_many_arguments)]
pub fn fma(f: Fmt, a: u64, b: u64, c: u64, negate_product: bool, negate_addend: bool, env: &mut Env) -> u64 {
    let (a, b, c) = (env.src(f, a, true), env.src(f, b, true), env.src(f, c, true));
    let sp = (a & f.sign() != 0) ^ (b & f.sign() != 0) ^ negate_product;
    let sc = (c & f.sign() != 0) != negate_addend;
    let (ia, ib, ic) = (is_inf(f, a), is_inf(f, b), is_inf(f, c));
    let (za, zb) = (is_zero(f, a), is_zero(f, b));
    let inf = |neg: bool| (if neg { f.sign() } else { 0 }) | f.emask() << f.mbits;
    if (ia && zb) || (ib && za) {
        env.flags = env.flags & !DE | IE;
        return f.indefinite();
    }
    if ia || ib {
        if ic && sc != sp {
            env.flags = env.flags & !DE | IE;
            return f.indefinite();
        }
        return inf(sp);
    }
    if ic {
        return inf(sc);
    }
    let product = value(f, a) * value(f, b);
    let product = if negate_product { -product } else { product };
    let addend = if negate_addend { -value(f, c) } else { value(f, c) };
    let v = &product + addend;
    if v.is_zero() {
        if product.is_zero() && is_zero(f, c) && sp == sc {
            return if sp { f.sign() } else { 0 };
        }
        // exact zero sum: -0 only rounding down
        return if env.rc() == 1 { f.sign() } else { 0 };
    }
    env.round(f, &v)
}

/// `fma_lane` (452-467): one lane of an FMA form, `ops` the instruction's operands 1, 2 and 3, `mul` and `add`
/// indexing them; a NaN operand gives the first of the multiplicands, then the addend, quieted, an SNaN not outranking
/// a QNaN; an SNaN sets IE.
#[allow(clippy::too_many_arguments)]
pub fn fma_lane(
    f: Fmt,
    ops: [u64; 3],
    mul: (usize, usize),
    add: usize,
    negate_product: bool,
    negate_addend: bool,
    env: &mut Env,
) -> u64 {
    let srcs = ops.map(|x| env.src(f, x, true));
    let nans: Vec<u64> = [mul.0, mul.1, add]
        .iter()
        .map(|i| srcs[*i])
        .filter(|x| is_nan(f, *x))
        .collect();
    if let Some(first) = nans.first() {
        if nans.iter().any(|x| is_snan(f, *x)) {
            env.flags |= IE;
        }
        // inf * 0 plus a QNaN addend is not Invalid (Xeon 8573C, set 049b cases 25743, 27363)
        env.flags &= !DE;
        return quiet(f, *first);
    }
    fma(
        f,
        srcs[mul.0],
        srcs[mul.1],
        srcs[add],
        negate_product,
        negate_addend,
        env,
    )
}

/// `execute` (533-560) with `check=False`: the new destination and the lanes' flags, for VEX.256's one check over both
/// halves (Sprint 183).
pub fn execute_unchecked(name: &str, dst: u128, src: u128, mxcsr: u32) -> (u128, u32) {
    let (base, kind) = name.split_at(name.len() - 2);
    let f = if kind.as_bytes()[1] == b'S' { F32 } else { F64 };
    let count = if kind.as_bytes()[0] == b'S' { 1 } else { 128 / f.bits };
    let mut env = Env::new(mxcsr);
    let (da, sb) = (lanes(dst, f.bits, 128 / f.bits), lanes(src, f.bits, 128 / f.bits));
    let mut out = da.clone();
    let mut lane_flags = 0;
    for i in 0..count as usize {
        env.flags = 0;
        out[i] = match base {
            "ADD" => arith("add", f, da[i], sb[i], &mut env),
            "SUB" => arith("sub", f, da[i], sb[i], &mut env),
            "MUL" => arith("mul", f, da[i], sb[i], &mut env),
            "DIV" => arith("div", f, da[i], sb[i], &mut env),
            "SQRT" => sqrt(f, sb[i], &mut env),
            "MIN" => minmax("min", f, da[i], sb[i], &mut env),
            _ => minmax("max", f, da[i], sb[i], &mut env),
        };
        lane_flags |= env.flags;
    }
    (join(&out, f.bits), lane_flags)
}

/// `CMP_PREDICATES` (471-478): predicates 0-31, (relation, true when unordered, signals on QNaN); None is UNORD/ORD
const CMP_PREDICATES: [(Option<&str>, bool, bool); 32] = [
    (Some("eq"), false, false),
    (Some("lt"), false, true),
    (Some("le"), false, true),
    (None, true, false),
    (Some("ne"), true, false),
    (Some("nlt"), true, true),
    (Some("nle"), true, true),
    (None, false, false),
    (Some("eq"), true, false),
    (Some("nge"), true, true),
    (Some("ngt"), true, true),
    (Some("false"), false, false),
    (Some("ne"), false, false),
    (Some("ge"), false, true),
    (Some("gt"), false, true),
    (Some("true"), true, false),
    (Some("eq"), false, true),
    (Some("lt"), false, false),
    (Some("le"), false, false),
    (None, true, true),
    (Some("ne"), true, true),
    (Some("nlt"), true, false),
    (Some("nle"), true, false),
    (None, false, true),
    (Some("eq"), true, true),
    (Some("nge"), true, false),
    (Some("ngt"), true, false),
    (Some("false"), false, true),
    (Some("ne"), false, true),
    (Some("ge"), false, false),
    (Some("gt"), false, false),
    (Some("true"), true, true),
];

/// `cmp_predicate` (481-497)
pub fn cmp_predicate(f: Fmt, a: u64, b: u64, imm: u64, env: &mut Env) -> bool {
    let (rel, unordered_result, signals) = CMP_PREDICATES[(imm & 31) as usize];
    let (a, b) = (env.src(f, a, true), env.src(f, b, true));
    if is_nan(f, a) || is_nan(f, b) {
        env.flags &= !DE;
        if signals || is_snan(f, a) || is_snan(f, b) {
            env.flags |= IE;
        }
        return match rel {
            None => unordered_result,
            Some("false") => false,
            Some("true") => true,
            Some(_) => unordered_result,
        };
    }
    let Some(rel) = rel else {
        return !unordered_result;
    };
    let (ka, kb) = (key(f, a), key(f, b));
    match rel {
        "eq" => ka == kb,
        "lt" => ka < kb,
        "le" => ka <= kb,
        "ne" => ka != kb,
        "nlt" => !(ka < kb),
        "nle" => !(ka <= kb),
        "nge" => !(ka >= kb),
        "ngt" => !(ka > kb),
        "ge" => ka >= kb,
        "gt" => ka > kb,
        "false" => false,
        _ => true,
    }
}

/// `round_to_integer` (500-519): ROUNDSD/ROUNDSS: imm bit 2 takes MXCSR.RC, else imm bits 1:0; imm bit 3 suppresses
/// Precision; an SNaN sets IE and is quieted; inf and zero pass
pub fn round_to_integer(f: Fmt, b: u64, imm: u64, env: &mut Env) -> u64 {
    let b = env.src(f, b, false);
    if is_nan(f, b) {
        if is_snan(f, b) {
            env.flags |= IE;
        }
        env.flags &= !DE;
        return quiet(f, b);
    }
    if is_inf(f, b) || is_zero(f, b) {
        return b;
    }
    let v = value(f, b);
    let negative = v.is_negative();
    let rc = if imm & 4 != 0 { env.rc() } else { (imm & 3) as u32 };
    let n = round_int(&v.abs(), negative, rc);
    let signed = if negative { -n.clone() } else { n.clone() };
    if Q::from_integer(signed.clone()) != v && imm & 8 == 0 {
        env.flags |= PE;
    }
    if n.is_zero() {
        return if negative { f.sign() } else { 0 };
    }
    env.round(f, &Q::from_integer(signed))
}

/// `lanes` (522-523)
pub fn lanes(x: u128, width: u32, count: u32) -> Vec<u64> {
    (0..count).map(|i| (x >> (width * i)) as u64 & mask(width)).collect()
}

/// `join` (526-527)
pub fn join(parts: &[u64], width: u32) -> u128 {
    parts.iter().enumerate().fold(0u128, |acc, (i, p)| {
        acc | u128::from(*p & mask(width)) << (width * i as u32)
    })
}

fn mask(w: u32) -> u64 {
    if w >= 64 { u64::MAX } else { (1 << w) - 1 }
}

/// `execute` (533-560): one legacy-SSE arithmetic instruction (ADD/SUB/MUL/DIV/SQRT/MIN/MAX in SS/SD/PS/PD) on a
/// 128-bit destination and a source: the new destination and MXCSR, or the fault (its flags merged into `.flags`).
pub fn execute(name: &str, dst: u128, src: u128, mxcsr: u32) -> Result<(u128, u32), SimdFault> {
    let (out, lane_flags) = execute_unchecked(name, dst, src, mxcsr);
    Env {
        mxcsr,
        flags: lane_flags,
    }
    .check()?;
    Ok((out, mxcsr | lane_flags))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 1/3 in binary32, round to nearest: 0x3EAAAAAB, inexact
    #[test]
    fn a_third_divides_to_the_nearest_float() {
        let mut env = Env::new(0x1F80);
        assert_eq!(arith("div", F32, 0x3F80_0000, 0x4040_0000, &mut env), 0x3EAA_AAAB);
        assert_eq!(env.flags, PE);
    }

    /// DAZ: a denormal source reads as zero, no DE
    #[test]
    fn daz_reads_a_denormal_as_zero() {
        let mut env = Env::new(0x1F80 | DAZ);
        assert_eq!(arith("add", F32, 1, 0, &mut env), 0);
        assert_eq!(env.flags, 0);
    }
}
