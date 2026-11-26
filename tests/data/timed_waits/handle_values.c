#include <windows.h>
/* Sprint 355 (card 4 of docs/design-handles-and-sync-objects-2026-11-26.md, its judge "Handles from two vCPUs"): two
 * threads on two vCPUs create, duplicate, wait on and close handles while CreateEventA and DuplicateHandle take the
 * handle table's lock and no runtime word, and CloseHandle and the waits take the word.
 *   - The design's rounds: each thread, 10,000 times, creates an unnamed event, duplicates one of the main thread's
 *     five objects by turns (`shared`, manual-reset and set; `quiet`, manual-reset and never set; `named`, the host's
 *     named event, set; `units`, a semaphore; `turn`, an auto-reset event), waits with a zero timeout on the duplicate
 *     of the first three (it must answer as its event is) and on the new event (WAIT_TIMEOUT), and closes both.
 *   - Meanwhile a third thread waits on and gives `units` and `turn`, blocking, waits on `named`, and makes pipes and
 *     sets and reads their flags (`third` below), until both workers end.
 *   - The bursts: each thread, 1,000 times, duplicates `shared` and a handle of the main thread (`main_thread`) 16
 *     times each, waits with a zero timeout for any of its 16 of each (16 lookups' references taken, then dropped,
 *     under the word) and closes all 32, the other thread's duplicates of the same objects, off the word, meeting them.
 *   - The drops: each thread duplicates `main_thread` and `ended`, a thread that has ended, 16 times each; then thread
 *     1 calls GetThreadId on its duplicates of both, again and again (a lookup's reference taken and dropped under the
 *     handle table's lock, no word), setting the auto-reset event `go` every 16th time once thread 2 reaches its
 *     granted waits (`granting`), so the word its zero-timeout waits hold does not stall it, while thread 2 waits 4,000
 *     times with a zero timeout for any of its 16 of `main_thread` (16 lookups' references dropped each time), then,
 *     2,000 times, waits for any of its 16 of `main_thread` and `go` (blocked, then granted `go`: its 17 entries'
 *     references dropped at the retry, waiters_drop), for all of its 16 of `ended` and `go` (blocked, then granted:
 *     all_drop), and once more with a zero timeout, all under the word, on the two objects thread 1's lookups change;
 *     then each closes its 32.
 * A spin of a pseudo-random length before each duplicate and each wait keeps the two threads' calls out of step. Each
 * value a create or a duplicate answers is claimed in `owner`, by its value / 4, until just before its close, so a value
 * two threads hold at once is counted in `collided`. Every close's answer is counted, TRUE in `closed`, FALSE in
 * `refused`, and every wait or duplicate that answers unlike its object in `wrong`. Each thread counts the rounds in
 * which it saw the other's round count move (`overlap`). While they run, the main thread waits 1 ms for any of `quiet`
 * and a duplicate of it, then 1 ms for all of `quiet` and `shared`, again and again (`waits`). After both end, it
 * closes its events and the thread handles, then makes 256 events, which every slot and every object entry must hold:
 * the first must answer the value the first create answered (`first`, `again`). The counts sit in a section of their
 * own, `.counts`, which the test reads. The output is "done" when no value collided, every close answered TRUE, every
 * wait answered as its object says, and the first value came back; else "failed". */
#define ROUNDS 10000
#define BURSTS 1000
#define BURST 16
#define DROPS 2000
#define FINAL 256
#define OWNERS 16384
#define CLAIMS (4 * ROUNDS + 2 * 2 * BURST * BURSTS + 2 * 2 * BURST)
struct judged {
    volatile LONG64 claimed, collided, closed, refused, wrong, first, again, made, overlap[2], waits, third;
};
__attribute__((section(".counts"))) struct judged c = {0, 0, 0, 0, 0, 0, 0, 0, {0, 0}, 0, 0};
static volatile LONG owner[OWNERS];                   /* a value / 4 -> the claiming thread's number, 0: unclaimed */
static volatile LONG arrived, finished, ready_to_drop, granting, drops_done;
static volatile LONG64 round_of[2];
static HANDLE shared, quiet, main_thread, ended, go, units, turn, named;
static DWORD main_id, ended_id;
static void line(const char *s, DWORD n) { DWORD w; WriteFile(GetStdHandle(STD_OUTPUT_HANDLE), s, n, &w, NULL); }
static void claim(HANDLE h, LONG me) {
    ULONG_PTR v = (ULONG_PTR)h >> 2;
    if (!h || v >= OWNERS || InterlockedCompareExchange(&owner[v], me, 0) != 0) InterlockedIncrement64(&c.collided);
    else InterlockedIncrement64(&c.claimed);
}
static void unclaim(HANDLE h, LONG me) {
    ULONG_PTR v = (ULONG_PTR)h >> 2;
    if (v < OWNERS) InterlockedCompareExchange(&owner[v], 0, me);
}
static void close_counted(HANDLE h) {
    if (CloseHandle(h)) InterlockedIncrement64(&c.closed);
    else InterlockedIncrement64(&c.refused);
}
static void expect(DWORD got, DWORD want) {
    if (got != want) InterlockedIncrement64(&c.wrong);
}
/* A spin of 0 to 63 turns, from the thread's own linear congruential sequence (Knuth's MMIX constants), so the two
 * threads' calls do not fall into step: the waits and closes take the runtime word, and in step the other thread's
 * duplicate met only the start of a wait. */
