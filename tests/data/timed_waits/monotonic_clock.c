#include <windows.h>
/* Sprint 360: QueryPerformanceCounter on two vCPUs, read as one clock. The main thread makes a reader and both read
 * QueryPerformanceCounter ROUNDS times, each on its own vCPU once both have started. Before each reading a thread takes
 * the other's last reading and its own; a reading below the larger counts in `below`, the first such reading and the
 * one it fell below kept. `moved` counts the readings before which the other's last had changed, which shows the two
 * read side by side. The figures sit in a section of their own, `.clock`, which crates/w2m_vm/tests/run_loop.rs reads;
 * the output is "done" when no reading fell below one taken before, else "below". QueryPerformanceCounter "does not go
 * backward" (Microsoft Learn, "Acquiring high-resolution time stamps"), so on Windows every `below` is 0. */
#define ROUNDS 200000
struct clock {
    volatile LONG64 below[2], moved[2], first_low[2], first_seen[2], last[2];
};
__attribute__((section(".clock"))) struct clock c = {{0, 0}, {0, 0}, {0, 0}, {0, 0}, {0, 0}};
static volatile LONG started;
static void line(const char *s, DWORD n) { DWORD w; WriteFile(GetStdHandle(STD_OUTPUT_HANDLE), s, n, &w, NULL); }
static void read_clock(int me) {
    int other = 1 - me;
    LONG64 other_before = -1;
    InterlockedIncrement(&started);
    while (started < 2) {}                                /* the other started: on two vCPUs it reads beside this one */
    for (int i = 0; i < ROUNDS; i++) {
        LONG64 theirs = c.last[other], mine = c.last[me];
        LONG64 seen = theirs > mine ? theirs : mine;
        LARGE_INTEGER t;
        QueryPerformanceCounter(&t);
        if (t.QuadPart < seen) {
            if (!c.below[me]) {
                c.first_low[me] = t.QuadPart;
                c.first_seen[me] = seen;
            }
            c.below[me]++;
        }
        if (theirs != other_before) {
            c.moved[me]++;
            other_before = theirs;
        }
        c.last[me] = t.QuadPart;
    }
}
static DWORD WINAPI reader(LPVOID p) {
    (void)p;
    read_clock(1);
    return 0;
}
void start(void) {
    HANDLE t = CreateThread(NULL, 0, reader, NULL, 0, NULL);
    read_clock(0);
    WaitForSingleObject(t, INFINITE);
    if (c.below[0] || c.below[1]) line("below\n", 6);
    else line("done\n", 5);
    ExitProcess(0);
}
