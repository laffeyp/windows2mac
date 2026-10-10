"""family_run.py — a Windows function family judged on its own record (Sprint 044).

Sprint 042's pipeline (scripts/file_run.py) for any family of functions that share state or a
probe: the family's no-CRT probe ran on the windows-2025 and windows-2022 runners; `rows` derives
the contract's rows from a probe's output (identity, or a relation where values differ between
machines); the contract is those rows from every runner execution, which must agree; the guest
C file implements the family; the item run runs each runner binary in the managed VM, derives its
rows the same way, and decides once.

Usage: uv run python scripts/family_run.py <family> --contract | --run | --plants
"""

from __future__ import annotations

import argparse
import hashlib
import json
import sys
import tempfile
import time
from collections import Counter
from collections.abc import AsyncIterator, Callable
from dataclasses import dataclass, field
from functools import partial
from pathlib import Path
from typing import Any

import msgspec

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))

from w2m import VOCAB_VERSION, events  # noqa: E402
from w2m.compare import compare_exact_rows  # noqa: E402
from w2m.contract import Contract, ExactRows, load_contract  # noqa: E402
from w2m.file_bridge import FileBridge  # noqa: E402
from w2m.loader import load  # noqa: E402
from w2m.record import producer, run_with_provenance  # noqa: E402

GUEST = ROOT / "tools" / "guest_runtime"
RETRIEVED = "retrieved 2026-10-10"


@dataclass(frozen=True)
class Run:
    image: str            # windows-2025, windows-2022
    binary: str           # the probe binary the runner built and ran (sha256 hex)
    execution: str        # the workflow's execution id prefix
    cpu: str


@dataclass(frozen=True)
class Plant:
    old: str              # text of the guest C file, present exactly once
    new: str
    reach: tuple[str, ...]  # the rows it changes, named before it runs


@dataclass(frozen=True)
class Family:
    name: str
    contract_id: str
    version: str
    summary: str
    probe: str            # probes/<probe>/
    guest_file: str       # tools/guest_runtime/<guest_file>
    runs: dict[str, Run]
    documented: tuple[dict[str, str], ...]
    open_questions: tuple[str, ...]
    derive: Callable[[list[dict[str, Any]]], list[dict[str, Any]]] = lambda printed: printed
    plants: dict[str, Plant] = field(default_factory=dict)
    excluded: dict[str, str] = field(default_factory=dict)    # row -> why the runners may differ on it

    def rows(self, printed: list[dict[str, Any]]) -> list[dict[str, Any]]:
        """The contract's rows from a probe's printed rows: derived, then the excluded rows dropped."""
        return [r for r in self.derive(printed) if r.get("s") not in self.excluded]

    @property
    def contract_path(self) -> Path:
        return ROOT / "contracts" / self.contract_id / f"{self.version}.json"


def documented(eid: str, page: str, updated: str, quote: str) -> dict[str, str]:
    return {"evidence_id": eid, "category": "documented", "applicability": "Windows 10 and later",
            "source_ref": f"https://learn.microsoft.com/en-us/windows/win32/{page} (page updated {updated}; {RETRIEVED})",
            "quote": quote}


def parse(output: bytes) -> list[dict[str, Any]]:
    out = []
    for line in output.splitlines():
        try:
            out.append(json.loads(line))
        except json.JSONDecodeError:
            continue
    return out


# ---- families -------------------------------------------------------------------------------

def process_query_rows(printed: list[dict[str, Any]]) -> list[dict[str, Any]]:
    """IsProcessorFeaturePresent's answer depends on the CPU; where a CPUID bit names the feature,
    the row is whether the answer equals that bit (on Windows, the runner's CPU; in w2m, the guest's
    CPUID profile). Every other row is as printed."""
    rows = []
    for r in printed:
        if "cpuid" in r:
            rows.append({"s": r["s"], "equals_cpuid": r["value"] == r["cpuid"]})
        else:
            rows.append(r)
    return rows


