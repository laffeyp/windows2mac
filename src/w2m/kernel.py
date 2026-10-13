# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""kernel.py — what Windows processes share, and the machine that runs several (Sprint 055).

Each guest process is a VM of its own (vm_backend.VM, one vm_monitor each: Hypervisor.framework allows one VM per
process, and each monitor is a process). What processes share lives here, in the host, as wineserver holds it for
Wine (BLACKBOARD Decision 2026-10-11): named events, pipes, and a record per process. The guest runtime reaches it
with host request HOST_KERNEL (tools/guest_runtime/processes.c); the Windows rules (error codes, which handle is
which) stay in the guest.

The Machine runs its VMs on the one Mac thread: a VM runs until none of its threads can run ("idle") or its time
slice ends while another VM has a ready thread ("slice"), then the next VM with a ready thread runs. A shared object
signaled in one VM makes every blocked thread in every VM check its wait again, as a signal inside one VM does
(threads.c). When no VM has a ready thread, one with a timed wait runs after a millisecond, or the run stops: every
thread of every process waits forever.

A process holds handles on the objects here; when it ends, its handles close, as Windows closes an ending process's
handles: a pipe whose last writer ends is broken for its reader (Sprint 058). Object ids start at OBJECT_IDS, above
every process id, so a handle's id names one or the other.

Requests (a0 the sub-operation; the rest by sub-operation):
  EVENT_OPEN   (name, chars, manual | initial << 1) -> object id; second: 1 when the name existed
  EVENT_STATE  (id) -> 1 signaled, 0 not        EVENT_TAKE (id)   EVENT_SET (id)   EVENT_RESET (id)
  CLOSE        (id) -> 0: one handle fewer on an event, a pipe end, a process
  PIPE_CREATE  () -> the read end's id; second: the write end's id
  PIPE_READ    (id, buffer, n) -> bytes read; second: 0 data, 1 empty (the reader blocks), 2 broken (no writer)
  PIPE_WRITE   (id, buffer, n) -> bytes the pipe took: what fits in PIPE_CAPACITY (the writer waits for the rest)
  PROCESS_CREATE (command line, chars | count << 32, inherited) -> the child's process id; second: its first thread's
               id. inherited: count entries of (handle, kind, object id), then the three standard handles as
               (which, KIND_STANDARD, handle), the error mode (0, KIND_ERROR_MODE, mode) and the current directory
               (characters, KIND_DIRECTORY, address); each object gains a handle, which the child holds. The child's
               environment is the parent's block as it is now (Sprint 058), its process group the parent's (059)
  PROCESS_STATE (id) -> STILL_ACTIVE (259) while it runs, its exit code after; ~0 for no such process
  REF          (id) -> 0: one handle more (OpenProcess, inheritance)
