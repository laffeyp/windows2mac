#include <windows.h>
/* Sprint 356 (card 5 of docs/design-handles-and-sync-objects-2026-11-26.md, its judge "A semaphore's units across
 * vCPUs"): four threads release and wait on one semaphore, maximum 1,000, each 100,000 rounds, while ReleaseSemaphore
 * and the waits take the sync objects' lock and hold the runtime word only to park and to ready.
 *   - A round releases one unit, then waits for one, each wait with no timeout but one: by WaitForSingleObject; every
 *     8th round with a zero timeout (it answers WAIT_TIMEOUT when no unit is left, and its unit stays); every 16th
 *     from the 4th by WaitForMultipleObjects for any of the one handle, every 16th from the 12th for any of it and of
 *     `running`, the main thread's handle, which signals only at the run's end; every 32nd from the 16th for all of
 *     the one handle, and every 32nd for all of it and of `ended`, a thread that has ended.
 *   - Every release precedes its thread's wait, so every wait with no timeout has a unit to come: the run ends only
 *     when each such wait returned WAIT_OBJECT_0; a unit lost, or a grant lost, leaves a thread waiting for ever.
 * Strengthened for plant 4, each holding of the sync objects' lock removed in turn, which the semaphore's rounds alone
 * left at 20 of 20 "done" in ReleaseMutex and SetEvent, and in the waits that name one of the host's objects:
 *   - Two locks the four threads share, each entered by turns and held alone. A mutex, every 8th round waited on with
 *     no timeout and every other even round with a zero timeout; and an auto-reset event made set, every 8th round
 *     from the 5th waited on with no timeout and every other odd round with a zero timeout, then set again by its
 *     holder. A thread inside either lock counts its entry by an atomic add (`*_taken`) and by a plain add
 *     (`*_counted`), which a second holder at once would lose, and finds no other thread inside (`shared`).
 *   - Every 64th round the thread resets a manual-reset event, sets it and waits on it with no timeout (ResetEvent,
 *     SetEvent's signal of a manual-reset event's waiters), each reset followed by its own thread's set, so every
 *     such wait returns (`gated`).
 *   - The waits naming `running` or `ended` above, which hold the runtime word from their check to their park; and
 *     1 ms waits that time out (`watched`), each blocking under the word, its entry made and taken out among the
 *     four's: the main thread's on each worker's handle in turn until it has ended, and two more threads' on
 *     `running` until the four have ended. No wait here is on a named event, whose set would ready every blocked
 *     thread of the process (`w2m_signal(0)`) and so make good a grant a plant had lost.
 * A spin of a pseudo-random length before each release and each wait, and inside each lock, keeps the four threads'
 * calls out of step. The releases answered TRUE (`released`) and FALSE (`refused`), the semaphore's waits that
 * returned WAIT_OBJECT_0 (`satisfied`), its zero-timeout waits that answered WAIT_TIMEOUT (`timed_out`), and every
 * other answer of any call (`wrong`) are counted. After the four threads end the main thread releases one unit more,
 * whose previous count is the semaphore's final count (`final`), and which must equal the releases less the satisfied
 * waits. Each thread counts the rounds in which it saw the others' rounds move (`overlap`). The counts sit in a
 * section of their own, `.counts`, which the test reads. The output is "done" when every round's release answered
 * TRUE, no call answered otherwise than its timeout allows, the final count is the releases less the satisfied waits,
 * each lock held every entry alone, and every gate wait returned; else "failed". */
