"""Sprint 034: specs/guest_state.json against the runner's probes/guest_state output (run
37989842529), with planted faults so a clean result is a result, and the state block's own shape."""

from __future__ import annotations

import copy
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))
import check_guest_state  # noqa: E402

RUN = ROOT / "store" / "probe-runs" / "37989842529"
SPEC = json.loads((ROOT / "specs" / "guest_state.json").read_text())
RUNS = check_guest_state.runs_of(RUN)


def test_spec_conforms_to_three_runner_runs() -> None:
    assert len(RUNS) == 3
    n, diffs = check_guest_state.compare(SPEC, RUNS)
    assert n == 62 and diffs == []


def test_planted_header_offset_is_caught() -> None:
    bad = copy.deepcopy(SPEC)
    bad["windows"]["CONTEXT"]["fields"]["Rip"] = 0xF0
    _, diffs = check_guest_state.compare(bad, RUNS)
    assert diffs == ["CONTEXT.Rip: spec 0xf0, runner 0xf8"]


def test_planted_observed_offset_is_caught() -> None:
    bad = copy.deepcopy(SPEC)
    f = next(f for f in bad["windows"]["TEB"]["fields"] if f["name"] == "LastErrorValue")
    f["offset"] = 0x6C
    _, diffs = check_guest_state.compare(bad, RUNS)
    assert len(diffs) == 3 and all("spec offset 0x6c, check read 0x68" in d for d in diffs)


def test_planted_default_and_failed_run_are_caught() -> None:
    bad = copy.deepcopy(SPEC)
    bad["windows"]["defaults_at_thread_start"]["mxcsr"] = 0x9FC0
    _, diffs = check_guest_state.compare(bad, RUNS)
    assert len(diffs) == 3 and diffs[0].startswith("defaults.mxcsr: spec 0x9fc0")
    broken = copy.deepcopy(RUNS)
    broken[1] = [r if r["kind"] != "summary" else {"kind": "summary", "failures": 1} for r in broken[1]]
    _, diffs = check_guest_state.compare(SPEC, broken)
    assert diffs == ["run 2: summary {'kind': 'summary', 'failures': 1}"]


def test_state_block_fields_fit_without_overlap() -> None:
    sb = SPEC["state_block"]
    spans = sorted((f["offset"], f["offset"] + f["size"], f["field"]) for f in sb["fields"])
    for (a0, a1, a), (b0, _, b) in zip(spans, spans[1:]):
        assert a1 <= b0, f"{a} overlaps {b}"
    assert spans[-1][1] <= sb["size"] and sb["size"] % sb["align"] == 0
    names = {f["field"] for f in sb["fields"]}
    assert {f"xmm{i}" for i in range(16)} | {f"ymm{i}_high" for i in range(16)} <= names


def test_context_map_covers_every_register_the_runner_checked() -> None:
    m = SPEC["context_from_state"]["map"]
    for reg in ("Rax", "Rbx", "Rsi", "Rdi", "R12", "R13", "R14", "R15", "Rip", "EFlags", "MxCsr",
                "FltSave.ControlWord", "Xmm6"):
        assert reg in m, reg
    block = {f["field"] for f in SPEC["state_block"]["fields"]}
    assert set(m.values()) <= block
