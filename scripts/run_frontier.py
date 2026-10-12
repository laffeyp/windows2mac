# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""run_frontier.py — run an application in w2m until it stops, on the signal plane (Sprint 038).

One substrate run, five steps, each caused by the one before:

  harness     SESSION_INIT (purpose application)
  runner      EXECUTION_STARTED (subject application, profile managed_vm), then the run's end:
              SERVICE_UNIMPLEMENTED when the guest calls a Windows service w2m lacks (vocabulary 0.6,
              P15), else EXECUTION_COMPLETED with its exit status and its guest trace
  ingester    TRACE_INGESTED (or TRACE_REJECTED) for the run's guest trace: the calls UnityPlayer.dll and
              mono-2.0-bdwgc.dll made (call_trace.c; Sprint 060), in the content store
  comparator  COMPARISON_COMPLETED: those calls against the game's on Windows (scripts/call_compare.py; contract
              windows.game.calls), reference the game run's execution (scripts/record_game_run.py)
  closer      SESSION_COMPLETED, triggered by the comparison or a rejected trace

The stop is the frontier: the item the application needs next. Item runs that resolve it and
resume the frontier arrive with the pools (roadmap Sprints 041, 045).

The game is given the batch run's arguments (Sprint 054; .github/workflows/windows-game.yml, mode batch): the
reference trace and unity.log of milestone A come from that run, so the frontier takes its path. The log the game
writes to C:\\w2m\\unity.log is kept at store/frontier-logs/<execution id>/unity.log.

The record goes to store/runs/ and names the commit that ran (Sprint 060): a run from a tree with uncommitted
changes is refused there. --scratch writes to runs/ (not committed) and says so in the record's baseline.

