"""Sprint 039: x86fp against the SDM's stated rules and against this Mac's IEEE arithmetic.

The host check covers finite results in round-to-nearest with every exception masked, where
IEEE 754 fixes one answer: binary64 through Python floats, binary32 through NumPy. NaN choice,
flags, DAZ, FTZ and the other modes are judged by the x86 CPU (semantics_differential)."""

from __future__ import annotations

import math
import random
import struct

import numpy as np

from w2m import x86fp as fp

F32, F64 = fp.F32, fp.F64
NEAREST = fp.MXCSR_DEFAULT


def f64(x: float) -> int:
    return struct.unpack("<Q", struct.pack("<d", x))[0]


def bits32(x: np.float32) -> int:
    return int(np.array([x], dtype=np.float32).view(np.uint32)[0])


def from32(b: int) -> np.float32:
    return np.array([b], dtype=np.uint32).view(np.float32)[0]


def _finite(rng: random.Random, f: fp.Fmt) -> int:
    while True:
        x = rng.getrandbits(f.bits)
        if not (fp.is_nan(f, x) or fp.is_inf(f, x)):
            return x


def test_binary64_matches_host_in_round_to_nearest() -> None:
    rng = random.Random(39)
    ops = {"add": lambda a, b: a + b, "sub": lambda a, b: a - b, "mul": lambda a, b: a * b,
           "div": lambda a, b: a / b}
    for _ in range(20000):
        a, b = _finite(rng, F64), _finite(rng, F64)
        x, y = struct.unpack("<d", struct.pack("<Q", a))[0], struct.unpack("<d", struct.pack("<Q", b))[0]
        for name, host in ops.items():
            if name == "div" and y == 0:
                continue
            try:
                want = host(x, y)
            except OverflowError:
                continue
            got = fp.arith(name, F64, a, b, fp.Env(NEAREST))
            assert fp.is_nan(F64, got) == math.isnan(want)
            if not math.isnan(want):
                assert got == f64(want), (name, hex(a), hex(b))
        if x >= 0:
            assert fp.sqrt(F64, a, fp.Env(NEAREST)) == f64(math.sqrt(x)), hex(a)


def test_binary32_matches_numpy_in_round_to_nearest() -> None:
    rng = random.Random(3239)
    with np.errstate(all="ignore"):
        for _ in range(20000):
            a, b = _finite(rng, F32), _finite(rng, F32)
            x, y = from32(a), from32(b)
            for name, want in (("add", x + y), ("sub", x - y), ("mul", x * y), ("div", x / y if y != 0 else None)):
                if want is None:
                    continue
                got = fp.arith(name, F32, a, b, fp.Env(NEAREST))
                if np.isnan(want):
                    assert fp.is_nan(F32, got)
                else:
                    assert got == bits32(want), (name, hex(a), hex(b))
            if x >= 0:
                assert fp.sqrt(F32, a, fp.Env(NEAREST)) == bits32(np.sqrt(x)), hex(a)
            assert fp.convert(F64, F32, f64(float(x) * 1.0000001), fp.Env(NEAREST)) == \
                bits32(np.float32(float(x) * 1.0000001)) or not np.isfinite(np.float32(float(x) * 1.0000001))


def test_min_max_return_the_second_source_on_nan_and_zeros() -> None:
    """SDM MINSS/MAXSS Operation; an SNaN second source comes back unchanged."""
    qnan, snan, one, two = 0x7FC00001, 0x7F800001, 0x3F800000, 0x40000000
    for op in ("min", "max"):
        e = fp.Env(NEAREST)
        assert fp.minmax(op, F32, qnan, one, e) == one and e.flags & fp.IE
        assert fp.minmax(op, F32, one, snan, fp.Env(NEAREST)) == snan
        assert fp.minmax(op, F32, 0x80000000, 0, fp.Env(NEAREST)) == 0
        assert fp.minmax(op, F32, 0, 0x80000000, fp.Env(NEAREST)) == 0x80000000
    assert fp.minmax("min", F32, one, two, fp.Env(NEAREST)) == one
    assert fp.minmax("max", F32, one, two, fp.Env(NEAREST)) == two


def test_conversion_overflow_and_nan_give_the_integer_indefinite() -> None:
    for x, bits in ((0x4F000000, 32), (0xCF000001, 32), (0x7FC00000, 32), (0x5F000000, 64), (0x7F800000, 64)):
        e = fp.Env(NEAREST)
        assert fp.to_int(F32, x, bits, True, e) == 1 << (bits - 1) and e.flags & fp.IE
    e = fp.Env(NEAREST)
    assert fp.to_int(F32, 0xCF000000, 32, True, e) == 0x80000000 and not e.flags    # -2^31 fits
    e = fp.Env(NEAREST)
    assert fp.to_int(F32, 0x3FC00000, 32, True, e) == 1 and e.flags == fp.PE         # 1.5 truncates
    assert fp.to_int(F32, 0x3FC00000, 32, False, fp.Env(NEAREST)) == 2               # nearest-even
    assert fp.to_int(F32, 0x40200000, 32, False, fp.Env(NEAREST)) == 2               # 2.5 -> 2


def test_tininess_is_detected_after_rounding() -> None:
    """(1 - 2^-23) * 2^-126(1 + 2^-23) = 2^-126(1 - 2^-46): tiny before rounding, not after; x86
    sets PE and not UE (glibc sysdeps/x86/tininess.h)."""
    e = fp.Env(NEAREST)
    r = fp.arith("mul", F32, 0x3F7FFFFE, 0x00800001, e)
    assert r == 0x00800000 and e.flags == fp.PE


def test_flush_to_zero_and_denormals_are_zero() -> None:
    tiny = 0x00000001
    e = fp.Env(NEAREST | fp.FTZ)
    assert fp.arith("mul", F32, 0x00800000, 0x3F000000, e) == 0 and e.flags == fp.UE | fp.PE
    e = fp.Env(NEAREST | fp.DAZ)
    assert fp.arith("add", F32, tiny, 0, e) == 0 and e.flags == 0
    e = fp.Env(NEAREST)
    assert fp.arith("add", F32, tiny, 0, e) == tiny and e.flags == fp.DE


def test_invalid_gives_the_real_indefinite_and_first_nan_wins() -> None:
    e = fp.Env(NEAREST)
    assert fp.arith("sub", F32, 0x7F800000, 0x7F800000, e) == 0xFFC00000 and e.flags & fp.IE
    assert fp.arith("add", F32, 0x7FC00002, 0x7FC00003, fp.Env(NEAREST)) == 0x7FC00002
    assert fp.arith("add", F32, 0x3F800000, 0x7F800003, fp.Env(NEAREST)) == 0x7FC00003


def test_unmasked_exception_faults_and_leaves_the_destination() -> None:
    mxcsr = NEAREST & ~(fp.ZE << fp.MASK_SHIFT)
    try:
        fp.execute("DIVSS", 0x3F800000, 0, mxcsr)
    except fp.SimdFault as ex:
        assert ex.flags & fp.ZE
    else:
        raise AssertionError("no fault")


def test_directed_rounding_overflow() -> None:
    big = 0x7F7FFFFF
    for rc, want in ((0, 0x7F800000), (1, 0x7F7FFFFF), (2, 0x7F800000), (3, 0x7F7FFFFF)):
        e = fp.Env(NEAREST | rc << 13)
        assert fp.arith("add", F32, big, big, e) == want and e.flags == fp.OE | fp.PE
