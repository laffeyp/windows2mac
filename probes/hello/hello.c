/* hello.c — minimal w2m probe: proves the Windows runner loop (Sprint 006).
 *
 * Writes a guest trace (signals/0.1.json, category guest) to the path in argv[2]:
 * thread 1 starts and reads QPC; thread 2 starts (hb 1:1), reads QPC, exits; thread 1 joins it
 * and reads QPC again (hb 2:2). Lines are buffered and written after the join, so the file is
 * one writer's output; per-thread order is carried by thread_seq, not file position.
 *
 * Usage: hello.exe <execution_id> <trace_path>
 */
#include <windows.h>
#include <stdarg.h>
#include <stdio.h>
#include <stdlib.h>

#define MAXLINES 16
#define LINELEN 512

static char lines[MAXLINES][LINELEN];
static int nlines = 0;

static void emit(const char *fmt, ...) {
    va_list ap;
    va_start(ap, fmt);
    vsnprintf(lines[nlines++], LINELEN, fmt, ap);
    va_end(ap);
}

static unsigned long long qpc(void) {
    LARGE_INTEGER v;
    QueryPerformanceCounter(&v);
    return (unsigned long long)v.QuadPart;
}

static DWORD main_tid;
static DWORD worker_tid;
static char worker_lines[3][LINELEN];

static DWORD WINAPI worker(LPVOID arg) {
    (void)arg;
    worker_tid = GetCurrentThreadId();
    snprintf(worker_lines[0], LINELEN,
             "{\"tag\":\"GUEST_THREAD_STARTED\",\"thread_id\":%lu,\"thread_seq\":0,"
             "\"hb_refs\":[\"%lu:1\"],\"parent_thread_id\":%lu,\"start_address\":%llu,"
             "\"suspended\":false}",
             worker_tid, main_tid, main_tid, (unsigned long long)(ULONG_PTR)worker);
    snprintf(worker_lines[1], LINELEN,
             "{\"tag\":\"GUEST_CLOCK_READ\",\"thread_id\":%lu,\"thread_seq\":1,\"hb_refs\":[],"
             "\"clock\":\"qpc\",\"value\":%llu}",
             worker_tid, qpc());
    snprintf(worker_lines[2], LINELEN,
             "{\"tag\":\"GUEST_THREAD_EXITED\",\"thread_id\":%lu,\"thread_seq\":2,\"hb_refs\":[],"
             "\"exit_code\":0}",
             worker_tid);
    return 0;
}

typedef LONG (WINAPI *RtlGetVersionFn)(PRTL_OSVERSIONINFOW);

int main(int argc, char **argv) {
    if (argc != 3) {
        fprintf(stderr, "usage: hello.exe <execution_id> <trace_path>\n");
        return 2;
    }
    RTL_OSVERSIONINFOW ver = {sizeof(ver)};
    RtlGetVersionFn rtl = (RtlGetVersionFn)(void *)GetProcAddress(
        GetModuleHandleW(L"ntdll.dll"), "RtlGetVersion");
    if (rtl) rtl(&ver);

    main_tid = GetCurrentThreadId();
    emit("{\"tag\":\"GUEST_TRACE_OPENED\",\"execution_id\":\"%s\",\"vocab_version\":\"0.1\","
         "\"profile_id\":\"gha-windows-2025-x64-%lu.%lu.%lu\",\"trace_format_version\":\"0.1\"}",
         argv[1], ver.dwMajorVersion, ver.dwMinorVersion, ver.dwBuildNumber);
    emit("{\"tag\":\"GUEST_THREAD_STARTED\",\"thread_id\":%lu,\"thread_seq\":0,\"hb_refs\":[],"
         "\"parent_thread_id\":0,\"start_address\":%llu,\"suspended\":false}",
         main_tid, (unsigned long long)(ULONG_PTR)main);
    emit("{\"tag\":\"GUEST_CLOCK_READ\",\"thread_id\":%lu,\"thread_seq\":1,\"hb_refs\":[],"
         "\"clock\":\"qpc\",\"value\":%llu}",
         main_tid, qpc());

    HANDLE h = CreateThread(NULL, 0, worker, NULL, 0, NULL);
    if (!h) {
        fprintf(stderr, "CreateThread failed: %lu\n", GetLastError());
        return 1;
    }
    if (WaitForSingleObject(h, 10000) != WAIT_OBJECT_0) {
        fprintf(stderr, "worker did not finish within 10 s\n");
        return 1;
    }
    CloseHandle(h);
    for (int i = 0; i < 3; i++) emit("%s", worker_lines[i]);

    emit("{\"tag\":\"GUEST_CLOCK_READ\",\"thread_id\":%lu,\"thread_seq\":2,"
         "\"hb_refs\":[\"%lu:2\"],\"clock\":\"qpc\",\"value\":%llu}",
         main_tid, worker_tid, qpc());
    emit("{\"tag\":\"GUEST_TRACE_CLOSED\",\"line_count\":%d,\"thread_count\":2,"
         "\"dropped_count\":0}",
         nlines + 1);

    FILE *f = fopen(argv[2], "wb");
    if (!f) {
        fprintf(stderr, "cannot open %s\n", argv[2]);
        return 1;
    }
    for (int i = 0; i < nlines; i++) fprintf(f, "%s\n", lines[i]);
    fclose(f);
    printf("wrote %d lines to %s (windows %lu.%lu.%lu)\n", nlines, argv[2],
           ver.dwMajorVersion, ver.dwMinorVersion, ver.dwBuildNumber);
    return 0;
}
