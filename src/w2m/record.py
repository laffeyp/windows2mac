"""record.py — write one substrate run for a computation that is a single step (Sprint 038; review
2026-10-09 progress S4).

Fifteen scripts each hand-wrote the same harness: SESSION_INIT, the work's events, SESSION_COMPLETED.
`one_step_run` writes that run from a generator of events. It is for computations with no internal
dependency; a computation whose steps cause one another is a topology with triggers instead
(scripts/run_candidate_loop.py, scripts/record_backend_selection.py, scripts/run_frontier.py).
"""

from __future__ import annotations

import asyncio
import time
from collections import Counter
from collections.abc import AsyncIterator, Callable, Iterable
from pathlib import Path
from typing import Any, Literal, cast

import msgspec
from substrate.api import Producer, Runtime, TopologyBuilder, read_record, threshold_count

from w2m import VOCAB_VERSION, events


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

    result = asyncio.run(Runtime(root).run(topology))
    kinds = Counter(e["kind"] for e in read_record(result.record_root) if not e["kind"].startswith("substrate."))
    return Path(result.record_root), kinds
