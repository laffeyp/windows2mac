// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* topology.c — the processor topology a process sees (Sprint 047, family "processor topology").
 *
 * GetLogicalProcessorInformationEx, GetLogicalProcessorInformation, GetActiveProcessorCount,
 * GetProcessAffinityMask, SetThreadAffinityMask, GetNumaHighestNodeNumber and GetCurrentProcess, as
 * contracts/windows.system.processor_topology records Windows answering (Server 2025 and 2022 on Intel
 * Xeon 8573C and 6973P and AMD EPYC 7763; the rows differ only by OS, in relationship 7):
 *   - a size query (no buffer) or a short buffer: FALSE, ERROR_INSUFFICIENT_BUFFER (122), the length set to
 *     what is needed; a NULL length pointer: FALSE, ERROR_INVALID_PARAMETER; a relationship past 7 other than
 *     RelationAll: FALSE, ERROR_GEN_FAILURE (31), the length left; success leaves the last error
 *   - entries of 48 bytes (core, NUMA node, package, die, module), 56 (cache) and 80 (one group); RelationAll
 *     holds cores, NUMA nodes, caches, packages, the group and modules, and no die
 *   - every mask's bits are the active processors; one group; NUMA node 0 the highest
 *   - the classic call: 32-byte entries; SetThreadAffinityMask returns the old mask, and 0 with error 87 for 0
 * w2m's machine is the runners' (row machine, Sprint 061, Decision 2026-10-11 user: match the reference): four
 * processors on two cores with SMT (masks 0x3 and 0xC), one package, die, module and NUMA node over all four, an
 * L1d, an L1i and an L2 for each core and one L3; the cache sizes are the guest CPUID profile's i3-6100, itself
 * two cores and four threads (L1d and L1i 32 KB 8-way, L2 256 KB 8-way as leaf 0x80000006 says, L3 3 MB 12-way;
 * 64-byte lines), since the runners' sizes differ by CPU and the rows keep them as relations.
 */
#include "runtime.h"

typedef unsigned short u16;

#define ERROR_GEN_FAILURE 31u
#define ERROR_INVALID_PARAMETER 87u
#define ERROR_INSUFFICIENT_BUFFER 122u
#define RELATION_ALL 0xFFFFu

#define ALL W2M_PROCESSOR_MASK                       /* runtime.h: the four processors */
static const u64 CORE_MASKS[] = {0x3, 0xC};

static void set_error(u32 code) { w2m_teb()->last_error_value = code; }
static u64 thread_mask;                              /* SetThreadAffinityMask's mask; 0 is the start's, 1 */

struct cache { u8 level, associativity; u16 line; u32 size, type; u64 mask; };   /* type: 1 instruction, 2 data, 0 unified */
static const struct cache CACHES[] = {{1, 8, 64, 32 << 10, 2, 0x3}, {1, 8, 64, 32 << 10, 1, 0x3},
                                       {2, 8, 64, 256 << 10, 0, 0x3}, {1, 8, 64, 32 << 10, 2, 0xC},
                                       {1, 8, 64, 32 << 10, 1, 0xC}, {2, 8, 64, 256 << 10, 0, 0xC},
                                       {3, 12, 64, 3 << 20, 0, ALL}};

static void zero(u8 *p, u64 n) { for (u64 i = 0; i < n; i++) p[i] = 0; }

/* One Ex entry at p (when p is not NULL); returns its size. */
static u32 processor_entry(u8 *p, u32 relationship, u64 mask) {   /* PROCESSOR_RELATIONSHIP: core, package, die, module */
    if (p) {
        zero(p, 48);
        *(u32 *)p = relationship; *(u32 *)(p + 4) = 48;
        if (relationship == 0) p[8] = 1;                      /* Flags: LTP_PC_SMT, two logical processors a core */
        *(u16 *)(p + 30) = 1;                                 /* GroupCount, after Flags, EfficiencyClass, Reserved[20] */
        *(u64 *)(p + 32) = mask;                              /* GroupMask[0].Mask; .Group 0 */
    }
    return 48;
}
static u32 numa_entry(u8 *p, u32 relationship) {
    if (p) {
        zero(p, 48);
        *(u32 *)p = relationship; *(u32 *)(p + 4) = 48;      /* NodeNumber 0 */
        *(u16 *)(p + 30) = 1;                                 /* GroupCount, after Reserved[18] */
        *(u64 *)(p + 32) = ALL;
    }
    return 48;
}
static u32 cache_entry(u8 *p, const struct cache *c) {
    if (p) {
        zero(p, 56);
        *(u32 *)p = 2; *(u32 *)(p + 4) = 56;
        p[8] = c->level; p[9] = c->associativity; *(u16 *)(p + 10) = c->line; *(u32 *)(p + 12) = c->size;
        *(u32 *)(p + 16) = c->type;
        *(u16 *)(p + 38) = 1;                                 /* GroupCount, after Reserved[18] */
        *(u64 *)(p + 40) = c->mask;
    }
    return 56;
}
static u32 group_entry(u8 *p) {
    if (p) {
        zero(p, 80);
        *(u32 *)p = 4; *(u32 *)(p + 4) = 80;
        *(u16 *)(p + 8) = 1; *(u16 *)(p + 10) = 1;            /* MaximumGroupCount, ActiveGroupCount */
        p[32] = W2M_PROCESSORS; p[33] = W2M_PROCESSORS;       /* GroupInfo[0]: Maximum and ActiveProcessorCount */
        *(u64 *)(p + 72) = ALL;                               /* ActiveProcessorMask, after Reserved[38] */
    }
    return 80;
}

