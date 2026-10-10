"""decoder_differential.py — decoder differential over every PE in a directory (Sprints 016-017;
--record, a causal differential run with one unit per PE file, Sprint 043).

Positions come from the Capstone recursive traversal. Intel XED (primary reference, decision
2026-10-08) judges length at every position; Capstone and w2m's spec-generated decoder are each
compared with XED. The report also keeps the w2m-vs-Capstone mnemonic comparison and lists spec
gaps (opcodes the spec has no entry for), each with a byte example.

Usage: uv run python scripts/decoder_differential.py <dir> [--json out.json]
"""

from __future__ import annotations

import argparse
import json
import sys
import time
from collections import Counter
from pathlib import Path

from w2m.pe import PeError, parse
from w2m.x86dec import DecodeError, decode
from w2m.x86ref import walk
from w2m.xedref import decode_many

SUFFIXES = (".exe", ".dll", ".pyd")


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("root", type=Path)
    ap.add_argument("--json", type=Path)
    ap.add_argument("--record", action="store_true")
    ap.add_argument("--name", help="the corpus's name in the record (default: the directory's name)")
    ap.add_argument("--runs", type=Path, default=Path("runs"))
    ap.add_argument("--store", type=Path, default=Path("store"))
    a = ap.parse_args(argv)
    t0 = time.monotonic()
    total = agree = 0
    gaps: Counter[tuple[str, str]] = Counter()
    diffs: Counter[tuple[str, str, int, int]] = Counter()
    example: dict[object, str] = {}
    per_file: list[tuple[str, int, int, int]] = []
    xed_bad: Counter[tuple[str, str]] = Counter()
    cs_vs_xed: Counter[tuple[str, int, int]] = Counter()
    ours_vs_xed: Counter[tuple[str, int, int]] = Counter()
    iclass_vs_xed: Counter[tuple[str, str]] = Counter()
    against_xed: dict[str, list[tuple[str, str, str]]] = {}     # file -> (kind, detail, example bytes)
    for f in sorted(a.root.iterdir()):
        if f.suffix.lower() not in SUFFIXES:
            continue
        data = f.read_bytes()
        try:
            img = parse(data)
        except PeError as e:
            print(f"not a PE32+ x86-64 image: {f.name}: {e}")
            continue
        r = walk(img, data)
        mine = against_xed.setdefault(f.name, [])
        ok = 0
        insns = list(r.insns.values())
        raws = [img.read_rva(data, i.rva, 15) for i in insns]
        for ins, raw, x in zip(insns, raws, decode_many(raws)):
            total += 1
            if not x.ok:
                xk = (x.error, ins.mnemonic)
                xed_bad[xk] += 1
                example.setdefault(xk, ins.raw.hex())
            elif x.length != ins.size:
                ck = (x.iclass, x.length, ins.size)
                cs_vs_xed[ck] += 1
                example.setdefault(ck, ins.raw.hex())
            try:
                got = decode(raw)
            except DecodeError as e:
                gk = (str(e).split(" with ")[0].split(" modrm")[0], ins.mnemonic)
                gaps[gk] += 1
                example.setdefault(gk, ins.raw.hex())
                if x.ok:
                    mine.append(("ours_gap", f"xed={x.iclass} capstone={ins.mnemonic}: {gk[0]}", raw[:x.length].hex()))
                continue
            if x.ok and got.length != x.length:
                ok_key = (x.iclass, x.length, got.length)
                ours_vs_xed[ok_key] += 1
                example.setdefault(ok_key, raw[:x.length].hex())
                mine.append(("length_differs", f"xed={x.iclass} lengths {x.length}/{got.length}", raw[:x.length].hex()))
            elif x.ok and got.iclass != x.iclass:
                ik = (x.iclass, got.iclass)
                iclass_vs_xed[ik] += 1
                example.setdefault(ik, raw[:x.length].hex())
                mine.append(("iclass_differs", f"xed={x.iclass} ours={got.iclass}", raw[:x.length].hex()))
            if (got.length, got.mnemonic) == (ins.size, ins.mnemonic):
                agree += 1
                ok += 1
            else:
                dk = (ins.mnemonic, got.mnemonic, ins.size, got.length)
                diffs[dk] += 1
                example.setdefault(dk, ins.raw.hex())
        per_file.append((f.name, len(r.insns), ok, len(r.undecodable)))
    print(f"files {len(per_file)}; instructions {total}; agree {agree} ({agree / total:.2%}); "
          f"spec gaps {sum(gaps.values())} in {len(gaps)} kinds; disagreements "
          f"{sum(diffs.values())} in {len(diffs)} kinds; {time.monotonic() - t0:.0f} s")
    print(f"vs XED: XED cannot decode {sum(xed_bad.values())}; Capstone length differs "
          f"{sum(cs_vs_xed.values())}; ours length differs {sum(ours_vs_xed.values())}; ours iclass "
          f"differs {sum(iclass_vs_xed.values())}")
    for ik, n in iclass_vs_xed.most_common(10):
        print(f"  iclass vs XED (xed, ours) {n:6} {ik} {example[ik]}")
    for k3, n in (cs_vs_xed + ours_vs_xed).most_common(10):
        print(f"  length vs XED {n:6} {k3} {example[k3]}")
    for name, n, ok, und in sorted(per_file, key=lambda x: -x[1])[:8]:
        print(f"  {name:22} {n:8} insns  agree {ok / n:.1%}  capstone-undecodable {und}")
    print("top spec gaps (missing entry, capstone mnemonic): count, example bytes")
    for k, n in gaps.most_common(25):
        print(f"  {n:6} {k} {example[k]}")
    print("disagreements (capstone mnemonic, ours, capstone length, ours): count, example bytes")
    for dk, n in diffs.most_common(15):
        print(f"  {n:6} {dk} {example[dk]}")
    if a.json:
        a.json.write_text(json.dumps({
            "files": len(per_file), "instructions": total, "agree": agree,
            "gaps": [{"entry": k[0], "capstone": k[1], "count": n, "example": example[k]}
                     for k, n in gaps.most_common()],
            "disagreements": [{"capstone": k[0], "ours": k[1], "capstone_len": k[2],
                               "our_len": k[3], "count": n, "example": example[k]}
                              for k, n in diffs.most_common()]}, indent=1) + "\n")
    if a.record:
        print(json.dumps(record(a, against_xed, {name: n for name, n, _, _ in per_file}), indent=1))
    return 0


