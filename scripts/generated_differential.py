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

Signal plane (Sprint 025): --record writes a substrate run: SESSION_INIT, COMPARISON_COMPLETED
(relation exact, contract x86.decode at the spec version, reference XED), then per disagreement
class DISCREPANCY_REPORTED and DISCREPANCY_ROUTED, then SESSION_COMPLETED. Routing follows the
vocabulary's rule from the outcome class: length_differs -> implementation_repair (the spec row's
fields agree with XED; the decoder routine is wrong); every other class -> contract_revision (the
spec lacks the row or rule XED applies). Each class's example bytes are stored at
store/programs/<hex>.bin, named by minimized_program_sha256.

Usage: uv run python scripts/generated_differential.py [--per-row 48] [--chip ALL] [--json out]
       [--confirmations specs/x86_decode_confirmations.json] [--record] [--runs runs]
"""

from __future__ import annotations

import argparse
import asyncio
import hashlib
import json
import sys
import time
from collections import Counter
from collections.abc import AsyncIterator
from pathlib import Path
from typing import Any

import msgspec
from substrate.api import Runtime, TopologyBuilder, read_record, threshold_count

from w2m import VOCAB_VERSION, events

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
    kinds: Counter[tuple[str, str, str]] = Counter()
    example: dict[tuple[str, str, str], str] = {}
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
                        per_row[c.row]["operand_differs"] += 1
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
            kinds[(o, *k[:2], k[2])] += 1  # type: ignore[arg-type]
            example.setdefault((o, *k[:2], k[2]), f"{c.raw[:x.length or 15].hex()} [{c.note}]")  # type: ignore[arg-type]
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
            for k, n in top:
                print(f"  {n:6} {k[1:]} {example[k]}")
    if a.confirmations:
        rows = json.loads(SPEC.read_text())["entries"]
        bad = ("ours_accepts", "ours_gap", "length_differs", "iclass_differs", "operand_differs")
        conf = {}
        for idx, e in enumerate(rows):
            n = per_row.get(idx, Counter())
            key = "sha256:" + hashlib.sha256(json.dumps(e, sort_keys=True).encode()).hexdigest()
            conf[key] = {"row": idx, "mn": e["mn"], "cases": sum(n.values()), **dict(sorted(n.items())),
                         "confirmed": sum(n.values()) > 0 and not any(n[b] for b in bad)}
        ok = sum(v["confirmed"] for v in conf.values())
        a.confirmations.write_text(json.dumps({
            "reference": f"Intel XED v2026.08.23 (0bcb6237345c), chip {a.chip}",
            "cases_per_row": a.per_row, "rows": len(conf), "confirmed": ok, "by_row_sha256": conf},
            indent=1) + "\n")
        print(f"confirmations: {ok} of {len(conf)} rows confirmed by XED -> {a.confirmations}")
    if a.record:
        classes = [(k, n, example[k]) for k, n in kinds.most_common()]
        classes += [(("operand_differs", *k, ""), n, op_ex[k]) for k, n in op_diff.most_common()]
        record(a, classes, total, time.monotonic() - t0)
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


def record(a: argparse.Namespace, classes: list[tuple[tuple[str, ...], int, str]], total: int,
           elapsed: float) -> None:
    """One substrate run holding this differential's verdict and its routed discrepancies."""
    run_id = f"gendiff-{time.time_ns()}"
    ours = f"w2m-x86dec-spec-{SPEC_VERSION}"
    progs = a.store / "programs"
    progs.mkdir(parents=True, exist_ok=True)

    async def harness(_inp: Any) -> AsyncIterator[msgspec.Struct]:
        yield events.validate(events.SESSION_INIT(
            vocab_version=VOCAB_VERSION, profile=ours, contract_version=SPEC_VERSION,
            harness_version="0.0.1", purpose="conformance"))
        cmp_id = f"cmp-{run_id}"
        yield events.validate(events.COMPARISON_COMPLETED(
            comparison_id=cmp_id, contract_id="x86.decode", contract_version=SPEC_VERSION,
            relation="exact", reference_ref=XED_PROFILE, target_execution_id=run_id,
            verdict="discrepancy" if classes else "conforms",
            rule_ref=f"{total} generated cases, {a.per_row} per row, chip {a.chip}"))
        for i, (k, n, ex) in enumerate(classes, 1):
            raw = bytes.fromhex(ex.split()[0])
            digest = hashlib.sha256(raw).hexdigest()
            (progs / f"{digest}.bin").write_bytes(raw)
            did = f"{run_id}-d{i}"
            yield events.validate(events.DISCREPANCY_REPORTED(
                discrepancy_id=did, contract_id="x86.decode", contract_version=SPEC_VERSION,
                mismatch_class="exact", minimized_program_sha256=f"sha256:{digest}",
                initial_state_sha256=EMPTY,
                first_divergence=f"{k[0]}: xed={k[1] or '-'} ours={k[2] or '-'}"
                                 f"{' lengths ' + k[3] if k[3] else ''}; {n} cases",
                reference_profile_id=XED_PROFILE, destination_profile_id=ours,
                comparison_id=cmp_id))
            impl = k[0] == "length_differs"
            yield events.validate(events.DISCREPANCY_ROUTED(
                discrepancy_id=did,
                repair_path="implementation_repair" if impl else "contract_revision",
                reference_conforms=impl,
                grounds=("spec row fields agree with XED; the decoder's length routine is wrong"
                         if impl else "the spec lacks the row or validity rule XED applies "
                                      "(Decision 2026-10-08: XED judges)")))
        yield events.validate(events.SESSION_COMPLETED(
            outcome="completed_with_discrepancies" if classes else "completed",
            discrepancy_count=len(classes), accepted_count=0, elapsed_seconds=round(elapsed, 3)))

    def topology(b: TopologyBuilder) -> None:
        b.baseline(spec_version=SPEC_VERSION, reference=XED_PROFILE, cases=total, chip=a.chip)
        b.producer_kind("comparator", schemas=list(events.HARNESS_EVENTS), schema_version=1,
                        factory=lambda: harness)
        b.initial("comparator", input=None)
        b.termination(threshold_count("substrate.ProducerCompleted", 1))

    result = asyncio.run(Runtime(a.runs / run_id).run(topology))
    kinds = Counter(e["kind"] for e in read_record(result.record_root)
                    if not e["kind"].startswith("substrate."))
    print(f"record {result.record_root}: {dict(kinds)}")


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
