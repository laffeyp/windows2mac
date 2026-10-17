// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* process.c — the process queries the C runtime makes first (Sprint 044, family process_queries).
 *
 * GetSystemTimeAsFileTime, GetCurrentProcessId, QueryPerformanceCounter, QueryPerformanceFrequency,
 * IsProcessorFeaturePresent, InitializeSListHead, GetStartupInfoW, IsDebuggerPresent. The behavior
 * is contracts/windows.process.queries/0.1.0.json, whose rows come from four runner machines
 * (Server 2025 and 2022; AMD EPYC 7763 and 9V74, Intel Xeon 8370C; store/probe-runs/380378*, 380379*,
 * 38038027558).
 *
 * The counter stays in the guest (Decision 2026-10-09, hot imports: 16,912,633 QueryPerformanceCounter
 * calls in a 180 s game run): CNTVCT_EL0 scaled to the 10 MHz frequency every observed Windows
 * reports. The wall clock asks the host once, then counts from the same counter. With the monitor's
 * clock mode 1 (lockstep), both come from the host's fixed clock instead.
 */
#include "runtime.h"

#define QPC_FREQUENCY 10000000ull     /* QueryPerformanceFrequency on every observed runner */
#define NO_CPUID 0xFFFFFFFFu

static u64 clock_mode(void) { return *(const volatile u64 *)(W2M_MONITOR_DATA_VA + W2M_CLOCK_MODE_OFF); }
static int clock_from_host(void) { return clock_mode() == 1; }
static u64 monitor_word(u64 off) { return *(const volatile u64 *)(W2M_MONITOR_DATA_VA + off); }

static u64 cntvct(void) {
    u64 v;
    __asm__ volatile("isb\n\tmrs %0, cntvct_el0" : "=r"(v));    /* newline: ";" starts a comment in Apple's assembler */
    return v;
}

static u64 cntfrq(void) {
    u64 v;
    __asm__ volatile("mrs %0, cntfrq_el0" : "=r"(v));
    return v;
}

/* ticks of `from` Hz as ticks of `to` Hz, without 128-bit division (no compiler runtime here). */
static u64 rescale(u64 ticks, u64 from, u64 to) {
    return ticks / from * to + ticks % from * to / from;
}

static u64 counter_now(void) {
    if (clock_from_host()) return w2m_host(W2M_HOST_CLOCK, 0, 0, 0, 0, 0);
    if (clock_mode() == 2)                 /* the instructions this process ran at a set rate, plus the idle warp */
        return rescale(monitor_word(0), monitor_word(W2M_CLOCK_RATE_OFF), QPC_FREQUENCY) + *(const volatile u64 *)W2M_CLOCK_WARP_VA;
    return rescale(cntvct(), cntfrq(), QPC_FREQUENCY);
}

/* The 10 MHz counter (QueryPerformanceCounter's; system_info.c's tick count too). */
u64 w2m_counter_now(void) { return counter_now(); }

/* "Retrieves the current value of the performance counter"; nonzero on success. */
u64 k32_QueryPerformanceCounter(u64 out) {
    if (out == 0) w2m_unimplemented(W2M_API_QueryPerformanceCounter, 0);
    *(u64 *)out = counter_now();
    return w2m_logged(W2M_API_QueryPerformanceCounter, out, 1);
}

/* "Retrieves the frequency of the performance counter ... fixed at system boot"; nonzero on success. */
u64 k32_QueryPerformanceFrequency(u64 out) {
    if (out == 0) w2m_unimplemented(W2M_API_QueryPerformanceFrequency, 0);
    *(u64 *)out = QPC_FREQUENCY;
    return w2m_logged(W2M_API_QueryPerformanceFrequency, out, 1);
}

static u64 base_filetime, base_counter;

/* The system time as 100 ns intervals since 1601-01-01: the host's wall clock once, then the counter
 * (thread_queries.c reads it too). */
u64 w2m_filetime_now(void) {
    if (clock_from_host()) return w2m_host(W2M_HOST_FILETIME, 0, 0, 0, 0, 0);
    u64 c = counter_now();                                          /* 10 MHz: one tick is 100 ns */
    if (base_filetime == 0) {
        base_filetime = clock_mode() == 2 ? monitor_word(W2M_CLOCK_EPOCH_OFF) : w2m_host(W2M_HOST_FILETIME, 0, 0, 0, 0, 0);
        base_counter = c;
    }
    return base_filetime + (c - base_counter);
}

/* The first time the process read the clock: thread_queries.c's creation time. */
u64 w2m_filetime_start(void) {
    if (base_filetime == 0) (void)w2m_filetime_now();
    return base_filetime;
}

/* "Retrieves the current system date and time ... in UTC" as 100 ns intervals since 1601-01-01. */
W2M_VOID u64 k32_GetSystemTimeAsFileTime(u64 out) {
    u64 now = w2m_filetime_now();
    *(u32 *)out = (u32)now;                                    /* FILETIME: dwLowDateTime, dwHighDateTime */
    *(u32 *)(out + 4) = (u32)(now >> 32);
    return w2m_logged(W2M_API_GetSystemTimeAsFileTime, out, 0);
}

