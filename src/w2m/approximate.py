"""approximate.py — RCPPS, RCPSS, RSQRTPS and RSQRTSS as the reference CPU computes them (Sprint 046).

The SDM bounds these results' relative error by 1.5 * 2^-12 and leaves the bits to the CPU; AMD's EPYC
7763, 9V45 and 9V74 agree with each other and not with Intel's Xeon 8573C (probes/approximations). The
tables in specs/approximations.json come from one CPU (scripts/approximation_tables.py): the result of a
normal input is the entry for its mantissa's top 12 bits with the exponent moved; zeros and denormals
give a signed infinity, infinities a signed zero (RSQRT of -inf the default NaN), NaNs are quieted,
negative RSQRT inputs give the default NaN, and a result below the normal range is a signed zero. These
instructions raise no SIMD floating-point exception (SDM).
"""

from __future__ import annotations

import json
from functools import cache
from pathlib import Path

SPEC = Path(__file__).resolve().parents[2] / "specs" / "approximations.json"
DEFAULT_NAN = 0xFFC00000


@cache
def _tables() -> dict:
    return json.loads(SPEC.read_text())


def _special(x: int, rsqrt: bool) -> int | None:
    sign, e, m = x >> 31, (x >> 23) & 0xFF, x & 0x7FFFFF
    if e == 0xFF and m:                                      # NaN: quieted
        return x | 0x400000
    if e == 0xFF:                                            # infinity
        return DEFAULT_NAN if rsqrt and sign else sign << 31
    if e == 0:                                               # zero or denormal: an infinity of its sign
        return sign << 31 | 0x7F800000
    if rsqrt and sign:
        return DEFAULT_NAN
    return None


def rcp(x: int) -> int:
    s = _special(x, False)
    if s is not None:
        return s
    t = _tables()["rcp"][(x >> 11) & 0xFFF]
    e = ((t >> 23) & 0xFF) - (((x >> 23) & 0xFF) - 127)
    return (x & 0x80000000) | (t & 0x7FFFFF | e << 23 if 1 <= e <= 254 else 0)


def rsqrt(x: int) -> int:
    s = _special(x, True)
    if s is not None:
        return s
    e_in = (x >> 23) & 0xFF
    e0 = 127 if (e_in - 127) % 2 == 0 else 128
    t = _tables()["rsqrt_even" if e0 == 127 else "rsqrt_odd"][(x >> 11) & 0xFFF]
    e = ((t >> 23) & 0xFF) - (e_in - e0) // 2
    return t & 0x7FFFFF | e << 23 if 1 <= e <= 254 else 0