/* The entries for `relationship` written at `out` (NULL: sized only); returns their total size. */
static u32 entries(u32 relationship, u8 *out) {
    u32 n = 0;
#define AT (out ? out + n : 0)
    if (relationship == 0 || relationship == RELATION_ALL)
        for (u32 i = 0; i < sizeof CORE_MASKS / sizeof CORE_MASKS[0]; i++) n += processor_entry(AT, 0, CORE_MASKS[i]);
    if (relationship == 1 || relationship == 6 || relationship == RELATION_ALL) n += numa_entry(AT, 1);   /* NumaNodeEx too: Relationship 1 (row ex.query.6) */
    if (relationship == 2 || relationship == RELATION_ALL)
        for (u32 i = 0; i < sizeof CACHES / sizeof CACHES[0]; i++) n += cache_entry(AT, &CACHES[i]);
    if (relationship == 3 || relationship == RELATION_ALL) n += processor_entry(AT, 3, ALL);
    if (relationship == 4 || relationship == RELATION_ALL) n += group_entry(AT);
    if (relationship == 5) n += processor_entry(AT, 5, ALL);
    if (relationship == 7 || relationship == RELATION_ALL) n += processor_entry(AT, 7, ALL);
#undef AT
    return n;
}

u64 k32_GetLogicalProcessorInformationEx(u64 relationship, u64 buffer, u64 length) {
    u32 r = (u32)relationship;
    if (!length) {
        set_error(ERROR_INVALID_PARAMETER);
        return w2m_logged(W2M_API_GetLogicalProcessorInformationEx, r, 0);
    }
    if (r > 7 && r != RELATION_ALL) {
        set_error(ERROR_GEN_FAILURE);
        return w2m_logged(W2M_API_GetLogicalProcessorInformationEx, r, 0);
    }
    u32 need = entries(r, 0);
    if (!buffer || *(u32 *)length < need) {
        *(u32 *)length = need;
        set_error(ERROR_INSUFFICIENT_BUFFER);
        return w2m_logged(W2M_API_GetLogicalProcessorInformationEx, r, 0);
    }
    entries(r, (u8 *)buffer);
    *(u32 *)length = need;
    return w2m_logged(W2M_API_GetLogicalProcessorInformationEx, r, 1);
}

/* The classic call: SYSTEM_LOGICAL_PROCESSOR_INFORMATION, 32 bytes each (ProcessorMask, Relationship, a
 * 16-byte union): the core, the NUMA node, the caches, the package. */
u64 k32_GetLogicalProcessorInformation(u64 buffer, u64 length) {
    if (!length) {
        set_error(ERROR_INVALID_PARAMETER);
        return w2m_logged(W2M_API_GetLogicalProcessorInformation, 0, 0);
    }
    u32 cores = sizeof CORE_MASKS / sizeof CORE_MASKS[0];
    u32 count = cores + 2 + sizeof CACHES / sizeof CACHES[0], need = 32 * count;
    if (!buffer || *(u32 *)length < need) {
        *(u32 *)length = need;
        set_error(ERROR_INSUFFICIENT_BUFFER);
        return w2m_logged(W2M_API_GetLogicalProcessorInformation, need, 0);
    }
    u8 *p = (u8 *)buffer;
    zero(p, need);
    for (u32 i = 0; i < cores; i++, p += 32) {               /* the cores: Flags LTP_PC_SMT */
        *(u64 *)p = CORE_MASKS[i]; *(u32 *)(p + 8) = 0; p[16] = 1;
    }
    *(u64 *)p = ALL; *(u32 *)(p + 8) = 1; p += 32;           /* NUMA node 0 */
    for (u32 i = 0; i < sizeof CACHES / sizeof CACHES[0]; i++, p += 32) {
        const struct cache *c = &CACHES[i];
        *(u64 *)p = c->mask; *(u32 *)(p + 8) = 2;
        p[16] = c->level; p[17] = c->associativity; *(u16 *)(p + 18) = c->line; *(u32 *)(p + 20) = c->size;
        *(u32 *)(p + 24) = c->type;
    }
    *(u64 *)p = ALL; *(u32 *)(p + 8) = 3;                    /* package */
    *(u32 *)length = need;
    return w2m_logged(W2M_API_GetLogicalProcessorInformation, need, 1);
}

u64 k32_GetActiveProcessorCount(u64 group) {
    return w2m_logged(W2M_API_GetActiveProcessorCount, group,
                      ((u16)group == 0 || (u16)group == 0xFFFF) ? W2M_PROCESSORS : 0);
}

u64 k32_GetCurrentProcess(void) { return w2m_logged(W2M_API_GetCurrentProcess, 0, ~0ull); }   /* the pseudo-handle -1 */

u64 k32_GetProcessAffinityMask(u64 process, u64 process_mask, u64 system_mask) {
    if (process != ~0ull) w2m_unimplemented(W2M_API_GetProcessAffinityMask, process);     /* another process */
    *(u64 *)process_mask = ALL;
    *(u64 *)system_mask = ALL;
    return w2m_logged(W2M_API_GetProcessAffinityMask, process, 1);
}

u64 k32_SetThreadAffinityMask(u64 thread, u64 mask) {
    if (thread != ~1ull) w2m_unimplemented(W2M_API_SetThreadAffinityMask, thread);       /* another thread */
    if (!mask || (mask & ~ALL)) {                            /* row thread_affinity.zero; bits outside the process */
        set_error(ERROR_INVALID_PARAMETER);
        return w2m_logged(W2M_API_SetThreadAffinityMask, mask, 0);
    }
    u64 old = thread_mask ? thread_mask : ALL;
    thread_mask = mask;
    return w2m_logged(W2M_API_SetThreadAffinityMask, mask, old);
}

u64 k32_GetNumaHighestNodeNumber(u64 highest) {
    *(u32 *)highest = 0;
    return w2m_logged(W2M_API_GetNumaHighestNodeNumber, 0, 1);
}
