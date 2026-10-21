# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""Processes beside several vCPUs (Sprint 122). Guests in tests/data/timed_waits, built as the others there:

- spinchild (Sprint 120): a spinner thread, a child process that sleeps for good, and the main thread sleeping 20 ms.
- handshake: a spinner thread, a child process that sets a named event and ends, and the main thread waiting on that
  event. On two vCPUs before, the main thread's vCPU 0 waited in the host for the spinner's vCPU while no other
  process ran: w2m refused the wait (2 runs of 5) or the run spun to its limit (3 of 5).
- busychild: a child process that spins for good; a worker sets an event 20 times and spins until the main thread,
  waiting on it, has taken its round. On one vCPU before, the main thread ran 111 instructions in 20 s: each time the
  child's turn ended, the parent ran again the thread that had been sliced out (the worker), never the readied one.
- crosswake: the child's main thread waits on a named event while a thread of its own spins; the parent's worker, on
  the other vCPU, sets the event and waits for the child's answer, while the parent's main thread waits for the worker.
- pingpong: the parent's worker, on vCPU 1, and the child's main thread, on the Machine's thread, take turns over two
  named events, 2,000 rounds, each wake reaching the other process from a thread that does not serve it.

With its threads on several vCPUs, a process's vCPU 0 hands the Machine on when it has no thread (VM._switch answers
IDLE); the Machine runs another process or waits for a change. A wake from a host thread into another process is left
pending (VM.wake_later) and the process the Machine's thread runs is kicked, so it applies the wake (Machine.wake)."""

from __future__ import annotations

import inspect
import textwrap
from typing import Any

import pytest

from test_timed_waits import run_guest

HAND_ON = "if nxt is None and self.machine is not None and self.lock.shared:"


def planted_waiting_vcpu0(monkeypatch: pytest.MonkeyPatch) -> None:
    """The plant: vCPU 0 waits in `_switch` for another vCPU as before Sprint 122, with no refusal."""
    from w2m import vm_backend
    source = textwrap.dedent(inspect.getsource(vm_backend.VM._switch))
    assert source.count(HAND_ON) == 1
    space: dict[str, Any] = {}
    exec(source.replace(HAND_ON, "if False:"), vm_backend.__dict__, space)   # noqa: S102 - a plant of our own code
    monkeypatch.setattr(vm_backend.VM, "_switch", space["_switch"])


@pytest.mark.hvf
@pytest.mark.timeout(240)
@pytest.mark.parametrize("name", ["spinchild", "handshake", "busychild", "crosswake", "pingpong"])
@pytest.mark.parametrize("vcpus", [1, 2])
def test_a_process_beside_threads_on_several_vcpus_ends(name: str, vcpus: int) -> None:
    for _ in range(5 if vcpus > 1 else 1):
        assert run_guest(name, vcpus) == ("ExitProcess", b"done\n")


@pytest.mark.hvf
@pytest.mark.timeout(400)
def test_planted_vcpu0_waiting_for_another_vcpu_keeps_the_child_from_running(monkeypatch: pytest.MonkeyPatch) -> None:
    """vCPU 0 waits for the spinner's vCPU, and the child that would set the event never runs. A run reaches that
    wait only when the main thread keeps vCPU 0 and the spinner lands on vCPU 1 (a race; when the spinner lands on vCPU
    0, the child's start kicks it and the child runs; 11 runs of 20 do not end, measured), so the test runs until one
    does not end (20 tries)."""
    planted_waiting_vcpu0(monkeypatch)
    seen: list[tuple[str, bytes]] = []
    while len(seen) < 20 and (not seen or seen[-1] == ("ExitProcess", b"done\n")):
        seen.append(run_guest("handshake", 2))
    assert seen[-1][1] == b"" and seen[-1][0] in ("timeout", "the test's limit"), seen


@pytest.mark.hvf
@pytest.mark.timeout(60)
def test_planted_no_kick_leaves_the_childs_wake_pending(monkeypatch: pytest.MonkeyPatch) -> None:
    """Without the kick, the child runs its spinner on with no slice end, and its main thread never learns of the
    parent's wake."""
    from w2m import vm_backend
    monkeypatch.setattr(vm_backend.VM, "kick_vcpu0", lambda self: None)
    stop, out = run_guest("crosswake", 2)
    assert out == b"" and stop in ("timeout", "child process 4100: timeout", "the test's limit"), stop


@pytest.mark.hvf
@pytest.mark.timeout(60)
def test_planted_sliced_out_thread_first_starves_the_readied_one(monkeypatch: pytest.MonkeyPatch) -> None:
    """After another process's turn, the thread sliced out runs again, as before: on one vCPU busychild's main thread
    never runs."""
    from w2m import vm_backend
    source = textwrap.dedent(inspect.getsource(vm_backend.VM.steps))
    turn = "self._switch(vcpu, x, self.threads[self.current_tid(vcpu)], 0, saved=True)"
    assert source.count(turn) == 1
    space: dict[str, Any] = {}
    exec(source.replace(turn, "self._load_thread(vcpu, self.threads[self.current_tid(vcpu)])"),   # noqa: S102
         vm_backend.__dict__, space)
    monkeypatch.setattr(vm_backend.VM, "steps", space["steps"])
    stop, out = run_guest("busychild", 1)
    assert out == b"" and stop in ("timeout", "the test's limit"), stop


@pytest.mark.hvf
@pytest.mark.timeout(400)
def test_planted_kick_into_other_processes_only_leaves_a_new_child_unrun(monkeypatch: pytest.MonkeyPatch) -> None:
    """The kick as first written: only a wake into another process than the requester's kicks. When the spinner lands
    on vCPU 0 and the main thread starts the child from vCPU 1, vCPU 0 spins on with no slice end and the child never
    runs (30 runs of 60 before the kick was widened; 8 of 20 under this plant, measured), so the test runs until one
    does not end (20 tries: a miss about 4e-5 at that rate)."""
    from w2m import kernel
    source = textwrap.dedent(inspect.getsource(kernel.Machine.wake))
    rule = "threading.current_thread() is not self._thread"
    assert source.count(rule) == 1
    space: dict[str, Any] = {}
    exec(source.replace(rule, "running.vm is not by"), kernel.__dict__, space)   # noqa: S102 - a plant of our own code
    monkeypatch.setattr(kernel.Machine, "wake", space["wake"])
    seen: list[tuple[str, bytes]] = []
    while len(seen) < 20 and (not seen or seen[-1] == ("ExitProcess", b"done\n")):
        seen.append(run_guest("handshake", 2))
    assert seen[-1] == ("timeout", b""), seen
