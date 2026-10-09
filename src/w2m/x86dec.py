"""x86dec.py — w2m's own x86-64 decoder (length, mnemonic, XED-style iclass), driven by tables
generated from specs/x86_decode.json (Intel SDM opcode maps). Compared against Intel XED (the
primary reference) and Capstone at every position; every disagreement is reported.

Structure follows Intel SDM Vol 2A sections 2.1-2.3: legacy prefixes, REX, VEX (C4/C5), opcode
(one-byte, 0F, 0F38, 0F3A), ModRM, SIB, displacement, immediate.
"""

from __future__ import annotations

from dataclasses import dataclass

from w2m.x86dec_tables import LOCKABLE, REP_STRING, REPE_STRING, TABLE, Entry

LEGACY = frozenset({0xF0, 0xF2, 0xF3, 0x2E, 0x36, 0x3E, 0x26, 0x64, 0x65, 0x66, 0x67})
MAPS_3 = {0x38: "0f38", 0x3A: "0f3a"}
VEX_MAP = {1: "0f", 2: "0f38", 3: "0f3a"}
EVEX_MAP = {1: "0f", 2: "0f38", 3: "0f3a"}
XOP_MAP = {8: "x8", 9: "x9", 10: "xa"}
VEX_PFX = {0: "none", 1: "66", 2: "f3", 3: "f2"}
MAX_LEN = 15


@dataclass(frozen=True)
class Fields:
    """Decoded fields the operand layer needs (src/w2m/x86ops.py)."""

    prefixes: tuple[int, ...]
    rex: int
    enc: str
    opcode: int
    modrm: int | None
    sib: int | None
    disp: int            # sign-extended displacement value (0 when absent)
    disp_size: int
    imm: bytes           # raw immediate bytes, in encoding order
    opsize: int          # 16, 32, or 64
    asize: int           # 32 or 64 (address size)
    ops: str | None      # the row's operand template (SDM Appendix A notation), if any
    vvvv: int | None


@dataclass(frozen=True)
class Decoded:
    length: int
    mnemonic: str
    iclass: str
    fields: Fields | None = None
    row: str | None = None    # sha256 of the spec row that decoded it (key of x86_decode_confirmations.json)


class DecodeError(ValueError):
    """The spec has no entry for these bytes, or they exceed 15 bytes."""


class InvalidEncoding(DecodeError):
    """The bytes break a validity rule of the spec (LOCK, VEX.vvvv, VEX.L, a prefix before VEX/
    EVEX/XOP, length over 15): the guest observes a fault (v0.2 section 5.1), not an instruction.
    #UD for encoding rules; #GP for length over 15 (SDM Vol 3A 6.15, interrupt 13)."""


class Truncated(DecodeError):
    """The input ends inside the instruction (XED BUFFER_TOO_SHORT). Not a guest fault: the
    bytes that would decide the instruction are not in the input."""


def _pick(cands: tuple[Entry, ...], mpfx: list[str]) -> list[Entry]:
    """Mandatory-prefix selection: the most specific prefix present that has entries wins."""
    for p in mpfx:
        hit = [c for c in cands if c[0] == p]
        if hit:
            return hit
    return []


def decode(b: bytes, features: frozenset[str] | None = None) -> Decoded:
    """Decode one instruction from the start of `b`. `features` is the CPU profile's feature set
    (XED feature names such as BMI1, LZCNT, CET_IBT, CET_SS); None means a CPU with every feature.
    Raises InvalidEncoding (guest fault), Truncated (input too short), or DecodeError (no spec
    row); never IndexError."""
    try:
        d = _decode(b, features)
    except IndexError:
        raise Truncated(f"input of {len(b)} bytes ends inside the instruction") from None
    if d.length > len(b):
        raise Truncated(f"instruction needs {d.length} bytes; input has {len(b)}")
    return d


