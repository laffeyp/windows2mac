"""x86dec.py — w2m's own x86-64 decoder (length and mnemonic), driven by tables generated from
specs/x86_decode.json (Intel SDM opcode maps). The second of two independent decoders; it is
compared against Capstone instruction by instruction and every disagreement is reported.

Structure follows Intel SDM Vol 2A sections 2.1-2.3: legacy prefixes, REX, VEX (C4/C5), opcode
(one-byte, 0F, 0F38, 0F3A), ModRM, SIB, displacement, immediate.
"""

from __future__ import annotations

from dataclasses import dataclass

from w2m.x86dec_tables import LOCKABLE, REP_STRING, TABLE, Entry

LEGACY = frozenset({0xF0, 0xF2, 0xF3, 0x2E, 0x36, 0x3E, 0x26, 0x64, 0x65, 0x66, 0x67})
MAPS_3 = {0x38: "0f38", 0x3A: "0f3a"}
VEX_MAP = {1: "0f", 2: "0f38", 3: "0f3a"}
VEX_PFX = {0: "none", 1: "66", 2: "f3", 3: "f2"}
MAX_LEN = 15


@dataclass(frozen=True)
class Decoded:
    length: int
    mnemonic: str


class DecodeError(ValueError):
    """The spec has no entry for these bytes, or they exceed 15 bytes."""


def _pick(cands: tuple[Entry, ...], mpfx: list[str]) -> list[Entry]:
    """Mandatory-prefix selection: the most specific prefix present that has entries wins."""
    for p in mpfx:
        hit = [c for c in cands if c[0] == p]
        if hit:
            return hit
    return []


def decode(b: bytes) -> Decoded:
    i = 0
    pfx: list[int] = []
    while i < MAX_LEN and b[i] in LEGACY:
        pfx.append(b[i])
        i += 1
    rex = 0
    if 0x40 <= b[i] <= 0x4F:
        rex = b[i]
        i += 1
    vex = False
    w = (rex >> 3) & 1
    if b[i] in (0xC4, 0xC5) and not rex:
        vex = True
        if b[i] == 0xC5:
            p = b[i + 1]
            mp, pp = "0f", p & 3
            i += 2
        else:
            p1, p2 = b[i + 1], b[i + 2]
            if (p1 & 0x1F) not in VEX_MAP:
                raise DecodeError(f"VEX map field {p1 & 0x1F}")
            mp, pp, w = VEX_MAP[p1 & 0x1F], p2 & 3, p2 >> 7
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
        reg = (modrm >> 3) & 7
        chosen = [c for c in chosen if c[6] is None or int(c[6], 16) == modrm]
        chosen = [c for c in chosen if c[1] is None or c[1] == reg]
    chosen = [c for c in chosen if c[7] is None or c[7] == w]
    if not chosen:
        raise DecodeError(f"no entry for map {mp} opcode 0x{op:02x} modrm "
                          f"{'none' if modrm is None else hex(modrm)} w {w}")
    _, _, has_modrm, imm, mn, sizes, _, _ = chosen[0]
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
    opsize = "64" if (rex >> 3) & 1 else "16" if 0x66 in pfx else "32"
    i += {None: 0, "b": 1, "w": 2, "d": 4,
          "z": 2 if opsize == "16" else 4,
          "v": 8 if opsize == "64" else 2 if opsize == "16" else 4}[imm]
    if i > MAX_LEN:
        raise DecodeError(f"length {i} exceeds 15")
    if sizes:
        mn = dict(sizes)[opsize]
    if mp == "1" and op == 0x90 and rex & 1:
        mn = "xchg"                                   # 41 90: xchg r8, rax (SDM: NOP is 90 without REX.B)
    if 0xF0 in pfx and mn in LOCKABLE:
        mn = "lock " + mn
    # REP applies to the one-byte string opcodes only. "movsd" also names SSE F2 0F 10/11,
    # where F2 is a mandatory prefix, not a repeat (SDM Vol 2B MOVSD notes both instructions).
    string_op = mp == "1" and mn in REP_STRING
    if string_op and 0xF3 in pfx:
        mn = "rep " + mn
    elif string_op and 0xF2 in pfx:
        mn = "repne " + mn
    return Decoded(i, mn)
