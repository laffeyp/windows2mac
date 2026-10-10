"""sse_operand_templates.py — operand templates for legacy-SSE rows of specs/x86_decode.json,
transcribed from the SDM instruction pages (Sprint 039).

Each page's Opcode/Instruction column (felixcloutier transcription, cached in build/sdm_pages/)
gives the operands, and its Op/En column the ModRM roles: A puts operand 1 in ModRM.reg and
operand 2 in ModRM.r/m; B and C the reverse. Notation (SDM Vol 2D Appendix A, Table A-1):

  V      an XMM register from ModRM.reg
  W<s>   an XMM register or memory from ModRM.r/m, memory size s: d (4 bytes), q (8), dq (16)
  G<s>   a general register from ModRM.reg; E<s> a general register or memory from ModRM.r/m;
         s is d, q, or y (d, or q with REX.W)

A row whose register form and memory form are separate SDM lines (MOVSS xmm1, xmm2 and
MOVSS xmm1, m32) takes the memory line's size. The script only writes rows that have no template
yet, and refuses a row whose SDM lines disagree on roles.

Usage: uv run python scripts/sse_operand_templates.py [--write]
"""

from __future__ import annotations

import argparse
import html
import json
import re
import sys
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parent.parent
SPEC = ROOT / "specs" / "x86_decode.json"
PAGES = ROOT / "build" / "sdm_pages"
ICLASSES = {
    "MOVSS": "movss", "MOVSD_XMM": "movsd", "MOVAPS": "movaps", "MOVUPS": "movups", "MOVAPD": "movapd",
    "MOVUPD": "movupd", "MOVD": "movd:movq", "MOVQ": "movd:movq movq", "XORPS": "xorps", "XORPD": "xorpd",
    "ANDPS": "andps", "ANDPD": "andpd",
    **{f"{op}{t}": f"{op.lower()}{t.lower()}" for op in ("ADD", "SUB", "MUL", "DIV", "SQRT", "MIN", "MAX")
       for t in ("SS", "SD", "PS", "PD")},
    **{c: c.lower() for c in ("CVTTSS2SI", "CVTTSD2SI", "CVTSS2SI", "CVTSD2SI", "CVTSI2SS", "CVTSI2SD",
                              "CVTSS2SD", "CVTSD2SS", "UCOMISS", "UCOMISD", "COMISS", "COMISD")},
}
LINE = re.compile(r"^(NP |66 |F2 |F3 )?(REX\.W\s*\+?\s*)?0F ([0-9A-F]{2})\s*/r\s+([A-Z0-9]+)\s+(.*)$")
MEM_SIZE = {"m32": "d", "m64": "q", "m128": "dq"}


def _text(h: str) -> str:
    return re.sub(r"\s+", " ", html.unescape(re.sub(r"<[^>]+>", " ", h))).strip()


def sdm_lines(page: str) -> list[tuple[str, bool, int, str, list[str], str]]:
    """(prefix, rex_w, opcode, mnemonic, operands, op_en) for each legacy line of a page."""
    h = (PAGES / f"{page}.html").read_text(errors="replace")
    out = []
    for row in re.findall(r"<tr>(.*?)</tr>", h.split("<table")[1], re.S):
        cells = [_text(c) for c in re.findall(r"<t[dh][^>]*>(.*?)</t[dh]>", row, re.S)]
        if len(cells) < 2:
            continue
        m = LINE.match(cells[0].replace("0F 57/r", "0F 57 /r").replace("0F 51/r", "0F 51 /r"))
        if not m:
            continue
        pfx = (m.group(1) or "NP ").strip()
        ops = [o.strip() for o in m.group(5).split(",")]
        out.append((pfx, bool(m.group(2)), int(m.group(3), 16), m.group(4), ops, cells[1].strip()))
    return out


def _role(op: str, rex_w: bool) -> tuple[str, str]:
    """(kind, size) for one SDM operand: kind xmm or gpr; size d, q, dq, or '' for a register."""
    parts = op.split("/")
    mem = next((MEM_SIZE[p] for p in parts if p in MEM_SIZE), "")
    if any(p.startswith("xmm") for p in parts):
        return "xmm", mem
    if any(p in ("r32", "r64", "r") for p in parts):
        width = "q" if "r64" in parts or "m64" in parts or rex_w else "d"
        return "gpr", mem or width
    if parts[0] in MEM_SIZE:
        return "mem", mem
    raise ValueError(f"operand {op!r}")


