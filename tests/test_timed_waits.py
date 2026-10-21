# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""Every timed wait ends (Sprint 120). Three small guests in tests/data/timed_waits, each built from its .c beside it
with `x86_64-w64-mingw32-gcc -O1 -Wall -Werror -ffreestanding -fno-builtin -fno-stack-protector -nostdlib -e start -o X.exe X.c
-lkernel32` (GCC 16.2.0):

- sleeper: Sleep(20) while a second thread spins, then "woke" and "done". On the wall clock the sleep never ended, on
  one vCPU or two (the run stopped at its limit): only the instruction clock made an expired wait ready.
- mutual: two threads suspend each other at once, 20 rounds; the main thread resumes both. On two vCPUs each host
  waited for the other's thread.
- mutual_context: as mutual, and one thread reads the other's context while it holds it suspended (20 reads). A
  GetThreadContext whose caller is suspended while it waits is retried (threads.c), which a run reaches when
  SuspendThread does not wait (a reach plant: 3 runs of 6, measured).
- twoproc: the program starts itself as a child, passing its counter, and spins polling the child's handle; the child
  reads its counter, sleeps 1 ms and exits 7 if that reading was not behind the parent's. The child never ran: a
  process of one thread kept its whole run's budget once it had started a second process, so it never reached a
  slice's end, where the Machine gives another process its turn. On the instruction clock each process counted only
  its own instructions, so the child started behind its parent and its sleep stood still while the parent spun.
- spinchild: a spinner thread, a child process that sleeps, and the main thread sleeping 20 ms: on two vCPUs vCPU 0
  waits for vCPU 1 while another process exists, which w2m refuses (vCPU 0 drives the Machine).
- qpc, qpc2: QueryPerformanceCounter's value, read by the main thread on vCPU 0 and by a worker on vCPU 1, which on the
  wall clock is the host's time.monotonic count (the host reads the guest's clock to wake expired waits).

Every run here also stops after twice its limit in all (Machine.stop_when): the VM's limit bounds one run of the VM,
which a host thread's wait, or a guest that calls the host often, never reaches."""

from __future__ import annotations

import sys
import tempfile
import time
from collections.abc import Callable
from pathlib import Path
from typing import Any

import pytest

ROOT = Path(__file__).resolve().parent.parent
DATA = ROOT / "tests" / "data" / "timed_waits"
sys.path.insert(0, str(ROOT / "scripts"))

import family_run  # noqa: E402,F401  (w2m on the path)
from w2m.file_bridge import FileBridge  # noqa: E402
from w2m.kernel import Machine, spawner  # noqa: E402
from w2m.kernel32 import Unimplemented, monotonic_count  # noqa: E402
from w2m.loader import load  # noqa: E402

LIMIT_MS = 10_000          # each run fixed here ends in under 3 s (measured); a hang stops at this


def run_guest(name: str, vcpus: int, instruction_clock: bool = False, sources: dict[str, str] | None = None,
              on_close: Callable[[Any], None] | None = None) -> tuple[str, bytes]:
    """A guest of tests/data/timed_waits with its threads on `vcpus` vCPUs: (the stop, its standard output)."""
    from w2m.vm_backend import VM
    clock = family_run.INSTRUCTION_CLOCK if instruction_clock else None
    data = (DATA / f"{name}.exe").read_bytes()
    with tempfile.TemporaryDirectory(prefix="w2m-timed-") as d:
        files = FileBridge(Path(d))
        (Path(d) / "probe.exe").write_bytes(data)              # the program's own file: twoproc starts itself
        vm = VM(load(data, lambda n: None, "probe.exe"), files=files, image_name="probe.exe", command_line="probe.exe",
                runtime_sources=sources, instruction_clock=clock)
        machine = Machine(spawner(files, lambda n: None, sources, instruction_clock=clock), max_steps=1 << 40,
                          timeout_ms=LIMIT_MS)
        ends = time.monotonic() + 2 * LIMIT_MS / 1000
        machine.stop_when = lambda: "the test's limit" if time.monotonic() > ends else None
        try:
            vm.thread_vcpus = vcpus
            r = machine.run(vm)
            if on_close is not None:
                on_close(vm)
        finally:
            vm.close()
            files.close_all()
    return r.stop, r.stdout


