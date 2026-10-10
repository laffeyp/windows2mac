"""generated_differential.py — w2m decoder vs Intel XED on generated encodings (Sprint 020).

Outcome classes per case:
  both_reject      XED and ours both refuse the bytes
  agree            same length and iclass
  ours_accepts     XED refuses (the CPU would raise #UD); ours decodes — a validity gap
  ours_gap         XED decodes; ours has no entry — a coverage gap outside the corpus
  length_differs   both decode, different length — a structural bug
  iclass_differs   both decode, same length, different iclass
Operands (Sprint 022): on `agree` cases whose row carries an operand template, the explicit
operands are compared with XED's after x86ops.normalize.

Per-row confirmation (Sprint 023, review 2026-10-08 item 4.2): --confirmations writes, for every
spec row, its outcome counts and whether XED confirmed it (no ours_accepts, ours_gap, length,
iclass or operand difference among its cases). Rows are keyed by the sha256 of their canonical
JSON, so an edited row loses its confirmation until the differential runs again.

Signal plane (Sprint 025; causal since Sprint 043): --record writes one differential run
(src/w2m/differential.py) whose units are the ways the decoder can disagree with XED: length,
iclass, ours_accepts, ours_gap, operands. Each unit's comparison causes its acceptance or its report;
length_differs routes to implementation_repair (the spec row's fields agree with XED; the decoder
routine is wrong), every other unit to contract_revision (the spec lacks the row or rule XED
applies). A report's example bytes are stored at store/programs/<hex>.bin, named by
minimized_program_sha256.

Usage: uv run python scripts/generated_differential.py [--per-row 48] [--chip ALL] [--json out]
       [--confirmations specs/x86_decode_confirmations.json] [--record] [--runs runs]
"""

from __future__ import annotations

import argparse
import hashlib
import json
import sys
import time
from collections import Counter
from pathlib import Path
from typing import Any



from w2m.encgen import SPEC, cases
from w2m.x86dec import DecodeError, Decoded, decode
from w2m.x86dec_tables import SPEC_VERSION
from w2m.x86ops import OperandError, normalize, operands
from w2m.xedref import decode_many


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--per-row", type=int, default=48)
    ap.add_argument("--chip", default="ALL")
    ap.add_argument("--json", type=Path)
    ap.add_argument("--confirmations", type=Path)
    ap.add_argument("--record", action="store_true")
    ap.add_argument("--runs", type=Path, default=Path("runs"))
    ap.add_argument("--store", type=Path, default=Path("store"))
    a = ap.parse_args(argv)
    t0 = time.monotonic()
    cs = list(cases(per_row=a.per_row))
    xs = decode_many([c.raw for c in cs], a.chip)
    outcome: Counter[str] = Counter()
    kinds: Counter[tuple[str, str, str, str]] = Counter()          # (outcome, xed, ours, lengths)
    example: dict[tuple[str, str, str, str], str] = {}
    per_row: dict[int, Counter[str]] = {}
    op_n = 0
    op_diff: Counter[tuple[str, str]] = Counter()
    op_ex: dict[tuple[str, str], str] = {}
    for c, x in zip(cs, xs):
        try:
            g: Decoded | None = decode(c.raw)   # same bytes XED sees; no padding
            assert g is not None
            ours: tuple[int, str] | None = (g.length, g.iclass)
        except DecodeError:   # InvalidEncoding, Truncated, no row; any other exception is a bug
            g, ours = None, None
        if not x.ok and ours is None:
            o, k = "both_reject", ("", "", "")
        elif x.ok and ours is not None and (x.length, x.iclass) == ours:
            o, k = "agree", ("", "", "")
            if g is not None and g.fields is not None and g.fields.ops:
                try:
                    mine = ";".join(map(normalize, operands(g.fields)))
                except OperandError:
                    mine = None
                if mine is not None:
                    op_n += 1
                    theirs = ";".join(map(normalize, x.operands))
                    if mine != theirs:
                        op_diff[(x.iform, f"{theirs} | {mine}")] += 1
                        per_row.setdefault(c.row, Counter())["operand_differs"] += 1
                        op_ex.setdefault((x.iform, f"{theirs} | {mine}"), c.raw[:x.length].hex())
        elif not x.ok:
            assert ours is not None
            o, k = "ours_accepts", (x.error, ours[1], "")
        elif ours is None:
            o, k = "ours_gap", (x.iclass, "", "")
        elif ours[0] != x.length:
            o, k = "length_differs", (x.iclass, ours[1], f"{x.length}/{ours[0]}")
        else:
            o, k = "iclass_differs", (x.iclass, ours[1], "")
        outcome[o] += 1
        per_row.setdefault(c.row, Counter())[o] += 1
        if o not in ("both_reject", "agree"):
            kinds[(o, k[0], k[1], k[2])] += 1
            example.setdefault((o, k[0], k[1], k[2]), f"{c.raw[:x.length or 15].hex()} [{c.note}]")
    total = len(cs)
    print(f"cases {total} ({a.per_row} per row, chip {a.chip}); {time.monotonic() - t0:.0f} s")
    for o in ("agree", "both_reject", "ours_accepts", "ours_gap", "length_differs", "iclass_differs"):
        print(f"  {o:15} {outcome[o]:7} ({outcome[o] / total:.2%})")
    print(f"operands: compared {op_n}; differ {sum(op_diff.values())} in {len(op_diff)} kinds")
    for ok_, n in op_diff.most_common(12):
        print(f"  {n:6} {ok_[0]} xed | ours: {ok_[1]} {op_ex[ok_]}")
    for o in ("length_differs", "iclass_differs", "ours_gap", "ours_accepts"):
        top = [(k, n) for k, n in kinds.most_common() if k[0] == o][:12]
        if top:
            print(f"{o} (xed, ours, lengths): count, example")
            for kind, n in top:
                print(f"  {n:6} {kind[1:]} {example[kind]}")
    if a.confirmations:
        rows = json.loads(SPEC.read_text())["entries"]
        bad = ("ours_accepts", "ours_gap", "length_differs", "iclass_differs", "operand_differs")
        conf = {}
        for idx, e in enumerate(rows):
            row_counts = per_row.get(idx, Counter())
            key = "sha256:" + hashlib.sha256(json.dumps(e, sort_keys=True).encode()).hexdigest()
            conf[key] = {"row": idx, "mn": e["mn"], "cases": sum(row_counts.values()), **dict(sorted(row_counts.items())),
                         "confirmed": sum(row_counts.values()) > 0 and not any(row_counts[b] for b in bad)}
        ok = sum(v["confirmed"] for v in conf.values())
        a.confirmations.write_text(json.dumps({
            "reference": f"Intel XED v2026.08.23 (0bcb6237345c), chip {a.chip}",
            "cases_per_row": a.per_row, "rows": len(conf), "confirmed": ok, "by_row_sha256": conf},
            indent=1) + "\n")
        print(f"confirmations: {ok} of {len(conf)} rows confirmed by XED -> {a.confirmations}")
    if a.record:
        classes: list[tuple[tuple[str, ...], int, str]] = [(k, n, example[k]) for k, n in kinds.most_common()]
        classes += [(("operand_differs", *k, ""), n, op_ex[k]) for k, n in op_diff.most_common()]
        print(json.dumps(record(a, classes, total), indent=1))
    if a.json:
        a.json.write_text(json.dumps({"cases": total, "chip": a.chip, "outcomes": dict(outcome),
                                      "operands_compared": op_n,
                                      "operand_differs": [{"iform": k[0], "xed_vs_ours": k[1],
                                                           "count": n, "example": op_ex[k]}
                                                          for k, n in op_diff.most_common()],
                                      "kinds": [{"outcome": k[0], "xed": k[1], "ours": k[2],
                                                 "lengths": k[3], "count": n, "example": example[k]}
                                                for k, n in kinds.most_common()]}, indent=1) + "\n")
    return 0


