// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* runtime.c — kernel32 services run inside the managed VM (Sprint 035).
 *
 * Each k32_* function is entered from an AArch64 bridge stub that src/w2m/vm_backend.py generates
 * per import: the stub saves the guest's 16 GPRs in the state block, passes the Microsoft x64
 * arguments (RCX, RDX, R8, R9) in x0-x3 and the guest RSP in x4 (a fifth argument sits at
 * [RSP + 0x28]: return address, then 32 bytes of shadow space; Microsoft Learn, x64 calling
 * convention), switches to the runtime stack, and calls here. A function marked void in the
 * bridge table leaves RAX unchanged, as kernel32.py does.
 *
 * Each service cites the Microsoft Learn page it implements (as src/w2m/kernel32.py, retrieved
 * 2026-10-08). Host effects go through host_call; everything else stays in guest memory.
 */
#include "runtime.h"
#include "cpuid_table.h"

#define STD_OUTPUT_HANDLE 0xFFFFFFF5u           /* (DWORD)-11, GetStdHandle */
#define STDOUT_HANDLE 0x74ull                   /* the runtime's handle value for standard output */

static u64 host_call(u64 op, u64 a0, u64 a1) {
    return w2m_host(op, a0, a1, 0, 0, 0);
}

/* A host request on the transport page (hvc #8); `second`, when given, receives arg[5]. */
u64 w2m_host(u64 op, u64 a0, u64 a1, u64 a2, u64 a3, u64 *second) {
    volatile struct w2m_transport *t = (volatile struct w2m_transport *)W2M_TRANSPORT_VA;
    t->op = op;
    t->arg[0] = a0;
    t->arg[1] = a1;
    t->arg[2] = a2;
    t->arg[3] = a3;
    __asm__ volatile("hvc #8" ::: "memory");
    if (second) *second = t->arg[5];
    return t->result;
}

void w2m_unimplemented(u64 api, u64 detail) {
    host_call(W2M_HOST_UNIMPLEMENTED, api, detail);     /* the host stops the run; no return */
    for (;;) {}
}

u64 w2m_logged(u64 api, u64 arg0, u64 ret) {
    struct w2m_ring *r = (struct w2m_ring *)W2M_RING_VA;
    if (api < W2M_RING_APIS) r->per_api[api]++;
    if (r->count < r->capacity) {
        r->rec[r->count].api = api;
        r->rec[r->count].arg0 = arg0;
        r->rec[r->count].ret = ret;
    }
    r->count++;
    return ret;
}

/* "Retrieves the thread identifier of the calling thread." */
u64 k32_GetCurrentThreadId(void) {
    return w2m_logged(W2M_API_GetCurrentThreadId, 0, (u32)w2m_teb()->client_thread);
}

/* "Sets the last-error code for the calling thread." Returns nothing: RAX is left as it was. */
W2M_VOID u64 k32_SetLastError(u64 code) {
    w2m_set_last_error((u32)code);
    return w2m_logged(W2M_API_SetLastError, (u32)code, 0);
}

/* "Retrieves the calling thread's last-error code value." */
u64 k32_GetLastError(void) {
    return w2m_logged(W2M_API_GetLastError, 0, w2m_get_last_error());
}


/* WriteFile(hFile, lpBuffer, nNumberOfBytesToWrite, lpNumberOfBytesWritten, lpOverlapped):
 * "sets this value to zero before doing any work"; nonzero (TRUE) on success. */
u64 k32_WriteFile(u64 handle, u64 buffer, u64 count, u64 written, u64 rsp) {
    u64 overlapped = *(u64 *)(rsp + 0x28);
    count &= 0xFFFFFFFFull;
    if (written == 0 || overlapped != 0) w2m_unimplemented(W2M_API_WriteFile, written);
    *(u32 *)written = 0;
    if (handle != STDOUT_HANDLE)
        return w2m_logged(W2M_API_WriteFile, handle, w2m_file_write(handle, buffer, count, written));
    host_call(W2M_HOST_WRITE, buffer, count);
    *(u32 *)written = (u32)count;
    return w2m_logged(W2M_API_WriteFile, handle, 1);
}

/* "Ends the calling process and all its threads." */
W2M_VOID u64 k32_ExitProcess(u64 code) {
    w2m_logged(W2M_API_ExitProcess, (u32)code, 0);
    host_call(W2M_HOST_EXIT, (u32)code, 0);
    for (;;) {}
}

/* ---- CPUID (Sprint 040) -------------------------------------------------------------------- */

/* Entered from translated code at a CPUID with the guest GPRs in the state block. The guest
 * profile is specs/guest_cpuid.json (generated into cpuid_table.h); an input above the maximum
 * basic or extended leaf returns the highest basic leaf (SDM CPUID); 32-bit results zero-extend. */