@pytest.mark.hvf
@pytest.mark.timeout(60)
@pytest.mark.parametrize("vcpus", [1, 2])
def test_a_sleep_ends_beside_a_spinning_thread(vcpus: int) -> None:
    assert run_guest("sleeper", vcpus) == ("ExitProcess", b"woke\ndone\n")


def expiry_on_the_instruction_clock_only(monkeypatch: pytest.MonkeyPatch) -> dict[str, str]:
    """The plant: the host and threads.c make expired waits ready on the instruction clock only, as before."""
    from w2m import vm_backend
    wake = vm_backend.VM._wake_expired

    def instruction_clock_only(self: Any, vcpu: Any) -> None:
        if self.instruction_clock:
            wake(self, vcpu)
    monkeypatch.setattr(vm_backend.VM, "_wake_expired", instruction_clock_only)
    text = (ROOT / "tools" / "guest_runtime" / "threads.c").read_text()
    fixed = "    if (*monitor_word(W2M_CLOCK_MODE_OFF) == 1) return;"
    assert text.count(fixed) == 1
    return {"threads.c": text.replace(fixed, "    if (*monitor_word(W2M_CLOCK_MODE_OFF) != 2) return;")}


@pytest.mark.hvf
@pytest.mark.timeout(60)
def test_planted_expiry_on_the_instruction_clock_only_never_ends_the_sleep(monkeypatch: pytest.MonkeyPatch) -> None:
    sources = expiry_on_the_instruction_clock_only(monkeypatch)
    assert run_guest("sleeper", 1, sources=sources) == ("timeout", b"")


@pytest.mark.hvf
@pytest.mark.timeout(60)
def test_planted_untimed_host_wait_never_ends_the_sleep(monkeypatch: pytest.MonkeyPatch) -> None:
    """vCPU 0 waits in the host while vCPU 1 runs the spinner, whose run began before the sleep's deadline existed:
    only the host wait's timeout wakes the sleeper. A run reaches that wait only when the threads land so (a race), so
    the plant counts the host waits that had a deadline pending, and the test runs until one did."""
    from w2m import vm_backend
    seconds = vm_backend.VM._seconds_to_deadline
    reached = [0]

    def untimed(self: Any, vcpu: Any) -> None:
        reached[0] += seconds(self, vcpu) is not None
        return None
    monkeypatch.setattr(vm_backend.VM, "_seconds_to_deadline", untimed)
    seen: list[tuple[str, bytes]] = []
    while len(seen) < 30 and not reached[0]:            # a run that misses the wait ends in about 0.1 s
        seen.append(run_guest("sleeper", 2))
    assert reached[0] and seen[-1] == ("timeout", b""), (reached, seen)


@pytest.mark.hvf
@pytest.mark.timeout(60)
@pytest.mark.parametrize("instruction_clock", [False, True])
def test_a_child_sleeps_while_its_parent_spins(instruction_clock: bool) -> None:
    assert run_guest("twoproc", 1, instruction_clock) == ("ExitProcess", b"child ended\n")


@pytest.mark.hvf
@pytest.mark.timeout(60)
def test_planted_child_clock_behind_its_parent(monkeypatch: pytest.MonkeyPatch) -> None:
    """The plant: a process's clock catches up to the machine's on a resume, not at its start."""
    from w2m import vm_backend
    catch_up = vm_backend.VM._catch_up
    started: set[int] = set()

    def not_at_start(self: Any, vcpu: Any) -> None:
        if id(self) in started:
            catch_up(self, vcpu)
        started.add(id(self))
    monkeypatch.setattr(vm_backend.VM, "_catch_up", not_at_start)
    assert run_guest("twoproc", 1, instruction_clock=True) == ("ExitProcess", b"clock behind\n")


