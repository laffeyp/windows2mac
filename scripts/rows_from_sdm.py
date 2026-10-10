# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""rows_from_sdm.py — propose x86 decode spec rows from the Intel SDM instruction pages (Sprint 036).

Source: the SDM Vol 2 instruction pages as transcribed at felixcloutier.com/x86 (one page per
instruction; its first table's "Opcode/Instruction" column is the SDM's). Each opcode line is
parsed into the fields a row of specs/x86_decode.json carries:

  [NP|66|F2|F3] [REX.W +] [0F [38|3A]] op [/digit | /r | C0+i | <ModRM byte>] [ib|iw|id|cb|cw|cd]
  VEX.<128|256|L0|L1|LIG|LZ>[.66|.F2|.F3].<0F|0F38|0F3A>.<W0|W1|WIG> op /r [ib|/is4]
  x87: D8 /0, D8 C0+i, D9 E8

Only lines whose mnemonic is one of the requested XED iclasses (after the suffix rules below)
are kept, so XED's name decides which lines are wanted and the SDM line decides the fields.
EVEX lines are skipped (no game code uses them). XED then judges every row through
generated_differential.py, so the fields are confirmed by an independent decoder rather than
copied from it.

Usage: uv run python scripts/rows_from_sdm.py ICLASS... --out rows.json
"""

from __future__ import annotations

import argparse
import html
import json
import re
import sys
import urllib.request
from pathlib import Path

BASE = "https://www.felixcloutier.com/x86/"
CACHE = Path(__file__).resolve().parent.parent / "build" / "sdm_pages"
RETRIEVED = "2026-10-09"
ICLASS_TO_MNEMONIC = {"CMPSD_XMM": "CMPSD", "MOVSD_XMM": "MOVSD", "CALL_FAR": "CALL", "RET_FAR": "RET",
                      "IRETD": "IRET", "FWAIT": "WAIT"}
PAGE_ALIASES = {"UD0": "ud", "VBROADCASTSS": "vbroadcast", "VBROADCASTF128": "vbroadcast",
                "VPMOVSXDQ": "pmovsx", "VPBROADCASTD": "vpbroadcast", "VPBROADCASTQ": "vpbroadcast"}                      # family pages named for the family
MNEMONIC = re.compile(r"[A-Z][A-Z0-9]{2,}")


def _has_mnemonic(cell: str) -> bool:
    return any(MNEMONIC.fullmatch(t) and not re.fullmatch(r"[0-9A-F]{2}", t) and not t.startswith(("VEX.", "REX."))
               for t in cell.split())


# Pages that hold several instructions under one mnemonic: the line must match to belong to the
# iclass (CALL near/far, RET near/far, string CMPSD vs SSE2 CMPSD)
LINE_FILTER = {"CALL_FAR": r"m16:|ptr16:", "RET_FAR": r"^(REX\.W )?C[AB]\b", "CMPSD_XMM": r"\bxmm"}
IMM = {"ib": "b", "iw": "w", "id": "d", "cb": "b", "cw": "w", "cd": "d", "io": "q"}


def fetch(page: str) -> str:
    CACHE.mkdir(parents=True, exist_ok=True)
    f = CACHE / f"{page or 'index'}.html"
    if not f.exists():
        req = urllib.request.Request(BASE + page, headers={"User-Agent": "w2m rows_from_sdm.py"})
        f.write_bytes(urllib.request.urlopen(req, timeout=30).read())
    return f.read_text(errors="replace")


def index() -> dict[str, list[str]]:
    """Mnemonic -> page names, from the index's links and from page names ('fadd:faddp:fiadd')."""
    out: dict[str, list[str]] = {}
    for href, text in re.findall(r"<a href='/x86/([^']+)'>([^<]+)</a>", fetch("")):
        names = re.split(r"[/,]\s*", html.unescape(text)) + href.split(":")
        for m in names:
            pages = out.setdefault(m.strip().upper(), [])
            if href not in pages:
                pages.append(href)
    return out


def opcode_lines(page: str) -> list[str]:
    """The page's opcode lines valid in 64-bit mode (the SDM's 64-bit column: "Valid", "V/V";
    lines marked "Invalid", "N.E." or "I/V" are dropped)."""
    return [text for text, valid in _lines(page) if valid]


