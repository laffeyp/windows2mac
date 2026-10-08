/* event_auto_reset.c — probe for windows.event.auto_reset.single_signal@0.1.0 (Sprint 007).
 *
 * main creates an auto-reset event (non-signaled), starts waiters a and b, each calling
 * WaitForSingleObject(event, 2000). When both have announced themselves, main polls the kernel's
 * view of both threads (NtQuerySystemInformation(SystemProcessInformation)) until each shows
 * ThreadState 5 (Wait) with WaitReason 6 (UserRequest), at most 500 polls 1 ms apart, records one
 * GUEST_THREAD_STATE_OBSERVED line per waiter (vocabulary 0.5), then calls SetEvent once, joins
 * both, and calls WaitForSingleObject(event, 0). Sprint 028 replaced the earlier Sleep(200)
 * assumption with this witness: between announcing itself and WaitForSingleObject a waiter calls
 * nothing that can block, so Wait/UserRequest there means it is inside that wait. Each thread buffers its own lines; main writes the file after
 * the joins. The released waiter's wait return carries an hb edge to main's SetEvent line;
 * main's joins carry hb edges to each waiter's exit line.
 *
 * Usage: event_auto_reset.exe <execution_id> <trace_path> <roles_path>
 */
#define _CRT_SECURE_NO_WARNINGS /* MSVC CRT deprecates fopen/snprintf */
#include <windows.h>
#include <bcrypt.h>
#include <stdarg.h>
#include <stdio.h>
#include <string.h>

#define LINELEN 640
#define MAXLINES 12

/* Layouts from Microsoft Learn, NtQuerySystemInformation (winternl.h), x64. Declared here so the
 * probe does not depend on which members a given SDK's winternl.h exposes. The function "may be
 * altered or unavailable in future versions", so it is linked at run time, as the page directs. */
typedef struct {
    ULONG NextEntryOffset;
    ULONG NumberOfThreads;
    BYTE Reserved1[48];
    USHORT NameLength, NameMaximumLength;
    PWSTR NameBuffer;
    LONG BasePriority;
    HANDLE UniqueProcessId;
    HANDLE InheritedFromUniqueProcessId;
    ULONG HandleCount;
    ULONG SessionId;
    PVOID Reserved3;
    SIZE_T PeakVirtualSize, VirtualSize;
    ULONG Reserved4;
    SIZE_T PeakWorkingSetSize, WorkingSetSize;
    PVOID Reserved5;
    SIZE_T QuotaPagedPoolUsage;
    PVOID Reserved6;
    SIZE_T QuotaNonPagedPoolUsage, PagefileUsage, PeakPagefileUsage, PrivatePageCount;
    LARGE_INTEGER Reserved7[6];
} W2M_PROCESS_INFO; /* followed by NumberOfThreads W2M_THREAD_INFO entries */
typedef struct {
    LARGE_INTEGER Reserved1[3];
    ULONG Reserved2;
    PVOID StartAddress;
    HANDLE UniqueProcess, UniqueThread; /* CLIENT_ID */
    LONG Priority;
    LONG BasePriority;
    ULONG Reserved3;
    ULONG ThreadState; /* .NET System.Diagnostics.ThreadState: 5 = Wait */
    ULONG WaitReason;  /* .NET ThreadWaitReason: 6 = UserRequest; valid only in Wait */
} W2M_THREAD_INFO;
_Static_assert(sizeof(W2M_PROCESS_INFO) == 256, "SYSTEM_PROCESS_INFORMATION x64 size");
_Static_assert(sizeof(W2M_THREAD_INFO) == 80, "SYSTEM_THREAD_INFORMATION x64 size");
typedef LONG(NTAPI *NtQsiFn)(ULONG, PVOID, ULONG, PULONG);
#define SYSTEM_PROCESS_INFORMATION_CLASS 5
#define STATUS_INFO_LENGTH_MISMATCH ((LONG)0xC0000004)

typedef struct {
    DWORD tid;
    int seq;
    int n;
    char lines[MAXLINES][LINELEN];
} Tr;

static Tr tr_main, tr_a, tr_b;
static HANDLE g_event;
static volatile LONG g_ready = 0;
static volatile LONG g_setevent_seq = -1;

static void sha256_hex(const char *s, char out[72]) {
    UCHAR h[32];
    BCryptHash(BCRYPT_SHA256_ALG_HANDLE, NULL, 0, (PUCHAR)s, (ULONG)strlen(s), h, 32);
    strcpy(out, "sha256:");
    for (int i = 0; i < 32; i++) sprintf(out + 7 + 2 * i, "%02x", h[i]);
}

