# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""merge_sdm_rows.py — add rows proposed by rows_from_sdm.py to specs/x86_decode.json (Sprint 036).

A proposed row is skipped when the spec already has a row with the same decoding key (map, opcode,
encoding class, mandatory prefix, ModRM.reg, ModRM byte, form, W, L). Three rows cannot come from
SDM text and are written here with their reason:

  FEMMS (0F 0E), VFMADDPS (FMA4, VEX 0F3A 68 /is4), MOVNTSS and MOVNTSD (SSE4a, F3/F2 0F 2B) are
  AMD instructions with no SDM page; the Key Locker instructions (F3 0F38 DC-DF) are documented
  outside the SDM;
  UD0 (0F FF): the SDM page shows /r and footnotes that "some older processors decode the UD0
  instruction without a ModR/M byte"; XED, the judge, decodes it without one (Sprint 036:
  length_differs on every generated case with a ModRM byte), so the row follows XED.

Usage: uv run python scripts/merge_sdm_rows.py rows.json [--replace]
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

SPEC = Path(__file__).resolve().parent.parent / "specs" / "x86_decode.json"
KL = ("not in the SDM; not retrieved for this row: fields from the opcode as Intel documents it, "
      "confirmed by XED in scripts/generated_differential.py")
AMD = ("not retrieved for this row: fields from the opcode as AMD documents it, confirmed by XED in "
       "scripts/generated_differential.py")
MANUAL = [
    {"map": "0f", "op": 0x0E, "mn": "femms", "pfx": "none", "modrm": False, "iclass": "FEMMS",
     "src": f"AMD 3DNow! FEMMS, opcode 0F 0E (AMD64 Architecture Programmer's Manual Vol 5; {AMD})"},
    {"map": "0f3a", "op": 0x68, "mn": "vfmaddps", "pfx": "66", "modrm": True, "vex": True, "imm": "b",
     "iclass": "VFMADDPS",
     "src": f"AMD FMA4 VFMADDPS, VEX map 0F3A opcode 68 /r /is4, pp 66 (AMD64 Architecture Programmer's Manual Vol 6; {AMD})"},
    *[{"map": "0f38", "op": op, "mn": mn, "pfx": "f3", "modrm": True, "form": form, "iclass": mn.upper(),
       "src": f"Intel Key Locker {mn.upper()}, opcode F3 0F 38 {op:02X} /r (Intel Key Locker Specification; {KL})"}
      for op, mn, form in ((0xDC, "aesenc128kl", "mem"), (0xDC, "loadiwkey", "reg"), (0xDD, "aesdec128kl", "mem"),
                           (0xDE, "aesenc256kl", "mem"), (0xDF, "aesdec256kl", "mem"))],
    {"map": "0f", "op": 0x2B, "mn": "movntss", "pfx": "f3", "modrm": True, "form": "mem", "iclass": "MOVNTSS",
     "src": f"AMD SSE4a MOVNTSS m32, xmm, opcode F3 0F 2B /r (AMD64 Architecture Programmer's Manual Vol 4; {AMD})"},
    {"map": "0f", "op": 0x2B, "mn": "movntsd", "pfx": "f2", "modrm": True, "form": "mem", "iclass": "MOVNTSD",
     "src": f"AMD SSE4a MOVNTSD m64, xmm, opcode F2 0F 2B /r (AMD64 Architecture Programmer's Manual Vol 4; {AMD})"},
]


def _enc(e: dict[str, object]) -> str:
    return "evex" if e.get("evex") else "xop" if e.get("xop") else "vex" if e.get("vex") else "legacy"


def _key(e: dict[str, object]) -> tuple[object, ...]:
    return (e["map"], e["op"], _enc(e), e["pfx"], e.get("reg"), e.get("mod"), e.get("form"), e.get("w"), e.get("l"))


def main(argv: list[str]) -> int:
    proposed = json.loads(Path(argv[0]).read_text())
    spec = json.loads(SPEC.read_text())
    if "--replace" in argv:
        # hand-written rows of the same instructions give way to the SDM lines (Sprint 036: they
        # lacked L, W, vvvv, form and NP constraints the SDM states)
        # General-purpose rows (operand templates, or the one-byte map) stay: they carry operand-size
        # immediates (z), sizes and templates that one SDM line per size does not (MOV C7 /0 iz).
        # Matched by XED iclass, not mnemonic: CALL_FAR and CALL_NEAR share "call".
        def ident(e: dict[str, object]) -> tuple[object, ...]:
            return (e["iclass"], e["map"], bool(e.get("vex")))
        keep = {ident(e) for e in spec["entries"]
                if not e.get("vex") and (e.get("ops") or e["map"] == "1")}
        names = {ident(r) for r in proposed} - keep
        proposed = [r for r in proposed if ident(r) not in keep]
        before = len(spec["entries"])
        spec["entries"] = [e for e in spec["entries"]
                           if ident(e) not in names or e.get("evex") or e.get("xop")]
        print(f"replaced {before - len(spec['entries'])} hand-written rows; kept the general-purpose rows of "
              f"{len(keep)} iclasses")
    have = {_key(e): e for e in spec["entries"]}
    added, present, conflicts = [], 0, []
    for r in proposed + MANUAL:
        row = {k: r[k] for k in ("map", "op", "mn", "pfx", "modrm", "src") if k in r}
        for f in ("vex", "reg", "mod", "form", "w", "l", "imm", "np", "vvvv"):
            if f in r:
                row[f] = r[f]
        row["iclass"] = r["iclass"]
        if row["iclass"] == "UD0":
            row["modrm"] = False
            row["src"] = str(row["src"]) + "; decoded without a ModR/M byte as XED does (the SDM page's footnote on older processors)"
        k = _key(row)
        if k in have:
            if have[k]["iclass"] == row["iclass"]:
                present += 1
            else:                                   # same bytes, another iclass (IRET vs IRETD at CF)
                conflicts.append((row["iclass"], have[k]["iclass"], k))
            continue
        have[k] = row
        added.append(row)
    if conflicts:
        print(f"conflicts (not merged): {conflicts}")
        return 1
    spec["entries"].extend(added)
    SPEC.write_text(json.dumps(spec, indent=1, ensure_ascii=False) + "\n")
    print(f"added {len(added)}; already present {present}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
