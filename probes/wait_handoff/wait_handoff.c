// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* wait_handoff.c — who gets a released object when a thread is already waiting for it (Sprint 065).
 *
 * The game's 16 Background Job.Worker threads each wait on one semaphore (CreateSemaphoreExW, max 0x7fffffff;
 * game run 38216081440 seq 1317, waits 1402-1508) and all 16 waits return on Windows; in w2m the running worker
 * takes every unit before a woken waiter runs, and 15 never return (frontier 643a6283510c-1791830368398119000).
 * The rows ask Windows: with a thread blocked on a semaphore, an auto-reset event or a mutex, does the releasing
 * thread's own zero-timeout wait right after the release find the object signaled? And with two waiters, which
 * one the release wakes. Waiters block in a known order: each sets `ready` and the main thread sleeps 200 ms.
 * No C runtime. Rows: each wait's return, the released count, which waiters have finished.
 */
#include <windows.h>

static char out[8192];
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

typedef struct {
    HANDLE object, ready, finish;
    int mutex;
    volatile DWORD ret;
    volatile LONG done;
} waiter_t;

/* Waits on `object`; holds what it got (a mutex) until `finish` is set. */
static DWORD WINAPI waiter(LPVOID p) {
    waiter_t *w = (waiter_t *)p;
    SetEvent(w->ready);
    w->ret = WaitForSingleObject(w->object, INFINITE);
    InterlockedExchange(&w->done, 1);
    WaitForSingleObject(w->finish, INFINITE);
    if (w->mutex && w->ret == WAIT_OBJECT_0) ReleaseMutex(w->object);
    return 0;
}

static HANDLE spawn_as(waiter_t *w, HANDLE object, HANDLE finish, int mutex) {
    w->object = object;
    w->mutex = mutex;
    w->ready = CreateEventW(NULL, FALSE, FALSE, NULL);
    w->finish = finish;
    w->ret = 0xFFFF;
    w->done = 0;
    HANDLE h = CreateThread(NULL, 0, waiter, w, 0, NULL);
    WaitForSingleObject(w->ready, INFINITE);
    Sleep(200);                                     /* the waiter is in its wait */
    return h;
}

static HANDLE spawn(waiter_t *w, HANDLE object, HANDLE finish) { return spawn_as(w, object, finish, 0); }

static void finish(HANDLE ev, HANDLE *threads, int count) {
    SetEvent(ev);
    WaitForMultipleObjects((DWORD)count, threads, TRUE, INFINITE);
}

void start(void) {
    /* a semaphore at 0, one waiter: release one, then wait on it at once */
    HANDLE fin = CreateEventW(NULL, TRUE, FALSE, NULL);
    HANDLE sem = CreateSemaphoreW(NULL, 0, 0x7FFFFFFF, NULL);
    waiter_t a, b, c;
    HANDLE t[3];
    t[0] = spawn(&a, sem, fin);
    LONG prev = -1;
    BOOL r = ReleaseSemaphore(sem, 1, &prev);
    DWORD mine = WaitForSingleObject(sem, 0);
    Sleep(200);
    row("semaphore.one_waiter"); field("release", r); field("previous", (unsigned long long)prev);
    field("releaser_wait", mine); field("waiter_done", a.done); field("waiter_ret", a.ret); end();
    finish(fin, t, 1);

    /* three waiters, release two: the releaser's wait, how many finish */
    ResetEvent(fin);
    t[0] = spawn(&a, sem, fin);
    t[1] = spawn(&b, sem, fin);
    t[2] = spawn(&c, sem, fin);
    ReleaseSemaphore(sem, 2, &prev);
    mine = WaitForSingleObject(sem, 0);
    Sleep(200);
    row("semaphore.three_waiters_two_released"); field("previous", (unsigned long long)prev);
    field("releaser_wait", mine); field("done", (unsigned long long)(a.done + b.done + c.done));
    field("first_done", a.done); field("second_done", b.done); field("third_done", c.done); end();
    ReleaseSemaphore(sem, 1, NULL);
    finish(fin, t, 3);

    /* two waiters, release one: which wakes */
    ResetEvent(fin);
    t[0] = spawn(&a, sem, fin);
    t[1] = spawn(&b, sem, fin);
    ReleaseSemaphore(sem, 1, NULL);
    Sleep(200);
    row("semaphore.order"); field("first_done", a.done); field("second_done", b.done); end();
    ReleaseSemaphore(sem, 1, NULL);
    finish(fin, t, 2);

    /* an auto-reset event, one waiter: set, then wait on it at once */
    ResetEvent(fin);
    HANDLE ev = CreateEventW(NULL, FALSE, FALSE, NULL);
    t[0] = spawn(&a, ev, fin);
    BOOL s = SetEvent(ev);
    mine = WaitForSingleObject(ev, 0);
    Sleep(200);
    row("event_auto.one_waiter"); field("set", s); field("releaser_wait", mine); field("waiter_done", a.done);
    field("waiter_ret", a.ret); end();
    finish(fin, t, 1);

    /* a mutex the main thread owns, one waiter: release, then wait on it at once */
    ResetEvent(fin);
    HANDLE mx = CreateMutexA(NULL, FALSE, NULL);
    DWORD own = WaitForSingleObject(mx, 0);
    t[0] = spawn_as(&a, mx, fin, 1);
    BOOL rm = ReleaseMutex(mx);
    mine = WaitForSingleObject(mx, 0);
    Sleep(200);
    row("mutex.one_waiter"); field("owned", own); field("release", rm); field("releaser_wait", mine);
    field("waiter_done", a.done); field("waiter_ret", a.ret); end();
    if (mine == WAIT_OBJECT_0) ReleaseMutex(mx);
    finish(fin, t, 1);
    ExitProcess(0);
}