def _lines(page: str) -> list[tuple[str, bool]]:
    t = fetch(page)
    table = re.search(r"<table>(.*?)</table>", t, re.S)
    if not table:
        return []
    rows = [[html.unescape(re.sub(r"<[^>]+>", "", re.sub(r"<sup>.*?</sup>", "", c))).strip()
             for c in re.findall(r"<t[dh][^>]*>(.*?)</t[dh]>", row, re.S)]
            for row in re.findall(r"<tr>(.*?)</tr>", table.group(1), re.S)]
    if not rows:
        return []
    split = rows[0] and rows[0][0].lower().startswith("opcode") and len(rows[0]) > 1 and \
        rows[0][1].lower().startswith("instruction")          # Opcode | Instruction columns
    col64 = next((i for i, h in enumerate(rows[0]) if "64" in h), None)
    lines = []
    for cells in rows[1:]:
        if cells:
            joined = len(cells) > 1 and (split or not _has_mnemonic(cells[0]))
            text = f"{cells[0]} {cells[1]}" if joined else cells[0]
            text = re.sub(r"([0-9A-F]{2})/", r"\1 /", text)        # "3E/r" -> "3E /r"
            text = re.sub(r"((?:VEX|EVEX)\.[\w.]*\.)\s+", r"\1", text)   # "VEX.LZ. 0F38.W1" -> "VEX.LZ.0F38.W1"
            if not _has_mnemonic(text) and ":" not in page:
                text += " " + page.upper()                     # an empty Instruction cell (FCOS: "D9 FF")
            mode = cells[col64].strip().upper() if col64 is not None and col64 < len(cells) else "V"
            # "Valid", "V/V"; "Invalid*" is valid with a CPUID feature (LAHF/SAHF in 64-bit mode)
            valid = mode.startswith("V") or mode.endswith("*")
            lines.append((re.sub(r"\s+", " ", re.sub(r"(/r|ib|iw|id)\d\b", r"\1", text)), valid))
    return lines


