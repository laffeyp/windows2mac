# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""Guest threads on several vCPUs (Sprint 118; docs/design-several-vcpus-2026-10-17.md, chain step 5): the wait_handoff family's
probe, whose threads block on semaphores, events and mutexes and are released, runs with its threads on two vCPUs
and prints Windows' rows; vCPU 1 runs some of its instructions. The plant has the host's switch and park ignore the
RUNNING mark (a run entry's `vcpu` word): vCPU 1 then loads the thread vCPU 0 runs, which _load_thread refuses (the
mark is the witness of one vCPU per thread), stopping the run. Without that refusal the planted vCPU 1 ran the
thread's stale saved state (RIP 0), retired no instruction and went unseen in the rows (Sprint 118, measured).

A thread suspended while another vCPU runs it (Sprint 119, chain step 6): the threads and running_thread_context
families on two vCPUs print Windows' rows, SuspendThread reaching the other vCPU in about 6 runs of 10 (measured), so
each test runs until it has. The plants: SuspendThread not waiting for the other vCPU to put the thread down (the
spinner counts on after it is suspended: suspend.frozen differs; until Sprint 120 the run instead stopped at its limit,
vCPU 0's wait keeping no deadline), and a thread put down without its vectors written to its slot (seen by its witness,
not the rows)."""

from __future__ import annotations

import sys
from pathlib import Path
from typing import Any

import pytest

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))

import family_run  # noqa: E402

def on_vcpus(monkeypatch: pytest.MonkeyPatch, family: str, vcpus: int,
             sources: dict[str, str] | None = None) -> tuple[str, str, list[int], int]:
    """A family's probe (its first reference run) on `vcpus` vCPUs: (verdict, stop, instructions by vCPU, suspends of
    a thread another vCPU ran)."""
    from w2m import vm_backend
    from w2m.compare import compare_exact_rows
    f = family_run.FAMILIES[family]
    run = next(iter(f.runs.values()))
    counts: list[int] = []
    remote = [0]
    close = vm_backend.VM.close

    def counted_close(v: Any) -> None:
        if v.thread_vcpus == vcpus:
            counts[:] = [u.counts()[0] for u in v.vcpus]
            remote[0] = v.remote_suspends
        close(v)
    monkeypatch.setattr(vm_backend.VM, "close", counted_close)
    out, stop, _ = family_run.run_in_vm(run.binary, sources, f, vcpus=vcpus)
    cmp = compare_exact_rows(family_run.contract(f), family_run.derived_output(f, out), "cmp", "ref", "vm")
    return cmp.verdict, stop, counts, remote[0]


def wait_handoff(monkeypatch: pytest.MonkeyPatch, vcpus: int,
                 sources: dict[str, str] | None = None) -> tuple[str, str, list[int]]:
    """The family's probe on `vcpus` vCPUs: (verdict, stop, instructions by vCPU)."""
    verdict, stop, counts, _ = on_vcpus(monkeypatch, "wait_handoff", vcpus, sources)
    return verdict, stop, counts


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


RUNS = 10   # a suspend reached the other vCPU in 12 of 20 threads runs and 9 of 12 running_thread_context runs


def until_remote(monkeypatch: pytest.MonkeyPatch, family: str) -> list[tuple[str, str, int]]:
    """The family on two vCPUs, run until a SuspendThread reached a thread the other vCPU ran (at most RUNS times):
    each run's (verdict, stop, such suspends)."""
    seen: list[tuple[str, str, int]] = []
    while len(seen) < RUNS and not any(r for _, _, r in seen):
        verdict, stop, _, remote = on_vcpus(monkeypatch, family, 2)
        seen.append((verdict, stop, remote))
    return seen


@pytest.mark.hvf
@pytest.mark.timeout(300)                                 # up to RUNS probe runs
@pytest.mark.parametrize("family", ["threads", "running_thread_context"])
def test_thread_suspended_on_another_vcpu_prints_windows_rows(monkeypatch: pytest.MonkeyPatch, family: str) -> None:
    seen = until_remote(monkeypatch, family)
    assert all(v == "conforms" and s == "ExitProcess" for v, s, _ in seen), seen
    assert seen[-1][2] > 0, seen                                         # the suspend crossed vCPUs


@pytest.mark.hvf
@pytest.mark.timeout(300)                     # up to RUNS probe runs
def test_planted_suspend_not_waiting_lets_the_thread_run_on(monkeypatch: pytest.MonkeyPatch) -> None:
    """SuspendThread returns while the other vCPU still runs the thread: it counts on after it is suspended."""
    from w2m import vm_backend

    def no_wait(self: Any, vcpu: Any, t: Any) -> None:
        if self._held_elsewhere(vcpu, t):
            self.remote_suspends += 1
    monkeypatch.setattr(vm_backend.VM, "_stop_elsewhere", no_wait)
    # a crossing run diverged in 8 of 9 (Sprint 120, measured): the spinner, stopped within a slice since Sprint 120,
    # can stop before main reads its count, so the test runs until a crossing run diverges
    seen: list[tuple[str, str, int]] = []
    while len(seen) < RUNS and not any(r and v != "conforms" for v, _, r in seen):
        verdict, stop, _, remote = on_vcpus(monkeypatch, "threads", 2)
        seen.append((verdict, stop, remote))
    assert any(r and v != "conforms" for v, _, r in seen), seen


def pending_slot_reads(monkeypatch: pytest.MonkeyPatch) -> tuple[int, int]:
    """running_thread_context on two vCPUs, three times: GetThreadContext's reads of a thread's vector slot, and those
    made while a vCPU still owed that slot its save (vswap pending). The rows cannot tell: the spinner's vectors match
    any save made after it planted them (the plant conformed 10 runs of 10, Sprint 119, measured)."""
    from w2m import vm_backend
    reads = [0, 0]
    thread_vectors = vm_backend.VM.thread_vectors

    def watched(self: Any, vcpu: Any, t: Any) -> Any:
        if t.tid != self.current_tid(vcpu):
            reads[0] += 1
            reads[1] += any(v.vswap is not None and v.vswap[0] == self._vslot(t) for v in self.vcpus)
        return thread_vectors(self, vcpu, t)
    monkeypatch.setattr(vm_backend.VM, "thread_vectors", watched)
    for _ in range(3):
        on_vcpus(monkeypatch, "running_thread_context", 2)
    return reads[0], reads[1]


@pytest.mark.hvf
@pytest.mark.timeout(120)                                 # three probe runs
def test_context_reads_a_written_slot(monkeypatch: pytest.MonkeyPatch) -> None:
    slot, pending = pending_slot_reads(monkeypatch)
    assert slot > 0 and pending == 0, (slot, pending)


@pytest.mark.hvf
@pytest.mark.timeout(120)
def test_planted_vectors_left_in_the_vcpu_are_read_unwritten(monkeypatch: pytest.MonkeyPatch) -> None:
    """A thread put down with its vectors still in the vCPU: GetThreadContext reads its slot before they reach it."""
    from w2m import vm_backend
    monkeypatch.setattr(vm_backend.VM, "_vectors_to_slot", lambda self, vcpu: None)
    slot, pending = pending_slot_reads(monkeypatch)
    assert pending > 0, (slot, pending)                    # 13 of 18 reads in six runs (measured)
