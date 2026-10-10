# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""run_candidate_loop.py — generate, run, judge, accept or reject, report, route, repair (Sprint 031).

One substrate run of the v0.2 §11.2-11.4 loop for one contract and one probe:

  harness             SESSION_INIT ... SESSION_COMPLETED
  candidate_generator CANDIDATE_GENERATED     (a Responder writes the contract's functions)
  destination_runner  EXECUTION_STARTED, EXECUTION_COMPLETED | EXECUTION_FAILED
                      (the probe runs N times with the candidate: C compiled into the guest
                      runtime image in the VM, or a host build of it behind kernel32.py in the
                      model; Sprint 035)
  trace_ingester      TRACE_INGESTED
  comparator          COMPARISON_COMPLETED    (src/w2m/compare.py, unchanged)
  acceptance_guard    CANDIDATE_ACCEPTED | CANDIDATE_REJECTED, ACCEPTANCE_CONDITION_ALTERED
  reducer             REDUCTION_COMPLETED, DISCREPANCY_REPORTED, DISCREPANCY_ROUTED,
                      DISCREPANCY_RESOLVED

The guard hashes the acceptance condition (contract file, compare.py, contract.py) when the
session starts and again after each candidate's executions; the generator never sees the
hashes and cannot change what the guard compares against. A rejection with a discrepancy is
reported and routed; a rejection without one (refused code, inconclusive runs, an altered
acceptance condition) feeds its reason back. Either starts the next attempt until --attempts.

Each contract version is judged on its own probe (PROBE_SHA256): 0.1.0 on the Sprint 029
skeleton, 0.2.0 on probes/last_error_values (Sprint 032), whose values are chosen at run time.

Usage:
    uv run python scripts/run_candidate_loop.py --model qwen2.5-coder:7b
        [--contract windows.thread.last_error.set_get@0.2.0] [--backend managed_vm|model]
        [--runs-per-candidate 3] [--attempts 3] [--first-candidate FILE]

--first-candidate plants attempt 1 (for example a known-wrong implementation) so the model's
first reply is a repair from a structured discrepancy; generator_ref records the plant.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import sys
import time
from collections.abc import AsyncIterator, Callable
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Protocol, cast, get_args

import msgspec
from substrate.adapters import ModelUsage, call_responder_metered
from substrate.api import (
    PerEvent, Subscription, TopologyBuilder, read_record, threshold_count,
)

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))
from run_skeleton import wrap  # noqa: E402

from w2m import VOCAB_VERSION, events  # noqa: E402
from w2m.candidate import CandidateError, admit, extract_code, install, prompt  # noqa: E402
from w2m.compare import compare, comparator  # noqa: E402
from w2m.contract import load_contract  # noqa: E402
from w2m.loader import load  # noqa: E402
from w2m.runtime import Result, Runtime as ModelRuntime  # noqa: E402
from w2m.trace import trace_ingester, trace_path  # noqa: E402
from w2m.record import producer, run_with_provenance  # noqa: E402

EMPTY = "sha256:" + hashlib.sha256(b"").hexdigest()
ACCEPTANCE_FILES = (ROOT / "src" / "w2m" / "compare.py", ROOT / "src" / "w2m" / "contract.py")
NAMES = ("SetLastError", "GetLastError")
REFUSED = "candidate refused by admission check: "
TIMEOUT_SECONDS = 10
PROBE_SHA256 = {                    # contract@version -> the probe binary it is judged on
    "windows.thread.last_error.set_get@0.1.0":
        "sha256:18f721e66de4b57e59330135d7c137c03890c94180455210cf979b8ae35074c8",
    "windows.thread.last_error.set_get@0.2.0":
        "sha256:d210df33adce2f0807fefba2e572cc48bddf9649b8b2474658c92fb46f7b1210",
}


class Responder(Protocol):
    def respond(self, prompt: str) -> str: ...


