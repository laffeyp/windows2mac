"""rebuild_decode_spec.py — rebuild specs/x86_decode.json 0.7.0 from 0.6.3 (Sprint 036).

Steps, each recorded in the spec's revision note:
  1. the 0.6.3 spec from git (BASE_COMMIT), with the hand edits below;
  2. rows proposed from the SDM instruction pages for every iclass in
     specs/x86_decode_sdm_request.txt (the game's gap kinds, the instructions of its unconfirmed
     rows, and their prefix neighbours), by scripts/rows_from_sdm.py;
  3. merged by scripts/merge_sdm_rows.py --replace (hand-written vector rows of the same iclass
     give way; general-purpose rows stay);
  4. the post-merge edits below; then the decode tables and the XED confirmations regenerate.

Usage: uv run python scripts/rebuild_decode_spec.py
"""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parent.parent
SPEC = ROOT / "specs" / "x86_decode.json"
REQUEST = ROOT / "specs" / "x86_decode_sdm_request.txt"
ROWS = ROOT / "build" / "sdm_rows.json"
BASE_COMMIT = "366e255"                      # spec 0.6.3

LEGACY_PREFIX_OPCODES = [
    {"opcode": ["0f38", 240], "legacy": ["66"]}, {"opcode": ["0f38", 241], "legacy": ["66"]},
    {"opcode": ["0f", 188], "legacy": ["66", "f2"]}, {"opcode": ["0f", 189], "legacy": ["66", "f2"]},
    {"opcode": ["0f", 199], "legacy": ["66", "f2", "f3"]}]
LEGACY_NOTE = (
    "Opcodes with F2/F3 rows whose no-prefix row is a general-purpose instruction. A prefix with no "
    "row of its own keeps its legacy meaning only where listed: MOVBE (0F38 F0/F1, CRC32 at F2) takes "
    "66 as operand size and refuses F3; BSF/BSR (0F BC/BD, TZCNT/LZCNT at F3) take 66 as operand size "
    "and ignore F2; 0F C7 (CMPXCHG8B/16B, RDRAND, RDSEED; RDPID and SENDUIPI at F3) takes 66 as "
    "operand size and ignores F2, and F3 where no F3 row fits the ModRM. On every other SSE-family "
    "opcode the highest-precedence prefix must have a row (XED: 66 0F38 F0 is MOVBE, F2 0F BC is "
    "BSF, F3 0F C7 /1 is CMPXCHG8B, but F2 66 0F 6F is invalid).")
REVISION = (
    "; 0.7.0 (2026-10-09, Sprint 036): LEA memory-form only (SDM LEA: \"#UD If source operand is not a "
    "memory location\"); the mandatory prefix of an SSE-family opcode is the present prefix of highest "
    "precedence with no fallback, except prefixes.legacy_prefix_opcodes; SDM NP rows refuse 66/F2/F3 "
    "and NFx rows F2/F3 (row field np); ModRM.reg validity for MOV CR/DR/Sreg (row field reg_valid); "
    "rows generated from the SDM instruction pages (scripts/rows_from_sdm.py, scripts/merge_sdm_rows.py, "
    "scripts/rebuild_decode_spec.py) for the gap kinds Hail to the Rainbow reaches, replacing "
    "hand-written vector rows that lacked L, W, vvvv, form or NP constraints; AMD rows FEMMS, VFMADDPS, "
    "MOVNTSS, MOVNTSD and Key Locker rows from their opcodes, confirmed by XED; UD0 without ModR/M as XED "
    "decodes it; ENTER immediate wb; PUSHFQ/POPFQ/IRETD 16-bit iclasses; XGETBV, LFENCE, SFENCE, MFENCE NP; "
    "RDRAND/RDSEED NFx; MOV_CR/MOV_DR iclasses; XOP rotate-immediate W0/vvvv")


def run(*cmd: str) -> None:
    subprocess.run(cmd, cwd=ROOT, check=True)


