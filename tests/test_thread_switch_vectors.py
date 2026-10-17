# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""A thread switch's vectors (Sprint 068 item B): the monitor's `vswap` saves the outgoing thread's Q0-Q15, FPCR and
FPSR to its slot and loads the incoming thread's, in the same write as the next `set`. Before it, every switch made a
`vget` and a `vset` over the text pipe, 21 µs each (205,614 switches in a 400-million-instruction frontier). The
threads probes switch threads by time slice and by wait; their rows stay Windows', and the monitor sees a `vget` and a
`vset` only for each model step, plus the start's `vset`."""

from __future__ import annotations

import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))

import family_run  # noqa: E402


@pytest.mark.hvf
@pytest.mark.parametrize("family", ["threads", "running_thread_context"])
def test_switches_carry_vectors_without_pipe_round_trips(family: str, monkeypatch: pytest.MonkeyPatch) -> None:
    from w2m import vm_backend
    seen: list[dict[str, int]] = []
    close = vm_backend.VM.close

    def counted_close(vm: vm_backend.VM) -> None:
        seen.append(dict(vm.monitor_commands) | {f"hvc {k}": n for k, n in vm.exits.items()})
        close(vm)
    monkeypatch.setattr(vm_backend.VM, "close", counted_close)
    f = family_run.FAMILIES[family]
    out, stop, _ = family_run.run_in_vm(next(iter(f.runs.values())).binary, f=f)
    assert family_run.unequal_rows(f, out) == [], stop
    commands = seen[-1]                                  # the probe's own VM closes last
    assert commands.get("vswap", 0) > 0, commands
    # only the model's steps (hvc 10, 11) read and write vectors over the pipe; the one `vset` more is the start's
    steps = commands.get("hvc 10", 0) + commands.get("hvc 11", 0)
    assert commands.get("vget", 0) == steps and commands.get("vset", 0) == steps + 1, commands


@pytest.mark.hvf
def test_a_switch_saves_the_outgoing_vectors_and_loads_the_incoming() -> None:
    """Inside a running VM: thread 1 holds A in XMM0-15, thread 2's slot holds B. A switch to 2 puts B in the vCPU and
    A in 1's slot; a switch back puts A in the vCPU. The probes above hold no vector across a switch, so they pass
    with the incoming load dropped; this does not."""
    from w2m import vm_backend
    from w2m.loader import Loaded, Memory
    code_va, stack_va = 0x2000_0000, 0x4000_0000
    mem = Memory()
    mem.map(code_va, 0x1000, "code").data[:8] = bytes.fromhex("c6042500000000 00")   # mov byte [0], 0: the end
    mem.map(stack_va, 0x10000, "stack")
    vm = vm_backend.VM(Loaded(None, mem, code_va, {}, stack_va + 0x10000))   # type: ignore[arg-type]
    a = [(0x1111 * (i + 1)) << 64 | i for i in range(16)]
    b = [(0x2222 * (i + 1)) << 32 | 0xFF00 + i for i in range(16)]
    fpcr_a, fpcr_b = vm_backend.fpcr_for(0x1F80), vm_backend.fpcr_for(0x9FC0)
    seen: dict[str, object] = {}

    def swap(v: vm_backend.VM, x: list[int]) -> None:
        if seen:
            return
        one = v.threads[v.current_tid]
        v.set_vectors(a, fpcr_a, 0)
        v._save_thread(one, x)
        two = v._add_thread(99, one.teb, one.stack)
        two.x, two.monitor, two.state = list(x), one.monitor, one.state
        v._write_vectors(two, b, fpcr_b, 0x10)
        v.current_tid = 99
        v._load_thread(two)
        seen["in two"] = v.vectors()
        seen["one's slot"] = v.thread_vectors(one)
        v._save_thread(two, x)
        v.current_tid = one.tid
        v._load_thread(one)
        seen["back in one"] = v.vectors()
        two.status = "ended"
    try:
        vm.run(max_steps=1 << 20, timeout_ms=10_000, on_dispatch=swap)
    finally:
        vm.close()
    assert seen["in two"] == (b, fpcr_b, 0x10)
    assert seen["one's slot"] == (a, fpcr_a, 0)
    assert seen["back in one"] == (a, fpcr_a, 0)


@pytest.mark.hvf
def test_the_spin_census_counts_pauses_and_each_threads_instructions(monkeypatch: pytest.MonkeyPatch) -> None:
    """Sprint 068 item J. Three PAUSEs in a tiny program leave PAUSE_COUNT at 3; in the threads probe every thread's
    instructions, summed at its switches, add up to the VM's own count."""
    from w2m import vm_backend
    from w2m.loader import Loaded, Memory
    code_va, stack_va = 0x2000_0000, 0x4000_0000
    mem = Memory()
    code = bytes.fromhex("f390 f390 f390 c6042500000000 00")          # pause x3; mov byte [0], 0: the end
    mem.map(code_va, 0x1000, "code").data[:len(code)] = code
    mem.map(stack_va, 0x10000, "stack")
    vm = vm_backend.VM(Loaded(None, mem, code_va, {}, stack_va + 0x10000))   # type: ignore[arg-type]
    try:
        vm.run(max_steps=1 << 20, timeout_ms=10_000)
        assert vm._counts()[1] == 3
    finally:
        vm.close()
    seen: list[tuple[dict[int, tuple[int, int]], int]] = []
    close = vm_backend.VM.close

    def counted_close(v: vm_backend.VM) -> None:
        seen.append((v.thread_census(), v._counts()[0]))
        close(v)
    monkeypatch.setattr(vm_backend.VM, "close", counted_close)
    f = family_run.FAMILIES["threads"]
    family_run.run_in_vm(next(iter(f.runs.values())).binary, f=f)
    census, executed = seen[-1]
    assert len(census) > 1 and sum(n for n, _ in census.values()) == executed, (census, executed)


@pytest.mark.hvf
def test_the_census_outlives_the_vm() -> None:
    """run_frontier reads the census after the VM closes (2026-10-14: a frontier failed on the closed memory map)."""
    from w2m import vm_backend
    from w2m.loader import Loaded, Memory
    code_va, stack_va = 0x2000_0000, 0x4000_0000
    mem = Memory()
    code = bytes.fromhex("f390 c6042500000000 00")
    mem.map(code_va, 0x1000, "code").data[:len(code)] = code
    mem.map(stack_va, 0x10000, "stack")
    vm = vm_backend.VM(Loaded(None, mem, code_va, {}, stack_va + 0x10000))   # type: ignore[arg-type]
    vm.run(max_steps=1 << 20, timeout_ms=10_000)
    vm.close()
    (n, pauses), = vm.thread_census().values()
    assert pauses == 1 and n == 1                              # the PAUSE; the faulting store retires nothing (099)
