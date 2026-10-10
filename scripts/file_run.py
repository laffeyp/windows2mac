# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""file_run.py — CreateFileW and its companions judged on their own record (Sprint 042).

The pipeline for one Windows function, end to end:

  probes/create_file          a no-CRT probe; the windows-2025 and windows-2022 runners printed
                              188 rows each, equal byte for byte (RUNS)
  contract                    windows.file.create_file.dispositions_sharing@0.1.0: those rows,
                              relation exact (`exact_rows` in the contract file), built here
  tools/guest_runtime/file.c  the implementation, inside the guest runtime
  src/w2m/file_bridge.py      POSIX primitives under a per-run root on the Mac
  item run                    SESSION_INIT -> CANDIDATE_GENERATED (file.c's sha256)
                              -> COMPARISON_COMPLETED per runner binary, run in the managed VM
                              -> CANDIDATE_ACCEPTED, or DISCREPANCY_REPORTED + ROUTED
                              -> SESSION_COMPLETED

Usage: uv run python scripts/file_run.py --contract       (re)build the contract from the runs
       uv run python scripts/file_run.py --run [--runs store/runs]
       uv run python scripts/file_run.py --plants         each planted fault's unequal rows
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

CONTRACT_ID = "windows.file.create_file.dispositions_sharing"
VERSION = "0.1.0"
CONTRACT = ROOT / "contracts" / CONTRACT_ID / f"{VERSION}.json"
FILE_C = ROOT / "tools" / "guest_runtime" / "file.c"
# runner run -> (image, the probe binary it built and ran, execution id prefix)
RUNS = {"38031352273": ("windows-2025", "798e17f634c9adc4af0322029392232251034c3eeae7565db318ed0278b85fdb",
                        "create_file-1791614008-0"),
        "38031354036": ("windows-2022", "92685e0bbb89b5196a8850cc15157f9b499d598eac4cb4d74e6dbf257797cb40",
                        "create_file-1791614010-1")}
RETRIEVED = "retrieved 2026-10-10"


def _documented(eid: str, url: str, updated: str, quote: str) -> dict[str, str]:
    return {"evidence_id": eid, "category": "documented", "applicability": "Windows XP and later",
            "source_ref": f"{url} (page updated {updated}; {RETRIEVED})", "quote": quote}


def build_contract() -> dict[str, Any]:
    bodies = {(run, i): (ROOT / "store" / "probe-runs" / run / f"body-{i}.jsonl").read_bytes()
              for run in RUNS for i in (1, 2)}
    first = next(iter(bodies.values()))
    unequal = [k for k, b in bodies.items() if b != first]
    if unequal:
        raise SystemExit(f"runner outputs differ: {unequal}")
    rows = [json.loads(line) for line in first.splitlines()]
    profiles = {run: (ROOT / "store" / "probe-runs" / run / "profile.txt").read_text().strip() for run in RUNS}
    learn = "https://learn.microsoft.com/en-us/windows/win32"
    return {
        "contract_id": CONTRACT_ID, "version": VERSION, "status": "accepted",
        "summary": ("CreateFileW's five dispositions on absent and existing files, the sharing rule over every "
                    "pair of {read, write, both} x {no share, read, write, both}, DeleteFileW on closed and open "
                    "files, name forms, and ReadFile, WriteFile and SetFilePointerEx on one handle produce "
                    "exactly the rows Windows printed: validity, return values, last-error codes, sizes and bytes."),
        "applicability": "Windows Server 2025 build 26100 and Server 2022 build 20348 (both observed); NTFS",
        "roles": ["main"],
        "scenario": {
            "initial": {"directory": "the process's working directory, without the probe's files"},
            "execution": {"main": "probes/create_file/create_file.c, top to bottom; SetLastError(0xDEAD) before each "
                                  "call under test, GetLastError after it"},
            "observation": {"phase": "after each call, and after each scenario's handles are closed"}},
        "acceptance": {"relation": "exact_rows", "key": "s", "rows": rows},
        "evidence": [
            {"evidence_id": "runner-rows", "category": "observed",
             "source_ref": "; ".join(f"run {r}: {profiles[r]}, probe sha256:{RUNS[r][1]}" for r in RUNS),
             "applicability": "the two builds observed",
             "execution_ids": [f"{RUNS[r][2]}-{i}" for r in RUNS for i in (1, 2)]},
            _documented("ms-createfilew-dispositions", f"{learn}/api/fileapi/nf-fileapi-createfilew", "2025-07-01",
                        "If the specified file exists and is writable, the function truncates the file, the function "
                        "succeeds, and last-error code is set to ERROR_ALREADY_EXISTS (183). ... If the specified file "
                        "exists, the function fails and the last-error code is set to ERROR_FILE_EXISTS (80). ... If "
                        "the specified file exists, the function succeeds and the last-error code is set to "
                        "ERROR_ALREADY_EXISTS (183). ... The calling process must open the file with the GENERIC_WRITE "
                        "bit set as part of the dwDesiredAccess parameter."),
            _documented("ms-sharing-table", f"{learn}/fileio/creating-and-opening-files", "2026-07-17",
                        "the system compares the requested access and sharing modes to those specified when the file "
                        "was opened. If you specify an access or sharing mode that conflicts with the modes specified "
                        "in the previous call, CreateFile fails."),
            _documented("ms-deletefile", f"{learn}/api/fileapi/nf-fileapi-deletefilew", "2025-07-01",
                        "The DeleteFile function fails if an application attempts to delete a file that has other "
                        "handles open for normal I/O or as a memory-mapped file (FILE_SHARE_DELETE must have been "
                        "specified when other handles were opened). ... The use of POSIX delete causes the file to be "
                        "deleted while handles remain open. Subsequent calls to CreateFile to open the file fail with "
                        "ERROR_FILE_NOT_FOUND."),
            _documented("ms-setfilepointerex", f"{learn}/api/fileapi/nf-fileapi-setfilepointerex", "2025-07-01",
                        "Note that it is not an error to set the file pointer to a position beyond the end of the file. "
                        "... A write operation increases the size of the file to the file pointer position plus the "
                        "size of the buffer written, which results in the intervening bytes being zero initialized.")],
        "open_questions": [
            "DeleteFileW's page also says DeleteFile 'marks a file for deletion on close' with later opens failing "
            "with ERROR_ACCESS_DENIED; both observed builds instead deleted at once (POSIX delete): OPEN_EXISTING "
            "then failed with 2 and CREATE_NEW made a new file. The rows follow the observation.",
            "Naming Files says only 'Do not end a file or directory name with a space or a period'; the rows show "
            "Windows opens 'name.' and 'name ' as 'name'.",
            "ERROR_NEGATIVE_SEEK (131) for a position before the start is observed only; the SetFilePointerEx page "
            "does not name the code.",
            "Handle values are not compared (only validity); flags other than FILE_ATTRIBUTE_NORMAL, security "
            "attributes, templates, overlapped I/O and absolute paths are outside this scenario."],
        "sections": {"probe": "probes/create_file (Sprint 042)",
                     "implementation": "tools/guest_runtime/file.c with src/w2m/file_bridge.py"}}


def contract() -> Contract:
    return load_contract(CONTRACT)


def run_in_vm(binary: str, file_source: str | None = None,
              bridge: Callable[[Path], FileBridge] = FileBridge) -> tuple[bytes, str, float]:
    """The probe binary in the managed VM with a fresh root; (stdout, stop, seconds)."""
    from w2m.vm_backend import VM
    t0 = time.monotonic()
    with tempfile.TemporaryDirectory(prefix="w2m-files-") as d:
        files = bridge(Path(d))
        vm = VM(load((ROOT / "store" / "binaries" / f"{binary}.exe").read_bytes()), files=files,
                file_source=file_source)
        try:
            r = vm.run()
        finally:
            vm.close()
            files.close_all()
    return r.stdout, r.stop, time.monotonic() - t0


def unequal_rows(c: Contract, output: bytes) -> list[str]:
    """Every reference row the output does not reproduce exactly (by key)."""
    x = c.acceptance
    assert isinstance(x, ExactRows)
    got = {}
    for line in output.splitlines():
        try:
            row = json.loads(line)
        except json.JSONDecodeError:
            continue
        got[row.get(x.key)] = row
    return [r[x.key] for r in x.rows if got.get(r[x.key]) != r]


# Planted faults (WORKING_AGREEMENT: reach named before the plant runs). Each is (file.c text,
# replacement, the rows it changes); a bridge plant replaces the host's name folding instead.
PLANTS: dict[str, tuple[str, str, str]] = {
    "sharing_without_reverse_check": (
        "if ((access & ~f->share) || (f->access & ~share)) return 0;", "if (access & ~f->share) return 0;",
        "35 of the 144 share.* rows (the documented rule, which the 144 recorded rows follow)"),
    "already_exists_dropped": (
        "? ERROR_ALREADY_EXISTS : 0", "? 0 : 0",
        "2 rows: disp.create_always.existing, disp.open_always.existing"),
    "trailing_dot_kept": (
        "    if (!dots)\n        while", "    if (!dots && 0)\n        while",
        "2 rows: name.trailing_dot, name.trailing_space"),
}


def planted_source(name: str) -> str:
    old, new, _ = PLANTS[name]
    text = FILE_C.read_text()
    assert text.count(old) == 1, f"plant {name}: its target text is not in file.c exactly once"
    return text.replace(old, new)


class CaseSensitiveBridge(FileBridge):
    """Planted fault: names compare exactly. Reach: name.case.open_existing changes (2 instead of a
    handle); name.case.create_new then asks the Mac for O_EXCL on a name APFS (case-insensitive by
    default) already has, which the runtime reports as unimplemented, so the run stops there."""

    def fold(self, name: str) -> str:
        return name


def item_run(runs: Path, file_source: str | None = None,
             generator_ref: str = "hand (agent-authored)") -> dict[str, Any]:
    """Judge file.c (or `file_source`) on its own substrate record; return its summary."""
    from substrate.api import Once, PerEvent, Subscription, TopologyBuilder, read_record, threshold_count

    c = contract()
    source = file_source if file_source is not None else FILE_C.read_text()
    sha = "sha256:" + hashlib.sha256(source.encode()).hexdigest()
    cand = f"file-c-{sha[7:19]}"
    run_id = f"file-createfilew-{time.time_ns()}"
    reference = "; ".join(f"{RUNS[r][0]} run {r}" for r in RUNS)
    t0 = time.monotonic()
    state: dict[str, Any] = {"verdicts": {}, "divergence": {}}

    async def harness(_inp: Any) -> AsyncIterator[msgspec.Struct]:
        yield events.validate(events.SESSION_INIT(
            vocab_version=VOCAB_VERSION, profile=reference, contract_version=VERSION,
            harness_version="0.0.1", purpose="conformance"))

    async def generator(_inp: Any) -> AsyncIterator[msgspec.Struct]:
        yield events.validate(events.CANDIDATE_GENERATED(
            candidate_id=cand, contract_id=CONTRACT_ID, contract_version=VERSION,
            generator_ref=f"{generator_ref}: tools/guest_runtime/file.c", attempt=1, candidate_sha256=sha))

    def judge(run: str) -> Any:
        async def go(inp: dict[str, Any]) -> AsyncIterator[msgspec.Struct]:
            out, stop, seconds = run_in_vm(RUNS[run][1], file_source)
            cmp = compare_exact_rows(c, out, f"cmp-{cand}-{RUNS[run][0]}", f"{RUNS[run][2]}-1",
                                     f"managed_vm-{cand}-{RUNS[run][0]}")
            if stop != "ExitProcess" and cmp.verdict == "conforms":
                cmp = msgspec.structs.replace(cmp, verdict="discrepancy", first_divergence=f"run stopped: {stop}")
            state["verdicts"][run] = cmp.verdict
            state["divergence"][run] = cmp.first_divergence or (stop if stop != "ExitProcess" else None)
            yield cmp
        return go

    async def decide(_inp: Any) -> AsyncIterator[msgspec.Struct]:
        if all(v == "conforms" for v in state["verdicts"].values()):
            yield events.validate(events.CANDIDATE_ACCEPTED(
                candidate_id=cand, contract_id=CONTRACT_ID, contract_version=VERSION,
                comparison_ids=tuple(f"cmp-{cand}-{RUNS[r][0]}" for r in RUNS)))
            return
        empty = "sha256:" + "0" * 64
        for n, (run, verdict) in enumerate(state["verdicts"].items(), 1):
            if verdict == "conforms":
                continue
            did = f"{run_id}-d{n}"
            yield events.validate(events.DISCREPANCY_REPORTED(
                discrepancy_id=did, contract_id=CONTRACT_ID, contract_version=VERSION, mismatch_class="exact",
                minimized_program_sha256=empty, initial_state_sha256=empty,
                first_divergence=f"{RUNS[run][0]} binary: {state['divergence'][run]}"[:500],
                reference_profile_id=f"{RUNS[run][0]} run {run}", destination_profile_id="w2m-managed_vm",
                comparison_id=f"cmp-{cand}-{RUNS[run][0]}"))
            yield events.validate(events.DISCREPANCY_ROUTED(
                discrepancy_id=did, repair_path="implementation_repair", reference_conforms=True,
                grounds="Windows's printed rows are the reference; file.c or the bridge differs"))

    async def close(_inp: Any) -> AsyncIterator[msgspec.Struct]:
        bad = sum(1 for v in state["verdicts"].values() if v != "conforms")
        yield events.validate(events.SESSION_COMPLETED(
            outcome="completed_with_discrepancies" if bad else "completed", discrepancy_count=bad,
            accepted_count=0 if bad else 1, elapsed_seconds=round(time.monotonic() - t0, 3)))

    def topology(b: TopologyBuilder) -> None:
        b.baseline(contract_id=CONTRACT_ID, contract_version=VERSION, candidate_sha256=sha,
                   reference_runs=list(RUNS))
        b.producer_kind("harness", schemas=[events.SESSION_INIT], schema_version=1, factory=lambda: producer(harness))
        b.producer_kind("implementation_source", schemas=[events.CANDIDATE_GENERATED], schema_version=1,
                        factory=lambda: producer(generator))
        for run in RUNS:
            b.producer_kind(f"vm_judge_{RUNS[run][0].replace('-', '_')}", schemas=[events.COMPARISON_COMPLETED],
                            schema_version=1, factory=partial(lambda r: producer(judge(r)), run))
        b.producer_kind("acceptance", schemas=[events.CANDIDATE_ACCEPTED, events.DISCREPANCY_REPORTED,
                        events.DISCREPANCY_ROUTED], schema_version=1, factory=lambda: producer(decide))
        b.producer_kind("harness_close", schemas=[events.SESSION_COMPLETED], schema_version=1,
                        factory=lambda: producer(close))
        b.initial("harness", input=None)

        def on(kinds: set[str]) -> Subscription:
            return Subscription(kinds=frozenset(kinds))

        b.trigger("generate", subscription=on({"SESSION_INIT"}), predicate=lambda c: True,
                  starts="implementation_source", input_builder=lambda c: {}, policy=PerEvent())
        for run in RUNS:
            name = f"vm_judge_{RUNS[run][0].replace('-', '_')}"
            b.trigger(f"judge-{name}", subscription=on({"CANDIDATE_GENERATED"}), predicate=lambda c: True,
                      starts=name, policy=PerEvent(),
                      input_builder=lambda c: {"candidate_id": c.event.payload["candidate_id"]})
        b.trigger("decide", subscription=on({"COMPARISON_COMPLETED"}),
                  predicate=lambda c: len(state["verdicts"]) == len(RUNS), starts="acceptance", policy=Once(),
                  input_builder=lambda c: {})
        b.trigger("close", subscription=on({"substrate.ProducerCompleted"}),
                  predicate=lambda c: c.event.payload["producer"]["kind"] == "acceptance",
                  starts="harness_close", policy=PerEvent(), input_builder=lambda c: None)
        b.termination(threshold_count("SESSION_COMPLETED", 1))

    result = run_with_provenance(runs / run_id, topology)
    kinds = Counter(e["kind"] for e in read_record(result.record_root) if not e["kind"].startswith("substrate."))
    record = Path(result.record_root).resolve()       # relative to the repository, as family_run
    return {"record": str(record.relative_to(ROOT) if record.is_relative_to(ROOT) else record), "candidate_sha256": sha, "verdicts": dict(state["verdicts"]),
            "divergence": dict(state["divergence"]), "events": dict(kinds)}


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--contract", action="store_true")
    ap.add_argument("--run", action="store_true")
    ap.add_argument("--plants", action="store_true")
    ap.add_argument("--runs", type=Path, default=ROOT / "store" / "runs")
    a = ap.parse_args(argv)
    if a.contract:
        CONTRACT.parent.mkdir(parents=True, exist_ok=True)
        CONTRACT.write_text(json.dumps(build_contract(), indent=1) + "\n")
        print(f"{CONTRACT.relative_to(ROOT)}: {len(contract().acceptance.rows)} rows")  # type: ignore[union-attr]
    if a.run:
        s = item_run(a.runs)
        s["record"] = str(Path(s["record"]).relative_to(ROOT))
        (a.runs / "file-runs.json").write_text(json.dumps([s], indent=1) + "\n")
        print(json.dumps(s, indent=1))
    if a.plants:
        c = contract()
        binary = RUNS["38031352273"][1]
        for name in PLANTS:
            out, stop, _ = run_in_vm(binary, planted_source(name))
            rows = unequal_rows(c, out)
            print(f"{name}: {len(rows)} rows unequal ({stop}); named reach: {PLANTS[name][2]}; first {rows[:3]}")
        out, stop, _ = run_in_vm(binary, bridge=CaseSensitiveBridge)
        rows = unequal_rows(c, out)
        print(f"case_sensitive_names: {len(rows)} rows unequal ({stop}); first {rows[:3]}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