/* Append one guest line for thread t: tag, thread_id, thread_seq, hb_refs, then the rest. */
static void ev(Tr *t, const char *tag, const char *hb, const char *restfmt, ...) {
    char rest[LINELEN];
    va_list ap;
    va_start(ap, restfmt);
    vsnprintf(rest, sizeof rest, restfmt, ap);
    va_end(ap);
    snprintf(t->lines[t->n++], LINELEN,
             "{\"tag\":\"%s\",\"thread_id\":%lu,\"thread_seq\":%d,\"hb_refs\":[%s],%s}",
             tag, t->tid, t->seq++, hb, rest);
}

static void call(Tr *t, const char *hb, const char *api, const char *args, long long ret,
                 DWORD last_error) {
    char h[72];
    sha256_hex(args, h);
    ev(t, "GUEST_CALL_RETURNED", hb,
       "\"api\":\"%s\",\"args_sha256\":\"%s\",\"return_value\":%lld,\"last_error\":%lu",
       api, h, ret, last_error);
}

/* Fill state[i], reason[i] for each of n thread ids of this process; return how many were found. */
static int thread_states(NtQsiFn q, const DWORD *tids, ULONG *state, ULONG *reason, int n) {
    static BYTE *buf;
    static ULONG cap = 1 << 20;
    ULONG need = 0;
    LONG st;
    for (;;) {
        if (!buf) buf = (BYTE *)VirtualAlloc(NULL, cap, MEM_COMMIT | MEM_RESERVE, PAGE_READWRITE);
        if (!buf) return 0;
        st = q(SYSTEM_PROCESS_INFORMATION_CLASS, buf, cap, &need);
        if (st != STATUS_INFO_LENGTH_MISMATCH) break;
        VirtualFree(buf, 0, MEM_RELEASE);
        buf = NULL;
        cap = need + (1 << 16);
    }
    if (st < 0) return 0;
    DWORD pid = GetCurrentProcessId();
    int found = 0;
    for (BYTE *e = buf;; e += ((W2M_PROCESS_INFO *)e)->NextEntryOffset) {
        W2M_PROCESS_INFO *pi = (W2M_PROCESS_INFO *)e;
        if ((DWORD)(ULONG_PTR)pi->UniqueProcessId == pid) {
            W2M_THREAD_INFO *ti = (W2M_THREAD_INFO *)(pi + 1);
            for (ULONG k = 0; k < pi->NumberOfThreads; k++)
                for (int i = 0; i < n; i++)
                    if ((DWORD)(ULONG_PTR)ti[k].UniqueThread == tids[i]) {
                        state[i] = ti[k].ThreadState;
                        reason[i] = ti[k].WaitReason;
                        found++;
                    }
            return found;
        }
        if (!pi->NextEntryOffset) return 0;
    }
}

static DWORD WINAPI waiter(LPVOID p) {
    Tr *t = (Tr *)p;
    char hb[64];
    t->tid = GetCurrentThreadId();
    snprintf(hb, sizeof hb, "\"%lu:0\"", tr_main.tid);
    ev(t, "GUEST_THREAD_STARTED", hb,
       "\"parent_thread_id\":%lu,\"start_address\":%llu,\"suspended\":false",
       tr_main.tid, (unsigned long long)(ULONG_PTR)waiter);
    InterlockedIncrement(&g_ready);
    DWORD r = WaitForSingleObject(g_event, 2000);
    DWORD le = (r == WAIT_FAILED) ? GetLastError() : 0;
    hb[0] = 0;
    if (r == WAIT_OBJECT_0) snprintf(hb, sizeof hb, "\"%lu:%ld\"", tr_main.tid, g_setevent_seq);
    call(t, hb, "kernel32!WaitForSingleObject", "WaitForSingleObject(event,2000)",
         (long long)r, le);
    ev(t, "GUEST_THREAD_EXITED", "", "\"exit_code\":0");
    return 0;
}

static void profile(char *out, size_t n) {
    DWORD ubr = 0, sz = sizeof ubr;
    wchar_t wtype[64] = L"unknown";
    DWORD tsz = sizeof wtype;
    char type[64];
    RegGetValueW(HKEY_LOCAL_MACHINE, L"SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion", L"UBR",
                 RRF_RT_REG_DWORD, NULL, &ubr, &sz);
    RegGetValueW(HKEY_LOCAL_MACHINE, L"SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion",
                 L"InstallationType", RRF_RT_REG_SZ, NULL, wtype, &tsz);
    WideCharToMultiByte(CP_UTF8, 0, wtype, -1, type, sizeof type, NULL, NULL);
    RTL_OSVERSIONINFOW ver = {sizeof(ver)};
    typedef LONG(WINAPI * RtlGetVersionFn)(PRTL_OSVERSIONINFOW);
    RtlGetVersionFn rtl =
        (RtlGetVersionFn)(void *)GetProcAddress(GetModuleHandleW(L"ntdll.dll"), "RtlGetVersion");
    if (rtl) rtl(&ver);
    snprintf(out, n, "gha-windows-2025-x64-%lu.%lu.%lu.%lu-%s", ver.dwMajorVersion,
             ver.dwMinorVersion, ver.dwBuildNumber, ubr, type);
}