static void jitter(ULONG64 *seed) {
    *seed = *seed * 6364136223846793005ull + 1442695040888963407ull;
    for (volatile int k = (int)(*seed >> 58); k; k--) {}
}
static HANDLE duplicate(HANDLE source, LONG me) {
    HANDLE d = NULL;
    if (!DuplicateHandle(GetCurrentProcess(), source, GetCurrentProcess(), &d, 0, FALSE, DUPLICATE_SAME_ACCESS))
        InterlockedIncrement64(&c.wrong);
    claim(d, me);
    return d;
}
static void counted_round(LONG me, int other, LONG64 r, LONG64 *seen) {
    round_of[me - 1] = r;
    if (round_of[other] != *seen) {
        *seen = round_of[other];
        c.overlap[me - 1]++;
    }
}
/* The design's rounds duplicate these by turns; a wait on the duplicate is checked for the first three, whose state no
 * one changes, and none is made on the semaphore and the auto-reset event, which the third thread takes and gives. */
static HANDLE *const rotation[5] = {&shared, &quiet, &named, &units, &turn};
static void close_own(HANDLE h) {
    if (!CloseHandle(h)) InterlockedIncrement64(&c.refused);
}
/* The third thread, until both workers end: a 1 ms wait for all of `units` and `turn`, neither signaled (blocked, its
 * entry's references dropped at the timeout, all_drop); a 1 ms wait on `units` (blocked, waiter_drop); a release of
 * `units` and a set of `turn`, then a zero-timeout wait for all of them, which takes both; a zero-timeout wait on
 * `named`, the host's (signaled reads its handle count); and a pipe made, its read end's inherit flag set and read
 * back, cleared and read back (Get/SetHandleInformation, w2m_object_flags), both ends closed; all under the word, on
 * objects the workers duplicate and close, and slots the workers take and free. */
