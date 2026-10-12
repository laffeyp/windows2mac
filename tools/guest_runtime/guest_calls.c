// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* guest_calls.c — services that call the guest's own x86 code (Sprint 054; Sprint 053's exception dispatch first).
 *
 * Windows calls back into a program from inside a call it made: exception handlers from RaiseException, the window
 * procedure from CreateWindowExW, an enumeration's callback from EnumDisplayMonitors. A service here does it in
 * steps. It keeps its state in a frame on the guest's stack below its own stack pointer (w2m_guest_frame), sets the
 * guest's registers for the call and sends the bridge to the x86 CALL stub (src/w2m/guest_stubs.py: call rax, then
 * jump to W2mGuestReturn with the answer in RCX). W2mGuestReturn finds the frame at RSP + 0x20, where the stub's
 * call left it, and runs the frame's resume step, which calls the guest again or returns to the service's caller
 * (w2m_return_to: RAX the answer, RSP past the return address, RIP the return address). The guest's code follows
 * the x64 convention: it keeps the nonvolatile registers, so the caller sees them as it left them.
 */
#include "runtime.h"

#define RAX 0
#define RCX 1
#define RDX 2
#define RSP 4
#define R8 8
#define R9 9
#define FRAME_GAP 0x40                        /* bytes left free below the stack pointer a frame sits under */

u64 w2m_guest_stubs(void) {
    static u64 page;
    if (!page) page = w2m_host(W2M_HOST_EXCEPTION, W2M_EXCEPTION_STUBS, 0, 0, 0, 0);
    return page;
}

void *w2m_guest_frame(u64 rsp, u64 size, void (*resume)(struct w2m_guest_call *, u64)) {
    struct w2m_guest_call *c = (struct w2m_guest_call *)((rsp - FRAME_GAP - size) & ~15ull);
    u8 *b = (u8 *)c;
    for (u64 i = 0; i < size; i++) b[i] = 0;
    c->magic = W2M_GUEST_CALL_MAGIC;
    c->resume = resume;
    return c;
}

void w2m_call_guest(struct w2m_guest_call *c, u64 fn, u64 a0, u64 a1, u64 a2, u64 a3) {
    struct w2m_state *st = w2m_state();
    st->gpr[RAX] = fn;
    st->gpr[RCX] = a0;
    st->gpr[RDX] = a1;
    st->gpr[R8] = a2;
    st->gpr[R9] = a3;
    st->gpr[RSP] = (u64)c - 0x20;                 /* home space; `call` leaves RSP 8 mod 16 at entry */
    st->redirect_rip = w2m_guest_stubs() + W2M_STUB_CALL;
}

void w2m_return_to(u64 rsp, u64 value) {
    struct w2m_state *st = w2m_state();
    w2m_trace_return(rsp, value);             /* a traced call that ran guest code ends here (call_trace.c) */
    st->gpr[RAX] = value;
    st->gpr[RSP] = rsp + 8;
    st->redirect_rip = *(const u64 *)rsp;
}

struct w2m_guest_call *w2m_guest_call_at(u64 rsp) {
    struct w2m_guest_call *c = (struct w2m_guest_call *)(rsp + 0x20);
    return c->magic == W2M_GUEST_CALL_MAGIC ? c : 0;
}

/* Where guest code called through the CALL stub returns, its answer in RCX. */
W2M_VOID u64 k32_W2mGuestReturn(u64 answer, u64 a1, u64 a2, u64 a3, u64 rsp) {
    (void)a1, (void)a2, (void)a3;
    struct w2m_guest_call *c = w2m_guest_call_at(rsp);
    if (!c) w2m_unimplemented(W2M_API_W2mGuestReturn, rsp);
    w2m_logged(W2M_API_W2mGuestReturn, (u64)c->resume, answer);
    c->resume(c, answer);
    return 0;
}
