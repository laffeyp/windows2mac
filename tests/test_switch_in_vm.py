# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""Thread switches inside the VM (Sprint 069, recentering step 5): threads.c w2m_switch makes the switch the host's
_switch would make, at the same count. Each threaded family runs on the instruction clock, so its interleaving is a
function of the program, with SWITCH_IN_VM on and off: the rows are the same, and so is each thread's count of
instructions and PAUSEs, while the hand-overs that leave the VM (hvc 12) fall. A plant in the VM's rule changes a
census or a row; the VM's warp to the nearest deadline (Sprint 070) keeps both. (A plant that starts the rotation one thread late is caught too, by a hang: two threads
never switch.)"""

from __future__ import annotations

import dataclasses
import sys
from pathlib import Path
from typing import Any

import pytest

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))

import family_run  # noqa: E402

FAMILIES = ["threads", "running_thread_context", "wait_handoff", "thread_stacks", "late_tls", "processes"]   # probes with threads


def run(name: str, in_vm: bool, monkeypatch: pytest.MonkeyPatch,
        sources: dict[str, str] | None = None) -> tuple[bytes, str, dict[int, tuple[int, int]], dict[int, int], int]:
    """The family's probe on the instruction clock: its rows, its stop, the probe VM's thread census, exits by kind
    and switches made in the VM."""
    out, stop, census, exits, switches, _ = run_counted(name, in_vm, monkeypatch, sources)
    return out, stop, census, exits, switches


def run_counted(name: str, in_vm: bool, monkeypatch: pytest.MonkeyPatch, sources: dict[str, str] | None = None
                ) -> tuple[bytes, str, dict[int, tuple[int, int]], dict[int, int], int, dict[str, int]]:
    """run's, and the hand-overs the host made by cause."""
    from w2m import vm_backend
    seen: list[tuple[dict[int, tuple[int, int]], dict[int, int], int, dict[str, int]]] = []
    close = vm_backend.VM.close

    def counted_close(vm: Any) -> None:
        seen.append((vm.thread_census(), dict(vm.exits), vm.switches_in_vm(), dict(vm.handovers)))
        close(vm)
    monkeypatch.setattr(vm_backend.VM, "close", counted_close)
    monkeypatch.setattr(vm_backend, "SWITCH_IN_VM", in_vm)
    f = dataclasses.replace(family_run.FAMILIES[name], instruction_clock=True)
    out, stop, _ = family_run.run_in_vm(next(iter(f.runs.values())).binary, sources, f=f)
    census, exits, switches, handovers = seen[-1]        # the probe's own VM closes last
    return out, stop, census, exits, switches, handovers


@pytest.mark.hvf
@pytest.mark.parametrize("name", FAMILIES)
def test_the_vm_switches_as_the_host_does(name: str, monkeypatch: pytest.MonkeyPatch) -> None:
    out_host, stop_host, census_host, exits_host, switches_host = run(name, False, monkeypatch)
    out_vm, stop_vm, census_vm, exits_vm, switches_vm = run(name, True, monkeypatch)
    assert (out_vm, stop_vm) == (out_host, stop_host)
    assert census_vm == census_host
    assert switches_host == 0 and len(census_vm) > 1
    assert switches_vm > 0 and exits_vm.get(12, 0) < exits_host.get(12, 0), (exits_host, exits_vm, switches_vm)


WARP_OFF = ("if (!retry || *monitor_word(W2M_CLOCK_MODE_OFF) != 2) return 0;", "return 0;")


@pytest.mark.hvf
@pytest.mark.parametrize("name", ["threads", "running_thread_context", "wait_handoff", "processes"])
def test_the_vm_warps_as_the_host_does(name: str, monkeypatch: pytest.MonkeyPatch) -> None:
    """Sprint 070: a wait that leaves no thread ready warps the instruction clock in the VM (threads.c
    nearest_deadline) as the host's warp_to_deadline does: with the VM's warp taken out, the host warps instead, and
    the rows and census are the same while the host makes more hand-overs with no thread ready (the reach check:
    these four families wait with no thread ready; processes keeps some beside its child process)."""
    text = (family_run.GUEST / "threads.c").read_text()
    assert text.count(WARP_OFF[0]) == 1
    out, stop, census, _, _, handovers = run_counted(name, True, monkeypatch)
    out_host, stop_host, census_host, _, _, handovers_host = run_counted(
        name, True, monkeypatch, {"threads.c": text.replace(*WARP_OFF)})
    assert (out, stop, census) == (out_host, stop_host, census_host)
    assert handovers.get("no thread ready", 0) < handovers_host.get("no thread ready", 0), (handovers, handovers_host)


PLANTS = {
    "a suspended thread switched to": ("e->status == READY && !e->suspend && !held_elsewhere(e)) next = i;",
                                       "e->status == READY && !held_elsewhere(e)) next = i;"),
    "a slice end keeps its block's count": ("        *count -= stop_block_length(stop);",
                                            "        *count -= 0 * stop_block_length(stop);"),
}


@pytest.mark.hvf
@pytest.mark.parametrize("plant", PLANTS)
def test_a_planted_rule_changes_the_census(plant: str, monkeypatch: pytest.MonkeyPatch) -> None:
    old, new = PLANTS[plant]
    text = (family_run.GUEST / "threads.c").read_text()
    assert text.count(old) == 1
    out_host, _, census_host, _, _ = run("threads", False, monkeypatch)
    out, stop, census, _, _ = run("threads", True, monkeypatch, {"threads.c": text.replace(old, new)})
    assert (out, census) != (out_host, census_host), (plant, stop)