class PlantedFirst:
    """Answers the first prompt with a fixed candidate, then defers to `rest`."""

    def __init__(self, first: str, rest: Responder) -> None:
        self.first: str | None = first
        self.rest = rest

    def respond(self, prompt: str) -> str:
        if self.first is not None:
            reply, self.first = self.first, None
            return reply
        return self.rest.respond(prompt)

    async def arespond_metered(self, prompt: str) -> tuple[str, ModelUsage]:
        """Planted reply: marked as such; later replies: the model's real usage."""
        if self.first is not None:
            reply, self.first = self.first, None
            return reply, ModelUsage(model="planted", prompt_tokens=0, completion_tokens=0,
                                     wall_ms=0, estimated=True)
        return await call_responder_metered(self.rest, prompt)


def sha256_file(p: Path) -> str:
    return "sha256:" + hashlib.sha256(p.read_bytes()).hexdigest()


@dataclass
class Session:
    """State shared by the operators of one run (each operator writes only its own fields)."""
    contract_path: Path
    probe: bytes
    probe_sha: str
    backend: events.EnumProfileKind
    profile_id: str
    runs: int
    attempts: int
    acceptance_baseline: dict[str, str]
    generator_ref: str
    sources: dict[str, str] = field(default_factory=dict)          # candidate_id -> source
    results: dict[str, list[dict[str, Any]]] = field(default_factory=dict)
    roles: dict[str, dict[str, int]] = field(default_factory=dict)  # execution_id -> role map
    open_discrepancies: list[str] = field(default_factory=list)
    reported: int = 0
    accepted: int = 0
    attempt: int = 0
    finished: bool = False


def reference_conforms(store: Path, contracts: Path, cid: str, cver: str,
                       probe_sha: str) -> tuple[bool | None, str]:
    """Judge the committed Windows reference executions of the same probe under the contract."""
    contract = load_contract(contracts / cid / f"{cver}.json")
    verdicts = []
    for m in sorted((store / "executions").glob("*.json")):
        meta = json.loads(m.read_text())
        if meta.get("probe_sha256") != probe_sha or \
                meta.get("profile_kind", "windows_reference") != "windows_reference":
            continue
        raw = trace_path(store, "sha256:" + m.stem).read_bytes()
        verdicts.append(compare(contract, raw, meta["roles"], "ref", "ref", "ref").verdict)
    if not verdicts:
        return None, "no Windows reference execution of this probe in the store"
    ok = all(v == "conforms" for v in verdicts)
    return ok, (f"{sum(v == 'conforms' for v in verdicts)} of {len(verdicts)} committed Windows "
                f"reference executions of {probe_sha[:23]} conform")


def execute(s: Session, source: str, timeout_seconds: int) -> Result:
    cand = admit(source, NAMES)                       # raises CandidateError before anything runs
    ld = load(s.probe)
    if s.backend == "managed_vm":
        from w2m.guest_runtime import BuildError
        from w2m.vm_backend import VM
        try:
            vm = VM(ld, last_error_source=cand.source)
        except BuildError as e:
            raise CandidateError(f"does not build into the guest runtime: {e}") from None
        try:
            return vm.run(timeout_ms=timeout_seconds * 1000)
        finally:
            vm.close()
    rt = ModelRuntime(ld)
    install(rt.kernel, cand)
    return rt.run()