Usage: uv run python scripts/run_frontier.py <exe> [--scratch] [--max-steps N]
"""

from __future__ import annotations

import argparse
import hashlib
import json
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
from w2m.file_bridge import FileBridge  # noqa: E402
from w2m.kernel import Machine, spawner  # noqa: E402
from w2m.loader import load  # noqa: E402
from w2m.trace import trace_ingester  # noqa: E402
from w2m.vm_backend import TRACED_IMAGES, USER_ENVIRONMENT, VM  # noqa: E402
from w2m.record import producer, run_with_provenance, uncommitted  # noqa: E402

import call_compare  # noqa: E402
import record_game_run  # noqa: E402

SIGNATURES = json.loads(call_compare.SIGNATURES.read_text())["functions"]

EMPTY = "sha256:" + hashlib.sha256(b"").hexdigest()
BATCH_ARGUMENTS = '-batchmode -nographics -logFile "C:\\w2m\\unity.log"'   # the runner's batch mode, the log on C:
LOGS = ROOT / "store" / "frontier-logs"
REFERENCE = ROOT / "store" / "game-runs" / "38206263976"     # the batch run: C:\\w2m\\app, the game's resources (Sprint 061)
CONTRACT = ("windows.game.calls", "0.1.0")
# Planted faults the comparison must catch at the call they change (Sprint 060's observation contract): file, the
# exact text, its replacement, and the (function, field) the divergence names.
PLANTS = {
    "thread_priority_off_by_one": ("thread_queries.c",
                                   "(u32)w2m_thread_priority(tid));", "(u32)(w2m_thread_priority(tid) + 1));",
                                   ("GetThreadPriority", "ret")),
    "envvar_error_dropped": ("environment.c",
                             "        set_error(ERROR_ENVVAR_NOT_FOUND);\n        return w2m_logged(W2M_API_GetEnvironmentVariableW",
                             "        return w2m_logged(W2M_API_GetEnvironmentVariableW",
                             ("GetEnvironmentVariableW", "error"))}


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("exe", type=Path)
    ap.add_argument("--scratch", action="store_true", help="write to runs/, not store/runs/; a dirty tree allowed")
    ap.add_argument("--plant", choices=sorted(PLANTS), help="run with a planted fault (recorded in the baseline)")
    ap.add_argument("--max-steps", type=int, default=1_000_000_000)    # a run goes to its stop (Sprint 050: 20.6 M)
    a = ap.parse_args(argv)
    sources = None
    if a.plant:
        name, old, new, _ = PLANTS[a.plant]
        text = (ROOT / "tools" / "guest_runtime" / name).read_text()
        if text.count(old) != 1:
            print(f"plant {a.plant}: its anchor is not once in {name}")
            return 2
        sources = {name: text.replace(old, new)}
    base = ROOT / ("runs" if a.scratch else "store")     # traces/ and frontier-calls/ beside the records
    runs = ROOT / "runs" if a.scratch else base / "runs"
    changed = uncommitted()
    if changed and not a.scratch:
        print("refused: store/runs/ records name the commit that ran; uncommitted: " + ", ".join(changed[:10]))
        return 2
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
        # the program's directory is C:\\w2m\\app and the working directory, read-only (Sprint 048)
        files = FileBridge(here, read_only=True)
        vm = VM(load(data, lambda n: (here / n).read_bytes() if (here / n).is_file() else None, Path(a.exe).name),
                image_name=Path(a.exe).name, gui=True, files=files, environment=USER_ENVIRONMENT,     # a GUI program started by Steam
                command_line=f'"C:\\w2m\\app\\{Path(a.exe).name}" {BATCH_ARGUMENTS}', call_trace=TRACED_IMAGES,
                call_trace_from="UnityPlayer.dll!UnityMain", runtime_sources=sources)       # where jit_capture.c opens the Windows trace
        # a kernel.Machine: the game starts UnityCrashHandler64.exe as a second process (Sprint 055)
        machine = Machine(spawner(files), max_steps=a.max_steps, timeout_ms=60_000)
        try:
            r = machine.run(vm)
            calls, dropped = vm.traced_calls()
            threads = {t.tid: (t.parent, t.start, t.created_suspended) for t in vm.threads.values()}
            text = call_compare.guest_trace(xid, calls, threads, dropped, SIGNATURES)
            trace_sha = "sha256:" + hashlib.sha256(text).hexdigest()
            (base / "traces").mkdir(parents=True, exist_ok=True)
            (base / "traces" / f"{trace_sha[7:]}.jsonl").write_bytes(text)
            (base / "frontier-calls").mkdir(parents=True, exist_ok=True)
            (base / "frontier-calls" / f"{xid}.tsv").write_text(call_compare.write_tsv(calls))
            state.update(trace=trace_sha, trace_lines=text.count(b"\n"), traced=len(calls), dropped=dropped)
            totals = vm.calls_total()
            state["faults"] = vm.faults()
            state["processes"] = {p.pid: (p.vm.modules.image_name, p.exit_code,
                                          dict(sorted(p.vm.calls_total().items())),
                                          {t.tid: (t.status, p.vm.ld.thunks.get(t.x[17], hex(t.x[17])) if t.x else "running")
                                           for t in p.vm.threads.values()})
                                  for p in machine.processes.values() if p.vm is not vm}
        finally:
            for other in machine.processes.values():
                if other.vm is not vm:
                    other.vm.close()
                    other.vm.files.close_all()
            vm.close()
            files.close_all()
            log = files.drive / "w2m" / "unity.log"
            if log.is_file():
                (LOGS / xid).mkdir(parents=True, exist_ok=True)
                (LOGS / xid / "unity.log").write_bytes(log.read_bytes())
        state.update(stop=r.stop, instructions=r.instructions_executed, calls=r.calls,
                     host_requests=dict(sorted(vm.host_requests.items())), calls_total=totals)
        if vm.unimplemented is not None:
            name, ret = vm.unimplemented
            yield events.validate(events.SERVICE_UNIMPLEMENTED(
                execution_id=xid, service=name, guest_address=ret, instructions_executed=r.instructions_executed))
            return
        yield events.validate(events.EXECUTION_COMPLETED(
            execution_id=xid, exit_status="exited" if r.exit_code is not None else "faulted",
            trace_sha256=state["trace"], guest_line_count=state["trace_lines"],
            **({"exit_code": r.exit_code} if r.exit_code is not None else {}),
            unobserved=f"the guest trace holds the traced images' calls only; the run stopped: {r.stop}"))

    async def comparator(_inp: Any) -> AsyncIterator[msgspec.Struct]:
        windows = call_compare.read_tsv(REFERENCE / "jit_capture.bin.trace.tsv")
        found, compared = call_compare.compare(windows, call_compare.read_tsv(base / "frontier-calls" / f"{xid}.tsv"),
                                               SIGNATURES, call_compare.excluded(
                                                   json.loads(call_compare.CONTRACT.read_text()), REFERENCE.name))
        state.update(divergences=found, compared=compared)
        yield events.validate(events.COMPARISON_COMPLETED(
            comparison_id=f"cmp-{xid}", contract_id=CONTRACT[0], contract_version=CONTRACT[1],
            relation="state_relation", reference_ref=record_game_run.execution_id(REFERENCE),
            target_execution_id=xid, verdict="discrepancy" if found else "conforms",
            **({"first_divergence": found[0].text()[:500]} if found else {})))

    async def closer(_inp: Any) -> AsyncIterator[msgspec.Struct]:
        found = state.get("divergences", [])
        yield events.validate(events.SESSION_COMPLETED(
            outcome="completed_with_discrepancies" if found else "completed", discrepancy_count=len(found),
            accepted_count=0,
            elapsed_seconds=round(time.monotonic() - t0, 3)))

    def topology(b: TopologyBuilder) -> None:
        b.baseline(subject=str(a.exe.name), subject_sha256=sha, backend="managed_vm", scratch=a.scratch,
                   plant=a.plant or "")
        b.producer_kind("harness", schemas=[events.SESSION_INIT], schema_version=1, factory=lambda: producer(harness))
        b.producer_kind("destination_runner", schemas=[events.EXECUTION_STARTED, events.SERVICE_UNIMPLEMENTED,
                        events.EXECUTION_COMPLETED], schema_version=1, factory=lambda: producer(runner))
        b.producer_kind("ingester", schemas=[events.TRACE_INGESTED, events.TRACE_REJECTED], schema_version=1,
                        factory=lambda: producer(trace_ingester(base)))
        b.producer_kind("comparator", schemas=[events.COMPARISON_COMPLETED], schema_version=1,
                        factory=lambda: producer(comparator))
        b.producer_kind("closer", schemas=[events.SESSION_COMPLETED], schema_version=1, factory=lambda: producer(closer))
        b.initial("harness", input=None)
        b.trigger("run", subscription=Subscription(kinds=frozenset({"SESSION_INIT"})), predicate=lambda c: True,
                  starts="destination_runner", policy=PerEvent(), input_builder=lambda c: None)
        b.trigger("ingest", subscription=Subscription(kinds=frozenset({"SERVICE_UNIMPLEMENTED", "EXECUTION_COMPLETED"})),
                  predicate=lambda c: True, starts="ingester", policy=PerEvent(),
                  input_builder=lambda c: {"trace_sha256": state["trace"], "execution_id": xid})
        b.trigger("compare", subscription=Subscription(kinds=frozenset({"TRACE_INGESTED"})),
                  predicate=lambda c: True, starts="comparator", policy=PerEvent(), input_builder=lambda c: None)
        b.trigger("close", subscription=Subscription(kinds=frozenset({"COMPARISON_COMPLETED", "TRACE_REJECTED"})),
                  predicate=lambda c: True, starts="closer", policy=PerEvent(), input_builder=lambda c: None)
        b.termination(threshold_count("SESSION_COMPLETED", 1))

    result = run_with_provenance(runs / xid, topology)
    for e in read_record(result.record_root):
        if not e["kind"].startswith("substrate.") or e["kind"] in ("substrate.ProducerFailed",):
            print(e["kind"], {k: v for k, v in e["payload"].items() if k not in ("vocab_version", "harness_version")})
    print(f"stop: {state.get('stop')}; {state.get('instructions')} instructions; calls {state.get('calls')}")
    print(f"host requests by op {state.get('host_requests')}; calls by service {state.get('calls_total')}")
    count, first = state.get("faults", (0, []))
    print(f"faults dispatched {count}: " + "; ".join(f"0x{c:x} at 0x{rip:x} ({i0}, 0x{i1:x})" for c, rip, i0, i1 in first))
    for pid, (name, code, calls, threads) in state.get("processes", {}).items():
        print(f"process {pid} {name}: exit code {code}; threads {threads}; calls by service {calls}")
    print(f"traced calls {state.get('traced')} (left out {state.get('dropped')}); guest trace {state.get('trace')}")
    found = state.get("divergences", [])
    print(f"compared {state.get('compared')} calls with run {REFERENCE.name}: {len(found)} divergences")
    for d in found[:60]:
        print(f"  seq {d.windows_seq}: {d.text()}")
    if a.plant:
        fn, field = PLANTS[a.plant][3]
        hit = [d for d in found if d.function == fn and d.field == field]
        print(f"plant {a.plant}: " + (f"caught at {hit[0].text()}" if hit else f"NOT caught ({fn} {field})"))
    print(f"record {result.record_root}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
