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
    # Sprint 044: the SSE2 integer block the launcher's C runtime reaches
    "MOVDQU": "movdqu:vmovdqu8:vmovdqu16:vmovdqu32:vmovdqu64", "MOVDQA": "movdqa:vmovdqa32:vmovdqa64",
    "PAND": "pand", "POR": "por", "ORPS": "orps", "PSUBQ": "psubq", "PSUBD": "psubb:psubw:psubd",
    "PSRLQ": "psrlw:psrld:psrlq", "PUNPCKLBW": "punpcklbw:punpcklwd:punpckldq:punpcklqdq",
    "CVTDQ2PD": "cvtdq2pd", "MOVNTPS": "movntps", "PSRLDQ": "psrldq", "PSLLDQ": "pslldq",
    "PCMPEQB": "pcmpeqb:pcmpeqw:pcmpeqd", "PMOVMSKB": "pmovmskb", "PSHUFD": "pshufd",
}
LINE = re.compile(r"^(NP |66 |F2 |F3 )?(REX\.W\s*\+?\s*)?0F ([0-9A-F]{2})\s*/r\s+([A-Z0-9]+)\s+(.*)$")
MEM_SIZE = {"m32": "d", "m64": "q", "m128": "dq"}
# Sprint 044: rows whose SDM line the transcription above cannot read (an immediate, a register-only or
# memory-only ModRM.r/m operand), written from the SDM line each row cites: (iclass, prefix, opcode,
# ModRM.reg or None) -> template. Wdq for an r/m that the SDM restricts to a register (mod 11) or to memory:
# the decoder refuses the other form, so W reads only the allowed one.
MANUAL = {("PSRLQ", "66", 0x73, 2): "Wdq,Ib", ("PSRLDQ", "66", 0x73, 3): "Wdq,Ib", ("PSLLDQ", "66", 0x73, 7): "Wdq,Ib",
          ("PSRLQ", "66", 0xD3, None): "V,Wdq", ("PUNPCKLBW", "66", 0x60, None): "V,Wdq",
          ("PSHUFD", "66", 0x70, None): "V,Wdq,Ib", ("PMOVMSKB", "66", 0xD7, None): "Gd,Wdq",
          ("MOVNTPS", "none", 0x2B, None): "Wdq,V", ("STMXCSR", "none", 0xAE, 3): "Ed", ("LDMXCSR", "none", 0xAE, 2): "Ed",
          ("PREFETCHNTA", "none", 0x18, 0): "Mcl",
          # Sprint 046: the second SSE block, the classes UnityPlayer.dll reaches most without a rule (census
          # 2026-10-10) that raise no floating-point exception; MOVHLPS and MOVLHPS are the register-only rows
          ("SHUFPS", "none", 0xC6, None): "V,Wdq,Ib", ("PXOR", "66", 0xEF, None): "V,Wdq",
          ("ANDNPS", "none", 0x55, None): "V,Wdq", ("UNPCKLPS", "none", 0x14, None): "V,Wdq",
          ("UNPCKHPS", "none", 0x15, None): "V,Wdq", ("MOVHLPS", "none", 0x12, None): "V,Wdq",
          ("MOVLHPS", "none", 0x16, None): "V,Wdq", ("PUNPCKLDQ", "66", 0x62, None): "V,Wdq",
          ("PADDD", "66", 0xFE, None): "V,Wdq", ("PSRLD", "66", 0xD2, None): "V,Wdq", ("PSRLD", "66", 0x72, 2): "Wdq,Ib",
          ("PSLLD", "66", 0xF2, None): "V,Wdq", ("PSLLD", "66", 0x72, 6): "Wdq,Ib", ("PSRAD", "66", 0xE2, None): "V,Wdq",
          ("PSRAD", "66", 0x72, 4): "Wdq,Ib", ("MOVMSKPS", "none", 0x50, None): "Gd,Wdq",
          # CMPXCHG8B m64 / CMPXCHG16B m128 (REX.W): the row names both by operand size
          ("CMPXCHG8B", "none", 0xC7, 1): "Mqdq",
          # Sprint 046: the third SSE block, the SSE2 integer classes UnityPlayer.dll reaches most (census 2026-10-10);
          # MOVHPS's two rows are memory only (W reads memory there), PINSRW's source is r32/m16 (Ew: its low word)
          ("PUNPCKLQDQ", "66", 0x6C, None): "V,Wdq",
          ("PADDQ", "66", 0xD4, None): "V,Wdq",
          ("PCMPGTD", "66", 0x66, None): "V,Wdq",
          ("PCMPEQD", "66", 0x76, None): "V,Wdq",
          ("PANDN", "66", 0xDF, None): "V,Wdq",
          ("PSUBUSB", "66", 0xD8, None): "V,Wdq",
          ("PMAXUB", "66", 0xDE, None): "V,Wdq",
          ("UNPCKLPD", "66", 0x14, None): "V,Wdq",
          ("PACKUSWB", "66", 0x67, None): "V,Wdq",
          ("PADDSB", "66", 0xEC, None): "V,Wdq",
          ("PADDSW", "66", 0xED, None): "V,Wdq",
          ("PADDW", "66", 0xFD, None): "V,Wdq",
          ("PMULLW", "66", 0xD5, None): "V,Wdq",
          ("PSUBSB", "66", 0xE8, None): "V,Wdq",
          ("PSUBW", "66", 0xF9, None): "V,Wdq",
          ("PUNPCKHBW", "66", 0x68, None): "V,Wdq",
          ("PUNPCKHDQ", "66", 0x6A, None): "V,Wdq",
          ("PUNPCKHWD", "66", 0x69, None): "V,Wdq",
          ("PUNPCKLWD", "66", 0x61, None): "V,Wdq",
          ("PSLLQ", "66", 0x73, 6): "Wdq,Ib", ("PSLLQ", "66", 0xF3, None): "V,Wdq", ("PSRLW", "66", 0x71, 2): "Wdq,Ib",
          ("PSRLW", "66", 0xD1, None): "V,Wdq", ("PSRAW", "66", 0x71, 4): "Wdq,Ib", ("PSRAW", "66", 0xE1, None): "V,Wdq",
          ("PSHUFLW", "f2", 0x70, None): "V,Wdq,Ib", ("PSHUFHW", "f3", 0x70, None): "V,Wdq,Ib",
          ("PINSRW", "66", 0xC4, None): "V,Ew,Ib", ("MOVHPS", "none", 0x16, None): "V,Wq", ("MOVHPS", "none", 0x17, None): "Wq,V"}
