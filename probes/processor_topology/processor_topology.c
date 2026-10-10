// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* processor_topology.c — the processor topology a process can see (Sprint 047, family "processor topology").
 *
 * No C runtime. UnityPlayer.dll calls GetLogicalProcessorInformationEx(RelationGroup, NULL, &length) at
 * 0x18051750d (look-ahead 2026-10-10) and imports GetLogicalProcessorInformation, GetProcessAffinityMask,
 * SetThreadAffinityMask and GetNumaHighestNodeNumber. The runners differ in processors and caches, so the
 * rows hold what is structural (relationship codes, entry sizes, required lengths, errors) exactly and the
 * rest as relations to n = GetActiveProcessorCount(ALL_PROCESSOR_GROUPS): masks' bit counts, entry counts.
 */
#include <windows.h>

#define SENTINEL 0xDEADu

static char out[32768];
static unsigned n;
static unsigned char buf[65536];

static void put(const char *s) { while (*s) out[n++] = *s++; }
static void putu(unsigned long long v) {
    char t[24];
    int k = 0;
    do { t[k++] = (char)('0' + v % 10); v /= 10; } while (v);
    while (k) out[n++] = t[--k];
}
static void row(const char *s) { put("{\"s\":\""); put(s); put("\""); }
static void rowr(const char *s, unsigned r) { put("{\"s\":\""); put(s); putu(r); put("\""); }
static void field(const char *k, unsigned long long v) { put(",\""); put(k); put("\":"); putu(v); }
static void flag(const char *k, int v) { put(",\""); put(k); put(v ? "\":true" : "\":false"); }
static void end(void) { put("}\n"); }
static unsigned bits(unsigned long long m) { unsigned c = 0; while (m) { c += m & 1; m >>= 1; } return c; }

static unsigned active;

static void relation(unsigned r) {
    DWORD len = 0;
    SetLastError(SENTINEL);
    BOOL ok = GetLogicalProcessorInformationEx((LOGICAL_PROCESSOR_RELATIONSHIP)r, NULL, &len);
    DWORD err = GetLastError();
    rowr("ex.size_query.", r); field("ret", ok != 0); field("err", err); flag("length_set", len != 0); end();
    if (ok || len == 0 || len > sizeof buf) return;
    DWORD need = len, shorter = len - 1;
    SetLastError(SENTINEL);
    ok = GetLogicalProcessorInformationEx((LOGICAL_PROCESSOR_RELATIONSHIP)r, (PSYSTEM_LOGICAL_PROCESSOR_INFORMATION_EX)buf, &shorter);
    err = GetLastError();
    rowr("ex.short_buffer.", r); field("ret", ok != 0); field("err", err); flag("length_is_need", shorter == need); end();
    len = need;
    SetLastError(SENTINEL);
    ok = GetLogicalProcessorInformationEx((LOGICAL_PROCESSOR_RELATIONSHIP)r, (PSYSTEM_LOGICAL_PROCESSOR_INFORMATION_EX)buf, &len);
    err = GetLastError();
    unsigned entries = 0, cores = 0, packages = 0, caches = 0, numa = 0, groups = 0, sizes_ok = 1, masks_ok = 1;
    unsigned core_bits = 0, package_bits = 0, numa_bits = 0, l1d = 0, l1i = 0, l2 = 0, l3 = 0, smt = 0;
    unsigned group_ok = 1, max_groups = 0, active_groups = 0;
    for (DWORD off = 0; ok && off < len;) {
        PSYSTEM_LOGICAL_PROCESSOR_INFORMATION_EX e = (PSYSTEM_LOGICAL_PROCESSOR_INFORMATION_EX)(buf + off);
        entries++;
        switch (e->Relationship) {
        case RelationProcessorCore:
            cores++; core_bits += bits(e->Processor.GroupMask[0].Mask); smt |= e->Processor.Flags == LTP_PC_SMT;
            sizes_ok &= e->Size == 48; masks_ok &= e->Processor.GroupCount == 1 && e->Processor.GroupMask[0].Group == 0;
            break;
        case RelationProcessorPackage:
            packages++; package_bits += bits(e->Processor.GroupMask[0].Mask);
            sizes_ok &= e->Size == 48; masks_ok &= e->Processor.GroupCount == 1;
            break;
        case RelationCache:
            caches++;
            if (e->Cache.Level == 1 && e->Cache.Type == CacheData) l1d++;
            if (e->Cache.Level == 1 && e->Cache.Type == CacheInstruction) l1i++;
            if (e->Cache.Level == 2) l2++;
            if (e->Cache.Level == 3) l3++;
            sizes_ok &= e->Size == 48; masks_ok &= e->Cache.GroupCount <= 1 && e->Cache.LineSize == 64 && e->Cache.CacheSize != 0;
            break;
        case RelationNumaNode:
            numa++; numa_bits += bits(e->NumaNode.GroupMask.Mask);
            sizes_ok &= e->Size == 48; masks_ok &= e->NumaNode.NodeNumber == 0;
            break;
        case RelationGroup:
            groups++; max_groups = e->Group.MaximumGroupCount; active_groups = e->Group.ActiveGroupCount;
            sizes_ok &= e->Size == 80;
            group_ok &= e->Group.GroupInfo[0].ActiveProcessorCount == active &&
                        bits(e->Group.GroupInfo[0].ActiveProcessorMask) == active &&
                        e->Group.GroupInfo[0].MaximumProcessorCount >= active;
            break;
        default:
            break;
        }
        if (e->Size == 0) break;
        off += e->Size;
    }
    rowr("ex.query.", r); field("ret", ok != 0); field("err", err); flag("length_unchanged", len == need);
    field("length_over_entries", entries ? need / entries : 0);
    field("cores", cores ? 1 : 0); flag("core_bits_are_n", core_bits == (cores ? active : 0)); flag("smt", smt);
    flag("cores_le_n", cores <= active); field("packages", packages); flag("package_bits_are_n", package_bits == (packages ? active : 0));
    field("caches_present", caches != 0); field("l1d_present", l1d != 0); field("l1i_present", l1i != 0);
    field("l2_present", l2 != 0); field("l3_present", l3 != 0);
    field("numa_nodes", numa); flag("numa_bits_are_n", numa_bits == (numa ? active : 0));
    field("groups", groups); field("max_groups", max_groups); field("active_groups", active_groups); flag("group_ok", group_ok);
    flag("sizes_ok", sizes_ok); flag("masks_ok", masks_ok);
    end();
}