def topology_for(s: Session, responder: Responder, store: Path, contracts: Path, cid: str,
                 cver: str) -> Callable[[TopologyBuilder], None]:
    t0 = time.monotonic()
    contract = load_contract(s.contract_path)
    tag = type(contract.acceptance).__struct_config__.tag           # "exact" | "allowed_outcome_set"
    if tag not in get_args(events.EnumRelation):
        raise ValueError(f"{s.contract_path}: acceptance relation {tag!r} is not in the vocabulary's relation enum")
    relation = cast(events.EnumRelation, tag)
    (store / "candidates").mkdir(parents=True, exist_ok=True)

    async def harness_open(_inp: Any) -> AsyncIterator[msgspec.Struct]:
        yield events.validate(events.SESSION_INIT(
            vocab_version=VOCAB_VERSION, profile=s.profile_id, contract_version=cver,
            harness_version="0.0.1", purpose="conformance"))

    async def harness_close(_inp: Any) -> AsyncIterator[msgspec.Struct]:
        s.finished = True
        yield events.validate(events.SESSION_COMPLETED(
            outcome=("aborted" if not s.accepted
                     else "completed_with_discrepancies" if s.reported else "completed"),
            discrepancy_count=s.reported, accepted_count=s.accepted,
            elapsed_seconds=round(time.monotonic() - t0, 3)))

    async def generator(inp: dict[str, Any]) -> AsyncIterator[msgspec.Struct]:
        s.attempt += 1
        reply, usage = await call_responder_metered(
            responder, prompt(contract, NAMES, inp.get("feedback")))
        yield usage
        source = extract_code(reply)
        digest = hashlib.sha256(source.encode()).hexdigest()
        (store / "candidates" / f"{digest}.c").write_text(source)
        cand_id = f"cand-{digest[:16]}-a{s.attempt}"
        s.sources[cand_id] = source
        s.results[cand_id] = []
        extra = {k: inp[k] for k in ("parent_candidate_id", "discrepancy_id") if inp.get(k)}
        yield events.validate(events.CANDIDATE_GENERATED(
            candidate_id=cand_id, contract_id=cid, contract_version=cver,
            generator_ref=s.generator_ref, attempt=s.attempt, candidate_sha256=f"sha256:{digest}",
            **extra))

    async def runner(inp: dict[str, Any]) -> AsyncIterator[msgspec.Struct]:
        cand_id = inp["candidate_id"]
        profile = f"{s.profile_id}-{cand_id}"
        for i in range(1, s.runs + 1):
            xid = f"{cand_id}-x{i}"
            yield events.validate(events.EXECUTION_STARTED(
                execution_id=xid, subject_kind="probe", subject_sha256=s.probe_sha,
                profile_kind=s.backend, profile_id=profile, initial_state_sha256=EMPTY,
                injected_inputs_sha256=EMPTY, timeout_seconds=TIMEOUT_SECONDS,
                kill_grace_seconds=1))
            started = time.monotonic()
            try:
                r = execute(s, s.sources[cand_id], TIMEOUT_SECONDS)
            except CandidateError as e:
                yield events.validate(events.EXECUTION_FAILED(
                    execution_id=xid, failure="launch_failed",
                    diagnostic=f"{REFUSED}{e}"))
                return
            elapsed = time.monotonic() - started
            body = r.stdout if r.stdout.strip() else b""
            trace, roles = wrap(body, xid, profile) if body else (b"", {})
            if not body:
                yield events.validate(events.EXECUTION_FAILED(
                    execution_id=xid, failure="harness_error",
                    diagnostic=f"no guest trace; the run stopped: {r.stop}"))
                continue
            digest = "sha256:" + hashlib.sha256(trace).hexdigest()
            dst = trace_path(store, digest)
            dst.parent.mkdir(parents=True, exist_ok=True)
            dst.write_bytes(trace)
            meta = {"probe_sha256": s.probe_sha, "candidate_id": cand_id,
                    "candidate_sha256": "sha256:" + hashlib.sha256(
                        s.sources[cand_id].encode()).hexdigest(),
                    "roles": roles, "profile_kind": s.backend, "exit_code": r.exit_code,
                    "elapsed_seconds": round(elapsed, 6), "stop": r.stop}
            (store / "executions").mkdir(parents=True, exist_ok=True)
            (store / "executions" / f"{digest.removeprefix('sha256:')}.json").write_text(
                json.dumps(meta, indent=1, sort_keys=True) + "\n")
            s.roles[xid] = roles
            yield events.validate(events.EXECUTION_COMPLETED(
                execution_id=xid,
                exit_status="exited" if r.stop == "ExitProcess" else
                "timed_out" if r.stop == "timeout" else "faulted",
                trace_sha256=digest, guest_line_count=trace.count(b"\n"),
                elapsed_seconds=round(elapsed, 6),
                **({"exit_code": r.exit_code} if r.exit_code is not None else
                   {"unobserved": f"exit_code: the guest did not exit ({r.stop})"})))

    def candidate_of(execution_id: str) -> str:
        return execution_id.rsplit("-x", 1)[0]

    def candidate_done(ctx: Any) -> bool:
        """Record this event's outcome, then say whether the candidate's runs are all in. Each
        outcome is recorded here, as its event arrives: the runner can finish all its runs
        before the first event is read (Sprint 032: three failed runs gave three rejections)."""
        p = ctx.event.payload
        if ctx.event.kind == "COMPARISON_COMPLETED":
            cand = candidate_of(p["target_execution_id"])
            s.results[cand].append({"execution_id": p["target_execution_id"],
                                    "comparison_id": p["comparison_id"], "verdict": p["verdict"],
                                    "first_divergence": p.get("first_divergence")})
        else:
            cand = candidate_of(p["execution_id"])
            outcome = ({"refused": p["diagnostic"].removeprefix(REFUSED)}
                       if p["failure"] == "launch_failed" else {"stop": p["diagnostic"]})
            s.results[cand].append({"execution_id": p["execution_id"], **outcome})
        res = s.results[cand]
        return any("refused" in r for r in res) or len(res) == s.runs

    async def guard(inp: dict[str, Any]) -> AsyncIterator[msgspec.Struct]:
        cand_id = inp["candidate_id"]
        res = s.results[cand_id]
        for path, expected in s.acceptance_baseline.items():
            observed = sha256_file(Path(path))
            if observed != expected:
                yield events.validate(events.ACCEPTANCE_CONDITION_ALTERED(
                    candidate_id=cand_id,
                    altered="contract" if path.endswith(".json") else "checker",
                    path=str(Path(path).relative_to(ROOT) if Path(path).is_relative_to(ROOT)
                             else path),
                    expected_sha256=expected, observed_sha256=observed))
                yield events.validate(events.CANDIDATE_REJECTED(
                    candidate_id=cand_id, contract_id=cid, contract_version=cver,
                    reason=f"acceptance condition altered: {path}"))
                return
        refused = [r["refused"] for r in res if "refused" in r]
        verdicts = [r.get("verdict") for r in res]
        if refused:
            yield events.validate(events.CANDIDATE_REJECTED(
                candidate_id=cand_id, contract_id=cid, contract_version=cver,
                reason=f"refused by admission check: {refused[0]}"))
        elif verdicts and all(v == "conforms" for v in verdicts):
            s.accepted += 1
            yield events.validate(events.CANDIDATE_ACCEPTED(
                candidate_id=cand_id, contract_id=cid, contract_version=cver,
                comparison_ids=tuple(r["comparison_id"] for r in res)))
        elif "discrepancy" in verdicts:
            first = next(r for r in res if r.get("verdict") == "discrepancy")
            yield events.validate(events.CANDIDATE_REJECTED(
                candidate_id=cand_id, contract_id=cid, contract_version=cver,
                reason=f"{verdicts.count('discrepancy')} of {len(res)} comparisons are "
                       f"discrepancies; first: {first['first_divergence']}",
                discrepancy_id=f"disc-{cand_id}"))
        else:
            stops = [r.get("stop") or r.get("verdict") for r in res]
            yield events.validate(events.CANDIDATE_REJECTED(
                candidate_id=cand_id, contract_id=cid, contract_version=cver,
                reason=f"no conforming comparison: {stops}"))

    ref_ok, ref_grounds = reference_conforms(store, contracts, cid, cver, s.probe_sha)

    async def reducer(inp: dict[str, Any]) -> AsyncIterator[msgspec.Struct]:
        if inp["kind"] == "accepted":
            while s.open_discrepancies:                      # pop first: the close trigger
                did = s.open_discrepancies.pop(0)            # reads the list on each event
                yield events.validate(events.DISCREPANCY_RESOLVED(
                    discrepancy_id=did, resolution="implementation_repaired",
                    resolving_ref=inp["candidate_id"]))
            return
        cand_id, did = inp["candidate_id"], inp["discrepancy_id"]
        first = next(r for r in s.results[cand_id] if r.get("verdict") == "discrepancy")
        yield events.validate(events.REDUCTION_COMPLETED(
            reduction_id=f"red-{cand_id}", comparison_id=first["comparison_id"],
            source_program_sha256=s.probe_sha, minimized_program_sha256=s.probe_sha, steps=0,
            preserved_mismatch=True))
        yield events.validate(events.DISCREPANCY_REPORTED(
            discrepancy_id=did, contract_id=cid, contract_version=cver,
            mismatch_class=relation, minimized_program_sha256=s.probe_sha,
            initial_state_sha256=EMPTY, first_divergence=first["first_divergence"],
            reference_profile_id="windows_reference (committed store executions)",
            destination_profile_id=f"{s.profile_id}-{cand_id}",
            comparison_id=first["comparison_id"]))
        s.reported += 1
        s.open_discrepancies.append(did)
        yield events.validate(events.DISCREPANCY_ROUTED(
            discrepancy_id=did,
            repair_path=("implementation_repair" if ref_ok else
                         "contract_revision" if ref_ok is False else "undetermined"),
            reference_conforms=bool(ref_ok), grounds=ref_grounds))

    def retry_input(ctx: Any) -> dict[str, Any]:
        p = ctx.event.payload
        if ctx.event.kind == "DISCREPANCY_ROUTED":
            cand = p["discrepancy_id"].removeprefix("disc-")
            first = next(r for r in s.results[cand] if r.get("verdict") == "discrepancy")
            return {"parent_candidate_id": cand, "discrepancy_id": p["discrepancy_id"],
                    "feedback": f"DISCREPANCY_REPORTED {p['discrepancy_id']}: contract {cid}@{cver}, "
                                f"relation {relation}, first divergence: "
                                f"{first['first_divergence']}"}
        return {"parent_candidate_id": p["candidate_id"], "feedback": p["reason"]}

    def wants_retry(ctx: Any) -> bool:
        p = ctx.event.payload
        if ctx.event.kind == "CANDIDATE_REJECTED" and p.get("discrepancy_id"):
            return False                                   # the route starts the repair
        if ctx.event.kind == "DISCREPANCY_ROUTED" and p["repair_path"] != "implementation_repair":
            return False
        return s.attempt < s.attempts

    def wants_close(ctx: Any) -> bool:
        if s.finished:
            return False
        if ctx.event.kind == "CANDIDATE_ACCEPTED":
            return not s.open_discrepancies
        if ctx.event.kind == "DISCREPANCY_RESOLVED":
            return not s.open_discrepancies
        return not wants_retry(ctx) and not (
            ctx.event.kind == "CANDIDATE_REJECTED" and ctx.event.payload.get("discrepancy_id"))

    def topology(b: TopologyBuilder) -> None:
        b.baseline(contract=f"{cid}@{cver}", probe_sha256=s.probe_sha, backend=s.backend,
                   generator=s.generator_ref, attempts=s.attempts, runs_per_candidate=s.runs)
        b.producer_kind("harness", schemas=[events.SESSION_INIT], schema_version=1,
                        factory=lambda: producer(harness_open))
        b.producer_kind("harness_close", schemas=[events.SESSION_COMPLETED], schema_version=1,
                        factory=lambda: producer(harness_close))
        b.producer_kind("candidate_generator", schemas=[events.CANDIDATE_GENERATED, ModelUsage],
                        schema_version=1, factory=lambda: producer(generator), deterministic=False)
        b.producer_kind("destination_runner", schemas=[events.EXECUTION_STARTED,
                        events.EXECUTION_COMPLETED, events.EXECUTION_FAILED], schema_version=1,
                        factory=lambda: producer(runner))
        b.producer_kind("trace_ingester", schemas=[events.TRACE_INGESTED, events.TRACE_REJECTED],
                        schema_version=1, factory=lambda: producer(trace_ingester(store)))
        b.producer_kind("comparator", schemas=[events.COMPARISON_COMPLETED], schema_version=1,
                        factory=lambda: producer(comparator(store, contracts)))
        b.producer_kind("acceptance_guard", schemas=[events.CANDIDATE_ACCEPTED,
                        events.CANDIDATE_REJECTED, events.ACCEPTANCE_CONDITION_ALTERED],
                        schema_version=1, factory=lambda: producer(guard))
        b.producer_kind("reducer", schemas=[events.REDUCTION_COMPLETED,
                        events.DISCREPANCY_REPORTED, events.DISCREPANCY_ROUTED,
                        events.DISCREPANCY_RESOLVED], schema_version=1, factory=lambda: producer(reducer))
        b.initial("harness", input=None)

        def on(kinds: set[str]) -> Subscription:
            return Subscription(kinds=frozenset(kinds))

        b.trigger("first-attempt", subscription=on({"SESSION_INIT"}), predicate=lambda c: True,
                  starts="candidate_generator", input_builder=lambda c: {}, policy=PerEvent())
        b.trigger("run", subscription=on({"CANDIDATE_GENERATED"}), predicate=lambda c: True,
                  starts="destination_runner", policy=PerEvent(),
                  input_builder=lambda c: {"candidate_id": c.event.payload["candidate_id"]})
        b.trigger("ingest", subscription=on({"EXECUTION_COMPLETED"}), predicate=lambda c: True,
                  starts="trace_ingester", policy=PerEvent(),
                  input_builder=lambda c: {"trace_sha256": c.event.payload["trace_sha256"],
                                           "execution_id": c.event.payload["execution_id"]})
        b.trigger("compare", subscription=on({"TRACE_INGESTED"}), predicate=lambda c: True,
                  starts="comparator", policy=PerEvent(),
                  input_builder=lambda c: {
                      "contract_id": cid, "contract_version": cver,
                      "trace_sha256": c.event.payload["trace_sha256"],
                      "role_map": s.roles[c.event.payload["execution_id"]],
                      "comparison_id": "cmp-" + c.event.payload["execution_id"],
                      "reference_ref": f"{cid}@{cver}",
                      "target_execution_id": c.event.payload["execution_id"]})
        b.trigger("guard", subscription=on({"COMPARISON_COMPLETED", "EXECUTION_FAILED"}),
                  predicate=candidate_done, starts="acceptance_guard", policy=PerEvent(),
                  input_builder=lambda c: {"candidate_id": candidate_of(
                      c.event.payload.get("target_execution_id") or c.event.payload["execution_id"])})
        b.trigger("report", subscription=on({"CANDIDATE_REJECTED"}),
                  predicate=lambda c: bool(c.event.payload.get("discrepancy_id")),
                  starts="reducer", policy=PerEvent(),
                  input_builder=lambda c: {"kind": "rejected",
                                           "candidate_id": c.event.payload["candidate_id"],
                                           "discrepancy_id": c.event.payload["discrepancy_id"]})
        b.trigger("resolve", subscription=on({"CANDIDATE_ACCEPTED"}),
                  predicate=lambda c: bool(s.open_discrepancies), starts="reducer",
                  policy=PerEvent(),
                  input_builder=lambda c: {"kind": "accepted",
                                           "candidate_id": c.event.payload["candidate_id"]})
        b.trigger("repair", subscription=on({"DISCREPANCY_ROUTED", "CANDIDATE_REJECTED"}),
                  predicate=wants_retry, starts="candidate_generator", policy=PerEvent(),
                  input_builder=retry_input)
        b.trigger("abort", subscription=on({"substrate.ProducerFailed"}),
                  predicate=lambda c: not s.finished, starts="harness_close", policy=PerEvent(),
                  input_builder=lambda c: None)            # T2 holds when an operator fails
        b.trigger("close", subscription=on({"CANDIDATE_ACCEPTED", "DISCREPANCY_RESOLVED",
                                            "DISCREPANCY_ROUTED", "CANDIDATE_REJECTED"}),
                  predicate=wants_close, starts="harness_close", policy=PerEvent(),
                  input_builder=lambda c: None)
        b.termination(threshold_count("SESSION_COMPLETED", 1))

    return topology