def _decode(b: bytes, features: frozenset[str] | None) -> Decoded:
    i = 0
    pfx: list[int] = []
    rex = 0
    # Prefixes. A REX byte applies only if it immediately precedes the opcode (or VEX); a
    # legacy prefix after a REX cancels it (SDM Vol 2A 2.2.1).
    while i < MAX_LEN and (b[i] in LEGACY or 0x40 <= b[i] <= 0x4F):
        if 0x40 <= b[i] <= 0x4F:
            rex = b[i]
        else:
            pfx.append(b[i])
            rex = 0
        i += 1
    if i >= MAX_LEN:
        raise InvalidEncoding("prefixes reach 15 bytes; length over 15 (#GP)")
    v = b[i]
    if v in (0x62, 0xC4, 0xC5) or (v == 0x8F and (b[i + 1] & 0x1F) >= 8):
        # In 64-bit mode C4/C5 are always VEX and 62 always EVEX. A 66/F2/F3/F0 or REX before
        # VEX, EVEX or XOP causes #UD (SDM Vol 2A 2.3.2; XED BAD_LEGACY_PREFIX, BAD_REX_PREFIX).
        bad = [f"{p:02x}" for p in pfx if p in (0x66, 0xF2, 0xF3, 0xF0)]
        if bad:
            raise InvalidEncoding(f"legacy prefix {' '.join(bad)} before VEX/EVEX/XOP")
        if rex:
            raise InvalidEncoding(f"REX {rex:02x} before VEX/EVEX/XOP")
    enc = "legacy"
    w = (rex >> 3) & 1
    vl: int | None = None
    vvvv: int | None = None
    if b[i] == 0x62 and not rex:
        # EVEX (SDM Vol 2A 2.7): 62 P0 P1 P2; in 64-bit mode 62 is never BOUND
        p0, p1, p2 = b[i + 1], b[i + 2], b[i + 3]
        if (p0 & 7) not in EVEX_MAP:
            raise DecodeError(f"EVEX map field {p0 & 7}")
        enc, mp, w, pp, vl = "evex", EVEX_MAP[p0 & 7], p1 >> 7, p1 & 3, (p2 >> 5) & 3
        i += 4
        mpfx = [VEX_PFX[pp]]
    elif b[i] == 0x8F and (b[i + 1] & 0x1F) >= 8 and not rex:
        # XOP (AMD APM Vol 6): 8F RXB.mmmmm W.vvvv.L.pp; map field < 8 means POP r/m instead
        p1, p2 = b[i + 1], b[i + 2]
        if (p1 & 0x1F) not in XOP_MAP:
            raise DecodeError(f"XOP map field {p1 & 0x1F}")
        enc, mp, w, pp, vl = "xop", XOP_MAP[p1 & 0x1F], p2 >> 7, p2 & 3, (p2 >> 2) & 1
        i += 3
        mpfx = [VEX_PFX[pp]]
    elif b[i] in (0xC4, 0xC5) and not rex:
        enc = "vex"
        if b[i] == 0xC5:
            p = b[i + 1]
            mp, pp, vl, vvvv = "0f", p & 3, (p >> 2) & 1, (p >> 3) & 0xF
            i += 2
        else:
            p1, p2 = b[i + 1], b[i + 2]
            if (p1 & 0x1F) not in VEX_MAP:
                raise DecodeError(f"VEX map field {p1 & 0x1F}")
            mp, pp, w, vl = VEX_MAP[p1 & 0x1F], p2 & 3, p2 >> 7, (p2 >> 2) & 1
            vvvv = (p2 >> 3) & 0xF
            i += 3
        mpfx = [VEX_PFX[pp]]
    else:
        if b[i] == 0x0F:
            if b[i + 1] in MAPS_3:
                mp = MAPS_3[b[i + 1]]
                i += 2
            else:
                mp = "0f"
                i += 1
        else:
            mp = "1"
        reps = [x for x in pfx if x in (0xF2, 0xF3)]
        mpfx = ([f"{reps[-1]:02x}"] if reps else []) + (["66"] if 0x66 in pfx else []) + ["none"]
    op = b[i]
    i += 1
    cands = TABLE.get((mp, op, enc))
    if not cands:
        raise DecodeError(f"no entry for map {mp} opcode 0x{op:02x}"
                          f"{'' if enc == 'legacy' else f' ({enc})'}")
    chosen = _pick(cands, mpfx)
    sse_family = mp != "1" and any(c[0] != "none" for c in cands)
    if enc == "legacy" and sse_family and chosen and chosen[0][0] == "none" and len(mpfx) > 1:
        # a 66/F2/F3 prefix with no row of its own on an SSE-family opcode: not this instruction
        raise DecodeError(f"no entry for map {mp} opcode 0x{op:02x} with prefixes {mpfx}")
    if not chosen:
        raise DecodeError(f"no entry for map {mp} opcode 0x{op:02x} with prefixes {mpfx}")
    modrm = b[i] if any(c[2] for c in chosen) else None
    if modrm is not None:
        reg, is_reg = (modrm >> 3) & 7, (modrm >> 6) == 3
        chosen = [c for c in chosen if c[6] is None or int(c[6], 16) == modrm]
        chosen = [c for c in chosen if c[1] is None or c[1] == reg]
        chosen = [c for c in chosen if c[9] is None or (c[9] == "reg") == is_reg]
        exact = [c for c in chosen if c[6] is not None]
        chosen = exact or chosen
    chosen = [c for c in chosen if c[7] is None or c[7] == w]
    chosen = [c for c in chosen if c[8] is None or (enc in ("vex", "xop") and c[8] == vl)]
    if not chosen:
        raise DecodeError(f"no entry for map {mp} opcode 0x{op:02x} modrm "
                          f"{'none' if modrm is None else hex(modrm)} w {w} l {vl}")
    _, _, has_modrm, imm, mn, sizes, _, _, _, _, iclass, isizes, requires, fallback, lock_ok, vreq, opst, osz64, row = chosen[0]
    if 0xF0 in pfx and not (lock_ok and modrm is not None and modrm >> 6 != 3):
        raise InvalidEncoding(f"LOCK prefix on {mn} {'register' if modrm is not None and modrm >> 6 == 3 else ''} form")
    if enc == "vex" and vreq == "1111" and vvvv != 0xF:
        raise InvalidEncoding(f"VEX.vvvv must be 1111b for {mn}")
    sib: int | None = None
    disp_size = 0
    if has_modrm:
        assert modrm is not None
        i += 1
        mod, rm = modrm >> 6, modrm & 7
        if mod != 3:
            if rm == 4:
                sib = b[i]
                i += 1
                if (sib & 7) == 5 and mod == 0:
                    disp_size = 4
            if mod == 0 and rm == 5:
                disp_size = 4
            elif mod == 1:
                disp_size = 1
            elif mod == 2:
                disp_size = 4
    disp = int.from_bytes(b[i:i + disp_size], "little", signed=True) if disp_size else 0
    i += disp_size
    opsize = "64" if w and enc == "legacy" else "16" if 0x66 in pfx else "32"
    imm_size = {None: 0, "b": 1, "w": 2, "d": 4,
                "z": 2 if opsize == "16" else 4,
                "v": 8 if opsize == "64" else 2 if opsize == "16" else 4,
                "moffs": 4 if 0x67 in pfx else 8}[imm]
    imm_bytes = bytes(b[i:i + imm_size])
    i += imm_size
    if i > MAX_LEN:
        raise DecodeError(f"length {i} exceeds 15")
    if sizes:
        mn = dict(sizes)[opsize]
    if isizes:
        iclass = dict(isizes)[opsize]
    if requires and fallback and features is not None and requires not in features:
        mn, iclass = fallback                         # same bytes, older CPU (spec `features` note)
    if mp == "1" and op == 0x90 and rex & 1 and 0xF3 not in pfx:
        mn, iclass = "xchg", "XCHG"                   # 41 90: xchg r8, rax (SDM: NOP is 90 without REX.B)
    if 0xF0 in pfx and mn in LOCKABLE:
        mn = "lock " + mn
        if iclass != "XCHG":                            # XED: XCHG with memory is locked either way
            iclass += "_LOCK"
    # REP applies to the one-byte string opcodes only. "movsd" also names SSE F2 0F 10/11,
    # where F2 is a mandatory prefix, not a repeat (SDM Vol 2B MOVSD notes both instructions).
    if mp == "1" and mn in REP_STRING:
        if 0xF3 in pfx:
            word = "repe" if mn in REPE_STRING else "rep"
            mn, iclass = f"{word} {mn}", f"{word.upper()}_{iclass}"
        elif 0xF2 in pfx:
            # REPNE has meaning only for cmps/scas; on other string ops F2 repeats like REP (XED REP_*)
            mn = "repne " + mn
            iclass = ("REPNE_" if mn[6:] in REPE_STRING else "REP_") + iclass
    # SDM Table A-1: d64 defaults to 64 bits (66 selects 16); f64 forces 64 (66 ignored, Intel)
    eff = 64 if osz64 == "f64" or (osz64 == "d64" and opsize == "32") else int(opsize)
    fields = Fields(tuple(pfx), rex, enc, op, modrm, sib, disp, disp_size, imm_bytes, eff,
                    32 if 0x67 in pfx else 64, opst, vvvv)
    return Decoded(i, mn, iclass, fields, row)