/* "Retrieves the process identifier of the calling process": TEB.ClientId.UniqueProcess. */
u64 k32_GetCurrentProcessId(void) {
    return w2m_logged(W2M_API_GetCurrentProcessId, 0, (u32)w2m_teb()->client_process);
}

/* PF_* -> the guest CPUID bit that names the feature (leaf, register EAX..EDX = 0..3, bit), or the
 * answer every observed Windows gives when no CPUID bit names it. Rows: ipfp.* in the contract.
 * PF_VIRT_FIRMWARE_ENABLED (21) is 0: the guest is offered no virtualization (the runners differ on
 * it, so the contract excludes its row); PF_BMI2 (60) follows CPUID as Server 2025 does. */
struct feature { u32 pf, leaf, reg, bit, fixed; };
static const struct feature FEATURES[] = {
    {0, NO_CPUID, 0, 0, 0},  {1, NO_CPUID, 0, 0, 0},  {2, 1, 3, 8, 0},   {3, 1, 3, 23, 0},  {6, 1, 3, 25, 0},
    {7, NO_CPUID, 0, 0, 0},  {8, 1, 3, 4, 0},         {9, NO_CPUID, 0, 0, 1},              {10, 1, 3, 26, 0},
    {12, NO_CPUID, 0, 0, 1}, {13, 1, 2, 0, 0},        {14, 1, 2, 13, 0}, {15, NO_CPUID, 0, 0, 0},
    {16, NO_CPUID, 0, 0, 0}, {17, 1, 2, 26, 0},       {20, NO_CPUID, 0, 0, 0},              {21, NO_CPUID, 0, 0, 0},
    {22, 7, 1, 0, 0},        {23, NO_CPUID, 0, 0, 1}, {36, 1, 2, 9, 0},  {37, 1, 2, 19, 0}, {38, 1, 2, 20, 0},
    {39, 1, 2, 28, 0},       {40, 7, 1, 5, 0},        {41, 7, 1, 16, 0}, {60, 7, 1, 8, 0},  {61, 7, 2, 28, 0},
    {89, 7, 2, 5, 0},
};

/* "If the feature is supported, the return value is a nonzero value"; a feature the table does not
 * name stops the run instead of being guessed. */
u64 k32_IsProcessorFeaturePresent(u64 pf) {
    pf = (u32)pf;
    for (unsigned i = 0; i < sizeof FEATURES / sizeof FEATURES[0]; i++) {
        const struct feature *f = &FEATURES[i];
        if (f->pf != pf) continue;
        u64 present = f->fixed;
        if (f->leaf != NO_CPUID) {
            u32 r[4];
            w2m_cpuid_query(f->leaf, 0, r);
            present = r[f->reg] >> f->bit & 1;
        }
        return w2m_logged(W2M_API_IsProcessorFeaturePresent, pf, present);
    }
    w2m_unimplemented(W2M_API_IsProcessorFeaturePresent, pf);
    return 0;
}

/* "Initializes the head of a singly linked list": all 16 bytes zero (row slist.initialize). */
W2M_VOID u64 k32_InitializeSListHead(u64 head) {
    for (int i = 0; i < 16; i++) ((volatile u8 *)head)[i] = 0;
    return w2m_logged(W2M_API_InitializeSListHead, head, 0);
}

static const unsigned short empty_string[1];

/* "Retrieves the contents of the STARTUPINFO structure that was specified when the calling process
 * was created." The rows (contracts windows.process.queries, windows.console.gui_start) fix: cb 104,
 * lpReserved an empty string, lpDesktop "winsta0\default", lpTitle the module's path, every count,
 * flag and position 0, lpReserved2 NULL, and hStdInput, hStdOutput, hStdError not written at all
 * when the start passed no handles (STARTF_USESTDHANDLES clear: the caller's bytes stay). */
W2M_VOID u64 k32_GetStartupInfoW(u64 si) {
    u8 *b = (u8 *)si;
    for (int i = 0; i < 80; i++) b[i] = 0;                /* through wShowWindow, cbReserved2, lpReserved2 */
    *(u32 *)si = 104;                                      /* cb */
    *(u64 *)(si + 8) = (u64)empty_string;                  /* lpReserved */
    /* ProcessParameters' (Sprint 059: rows *.strings, *.window): the desktop's and the title's buffers themselves,
     * the positions, counts, fill, flags and show state from +0x88 */
    const u8 *p = w2m_parameters();
    *(u64 *)(si + 16) = *(const u64 *)(p + 0xC8);          /* lpDesktop: DesktopInfo.Buffer */
    *(u64 *)(si + 24) = *(const u64 *)(p + 0xB8);          /* lpTitle: WindowTitle.Buffer */
    for (int i = 0; i < 8; i++) *(u32 *)(si + 32 + 4 * i) = *(const u32 *)(p + 0x88 + 4 * i);
    *(unsigned short *)(si + 64) = (unsigned short)*(const u32 *)(p + 0xA8);   /* wShowWindow */
    return w2m_logged(W2M_API_GetStartupInfoW, si, 0);
}

/* "Determines whether the calling process is being debugged": PEB.BeingDebugged (offset 2). */
u64 k32_IsDebuggerPresent(void) {
    return w2m_logged(W2M_API_IsDebuggerPresent, 0, *(const volatile u8 *)(W2M_PEB_VA + 2));
}
