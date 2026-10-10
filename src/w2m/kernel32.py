# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""kernel32.py — kernel32 serviced in the host (Sprint 029; shared by the `model` runtime and the
managed-VM back end since Sprint 030).

A call arrives when guest control reaches a loader thunk (src/w2m/loader.py). Arguments follow
the Microsoft x64 calling convention: RCX, RDX, R8, R9, then the stack above the return address
and a 32-byte shadow area; the result goes in RAX. Each function cites the Microsoft Learn page
it implements (retrieved 2026-10-08).
"""

from __future__ import annotations

import time
from collections.abc import Callable
from dataclasses import dataclass, field

from w2m.loader import Memory

M64 = (1 << 64) - 1
STD_OUTPUT_HANDLE = 0xFFFFFFF5          # (DWORD)-11, Microsoft Learn GetStdHandle
STDOUT = 0x0000_0000_0000_0074         # the runtime's handle value for standard output


COUNTS_PER_SECOND = 10_000_000          # QueryPerformanceFrequency on current x64 Windows


class Unimplemented(RuntimeError):
    """The guest called an import, or used an argument form, the runtime does not implement."""


class HandlerFailed(RuntimeError):
    """A kernel32 handler raised (Sprint 032: a generated candidate's code is a handler)."""


def monotonic_count() -> int:
    """QueryPerformanceCounter's count: the host's monotonic clock in 100 ns units."""
    return time.monotonic_ns() // (1_000_000_000 // COUNTS_PER_SECOND)


@dataclass
class Thread:
    tid: int
    regs: list[int] = field(default_factory=lambda: [0] * 16)
    rip: int = 0
    flags: dict[str, bool | None] = field(default_factory=lambda: dict.fromkeys("CZSOPA"))
    last_error: int = 0
    xmm: list[int] = field(default_factory=lambda: [0] * 16)           # 128-bit values (Sprint 039)
    mxcsr: int = 0x1F80                                               # reset value, SDM Vol 1 10.2.3
    df: bool = False


class Kernel32:
    def __init__(self, mem: Memory, clock: Callable[[], int] = monotonic_count) -> None:
        self.mem = mem
        self.clock = clock
        self.th = Thread(0)
        self.out = bytearray()
        self.exit_code: int | None = None
        self.calls: list[str] = []
        self.handlers: dict[str, Callable[[], int | None]] = {
            "GetCurrentThreadId": self._get_current_thread_id, "SetLastError": self._set_last_error,
            "GetLastError": self._get_last_error, "GetStdHandle": self._get_std_handle,
            "WriteFile": self._write_file, "ExitProcess": self._exit_process,
            "QueryPerformanceCounter": self._query_performance_counter}

    def call(self, name: str, th: Thread) -> int | None:
        """Service `name` ("DLL!Function") for thread `th`; then return as RET would: pop the
        return address from [RSP] and give it back. None means the process exited."""
        dll, fn = name.split("!", 1)
        handler = self.handlers.get(fn) if dll.upper() == "KERNEL32.DLL" else None
        if handler is None:
            raise Unimplemented(f"import {name}")
        self.calls.append(fn)
        self.th = th
        try:
            ret = handler()
        except (Unimplemented, HandlerFailed):
            raise
        except Exception as e:                                    # noqa: BLE001 — reported, not hidden
            raise HandlerFailed(f"{fn} raised {type(e).__name__}: {e}") from e
        if self.exit_code is not None:
            return None
        if ret is not None:
            th.regs[0] = ret & M64
        sp = th.regs[4]
        th.regs[4] = sp + 8
        return int.from_bytes(self.mem.read(sp, 8), "little")

    def argument(self, i: int) -> int:
        """Argument i of the current call under the Microsoft x64 calling convention."""
        if i < 4:
            return self.th.regs[(1, 2, 8, 9)[i]]
        sp = self.th.regs[4]                                      # [rsp] = return address
        return int.from_bytes(self.mem.read(sp + 8 + 32 + 8 * (i - 4), 8), "little")

    def _get_current_thread_id(self) -> int:
        return self.th.tid                                        # "thread identifier of the calling thread"

    def _set_last_error(self) -> None:
        self.th.last_error = self.argument(0) & 0xFFFFFFFF            # per-thread last-error code; returns None
        return None

    def _get_last_error(self) -> int:
        return self.th.last_error                                 # "the calling thread's last-error code"

    def _get_std_handle(self) -> int:
        if self.argument(0) & 0xFFFFFFFF == STD_OUTPUT_HANDLE:
            return STDOUT
        raise Unimplemented(f"GetStdHandle({self.argument(0) & 0xFFFFFFFF:#x})")

    def _write_file(self) -> int:
        h, buf, n, pwritten, povl = (self.argument(i) for i in range(5))
        n &= 0xFFFFFFFF
        if pwritten == 0 or povl != 0:
            raise Unimplemented("WriteFile without lpNumberOfBytesWritten, or overlapped")
        self.mem.write(pwritten, (0).to_bytes(4, "little"))      # "sets this value to zero before doing any work"
        if h != STDOUT:
            self.th.last_error = 6                                # ERROR_INVALID_HANDLE
            return 0
        self.out += self.mem.read(buf, n)
        self.mem.write(pwritten, n.to_bytes(4, "little"))
        return 1                                                  # "nonzero (TRUE)" on success

    def _query_performance_counter(self) -> int:
        p = self.argument(0)
        if p == 0:
            raise Unimplemented("QueryPerformanceCounter(NULL)")
        self.mem.write(p, (self.clock() & M64).to_bytes(8, "little"))   # "receives the current
        return 1                                                  # performance-counter value"; nonzero

    def _exit_process(self) -> None:
        self.exit_code = self.argument(0) & 0xFFFFFFFF
        return None
