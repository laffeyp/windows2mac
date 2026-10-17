# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""The batch run's numbers on its record (vocabulary 0.9, P20; Sprint 108): run_measurements turns the runner's
numbers into MEASUREMENT_RECORDED events, measurement_table reads them back, and measurement_problems finds what does
not add up. The numbers here are a run's shape (two threads, three exit kinds), not a run's values."""

from __future__ import annotations

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))

import run_frontier  # noqa: E402

THREADS = {4: (700, 3), 8: (300, 1)}
ARGS = dict(xid="frontier-x", profile="game-abc", warm="cold", instructions=1000, pauses=4, threads=THREADS,
            run_ns=3_000_000_000, exit_ns={-1: 500_000_000, 0: 100_000_000, 8: 400_000_000}, exits={1: 7, 8: 2},
            switches=(5, {"a thread ready": 2}), wakes=(9, 1), load=[3.5, 2.0, 1.25])


def payloads(**changes: object) -> list[dict[str, object]]:
    import msgspec
    return [msgspec.to_builtins(e) for e in run_frontier.run_measurements(**{**ARGS, **changes})]  # type: ignore[arg-type]


def test_every_number_reads_back_from_the_events() -> None:
    recorded = payloads()
    table = run_frontier.measurement_table(recorded)
    assert len(table) == len(recorded) == len({r["measurement_id"] for r in recorded})
    assert table[("instructions_retired", "process")] == 1000
    assert table[("instructions_retired", "thread 8")] == 300 and table[("pause_instructions", "thread 4")] == 3
    assert table[("run_seconds", "vm")] == 3.0 and table[("run_seconds", "host hvc 8")] == 0.4
    assert table[("run_seconds", "host start")] == 0.5 and table[("run_seconds", "host other")] == 0.1
    assert table[("instructions_per_second", "process")] == 1000 / 4.0
    assert table[("vm_exits", "hvc 1")] == 7 and table[("thread_switches", "host: a thread ready")] == 2
    assert table[("thread_wakes", "rewaits")] == 1 and table[("load_average", "15 min")] == 1.25
    assert run_frontier.measurement_problems(table) == []


def test_planted_census_short_of_a_thread_is_found() -> None:
    table = run_frontier.measurement_table(payloads(threads={4: THREADS[4]}))
    assert run_frontier.measurement_problems(table) == ["instructions_retired: the process 1000.0, its threads 700.0",
                                                        "pause_instructions: the process 4.0, its threads 3.0"]