FAMILIES: dict[str, Family] = {
    "process_queries": Family(
        name="process_queries",
        contract_id="windows.process.queries",
        version="0.1.0",
        summary=("GetSystemTimeAsFileTime, GetCurrentProcessId, GetCurrentThreadId, QueryPerformanceCounter, "
                 "QueryPerformanceFrequency, IsProcessorFeaturePresent, InitializeSListHead, GetStartupInfoW and "
                 "IsDebuggerPresent answer as Windows does: fixed values exactly, machine-dependent values by the "
                 "relations every Windows machine observed holds."),
        probe="process_queries", guest_file="process.c",
        runs={"38037899422": Run("windows-2025", "7bddfdfb38be9ebb35220a9a9df48dc0f3e6453158cb613db62b337680caea99",
                                 "process_queries-1791620805-0", "AMD EPYC 7763"),
              "38037900735": Run("windows-2022", "d59934c1891538e6f2768682fa34e034a9d494acf546babd422ff294d4beee9c",
                                 "process_queries-1791620806-1", "AMD EPYC 7763"),
              "38037902222": Run("windows-2025", "c703c133a5a032adc6d13b8579bc11584de037354ccd7e23e8ce1cbf6d76bad2",
                                 "process_queries-1791620808-2", "AMD EPYC 9V74"),
              "38037903596": Run("windows-2022", "552490a3ed425d2f5d6ff271dea0823312fa003843907eb0ce8100d07010c970",
                                 "process_queries-1791620809-3", "AMD EPYC 9V74"),
              "38038027558": Run("windows-2022", "c816bb72de67ec46cb89bb91868742818027e65ea4fcc194a2215b4a04b8fecc",
                                 "process_queries-1791620943-3", "Intel Xeon Platinum 8370C")},
        documented=(
            documented("ms-isprocessorfeaturepresent", "api/processthreadsapi/nf-processthreadsapi-isprocessorfeaturepresent",
                       "2025-12-18", "If the feature is supported, the return value is a nonzero value. If the feature is "
                       "not supported, the return value is zero. ... Support for PF_ARM_SVE_INSTRUCTIONS_AVAILABLE through "
                       "PF_MOVDIR64B_INSTRUCTION_AVAILABLE ... were added in the Windows SDK (26100) and are supported by "
                       "Windows 11, version 24H2 and Windows Server 2025 or later."),
            documented("ms-queryperformancefrequency", "api/profileapi/nf-profileapi-queryperformancefrequency", "2025-11-21",
                       "The frequency of the performance counter is fixed at system boot and is consistent across all "
                       "processors. ... On systems that run Windows XP or later, the function will always succeed when given "
                       "valid parameters and will thus never return zero.")),
        open_questions=(
            "The rows for IsProcessorFeaturePresent are relations where a CPUID bit names the feature: on Windows the "
            "runner CPU's bit, in w2m the guest CPUID profile's (specs/guest_cpuid.json). The PF-to-CPUID-bit table is "
            "the agent's reading of the Intel SDM CPUID tables; 25 of 26 rows agree with Windows on five machines.",
            "GetStartupInfoW's lpReserved is non-null under the runner's cmd; what it points to is not observed. A "
            "process started by Steam may differ.",
            "Excluded rows: see `sections.excluded`."),
        derive=process_query_rows,
        excluded={"ipfp.60": "PF_BMI2_INSTRUCTIONS_AVAILABLE: Server 2022 answers 0 on CPUs with BMI2, Server 2025 "
                             "follows CPUID; Microsoft Learn: supported from Windows 11 24H2 and Server 2025. w2m answers "
                             "as Server 2025.",
                  "ipfp.21": "PF_VIRT_FIRMWARE_ENABLED: 1 on the four AMD machines, 0 on the Intel 8370C; a property of "
                             "the host machine. w2m answers 0: the guest is offered no virtualization."},
        plants={"counter_frequency_wrong": Plant("#define QPC_FREQUENCY 10000000ull", "#define QPC_FREQUENCY 24000000ull",
                                                 ("time.qpf", "time.counter_tracks_time")),
                "process_id_from_thread": Plant("(u32)w2m_teb()->client_process", "(u32)w2m_teb()->client_thread",
                                                ("ids.process", "ids.thread"))}),
}