XED_PROFILE = "xed-v2026.08.23-0bcb6237345c"
EMPTY = "sha256:" + hashlib.sha256(b"").hexdigest()


UNITS = ("length_differs", "iclass_differs", "ours_accepts", "ours_gap", "operand_differs")


def record(a: argparse.Namespace, classes: list[tuple[tuple[str, ...], int, str]], total: int) -> dict[str, Any]:
    """One causal differential run (Sprint 043): a unit per way the decoder can disagree with XED
    (UNITS), accepted when that way has no case. length_differs routes to implementation_repair (the
    row's fields agree with XED; the decoder's length routine is wrong); the others to
    contract_revision (the spec lacks the row or validity rule XED applies)."""
    from w2m import differential

    per_unit: dict[str, list[tuple[tuple[str, ...], int, str]]] = {u: [] for u in UNITS}
    for k, n, ex in classes:
        per_unit[k[0]].append((k, n, ex))

    def judge_unit(unit: str) -> differential.Verdict:
        found = per_unit[unit]
        rule = f"{total} generated cases, {a.per_row} per row, chip {a.chip}"
        if not found:
            return differential.Verdict("conforms", rule_ref=rule)
        lines = tuple(f"xed={k[1] or '-'} ours={k[2] or '-'}{' lengths ' + k[3] if k[3] else ''}; {n} cases; {ex}"
                      for k, n, ex in found)
        impl = unit == "length_differs"
        return differential.Verdict(
            "discrepancy", lines, rule, "implementation_repair" if impl else "contract_revision",
            "spec row fields agree with XED; the decoder's length routine is wrong" if impl else
            "the spec lacks the row or validity rule XED applies (Decision 2026-10-08: XED judges)",
            program=bytes.fromhex(found[0][2].split()[0]))

    spec_raw = SPEC.read_bytes()
    return differential.run(differential.Differential(
        name="decoder-generated", profile=XED_PROFILE, purpose="conformance",
        contract_id=lambda unit: "x86.decode", contract_version=SPEC_VERSION, relation="exact",
        implementation=f"w2m decoder, spec {SPEC_VERSION}",
        implementation_sha256="sha256:" + hashlib.sha256(spec_raw).hexdigest(),
        generator_ref="hand (agent-authored) and the Intel SDM rows", reference_ref=XED_PROFILE,
        target_execution_id=f"x86dec-spec-{SPEC_VERSION}", units=UNITS, compare=judge_unit,
        baseline={"cases": total, "chip": a.chip, "per_row": a.per_row}, programs=a.store / "programs"), a.runs)


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
