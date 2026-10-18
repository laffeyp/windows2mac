#include <windows.h>
static void hex(unsigned long long v) { char b[18]; for (int i = 15; i >= 0; i--) { b[i] = "0123456789abcdef"[v & 15]; v >>= 4; } b[16] = '\n'; DWORD w; WriteFile(GetStdHandle(STD_OUTPUT_HANDLE), b, 17, &w, NULL); }
void start(void) { LARGE_INTEGER c; QueryPerformanceCounter(&c); hex((unsigned long long)c.QuadPart); ExitProcess(0); }