# ---- the pipeline ---------------------------------------------------------------------------

def runner_rows(f: Family) -> dict[tuple[str, int], list[dict[str, Any]]]:
    """Every runner execution's rows (each run made two)."""
    return {(run, i): f.rows(parse((ROOT / "store" / "probe-runs" / run / f"body-{i}.jsonl").read_bytes()))
            for run in f.runs for i in (1, 2)}


def build_contract(f: Family) -> dict[str, Any]:
    by_run = runner_rows(f)
    first = next(iter(by_run.values()))
    unequal = {k: [a["s"] for a, b in zip(first, v) if a != b] for k, v in by_run.items() if v != first}
    if unequal:
        raise SystemExit(f"runner rows differ: {unequal}")
    return {
        "contract_id": f.contract_id, "version": f.version, "status": "accepted", "summary": f.summary,
        "applicability": "; ".join(sorted({f"{r.image} ({r.cpu})" for r in f.runs.values()})),
        "roles": ["main"],
        "scenario": {"initial": {"process": "started by the runner's cmd with standard output redirected"},
                     "execution": {"main": f"probes/{f.probe}/{f.probe}.c, top to bottom"},
                     "observation": {"phase": "after each call"}},
        "acceptance": {"relation": "exact_rows", "key": "s", "rows": first},
        "evidence": [
            {"evidence_id": "runner-rows", "category": "observed",
             "source_ref": "; ".join(f"run {k}: {r.image}, {r.cpu}, probe sha256:{r.binary}" for k, r in f.runs.items()),
             "applicability": "the builds and CPUs observed",
             "execution_ids": [f"{r.execution}-{i}" for r in f.runs.values() for i in (1, 2)]},
            *f.documented],
        "open_questions": list(f.open_questions),
        "sections": {"probe": f"probes/{f.probe} (Sprint 044)", "implementation": f"tools/guest_runtime/{f.guest_file}",
                     "rows": "scripts/family_run.py " + f.name,
                     "excluded": "; ".join(f"{k}: {v}" for k, v in sorted(f.excluded.items())) or "none"}}


def contract(f: Family) -> Contract:
    return load_contract(f.contract_path)


def run_in_vm(binary: str, sources: dict[str, str] | None = None) -> tuple[bytes, str, float]:
    """A runner's probe binary in the managed VM with a fresh file-system root; (stdout, stop, seconds)."""
    from w2m.vm_backend import VM
    t0 = time.monotonic()
    with tempfile.TemporaryDirectory(prefix="w2m-family-") as d:
        files = FileBridge(Path(d))
        vm = VM(load((ROOT / "store" / "binaries" / f"{binary}.exe").read_bytes()), files=files,
                runtime_sources=sources)
        try:
            r = vm.run(max_steps=1 << 40, timeout_ms=60_000)    # the clock probes spin until time advances
        finally:
            vm.close()
            files.close_all()
    return r.stdout, r.stop, time.monotonic() - t0


def derived_output(f: Family, out: bytes) -> bytes:
    return b"".join(json.dumps(r, separators=(",", ":")).encode() + b"\n" for r in f.rows(parse(out)))


def unequal_rows(f: Family, out: bytes) -> list[str]:
    c = contract(f)
    assert isinstance(c.acceptance, ExactRows)
    got = {r.get("s"): r for r in f.rows(parse(out))}
    return [r["s"] for r in c.acceptance.rows if got.get(r["s"]) != r]


def planted_sources(f: Family, name: str) -> dict[str, str]:
    p = f.plants[name]
    text = (GUEST / f.guest_file).read_text()
    assert text.count(p.old) == 1, f"plant {name}: its target text is not in {f.guest_file} exactly once"
    return {f.guest_file: text.replace(p.old, p.new)}


