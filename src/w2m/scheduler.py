# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""scheduler.py — guest threads on one vCPU (Sprint 051; BLACKBOARD Decision 2026-10-11).

Each guest thread is a `GuestThread`: its TEB, its stack, and, while it is not running, the vCPU state it left at an
x86 instruction boundary (the 31 registers, XMM0-15 with FPCR and FPSR in its vector slot, the monitor data words that
are per thread, and the state block), kept in the run table in the monitor's memory (Sprint 069). Threads switch when
the bridge hands the vCPU over (a wait that cannot complete, a yield, the end of a thread) and when a thread's time
slice runs out at a block's stop. The guest runtime's switch (threads.c w2m_switch) makes the switch inside the VM by
the rule `vm_backend._switch` applies in the host, which keeps what needs it: a thread's end (hvc #12), a wait with no
thread ready, lockstep, a machine of several processes. A thread blocked in a wait runs again when its key is
signaled (threads.c w2m_signal) or, for a finite wait, when its deadline passes on the instruction clock or no other
thread is ready.

The x86 stubs a thread runs at its start and end are written here: the modules' TLS callbacks and DllMain with
DLL_THREAD_ATTACH, the last error 187, the start routine, ExitThread with its return; at the end, DLL_THREAD_DETACH
the same way and W2mThreadEnd. contracts/windows.thread.threads records the order on Windows.
"""

from __future__ import annotations

import struct
from typing import Any

STILL_ACTIVE = 259
DLL_THREAD_ATTACH, DLL_THREAD_DETACH = 2, 3
START_LAST_ERROR = 187                   # what a new thread's start routine reads first (rows worker.start)
TIME_SLICE = 20_000                      # x86 instructions a thread runs before another ready thread gets the vCPU (Sprint 066 item 2)
# monitor data ranges each thread keeps (vm_backend.py offsets): lazy AF and DF, MXCSR, the x87 control word, the
# YMM upper halves
PER_THREAD_MONITOR = ((0x20, 0x40), (0x50, 0x58), (0x70, 0x72), (0x400, 0x500))
SCRATCH_OFF = 0x1900                     # runtime.h W2M_THREAD_SCRATCH_OFF
STATE_SIZE = 896                         # vm_backend.py: the state block (runtime.h struct w2m_state)
WAIT_KEY_OFF = 0x30                      # w2m_thread_scratch.wait_key: what a blocked thread waits on (recentering step 5)
CONTEXT_CONTROL, CONTEXT_INTEGER, CONTEXT_SEGMENTS, CONTEXT_FLOATING_POINT = 0x1, 0x2, 0x4, 0x8
CS, DS, ES, FS, GS, SS = 0x33, 0x2B, 0x2B, 0x53, 0x2B, 0x2B    # x64 user mode (rows context: CS 0x33, SS 0x2B)


# The run table (Sprint 069, recentering step 5): each thread's saved state, status, suspend count, deadline and census
# counters in the monitor's memory, where the guest runtime's switch (threads.c w2m_switch) reads and writes them as the
# host does; runtime.h struct w2m_run_table. The header's offsets, then each entry's.
RT_COUNT, RT_CURRENT, RT_CURRENT_ENTRY, RT_IN_VM, RT_BLOCKED, RT_WAITING, RT_SLICE_PAUSES, RT_SWITCHES = \
    0x00, 0x08, 0x10, 0x18, 0x20, 0x28, 0x30, 0x38
RT_ENTRIES = 0x100
E_TID, E_STATUS, E_SUSPEND, E_DEADLINE, E_TEB, E_VSLOT, E_INSTRUCTIONS, E_PAUSES = range(0, 0x40, 8)
E_X, E_MONITOR, E_STATE = 0x40, 0x140, 0x280
E_VCPU = 0x138                           # the vCPU that has the thread loaded, its index + 1; 0: none (Sprint 118)
E_MONITOR_AT = (0x00, 0x20, 0x28, 0x30)  # each PER_THREAD_MONITOR range's place in E_MONITOR
RUN_ENTRY = 0x600
STATUS_CODES = {"ready": 0, "blocked": 1, "ended": 2}
STATUS_NAMES = {v: k for k, v in STATUS_CODES.items()}


class GuestThread:
    """A guest thread: the host's own fields (its TEB, stack, exit code, origin), and a view of its run table entry for
    what the VM's switch reads and writes too: the registers it left at an x86 instruction boundary (x17 its RIP), the
    per-thread monitor words, the state block, its status, suspend count and deadline, and the instructions and PAUSEs
    it ran. A running thread's registers and monitor words are the vCPU's, not its entry's."""

    def __init__(self, mm: Any, table: int, index: int, tid: int, teb: int, stack: tuple[int, int, int],
                 parent: int = 0, start: int = 0, created_suspended: bool = False) -> None:
        self.mm, self.table, self.index = mm, table, index
        self.at = table + RT_ENTRIES + RUN_ENTRY * index
        self.tid, self.teb, self.stack = tid, teb, stack
        self.exit_code = STILL_ACTIVE
        self.parent = parent             # the creating thread's id; 0 for the first (GUEST_THREAD_STARTED; Sprint 060)
        self.start = start               # the start routine, or the first thread's entry
        self.created_suspended = created_suspended   # CREATE_SUSPENDED
        struct.pack_into("<QQ", mm, self.at + E_TID, tid, 0)
        struct.pack_into("<Q", mm, self.at + E_TEB, teb)
        struct.pack_into("<Q", mm, self.at + E_VCPU, 0)

    def _word(self, off: int) -> int:
        return int(struct.unpack_from("<Q", self.mm, self.at + off)[0])

    def _set_word(self, off: int, v: int) -> None:
        struct.pack_into("<Q", self.mm, self.at + off, v)

    @property
    def status(self) -> str:             # ready, blocked, ended
        return STATUS_NAMES[self._word(E_STATUS)]

    @status.setter
    def status(self, name: str) -> None:
        was, now = self._word(E_STATUS), STATUS_CODES[name]
        if (was == 1) != (now == 1):     # the header counts the blocked threads (threads.c w2m_signal reads it)
            blocked = struct.unpack_from("<Q", self.mm, self.table + RT_BLOCKED)[0]
            struct.pack_into("<Q", self.mm, self.table + RT_BLOCKED, blocked + (1 if now == 1 else -1))
        self._set_word(E_STATUS, now)

    @property
    def suspend(self) -> int:
        return self._word(E_SUSPEND)

    @suspend.setter
    def suspend(self, v: int) -> None:
        self._set_word(E_SUSPEND, v)

    @property
    def deadline(self) -> int:  # a blocked thread's wait deadline on the guest counter; ~0: none
        return self._word(E_DEADLINE)

    @deadline.setter
    def deadline(self, v: int) -> None:
        self._set_word(E_DEADLINE, v)

    @property
    def vslot(self) -> int:  # its vector slot's offset in the memory file (vm_backend.VECTOR_SLOTS)
        return self._word(E_VSLOT)

    @vslot.setter
    def vslot(self, v: int) -> None:
        self._set_word(E_VSLOT, v)

    @property
    def instructions(self) -> int:  # x86 instructions it ran (Sprint 068 item J)
        return self._word(E_INSTRUCTIONS)

    @instructions.setter
    def instructions(self, v: int) -> None:
        self._set_word(E_INSTRUCTIONS, v)

    @property
    def pauses(self) -> int:  # PAUSEs among them
        return self._word(E_PAUSES)

    @pauses.setter
    def pauses(self, v: int) -> None:
        self._set_word(E_PAUSES, v)

    @property
    def vcpu(self) -> int:  # the vCPU that has it loaded, its index + 1; 0: none (threads.c sets it in the VM too)
        return self._word(E_VCPU)

    @vcpu.setter
    def vcpu(self, v: int) -> None:
        self._set_word(E_VCPU, v)

    @property
    def x(self) -> list[int]:
        return list(struct.unpack_from("<31Q", self.mm, self.at + E_X))

    @x.setter
    def x(self, regs: list[int]) -> None:
        struct.pack_into("<31Q", self.mm, self.at + E_X, *regs)

    @property
    def monitor(self) -> list[bytes]:
        return [bytes(self.mm[self.at + E_MONITOR + p:self.at + E_MONITOR + p + b - a])
                for p, (a, b) in zip(E_MONITOR_AT, PER_THREAD_MONITOR)]

    @monitor.setter
    def monitor(self, ranges: list[bytes]) -> None:
        for p, data in zip(E_MONITOR_AT, ranges):
            self.mm[self.at + E_MONITOR + p:self.at + E_MONITOR + p + len(data)] = data

    @property
    def state(self) -> bytes:
        return bytes(self.mm[self.at + E_STATE:self.at + E_STATE + STATE_SIZE])

    @state.setter
    def state(self, data: bytes) -> None:
        self.mm[self.at + E_STATE:self.at + E_STATE + len(data)] = data


