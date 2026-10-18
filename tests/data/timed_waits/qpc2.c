#include <windows.h>
/* A worker thread reads QueryPerformanceCounter; the main thread spins until it has, keeping its own vCPU busy, so the
 * worker runs on another, and prints the reading. */
static volatile LONG done;
static volatile unsigned long long reading;
static void hex(unsigned long long v) { char b[18]; for (int i = 15; i >= 0; i--) { b[i] = "0123456789abcdef"[v & 15]; v >>= 4; } b[16] = '\n'; DWORD w; WriteFile(GetStdHandle(STD_OUTPUT_HANDLE), b, 17, &w, NULL); }
static DWORD WINAPI worker(LPVOID p) { (void)p; LARGE_INTEGER c; QueryPerformanceCounter(&c); reading = (unsigned long long)c.QuadPart; done = 1; return 0; }
void start(void) {
    HANDLE h = CreateThread(NULL, 0, worker, NULL, 0, NULL);
    while (!done) {}
    hex(reading);
    WaitForSingleObject(h, INFINITE);
    ExitProcess(0);
}
