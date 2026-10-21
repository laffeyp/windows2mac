#include <windows.h>
/* A worker waits on an event, so the vCPU it ran on has no thread; the main thread sets the event inside the VM and
 * spins until the worker has done its round of work, then a little longer so the worker is waiting again, 20 rounds.
 * The worker's rounds run on the vCPU that waits for a thread only if that vCPU learns of the ready thread. */
static volatile LONG rounds_done;
static volatile unsigned sink;
static HANDLE go;
static void line(const char *s, DWORD n) { DWORD w; WriteFile(GetStdHandle(STD_OUTPUT_HANDLE), s, n, &w, NULL); }
static DWORD WINAPI worker(LPVOID p) {
    (void)p;
    for (int r = 0; r < 20; r++) {
        WaitForSingleObject(go, INFINITE);
        unsigned sum = 0;
        for (unsigned i = 0; i < 2000000; i++) sum += i;
        sink = sum;
        rounds_done = r + 1;
    }
    return 0;
}
void start(void) {
    go = CreateEventA(NULL, FALSE, FALSE, NULL);
    HANDLE h = CreateThread(NULL, 0, worker, NULL, 0, NULL);
    for (int r = 0; r < 20; r++) {
        SetEvent(go);
        while (rounds_done <= r) {}
        for (volatile unsigned i = 0; i < 1000000; i++) {}   /* the worker reaches its wait and its vCPU waits */
    }
    WaitForSingleObject(h, INFINITE);
    line("done\n", 5);
    ExitProcess(0);
}
