// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* running_thread_context.c — the CONTEXT of a thread suspended while it runs, and of one suspended in a wait
 * (Sprint 066 item 4, plan row 066).
 *
 * No C runtime. mono-2.0-bdwgc.dll's collector suspends each of the game's threads, reads its CONTEXT and scans
 * the registers as possible roots (game run 38216081440 seq 2165-2349: SuspendThread, GetThreadContext,
 * ResumeThread on handle 0x364; review 2026-10-09 project §2). contracts/windows.thread.threads row "context"
 * holds RIP in the image and RSP in the stack only. Here the spinner (spin_planted, below) loads a known value
 * into every general register but RSP, into XMM0-15 and into MXCSR, then spins in a two-instruction loop that
 * changes none of them; the main thread suspends it and reads CONTEXT_ALL. The waiter blocks in
 * WaitForSingleObject; the main thread suspends it and reads CONTEXT_CONTROL | CONTEXT_INTEGER.
 *
 * Rows:
 *   context.running        ret, err, flags (ContextFlags after the call), gpr_planted (bit i: register i in
 *                          CONTEXT order RAX, RCX, RDX, RBX, RSP, RBP, ... R15 equals its planted value; RSP's bit is
 *                          never set), rsp_in_stack, rip_in_loop, xmm_planted (bit i: XMMi equals its planted
 *                          value), mxcsr, fltsave_mxcsr, seg_cs, seg_ss, seg_ds, seg_es, seg_fs, seg_gs, eflags_if
 *                          (EFlags bit 9), debug_zero (Dr0-3, Dr6, Dr7 all 0)
 *   context.running.again  after ResumeThread, the spinner counting again and SuspendThread: ret, count_moved,
 *                          gpr_planted, xmm_planted, rip_in_loop
 *   context.waiting        ret, err, flags, rip_in_image, rip_in_ntdll, rip_in_kernelbase, rip_in_kernel32,
 *                          rsp_in_stack, rsp_below_frame (RSP below a local of the waiter's frame that made the call)
 *   resume                 the counts ResumeThread answers for the two threads, err
 */
#include <windows.h>

#define SENTINEL 0xDEADu

static char out[4096];
static unsigned n;

static void put(const char *s) { while (*s && n < sizeof out - 64) out[n++] = *s++; }
static void putu(unsigned long long v) {
    char t[24];
    int k = 0;
    do { t[k++] = (char)('0' + v % 10); v /= 10; } while (v);
    while (k) out[n++] = t[--k];
}
static void row(const char *s) { put("{\"s\":\""); put(s); put("\""); }
static void field(const char *k, unsigned long long v) { put(",\""); put(k); put("\":"); putu(v); }
static void end(void) {
    put("}\n");
    DWORD written;
    WriteFile(GetStdHandle(STD_OUTPUT_HANDLE), out, n, &written, NULL);
    n = 0;
}

static ULONG_PTR teb(void) { ULONG_PTR t; __asm__ volatile("movq %%gs:0x30, %0" : "=r"(t)); return t; }

/* Register i's planted value (CONTEXT order); XMMi's low and high halves; MXCSR with FTZ and DAZ set. */
#define PLANT(i) (0x0101010101010101ull * (0x10 + (i)))
DECLSPEC_ALIGN(16) ULONG_PTR planted_xmm[32];
DWORD planted_mxcsr = 0x9FC0;
volatile LONG spin_count, spin_stop;
static volatile ULONG_PTR spin_base, spin_limit;

DWORD WINAPI spin_planted(LPVOID param);
extern char spin_loop[], spin_loop_end[];

/* Saves the callee-saved registers, XMM6-15 and MXCSR, plants every register, spins until spin_stop, restores. */
__asm__(
    ".intel_syntax noprefix\n"
    ".text\n"
    ".globl spin_planted\n"
    ".globl spin_loop\n"
    ".globl spin_loop_end\n"
    "spin_planted:\n"
    "  push rbx\n  push rbp\n  push rsi\n  push rdi\n  push r12\n  push r13\n  push r14\n  push r15\n"
    "  sub rsp, 168\n"
    "  movdqu [rsp], xmm6\n  movdqu [rsp+16], xmm7\n  movdqu [rsp+32], xmm8\n  movdqu [rsp+48], xmm9\n"
    "  movdqu [rsp+64], xmm10\n  movdqu [rsp+80], xmm11\n  movdqu [rsp+96], xmm12\n  movdqu [rsp+112], xmm13\n"
    "  movdqu [rsp+128], xmm14\n  movdqu [rsp+144], xmm15\n"
    "  stmxcsr [rsp+160]\n"
    "  lea rax, [rip+planted_xmm]\n"
    "  movdqa xmm0, [rax]\n  movdqa xmm1, [rax+16]\n  movdqa xmm2, [rax+32]\n  movdqa xmm3, [rax+48]\n"
    "  movdqa xmm4, [rax+64]\n  movdqa xmm5, [rax+80]\n  movdqa xmm6, [rax+96]\n  movdqa xmm7, [rax+112]\n"
    "  movdqa xmm8, [rax+128]\n  movdqa xmm9, [rax+144]\n  movdqa xmm10, [rax+160]\n  movdqa xmm11, [rax+176]\n"
    "  movdqa xmm12, [rax+192]\n  movdqa xmm13, [rax+208]\n  movdqa xmm14, [rax+224]\n  movdqa xmm15, [rax+240]\n"
    "  ldmxcsr [rip+planted_mxcsr]\n"
    "  movabs rax, 0x1010101010101010\n  movabs rcx, 0x1111111111111111\n  movabs rdx, 0x1212121212121212\n"
    "  movabs rbx, 0x1313131313131313\n  movabs rbp, 0x1515151515151515\n  movabs rsi, 0x1616161616161616\n"
    "  movabs rdi, 0x1717171717171717\n  movabs r8, 0x1818181818181818\n  movabs r9, 0x1919191919191919\n"
    "  movabs r10, 0x1a1a1a1a1a1a1a1a\n  movabs r11, 0x1b1b1b1b1b1b1b1b\n  movabs r12, 0x1c1c1c1c1c1c1c1c\n"
    "  movabs r13, 0x1d1d1d1d1d1d1d1d\n  movabs r14, 0x1e1e1e1e1e1e1e1e\n  movabs r15, 0x1f1f1f1f1f1f1f1f\n"
    "spin_loop:\n"
    "  lock inc dword ptr [rip+spin_count]\n"
    "  cmp dword ptr [rip+spin_stop], 0\n"
    "  je spin_loop\n"
    "spin_loop_end:\n"
    "  ldmxcsr [rsp+160]\n"
    "  movdqu xmm6, [rsp]\n  movdqu xmm7, [rsp+16]\n  movdqu xmm8, [rsp+32]\n  movdqu xmm9, [rsp+48]\n"
    "  movdqu xmm10, [rsp+64]\n  movdqu xmm11, [rsp+80]\n  movdqu xmm12, [rsp+96]\n  movdqu xmm13, [rsp+112]\n"
    "  movdqu xmm14, [rsp+128]\n  movdqu xmm15, [rsp+144]\n"
    "  add rsp, 168\n"
    "  pop r15\n  pop r14\n  pop r13\n  pop r12\n  pop rdi\n  pop rsi\n  pop rbp\n  pop rbx\n"
    "  mov eax, 5\n"
    "  ret\n"
    ".att_syntax prefix\n");

/* The spinner's stack bounds, read before it plants: a thread that starts at spin_planted reads nothing. */
static DWORD WINAPI spinner(LPVOID param) {
    spin_base = *(ULONG_PTR *)(teb() + 0x08);
    spin_limit = *(ULONG_PTR *)(teb() + 0x10);
    return spin_planted(param);
}

static HANDLE waiter_started, waiter_go;
static volatile ULONG_PTR wait_base, wait_limit, wait_frame;
static DWORD WINAPI waiter(LPVOID param) {
    volatile ULONG_PTR local = 0;
    (void)param;
    wait_base = *(ULONG_PTR *)(teb() + 0x08);
    wait_limit = *(ULONG_PTR *)(teb() + 0x10);
    wait_frame = (ULONG_PTR)&local;
    SetEvent(waiter_started);
    WaitForSingleObject(waiter_go, INFINITE);
    return 6 + (DWORD)local;
}

static void module_range(const wchar_t *name, ULONG_PTR *lo, ULONG_PTR *hi) {
    ULONG_PTR m = (ULONG_PTR)GetModuleHandleW(name);
    *lo = m;
    *hi = m ? m + ((IMAGE_NT_HEADERS64 *)(m + ((IMAGE_DOS_HEADER *)m)->e_lfanew))->OptionalHeader.SizeOfImage : 0;
}

static unsigned gpr_planted(const CONTEXT *c) {
    const DWORD64 *g = &c->Rax;
    unsigned bits = 0;
    for (unsigned i = 0; i < 16; i++)
        if (i != 4 && g[i] == PLANT(i)) bits |= 1u << i;
    return bits;
}

static unsigned xmm_planted(const CONTEXT *c) {
    const M128A *x = &c->Xmm0;
    unsigned bits = 0;
    for (unsigned i = 0; i < 16; i++)
        if (x[i].Low == planted_xmm[2 * i] && (ULONG_PTR)x[i].High == planted_xmm[2 * i + 1]) bits |= 1u << i;
    return bits;
}

static int in_loop(DWORD64 rip) { return rip >= (ULONG_PTR)spin_loop && rip < (ULONG_PTR)spin_loop_end; }

void start(void) {
    for (unsigned i = 0; i < 16; i++) {
        planted_xmm[2 * i] = 0x2020202020202020ull + i;
        planted_xmm[2 * i + 1] = 0x3030303030303030ull + i;
    }

    /* a thread suspended while it runs */
    HANDLE spin = CreateThread(NULL, 0, spinner, NULL, 0, NULL);
    while (spin_count < 1000) SwitchToThread();
    SetLastError(SENTINEL);
    SuspendThread(spin);
    static CONTEXT c;
    c.ContextFlags = CONTEXT_ALL;
    BOOL r = GetThreadContext(spin, &c);
    DWORD err = GetLastError();
    row("context.running"); field("ret", r != 0); field("err", err); field("flags", c.ContextFlags);
    field("gpr_planted", gpr_planted(&c)); field("rsp_in_stack", c.Rsp >= spin_limit && c.Rsp < spin_base);
    field("rip_in_loop", in_loop(c.Rip)); field("xmm_planted", xmm_planted(&c)); field("mxcsr", c.MxCsr);
    field("fltsave_mxcsr", c.FltSave.MxCsr); field("seg_cs", c.SegCs); field("seg_ss", c.SegSs);
    field("seg_ds", c.SegDs); field("seg_es", c.SegEs); field("seg_fs", c.SegFs); field("seg_gs", c.SegGs);
    field("eflags_if", c.EFlags >> 9 & 1);
    field("debug_zero", !(c.Dr0 | c.Dr1 | c.Dr2 | c.Dr3 | c.Dr6 | c.Dr7)); end();

    LONG frozen = spin_count;
    DWORD resumed_spin = ResumeThread(spin);
    while (spin_count == frozen) SwitchToThread();
    SuspendThread(spin);
    static CONTEXT again;
    again.ContextFlags = CONTEXT_ALL;
    r = GetThreadContext(spin, &again);
    row("context.running.again"); field("ret", r != 0); field("count_moved", spin_count != frozen);
    field("gpr_planted", gpr_planted(&again)); field("xmm_planted", xmm_planted(&again));
    field("rip_in_loop", in_loop(again.Rip)); end();
    ResumeThread(spin);
    spin_stop = 1;
    WaitForSingleObject(spin, INFINITE);
    CloseHandle(spin);

    /* a thread suspended in a wait */
    waiter_started = CreateEventW(NULL, FALSE, FALSE, NULL);
    waiter_go = CreateEventW(NULL, FALSE, FALSE, NULL);
    HANDLE wait = CreateThread(NULL, 0, waiter, NULL, 0, NULL);
    WaitForSingleObject(waiter_started, INFINITE);
    Sleep(20);                                       /* into its wait */
    SuspendThread(wait);
    static CONTEXT w;
    w.ContextFlags = CONTEXT_CONTROL | CONTEXT_INTEGER;
    SetLastError(SENTINEL);
    r = GetThreadContext(wait, &w);
    err = GetLastError();
    ULONG_PTR image_lo, image_hi, ntdll_lo, ntdll_hi, kb_lo, kb_hi, k32_lo, k32_hi;
    module_range(NULL, &image_lo, &image_hi);
    module_range(L"ntdll.dll", &ntdll_lo, &ntdll_hi);
    module_range(L"kernelbase.dll", &kb_lo, &kb_hi);
    module_range(L"kernel32.dll", &k32_lo, &k32_hi);
    row("context.waiting"); field("ret", r != 0); field("err", err); field("flags", w.ContextFlags);
    field("rip_in_image", w.Rip >= image_lo && w.Rip < image_hi);
    field("rip_in_ntdll", w.Rip >= ntdll_lo && w.Rip < ntdll_hi);
    field("rip_in_kernelbase", w.Rip >= kb_lo && w.Rip < kb_hi);
    field("rip_in_kernel32", w.Rip >= k32_lo && w.Rip < k32_hi);
    field("rsp_in_stack", w.Rsp >= wait_limit && w.Rsp < wait_base); field("rsp_below_frame", w.Rsp < wait_frame);
    end();
    SetLastError(SENTINEL);
    DWORD resumed_wait = ResumeThread(wait);
    err = GetLastError();
    row("resume"); field("spinner", resumed_spin); field("waiter", resumed_wait); field("err", err); end();
    SetEvent(waiter_go);
    WaitForSingleObject(wait, INFINITE);
    CloseHandle(wait);
    ExitProcess(0);
}