def item_run(f: Family, runs: Path, sources: dict[str, str] | None = None,
             generator_ref: str = "hand (agent-authored)") -> dict[str, Any]:
    """Judge the family's guest C (or `sources`) on its own substrate record."""
    from substrate.api import Once, PerEvent, Subscription, TopologyBuilder, read_record, threshold_count

    c = contract(f)
    text = (sources or {}).get(f.guest_file, (GUEST / f.guest_file).read_text())
    sha = "sha256:" + hashlib.sha256(text.encode()).hexdigest()
    cand = f"{f.name}-{sha[7:19]}"
    run_id = f"family-{f.name}-{time.time_ns()}"
    t0 = time.monotonic()
    state: dict[str, Any] = {"verdicts": {}, "divergence": {}}

    async def harness(_inp: Any) -> AsyncIterator[msgspec.Struct]:
        yield events.validate(events.SESSION_INIT(
            vocab_version=VOCAB_VERSION, profile="; ".join(f"{r.image} run {k}" for k, r in f.runs.items()),
            contract_version=f.version, harness_version="0.0.1", purpose="conformance"))

    async def generator(_inp: Any) -> AsyncIterator[msgspec.Struct]:
        yield events.validate(events.CANDIDATE_GENERATED(
            candidate_id=cand, contract_id=f.contract_id, contract_version=f.version,
            generator_ref=f"{generator_ref}: tools/guest_runtime/{f.guest_file}", attempt=1, candidate_sha256=sha))

    def judge(run: str) -> Callable[[Any], AsyncIterator[msgspec.Struct]]:
        async def go(_inp: dict[str, Any]) -> AsyncIterator[msgspec.Struct]:
            r = f.runs[run]
            out, stop, _ = run_in_vm(r.binary, sources)
            cmp = compare_exact_rows(c, derived_output(f, out), f"cmp-{cand}-{run}", f"{r.execution}-1",
                                     f"managed_vm-{cand}-{run}")
            if stop != "ExitProcess" and cmp.verdict == "conforms":
                cmp = msgspec.structs.replace(cmp, verdict="discrepancy", first_divergence=f"run stopped: {stop}")
            state["verdicts"][run] = cmp.verdict
            state["divergence"][run] = cmp.first_divergence or (stop if stop != "ExitProcess" else None)
            yield cmp
        return go

    async def decide(_inp: Any) -> AsyncIterator[msgspec.Struct]:
        if all(v == "conforms" for v in state["verdicts"].values()):
            yield events.validate(events.CANDIDATE_ACCEPTED(
                candidate_id=cand, contract_id=f.contract_id, contract_version=f.version,
                comparison_ids=tuple(f"cmp-{cand}-{r}" for r in f.runs)))
            return
        yield events.validate(events.CANDIDATE_REJECTED(
            candidate_id=cand, contract_id=f.contract_id, contract_version=f.version,
            reason=f"{sum(v != 'conforms' for v in state['verdicts'].values())} of {len(f.runs)} comparisons differ",
            discrepancy_id=f"{run_id}-d1"))
        first = next(r for r, v in state["verdicts"].items() if v != "conforms")
        yield events.validate(events.DISCREPANCY_REPORTED(
            discrepancy_id=f"{run_id}-d1", contract_id=f.contract_id, contract_version=f.version, mismatch_class="exact",
            minimized_program_sha256="sha256:" + f.runs[first].binary, initial_state_sha256="sha256:" + "0" * 64,
            first_divergence=f"{f.runs[first].image} binary: {state['divergence'][first]}"[:500],
            reference_profile_id=f"{f.runs[first].image} run {first}", destination_profile_id="w2m-managed_vm",
            comparison_id=f"cmp-{cand}-{first}"))
        yield events.validate(events.DISCREPANCY_ROUTED(
            discrepancy_id=f"{run_id}-d1", repair_path="implementation_repair", reference_conforms=True,
            grounds=f"Windows's rows are the reference; tools/guest_runtime/{f.guest_file} differs"))

    async def close(_inp: Any) -> AsyncIterator[msgspec.Struct]:
        bad = sum(1 for v in state["verdicts"].values() if v != "conforms")
        yield events.validate(events.SESSION_COMPLETED(
            outcome="completed_with_discrepancies" if bad else "completed", discrepancy_count=1 if bad else 0,
            accepted_count=0 if bad else 1, elapsed_seconds=round(time.monotonic() - t0, 3)))

    def topology(b: TopologyBuilder) -> None:
        b.baseline(contract_id=f.contract_id, contract_version=f.version, candidate_sha256=sha,
                   reference_runs=list(f.runs), family=f.name)
        b.producer_kind("harness", schemas=[events.SESSION_INIT], schema_version=1, factory=lambda: producer(harness))
        b.producer_kind("implementation_source", schemas=[events.CANDIDATE_GENERATED], schema_version=1,
                        factory=lambda: producer(generator))
        names = {run: f"vm_judge_{run}" for run in f.runs}
        for run, name in names.items():
            b.producer_kind(name, schemas=[events.COMPARISON_COMPLETED], schema_version=1,
                            factory=partial(lambda x: producer(judge(x)), run))
        b.producer_kind("acceptance", schemas=[events.CANDIDATE_ACCEPTED, events.CANDIDATE_REJECTED,
                        events.DISCREPANCY_REPORTED, events.DISCREPANCY_ROUTED], schema_version=1,
                        factory=lambda: producer(decide))
        b.producer_kind("harness_close", schemas=[events.SESSION_COMPLETED], schema_version=1,
                        factory=lambda: producer(close))
        b.initial("harness", input=None)

        def on(kinds: set[str]) -> Subscription:
            return Subscription(kinds=frozenset(kinds))

        b.trigger("implementation-on-session-init", subscription=on({"SESSION_INIT"}), predicate=lambda c: True,
                  starts="implementation_source", input_builder=lambda c: {}, policy=PerEvent())
        for name in names.values():
            b.trigger(f"{name}-on-candidate", subscription=on({"CANDIDATE_GENERATED"}), predicate=lambda c: True,
                      starts=name, policy=PerEvent(), input_builder=lambda c: {})
        b.trigger("acceptance-on-comparisons", subscription=on({"COMPARISON_COMPLETED"}),
                  predicate=lambda c: len(state["verdicts"]) == len(f.runs), starts="acceptance", policy=Once(),
                  input_builder=lambda c: {})
        b.trigger("close-on-decision", subscription=on({"substrate.ProducerCompleted"}),
                  predicate=lambda c: c.event.payload["producer"]["kind"] == "acceptance",
                  starts="harness_close", policy=PerEvent(), input_builder=lambda c: None)
        b.termination(threshold_count("SESSION_COMPLETED", 1))

    result = run_with_provenance(runs / run_id, topology)
    kinds = Counter(e["kind"] for e in read_record(result.record_root) if not e["kind"].startswith("substrate."))
    return {"family": f.name, "record": str(Path(result.record_root).relative_to(ROOT)), "candidate_sha256": sha,
            "verdicts": dict(state["verdicts"]), "divergence": dict(state["divergence"]), "events": dict(kinds)}


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("family", choices=sorted(FAMILIES))
    ap.add_argument("--contract", action="store_true")
    ap.add_argument("--run", action="store_true")
    ap.add_argument("--plants", action="store_true")
    ap.add_argument("--runs", type=Path, default=ROOT / "store" / "runs")
    a = ap.parse_args(argv)
    f = FAMILIES[a.family]
    if a.contract:
        f.contract_path.parent.mkdir(parents=True, exist_ok=True)
        f.contract_path.write_text(json.dumps(build_contract(f), indent=1) + "\n")
        print(f"{f.contract_path.relative_to(ROOT)}: {len(contract(f).acceptance.rows)} rows")  # type: ignore[union-attr]
    if a.run:
        s = item_run(f, a.runs)
        index = a.runs / "family-runs.json"
        done = json.loads(index.read_text()) if index.exists() else {}
        done[f.name] = s
        index.write_text(json.dumps(done, indent=1, sort_keys=True) + "\n")
        print(json.dumps(s, indent=1))
    if a.plants:
        binary = next(iter(f.runs.values())).binary
        for name, p in f.plants.items():
            out, stop, _ = run_in_vm(binary, planted_sources(f, name))
            rows = unequal_rows(f, out)
            print(f"{name}: unequal {rows} ({stop}); named reach {list(p.reach)}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
