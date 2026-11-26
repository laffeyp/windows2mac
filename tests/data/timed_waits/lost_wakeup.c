#include <windows.h>
/* Sprint 354 (card 3 of docs/design-handles-and-sync-objects-2026-11-26.md, "The lost wakeup"): two threads take turns
 * over two unnamed auto-reset events, 100,000 rounds, every wait INFINITE: the main thread sets ping and waits on
 * pong, the worker waits on ping and sets pong, each set granted to the other's wait (sync_objects.c grant, then
 * w2m_ready_thread). On two vCPUs each thread runs on its own. Once card 5 checks a wait under the sync objects' lock
 * and parks it under the runtime word, a set that falls between the two finds its waiter not yet BLOCKED; lost, it
 * leaves both threads waiting with no deadline, and the run stops as a deadlock. The output is "done" when every wait
 * returned WAIT_OBJECT_0, else "wait failed". */
#define ROUNDS 100000
static HANDLE ping, pong;
static volatile LONG failed;
static void line(const char *s, DWORD n) { DWORD w; WriteFile(GetStdHandle(STD_OUTPUT_HANDLE), s, n, &w, NULL); }
static DWORD WINAPI worker(LPVOID p) {
    (void)p;
    for (int r = 0; r < ROUNDS; r++) {
        if (WaitForSingleObject(ping, INFINITE) != WAIT_OBJECT_0) failed = 1;
        SetEvent(pong);
    }
    return 0;
}
void start(void) {
    ping = CreateEventA(NULL, FALSE, FALSE, NULL);
    pong = CreateEventA(NULL, FALSE, FALSE, NULL);
    HANDLE h = CreateThread(NULL, 0, worker, NULL, 0, NULL);
    for (int r = 0; r < ROUNDS; r++) {
        SetEvent(ping);
        if (WaitForSingleObject(pong, INFINITE) != WAIT_OBJECT_0) failed = 1;
    }
    if (WaitForSingleObject(h, INFINITE) != WAIT_OBJECT_0) failed = 1;
    if (failed) line("wait failed\n", 12);
    else line("done\n", 5);
    ExitProcess(0);
}