XED_PROFILE = "xed-v2026.08.23-0bcb6237345c"


def record(a: argparse.Namespace, against_xed: dict[str, list[tuple[str, str, str]]],
           sizes: dict[str, int]) -> dict[str, object]:
    """One causal differential run (Sprint 043): a unit per PE file, judged against Intel XED at
    every position Capstone's traversal reaches. A file with a length difference routes to
    implementation_repair (the decoder routine is wrong); one with only gaps or iclass differences
    to contract_revision (the spec lacks the row or rule XED applies)."""
    import hashlib

    from w2m import differential
    from w2m.encgen import SPEC
    from w2m.x86dec_tables import SPEC_VERSION

    def judge_unit(unit: str) -> differential.Verdict:
        found = against_xed[unit]
        rule = f"{sizes[unit]} positions of {unit} (Capstone traversal), each judged by XED"
        if not found:
            return differential.Verdict("conforms", rule_ref=rule)
        kinds = Counter((k, detail) for k, detail, _ in found)
        first_example = {(k, detail): ex for k, detail, ex in reversed(found)}
        lines = tuple(f"{k} {detail}: {n} positions; {first_example[(k, detail)]}" for (k, detail), n in kinds.most_common())
        impl = any(k == "length_differs" for k, _, _ in found)
        return differential.Verdict(
            "discrepancy", lines, rule, "implementation_repair" if impl else "contract_revision",
            "spec row fields agree with XED; the decoder's length routine is wrong" if impl else
            "the spec lacks the row or rule XED applies (Decision 2026-10-08: XED judges)",
            program=bytes.fromhex(found[0][2]))

    name = a.name or a.root.name
    return differential.run(differential.Differential(
        name=f"decoder-corpus-{name}", profile=XED_PROFILE, purpose="conformance",
        contract_id=lambda unit: "x86.decode", contract_version=SPEC_VERSION, relation="exact",
        implementation=f"w2m decoder, spec {SPEC_VERSION}",
        implementation_sha256="sha256:" + hashlib.sha256(SPEC.read_bytes()).hexdigest(),
        generator_ref="hand (agent-authored) and the Intel SDM rows", reference_ref=XED_PROFILE,
        target_execution_id=f"x86dec-spec-{SPEC_VERSION}-{name}", units=sorted(against_xed),
        compare=judge_unit, baseline={"corpus": name, "positions": sum(sizes.values())},
        programs=a.store / "programs"), a.runs)


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