void start(void) {
    active = GetActiveProcessorCount(ALL_PROCESSOR_GROUPS);
    row("active_count"); flag("over_zero", active > 0); flag("at_most_64", active <= 64); end();
    static const unsigned R[] = {RelationProcessorCore, RelationNumaNode, RelationCache, RelationProcessorPackage,
                                 RelationGroup, 5, 6, 7, RelationAll};
    for (unsigned i = 0; i < sizeof R / sizeof R[0]; i++) relation(R[i]);

    DWORD len = 0;
    SetLastError(SENTINEL);
    BOOL ok = GetLogicalProcessorInformationEx((LOGICAL_PROCESSOR_RELATIONSHIP)8, NULL, &len);
    DWORD err = GetLastError();
    row("ex.relation_8"); field("ret", ok != 0); field("err", err); field("length", len); end();
    SetLastError(SENTINEL);
    ok = GetLogicalProcessorInformationEx(RelationGroup, NULL, NULL);
    err = GetLastError();
    row("ex.null_length"); field("ret", ok != 0); field("err", err); end();

    /* the older call: an array of 32-byte entries over all relations */
    len = 0;
    SetLastError(SENTINEL);
    ok = GetLogicalProcessorInformation(NULL, &len);
    err = GetLastError();
    row("classic.size_query"); field("ret", ok != 0); field("err", err); flag("multiple_of_32", len % 32 == 0 && len); end();
    DWORD need = len;
    SetLastError(SENTINEL);
    ok = GetLogicalProcessorInformation((PSYSTEM_LOGICAL_PROCESSOR_INFORMATION)buf, &len);
    err = GetLastError();
    unsigned cores = 0, core_bits = 0, packages = 0, numa = 0;
    for (DWORD i = 0; ok && i < len / 32; i++) {
        PSYSTEM_LOGICAL_PROCESSOR_INFORMATION e = (PSYSTEM_LOGICAL_PROCESSOR_INFORMATION)buf + i;
        if (e->Relationship == RelationProcessorCore) { cores++; core_bits += bits(e->ProcessorMask); }
        if (e->Relationship == RelationProcessorPackage) packages++;
        if (e->Relationship == RelationNumaNode) numa++;
    }
    row("classic.query"); field("ret", ok != 0); field("err", err); flag("length_unchanged", len == need);
    flag("core_bits_are_n", core_bits == active); flag("cores_le_n", cores <= active); field("packages", packages); field("numa_nodes", numa); end();

    DWORD_PTR process = 0, system = 0;
    SetLastError(SENTINEL);
    ok = GetProcessAffinityMask(GetCurrentProcess(), &process, &system);
    err = GetLastError();
    row("process_affinity"); field("ret", ok != 0); field("err", err); flag("process_bits_are_n", bits(process) == active);
    flag("same_as_system", process == system); flag("low_bits", process == (active == 64 ? ~0ull : (1ull << active) - 1)); end();
    SetLastError(SENTINEL);
    DWORD_PTR old = SetThreadAffinityMask(GetCurrentThread(), 1);
    err = GetLastError();
    row("thread_affinity.set_1"); flag("old_is_process", old == process); field("err", err); end();
    SetLastError(SENTINEL);
    old = SetThreadAffinityMask(GetCurrentThread(), process);
    err = GetLastError();
    row("thread_affinity.restore"); field("old", old); field("err", err); end();
    SetLastError(SENTINEL);
    old = SetThreadAffinityMask(GetCurrentThread(), 0);
    err = GetLastError();
    row("thread_affinity.zero"); field("old", old); field("err", err); end();
    ULONG highest = 0xA5A5;
    SetLastError(SENTINEL);
    ok = GetNumaHighestNodeNumber(&highest);
    err = GetLastError();
    row("numa_highest"); field("ret", ok != 0); field("err", err); field("highest", highest); end();

    DWORD written;
    WriteFile(GetStdHandle(STD_OUTPUT_HANDLE), out, n, &written, NULL);
    ExitProcess(0);
}