/* The guest profile's answer for CPUID(leaf, sub) into r: EAX EBX ECX EDX. */
void w2m_cpuid_query(u32 leaf, u32 sub, u32 r[4]) {
    u32 a = 0, b = 0, c = 0, d = 0;
    if (leaf > W2M_CPUID_MAX_EXTENDED || (leaf > W2M_CPUID_MAX_BASIC && leaf < 0x80000000u))
        leaf = W2M_CPUID_MAX_BASIC;
    int found = 0, leaf_known = 0;
    for (unsigned i = 0; i < sizeof w2m_cpuid_table / sizeof w2m_cpuid_table[0]; i++) {
        if (w2m_cpuid_table[i].leaf != leaf) continue;
        leaf_known = 1;
        if (w2m_cpuid_table[i].any_subleaf || w2m_cpuid_table[i].subleaf == sub) {
            a = w2m_cpuid_table[i].eax; b = w2m_cpuid_table[i].ebx;
            c = w2m_cpuid_table[i].ecx; d = w2m_cpuid_table[i].edx;
            found = 1;
            break;
        }
    }
    if (!found && !leaf_known) {                       /* not in the table: the highest basic leaf */
        for (unsigned i = 0; i < sizeof w2m_cpuid_table / sizeof w2m_cpuid_table[0]; i++)
            if (w2m_cpuid_table[i].leaf == W2M_CPUID_MAX_BASIC) {
                a = w2m_cpuid_table[i].eax; b = w2m_cpuid_table[i].ebx;
                c = w2m_cpuid_table[i].ecx; d = w2m_cpuid_table[i].edx;
            }
    }
    r[0] = a; r[1] = b; r[2] = c; r[3] = d;
}

void w2m_cpuid(void) {
    struct w2m_state *st = (struct w2m_state *)W2M_STATE_VA;
    u32 leaf = (u32)st->gpr[0], sub = (u32)st->gpr[1], r[4];
    w2m_logged(W2M_API_Cpuid, (u64)leaf | (u64)sub << 32, 0);
    w2m_cpuid_query(leaf, sub, r);
    st->gpr[0] = r[0];
    st->gpr[3] = r[1];
    st->gpr[1] = r[2];
    st->gpr[2] = r[3];
}

/* ---- exceptions (Sprint 040) ------------------------------------------------------------- */

/* The reconstruction table: (host code address, x86 instruction address) pairs, sorted by host
 * address, one per translated instruction and one per block start; a bridge's range maps to 0.
 * Written by the host as it compiles. */
struct w2m_recon { u64 host, guest; };

static u64 guest_rip_at(u64 host_pc) {
    const struct w2m_recon *t = (const struct w2m_recon *)W2M_RECON_VA;
    u64 n = *(volatile u64 *)W2M_RECON_COUNT_VA, lo = 0, hi = n;
    while (lo < hi) {                                  /* first entry with host > host_pc */
        u64 mid = (lo + hi) / 2;
        if (t[mid].host <= host_pc) lo = mid + 1;
        else hi = mid;
    }
    return lo ? t[lo - 1].guest : 0;
}

/* RFLAGS from the flag registers (x19 CF, x20 ZF, x21 SF, x22 OF, x23 PF as 0/1), lazy AF and DF
 * (SDM Vol 1 3.4.3: CF bit 0, PF 2, AF 4, ZF 6, SF 7, IF 9, DF 10, OF 11; bit 1 reads 1). An
 * undefined flag reads 0 here; its bit in flags_undefined says so. */
static u64 materialize_rflags(const struct w2m_save_area *s, u64 af, u64 df) {
    u64 undef = s->x[24];
    u64 f = 0x202;
    if (!(undef & 1)) f |= (s->x[19] & 1) << 0;
    if (!(undef & 16)) f |= (s->x[23] & 1) << 2;
    if (!(undef & 32)) f |= af << 4;
    if (!(undef & 2)) f |= (s->x[20] & 1) << 6;
    if (!(undef & 4)) f |= (s->x[21] & 1) << 7;
    f |= df << 10;
    if (!(undef & 8)) f |= (s->x[22] & 1) << 11;
    return f;
}

