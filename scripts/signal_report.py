# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""signal_report.py — a sprint's Signal Report from its records (Sprint 111; SDD hardening review, 2026-10-17).

A sprint's records are those whose RunStarted baseline names a commit whose subject begins "Sprint NNN". The report,
in the kit's template (sdd-kit-3/templates/SIGNAL_REPORT.md), holds: the card's scope; the sprint's commits; the
signal trace (each kind of record counted with substrate's narration_summary, and substrate's narrate of each single
run, frontier and the like, with each event's time from the run's start); and the card's Emits checked tag by tag
against what fired. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's, which it cites.

Usage: uv run python scripts/signal_report.py NNN [--write]
"""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
from collections import Counter
from pathlib import Path

from substrate.api import narrate, narration_summary, read_first_envelope, read_record

ROOT = Path(__file__).resolve().parent.parent
STORE = ROOT / "store" / "runs"
GROUPED = ("class-", "family-")          # many records a sprint: counted, not narrated one by one


def sprint_commits(n: int) -> dict[str, str]:
    """Full sha -> subject of every commit whose subject begins "Sprint NNN"."""
    out = subprocess.run(["git", "log", "--all", "--format=%H%x09%s"], cwd=ROOT, check=True, capture_output=True,
                         text=True).stdout
    return {sha: subject for sha, subject in (line.split("\t", 1) for line in out.splitlines())
            if re.match(rf"Sprint {n:03d}\b", subject)}


def sprint_records(commits: dict[str, str], store: Path = STORE) -> list[Path]:
    found = []
    for d in sorted(store.iterdir()):
        if not d.is_dir():
            continue
        first = read_first_envelope(d)
        commit = (first or {}).get("payload", {}).get("baseline", {}).get("w2m_commit", "")
        if commit in commits:
            found.append(d)
    return found


def card_of(n: int) -> Path:
    return next((ROOT / "sprints").glob(f"sprint-{n:03d}-*.md"))


def section(text: str, heading: str) -> str:
    m = re.search(rf"^{re.escape(heading)}\n(.*?)(?=^#{{2,3}} |\Z)", text, re.S | re.M)
    return m.group(1).strip() if m else ""


def tag_names() -> set[str]:
    version = json.loads((ROOT / "signals" / "current.json").read_text())["version"]
    return {t["name"] for t in json.loads((ROOT / "signals" / f"{version}.json").read_text())["tags"]}


def emits_check(emits: str, fired: Counter[str], tags: set[str]) -> list[str]:
    """Each vocabulary tag the card's Emits names, fired or not; tags that fired unnamed; a "None" Emits with fires."""
    named = [t for t in sorted(tags) if re.search(rf"\b{t}\b", emits)]
    lines = [f"- `{t}`: " + (f"fired {fired[t]:,} time{'s' if fired[t] > 1 else ''}" if fired[t] else "NOT FIRED")
             for t in named]
    unnamed = sorted(t for t in fired if t not in named)
    if unnamed:
        lines.append("- fired, not named in Emits: " + ", ".join(f"`{t}` ({fired[t]})" for t in unnamed))
    if not named and fired:
        lines.append("- the card's Emits names no tag, and the sprint's records fired the ones above")
    if not named and not fired:
        lines.append("- the card's Emits names no tag, and the sprint made no record")
    return lines


def trace(records: list[Path]) -> tuple[list[str], Counter[str]]:
    lines: list[str] = []
    fired: Counter[str] = Counter()
    groups: dict[str, list[Path]] = {}
    for r in records:
        prefix = next((p for p in GROUPED if r.name.startswith(p)), "")
        groups.setdefault(prefix or r.name, []).append(r)
    for name, rs in groups.items():
        summaries = [narration_summary(r) for r in rs]
        events: Counter[str] = Counter()
        for s in summaries:
            events.update(s.application_events)
        fired.update(events)
        failed = sum(s.producers_failed for s in summaries)
        unfinished = sum(not s.finalised for s in summaries)
        lines.append(f"{name}{'* ' + str(len(rs)) + ' records' if name in GROUPED else ''}: "
                     + ", ".join(f"{k} {v}" for k, v in events.items())
                     + f"; producers failed {failed}; not finalised {unfinished}")
        if name in GROUPED:
            continue
        times = {e["seq"]: e["t"] for e in read_record(rs[0])}
        t0 = times.get(0, 0.0)
        repeated: Counter[str] = Counter()
        for line in narrate(rs[0]):
            if line.kind.startswith("substrate."):
                continue
            repeated[line.kind] += 1
            if repeated[line.kind] > 3:                      # MEASUREMENT_RECORDED: the first three, then a count
                continue
            lines.append(f"  t={times[line.seq] - t0:9.3f}  {line.text[:220]}")
        for kind, n in repeated.items():
            if n > 3:
                lines.append(f"  ... {kind} {n - 3} more")
    return lines, fired


def report(n: int, store: Path = STORE) -> str:
    card_path = card_of(n)
    card = card_path.read_text()
    commits = sprint_commits(n)
    records = sprint_records(commits, store)
    lines, fired = trace(records)
    emits = section(card, "### Emits")
    title = card.splitlines()[0].lstrip("# ").strip()
    scope = section(card, "## scope").split("\n\n")[0]
    return "\n".join([
        f"# SIGNAL_REPORT — {title}", "",
        f"Made by `scripts/signal_report.py {n}` from {len(records)} records under store/runs/ whose baseline names "
        f"one of the sprint's {len(commits)} commits. The self-grade and the Rubber Duck observations are the "
        f"BLACKBOARD tail entry's for Sprint {n:03d}.", "",
        "## 1. Observed", "", "### scope_confirmation", "", "> " + scope.replace("\n", "\n> "), "",
        "### work_performed", "", *(f"- {sha[:9]} {subject}" for sha, subject in commits.items()), "",
        "### signal_trace", "", "```", *(lines or ["(no record)"]), "```", "",
        "### signal_trace_tags", "", *(f"- `{t}`" for t in sorted(fired)), "",
        "## 2. Expected", "", "> " + (emits or "(the card has no Emits)").replace("\n", "\n> "), "",
        "## 3. Delta", "", "### emits_check", "", *emits_check(emits, fired, tag_names()), "",
        "## 4. Hypothesis", "", f"See the BLACKBOARD tail entry for Sprint {n:03d}: its Rubber Duck Pass and close.", ""])


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("sprint", type=int)
    ap.add_argument("--write", action="store_true", help="write signal-reports/sprint-NNN-report.md")
    a = ap.parse_args(argv)
    text = report(a.sprint)
    if a.write:
        out = ROOT / "signal-reports" / f"sprint-{a.sprint:03d}-report.md"
        out.write_text(text)
        print(out.relative_to(ROOT))
    else:
        print(text)
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