def run_loop(responder: Responder, generator_ref: str, *, contract: str, backend: events.EnumProfileKind,
             runs: int, attempts: int, store: Path, contracts: Path, runs_root: Path,
             probe: bytes, profile_id: str) -> Path:
    cid, cver = contract.split("@")
    cpath = contracts / cid / f"{cver}.json"
    s = Session(contract_path=cpath, probe=probe,
                probe_sha="sha256:" + hashlib.sha256(probe).hexdigest(), backend=backend,
                profile_id=profile_id, runs=runs, attempts=attempts,
                acceptance_baseline={str(p): sha256_file(p) for p in (cpath, *ACCEPTANCE_FILES)},
                generator_ref=generator_ref)
    root = runs_root / f"candidates-{time.time_ns()}"
    result = run_with_provenance(root, topology_for(s, responder, store, contracts, cid, cver))
    return Path(result.record_root)


def probe_binary(store: Path, contract: str) -> bytes:
    """The probe binary `contract` is judged on, from store/binaries, checked against its hash."""
    sha = PROBE_SHA256[contract]
    data = (store / "binaries" / f"{sha.removeprefix('sha256:')}.exe").read_bytes()
    if "sha256:" + hashlib.sha256(data).hexdigest() != sha:
        raise SystemExit(f"store/binaries copy of {sha} does not match its hash")
    return data


