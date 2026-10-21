# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""A parked vCPU wakes (Sprint 121). Two guests in tests/data/timed_waits, built as the others there:

- readied: a worker waits on an auto-reset event 20 times and does 8,000,000 instructions of work after each, while
  the main thread sets the event inside the VM, spins until the round is done, then a little longer so the worker is
  waiting again. Before, the vCPU that set the event switched to the worker, and the vCPU waiting with no thread
  learned of it only at the next host serve: the rounds ran on the waiting vCPU in 3 runs of 13.
- twoready: two workers, two events, set one after the other; each worker spins after its work until the other has
  done its round, so a readied worker that gets no vCPU leaves every vCPU spinning. One waiting vCPU takes one; the
  other needs a turn on the main thread's vCPU.

threads.c asks the host to wake a waiting vCPU (W2M_THREAD_WAKE) when it readies a thread while one waits
(RT_WAITING); the host ends the readying vCPU's slice only for threads readied beyond the waiting vCPUs
(`_slice_until`)."""

from __future__ import annotations

import struct
from typing import Any

import pytest

from test_timed_waits import ROOT, run_guest

WORK = 20 * 8_000_000 + 241          # the worker's instructions, its 20 rounds and its start and end (measured)


def counts(name: str, sources: dict[str, str] | None = None) -> tuple[str, bytes, list[int], int, int]:
    """A run on two vCPUs: the stop, the output, each vCPU's instructions, the wake requests, the switches in the VM."""
    from w2m.scheduler import RT_SWITCHES
    from w2m.vm_backend import RUN_TABLE
    seen: dict[str, Any] = {}

    def close(vm: Any) -> None:
        seen["by_vcpu"] = [v.counts()[0] for v in vm.vcpus]
        seen["wakes"] = vm.wakes
        seen["switches"] = struct.unpack_from("<Q", vm.mm, RUN_TABLE + RT_SWITCHES)[0]
    stop, out = run_guest(name, 2, sources=sources, on_close=close)
    return stop, out, seen["by_vcpu"], seen["wakes"], seen["switches"]


def planted_slice_rule(monkeypatch: pytest.MonkeyPatch, rule: str) -> None:
    """The host's slice rule with `rule` in place of "> self._takers" (a plant)."""
    import inspect
    import textwrap
    from w2m import vm_backend
    source = textwrap.dedent(inspect.getsource(vm_backend.VM._slice_until))
    assert source.count("> self._takers") == 1
    space: dict[str, Any] = {}
    exec(source.replace("> self._takers", rule), vm_backend.__dict__, space)   # noqa: S102 - a plant of our own code
    monkeypatch.setattr(vm_backend.VM, "_slice_until", space["_slice_until"])


@pytest.mark.hvf
@pytest.mark.timeout(120)
def test_a_thread_readied_in_the_vm_runs_on_the_waiting_vcpu() -> None:
    """All 20 rounds on the waiting vCPU, and vCPU 0 never switches to the worker (10 runs of 10, measured)."""
    for _ in range(5):
        stop, out, by_vcpu, wakes, switches = counts("readied")
        assert (stop, out) == ("ExitProcess", b"done\n")
        assert by_vcpu[1] == WORK and switches == 0 and wakes >= 19, (by_vcpu, wakes, switches)


@pytest.mark.hvf
@pytest.mark.timeout(60)
def test_planted_no_wake_request_leaves_the_readied_worker_behind_a_spinner() -> None:
    """Without the request nothing wakes the waiting vCPU, and the readying vCPU's slice is not ended for it: the
    main thread spins to the limit (5 runs of 5, measured)."""
    text = (ROOT / "tools" / "guest_runtime" / "threads.c").read_text()
    request = "    if (RUN->waiting) host(W2M_THREAD_WAKE, 0, 0, 0);\n"
    assert text.count(request) == 1
    stop, out, by_vcpu, wakes, _ = counts("readied", {"threads.c": text.replace(request, "    if (RUN->waiting) {}\n")})
    assert (stop, out, wakes) == ("timeout", b"", 0), by_vcpu


@pytest.mark.hvf
@pytest.mark.timeout(60)
def test_planted_old_slice_rule_switches_to_the_readied_worker(monkeypatch: pytest.MonkeyPatch) -> None:
    """The rule before: any ready thread ends the slice, so vCPU 0 races the woken vCPU for the worker (193 to 218
    switches in the VM in 3 runs of 3, measured)."""
    planted_slice_rule(monkeypatch, "> 0")
    stop, _, by_vcpu, _, switches = counts("readied")
    assert stop == "ExitProcess" and switches > 0 and by_vcpu[1] != WORK, (by_vcpu, switches)


@pytest.mark.hvf
@pytest.mark.timeout(120)
def test_two_threads_readied_for_one_waiting_vcpu_both_run() -> None:
    for _ in range(5):
        stop, out, by_vcpu, wakes, _ = counts("twoready")
        assert (stop, out) == ("ExitProcess", b"done\n") and wakes >= 19, (by_vcpu, wakes)


@pytest.mark.hvf
@pytest.mark.timeout(60)
def test_planted_waiting_vcpu_too_many_leaves_a_readied_worker_behind(monkeypatch: pytest.MonkeyPatch) -> None:
    """One waiting vCPU counted too many: a readied worker no vCPU takes gets no turn, and every vCPU spins (3 runs of
    3, measured, each before any wake request). A doubled count does not hang it (5 runs of 5): a vCPU that loads a
    worker while the other is ready gets a slice's end of its own and switches between the two."""
    planted_slice_rule(monkeypatch, "> self._takers + 1")
    stop, out, by_vcpu, _, _ = counts("twoready")
    assert (stop, out) == ("timeout", b""), by_vcpu
