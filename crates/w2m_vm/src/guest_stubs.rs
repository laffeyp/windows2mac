// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! The x86 code a service calls the guest's own code through (Sprint 147), carried from `src/w2m/guest_stubs.py`
//! (Sprint 053, `exceptions.c`; Sprint 054, `guest_calls.c`: window procedures and enumeration callbacks too).
//!
//! Services are guest runtime code; the handlers, window procedures and callbacks they call are x86. Three stubs on one
//! page bridge them:
//!
//! - `RETURN` (+0x00): `ret`. RaiseException's context resumes here, with RSP at the call's return address, so a
//!   handler that continues execution returns from RaiseException.
//! - `CALL` (+0x10): `call rax; mov rcx, rax; mov rax, <W2mGuestReturn's thunk>; jmp rax`. The runtime sets RAX to the
//!   function, RCX, RDX, R8, R9 to its arguments and RSP to the calling service's frame less 0x20; the answer comes back
//!   to W2mGuestReturn, which finds the frame at RSP + 0x20. `CALL_RETURN` is the return point an exception walk
//!   recognizes.
//! - `RESTORE` (+0x40): RCX holds a CONTEXT: XMM0-15 and MXCSR, then RSP from it less 16, RIP and EFLAGS written below
//!   it, the GPRs (RCX last), `popfq`, `ret`: the context resumed whole (RtlRestoreContext's work).

/// guest_stubs.py 24-26.
pub const RETURN: usize = 0x00;
pub const CALL: usize = 0x10;
pub const RESTORE: usize = 0x40;
pub const CALL_RETURN: usize = CALL + 2;
pub const SIZE: usize = 0x200;

/// guest_stubs.py 28-30: CONTEXT offsets (runtime.h `w2m_context`; Microsoft Learn, CONTEXT x64).
const MX_CSR: u8 = 0x34;
const EFLAGS: u8 = 0x44;
const RSP_OFF: i32 = 0x98;
const RIP_OFF: i32 = 0xF8;
const XMM0: i32 = 0x1A0;
/// Rax; Rcx Rdx Rbx Rsp Rbp Rsi Rdi R8-R15 follow, 8 bytes each
const GPR: i32 = 0x78;

/// `_mov_from_rcx` (33-35): `mov reg64, [rcx + disp32]`.
fn mov_from_rcx(reg: u8, disp: i32) -> Vec<u8> {
    let mut out = vec![0x48 | (reg >> 3) << 2, 0x8B, 0x80 | (reg & 7) << 3 | 1];
    out.extend(disp.to_le_bytes());
    out
}

/// `_restore` (38-48).
fn restore() -> Vec<u8> {
    let mut code = Vec::new();
    for i in 0..16u8 {
        // movups xmmI, [rcx + XMM0 + 16 i]
        if i >= 8 {
            code.push(0x44);
        }
        code.extend([0x0F, 0x10, 0x80 | (i & 7) << 3 | 1]);
        code.extend((XMM0 + 16 * i32::from(i)).to_le_bytes());
    }
    code.extend([0x0F, 0xAE, 0x51, MX_CSR]); // ldmxcsr [rcx + 0x34]
    code.extend(mov_from_rcx(4, RSP_OFF)); // mov rsp, [rcx + Rsp]
    code.extend([0x48, 0x83, 0xEC, 0x10]); // sub rsp, 16
    code.extend(mov_from_rcx(0, RIP_OFF)); // mov rax, [rcx + Rip]
    code.extend([0x48, 0x89, 0x44, 0x24, 0x08]); // mov [rsp + 8], rax
    code.extend([0x8B, 0x41, EFLAGS]); // mov eax, [rcx + EFlags]
    code.extend([0x48, 0x89, 0x04, 0x24]); // mov [rsp], rax
    for reg in [0u8, 2, 3, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 1] {
        // every GPR but RSP; RCX, the base, last
        code.extend(mov_from_rcx(reg, GPR + 8 * i32::from(reg)));
    }
    code.extend([0x9D, 0xC3]); // popfq; ret
    code
}

/// `page` (51-60): the stub page, with W2mGuestReturn's thunk address in `CALL`.
pub fn page(resume_thunk: u64) -> Vec<u8> {
    let mut out = vec![0xCC; SIZE];
    out[RETURN] = 0xC3;
    let mut call = vec![0xFF, 0xD0, 0x48, 0x89, 0xC1, 0x48, 0xB8];
    call.extend(resume_thunk.to_le_bytes());
    call.extend([0xFF, 0xE0]);
    out[CALL..CALL + call.len()].copy_from_slice(&call);
    let r = restore();
    assert!(RESTORE + r.len() <= SIZE);
    out[RESTORE..RESTORE + r.len()].copy_from_slice(&r);
    out
}
