// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* wait_closed_handle.c — a semaphore's last handle closed while another thread waits on it, then a new semaphore made,
 * released once and waited on (Sprint 350's amendment; found by builder-350's reading of sync_objects.c).
 *
 * No C runtime. A worker sets an event and waits 1000 ms on a semaphore with no unit. The main thread waits for the
 * event, sleeps 200 ms so the worker is in its wait, closes the semaphore's only handle, makes a new semaphore (no
 * unit, maximum 1), releases it once and waits on it with a zero timeout. Microsoft Learn, WaitForSingleObject: "If this
 * handle is closed while the wait is still pending, the function's behavior is undefined", so the worker's row is
 * recorded and not compared; the main thread's calls are on a semaphore no other thread names, and are defined. w2m
 * at 23b860e79 frees the closed semaphore while the worker's waiter entry still names its index (sync_objects.c
 * 114-127, 394), gives that index to the new semaphore (69-72), and lets a release grant the stale entry (214-230), so
 * the main thread's wait can miss the unit it released. Rows: "main", the close, the make,
 * the release and its previous count, the zero-timeout wait and the wait for the worker's end; "worker", its wait's
 * result and last error.
 */
#include <windows.h>

static char out[256];
static unsigned n;

static void put(const char *s) { while (*s && n < sizeof out - 32) out[n++] = *s++; }
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

static HANDLE semaphore, ready;
static DWORD worker_wait, worker_error;

static DWORD WINAPI worker(LPVOID param) {
    (void)param;
    SetEvent(ready);
    SetLastError(0);
    worker_wait = WaitForSingleObject(semaphore, 1000);
    worker_error = GetLastError();
    return 0;
}

void start(void) {
    DWORD tid;
    semaphore = CreateSemaphoreW(NULL, 0, 1, NULL);
    ready = CreateEventW(NULL, FALSE, FALSE, NULL);
    HANDLE thread = CreateThread(NULL, 0, worker, NULL, 0, &tid);
    WaitForSingleObject(ready, INFINITE);
    Sleep(200);
    BOOL closed = CloseHandle(semaphore);
    HANDLE again = CreateSemaphoreW(NULL, 0, 1, NULL);
    LONG previous = -1;
    BOOL released = ReleaseSemaphore(again, 1, &previous);
    DWORD wait = WaitForSingleObject(again, 0);
    DWORD ended = WaitForSingleObject(thread, INFINITE);
    row("main");
    field("closed", closed != 0);
    field("made", again != NULL);
    field("released", released != 0);
    field("previous", (unsigned long long)(ULONG)previous);
    field("wait", wait);
    field("ended", ended);
    end();
    row("worker");
    field("wait", worker_wait);
    field("error", worker_error);
    end();
    ExitProcess(0);
}