#define ROUNDS 100000
#define THREADS 4
#define WATCHERS 2
#define MAXIMUM 1000
struct judged {
    volatile LONG64 released, refused, satisfied, timed_out, wrong, final, overlap[THREADS];
    volatile LONG64 mutex_taken, mutex_counted, turn_taken, turn_counted, shared, gated, watched;
};
__attribute__((section(".counts"))) struct judged c = {0, 0, 0, 0, 0, 0, {0, 0, 0, 0}, 0, 0, 0, 0, 0, 0, 0};
static volatile LONG64 round_of[THREADS];
static volatile LONG in_mutex, in_turn, finished;
static HANDLE units, mutex, turn, gate, running, ended;
static void line(const char *s, DWORD n) { DWORD w; WriteFile(GetStdHandle(STD_OUTPUT_HANDLE), s, n, &w, NULL); }
/* A spin of 0 to 63 turns, from the thread's own linear congruential sequence (Knuth's MMIX constants). */
static void jitter(ULONG64 *seed) {
    *seed = *seed * 6364136223846793005ull + 1442695040888963407ull;
    for (volatile int k = (int)(*seed >> 58); k; k--) {}
}
static void counted(DWORD got, DWORD ms) {
    if (got == WAIT_OBJECT_0) InterlockedIncrement64(&c.satisfied);
    else if (got == WAIT_TIMEOUT && ms == 0) InterlockedIncrement64(&c.timed_out);
    else InterlockedIncrement64(&c.wrong);
}
/* A lock's holder inside: its entry counted twice, another holder looked for. */
static void inside(volatile LONG *in, volatile LONG64 *taken, volatile LONG64 *count, ULONG64 *seed) {
    InterlockedIncrement64(taken);
    if (InterlockedExchange(in, 1) != 0) InterlockedIncrement64(&c.shared);
    *count = *count + 1;
    jitter(seed);
    InterlockedExchange(in, 0);
}
/* A lock waited on for ms: 1 when held, its answer counted wrong unless WAIT_OBJECT_0, or WAIT_TIMEOUT at 0 ms. */
static int held(HANDLE lock, DWORD ms) {
    DWORD got = WaitForSingleObject(lock, ms);
    if (got == WAIT_OBJECT_0) return 1;
    if (got != WAIT_TIMEOUT || ms != 0) InterlockedIncrement64(&c.wrong);
    return 0;
}
/* A 1 ms wait on h: 1 when it timed out, counted; 0 when h signaled. */
static int watch(HANDLE h) {
    DWORD got = WaitForSingleObject(h, 1);
    if (got == WAIT_TIMEOUT) {
        InterlockedIncrement64(&c.watched);
        return 1;
    }
    if (got != WAIT_OBJECT_0) InterlockedIncrement64(&c.wrong);
    return 0;
}
static DWORD WINAPI nothing(LPVOID p) {
    (void)p;
    return 0;
}
static DWORD WINAPI watcher(LPVOID p) {
    (void)p;
    while (!finished)
        if (!watch(running)) InterlockedIncrement64(&c.wrong);       /* the main thread runs to the end */
    return 0;
}
static DWORD WINAPI worker(LPVOID p) {
    int me = (int)(INT_PTR)p;
    ULONG64 seed = 0x9E3779B97F4A7C15ull * (ULONG64)(me + 1);
    LONG64 seen = 0;
    HANDLE with_running[2] = {units, running}, with_ended[2] = {units, ended};
    for (LONG64 r = 1; r <= ROUNDS; r++) {
        jitter(&seed);
        LONG previous = -1;
        if (ReleaseSemaphore(units, 1, &previous)) InterlockedIncrement64(&c.released);
        else InterlockedIncrement64(&c.refused);
        jitter(&seed);
        if (r % 32 == 0) counted(WaitForMultipleObjects(2, with_ended, TRUE, INFINITE), INFINITE);
        else if (r % 32 == 16) counted(WaitForMultipleObjects(1, &units, TRUE, INFINITE), INFINITE);
        else if (r % 16 == 12) counted(WaitForMultipleObjects(2, with_running, FALSE, INFINITE), INFINITE);
        else if (r % 16 == 4) counted(WaitForMultipleObjects(1, &units, FALSE, INFINITE), INFINITE);
        else if (r % 8 == 0) counted(WaitForSingleObject(units, 0), 0);
        else counted(WaitForSingleObject(units, INFINITE), INFINITE);
        if (r % 2 == 0 && held(mutex, r % 8 == 0 ? INFINITE : 0)) {
            inside(&in_mutex, &c.mutex_taken, &c.mutex_counted, &seed);
            if (!ReleaseMutex(mutex)) InterlockedIncrement64(&c.wrong);
        }
        if (r % 2 == 1 && held(turn, r % 8 == 5 ? INFINITE : 0)) {
            inside(&in_turn, &c.turn_taken, &c.turn_counted, &seed);
            if (!SetEvent(turn)) InterlockedIncrement64(&c.wrong);
        }
        if (r % 64 == 0) {
            if (!ResetEvent(gate) || !SetEvent(gate)) InterlockedIncrement64(&c.wrong);
            if (WaitForSingleObject(gate, INFINITE) == WAIT_OBJECT_0) InterlockedIncrement64(&c.gated);
            else InterlockedIncrement64(&c.wrong);
        }
        round_of[me] = r;
        LONG64 others = 0;
        for (int j = 0; j < THREADS; j++)
            if (j != me) others += round_of[j];
        if (others != seen) {
            seen = others;
            c.overlap[me]++;
        }
    }
    return 0;
}
void start(void) {
    units = CreateSemaphoreW(NULL, 0, MAXIMUM, NULL);
    mutex = CreateMutexA(NULL, FALSE, NULL);
    turn = CreateEventA(NULL, FALSE, TRUE, NULL);
    gate = CreateEventA(NULL, TRUE, FALSE, NULL);
    if (!DuplicateHandle(GetCurrentProcess(), GetCurrentThread(), GetCurrentProcess(), &running, 0, FALSE,
                         DUPLICATE_SAME_ACCESS))
        c.wrong++;
    ended = CreateThread(NULL, 0, nothing, NULL, 0, NULL);
    if (WaitForSingleObject(ended, INFINITE) != WAIT_OBJECT_0) c.wrong++;
    HANDLE t[THREADS], w[WATCHERS];
    for (int i = 0; i < THREADS; i++) t[i] = CreateThread(NULL, 0, worker, (LPVOID)(INT_PTR)i, 0, NULL);
    for (int i = 0; i < WATCHERS; i++) w[i] = CreateThread(NULL, 0, watcher, NULL, 0, NULL);
    for (int i = 0; i < THREADS; i++)
        while (watch(t[i])) {}
    finished = 1;
    for (int i = 0; i < WATCHERS; i++)
        if (WaitForSingleObject(w[i], INFINITE) != WAIT_OBJECT_0) c.wrong++;
    LONG previous = -1;
    if (!ReleaseSemaphore(units, 1, &previous)) c.wrong++;
    c.final = previous;
    if (c.released == (LONG64)THREADS * ROUNDS && c.refused == 0 && c.wrong == 0 &&
        c.final == c.released - c.satisfied && c.mutex_counted == c.mutex_taken && c.turn_counted == c.turn_taken &&
        c.shared == 0 && c.mutex_taken >= (LONG64)THREADS * (ROUNDS / 8) &&
        c.turn_taken >= (LONG64)THREADS * (ROUNDS / 8) && c.gated == (LONG64)THREADS * (ROUNDS / 64))
        line("done\n", 5);
    else
        line("failed\n", 7);
    ExitProcess(0);
}