def template(lines: list[tuple[str, bool, int, str, list[str], str]]) -> str:
    """One template for the SDM lines of one spec row (same prefix and opcode)."""
    widths = {w for _, w, *_ in lines}
    reg_side: list[str] | None = None
    rm_kind, rm_size, rm_gpr_sizes = "", "", set()
    for _, rex_w, _, _, ops, en in lines:
        if en == "A":
            reg_op, rm_op, order = ops[0], ops[1], "reg,rm"
        elif en in ("B", "C"):
            rm_op, reg_op, order = ops[0], ops[1], "rm,reg"
        else:
            raise ValueError(f"Op/En {en}")
        rk, rs = _role(reg_op, rex_w)
        mk, ms = _role(rm_op, rex_w)
        side = [order, rk]
        if reg_side is not None and reg_side != side:
            raise ValueError(f"lines disagree: {reg_side} vs {side}")
        reg_side = side
        if mk == "gpr":
            rm_gpr_sizes.add(ms)
            rm_kind = "gpr"
        else:
            rm_kind = rm_kind or "xmm"
            rm_size = rm_size or ms
        if rk == "gpr":
            rm_gpr_sizes.add("G" + rs)
    assert reg_side is not None
    order, rk = reg_side
    gsize = "y" if widths == {False, True} else ("q" if widths == {True} else "d")
    reg = "V" if rk == "xmm" else "G" + gsize
    if rm_kind == "gpr":
        sizes = {s for s in rm_gpr_sizes if not s.startswith("G")}
        rm = "E" + ("y" if len(sizes) > 1 or widths == {False, True} else sizes.pop())
    else:
        if not rm_size:
            raise ValueError("no memory size on any line")
        rm = "W" + rm_size
    return f"{reg},{rm}" if order == "reg,rm" else f"{rm},{reg}"


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--write", action="store_true")
    a = ap.parse_args(argv)
    spec = json.loads(SPEC.read_text())
    lines_by_key: dict[tuple[str, int, str, bool | None], list[tuple[Any, ...]]] = {}
    for iclass, pages in ICLASSES.items():
        mnemonic = "MOVSD" if iclass == "MOVSD_XMM" else iclass
        for page in pages.split():
            for ln in sdm_lines(page):
                pfx, rex_w, op, mn, ops, en = ln
                if mn != mnemonic or any(o.startswith("mm") for o in ops):
                    continue
                key = ("none" if pfx == "NP" else pfx.lower(), op, iclass, rex_w)
                lines_by_key.setdefault(key, []).append(ln)
    written = 0
    for e in spec["entries"]:
        if e["map"] != "0f" or e.get("vex") or e.get("evex") or e["iclass"] not in ICLASSES or e.get("ops"):
            continue
        if e["iclass"] in ("MOVD", "MOVQ") and e["pfx"] == "none":
            continue                                  # the MMX forms (mm registers): not lifted
        group = [ln for (pfx, op, ic, w), lns in lines_by_key.items() for ln in lns
                 if pfx == e["pfx"] and op == e["op"] and ic == e["iclass"]
                 and ("w" not in e or w == bool(e["w"]))]
        if not group:
            print(f"no SDM line for {e['iclass']} {e['pfx']} 0F {e['op']:02X}", file=sys.stderr)
            return 1
        e["ops"] = template(group)
        e["src_ops"] = ("SDM Vol 2 instruction page Opcode/Instruction and Op/En columns "
                        "(felixcloutier transcription, retrieved 2026-10-09); notation Vol 2D Table A-1: V, W, G, E")
        written += 1
        print(f"{e['iclass']:10} {e['pfx']:4} 0F {e['op']:02X} {'w' + str(e['w']) if 'w' in e else '  '} {e['ops']}")
    print(f"{written} rows")
    if a.write:
        spec["version"] = "0.7.1"
        spec["revision"] += ("; 0.7.1 (2026-10-09): operand templates for the legacy-SSE rows the model lifts "
                             "(V, W, G, E with d, q, dq, y; scripts/sse_operand_templates.py; Sprint 039)")
        SPEC.write_text(json.dumps(spec, indent=1) + "\n")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