"""

from __future__ import annotations

import struct
import time
from collections import Counter
from collections.abc import Callable, Generator
from dataclasses import dataclass, field
from typing import Any

from w2m.kernel32 import Unimplemented

EVENT_OPEN, EVENT_STATE, EVENT_TAKE, EVENT_SET, EVENT_RESET, CLOSE = 1, 2, 3, 4, 5, 6
PIPE_CREATE, PIPE_READ, PIPE_WRITE = 7, 8, 9
PROCESS_CREATE, PROCESS_STATE, REF = 10, 11, 12
STILL_ACTIVE = 259
M64 = (1 << 64) - 1
# handle kinds in PROCESS_CREATE's inherited list (processes.c)
KIND_PROCESS, KIND_SHARED_EVENT, KIND_PIPE_READ, KIND_PIPE_WRITE, KIND_REMOTE_THREAD = 7, 9, 10, 11, 12   # runtime.h
KIND_STANDARD = 100                       # W2M_INHERIT_STANDARD: a standard handle, not an object
KIND_ERROR_MODE = 101                     # W2M_INHERIT_ERROR_MODE: the parent's error mode
KIND_DIRECTORY = 102                      # W2M_INHERIT_DIRECTORY: the parent's current directory (Sprint 058)
OBJECT_IDS = 1 << 32                      # the first object id: above every process id (Sprint 058)
PIPE_CAPACITY = 4096                      # a pipe made with size 0 (rows pipe.capacity, pipe.large_write; Sprint 058)


@dataclass
class Event:
    name: str
    manual: bool
    signaled: bool
    handles: int = 1


@dataclass
class Pipe:
    data: bytearray = field(default_factory=bytearray)
    readers: int = 1
    writers: int = 1


@dataclass
class Process:
    pid: int
    vm: Any
    steps: Generator[str, None, Any] | None = None
    exit_code: int = STILL_ACTIVE
    handles: int = 0
    result: Any = None
    held: Counter[int] = field(default_factory=Counter)   # object or process id -> the handles this process holds


class Machine:
    """The VMs of one run and the objects they share. `spawn(command_line, inherited)` builds a child process's VM
    (the driver supplies it: which files, which environment); `run_args` are VM.steps' arguments for every VM."""

    def __init__(self, spawn: Callable[[str, int, int, list[tuple[int, int, int]], tuple[str, ...], str, int], Any],
                 **run_args: Any) -> None:
        self.spawn = spawn
        self.run_args = run_args
        self.vms: list[Any] = []
        self.processes: dict[int, Process] = {}
        self.objects: dict[int, Event | Pipe] = {}
        self.pipe_ends: dict[int, tuple[int, bool]] = {}      # an end's id -> the pipe's id, whether it reads
        self.names: dict[str, int] = {}
        self.next_id = OBJECT_IDS
        self.next_pid = 4096
        self.next_tid = 4
        self.main: Any = None
        # a reason to stop the main process at its next exit (a signal), and a test of the run asked at each progress
        # line (vm_backend.PROGRESS_SECONDS) that names a reason to stop or None; either ends the run with its record
        self.stop_requested: str | None = None
        self.stop_when: Callable[[], str | None] | None = None

    def _new(self, obj: Event | Pipe) -> int:
        oid = self.next_id
        self.next_id += 1
        self.objects[oid] = obj
        return oid

    def add(self, vm: Any) -> Process:
        """A VM joins the machine: its process id is the one it was built with (vm.process_id)."""
        vm.machine = self
        p = Process(vm.process_id, vm)
        self.processes[vm.process_id] = p
        self.vms.append(vm)
        return p

    def allocate_ids(self) -> tuple[int, int]:
        """A new process's id and its first thread's, as Windows numbers them: multiples of 4, never reused here."""
        pid = max(self.processes, default=self.next_pid - 4) + 4
        self.next_tid += 0x100000                  # each process numbers its threads from here up in steps of 4
        tid = self.next_tid
        return pid, tid

    # ---- the shared objects ----------------------------------------------------------------------------------

    def wake(self) -> None:
        for vm in self.vms:
            vm.wake_all()

    def request(self, vm: Any, a: list[int]) -> tuple[int, int]:
        sub = a[0]
        held = self.processes[vm.process_id].held
        if sub == EVENT_OPEN:
            name = vm.read_text(a[1], a[2])
            if name in self.names:
                ev = self.objects[self.names[name]]
                assert isinstance(ev, Event)
                ev.handles += 1
                held[self.names[name]] += 1
                return self.names[name], 1
            oid = self._new(Event(name, bool(a[3] & 1), bool(a[3] & 2)))
            self.names[name] = oid
            held[oid] += 1
            return oid, 0
        if sub in (EVENT_STATE, EVENT_TAKE, EVENT_SET, EVENT_RESET):
            ev = self.objects[a[1]]
            assert isinstance(ev, Event)
            if sub == EVENT_STATE:
                return int(ev.signaled), 0
            if sub == EVENT_TAKE and not ev.manual:
                ev.signaled = False
            elif sub == EVENT_SET:
                ev.signaled = True
                self.wake()
            elif sub == EVENT_RESET:
                ev.signaled = False
            return 0, 0
        if sub == CLOSE:
            if held[a[1]] <= 0:
                raise Unimplemented(f"a close of host object {a[1]}, which process {vm.process_id} holds no handle on")
            held[a[1]] -= 1
            self._close(a[1])
            return 0, 0
        if sub == REF:
            self._ref(a[1])
            held[a[1]] += 1
            return 0, 0
        if sub == PIPE_CREATE:
            pid = self._new(Pipe())
            r, w = self.next_id, self.next_id + 1
            self.next_id += 2
            self.pipe_ends[r] = (pid, True)
            self.pipe_ends[w] = (pid, False)
            held[r] += 1
            held[w] += 1
            return r, w
        if sub in (PIPE_READ, PIPE_WRITE):
            pipe = self.objects[self.pipe_ends[a[1]][0]]
            assert isinstance(pipe, Pipe)
            if sub == PIPE_WRITE:
                if pipe.readers == 0:
                    raise Unimplemented("a write to a pipe no process reads (ERROR_NO_DATA): not in the rows")
                n = min(a[3], PIPE_CAPACITY - len(pipe.data))
                pipe.data += vm.read_bytes(a[2], n)
                self.wake()
                return n, 0
            if not pipe.data:
                return 0, 2 if pipe.writers == 0 else 1
            n = min(a[3], len(pipe.data))
            vm.write_bytes(a[2], bytes(pipe.data[:n]))
            del pipe.data[:n]
            self.wake()                                  # a writer waiting on a full pipe has room now
            return n, 0
        if sub == PROCESS_CREATE:
            line = vm.read_text(a[1], a[2] & 0xFFFFFFFF)
            count = a[2] >> 32
            raw = vm.read_bytes(a[3], 24 * count)
            listed = [tuple(int(x) for x in struct.unpack_from("<QQQ", raw, 24 * i)) for i in range(count)]
            directories = [vm.read_text(o, h) for h, k, o in listed if k == KIND_DIRECTORY]
            if len(directories) != 1:
                raise Unimplemented(f"a process start that names {len(directories)} current directories")
            inherited = [(h, k, o) for h, k, o in listed if k != KIND_DIRECTORY]
            pid, tid = self.allocate_ids()
            child = self.spawn(line, pid, tid, inherited, vm.environment_now(), directories[0], vm.process_group)
            p = self.add(child)
            for _, kind, oid in inherited:
                if kind < KIND_STANDARD:
                    self._ref(oid)
                    p.held[oid] += 1                     # the child's handle on it
            p.handles = 1
            held[pid] += 1                               # the parent's handle on the child
            self.wake()
            return pid, tid
        if sub == PROCESS_STATE:
            found = self.processes.get(a[1])
            return (M64 if found is None else found.exit_code), 0
        raise Unimplemented(f"kernel request {sub}")

    def _ref(self, oid: int) -> None:
        if oid in self.pipe_ends:
            pipe = self.objects[self.pipe_ends[oid][0]]
            assert isinstance(pipe, Pipe)
            if self.pipe_ends[oid][1]:
                pipe.readers += 1
            else:
                pipe.writers += 1
        elif oid in self.processes:
            self.processes[oid].handles += 1
        else:
            ev = self.objects[oid]
            assert isinstance(ev, Event)
            ev.handles += 1

    def _close(self, oid: int) -> None:
        if oid in self.pipe_ends:
            pipe = self.objects[self.pipe_ends[oid][0]]
            assert isinstance(pipe, Pipe)
            if self.pipe_ends[oid][1]:
                pipe.readers -= 1
            else:
                pipe.writers -= 1
                if pipe.writers == 0:
                    self.wake()                  # a reader waiting on an empty pipe now sees it broken
            return
        if oid in self.processes:
            self.processes[oid].handles -= 1
            return
        ev = self.objects[oid]
        assert isinstance(ev, Event)
        ev.handles -= 1
        if ev.handles == 0:
            del self.objects[oid]
            self.names.pop(ev.name, None)

    def _release(self, p: Process) -> None:
        """A process ended: every handle it held closes (Windows: "Any open handles to the process are closed")."""
        for oid, n in list(p.held.items()):
            for _ in range(n):
                self._close(oid)
        p.held.clear()

    # ---- running ----------------------------------------------------------------------------------------------

    def others_ready(self, vm: Any) -> bool:
        return any(o is not vm and o.ready() for o in self.vms)

    def run(self, main: Any) -> Any:
        """Run every VM until the main one's process ends or stops; its Result. The others stop with it."""
        self.main = main
        if main.process_id not in self.processes:
            self.add(main)
        current = self.processes[main.process_id]
        try:
            while True:
                if current.steps is None:
                    current.steps = current.vm.steps(**self.run_args)
                try:
                    next(current.steps)
                except StopIteration as done:
                    current.result = done.value
                    current.exit_code = current.vm.exit_code if current.vm.exit_code is not None else 0
                    self.vms.remove(current.vm)
                    self._release(current)
                    self.wake()
                    if current.vm is main:
                        return current.result
                    if not current.result.stop.startswith("ExitProcess"):
                        return self._child_stopped(main, current)
                nxt = self._next(current)
                if nxt is None:
                    return self._deadlocked(main)
                current = nxt
        finally:
            for p in self.processes.values():
                if p.steps is not None and p.vm is not main:
                    p.steps.close()

    def _next(self, current: Process) -> Process | None:
        live = [self.processes[vm.process_id] for vm in self.vms]
        if not live:
            return None
        i = live.index(current) if current in live else -1
        rotation = live[i + 1:] + live[:i + 1]
        for p in rotation:
            if p.steps is None or p.vm.ready():
                return p
        timed = [p for p in rotation if p.vm.waits_with_deadline()]
        if timed:
            if not getattr(timed[0].vm, "instruction_clock", None):   # an instruction clock jumps instead of waiting
                time.sleep(0.001)
            return timed[0]
        return None

    def _deadlocked(self, main: Any) -> Any:
        """Every thread of every process waits with no timeout: the main VM's run stops there, as a VM stops on any
        Unimplemented (its Result holds what it wrote so far; Sprint 058: the Machine raised, and the run's output was
        lost)."""
        p = self.processes[main.process_id]
        stop = Unimplemented("every thread of every process waits with no timeout (a deadlock)")
        if p.steps is None:
            raise stop
        try:
            p.steps.throw(stop)
        except StopIteration as done:
            return done.value
        raise stop

    def _child_stopped(self, main: Any, child: Process) -> Any:
        """A child process stopped on something w2m lacks: the run stops there, naming it."""
        main.unimplemented = child.vm.unimplemented
        result = child.result
        result.stop = f"child process {child.pid}: {result.stop}"
        return result


