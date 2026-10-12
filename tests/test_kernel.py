# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""The Machine's shared objects, with stand-in VMs (Sprint 058, the review of Sprint 055). Each test names the
defect it checks; each failed on d5ecffe8's kernel.py."""

from __future__ import annotations

import struct
from collections.abc import Callable, Generator
from dataclasses import dataclass
from typing import Any

from w2m.kernel import CLOSE, EVENT_OPEN, PIPE_CREATE, PIPE_READ, PIPE_WRITE, PROCESS_CREATE, Machine
from w2m.kernel32 import Unimplemented

KIND_PIPE_WRITE, KIND_DIRECTORY = 11, 102
LINE, LIST, NAME, BUFFER = 0x100, 0x1000, 0x2000, 0x8000


@dataclass
class Result:
    stop: str = "ExitProcess"


class StandIn:
    """A VM as the Machine sees one: memory, an environment, and a program run as VM.steps runs."""

    def __init__(self, pid: int, program: Callable[[StandIn], Generator[str, None, None]] | None = None,
                 environment: tuple[str, ...] = ("PATH=C:\\Windows",)) -> None:
        self.process_id = pid
        self.program = program
        self.environment = environment
        self.mem = bytearray(0x20000)
        self.machine: Any = None
        self.exit_code = 0
        self.unimplemented = None
        self.seen: dict[str, Any] = {}

    def read_text(self, va: int, chars: int) -> str:
        return bytes(self.mem[va:va + 2 * chars]).decode("utf-16-le")

    def read_bytes(self, va: int, n: int) -> bytes:
        return bytes(self.mem[va:va + n])

    def write_bytes(self, va: int, data: bytes) -> None:
        self.mem[va:va + len(data)] = data

    def environment_now(self) -> tuple[str, ...]:
        return self.environment

    def steps(self, **_: Any) -> Generator[str, None, Result]:
        try:
            if self.program is not None:
                yield from self.program(self)
        except Unimplemented as e:                      # as VM.steps stops on one
            return Result(f"Unimplemented: {e}")
        return Result()

    def ready(self) -> bool:
        return not self.seen.get("waits")

    def wake_all(self) -> None:
        pass

    def waits_with_deadline(self) -> bool:
        return False

    def ask(self, *a: int) -> tuple[int, int]:
        answer: tuple[int, int] = self.machine.request(self, list(a) + [0] * (4 - len(a)))
        return answer

    def start_child(self, entries: list[tuple[int, int, int]], directory: str = "C:\\w2m\\app") -> tuple[int, int]:
        line = "child".encode("utf-16-le")
        self.mem[LINE:LINE + len(line)] = line
        text = directory.encode("utf-16-le")
        self.mem[NAME:NAME + len(text)] = text
        listed = [*entries, (len(directory), KIND_DIRECTORY, NAME)]
        for i, e in enumerate(listed):
            struct.pack_into("<QQQ", self.mem, LIST + 24 * i, *e)
        return self.ask(PROCESS_CREATE, LINE, 5 | len(listed) << 32, LIST)


def machine(children: dict[str, Any]) -> Machine:
    def spawn(line: str, pid: int, tid: int, inherited: list[tuple[int, int, int]], *rest: Any) -> StandIn:
        children["inherited"] = inherited
        children["rest"] = rest
        return StandIn(pid)
    return Machine(spawn)


def test_a_pipe_whose_writer_process_ended_is_broken() -> None:
    """Defect 1: the child ended holding the pipe's last write end, and the parent's read waited forever (status 1).
    Windows closes an ending process's handles: the read answers ERROR_BROKEN_PIPE (row writer.pipe)."""
    children: dict[str, Any] = {}
    m = machine(children)

    def parent(vm: StandIn) -> Generator[str, None, None]:
        r, w = vm.ask(PIPE_CREATE)
        vm.start_child([(8, KIND_PIPE_WRITE, w)])
        vm.ask(CLOSE, w)
        yield "idle"                                   # the child runs and ends
        vm.seen["read"] = vm.ask(PIPE_READ, r, BUFFER, 8)

    p = StandIn(4096, parent)
    m.run(p)
    assert p.seen["read"] == (0, 2)


def test_object_ids_are_not_process_ids() -> None:
    """Defect 5: objects were numbered from 1 and processes from 4100, and _ref and _close looked both up by the
    number: the 4100th object's handle closed the child's process instead."""
    m = machine({})
    p = StandIn(4096)
    m.add(p)
    child, _ = p.start_child([])
    ids = []
    for i in range(child + 4):
        name = f"e{i}".encode("utf-16-le")
        p.mem[NAME:NAME + len(name)] = name
        ids.append(p.ask(EVENT_OPEN, NAME, len(name) // 2, 0)[0])
    assert not set(ids) & set(m.processes)


def test_a_child_gets_the_parents_environment_and_directory_as_they_are_now() -> None:
    """Defects 2 and 3: the spawner gave the child the environment the run started with and the program's directory.
    CreateProcessW with no environment and no directory gives the parent's current ones (rows writer.environment,
    writer.directory)."""
    children: dict[str, Any] = {}
    m = machine(children)
    p = StandIn(4096, environment=("PATH=C:\\Windows", "W2M_PROBE_VAR=from_parent"))
    m.add(p)
    p.start_child([], directory="C:\\w2m\\app\\w2m_probe_dir")
    assert children["rest"] == (("PATH=C:\\Windows", "W2M_PROBE_VAR=from_parent"), "C:\\w2m\\app\\w2m_probe_dir")
    assert all(k != KIND_DIRECTORY for _, k, _ in children["inherited"])


def test_a_pipe_holds_4096_bytes() -> None:
    """Defect 6: the pipe took every write whole. Windows's holds 4096 bytes for a size of 0 (row pipe.capacity): a
    write takes what fits, and the writer waits for the rest."""
    m = machine({})
    p = StandIn(4096)
    m.add(p)
    r, w = p.ask(PIPE_CREATE)
    assert p.ask(PIPE_WRITE, w, BUFFER, 10000)[0] == 4096
    assert p.ask(PIPE_WRITE, w, BUFFER, 10)[0] == 0
    assert p.ask(PIPE_READ, r, BUFFER, 1000)[0] == 1000
    assert p.ask(PIPE_WRITE, w, BUFFER, 5904)[0] == 1000


def test_a_deadlock_stops_the_main_run_with_its_output() -> None:
    """Found by a plant run in Sprint 058: when every thread waited, Machine.run raised, so the run's rows were lost and
    the plant run crashed. The main VM's run stops there, as it stops on any Unimplemented."""
    m = machine({})

    def waits(vm: StandIn) -> Generator[str, None, None]:
        vm.seen["waits"] = True
        yield "idle"

    p = StandIn(4096, waits)
    r = m.run(p)
    assert r.stop == "Unimplemented: every thread of every process waits with no timeout (a deadlock)"