def parse(line: str) -> dict[str, object] | None:
    """One SDM opcode line -> a spec row (without src), or None when it is not a form this spec
    represents (EVEX, 32-bit only)."""
    tok = [t for t in line.replace("REX.W +", "REX.W").replace(" + ", "+").split()
           if not re.fullmatch(r"\d", t)]                     # footnote markers ("/r 1")
    row: dict[str, object] = {"pfx": "none", "modrm": False}
    i = 0
    if tok[0].startswith("EVEX"):
        return None
    if tok[0].startswith("VEX."):
        parts = tok[0].split(".")[1:]
        row["vex"] = True
        for p in parts:
            if p in ("128", "L0", "LZ"):
                row["l"] = 0
            elif p in ("256", "L1"):
                row["l"] = 1
            elif p in ("66", "F2", "F3"):
                row["pfx"] = p.lower()
            elif p in ("0F", "0F38", "0F3A"):
                row["map"] = p.lower()
            elif p in ("W0", "W1", "0", "1"):
                row["w"] = int(p[-1])                       # older SDM lines write W0 as ".0"
        i = 1
    else:
        if tok[0] == "NFx":
            row["np"] = "nfx"                               # SDM NFx: F2 and F3 are not allowed
            tok = tok[1:]
        if tok[0] in ("NP", "66", "F2", "F3"):
            row["pfx"] = "none" if tok[0] == "NP" else tok[0].lower()
            if tok[0] == "NP":
                row["np"] = True                            # SDM: 66, F2 and F3 are not allowed
            i = 1
        if tok[i] == "REX.W":
            row["w"] = 1
            i += 1
        elif tok[i] == "REX":
            i += 1
        if tok[i] == "0F":
            if tok[i + 1] in ("38", "3A"):
                row["map"] = "0f" + tok[i + 1].lower()
                i += 2
            else:
                row["map"] = "0f"
                i += 1
        else:
            row["map"] = "1"
    m = re.fullmatch(r"([0-9A-F]{2})(\+r[bwdo]|\+i)?", tok[i])
    if not m:
        return None
    row["op"] = int(m.group(1), 16)
    if m.group(2):
        return None                                  # +rb/+i register-in-opcode forms: not needed here
    i += 1
    mnemonic_at = len(tok)
    while i < len(tok):
        t = tok[i]
        if re.fullmatch(r"/0?[0-7]", t):
            row["modrm"], row["reg"] = True, int(t[-1])       # "/5" or "/05" (INCSSPD)
        elif t in ("/r", "/is4"):
            row["modrm"] = True
            if t == "/is4":
                row["imm"] = "b"
        elif re.fullmatch(r"C0\+i|C8\+i|D0\+i|D8\+i|E0\+i|E8\+i|F0\+i|F8\+i", t):
            row["modrm"], row["reg"], row["form"] = True, (int(t[:2], 16) - 0xC0) >> 3, "reg"
        elif re.fullmatch(r"[0-9A-F]{2}", t) and row["map"] == "1" and 0xD8 <= int(row["op"]) <= 0xDF:  # type: ignore[call-overload]
            row["modrm"], row["mod"] = True, t.lower()
        elif re.fullmatch(r"[0-9A-F]{2}", t) and row["map"] == "1" and row["op"] == 0xC8:
            row["imm"] = str(row.get("imm", "")) + "b"                 # ENTER's level byte (00, 01)
        elif t in IMM:
            row["imm"] = IMM[t] if "imm" not in row else str(row["imm"]) + IMM[t]
        else:
            mnemonic_at = i
            break
        i += 1
    if mnemonic_at >= len(tok):
        return None
    row["mn"] = tok[mnemonic_at].lower()
    operands = " ".join(tok[mnemonic_at + 1:])
    if row.get("modrm") and "reg" in row and "form" not in row and "mod" not in row and row["map"] != "1" \
            and not re.search(r"(^|/)m(\d|$)|\br/m|/m\d", operands):
        row["form"] = "reg"                                    # PSRAW mm, imm8: 0F 71 /4 ib, mod = 3 only
    if row.get("modrm") and "reg" in row and "form" not in row and "mod" not in row:
        if re.search(r"\bm(8|16|32|64|80|128|256)?(fp|int|bcd|byte)?\b|m16:|m2byte|m14/28byte|m94/108byte", operands) \
                and not re.search(r"\br/m|\bST\(i\)|\bmm/|\bxmm\d?/|\bymm\d?/", operands):
            row["form"] = "mem"
    if row.get("modrm") and "reg" not in row and "form" not in row:
        ops = [o.strip() for o in operands.split(",")]
        memory = re.compile(r"(^|/)m(\d|$|fp|int|bcd|16:|\d*byte)")   # m64, m32fp, m16:64; not mm, mm1
        has_mem = any(memory.search(o) for o in ops)
        has_rm = any("/" in o for o in ops)
        if not has_mem:
            row["form"] = "reg"                            # VMOVLHPS xmm1, xmm2, xmm3; MOVQ2DQ xmm, mm
        elif not has_rm:
            row["form"] = "mem"                            # VMOVHPS xmm2, xmm1, m64
    if row.get("vex") and row.get("modrm"):
        regs = [o for o in operands.split(",") if o.strip() and not re.fullmatch(r"\s*imm8\s*", o)]
        if len(regs) <= (1 if "reg" in row else 2):            # /digit with one operand: VLDMXCSR m32
            row["vvvv"] = "1111"                           # no NDS operand: VEX.vvvv must be 1111b (SDM 2.3.6)
    row["operands"] = operands
    return row


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("iclass", nargs="+")
    ap.add_argument("--out", type=Path, required=True)
    a = ap.parse_args(argv)
    pages = index()
    rows, missing = [], []
    for ic in a.iclass:
        mn = ICLASS_TO_MNEMONIC.get(ic, ic)
        candidates = pages.get(mn, []) + (pages.get(mn[1:], []) if mn.startswith("V") else []) + \
            ([PAGE_ALIASES[mn]] if mn in PAGE_ALIASES else [])
        found = 0
        for page, line in ((p, line) for p in candidates for line in opcode_lines(p)):
            if ic in LINE_FILTER and not re.search(LINE_FILTER[ic], line):
                continue
            r = parse(line)
            if r is None:
                continue
            got = str(r["mn"]).upper()
            if got != mn and got.rstrip("0123456789") == mn:
                r["mn"] = mn.lower()                           # a footnote digit on the mnemonic
            elif got != mn:
                continue
            r["iclass"] = ic
            r["src"] = (f"Intel SDM Vol 2 instruction page {mn} (opcode line \"{line}\"), as transcribed "
                        f"at {BASE}{page}, retrieved {RETRIEVED}")
            rows.append(r)
            found += 1
        if not found:
            missing.append(ic)
    # where one form needs W=1, its partner on the same opcode is W=0, not "any" (MOVD/MOVQ 0F 6E)
    def same_but_w(x: dict[str, object]) -> tuple[object, ...]:
        return tuple((k, x.get(k)) for k in ("map", "op", "vex", "pfx", "reg", "mod", "form", "l"))
    w1 = {same_but_w(r) for r in rows if r.get("w") == 1}
    for r in rows:
        if "w" not in r and same_but_w(r) in w1:
            r["w"] = 0
    a.out.write_text(json.dumps(rows, indent=1) + "\n")
    print(f"{len(rows)} rows for {len(a.iclass) - len(missing)} iclasses; no line found: {missing}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
