#include <windows.h>
/* Two workers wait on two events, so the vCPU they ran on has no thread; the main thread sets both events inside the
 * VM, one after the other, and spins until both have done their round of work, then a little longer so both are
 * waiting again, 20 rounds; each worker spins after its work until the other has done its round. One waiting vCPU
 * can take one readied worker; the other must get a turn on the main thread's vCPU, or every vCPU spins. */
static volatile LONG done_a, done_b;
static volatile unsigned sink;
static HANDLE go_a, go_b;
static void line(const char *s, DWORD n) { DWORD w; WriteFile(GetStdHandle(STD_OUTPUT_HANDLE), s, n, &w, NULL); }
static void work(HANDLE go, volatile LONG *done, volatile LONG *other) {
    for (int r = 0; r < 20; r++) {
        WaitForSingleObject(go, INFINITE);
        unsigned sum = 0;
        for (unsigned i = 0; i < 1000000; i++) sum += i;
        sink = sum;
        *done = r + 1;
        while (*other <= r) {}                             /* each worker waits for the other's round */
    }
}
static DWORD WINAPI worker_a(LPVOID p) { (void)p; work(go_a, &done_a, &done_b); return 0; }
static DWORD WINAPI worker_b(LPVOID p) { (void)p; work(go_b, &done_b, &done_a); return 0; }
void start(void) {
    go_a = CreateEventA(NULL, FALSE, FALSE, NULL);
    go_b = CreateEventA(NULL, FALSE, FALSE, NULL);
    HANDLE a = CreateThread(NULL, 0, worker_a, NULL, 0, NULL), b = CreateThread(NULL, 0, worker_b, NULL, 0, NULL);
    for (int r = 0; r < 20; r++) {
        SetEvent(go_a);
        SetEvent(go_b);
        while (done_a <= r || done_b <= r) {}
        for (volatile unsigned i = 0; i < 1000000; i++) {}   /* both workers reach their waits */
    }
    WaitForSingleObject(a, INFINITE);
    WaitForSingleObject(b, INFINITE);
    line("done\n", 5);
    ExitProcess(0);
}
