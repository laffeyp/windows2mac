// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* slim_locks.c — SRW locks and condition variables on one thread (Sprint 046, family "slim locks").
 *
 * No C runtime. UnityPlayer.dll's C runtime gets InitializeConditionVariable through GetProcAddress in
 * its DllMain (scratch run of the launcher, 2026-10-10). One thread only: the rows are each call's return
 * and last error and the lock or variable's pointer-sized word after it (the structures are opaque on
 * Microsoft Learn; a program can still read them). Waits between threads are Sprint 047's.
 */
#include <windows.h>

#define SENTINEL 0xDEADu

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
static void end(void) { put("}\n"); }
static void finish(void) {
    DWORD written;
    WriteFile(GetStdHandle(STD_OUTPUT_HANDLE), out, n, &written, NULL);
    ExitProcess(0);
}

static SRWLOCK lock;
static CONDITION_VARIABLE cv;

static void word(const char *name, void *p) { row(name); field("word", *(unsigned long long *)p); end(); }

void start(void) {
    *(unsigned long long *)&lock = 0xA5A5A5A5A5A5A5A5ull;
    *(unsigned long long *)&cv = 0xA5A5A5A5A5A5A5A5ull;
    InitializeSRWLock(&lock);
    word("srw.initialized", &lock);
    InitializeConditionVariable(&cv);
    word("cv.initialized", &cv);

    AcquireSRWLockExclusive(&lock);
    word("srw.exclusive", &lock);
    SetLastError(SENTINEL);
    BOOLEAN again = TryAcquireSRWLockExclusive(&lock);
    DWORD err = GetLastError();
    row("srw.try_exclusive_while_held"); field("ret", again != 0); field("err", err); field("word", *(unsigned long long *)&lock); end();
    SetLastError(SENTINEL);
    BOOLEAN shared = TryAcquireSRWLockShared(&lock);
    err = GetLastError();
    row("srw.try_shared_while_exclusive"); field("ret", shared != 0); field("err", err); end();

    SetLastError(SENTINEL);
    BOOL slept = SleepConditionVariableSRW(&cv, &lock, 0, 0);
    err = GetLastError();
    row("cv.sleep_timeout_zero"); field("ret", slept != 0); field("err", err);
    field("lock_word", *(unsigned long long *)&lock); field("cv_word", *(unsigned long long *)&cv); end();
    SetLastError(SENTINEL);
    slept = SleepConditionVariableSRW(&cv, &lock, 1, 0);
    err = GetLastError();
    row("cv.sleep_timeout_one_ms"); field("ret", slept != 0); field("err", err);
    field("lock_word", *(unsigned long long *)&lock); field("cv_word", *(unsigned long long *)&cv); end();

    ReleaseSRWLockExclusive(&lock);
    word("srw.released", &lock);

    SetLastError(SENTINEL);
    BOOLEAN got = TryAcquireSRWLockExclusive(&lock);
    err = GetLastError();
    row("srw.try_exclusive_free"); field("ret", got != 0); field("err", err); field("word", *(unsigned long long *)&lock); end();
    ReleaseSRWLockExclusive(&lock);

    AcquireSRWLockShared(&lock);
    word("srw.shared_once", &lock);
    AcquireSRWLockShared(&lock);
    word("srw.shared_twice", &lock);
    SetLastError(SENTINEL);
    got = TryAcquireSRWLockExclusive(&lock);
    err = GetLastError();
    row("srw.try_exclusive_while_shared"); field("ret", got != 0); field("err", err); end();
    ReleaseSRWLockShared(&lock);
    word("srw.shared_released_once", &lock);
    ReleaseSRWLockShared(&lock);
    word("srw.shared_released_twice", &lock);

    SetLastError(SENTINEL);
    WakeConditionVariable(&cv);
    err = GetLastError();
    row("cv.wake_one_no_waiters"); field("err", err); field("word", *(unsigned long long *)&cv); end();
    SetLastError(SENTINEL);
    WakeAllConditionVariable(&cv);
    err = GetLastError();
    row("cv.wake_all_no_waiters"); field("err", err); field("word", *(unsigned long long *)&cv); end();
    finish();
}
