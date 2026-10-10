"""Sprint 042 grading surface: CreateFileW and its companions, judged by Windows's own rows.

The contract is the runners' output (probes/create_file on Server 2025 and Server 2022, equal byte
for byte); the probe's own runner binaries run in the managed VM over tools/guest_runtime/file.c
and src/w2m/file_bridge.py; each planted fault changes exactly the rows named for it beforehand.
"""

from __future__ import annotations

import json
import sys
from collections import Counter
from itertools import product
from pathlib import Path

import pytest
from substrate.api import read_record, trace_ancestry

from w2m.compare import compare_exact_rows
from w2m.contract import ExactRows

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))
import file_run  # noqa: E402


def _rows() -> tuple[dict[str, object], ...]:
    a = file_run.contract().acceptance
    assert isinstance(a, ExactRows)
    return a.rows


def test_contract_is_the_runners_output() -> None:
    """The committed contract is what --contract builds from the stored runs, and its rows are
    every runner execution's output, line for line."""
    assert json.loads(file_run.CONTRACT.read_text()) == file_run.build_contract()
    rows = _rows()
    assert len(rows) == 188
    for run in file_run.RUNS:
        for i in (1, 2):
            body = (ROOT / "store" / "probe-runs" / run / f"body-{i}.jsonl").read_bytes()
            assert [json.loads(line) for line in body.splitlines()] == list(rows)


def test_windows_sharing_rows_follow_the_documented_rule() -> None:
    """The sharing plant's reach (35) is counted from the documented rule; the 144 recorded pairs
    follow that rule exactly, so the count holds for the record too."""
    access = {"r": {"R"}, "w": {"W"}, "rw": {"R", "W"}}
    share = {"0": set(), "r": {"R"}, "w": {"W"}, "rw": {"R", "W"}}
    share_rows = {r["s"]: r for r in _rows() if str(r["s"]).startswith("share.")}
    assert len(share_rows) == 144
    reverse_dropped = 0
    for a1, s1, a2, s2 in product(access, share, access, share):
        rule = access[a2] <= share[s1] and access[a1] <= share[s2]
        row = share_rows[f"share.{a1}.{s1}.{a2}.{s2}"]
        assert row["first"] is True and row["valid"] is rule and row["err"] == (0 if rule else 32)
        reverse_dropped += rule != (access[a2] <= share[s1])
    assert reverse_dropped == 35


def test_comparator_names_the_first_unequal_row() -> None:
    c = file_run.contract()
    good = b"".join(json.dumps(r, separators=(",", ":")).encode() + b"\n" for r in _rows())
    assert compare_exact_rows(c, good, "c", "r", "t").verdict == "conforms"
    bad = good.replace(b'"s":"disp.open_always.existing","valid":true,"err":183',
                       b'"s":"disp.open_always.existing","valid":true,"err":0')
    out = compare_exact_rows(c, bad, "c", "r", "t")
    assert out.verdict == "discrepancy" and out.relation == "exact"
    assert out.first_divergence == "row disp.open_always.existing: err reference 183, target 0"
    assert compare_exact_rows(c, good.rsplit(b"\n", 2)[0] + b"\n", "c", "r", "t").verdict == "discrepancy"


@pytest.mark.hvf
@pytest.mark.parametrize("run", sorted(file_run.RUNS))
def test_runner_binary_in_the_vm_prints_windows_rows(run: str) -> None:
    out, stop, _ = file_run.run_in_vm(file_run.RUNS[run][1])
    assert stop == "ExitProcess"
    assert file_run.unequal_rows(file_run.contract(), out) == []


@pytest.mark.hvf
@pytest.mark.parametrize("name, rows", [
    ("sharing_without_reverse_check", 35),
    ("already_exists_dropped", ["disp.create_always.existing", "disp.open_always.existing"]),
    ("trailing_dot_kept", ["name.trailing_dot", "name.trailing_space"]),
])
def test_planted_fault_changes_exactly_its_named_rows(name: str, rows: int | list[str]) -> None:
    out, stop, _ = file_run.run_in_vm(file_run.RUNS["38031352273"][1], file_run.planted_source(name))
    assert stop == "ExitProcess"
    got = file_run.unequal_rows(file_run.contract(), out)
    if isinstance(rows, int):
        assert len(got) == rows and all(r.startswith("share.") for r in got)
    else:
        assert got == rows


@pytest.mark.hvf
def test_case_sensitive_names_stop_at_the_case_scenario() -> None:
    out, stop, _ = file_run.run_in_vm(file_run.RUNS["38031352273"][1], bridge=file_run.CaseSensitiveBridge)
    assert stop.startswith("Unimplemented: CreateFileW")
    assert out == b""                                   # the probe prints at exit; nothing reached it


def test_committed_item_run_accepts_file_c_once() -> None:
    (s,) = json.loads((ROOT / "store" / "runs" / "file-runs.json").read_text())
    import hashlib
    assert s["candidate_sha256"] == "sha256:" + hashlib.sha256(file_run.FILE_C.read_bytes()).hexdigest(), \
        "file.c changed since its item run"
    evs = list(read_record(ROOT / s["record"]))
    kinds = Counter(e["kind"] for e in evs)
    assert kinds["COMPARISON_COMPLETED"] == 2 and kinds["CANDIDATE_ACCEPTED"] == 1
    assert "DISCREPANCY_REPORTED" not in kinds
    acc = next(e for e in evs if e["kind"] == "CANDIDATE_ACCEPTED")
    chain = [x.kind for x in trace_ancestry(ROOT / s["record"], acc["producer"]["instance"])]
    assert chain[0] == "acceptance" and chain[-1] == "harness" and "implementation_source" in chain


def test_runtime_missing_a_service_is_refused_at_build() -> None:
    """Sprint 042 defect: a file.c without k32_SetFilePointerEx linked (a preload image resolves
    nothing) and the VM then failed with a bare KeyError while binding imports."""
    from w2m import guest_runtime
    from w2m.vm_backend import build_runtime
    text = file_run.FILE_C.read_text()
    start = text.index("/* SetFilePointerEx(")
    with pytest.raises(guest_runtime.BuildError, match="does not define k32_SetFilePointerEx"):
        build_runtime(None, text[:start])


@pytest.mark.hvf
def test_path_with_a_nul_stops_the_run_by_name() -> None:
    """Sprint 042 defect: a file.c that passed a length past the terminator handed the bridge a
    path with a NUL, and os.open raised ValueError out of the run."""
    text = file_run.FILE_C.read_text().replace("return w2m_host(W2M_FILE_LOOKUP, (u64)path, chars, 0, 0, id);",
                                               "return w2m_host(W2M_FILE_LOOKUP, (u64)path, chars + 1, 0, 0, id);")
    out, stop, _ = file_run.run_in_vm(file_run.RUNS["38031352273"][1], text)
    assert stop.startswith("BridgeRequestError: path of 13 characters has a NUL at character 12")