def base_edits(d: dict[str, Any]) -> None:
    entries = d["entries"]

    def one(mn: str) -> dict[str, Any]:
        hits: list[dict[str, Any]] = [e for e in entries if e["mn"] == mn]
        assert len(hits) == 1, (mn, len(hits))
        return hits[0]

    lea = entries[134]
    assert lea["mn"] == "lea"
    lea["form"] = "mem"
    for mn, short in (("pushfq", "PUSHF"), ("popfq", "POPF")):
        r = one(mn)
        r["iclass_sizes"] = {"16": short, "32": r["iclass"], "64": r["iclass"]}
        r["sizes"] = {"16": short.lower(), "32": mn, "64": mn}
        r["src_validity"] = "XED: 66 9C/9D is the 16-bit PUSHF/POPF (Sprint 036)"
    one("xgetbv")["np"] = True
    for mn in ("lfence", "sfence", "mfence"):
        one(mn)["np"] = True                                 # SDM: NP 0F AE E8/F8/F0
    for mn in ("rdrand", "rdseed"):
        one(mn)["np"] = "nfx"                                # SDM: NFx 0F C7 /6, /7 (66 selects r16)
    for op in (0x8C, 0x8E):
        for r in [e for e in entries if e["map"] == "1" and e["op"] == op]:
            r["reg_valid"] = [0, 1, 2, 3, 4, 5, 8, 9, 10, 11, 12, 13]   # ES-GS; REX.R ignored (XED)
    for mn in ("vprotd", "vprotq"):
        for r in [e for e in entries if e["mn"] == mn and e.get("xop") and e.get("imm") == "b"]:
            r["w"], r["vvvv"] = 0, "1111"
            r["src_validity"] = "AMD XOP immediate form: XOP.W0 and vvvv 1111 (XED refuses W1 and other vvvv; Sprint 036)"
    d["version"] = "0.7.0"
    d["revision"] += REVISION
    d["prefixes"]["legacy_prefix_opcodes"] = LEGACY_PREFIX_OPCODES
    d["prefixes"]["legacy_prefix_opcodes_note"] = LEGACY_NOTE
    d["immediates"]["wb"] = "3 bytes: a word then a byte (ENTER iw ib)"


def post_merge_edits(d: dict[str, Any]) -> None:
    entries = d["entries"]
    for r in [e for e in entries if e["iclass"] == "IRETD"]:
        r["iclass_sizes"] = {"16": "IRET", "32": "IRETD", "64": "IRETD"}
        r["sizes"] = {"16": "iret", "32": "iretd", "64": "iretd"}
    for r in [e for e in entries if e["map"] == "0f" and e["op"] in (0x20, 0x22) and e["mn"] == "mov"]:
        r["iclass"], r["reg_valid"] = "MOV_CR", [0, 2, 3, 4, 8]   # SDM MOV to/from Control Registers
    for r in [e for e in entries if e["map"] == "0f" and e["op"] in (0x21, 0x23) and e["mn"] == "mov"]:
        r["iclass"], r["reg_valid"] = "MOV_DR", list(range(8))    # SDM MOV to/from Debug Registers


def main() -> int:
    base = json.loads(subprocess.run(["git", "show", f"{BASE_COMMIT}:specs/x86_decode.json"], cwd=ROOT,
                                     check=True, capture_output=True, text=True).stdout)
    assert base["version"] == "0.6.3", base["version"]
    base_edits(base)
    SPEC.write_text(json.dumps(base, indent=1, ensure_ascii=False) + "\n")
    ROWS.parent.mkdir(exist_ok=True)
    run("uv", "run", "python", "scripts/rows_from_sdm.py", *REQUEST.read_text().split(), "--out", str(ROWS))
    run("uv", "run", "python", "scripts/merge_sdm_rows.py", str(ROWS), "--replace")
    d = json.loads(SPEC.read_text())
    post_merge_edits(d)
    SPEC.write_text(json.dumps(d, indent=1, ensure_ascii=False) + "\n")
    run("uv", "run", "python", "scripts/gen_decoder.py")
    run("uv", "run", "python", "scripts/generated_differential.py", "--confirmations",
        "specs/x86_decode_confirmations.json")
    return 0


if __name__ == "__main__":
    sys.exit(main())
