#include <windows.h>
/* Sprint 353 (reviewer-4's twenty-first review, Medium 1): three CPU-bound threads on two vCPUs, each counting in the
 * same loop. The main thread makes the first and spins until it counts, so the first runs on vCPU 1 while this thread
 * holds vCPU 0; it makes the second and the third while the first runs there, then waits with no timeout. The third
 * names the first's count AFTER_WAIT ahead and waits on an event, which the first sets from its vCPU on reaching that
 * count; from then all three are ready, and the third, at WINDOW, readies the main thread, which stops them. Each
 * thread's count, and every count at the first's set (`at_go`) and at the third's WINDOW (`at_ran`), sit in a section
 * of their own, `.counts`, which the test reads to judge each thread's share of that window. The output is "done"
 * when every count is above 0. */
#define AFTER_WAIT 4000000
#define WINDOW 40000000
struct counts {
    volatile LONG64 counts[3], at_go[3], at_ran[3];
};
__attribute__((section(".counts"))) struct counts c = {{0, 0, 0}, {0, 0, 0}, {0, 0, 0}};
static volatile LONG64 mark[3] = {0, 0, WINDOW};        /* the count at which a thread acts; 0: never */
static volatile LONG stop;
static HANDLE go, ran;
static void line(const char *s, DWORD n) { DWORD w; WriteFile(GetStdHandle(STD_OUTPUT_HANDLE), s, n, &w, NULL); }
static void act(int i) {
    volatile LONG64 *at = i == 0 ? c.at_go : c.at_ran;
    for (int j = 0; j < 3; j++) at[j] = c.counts[j];
    SetEvent(i == 0 ? go : ran);
}
static DWORD WINAPI counter(LPVOID p) {
    int i = (int)(INT_PTR)p;
    if (i == 2) {
        mark[0] = c.counts[0] + AFTER_WAIT;
        WaitForSingleObject(go, INFINITE);
    }
    while (!stop) {
        if (++c.counts[i] == mark[i]) act(i);
    }
    return 0;
}
void start(void) {
    HANDLE t[3];
    go = CreateEventA(NULL, FALSE, FALSE, NULL);
    ran = CreateEventA(NULL, FALSE, FALSE, NULL);
    t[0] = CreateThread(NULL, 0, counter, (LPVOID)0, 0, NULL);
    while (!c.counts[0]) {}                               /* the first counts on another vCPU, or after a slice */
    t[1] = CreateThread(NULL, 0, counter, (LPVOID)1, 0, NULL);
    t[2] = CreateThread(NULL, 0, counter, (LPVOID)2, 0, NULL);
    WaitForSingleObject(ran, INFINITE);
    stop = 1;
    WaitForMultipleObjects(3, t, TRUE, INFINITE);
    if (c.counts[0] > 0 && c.counts[1] > 0 && c.counts[2] > 0) line("done\n", 5);
    else line("starved\n", 8);
    ExitProcess(0);
}
