"""run_frontier.py — run an application in w2m until it stops, on the signal plane (Sprint 038).

One substrate run, three steps, each caused by the one before:

  harness   SESSION_INIT (purpose application)
  runner    EXECUTION_STARTED (subject application, profile managed_vm), then the run's end:
            SERVICE_UNIMPLEMENTED when the guest calls a Windows service w2m lacks (vocabulary 0.6,
            P15), else EXECUTION_COMPLETED with its exit status
  closer    SESSION_COMPLETED, triggered by that end

The stop is the frontier: the item the application needs next. Item runs that resolve it and
resume the frontier arrive with the pools (roadmap Sprints 041, 045).

Usage: uv run python scripts/run_frontier.py <exe> [--runs runs] [--max-steps N]
"""

from __future__ import annotations

import argparse
import hashlib
import sys
import time
from collections.abc import AsyncIterator
from pathlib import Path
from typing import Any

import msgspec
from substrate.api import PerEvent, Subscription, TopologyBuilder, read_record, threshold_count

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))
from w2m import VOCAB_VERSION, events  # noqa: E402
from w2m.loader import load  # noqa: E402
from w2m.vm_backend import VM  # noqa: E402
from w2m.record import producer, run_with_provenance  # noqa: E402

EMPTY = "sha256:" + hashlib.sha256(b"").hexdigest()


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("exe", type=Path)
    ap.add_argument("--runs", type=Path, default=ROOT / "runs")
    ap.add_argument("--max-steps", type=int, default=10_000_000)
    a = ap.parse_args(argv)
    data = a.exe.read_bytes()
    sha = "sha256:" + hashlib.sha256(data).hexdigest()
    xid = f"frontier-{sha[7:19]}-{time.time_ns()}"
    t0 = time.monotonic()
    state: dict[str, Any] = {}

    async def harness(_inp: Any) -> AsyncIterator[msgspec.Struct]:
        yield events.validate(events.SESSION_INIT(
            vocab_version=VOCAB_VERSION, profile="w2m-vm", contract_version=VOCAB_VERSION,
            harness_version="0.0.1", purpose="application"))

    async def runner(_inp: Any) -> AsyncIterator[msgspec.Struct]:
        yield events.validate(events.EXECUTION_STARTED(
            execution_id=xid, subject_kind="application", subject_sha256=sha, profile_kind="managed_vm",
            profile_id="w2m-vm", initial_state_sha256=EMPTY, injected_inputs_sha256=EMPTY))
        here = a.exe.resolve().parent                        # the program's directory supplies its DLLs (Sprint 046)
        vm = VM(load(data, lambda n: (here / n).read_bytes() if (here / n).is_file() else None, Path(a.exe).name),
                image_name=Path(a.exe).name, gui=True)     # the game: a GUI program started by Steam
        try:
            r = vm.run(max_steps=a.max_steps, timeout_ms=60_000)
            totals = vm.calls_total()
        finally:
            vm.close()
        state.update(stop=r.stop, instructions=r.instructions_executed, calls=r.calls,
                     host_requests=dict(sorted(vm.host_requests.items())), calls_total=totals)
        if vm.unimplemented is not None:
            name, ret = vm.unimplemented
            yield events.validate(events.SERVICE_UNIMPLEMENTED(
                execution_id=xid, service=name, guest_address=ret, instructions_executed=r.instructions_executed))
            return
        yield events.validate(events.EXECUTION_COMPLETED(
            execution_id=xid, exit_status="exited" if r.exit_code is not None else "faulted",
            trace_sha256=EMPTY, guest_line_count=0,
            **({"exit_code": r.exit_code} if r.exit_code is not None else {}),
            unobserved=f"no guest trace: an application writes none; the run stopped: {r.stop}"))

    async def closer(_inp: Any) -> AsyncIterator[msgspec.Struct]:
        yield events.validate(events.SESSION_COMPLETED(
            outcome="completed", discrepancy_count=0, accepted_count=0,
            elapsed_seconds=round(time.monotonic() - t0, 3)))

    def topology(b: TopologyBuilder) -> None:
        b.baseline(subject=str(a.exe.name), subject_sha256=sha, backend="managed_vm")
        b.producer_kind("harness", schemas=[events.SESSION_INIT], schema_version=1, factory=lambda: producer(harness))
        b.producer_kind("destination_runner", schemas=[events.EXECUTION_STARTED, events.SERVICE_UNIMPLEMENTED,
                        events.EXECUTION_COMPLETED], schema_version=1, factory=lambda: producer(runner))
        b.producer_kind("closer", schemas=[events.SESSION_COMPLETED], schema_version=1, factory=lambda: producer(closer))
        b.initial("harness", input=None)
        b.trigger("run", subscription=Subscription(kinds=frozenset({"SESSION_INIT"})), predicate=lambda c: True,
                  starts="destination_runner", policy=PerEvent(), input_builder=lambda c: None)
        b.trigger("close", subscription=Subscription(kinds=frozenset({"SERVICE_UNIMPLEMENTED", "EXECUTION_COMPLETED"})),
                  predicate=lambda c: True, starts="closer", policy=PerEvent(), input_builder=lambda c: None)
        b.termination(threshold_count("SESSION_COMPLETED", 1))

    result = run_with_provenance(a.runs / xid, topology)
    for e in read_record(result.record_root):
        if not e["kind"].startswith("substrate.") or e["kind"] in ("substrate.ProducerFailed",):
            print(e["kind"], {k: v for k, v in e["payload"].items() if k not in ("vocab_version", "harness_version")})
    print(f"stop: {state.get('stop')}; {state.get('instructions')} instructions; calls {state.get('calls')}")
    print(f"host requests by op {state.get('host_requests')}; calls by service {state.get('calls_total')}")
    print(f"record {result.record_root}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