static void context_from_state(struct w2m_context *c, const struct w2m_state *st) {
    u8 *b = (u8 *)c;
    for (u64 i = 0; i < sizeof *c; i++) b[i] = 0;
    c->context_flags = 0x10001F;                       /* CONTEXT_ALL (winnt.h): CONTEXT_AMD64 | 0x1F */
    c->mx_csr = st->mxcsr;
    c->seg_cs = 0x33;                                  /* the x64 user selectors Windows reports */
    c->seg_ds = c->seg_es = c->seg_gs = c->seg_ss = 0x2B;
    c->seg_fs = 0x53;
    c->eflags = (u32)st->rflags;
    for (int i = 0; i < 16; i++) c->gpr[i] = st->gpr[i];
    c->rip = st->rip;
    c->flt_save.control_word = st->x87_control_word;
    c->flt_save.status_word = st->x87_status_word;
    c->flt_save.tag_word = st->x87_tag_abridged;
    c->flt_save.mx_csr = st->mxcsr;
    c->flt_save.mx_csr_mask = 0xFFFF;
    for (int i = 0; i < 8; i++)
        for (int j = 0; j < 16; j++) c->flt_save.float_registers[i][j] = st->st[i][j];
    for (int i = 0; i < 16; i++)
        for (int j = 0; j < 16; j++) c->flt_save.xmm_registers[i][j] = st->xmm[i][j];
}

/* Entered from the EL1 synchronous-exception vector after it saved the registers, with the
 * runtime stack. A data abort in translated code becomes a Windows access violation: the
 * thread's state block is filled from the saved registers (the commit rule makes them the state
 * before the faulting instruction), and the exception record and CONTEXT are built from it. With
 * no vectored handler yet (Sprint 047), the exception is unhandled and the process ends with the
 * exception code. Anything else, or a fault outside translated code, is a runtime fault the host
 * reports. */
/* MXCSR's exception flags from FPSR's cumulative bits: IOC IE, DZC ZE, OFC OE, UFC UE, IXC PE, IDC DE. */
static u32 mxcsr_now(void) {
    u64 fpsr;
    __asm__ volatile("mrs %0, S3_3_C4_C4_1" : "=r"(fpsr));     /* FPSR, by encoding: -mgeneral-regs-only rejects the name */
    u32 m = *(const volatile u32 *)(W2M_MONITOR_DATA_VA + 0x50);
    if (fpsr & 1) m |= 1;
    if (fpsr & 2) m |= 4;
    if (fpsr & 4) m |= 8;
    if (fpsr & 8) m |= 16;
    if (fpsr & 16) m |= 32;
    if (fpsr & 128) m |= 2;
    return m;
}

void w2m_exception(void) {
    const struct w2m_save_area *s = (const struct w2m_save_area *)W2M_SAVE_VA;
    u64 ec = s->esr >> 26 & 0x3F;
    u64 rip = s->sw_code ? s->sw_rip
            : (s->elr >= W2M_CODE_START && s->elr < W2M_CODE_END) ? guest_rip_at(s->elr) : 0;
    if (!s->sw_code && (ec != 0x25 || rip == 0)) {
        host_call(W2M_HOST_RUNTIME_FAULT, s->elr, s->esr);
        for (;;) {}
    }
    struct w2m_state *st = (struct w2m_state *)W2M_STATE_VA;
    const volatile u64 *af = (const volatile u64 *)(W2M_MONITOR_DATA_VA + 0x20);
    u64 af_bit = ((af[0] ^ af[1] ^ af[2]) >> 4) & 1;
    u64 df = *(const volatile u8 *)(W2M_MONITOR_DATA_VA + 0x38) & 1;
    for (int i = 0; i < 16; i++) st->gpr[i] = s->x[i];
    st->rip = rip;
    st->flags_undefined = s->x[24];
    st->rflags = materialize_rflags(s, af_bit, df);
    st->af_operands[0] = af[0];
    st->af_operands[1] = af[1];
    st->af_operands[2] = af[2];
    st->df = (u8)df;
    st->mxcsr = mxcsr_now();
    for (int i = 0; i < 16; i++)
        for (int j = 0; j < 16; j++) st->xmm[i][j] = s->q[i][j];

    struct w2m_exception_area *e = (struct w2m_exception_area *)W2M_EXCEPTION_VA;
    struct w2m_exception_record *r = &e->record;
    u8 *rb = (u8 *)r;
    for (u64 i = 0; i < sizeof *r; i++) rb[i] = 0;
    r->address = rip;
    if (s->sw_code) {                                  /* an SIMD exception: STATUS_FLOAT_* (Sprint 039) */
        r->code = (u32)s->sw_code;
    } else {
        r->code = 0xC0000005u;                         /* STATUS_ACCESS_VIOLATION */
        r->number_parameters = 2;
        r->information[0] = s->esr >> 6 & 1;          /* ESR ISS WnR: 0 read, 1 write */
        r->information[1] = s->far;
    }
    context_from_state(&e->context, st);
    e->magic = W2M_EXCEPTION_MAGIC;
    w2m_logged(W2M_API_Exception, r->code, rip);
    host_call(W2M_HOST_EXIT, r->code, 0);              /* unhandled: the process ends */
    for (;;) {}
}

/* The linker needs an entry symbol; nothing enters here. */
void w2m_image_entry(void) {}
