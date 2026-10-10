"""string_compare.py — the SSE4.2 packed string compares, PCMPISTRI and its three siblings (Sprint 047).

Intel SDM Vol. 2B, PCMPISTRI, and Vol. 2A §4.1 ("Imm8 Control Byte Operation for PCMPESTRI / PCMPESTRM /
PCMPISTRI / PCMPISTRM"). imm8 bits 1:0 pick the elements (unsigned bytes, unsigned words, signed bytes,
signed words), bits 3:2 the aggregation (equal any, ranges, equal each, equal ordered), bits 5:4 the
polarity, bit 6 the index (least or most significant set bit) or the mask's form. Element i of the
first operand (xmm1) is compared with element j of the second (xmm2/m128); an element past its operand's
length is invalid, and Table 4-7 overrides a comparison that touches one. The result IntRes2 gives ECX
(the index, or the element count when it is zero) or XMM0 (the mask), and the flags: CF = IntRes2 != 0,
ZF = the second operand is shorter than the element count, SF = the first is, OF = IntRes2 bit 0, AF and PF 0.
"""

from __future__ import annotations

EQUAL_ANY, RANGES, EQUAL_EACH, EQUAL_ORDERED = 0, 1, 2, 3


def elements(v: int, imm: int) -> list[int]:
    """The 16 bytes or 8 words of a 128-bit value, signed when imm8 bit 1 says so."""
    words, signed = imm & 1, imm >> 1 & 1
    bits = 16 if words else 8
    out = [(v >> (bits * i)) & ((1 << bits) - 1) for i in range(128 // bits)]
    return [e - (1 << bits) if signed and e >> (bits - 1) else e for e in out] if signed else out


def implicit_length(v: int, imm: int) -> int:
    """The index of the first zero element, or the element count."""
    e = elements(v, imm)
    return next((i for i, x in enumerate(e) if x == 0), len(e))


def explicit_length(reg: int, imm: int, wide: bool) -> int:
    """EAX or EDX (RAX or RDX with REX.W) as a signed length: its absolute value, saturated to the count."""
    n = 8 if imm & 1 else 16
    bits = 64 if wide else 32
    v = reg & ((1 << bits) - 1)
    v = v - (1 << bits) if v >> (bits - 1) else v
    return min(abs(v), n)


def _override(aggregation: int, valid1: bool, valid2: bool, result: bool) -> bool:
    """SDM Table 4-7: xmm1's element `valid1`, xmm2's `valid2`."""
    if valid1 and valid2:
        return result
    if not valid1 and not valid2:
        return aggregation in (EQUAL_EACH, EQUAL_ORDERED)
    if not valid1:
        return aggregation == EQUAL_ORDERED
    return False


def compare(src1: int, src2: int, imm: int, length1: int, length2: int) -> tuple[int, dict[str, bool]]:
    """(IntRes2, flags) for xmm1 = src1 with `length1` valid elements and xmm2/m128 = src2 with `length2`."""
    a, b = elements(src1, imm), elements(src2, imm)
    n = len(a)
    aggregation = imm >> 2 & 3

    def boolres(j: int, i: int) -> bool:          # xmm2's element j against xmm1's element i
        if aggregation == RANGES:
            return b[j] >= a[i] if i % 2 == 0 else b[j] <= a[i]
        return b[j] == a[i]

    def over(j: int, i: int) -> bool:
        return _override(aggregation, i < length1, j < length2, boolres(j, i))

    res1 = 0
    for j in range(n):
        if aggregation == EQUAL_ANY:
            bit = any(over(j, i) for i in range(n))
        elif aggregation == RANGES:
            bit = any(over(j, i) and over(j, i + 1) for i in range(0, n, 2))
        elif aggregation == EQUAL_EACH:
            bit = over(j, j)
        else:
            bit = all(over(j + i, i) for i in range(n - j))
        res1 |= int(bit) << j
    polarity = imm >> 4 & 3
    full = (1 << n) - 1
    if polarity == 1:
        res2 = res1 ^ full
    elif polarity == 3:
        res2 = res1 ^ ((1 << length2) - 1)
    else:
        res2 = res1
    flags = {"C": res2 != 0, "Z": length2 < n, "S": length1 < n, "O": bool(res2 & 1), "A": False, "P": False}
    return res2, flags


def index(res2: int, imm: int) -> int:
    """PCMP?STRI's ECX: the least (imm8 bit 6 clear) or most significant set bit, else the element count."""
    n = 8 if imm & 1 else 16
    if res2 == 0:
        return n
    return res2.bit_length() - 1 if imm >> 6 & 1 else (res2 & -res2).bit_length() - 1


def mask(res2: int, imm: int) -> int:
    """PCMP?STRM's XMM0: IntRes2 in the low bits (imm8 bit 6 clear), or each bit widened to its element."""
    if not imm >> 6 & 1:
        return res2
    words = imm & 1
    bits, n = (16, 8) if words else (8, 16)
    return sum(((1 << bits) - 1) << (bits * i) for i in range(n) if res2 >> i & 1)
