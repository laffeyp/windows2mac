// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* integrity_level.c — the process token's integrity level, and LocalAlloc (Sprint 048, family "integrity level").
 *
 * No C runtime. UnityPlayer.dll's function at 0x180525ca0 (frontier 2026-10-10, 2,653,374 instructions) asks
 * whether the process runs at low integrity: OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY), then
 * GetTokenInformation(TokenIntegrityLevel) with no buffer (it accepts ERROR_INSUFFICIENT_BUFFER), LocalAlloc(LPTR)
 * of the size returned, GetTokenInformation again, GetSidSubAuthority(label's SID, 0), CloseHandle and LocalFree;
 * its answer is RID == 0x1000. This probe makes those calls in that order and records each return, last error
 * (after SetLastError(0xDEAD)) and the label: its attributes, the SID's place in the buffer, revision, count,
 * authority and RID (a runner's RID is its own; the family compares what it means), plus LocalAlloc's block:
 * alignment, zero fill, HeapSize on the process heap.
 */
#include <windows.h>

#define SENTINEL 0xDEADu

static char out[8192];
static unsigned n;

static void put(const char *s) { while (*s) out[n++] = *s++; }
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

void start(void) {
    HANDLE token = NULL;
    SetLastError(SENTINEL);
    BOOL ok = OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &token);
    DWORD err = GetLastError();
    row("open"); field("ret", ok != 0); field("err", err); field("handle_non_null", token != NULL);
    field("handle_multiple_of_4", ((ULONG_PTR)token & 3) == 0); end();

    DWORD size = 0xDEAD;
    SetLastError(SENTINEL);
    ok = GetTokenInformation(token, TokenIntegrityLevel, NULL, 0, &size);
    err = GetLastError();
    row("size_query"); field("ret", ok != 0); field("err", err); field("size", size); end();

    SetLastError(SENTINEL);
    BYTE *label = (BYTE *)LocalAlloc(LPTR, size);
    err = GetLastError();
    int zero = 1;
    for (DWORD i = 0; i < size; i++) zero &= label[i] == 0;
    row("local_alloc"); field("non_null", label != NULL); field("err", err); field("aligned_16", ((ULONG_PTR)label & 15) == 0);
    field("zeroed", zero); field("heap_size", HeapSize(GetProcessHeap(), 0, label)); end();

    DWORD got = 0xDEAD;
    SetLastError(SENTINEL);
    ok = GetTokenInformation(token, TokenIntegrityLevel, label, size - 1, &got);
    err = GetLastError();
    row("short_buffer"); field("ret", ok != 0); field("err", err); field("size", got); end();

    got = 0xDEAD;
    SetLastError(SENTINEL);
    ok = GetTokenInformation(token, TokenIntegrityLevel, label, size, &got);
    err = GetLastError();
    TOKEN_MANDATORY_LABEL *l = (TOKEN_MANDATORY_LABEL *)label;
    SID *sid = (SID *)l->Label.Sid;
    row("info"); field("ret", ok != 0); field("err", err); field("size", got);
    field("attributes", l->Label.Attributes); field("sid_offset", (BYTE *)sid - label);
    field("revision", sid->Revision); field("count", sid->SubAuthorityCount);
    put(",\"authority\":[");
    for (int i = 0; i < 6; i++) { if (i) put(","); putu(sid->IdentifierAuthority.Value[i]); }
    put("]"); field("rid", sid->SubAuthority[0]); end();

    SetLastError(SENTINEL);
    DWORD *rid = GetSidSubAuthority(sid, 0);
    err = GetLastError();
    row("sub_authority"); field("is_field", rid == &sid->SubAuthority[0]); field("err", err); end();

    SetLastError(SENTINEL);
    ok = CloseHandle(token);
    err = GetLastError();
    row("close"); field("ret", ok != 0); field("err", err); end();

    SetLastError(SENTINEL);
    HLOCAL left = LocalFree(label);
    err = GetLastError();
    row("local_free"); field("null", left == NULL); field("err", err); end();

    SetLastError(SENTINEL);
    BYTE *plain = (BYTE *)LocalAlloc(LMEM_FIXED, 40);
    err = GetLastError();
    row("local_alloc.fixed"); field("non_null", plain != NULL); field("err", err); field("aligned_16", ((ULONG_PTR)plain & 15) == 0);
    field("heap_size", HeapSize(GetProcessHeap(), 0, plain)); end();
    LocalFree(plain);
    ExitProcess(0);
}