@pytest.mark.hvf
@pytest.mark.timeout(60)
def test_planted_whole_budget_never_gives_the_child_a_turn(monkeypatch: pytest.MonkeyPatch) -> None:
    from w2m import vm_backend
    set_budget = vm_backend.VM._set_budget

    def first_only(self: Any, vcpu: Any) -> None:            # the budget set at the run's start, never again
        if not getattr(self, "_planted_budget", False):
            self._planted_budget = True
            set_budget(self, vcpu)
    monkeypatch.setattr(vm_backend.VM, "_set_budget", first_only)
    assert run_guest("twoproc", 1) == ("the test's limit", b"")


RUNS = 5    # a suspend crossed vCPUs in each of 3 runs of 3 (6 times each), measured


def mutual_until_crossed(monkeypatch: pytest.MonkeyPatch, name: str = "mutual") -> list[tuple[str, bytes, int]]:
    """A guest on two vCPUs until a SuspendThread reached a thread the other vCPU ran (at most RUNS runs): each run's
    (stop, output, such suspends)."""
    seen: list[tuple[str, bytes, int]] = []
    while len(seen) < RUNS and not any(r for _, _, r in seen):
        remote = [0]
        stop, out = run_guest(name, 2, on_close=lambda vm: remote.__setitem__(0, vm.remote_suspends))
        seen.append((stop, out, remote[0]))
    return seen


@pytest.mark.hvf
@pytest.mark.timeout(120)
def test_two_threads_suspending_each_other_both_end(monkeypatch: pytest.MonkeyPatch) -> None:
    seen = mutual_until_crossed(monkeypatch)
    assert all(s == "ExitProcess" and o == b"done\n" for s, o, _ in seen), seen
    assert seen[-1][2] > 0, seen


@pytest.mark.hvf
@pytest.mark.timeout(120)
def test_planted_waits_of_sprint_119_hang_on_a_mutual_suspend(monkeypatch: pytest.MonkeyPatch) -> None:
    """The waits as Sprint 119 had them: no notify when a suspend changes what a waiter waits on, and no return when
    the waiter's own thread is suspended."""
    from w2m import vm_backend

    def await_put_down(self: Any, vcpu: Any, t: Any) -> bool:
        self.vcpus[t.vcpu - 1].kick()
        while self._held_elsewhere(vcpu, t):
            if self._stopping is not None:
                raise vm_backend.VcpusStopped(self._stopping)
            self._wait_for_change(vcpu)
        return True

    def stop_elsewhere(self: Any, vcpu: Any, t: Any) -> None:
        if self._held_elsewhere(vcpu, t):
            self.remote_suspends += 1
            self._await_put_down(vcpu, t)
    monkeypatch.setattr(vm_backend.VM, "_await_put_down", await_put_down)
    monkeypatch.setattr(vm_backend.VM, "_stop_elsewhere", stop_elsewhere)
    seen = mutual_until_crossed(monkeypatch)
    assert seen[-1][2] > 0 and "nothing can wake one (a deadlock)" in seen[-1][0], seen


@pytest.mark.hvf
@pytest.mark.timeout(60)
def test_the_guest_counter_is_the_hosts_monotonic_count() -> None:
    before = monotonic_count()
    stop, out = run_guest("qpc", 1)
    after = monotonic_count()
    assert stop == "ExitProcess"
    assert before <= int(out, 16) <= after, (before, out, after)