static DWORD WINAPI third(LPVOID p) {
    (void)p;
    HANDLE both[2] = {units, turn};
    while (finished < 2) {
        expect(WaitForMultipleObjects(2, both, TRUE, 1), WAIT_TIMEOUT);
        expect(WaitForSingleObject(units, 1), WAIT_TIMEOUT);
        if (!ReleaseSemaphore(units, 1, NULL)) InterlockedIncrement64(&c.wrong);
        if (!SetEvent(turn)) InterlockedIncrement64(&c.wrong);
        expect(WaitForMultipleObjects(2, both, TRUE, 0), WAIT_OBJECT_0);
        expect(WaitForSingleObject(named, 0), WAIT_OBJECT_0);
        HANDLE r = NULL, w = NULL;
        DWORD f = 9;
        if (!CreatePipe(&r, &w, NULL, 0)) InterlockedIncrement64(&c.wrong);
        SetHandleInformation(r, HANDLE_FLAG_INHERIT, HANDLE_FLAG_INHERIT);
        if (!GetHandleInformation(r, &f) || f != HANDLE_FLAG_INHERIT) InterlockedIncrement64(&c.wrong);
        SetHandleInformation(r, HANDLE_FLAG_INHERIT, 0);
        if (!GetHandleInformation(r, &f) || f != 0) InterlockedIncrement64(&c.wrong);
        close_own(r);
        close_own(w);
        c.third++;
    }
    return 0;
}
static DWORD WINAPI at_once(LPVOID p) {
    (void)p;
    return 0;
}
static DWORD WINAPI worker(LPVOID p) {
    LONG me = (LONG)(INT_PTR)p;
    int other = 2 - me;
    ULONG64 seed = (ULONG64)me;
    InterlockedIncrement(&arrived);
    while (arrived < 2) {}
    LONG64 seen = round_of[other];
    for (int r = 0; r < ROUNDS; r++) {
        HANDLE e = CreateEventA(NULL, TRUE, FALSE, NULL);
        claim(e, me);
        jitter(&seed);
        HANDLE d = duplicate(*rotation[r % 5], me);
        jitter(&seed);
        if (r % 5 < 3) expect(WaitForSingleObject(d, 0), r % 5 == 1 ? WAIT_TIMEOUT : WAIT_OBJECT_0);
        expect(WaitForSingleObject(e, 0), WAIT_TIMEOUT);
        unclaim(d, me);
        close_counted(d);
        unclaim(e, me);
        close_counted(e);
        counted_round(me, other, r + 1, &seen);
    }
    HANDLE set[BURST], running[BURST];
    for (int b = 0; b < BURSTS; b++) {
        for (int i = 0; i < BURST; i++) {
            jitter(&seed);
            set[i] = duplicate(shared, me);
            running[i] = duplicate(main_thread, me);
        }
        jitter(&seed);
        expect(WaitForMultipleObjects(BURST, set, FALSE, 0), WAIT_OBJECT_0);
        expect(WaitForMultipleObjects(BURST, running, FALSE, 0), WAIT_TIMEOUT);
        for (int i = 0; i < BURST; i++) {
            unclaim(set[i], me);
            close_counted(set[i]);
            unclaim(running[i], me);
            close_counted(running[i]);
        }
        counted_round(me, other, ROUNDS + b + 1, &seen);
    }
    HANDLE any[BURST + 1], all[BURST + 1];
    for (int i = 0; i < BURST; i++) {
        any[i] = duplicate(main_thread, me);
        all[i] = duplicate(ended, me);
    }
    any[BURST] = all[BURST] = go;
    InterlockedIncrement(&ready_to_drop);
    while (ready_to_drop < 2) {}
    if (me == 1) {                                     /* lookups, no word, until the other's drops are done */
        for (LONG64 k = 0; !drops_done; k++) {
            if (GetThreadId(any[k % BURST]) != main_id) InterlockedIncrement64(&c.wrong);
            if (GetThreadId(all[k % BURST]) != ended_id) InterlockedIncrement64(&c.wrong);
            if (granting && k % 16 == 15) SetEvent(go);
        }
    } else {                                           /* drops, under the word, against those lookups */
        for (int z = 0; z < 2 * DROPS; z++) expect(WaitForMultipleObjects(BURST, any, FALSE, 0), WAIT_TIMEOUT);
        granting = 1;
        for (int d = 0; d < DROPS; d++) {
            expect(WaitForMultipleObjects(BURST + 1, any, FALSE, INFINITE), WAIT_OBJECT_0 + BURST);
            expect(WaitForMultipleObjects(BURST + 1, all, TRUE, INFINITE), WAIT_OBJECT_0);
            expect(WaitForMultipleObjects(BURST, any, FALSE, 0), WAIT_TIMEOUT);
        }
        drops_done = 1;
    }
    for (int i = 0; i < BURST; i++) {
        unclaim(any[i], me);
        close_counted(any[i]);
        unclaim(all[i], me);
        close_counted(all[i]);
    }
    InterlockedIncrement(&finished);
    return 0;
}
void start(void) {
    HANDLE first = CreateEventA(NULL, TRUE, TRUE, NULL);
    shared = first;
    quiet = CreateEventA(NULL, TRUE, FALSE, NULL);
    HANDLE quiet_too = NULL;
    DuplicateHandle(GetCurrentProcess(), quiet, GetCurrentProcess(), &quiet_too, 0, FALSE, DUPLICATE_SAME_ACCESS);
    DuplicateHandle(GetCurrentProcess(), GetCurrentThread(), GetCurrentProcess(), &main_thread, 0, FALSE,
                    DUPLICATE_SAME_ACCESS);
    main_id = GetCurrentThreadId();
    go = CreateEventA(NULL, FALSE, FALSE, NULL);
    ended = CreateThread(NULL, 0, at_once, NULL, 0, NULL);
    WaitForSingleObject(ended, INFINITE);
    ended_id = GetThreadId(ended);
    units = CreateSemaphoreW(NULL, 0, 1000, NULL);
    turn = CreateEventA(NULL, FALSE, FALSE, NULL);
    named = CreateEventW(NULL, TRUE, TRUE, L"w2m_handle_values");
    HANDLE t[3];
    t[0] = CreateThread(NULL, 0, worker, (LPVOID)1, 0, NULL);
    t[1] = CreateThread(NULL, 0, worker, (LPVOID)2, 0, NULL);
    t[2] = CreateThread(NULL, 0, third, NULL, 0, NULL);
    HANDLE any[2] = {quiet, quiet_too}, all[2] = {quiet, shared};
    while (finished < 2) {
        expect(WaitForMultipleObjects(2, any, FALSE, 1), WAIT_TIMEOUT);
        expect(WaitForMultipleObjects(2, all, TRUE, 1), WAIT_TIMEOUT);
        c.waits++;
    }
    WaitForMultipleObjects(3, t, TRUE, INFINITE);
    close_counted(t[0]);
    close_counted(t[1]);
    close_counted(t[2]);
    close_counted(units);
    close_counted(turn);
    close_counted(named);
    close_counted(main_thread);
    close_counted(ended);
    close_counted(go);
    close_counted(quiet_too);
    close_counted(quiet);
    close_counted(shared);
    static HANDLE made[FINAL];
    for (int i = 0; i < FINAL; i++) {
        made[i] = CreateEventA(NULL, FALSE, FALSE, NULL);
        if (made[i]) c.made++;
    }
    c.first = (LONG64)(ULONG_PTR)first;
    c.again = (LONG64)(ULONG_PTR)made[0];
    for (int i = 0; i < FINAL; i++) close_counted(made[i]);
    if (c.collided == 0 && c.refused == 0 && c.wrong == 0 && c.claimed == CLAIMS && c.closed == CLAIMS + 12 + FINAL &&
        c.made == FINAL && c.again == c.first)
        line("done\n", 5);
    else
        line("failed\n", 7);
    ExitProcess(0);
}
