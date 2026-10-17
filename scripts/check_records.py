# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""check_records.py — every record checked against the vocabulary's rules (Sprint 109; SDD hardening review,
2026-10-17, item 4).

check_replay.py proves each record's stream intact (substrate level 2); this reads each record's application events
(through substrate's read_record) and checks what the vocabulary declares of them: T1, T2, T3, T4 and T7
(`temporal_invariants`), S1 to S13 where their tags occur (`state_transitions`; a rule whose payload match is prose
is checked for order alone, and its count says so), and E1 and E6 (`evidence_constraints`). T5, T6 and S14 are the
guest trace's, enforced at ingest by src/w2m/trace.py.

Prints, per rule, the records it applied to, its violations and what it could not check. With --since YYYY-MM-DD,
exit 1 on a violation in a record made that day or later (a record's time is its directory's nanosecond suffix);
older records are reported, never rewritten.

Usage: uv run python scripts/check_records.py [--since 2026-10-17] [--show N] [record ...]
"""

from __future__ import annotations

import argparse
import datetime
import json
import re
import sys
from collections import Counter
from collections.abc import Callable
from pathlib import Path
from typing import Any

from substrate.api import read_record

ROOT = Path(__file__).resolve().parent.parent
STORE = ROOT / "store" / "runs"
SHA256 = re.compile(r"^sha256:[0-9a-f]{64}$")
PLACEHOLDER = "sha256:" + "0" * 64
# violations known, with their cause, which the gate reports and does not hold: (rule, record name prefix) -> where
# the cause and its fix are recorded. Each is a defect with a sprint to come, never a rule waived.
KNOWN = {(rule, prefix): f"BLACKBOARD Deferred 2026-10-17 (Sprint 109): class and family runs report {what}"
         for rule, what in (("S5", "with no reduction"), ("E1", "zero program and state hashes"))
         for prefix in ("class-", "family-")}
Event = dict[str, Any]


def vocabulary() -> dict[str, Any]:
    version = json.loads((ROOT / "signals" / "current.json").read_text())["version"]
    return dict(json.loads((ROOT / "signals" / f"{version}.json").read_text()))


def plain_fields(match: list[str]) -> tuple[list[str], dict[str, str]] | None:
    """A rule's payload match as the fields both events share and `field=value` filters on the `from` event (S4's
    verdict=discrepancy); None when any entry is prose (checked for order alone)."""
    if not all(re.fullmatch(r"[a-z_]+(=[a-z_]+)?", m) for m in match):
        return None
    return [m for m in match if "=" not in m], dict(m.split("=") for m in match if "=" in m)


def key(e: Event, fields: list[str]) -> tuple[Any, ...]:
    return tuple(e["payload"].get(f) for f in fields)


class Checker:
    """The rules, read once from the vocabulary; `check(events)` gives (rule, problem) pairs and the rules that
    applied but could not be checked."""

    def __init__(self, vocab: dict[str, Any]) -> None:
        self.temporal = {r["id"]: r for r in vocab["temporal_invariants"]["invariants"]}
        self.states = [r for r in vocab["state_transitions"]["rules"] if not r["from_tag"].startswith("GUEST_")]
        self.sha_fields = {t["name"]: [f for f, ty in t["types"].items() if ty == "sha256"] for t in vocab["tags"]}

    def check(self, events: list[Event]) -> tuple[list[tuple[str, str]], Counter[str], Counter[str]]:
        found: list[tuple[str, str]] = []
        applied: Counter[str] = Counter()
        unchecked: Counter[str] = Counter()
        kinds = [e["kind"] for e in events]
        if not events:
            return found, applied, unchecked
        applied["T1"] += 1
        if kinds[0] != "SESSION_INIT":
            found.append(("T1", f"first event {kinds[0]}"))
        applied["T2"] += 1
        if kinds[-1] != "SESSION_COMPLETED":
            found.append(("T2", f"last event {kinds[-1]}"))
        self._t3(events, found, applied, unchecked)
        self._t4(events, found, applied)
        self._after("T7", events, "CONTRACT_REVISED", {"DEPENDENTS_INVALIDATED"}, ["contract_id", "to_version"],
                    found, applied)
        for rule in self.states:
            self._state(rule, events, found, applied, unchecked)
        for e in events:                                         # E1: every sha256-typed field
            for f in self.sha_fields.get(e["kind"], []):
                v = e["payload"].get(f)
                if v is not None:
                    applied["E1"] += 1
                    if not isinstance(v, str) or not SHA256.match(v):
                        found.append(("E1", f"{e['kind']}.{f} = {v!r}"))
                    elif v == PLACEHOLDER:                         # the form holds, the value says nothing
                        found.append(("E1", f"{e['kind']}.{f} is the zero placeholder"))
            if e["kind"] == "COMPARISON_COMPLETED" and e["payload"].get("verdict") == "discrepancy":
                applied["E6"] += 1
                if not e["payload"].get("first_divergence"):
                    found.append(("E6", f"{e['payload'].get('comparison_id')}: a discrepancy with no first_divergence"))
        return found, applied, unchecked

    def _t3(self, events: list[Event], found: list[tuple[str, str]], applied: Counter[str],
            unchecked: Counter[str]) -> None:
        ends = [e for e in events if e["kind"] in ("EXECUTION_COMPLETED", "EXECUTION_FAILED")]
        for s in (e for e in events if e["kind"] == "EXECUTION_STARTED"):
            xid = s["payload"]["execution_id"]
            closing = [e for e in ends if e["payload"].get("execution_id") == xid and e["seq"] > s["seq"]]
            applied["T3"] += 1
            if len(closing) != 1:
                found.append(("T3", f"{xid}: {len(closing)} completions"))
                continue
            limit = s["payload"].get("timeout_seconds")
            if limit is None:
                unchecked["T3"] += 1                             # T3 itself: not checked with no timeout_seconds
                continue
            within = limit + (s["payload"].get("kill_grace_seconds") or 0)
            if closing[0]["t"] - s["t"] > within:
                found.append(("T3", f"{xid}: closed after {closing[0]['t'] - s['t']:.1f} s, past {within} s"))

    def _t4(self, events: list[Event], found: list[tuple[str, str]], applied: Counter[str]) -> None:
        end = next((e["seq"] for e in events if e["kind"] == "SESSION_COMPLETED"), None)
        for c in (e for e in events if e["kind"] == "EXECUTION_COMPLETED"):
            applied["T4"] += 1
            sha = c["payload"].get("trace_sha256")
            follow = [e for e in events if e["kind"] in ("TRACE_INGESTED", "TRACE_REJECTED") and e["seq"] > c["seq"]
                      and e["payload"].get("trace_sha256") == sha and (end is None or e["seq"] < end)]
            if len(follow) != 1:
                found.append(("T4", f"{c['payload'].get('execution_id')}: {len(follow)} ingests of {sha}"))

    def _after(self, rule: str, events: list[Event], first: str, then: set[str], fields: list[str],
               found: list[tuple[str, str]], applied: Counter[str]) -> None:
        """Every `first` is followed by one of `then` with the same `fields`."""
        for a in (e for e in events if e["kind"] == first):
            applied[rule] += 1
            if not any(e["kind"] in then and e["seq"] > a["seq"] and key(e, fields) == key(a, fields) for e in events):
                found.append((rule, f"{first} {key(a, fields)} with no {'/'.join(sorted(then))} after it"))

    def _state(self, rule: dict[str, Any], events: list[Event], found: list[tuple[str, str]],
               applied: Counter[str], unchecked: Counter[str]) -> None:
        """S1 to S13 by kind. pairing_ordering and allowed_set: each `to` has a `from` before it with the same
        fields; forbidden_after: no forbidden `to` after a `from` with the same fields; forced_next: the next event
        with the same fields after a `from` is an allowed one."""
        rid, src = rule["id"], rule["from_tag"]
        parsed = plain_fields(rule.get("payload_match", []))
        fields, wanted = parsed if parsed is not None else (None, {})
        allowed = set(rule.get("to_tags_allowed", [])) | set(rule.get("to_tags_forbidden", []))
        if not any(e["kind"] in allowed for e in events):    # nothing that must follow, or must not
            return
        applied[rid] += 1
        if fields is None:
            unchecked[rid] += 1                                  # prose: order alone below

        def source(e: Event) -> bool:
            return e["kind"] == src and all(str(e["payload"].get(f)) == v for f, v in wanted.items())
        same: Callable[[Event, Event], bool] = (lambda a, b: True) if fields is None else (
            lambda a, b: key(a, fields) == key(b, fields))
        kind = rule["kind"]
        if kind in ("pairing_ordering", "allowed_set"):
            for b in (e for e in events if e["kind"] in rule["to_tags_allowed"]):
                if not any(source(a) and a["seq"] < b["seq"] and same(a, b) for a in events):
                    found.append((rid, f"{b['kind']} with no {src} before it"))
        elif kind == "forbidden_after":
            for a in (e for e in events if source(e)):
                for b in events:
                    if b["kind"] in rule["to_tags_forbidden"] and b["seq"] > a["seq"] and same(a, b):
                        found.append((rid, f"{b['kind']} after {src}"))
        elif kind == "forced_next":
            for a in (e for e in events if source(e)):
                nxt = next((b for b in events if b["seq"] > a["seq"] and b["kind"] != src and same(a, b)), None)
                if nxt is None or nxt["kind"] not in rule["to_tags_allowed"]:
                    found.append((rid, f"{src} followed by {nxt['kind'] if nxt else 'nothing'}"))


def application_events(record: Path) -> list[Event]:
    return [e for e in read_record(record) if not e["kind"].startswith("substrate.")]


def made_on(record: Path) -> datetime.date | None:
    m = re.search(r"-(\d{19})$", record.name)
    return datetime.datetime.fromtimestamp(int(m.group(1)) / 1e9, datetime.UTC).date() if m else None


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("records", nargs="*", type=Path)
    ap.add_argument("--since", type=datetime.date.fromisoformat)
    ap.add_argument("--show", type=int, default=3, help="examples printed per rule")
    a = ap.parse_args(argv)
    checker = Checker(vocabulary())
    records = a.records or sorted(d for d in STORE.iterdir() if d.is_dir())
    violations: Counter[str] = Counter()
    applied: Counter[str] = Counter()
    unchecked: Counter[str] = Counter()
    examples: dict[str, list[str]] = {}
    held: list[str] = []
    known: Counter[str] = Counter()
    for r in records:
        found, used, skipped = checker.check(application_events(r))
        applied.update(used)
        unchecked.update(skipped)
        for rule, problem in found:
            violations[rule] += 1
            examples.setdefault(rule, []).append(f"{r.name}: {problem}")
            day = made_on(r)
            if a.since is None or day is None or day < a.since:
                continue
            cause = next((c for (k, prefix), c in KNOWN.items() if k == rule and r.name.startswith(prefix)), None)
            if cause is not None:
                known[cause] += 1
            else:
                held.append(f"{rule} {r.name}: {problem}")
    for rule in sorted(applied, key=lambda k: (k[0], int(k[1:]))):
        print(f"{rule:4} applied {applied[rule]:>7}  violations {violations[rule]:>6}  unchecked {unchecked[rule]:>6}")
        for ex in examples.get(rule, [])[:a.show]:
            print(f"       {ex}")
    print(f"{len(records)} records; {sum(violations.values())} violations"
          + (f"; in records made on or after {a.since}: {len(held)} held, {sum(known.values())} known"
             if a.since else ""))
    for cause, n in sorted(known.items()):
        print(f"  known, not held ({n}): {cause}")
    for h in held[:20]:
        print(f"  held: {h}")
    return 1 if held else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