def summary(record: Path) -> list[str]:
    lines = []
    for e in read_record(record):
        k, p = e["kind"], e["payload"]
        if k.startswith("substrate.") or k == "ModelUsage":
            continue
        keys = [x for x in ("candidate_id", "attempt", "candidate_sha256", "execution_id",
                            "verdict", "first_divergence", "reason", "discrepancy_id",
                            "repair_path", "resolution", "outcome", "failure", "diagnostic")
                if x in p and p[x] is not None]
        lines.append(f"{k}: " + ", ".join(f"{x}={str(p[x])[:90]}" for x in keys))
    return lines


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--model", required=True, help="Ollama model tag (local)")
    ap.add_argument("--contract", default="windows.thread.last_error.set_get@0.2.0")
    ap.add_argument("--backend", choices=("managed_vm", "model"), default="managed_vm")
    ap.add_argument("--runs-per-candidate", type=int, default=3)
    ap.add_argument("--attempts", type=int, default=3)
    ap.add_argument("--store", type=Path, default=ROOT / "store")
    ap.add_argument("--contracts", type=Path, default=ROOT / "contracts")
    ap.add_argument("--runs", type=Path, default=ROOT / "runs")
    ap.add_argument("--first-candidate", type=Path)
    a = ap.parse_args(argv)
    if a.model.endswith(":cloud") or "-cloud" in a.model:
        raise SystemExit("cloud model tags send the prompt off this machine; use a local model")
    from substrate.adapters import OllamaResponder
    responder: Responder = OllamaResponder(a.model, temperature=0.2, num_ctx=8192, timeout=600.0)
    ref = f"ollama:{a.model} via scripts/run_candidate_loop.py"
    if a.first_candidate:
        responder = PlantedFirst(a.first_candidate.read_text(), responder)
        ref += f"; attempt 1 planted from {a.first_candidate.name}"
    record = run_loop(responder, ref,
                      contract=a.contract, backend=a.backend, runs=a.runs_per_candidate,
                      attempts=a.attempts, store=a.store, contracts=a.contracts,
                      runs_root=a.runs, probe=probe_binary(a.store, a.contract),
                      profile_id=f"w2m-{'vm' if a.backend == 'managed_vm' else 'model'}")
    print(f"record {record}")
    print("\n".join(summary(record)))
    for e in read_record(record):
        if e["kind"] in ("substrate.ProducerFailed", "substrate.RunFinalised") and \
                e["payload"].get("error"):
            print(f"{e['kind']}: {e['payload']['error']}")
    return 0 if any(e["kind"] == "CANDIDATE_ACCEPTED" for e in read_record(record)) else 1


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