def _call(fn: int, rcx: int, edx: int) -> bytes:
    """sub rsp, 0x28; mov rcx, rcx; mov edx, edx; xor r8d, r8d; mov rax, fn; call rax; add rsp, 0x28."""
    return (b"\x48\x83\xec\x28" + b"\x48\xb9" + struct.pack("<Q", rcx) + b"\xba" + struct.pack("<I", edx & 0xFFFFFFFF)
            + b"\x45\x31\xc0" + b"\x48\xb8" + struct.pack("<Q", fn) + b"\xff\xd0" + b"\x48\x83\xc4\x28")


def attach_stub(callbacks: list[tuple[int, int]], start: int, parameter: int, exit_thread: int) -> bytes:
    """A new thread's first code: each (module base, function) with DLL_THREAD_ATTACH, the last error 187, the start
    routine with its parameter, then ExitThread with the routine's return. Entered with RSP = 8 mod 16, as a
    function is."""
    code = b"".join(_call(fn, base, DLL_THREAD_ATTACH) for base, fn in callbacks)
    code += b"\x65\xc7\x04\x25\x68\x00\x00\x00" + struct.pack("<I", START_LAST_ERROR)   # mov dword gs:[0x68], 187
    code += (b"\x48\x83\xec\x28" + b"\x48\xb9" + struct.pack("<Q", parameter) + b"\x48\xb8" + struct.pack("<Q", start)
             + b"\xff\xd0" + b"\x89\xc1" + b"\x48\xb8" + struct.pack("<Q", exit_thread) + b"\xff\xd0")   # call, then
    return code + b"\xcc"                                            # ExitThread(eax): never returns


