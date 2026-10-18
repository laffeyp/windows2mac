# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""The kick (Sprint 117; docs/design-several-vcpus.md, a kick, not a cut): a host thread stops one running vCPU on
demand through its channel's kick word, and vm_monitor.c's kicker calls hv_vcpus_exit for it. vCPU 1 spins on `b .`
from inside vCPU 0's first dispatch miss (tests/test_vcpus.py's harness); vCPU 0 runs an x86 `jmp $` to max_steps."""

from __future__ import annotations

import threading
import time
from typing import Any

import pytest

CODE, STACK = 0x2000_0000, 0x4000_0000
PROGRAM = bytes.fromhex("b801000000" "c6042500000000 00")    # mov eax, 1; mov byte [0], 0: the end
SPIN = 0x14000000                                          # b .
LOOP = bytes.fromhex("ebfe")                               # jmp $


def vm_with(program: bytes, vcpus: int = 1) -> Any:
    from w2m import vm_backend
    from w2m.loader import Loaded, Memory
    mem = Memory()
    mem.map(CODE, 0x1000, "code").data[:len(program)] = program
    mem.map(STACK, 0x10000, "stack")
    vm = vm_backend.VM(Loaded(None, mem, CODE, {}, STACK + 0x10000))   # type: ignore[arg-type]
    vm.vcpu_count = vcpus
    return vm


def spin_runs(monkeypatch: pytest.MonkeyPatch) -> dict[str, tuple[str, float]]:
    """vCPU 1 on `b .`, three runs: kicked 200 ms in (deadline 5 s), kicked before the run (5 s), not kicked (300 ms).
    Each run's kind and seconds."""
    from w2m import vm_backend
    monkeypatch.setattr(vm_backend, "TRANSLATION_CACHE_DIR", None)
    vm = vm_with(PROGRAM, vcpus=2)
    seen: dict[str, tuple[str, float]] = {}

    def on_dispatch(v: Any, x: list[int]) -> None:
        if seen:
            return
        at = v.code_cursor + 0x10000                            # code-cache words no block has reached
        v._write_words(at, [SPIN])
        one = v.vcpus[1]
        assert one.channel is not None

        def run(ms: int) -> tuple[str, float]:
            begun = time.monotonic()
            e, _ = one.channel.run(vm_backend.MONITOR_VA + at, vm_backend.CPSR_EL1H, [0] * 31, ms, None)
            return e.kind, time.monotonic() - begun

        timer = threading.Timer(0.2, one.kick)
        timer.start()
        seen["live"] = run(5_000)
        timer.join()
        one.kick()
        time.sleep(0.05)                                         # the kicker has called hv_vcpus_exit: vCPU 1 is stopped
        seen["between"] = run(5_000)
        seen["none"] = run(300)

    try:
        vm.run(max_steps=1 << 20, timeout_ms=20_000, on_dispatch=on_dispatch)
    finally:
        vm.close()
    return seen


@pytest.mark.hvf
def test_a_kick_stops_a_running_vcpu_and_one_between_runs_is_kept(monkeypatch: pytest.MonkeyPatch) -> None:
    seen = spin_runs(monkeypatch)
    kind, seconds = seen["live"]
    assert kind == "kick" and 0.2 <= seconds < 0.4, seen               # judge 1
    kind, seconds = seen["between"]
    assert kind == "kick" and seconds < 0.1, seen                      # judge 2
    kind, seconds = seen["none"]
    assert kind == "cancel" and 0.3 <= seconds < 0.5, seen             # the plant: the deadline, not a kick


def loop_run(monkeypatch: pytest.MonkeyPatch, kicks: int) -> tuple[int, int, str]:
    """vCPU 0 on `jmp $` to max_steps through steps, kicked `kicks` times 20 ms apart from a host thread once the run
    has its channel: (instructions, VM.kicks, stop)."""
    from w2m import vm_backend
    monkeypatch.setattr(vm_backend, "TRANSLATION_CACHE_DIR", None)
    vm = vm_with(LOOP)
    started = threading.Event()
    failed: list[BaseException] = []

    def on_dispatch(v: Any, x: list[int]) -> None:
        started.set()

    def kicker() -> None:
        try:
            started.wait(10)
            for _ in range(kicks):
                time.sleep(0.02)
                vm.vcpus[0].kick()
        except BaseException as e:                              # noqa: BLE001 - reported after the run
            failed.append(e)
    t = threading.Thread(target=kicker)
    t.start()
    try:
        r = vm.run(max_steps=4_000_000_000, timeout_ms=60_000, on_dispatch=on_dispatch)
        t.join(10)
        return r.instructions_executed, vm.kicks, r.stop
    finally:
        vm.close()
        assert not failed, failed


@pytest.mark.hvf
def test_kicks_lose_no_instruction(monkeypatch: pytest.MonkeyPatch) -> None:
    kicked = loop_run(monkeypatch, 10)
    alone = loop_run(monkeypatch, 0)
    assert kicked[1] == 10 and alone[1] == 0, (kicked, alone)          # judge 3
    assert kicked[2] == alone[2] == "max_steps", (kicked, alone)
    assert kicked[0] == alone[0], (kicked, alone)
