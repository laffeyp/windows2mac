#include <windows.h>
/* Sprint 353's measure of slice_until's rule, committed in Sprint 354 (reviewer-4's twenty-fourth review, Low 1): two
 * threads on two vCPUs each count to N and end; the main thread waits with no timeout and prints the counter ticks the
 * two took, in hex. No thread waits with a deadline, so before Sprint 353 each vCPU's slice never ended; with its rule
 * it ends at each TIME_SLICE while the other vCPU holds a thread, and the ticks read that rule's cost. */
#define N 300000000
static volatile LONG64 counts[2];
static void line(const char *s, DWORD n) { DWORD w; WriteFile(GetStdHandle(STD_OUTPUT_HANDLE), s, n, &w, NULL); }
static DWORD WINAPI counter(LPVOID p) {
    int i = (int)(INT_PTR)p;
    while (++counts[i] < N) {}
    return 0;
}
void start(void) {
    LARGE_INTEGER a, b;
    QueryPerformanceCounter(&a);
    HANDLE t[2];
    t[0] = CreateThread(NULL, 0, counter, (LPVOID)0, 0, NULL);
    t[1] = CreateThread(NULL, 0, counter, (LPVOID)1, 0, NULL);
    WaitForMultipleObjects(2, t, TRUE, INFINITE);
    QueryPerformanceCounter(&b);
    unsigned long long d = (unsigned long long)(b.QuadPart - a.QuadPart);
    char s[17];
    for (int i = 0; i < 16; i++) s[i] = "0123456789abcdef"[(d >> (60 - 4 * i)) & 15];
    s[16] = '\n';
    line(s, 17);
    ExitProcess(0);
}