int main(int argc, char **argv) {
    if (argc != 4) {
        fprintf(stderr, "usage: event_auto_reset.exe <execution_id> <trace_path> <roles_path>\n");
        return 2;
    }
    char prof[160];
    profile(prof, sizeof prof);
    tr_main.tid = GetCurrentThreadId();
    ev(&tr_main, "GUEST_THREAD_STARTED", "",
       "\"parent_thread_id\":0,\"start_address\":%llu,\"suspended\":false",
       (unsigned long long)(ULONG_PTR)main);

    g_event = CreateEventW(NULL, FALSE, FALSE, NULL);
    if (!g_event) {
        fprintf(stderr, "CreateEventW failed: %lu\n", GetLastError());
        return 1;
    }
    HANDLE ha = CreateThread(NULL, 0, waiter, &tr_a, 0, NULL);
    HANDLE hbt = CreateThread(NULL, 0, waiter, &tr_b, 0, NULL);
    if (!ha || !hbt) {
        fprintf(stderr, "CreateThread failed: %lu\n", GetLastError());
        return 1;
    }
    while (InterlockedCompareExchange(&g_ready, 0, 0) < 2) Sleep(1);

    /* Witness the precondition: both waiters in Wait (5) for UserRequest (6). */
    NtQsiFn q = (NtQsiFn)(void *)GetProcAddress(GetModuleHandleW(L"ntdll.dll"),
                                                "NtQuerySystemInformation");
    DWORD tids[2] = {GetThreadId(ha), GetThreadId(hbt)};
    ULONG state[2] = {0, 0}, reason[2] = {0, 0};
    int found = 0, polls = 0;
    while (q && polls < 500) {
        polls++;
        found = thread_states(q, tids, state, reason, 2);
        if (found == 2 && state[0] == 5 && reason[0] == 6 && state[1] == 5 && reason[1] == 6) break;
        Sleep(1);
    }
    if (found == 2)
        for (int i = 0; i < 2; i++)
            ev(&tr_main, "GUEST_THREAD_STATE_OBSERVED", "",
               "\"subject_thread_id\":%lu,\"thread_state\":%lu,\"wait_reason\":%lu,"
               "\"source\":\"ntdll!NtQuerySystemInformation(SystemProcessInformation), poll %d of 500\"",
               tids[i], state[i], reason[i], polls);

    InterlockedExchange(&g_setevent_seq, tr_main.seq);
    BOOL ok = SetEvent(g_event);
    call(&tr_main, "", "kernel32!SetEvent", "SetEvent(event)", ok, ok ? 0 : GetLastError());

    char hb[64];
    DWORD r = WaitForSingleObject(ha, 10000);
    snprintf(hb, sizeof hb, "\"%lu:2\"", tr_a.tid);
    call(&tr_main, r == WAIT_OBJECT_0 ? hb : "", "kernel32!WaitForSingleObject",
         "WaitForSingleObject(thread_a,10000)", (long long)r, 0);
    r = WaitForSingleObject(hbt, 10000);
    snprintf(hb, sizeof hb, "\"%lu:2\"", tr_b.tid);
    call(&tr_main, r == WAIT_OBJECT_0 ? hb : "", "kernel32!WaitForSingleObject",
         "WaitForSingleObject(thread_b,10000)", (long long)r, 0);
    r = WaitForSingleObject(g_event, 0);
    call(&tr_main, "", "kernel32!WaitForSingleObject", "WaitForSingleObject(event,0)",
         (long long)r, 0);

    FILE *f = fopen(argv[2], "wb");
    if (!f) {
        fprintf(stderr, "cannot open %s\n", argv[2]);
        return 1;
    }
    int total = 2 + tr_main.n + tr_a.n + tr_b.n;
    fprintf(f, "{\"tag\":\"GUEST_TRACE_OPENED\",\"execution_id\":\"%s\",\"vocab_version\":\"0.5\","
               "\"profile_id\":\"%s\",\"trace_format_version\":\"0.1\"}\n",
            argv[1], prof);
    Tr *all[3] = {&tr_main, &tr_a, &tr_b};
    for (int k = 0; k < 3; k++)
        for (int i = 0; i < all[k]->n; i++) fprintf(f, "%s\n", all[k]->lines[i]);
    fprintf(f, "{\"tag\":\"GUEST_TRACE_CLOSED\",\"line_count\":%d,\"thread_count\":3,"
               "\"dropped_count\":0}\n",
            total);
    fclose(f);

    FILE *rf = fopen(argv[3], "wb");
    if (!rf) {
        fprintf(stderr, "cannot open %s\n", argv[3]);
        return 1;
    }
    fprintf(rf, "{\"main\": %lu, \"a\": %lu, \"b\": %lu}\n", tr_main.tid, tr_a.tid, tr_b.tid);
    fclose(rf);
    printf("%s: %d lines, profile %s\n", argv[1], total, prof);
    return 0;
}
