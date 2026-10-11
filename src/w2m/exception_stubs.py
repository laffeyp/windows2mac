# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""exception_stubs.py — the x86 code exception dispatch runs through (Sprint 053; tools/guest_runtime/exceptions.c).

The dispatcher is guest runtime code; the handlers it calls are x86. Three stubs on one page bridge them:

  +0x00 RETURN   ret. RaiseException's context resumes here, with RSP at the call's return address, so a handler
                 that continues execution returns from RaiseException (Windows's context points inside
                 KERNELBASE!RaiseException, after its call to RtlRaiseException).
  +0x10 CALL     call rax; mov rcx, rax; mov rax, <W2mExceptionResume's thunk>; jmp rax. The runtime sets RAX to the
                 handler, RCX, RDX, R8, R9 to its arguments and RSP to the dispatch frame less 0x20; the handler's
                 answer comes back to the service, which finds its frame at RSP + 0x20. CALL + 2 is the return point
                 a walk recognizes.
  +0x40 RESTORE  rcx = a CONTEXT: XMM0-15 and MXCSR, then RSP from it less 16, RIP and EFLAGS written below it,
                 the GPRs (RCX last), popfq, ret: the context resumed whole (RtlRestoreContext's work).
"""

from __future__ import annotations

import struct

RETURN, CALL, RESTORE = 0x00, 0x10, 0x40
CALL_RETURN = CALL + 2
SIZE = 0x200

# CONTEXT offsets (runtime.h w2m_context; Microsoft Learn, CONTEXT x64)
MX_CSR, EFLAGS, RSP_OFF, RIP_OFF, XMM0 = 0x34, 0x44, 0x98, 0xF8, 0x1A0
GPR = 0x78                                   # Rax; Rcx Rdx Rbx Rsp Rbp Rsi Rdi R8-R15 follow, 8 bytes each


def _mov_from_rcx(reg: int, disp: int) -> bytes:
    """mov reg64, [rcx + disp32]."""
    return bytes([0x48 | (reg >> 3) << 2, 0x8B, 0x80 | (reg & 7) << 3 | 1]) + struct.pack("<i", disp)


def _restore() -> bytes:
    code = b""
    for i in range(16):                                                   # movups xmmI, [rcx + XMM0 + 16 i]
        code += (b"\x44" if i >= 8 else b"") + bytes([0x0F, 0x10, 0x80 | (i & 7) << 3 | 1]) + struct.pack("<i", XMM0 + 16 * i)
    code += bytes([0x0F, 0xAE, 0x51, MX_CSR])                             # ldmxcsr [rcx + 0x34]
    code += _mov_from_rcx(4, RSP_OFF) + b"\x48\x83\xec\x10"               # mov rsp, [rcx + Rsp]; sub rsp, 16
    code += _mov_from_rcx(0, RIP_OFF) + b"\x48\x89\x44\x24\x08"           # mov rax, [rcx + Rip]; mov [rsp + 8], rax
    code += b"\x8b\x41" + bytes([EFLAGS]) + b"\x48\x89\x04\x24"           # mov eax, [rcx + EFlags]; mov [rsp], rax
    for reg in (0, 2, 3, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 1):      # every GPR but RSP; RCX, the base, last
        code += _mov_from_rcx(reg, GPR + 8 * reg)
    return code + b"\x9d\xc3"                                             # popfq; ret


def page(resume_thunk: int) -> bytes:
    """The stub page, with W2mExceptionResume's thunk address in CALL."""
    out = bytearray(b"\xcc" * SIZE)
    out[RETURN] = 0xC3
    call = b"\xff\xd0" + b"\x48\x89\xc1" + b"\x48\xb8" + struct.pack("<Q", resume_thunk) + b"\xff\xe0"
    out[CALL:CALL + len(call)] = call
    restore = _restore()
    assert RESTORE + len(restore) <= SIZE
    out[RESTORE:RESTORE + len(restore)] = restore
    return bytes(out)
