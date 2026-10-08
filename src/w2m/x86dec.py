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
VEX_PFX = {0: "none", 1: "66", 2: "f3", 3: "f2"}
MAX_LEN = 15


@dataclass(frozen=True)
class Decoded:
    length: int
    mnemonic: str
    iclass: str


class DecodeError(ValueError):
    """The spec has no entry for these bytes, or they exceed 15 bytes."""


def _pick(cands: tuple[Entry, ...], mpfx: list[str]) -> list[Entry]:
    """Mandatory-prefix selection: the most specific prefix present that has entries wins."""
    for p in mpfx:
        hit = [c for c in cands if c[0] == p]
        if hit:
            return hit
    return []


def decode(b: bytes, features: frozenset[str] | None = None) -> Decoded:
    """Decode one instruction. `features` is the CPU profile's feature set (XED feature names such
    as BMI1, LZCNT, CET_IBT, CET_SS); None means a CPU with every feature."""
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
    vex = False
    w = (rex >> 3) & 1
    vl: int | None = None
    if b[i] in (0xC4, 0xC5) and not rex:
        vex = True
        if b[i] == 0xC5:
            p = b[i + 1]
            mp, pp, vl = "0f", p & 3, (p >> 2) & 1
            i += 2
        else:
            p1, p2 = b[i + 1], b[i + 2]
            if (p1 & 0x1F) not in VEX_MAP:
                raise DecodeError(f"VEX map field {p1 & 0x1F}")
            mp, pp, w, vl = VEX_MAP[p1 & 0x1F], p2 & 3, p2 >> 7, (p2 >> 2) & 1
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
    cands = TABLE.get((mp, op, vex))
    if not cands:
        raise DecodeError(f"no entry for map {mp} opcode 0x{op:02x}{' (vex)' if vex else ''}")
    chosen = _pick(cands, mpfx)
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
    chosen = [c for c in chosen if c[8] is None or c[8] == vl]
    if not chosen:
        raise DecodeError(f"no entry for map {mp} opcode 0x{op:02x} modrm "
                          f"{'none' if modrm is None else hex(modrm)} w {w} l {vl}")
    _, _, has_modrm, imm, mn, sizes, _, _, _, _, iclass, isizes, requires, fallback = chosen[0]
    if has_modrm:
        assert modrm is not None
        i += 1
        mod, rm = modrm >> 6, modrm & 7
        if mod != 3:
            if rm == 4:
                sib = b[i]
                i += 1
                if (sib & 7) == 5 and mod == 0:
                    i += 4
            if mod == 0 and rm == 5:
                i += 4
            elif mod == 1:
                i += 1
            elif mod == 2:
                i += 4
    opsize = "64" if w and not vex else "16" if 0x66 in pfx else "32"
    i += {None: 0, "b": 1, "w": 2, "d": 4,
          "z": 2 if opsize == "16" else 4,
          "v": 8 if opsize == "64" else 2 if opsize == "16" else 4,
          "moffs": 4 if 0x67 in pfx else 8}[imm]
    if i > MAX_LEN:
        raise DecodeError(f"length {i} exceeds 15")
    if sizes:
        mn = dict(sizes)[opsize]
    if isizes:
        iclass = dict(isizes)[opsize]
    if requires and fallback and features is not None and requires not in features:
        mn, iclass = fallback                         # same bytes, older CPU (spec `features` note)
    if mp == "1" and op == 0x90 and rex & 1:
        mn, iclass = "xchg", "XCHG"                   # 41 90: xchg r8, rax (SDM: NOP is 90 without REX.B)
    if 0xF0 in pfx and mn in LOCKABLE:
        mn, iclass = "lock " + mn, iclass + "_LOCK"
    # REP applies to the one-byte string opcodes only. "movsd" also names SSE F2 0F 10/11,
    # where F2 is a mandatory prefix, not a repeat (SDM Vol 2B MOVSD notes both instructions).
    if mp == "1" and mn in REP_STRING:
        if 0xF3 in pfx:
            word = "repe" if mn in REPE_STRING else "rep"
            mn, iclass = f"{word} {mn}", f"{word.upper()}_{iclass}"
        elif 0xF2 in pfx:
            mn, iclass = "repne " + mn, "REPNE_" + iclass
    return Decoded(i, mn, iclass)
