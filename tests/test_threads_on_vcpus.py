# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""Guest threads on several vCPUs (Sprint 118; docs/design-several-vcpus.md, chain step 5): the wait_handoff family's
probe, whose threads block on semaphores, events and mutexes and are released, runs with its threads on two vCPUs
and prints Windows' rows; vCPU 1 runs some of its instructions. The plant has the host's switch and park ignore the
RUNNING mark (a run entry's `vcpu` word): vCPU 1 then loads the thread vCPU 0 runs, which _load_thread refuses (the
mark is the witness of one vCPU per thread), stopping the run. Without that refusal the planted vCPU 1 ran the
thread's stale saved state (RIP 0), retired no instruction and went unseen in the rows (Sprint 118, measured)."""

from __future__ import annotations

import sys
from pathlib import Path
from typing import Any

import pytest

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))

import family_run  # noqa: E402

def wait_handoff(monkeypatch: pytest.MonkeyPatch, vcpus: int,
                 sources: dict[str, str] | None = None) -> tuple[str, str, list[int]]:
    """The family's probe on `vcpus` vCPUs: (verdict, stop, instructions by vCPU)."""
    from w2m import vm_backend
    from w2m.compare import compare_exact_rows
    f = family_run.FAMILIES["wait_handoff"]
    run = next(iter(f.runs.values()))
    counts: list[int] = []
    close = vm_backend.VM.close

    def counted_close(v: Any) -> None:
        if v.thread_vcpus == vcpus:
            counts[:] = [u.counts()[0] for u in v.vcpus]
        close(v)
    monkeypatch.setattr(vm_backend.VM, "close", counted_close)
    out, stop, _ = family_run.run_in_vm(run.binary, sources, f, vcpus=vcpus)
    cmp = compare_exact_rows(family_run.contract(f), family_run.derived_output(f, out), "cmp", "ref", "vm")
    return cmp.verdict, stop, counts


@pytest.mark.hvf
def test_threads_run_on_two_vcpus_and_print_windows_rows(monkeypatch: pytest.MonkeyPatch) -> None:
    verdict, stop, counts = wait_handoff(monkeypatch, 2)
    assert verdict == "conforms" and stop == "ExitProcess", (verdict, stop)
    assert len(counts) >= 2 and counts[1] > 0, counts                     # vCPU 1 ran guest instructions


def planted(monkeypatch: pytest.MonkeyPatch) -> tuple[str, str, list[int]]:
    """The host's switch and park ignore the mark: vCPU 1, parked at the start, takes the thread vCPU 0 runs."""
    from w2m import vm_backend
    monkeypatch.setattr(vm_backend.VM, "_held_elsewhere", lambda self, vcpu, t: False)
    return wait_handoff(monkeypatch, 2)


@pytest.mark.hvf
def test_planted_mark_ignored_loads_one_thread_on_two_vcpus(monkeypatch: pytest.MonkeyPatch) -> None:
    verdict, stop, _ = planted(monkeypatch)
    assert "loaded on vCPU 1 while vCPU 0 has it" in stop, (verdict, stop)
