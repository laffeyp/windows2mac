"""check_vocab.py — structural gate for signals/<version>.json (Sprint 000 self-review, BOOTSTRAP Steps 1-10).

Usage: python3 scripts/check_vocab.py [signals/<version>.json]   (default: signals/current.json)
Exit 0 when every check passes; 1 with the failures listed on stderr.
"""
from __future__ import annotations

import json
import re
import sys

IRREGULAR_PAST = {"READ", "BUILT", "INIT"}  # INIT: the SESSION_INIT pattern (TECHNIQUES.md #9)
FK_ALIAS = {
    "evidence": "evidence", "deciding_evidence": "evidence", "superseded_by": "evidence",
    "contract": "contract", "probe": "probe", "parent_probe": "probe",
    "execution": "execution", "target_execution": "execution", "model_check": "modelcheck",
    "comparison": "comparison", "reduction": "discrepancy", "discrepancy": "discrepancy",
    "triggering_discrepancy": "discrepancy", "candidate": "candidate", "parent_candidate": "candidate",
    "translation": "translation", "artifact": "artifact", "evaluation": "backend",
    "measurement": "measurement", "application_profile": "applicationprofile",
    "destination_profile": "destinationprofile", "host": "destinationprofile",
    "reference_profile": "referenceprofile", "profile": "referenceprofile", "bundle": "bundle",
    "app": "distribution", "build": "distribution", "from_build": "distribution",
    "to_build": "distribution", "depot": "distribution", "thread": "guestthread",
    "parent_thread": "guestthread",
}


def check(path: str) -> list[str]:
    v = json.load(open(path))
    tags = {t["name"]: t for t in v["tags"]}
    cats = {c["name"] for c in v["categories"]}
    enums, types = v["enums"], v["types"]
    errs: list[str] = []
    for n, t in tags.items():
        for k in ("name", "category", "stratum", "payload", "note"):
            if k not in t:
                errs.append(f"{n}: missing {k}")
        if t["category"] not in cats:
            errs.append(f"{n}: unknown category {t['category']}")
        if t["stratum"] not in ("event", "ambient", "summary", "incident"):
            errs.append(f"{n}: bad stratum {t['stratum']}")
        fields = t["payload"] + t.get("optional_payload", [])
        if set(fields) != set(t["types"]):
            errs.append(f"{n}: untyped or extra fields {sorted(set(fields) ^ set(t['types']))}")
        for f, ty in t["types"].items():
            base = ty[5:-1] if ty.startswith("list<") else ty
            if base.startswith("enum:"):
                if base[5:] not in enums:
                    errs.append(f"{n}.{f}: unknown enum {base}")
            elif base not in types:
                errs.append(f"{n}.{f}: unknown type {ty}")
        if not re.fullmatch(r"[A-Z0-9]+(_[A-Z0-9]+)+", n):
            errs.append(f"{n}: name is not UPPER_SNAKE with an entity and a verb")
        last = n.split("_")[-1]
        if not (last.endswith("ED") or last in IRREGULAR_PAST):
            errs.append(f"{n}: verb {last} is not past tense")
    emitted: set[str] = set()
    for o in v["operators"]["operators"]:
        for t in o["emits"] + o["consumes"]:
            if t not in tags:
                errs.append(f"operator {o['name']}: unknown tag {t}")
        emitted |= set(o["emits"])
    for n in sorted(set(tags) - emitted):
        errs.append(f"{n}: no operator emits it")
    for r in v["state_transitions"]["rules"]:
        for t in [r["from_tag"], *r.get("to_tags_allowed", []), *r.get("to_tags_forbidden", [])]:
            if t not in tags and t != "guest:*":
                errs.append(f"{r['id']}: unknown tag {t}")
    for i in v["temporal_invariants"]["invariants"]:
        for t in i["tags"]:
            if t not in tags and t != "guest:*":
                errs.append(f"{i['id']}: unknown tag {t}")
    cons = {c["id"]: c["spec"] for c in v["evidence_constraints"]["constraints"]}
    cadenced = " ".join(i["rule"] for i in v["temporal_invariants"]["invariants"] if i["kind"] == "cadence")
    for n, t in tags.items():
        if t["stratum"] == "incident" and n not in cons.get("E7", ""):
            errs.append(f"{n}: incident tag without a diagnostic constraint (E7)")
        if t["stratum"] == "summary" and n not in cons.get("E8", ""):
            errs.append(f"{n}: summary tag without an outcome constraint (E8)")
        if t["stratum"] == "ambient" and n not in cadenced:
            errs.append(f"{n}: ambient tag without a cadence invariant")
    ents = {e["name"].lower() for e in v["ontology"]["entities"]}
    fk = [(n, f) for n, t in tags.items() for f in t["types"] if re.search(r"_ids?$", f)]
    unresolved = [f"{n}.{f}" for n, f in fk if FK_ALIAS.get(re.sub(r"_ids?$", "", f)) not in ents]
    if fk and len(unresolved) / len(fk) > 0.30:
        errs.append(f"foreign-key resolution below 70%: {unresolved}")
    print(f"{path}: {len(tags)} tags, {len(cats)} categories, {len(fk)} foreign keys "
          f"({len(fk) - len(unresolved)} resolved), {len(v['operators']['operators'])} operators")
    return errs


if __name__ == "__main__":
    current = "signals/" + json.load(open("signals/current.json"))["version"] + ".json"
    errors = check(sys.argv[1] if len(sys.argv) > 1 else current)
    for e in errors:
        print(e, file=sys.stderr)
    sys.exit(1 if errors else 0)