def detach_stub(callbacks: list[tuple[int, int]], thread_end: int) -> bytes:
    """A thread's last code, from ExitThread: each (module base, function) with DLL_THREAD_DETACH, then W2mThreadEnd."""
    code = b"".join(_call(fn, base, DLL_THREAD_DETACH) for base, fn in callbacks)
    return code + b"\x48\x83\xec\x28" + b"\x48\xb8" + struct.pack("<Q", thread_end) + b"\xff\xd0\xcc"


def write_context(buf: bytearray, flags: int, gpr: list[int], rip: int, eflags: int, mxcsr: int, fcw: int,
                  xmm: list[int]) -> None:
    """A Windows x64 CONTEXT (runtime.h w2m_context offsets) for the parts `flags` names; ContextFlags stays as asked
    (row context: 0x100003)."""
    if flags & CONTEXT_CONTROL:
        struct.pack_into("<H", buf, 56, CS)
        struct.pack_into("<H", buf, 66, SS)
        struct.pack_into("<I", buf, 68, eflags)
        struct.pack_into("<Q", buf, 120 + 8 * 4, gpr[4])
        struct.pack_into("<Q", buf, 248, rip)
    if flags & CONTEXT_INTEGER:
        for i in range(16):
            if i != 4:
                struct.pack_into("<Q", buf, 120 + 8 * i, gpr[i])
    if flags & CONTEXT_SEGMENTS:
        struct.pack_into("<HHHH", buf, 58, DS, ES, FS, GS)
    if flags & CONTEXT_FLOATING_POINT:
        struct.pack_into("<I", buf, 52, mxcsr)
        struct.pack_into("<H", buf, 256, fcw)
        struct.pack_into("<I", buf, 256 + 24, mxcsr)
        struct.pack_into("<I", buf, 256 + 28, 0xFFFF)                   # MxCsr_Mask (the runner's)
        for i, v in enumerate(xmm):
            buf[256 + 160 + 16 * i:256 + 176 + 16 * i] = v.to_bytes(16, "little")