MMX_FORMS = {"PSRLQ", "PUNPCKLBW", "PMOVMSKB", "PSHUFD"}       # their NP rows use mm registers: not lifted


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
        manual = MANUAL.get((e["iclass"], e["pfx"], e["op"], e.get("reg")))
        if manual and e["map"] == "0f" and not e.get("vex") and not e.get("evex") and not e.get("ops"):
            e["ops"] = manual
            e["src_ops"] = f"written by hand from the row's SDM line ({e['src']}); notation Vol 2D Table A-1 (Sprint 044)"
            written += 1
            print(f"{e['iclass']:10} {e['pfx']:4} 0F {e['op']:02X} /{e.get('reg', 'r')} {e['ops']} (by hand)")
            continue
        if e["map"] != "0f" or e.get("vex") or e.get("evex") or e["iclass"] not in ICLASSES or e.get("ops"):
            continue
        if e["iclass"] in MMX_FORMS and e["pfx"] == "none":
            continue                                  # the MMX forms (mm registers): not lifted
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
        if spec["version"] == "0.7.1":
            spec["version"] = "0.7.2"
            spec["revision"] += ("; 0.7.2 (2026-10-10): operand templates for the SSE2 integer rows the launcher's C "
                                 "runtime reaches, MXCSR access and PREFETCHNTA (Sprint 044)")
        elif spec["version"] == "0.7.4":
            spec["version"] = "0.7.5"
            spec["revision"] += "; 0.7.5 (2026-10-10): operand templates for the third SSE block (Sprint 046)"
        elif spec["version"] == "0.7.3":
            spec["version"] = "0.7.4"
            spec["revision"] += "; 0.7.4 (2026-10-10): CMPXCHG8B/16B's template Mqdq (Sprint 046)"
        elif spec["version"] == "0.7.2":
            spec["version"] = "0.7.3"
            spec["revision"] += ("; 0.7.3 (2026-10-10): operand templates for the second SSE block, the classes "
                                 "UnityPlayer.dll reaches most without a rule and without floating-point exceptions (Sprint 046)")
        SPEC.write_text(json.dumps(spec, indent=1) + "\n")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
