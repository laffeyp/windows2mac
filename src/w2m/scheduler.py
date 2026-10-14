# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""scheduler.py — guest threads on one vCPU (Sprint 051; BLACKBOARD Decision 2026-10-11).

Each guest thread is a `GuestThread`: its TEB, its stack, and, while it is not running, the vCPU state it left at an
x86 instruction boundary (the 31 registers, XMM0-15 with FPCR and FPSR, the monitor data words that are per thread,
and the state block). The VM (vm_backend.py) switches threads when the bridge hands the vCPU over (hvc #12: a wait
that cannot complete, a yield, the end of a thread) and when a thread's time slice runs out (the step budget,
hvc #4). A thread blocked in a wait runs again when any waitable thing is signaled (W2M_THREAD_SIGNAL) or, for a
finite wait, when no other thread is ready, to check its deadline itself (the guest runtime's counter).

The x86 stubs a thread runs at its start and end are written here: the modules' TLS callbacks and DllMain with
DLL_THREAD_ATTACH, the last error 187, the start routine, ExitThread with its return; at the end, DLL_THREAD_DETACH
the same way and W2mThreadEnd. contracts/windows.thread.threads records the order on Windows.
"""

from __future__ import annotations

import struct
from dataclasses import dataclass, field

STILL_ACTIVE = 259
DLL_THREAD_ATTACH, DLL_THREAD_DETACH = 2, 3
START_LAST_ERROR = 187                   # what a new thread's start routine reads first (rows worker.start)
TIME_SLICE = 20_000                      # x86 instructions a thread runs before another ready thread gets the vCPU (Sprint 066 item 2)
# monitor data ranges each thread keeps (vm_backend.py offsets): lazy AF and DF, MXCSR, the x87 control word, the
# YMM upper halves
PER_THREAD_MONITOR = ((0x20, 0x40), (0x50, 0x58), (0x70, 0x72), (0x400, 0x500))
SCRATCH_OFF = 0x1900                     # runtime.h W2M_THREAD_SCRATCH_OFF
CONTEXT_CONTROL, CONTEXT_INTEGER, CONTEXT_SEGMENTS, CONTEXT_FLOATING_POINT = 0x1, 0x2, 0x4, 0x8
CS, DS, ES, FS, GS, SS = 0x33, 0x2B, 0x2B, 0x53, 0x2B, 0x2B    # x64 user mode (rows context: CS 0x33, SS 0x2B)


@dataclass
class GuestThread:
    tid: int
    teb: int
    stack: tuple[int, int, int]          # DeallocationStack, StackLimit, StackBase
    x: list[int] = field(default_factory=list)
    xmm: list[int] = field(default_factory=lambda: [0] * 16)
    fpcr: int = 0
    fpsr: int = 0
    monitor: list[bytes] = field(default_factory=list)
    state: bytes = b""
    status: str = "ready"                # ready, blocked, ended
    suspend: int = 0
    exit_code: int = STILL_ACTIVE
    deadline: int = 0                    # a blocked thread's wait deadline on the guest counter; ~0: none
    parent: int = 0                      # the creating thread's id; 0 for the first (GUEST_THREAD_STARTED; Sprint 060)
    start: int = 0                       # the start routine, or the first thread's entry
    created_suspended: bool = False      # CREATE_SUSPENDED
    vslot: int = 0                       # its vector slot's offset in the memory file (vm_backend.VECTOR_SLOTS)


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
