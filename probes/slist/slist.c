// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* slist.c — the interlocked singly linked lists UnityPlayer.dll uses (Sprint 063, family "interlocked lists").
 *
 * No C runtime: entry `start`, kernel32 imports only. Game run 38206263976 calls InitializeSListHead on its own
 * 16-byte headers (seq 1570-1574, 2723-2725, 3283-3285), then InterlockedPopEntrySList (seq 2705 on),
 * InterlockedFlushSList (seq 3002 on) and InterlockedPushEntrySList (seq 3210 on), from four threads; 40,403 pushes,
 * 40,870 flushes and 9,326 pops in 180 s (jit_capture.bin.imports.tsv). Microsoft Learn, InterlockedPushEntrySList:
 * "The return value is the previous first item in the list. If the list was previously empty, the return value is
 * NULL", and "All list items must be aligned on a MEMORY_ALLOCATION_ALIGNMENT boundary; otherwise, this function
 * will behave unpredictably."
 *
 * Each row prints the header's two quadwords after the call as the fields Windows keeps in them: the low quadword's
 * low 16 bits (depth) and the rest (sequence), the high quadword's low 4 bits and whether the rest is the expected
 * first entry. Addresses are printed as relations; the last error after each call against a sentinel.
 */
#include <windows.h>

#define SENTINEL 0xDEADu
#define MANY 500

static char out[16384];
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
static void flag(const char *k, int v) { put(",\""); put(k); put(v ? "\":true" : "\":false"); }
static void end(void) { put("}\n"); }

typedef struct __declspec(align(16)) { SLIST_ENTRY entry; ULONG_PTR value; } item_t;

static __declspec(align(16)) unsigned long long head[2];
static item_t items[8];
static item_t many[2 * MANY];
static __declspec(align(16)) unsigned long long head2[2];
static unsigned char seen[2 * MANY];

/* the header after a call: depth, sequence, the low 4 bits of the high quadword, and its first entry */
static void header(const void *first) {
    field("depth", head[0] & 0xFFFF);
    field("sequence", head[0] >> 16);
    field("low_bits", head[1] & 0xF);
    flag("first_is", (head[1] & ~0xFull) == (unsigned long long)(ULONG_PTR)first);
}

static DWORD WINAPI pusher(LPVOID param) {
    (void)param;
    for (int i = 0; i < MANY; i++) InterlockedPushEntrySList((PSLIST_HEADER)head2, &many[MANY + i].entry);
    return 0;
}

void start(void) {
    for (int i = 0; i < 16; i++) ((unsigned char *)head)[i] = 0xA5;
    for (int i = 0; i < 8; i++) {
        items[i].entry.Next = (PSLIST_ENTRY)(ULONG_PTR)0xA5A5A5A5A5A5A5A0ull;
        items[i].value = (ULONG_PTR)i;
    }

    SetLastError(SENTINEL);
    InitializeSListHead((PSLIST_HEADER)head);
    row("init"); field("err", GetLastError()); field("low", head[0]); field("high", head[1]); end();

    SetLastError(SENTINEL);
    PSLIST_ENTRY r = InterlockedPopEntrySList((PSLIST_HEADER)head);
    row("pop.empty"); flag("null", r == NULL); field("err", GetLastError()); header(NULL); end();

    SetLastError(SENTINEL);
    r = InterlockedFlushSList((PSLIST_HEADER)head);
    row("flush.empty"); flag("null", r == NULL); field("err", GetLastError()); header(NULL); end();

    for (int i = 0; i < 3; i++) {
        SetLastError(SENTINEL);
        r = InterlockedPushEntrySList((PSLIST_HEADER)head, &items[i].entry);
        row(i == 0 ? "push.1" : i == 1 ? "push.2" : "push.3");
        flag("returns_previous", r == (i ? &items[i - 1].entry : NULL));
        field("err", GetLastError());
        header(&items[i]);
        flag("entry_links_previous", items[i].entry.Next == (i ? &items[i - 1].entry : NULL));
        end();
    }

    SetLastError(SENTINEL);
    r = InterlockedPopEntrySList((PSLIST_HEADER)head);
    row("pop.1"); flag("returns_first", r == &items[2].entry); field("err", GetLastError()); header(&items[1]);
    flag("popped_keeps_next", items[2].entry.Next == &items[1].entry); end();

    SetLastError(SENTINEL);
    r = InterlockedFlushSList((PSLIST_HEADER)head);
    row("flush.full"); flag("returns_first", r == &items[1].entry); field("err", GetLastError()); header(NULL);
    flag("chain_kept", items[1].entry.Next == &items[0].entry && items[0].entry.Next == NULL); end();

    SetLastError(SENTINEL);
    r = InterlockedPopEntrySList((PSLIST_HEADER)head);
    row("pop.after_flush"); flag("null", r == NULL); field("err", GetLastError()); header(NULL); end();

    SetLastError(SENTINEL);
    r = InterlockedPushEntrySList((PSLIST_HEADER)head, &items[3].entry);
    row("push.after_flush"); flag("returns_previous", r == NULL); field("err", GetLastError()); header(&items[3]);
    end();

    /* a header that holds an entry, initialized again: the game re-initializes its headers (seq 3283-3285) */
    InitializeSListHead((PSLIST_HEADER)head);
    row("init.again"); field("low", head[0]); field("high", head[1]); end();

    /* two threads push onto one list at once, as the game's threads do */
    InitializeSListHead((PSLIST_HEADER)head2);
    HANDLE t = CreateThread(NULL, 0, pusher, NULL, 0, NULL);
    for (int i = 0; i < MANY; i++) InterlockedPushEntrySList((PSLIST_HEADER)head2, &many[i].entry);
    WaitForSingleObject(t, INFINITE);
    unsigned long long depth = head2[0] & 0xFFFF, sequence = head2[0] >> 16;
    unsigned popped = 0;
    int distinct = 1;
    while ((r = InterlockedPopEntrySList((PSLIST_HEADER)head2)) != NULL) {
        unsigned i = (unsigned)((item_t *)r - many);
        if (i >= 2 * MANY || seen[i]) distinct = 0; else seen[i] = 1;
        popped++;
    }
    row("threads.push"); field("depth", depth); field("sequence", sequence); field("popped", popped);
    flag("distinct", distinct); field("depth_after", head2[0] & 0xFFFF); field("sequence_after", head2[0] >> 16);
    end();

    DWORD written;
    WriteFile(GetStdHandle(STD_OUTPUT_HANDLE), out, n, &written, NULL);
    ExitProcess(0);
}
