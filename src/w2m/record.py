# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""record.py — write one substrate run for a computation that is a single step (Sprint 038; review
2026-10-09 progress S4).

Fifteen scripts each hand-wrote the same harness: SESSION_INIT, the work's events, SESSION_COMPLETED.
`one_step_run` writes that run from a generator of events. It is for computations with no internal
dependency; a computation whose steps cause one another is a topology with triggers instead
(scripts/run_candidate_loop.py, scripts/record_backend_selection.py, scripts/run_frontier.py).
"""

from __future__ import annotations

import asyncio
import subprocess
import time
from collections import Counter
from collections.abc import AsyncIterator, Callable, Iterable
from functools import cache
from pathlib import Path
from typing import Any, Literal, cast

import msgspec
from substrate.api import Producer, Runtime, RunResult, TopologyBuilder, read_record, threshold_count

from w2m import VOCAB_VERSION, events


ROOT = Path(__file__).resolve().parents[2]


def _commit(repo: Path) -> str:
    """HEAD of `repo`, with '-dirty' when tracked files differ from it."""
    head = subprocess.run(["git", "-C", str(repo), "rev-parse", "HEAD"], capture_output=True, text=True)
    if head.returncode:
        return "unknown"
    dirty = bool(_changes(repo))
    return head.stdout.strip() + ("-dirty" if dirty else "")


def _changes(repo: Path) -> list[str]:
    """Tracked files that differ from HEAD, and untracked files under src/ and tools/ (the guest runtime links
    every tools/guest_runtime/*.c, tracked or not; audit 2026-10-13). A failing git is an error, never a clean tree."""
    tracked = subprocess.run(["git", "-C", str(repo), "status", "--porcelain", "--untracked-files=no"],
                             capture_output=True, text=True, check=True).stdout
    untracked = subprocess.run(["git", "-C", str(repo), "ls-files", "--others", "--exclude-standard", "--", "src", "tools"],
                               capture_output=True, text=True, check=True).stdout
    return [line[3:] for line in tracked.splitlines() if line.strip()] + [f for f in untracked.splitlines() if f]


def uncommitted() -> list[str]:
    """Tracked files that differ from HEAD, in w2m and the substrate clone (Sprint 060: a record in store/ names
    the code that ran, so a run that writes there starts from a committed tree)."""
    return [prefix + f for prefix, repo in (("", ROOT), ("substrate/", ROOT / "substrate")) for f in _changes(repo)]


@cache
def provenance() -> dict[str, str]:
    """The code that writes a record (review 2026-10-10 D2): w2m's commit and the substrate clone's.
    SESSION_INIT.harness_version is a semver by the vocabulary, so the commits go in the run's
    baseline, which substrate writes into RunStarted."""
    return {"w2m_commit": _commit(ROOT), "substrate_commit": _commit(ROOT / "substrate")}


def run_with_provenance(root: Path, topology: Callable[[TopologyBuilder], None]) -> RunResult:
    """Run `topology` at `root` with provenance() added to its baseline. Every w2m record is
    written through here (tests/test_record_provenance.py)."""
    def stamped(b: TopologyBuilder) -> None:
        topology(b)
        b.baseline(**provenance())
    return asyncio.run(Runtime(root).run(stamped))


def producer(fn: Callable[[Any], AsyncIterator[msgspec.Struct]]) -> Producer:
    """A w2m producer as substrate's `Producer`. substrate's protocol declares
    `AsyncIterable[substrate.types.Event]`, but Event is the bus frame the kernel builds around what
    a producer yields; producers yield the frozen msgspec Structs named in `producer_kind(schemas=)`
    (substrate 1.1.1 kernel/topology.py: each schema must subclass Struct; kernel/sequencer.py:
    `isinstance(obj, Struct)`). The annotation is substrate's defect; this is the one place w2m
    bridges it."""
    return cast(Producer, fn)


Purpose = Literal["contract_construction", "conformance", "compilation", "capability", "measurement",
                  "application", "product"]
Outcome = Literal["completed", "completed_with_discrepancies", "aborted"]


def one_step_run(root: Path, *, profile: str, contract_version: str, purpose: Purpose,
                 work: Callable[[], Iterable[msgspec.Struct]], baseline: dict[str, Any],
                 outcome: Callable[[list[msgspec.Struct]], Outcome] = lambda evs: "completed") -> tuple[Path, Counter[str]]:
    """Run `work` as the single producer of a session at `root`; return the record and its tag counts."""
    async def harness(_inp: Any) -> AsyncIterator[msgspec.Struct]:
        t0 = time.monotonic()
        yield events.validate(events.SESSION_INIT(
            vocab_version=VOCAB_VERSION, profile=profile, contract_version=contract_version,
            harness_version="0.0.1", purpose=purpose))
        done: list[msgspec.Struct] = []
        for e in work():
            done.append(e)
            yield events.validate(e)
        discrepancies = sum(type(e).__name__ == "DISCREPANCY_REPORTED" for e in done)
        yield events.validate(events.SESSION_COMPLETED(
            outcome=outcome(done), discrepancy_count=discrepancies, accepted_count=0,
            elapsed_seconds=round(time.monotonic() - t0, 3)))

    def topology(b: TopologyBuilder) -> None:
        b.baseline(**baseline)
        b.producer_kind("work", schemas=list(events.HARNESS_EVENTS), schema_version=1,
                        factory=lambda: producer(harness))
        b.initial("work", input=None)
        b.termination(threshold_count("substrate.ProducerCompleted", 1))

    result = run_with_provenance(root, topology)
    kinds = Counter(e["kind"] for e in read_record(result.record_root) if not e["kind"].startswith("substrate."))
    return Path(result.record_root), kinds