@pytest.mark.hvf
@pytest.mark.timeout(60)
def test_the_guest_counter_on_vcpu_1_is_the_hosts_monotonic_count(monkeypatch: pytest.MonkeyPatch) -> None:
    """The worker reads the counter and ends within one slice, so the vCPU that serves its end (THREAD_END) read it.
    Which vCPU runs the worker is a race (the main thread ran on vCPU 1 in 1 run of 5, measured): run until it is 1."""
    from w2m import vm_backend
    ends: list[tuple[int, int]] = []
    request = vm_backend.VM._thread_request

    def seen(self: Any, vcpu: Any, sub: int, a0: int, a1: int, a2: int) -> int:
        if sub == vm_backend.THREAD_END:
            ends.append((self.current_tid(vcpu), vcpu.index))
        return request(self, vcpu, sub, a0, a1, a2)
    monkeypatch.setattr(vm_backend.VM, "_thread_request", seen)
    for _ in range(5):
        before = monotonic_count()
        stop, out = run_guest("qpc2", 2)
        after = monotonic_count()
        assert stop == "ExitProcess" and before <= int(out, 16) <= after, (before, out, after)
        if (8, 1) in ends:
            break
    assert (8, 1) in ends, ends


@pytest.mark.hvf
@pytest.mark.timeout(120)
def test_several_vcpus_refuse_to_wait_while_another_process_could_run() -> None:
    stops: list[str] = []
    # reached in 11 runs of 20 on 1953fbfe (measured; the other 9 end with "done"): 5 runs missed it 1 time in 5 test
    # runs, so 30 (a miss about 4e-11 at that rate)
    while len(stops) < 30 and not any("several processes" in s for s in stops):
        stops.append(run_guest("spinchild", 2)[0])
    assert stops[-1].startswith("Unimplemented: a thread waiting on vCPU 0 for another vCPU while another process"), stops


@pytest.mark.hvf
@pytest.mark.timeout(60)
def test_several_vcpus_refuse_the_instruction_clock() -> None:
    with pytest.raises(Unimplemented, match="threads on several vCPUs with the instruction clock"):
        run_guest("sleeper", 2, instruction_clock=True)


@pytest.mark.hvf
@pytest.mark.timeout(120)
def test_threads_suspending_each_other_read_a_context(monkeypatch: pytest.MonkeyPatch) -> None:
    seen = mutual_until_crossed(monkeypatch, "mutual_context")
    assert all(s == "ExitProcess" and o == b"done\n" for s, o, _ in seen), seen
    assert seen[-1][2] > 0, seen


@pytest.mark.hvf
@pytest.mark.timeout(300)
def test_a_context_read_whose_caller_is_suspended_is_retried(monkeypatch: pytest.MonkeyPatch) -> None:
    """The reach plant: SuspendThread does not wait, so a GetThreadContext finds the thread still held and waits, and
    the thread it waits for suspends the reader meanwhile. Each run still reads all 20 contexts and ends."""
    from w2m import vm_backend

    def no_wait(self: Any, vcpu: Any, t: Any) -> None:
        self._notify_waiters()
        if self._held_elsewhere(vcpu, t):
            self.remote_suspends += 1
            self.vcpus[t.vcpu - 1].kick()
    monkeypatch.setattr(vm_backend.VM, "_stop_elsewhere", no_wait)
    retried = [0]
    request = vm_backend.VM._thread_request

    def counted(self: Any, vcpu: Any, sub: int, a0: int, a1: int, a2: int) -> int:
        answer = request(self, vcpu, sub, a0, a1, a2)
        retried[0] += sub == vm_backend.THREAD_CONTEXT and answer == 0
        return answer
    monkeypatch.setattr(vm_backend.VM, "_thread_request", counted)
    seen: list[tuple[str, bytes]] = []
    while len(seen) < 40 and not retried[0]:             # 3 runs of 6 alone, 0 of 10 in the gates; a run 0.2 s
        seen.append(run_guest("mutual_context", 2))
    assert retried[0] and all(r == ("ExitProcess", b"done\n") for r in seen), (retried, seen)