def spawner(files: Any, dlls: Callable[[str], bytes | None] | None = None, runtime_sources: dict[str, str] | None = None
            ) -> Callable[[str, int, int, list[tuple[int, int, int]], tuple[str, ...], str, int], Any]:
    """The Machine's spawn for a run: a child's VM from its command line, its exe found by the first token (quoted or
    not) on the same drive and mount as the parent's files, its own file descriptors, the parent's environment and
    current directory as they are at the start (Sprint 058), the parent's guest runtime (a family's planted one too),
    and a GUI start when its subsystem is Windows GUI (2)."""
    from pathlib import Path

    from w2m.file_bridge import FileBridge
    from w2m.loader import load
    from w2m.pe import parse as parse_pe
    from w2m.vm_backend import VM

    def spawn(line: str, pid: int, tid: int, inherited: list[tuple[int, int, int]], environment: tuple[str, ...],
              directory: str, group: int) -> Any:
        first = line[1:line.index('"', 1)] if line.startswith('"') else line.split(" ", 1)[0]
        host = files.host_file(first)
        if host is None:
            raise Unimplemented(f"CreateProcessW of {first!r}: no such file (ERROR_FILE_NOT_FOUND is not in the rows)")
        data = host.read_bytes()
        child_files = FileBridge(files.root, files.read_only, drive=files.drive)
        here = host.parent
        resolve = dlls if dlls is not None else (lambda n: (here / n).read_bytes() if (here / n).is_file() else None)
        name = Path(host).name
        vm = VM(load(data, resolve, name), tid=tid, files=child_files, image_name=name,
                gui=parse_pe(data).subsystem == 2, command_line=line, environment=environment, process_id=pid,
                inherited=inherited, runtime_sources=runtime_sources, directory=directory, process_group=group)
        return vm

    return spawn
