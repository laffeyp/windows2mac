# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""A discrepancy report names the program and the state it failed from (Sprint 112): a class case's instruction bytes
and the rest of the case; a probe's binary and what its VM run started from. The hashes are canonical: the same
inputs in another order hash the same, and a changed input does not."""

from __future__ import annotations

import dataclasses
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))

import class_run  # noqa: E402
import family_run  # noqa: E402
import vm_oracle  # noqa: E402
from w2m.record import state_sha256  # noqa: E402


def test_the_state_hash_is_canonical() -> None:
    assert state_sha256({"a": 1, "b": [b"\x01", 2]}) == state_sha256({"b": [b"\x01", 2], "a": 1})
    assert state_sha256({"a": 1}) != state_sha256({"a": 2})


def test_a_case_hashes_its_bytes_and_its_state_apart() -> None:
    _, cases, _ = class_run.cases_for("FCOS")
    a, b = cases[0], cases[1]
    assert class_run.case_hashes(a)[0] == class_run.case_hashes(dataclasses.replace(a, regs=b.regs))[0]
    assert class_run.case_hashes(a)[1] != class_run.case_hashes(dataclasses.replace(a, regs=[r ^ 1 for r in a.regs]))[1]
    assert class_run.case_hashes(a)[1] == class_run.case_hashes(dataclasses.replace(a, label="other"))[1]


def test_an_example_names_its_case() -> None:
    assert vm_oracle.case_of("case 321: 644ed9ff model 3ff") == 321


def test_a_probe_state_is_what_its_run_starts_from() -> None:
    f = family_run.FAMILIES["threads"]
    binary = next(iter(f.runs.values())).binary
    inputs = family_run.vm_inputs(binary, f)
    assert inputs["program"] == binary and inputs["environment"] == list(f.environment)
    assert state_sha256(inputs) != state_sha256({**inputs, "gui": not inputs["gui"]})
