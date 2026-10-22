# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""golden.py — the Python oracle's verdicts as data, and a check of a commit's records against them (Sprint 123).

Step 0 of docs/rust-port-plan-2026-10-21.md. `--write` reads the records under store/runs/ whose RunStarted baseline names the
golden commit (4b36b5ad, code identical to f2d071af) and writes specs/golden/python-final.json: per class run, per
family run (keyed by family and vCPU count) and per batch run, the verdict each record states. A batch run's guest
trace is held by the sha256 of its body, lines 2 on: line 1, GUEST_TRACE_OPENED, names the run (BLACKBOARD Decision
2026-10-22). `--check COMMIT` reads the records COMMIT made and reports each key whose verdict differs from the golden
file, or that either side lacks; exit 1 on any.

Usage: uv run python scripts/golden.py --write
       uv run python scripts/golden.py --check COMMIT [--golden PATH]
"""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
import sys
from pathlib import Path
from typing import Any

from substrate.api import read_first_envelope, read_record

ROOT = Path(__file__).resolve().parent.parent
STORE = ROOT / "store" / "runs"
TRACES = ROOT / "store" / "traces"
GOLDEN = ROOT / "specs" / "golden" / "python-final.json"
GOLDEN_COMMIT = "4b36b5ad"


def full_sha(commit: str) -> str:
    return subprocess.check_output(["git", "-C", str(ROOT), "rev-parse", commit], text=True).strip()


def trace_body_sha256(trace_sha256: str) -> str:
    """The sha256 of a stored guest trace's lines 2 on (line 1 names the run)."""
    path = TRACES / f"{trace_sha256.removeprefix('sha256:')}.jsonl"
    with path.open("rb") as f:
        f.readline()
        return hashlib.sha256(f.read()).hexdigest()


def verdict(run: Path, baseline: dict[str, Any]) -> tuple[str, dict[str, Any]] | None:
    """A record's key and the verdict it states, or None for a record that is not a class, family or batch run."""
    kind = run.name.split("-", 1)[0]
    events = [e for e in read_record(run) if not e["kind"].startswith("substrate.")]
    by_kind: dict[str, list[dict[str, Any]]] = {}
    for e in events:
        by_kind.setdefault(e["kind"], []).append(e["payload"])
    # A comparison is named by what it compared against (model, managed_vm, a reference run), not by the candidate's
    # id, which hashes w2m's code and changes with any change to it (measured: every key at f1838b96 against 4b36b5ad).
    candidate = (by_kind.get("CANDIDATE_GENERATED") or [{}])[0].get("candidate_id")
    prefix = f"cmp-{candidate}-" if candidate else ""
    comparisons = [
        {
            "comparison": p["comparison_id"].removeprefix(prefix),
            "relation": p.get("relation"),
            "first_divergence": p["first_divergence"],
        }
        for p in by_kind.get("COMPARISON_COMPLETED", [])
    ]
    done = (by_kind.get("SESSION_COMPLETED") or [{}])[-1]
    out: dict[str, Any] = {
        "outcome": done.get("outcome"),
        "accepted": done.get("accepted_count"),
        "discrepancies": done.get("discrepancy_count"),
        "comparisons": comparisons,
    }
    if kind in ("class", "family"):
        generated = (by_kind.get("CANDIDATE_GENERATED") or [{}])[0]
        out["contract"] = f"{generated.get('contract_id')}@{generated.get('contract_version')}"
    if kind == "class":
        out["cases"] = baseline.get("cases")
        return f"class:{baseline['iclass']}", out
    if kind == "family":
        return f"family:{baseline['family']}@{baseline.get('vcpus', 1)}", out
    if kind == "frontier":
        completed = by_kind["EXECUTION_COMPLETED"][0]
        ingested = by_kind["TRACE_INGESTED"][0]
        out.update(
            exit_status=completed["exit_status"],
            lines=completed["guest_line_count"],
            threads=ingested["thread_count"],
            trace_body_sha256=trace_body_sha256(ingested["trace_sha256"]),
        )
        for c in out["comparisons"]:
            c["comparison"] = "cmp-frontier"            # the id names the run
        return f"frontier:{baseline['subject']}", out
    return None


def verdicts(commit: str) -> dict[str, dict[str, Any]]:
    sha = full_sha(commit)
    found: dict[str, dict[str, Any]] = {}
    for run in sorted(STORE.iterdir()):
        if not run.is_dir():
            continue
        first = read_first_envelope(run)
        baseline = (first or {}).get("payload", {}).get("baseline", {})
        if baseline.get("w2m_commit") != sha:
            continue
        got = verdict(run, baseline)
        if got is None:
            continue
        key, value = got
        if key in found:
            raise SystemExit(f"two records for {key} at {commit}: {run}")
        found[key] = value
    return found


def write() -> int:
    found = verdicts(GOLDEN_COMMIT)
    GOLDEN.parent.mkdir(parents=True, exist_ok=True)
    doc = {"commit": full_sha(GOLDEN_COMMIT), "tag": "python-final", "verdicts": dict(sorted(found.items()))}
    GOLDEN.write_text(json.dumps(doc, indent=1, sort_keys=True) + "\n")
    kinds = [k.split(":", 1)[0] for k in found]
    print(f"{GOLDEN.relative_to(ROOT)}: {len(found)} keys ({', '.join(f'{kinds.count(k)} {k}' for k in sorted(set(kinds)))})")
    return 0


def check(commit: str, golden: Path) -> int:
    want = json.loads(golden.read_text())["verdicts"]
    got = verdicts(commit)
    differ = sorted(k for k in want.keys() | got.keys() if want.get(k) != got.get(k))
    for k in differ:
        side = "missing at " + commit if k not in got else "not golden" if k not in want else "differs"
        print(f"{k}: {side}")
    print(f"{commit}: {len(got)} keys, golden {len(want)}, {len(differ)} differ")
    return 1 if differ else 0


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--write", action="store_true")
    ap.add_argument("--check", metavar="COMMIT")
    ap.add_argument("--golden", type=Path, default=GOLDEN)
    a = ap.parse_args()
    if a.write:
        return write()
    if a.check:
        return check(a.check, a.golden)
    ap.error("--write or --check COMMIT")
    return 2


if __name__ == "__main__":
    sys.exit(main())
