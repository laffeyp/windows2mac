"""encgen.py — generated x86-64 encodings for the decoder differential (Sprint 020).

Corpus binaries exercise the regular part of the encoding space compilers emit; decoder bugs
live outside it (Paleari et al., ISSTA 2010; Woodruff et al., LangSec 2021). For every row of
specs/x86_decode.json this module builds encodings that vary what the row leaves free: legacy
prefixes, REX bits, ModRM mode and r/m (including SIB, base-5, RIP-relative), the ModRM.reg
field when the row does not fix it, VEX/EVEX W, L, and vvvv, and immediate/displacement bytes;
for VEX/XOP/EVEX rows, a quarter of cases also appear with a legacy prefix or REX before the
escape (added 2026-10-08 after the project review found the class untested).
Seeded: the same seed gives the same cases.
"""

from __future__ import annotations

import json
import random
from collections.abc import Iterator
from dataclasses import dataclass
from pathlib import Path

SPEC = Path(__file__).resolve().parents[2] / "specs" / "x86_decode.json"
LEGACY_EXTRA = [b"", b"\x66", b"\x67", b"\x2e", b"\x64", b"\xf0", b"\xf2", b"\xf3", b"\x66\x67",
                b"\xf0\x66"]
MAP_ESC = {"1": b"", "0f": b"\x0f", "0f38": b"\x0f\x38", "0f3a": b"\x0f\x3a"}
VEX_MM = {"0f": 1, "0f38": 2, "0f3a": 3}
XOP_MM = {"x8": 8, "x9": 9, "xa": 10}
PP = {"none": 0, "66": 1, "f3": 2, "f2": 3}


@dataclass(frozen=True)
class Case:
    raw: bytes
    row: int          # index into spec["entries"]
    note: str         # what was varied


def _modrm_tail(rng: random.Random, reg: int, mod: int, rm: int) -> bytes:
    out = bytes([(mod << 6) | (reg << 3) | rm])
    if mod != 3 and rm == 4:
        sib = rng.randrange(256)
        if rng.random() < 0.3:
            sib = (sib & ~7) | 5          # base 5: disp32 when mod 0
        out += bytes([sib])
    return out + bytes(rng.randrange(256) for _ in range(6))   # displacement/immediate room


def cases(seed: int = 2026_10_08, per_row: int = 48) -> Iterator[Case]:
    rng = random.Random(seed)
    prng = random.Random(seed + 1)   # separate stream: the unprefixed cases do not depend on it
    spec = json.loads(SPEC.read_text())
    for idx, e in enumerate(spec["entries"]):
        enc = "evex" if e.get("evex") else "xop" if e.get("xop") else "vex" if e.get("vex") else "legacy"
        for k in range(per_row):
            reg = e["reg"] if e.get("reg") is not None else rng.randrange(8)
            if e.get("mod"):
                mbyte = int(e["mod"], 16)
                mod, reg, rm = mbyte >> 6, (mbyte >> 3) & 7, mbyte & 7
            else:
                mod = 3 if e.get("form") == "reg" else rng.randrange(3) if e.get("form") == "mem" else rng.randrange(4)
                rm = [4, 5][k % 2] if k < 4 else rng.randrange(8)
            tail = _modrm_tail(rng, reg, mod, rm) if e["modrm"] else bytes(rng.randrange(256) for _ in range(8))
            if enc == "legacy":
                mand = b"" if e["pfx"] == "none" else bytes.fromhex(e["pfx"])
                extra = rng.choice(LEGACY_EXTRA)
                rex = b""
                if rng.random() < 0.6 or e.get("w") == 1:
                    w = e.get("w", rng.randrange(2))
                    rex = bytes([0x40 | (w << 3) | rng.randrange(8)])
                raw = extra + mand + rex + MAP_ESC[e["map"]] + bytes([e["op"]]) + tail
                note = f"legacy extra={extra.hex() or '-'} rex={rex.hex() or '-'} mod={mod} rm={rm}"
            elif enc in ("vex", "xop"):
                w = e.get("w", rng.randrange(2))
                lbit = e.get("l", rng.randrange(2))
                vvvv = 0xF if rng.random() < 0.7 else rng.randrange(16)
                rxb = rng.randrange(8) if enc == "xop" else 7 if rng.random() < 0.5 else rng.randrange(8)
                mm = XOP_MM[e["map"]] if enc == "xop" else VEX_MM[e["map"]]
                p1 = (rxb << 5) | mm
                p2 = (w << 7) | (vvvv << 3) | (lbit << 2) | PP[e["pfx"]]
                if enc == "vex" and mm == 1 and w == 0 and rxb & 3 == 3 and rng.random() < 0.5:
                    raw = b"\xc5" + bytes([((rxb >> 2) << 7) | (vvvv << 3) | (lbit << 2) | PP[e["pfx"]]])
                else:
                    raw = (b"\x8f" if enc == "xop" else b"\xc4") + bytes([p1, p2])
                raw += bytes([e["op"]]) + tail
                note = f"{enc} w={w} l={lbit} vvvv={vvvv:x} mod={mod} rm={rm}"
            else:
                w = e.get("w", rng.randrange(2))
                ll = rng.randrange(3)
                p0 = (rng.randrange(16) << 4) | VEX_MM[e["map"]]
                p1 = (w << 7) | ((0xF if rng.random() < 0.7 else rng.randrange(16)) << 3) | 4 | PP[e["pfx"]]
                p2 = (rng.randrange(2) << 7) | (ll << 5) | (rng.randrange(2) << 4) | 8 | rng.randrange(8)
                raw = b"\x62" + bytes([p0, p1, p2, e["op"]]) + tail
                note = f"evex w={w} ll={ll} mod={mod} rm={rm}"
            yield Case(raw[:15], idx, note)
            if enc != "legacy" and prng.random() < 0.25:
                # a legacy prefix or REX before the VEX/XOP/EVEX escape: #UD (SDM Vol 2A 2.3.2)
                pre = prng.choice([b"\x66", b"\xf2", b"\xf3", b"\xf0", b"\x2e", b"\x67",
                                   b"\x40", b"\x48"])
                yield Case((pre + raw)[:15], idx, f"prefix={pre.hex()} {note}")
