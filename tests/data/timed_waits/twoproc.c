#include <windows.h>
/* The parent starts itself as a child, passing its counter, and spins polling the child's handle; the child sleeps
 * 1 ms and exits 7 when its first counter reading is not behind the parent's (one clock for the machine), else 8. The child's
 * sleep must end while the parent spins. */
static void say(const char *s, DWORD n) { DWORD w; WriteFile(GetStdHandle(STD_OUTPUT_HANDLE), s, n, &w, NULL); }
static unsigned long long parse(const WCHAR *p) {
    unsigned long long v = 0;
    for (; *p; p++) v = v * 16 + (*p <= L'9' ? *p - L'0' : *p - L'a' + 10);
    return v;
}
void start(void) {
    const WCHAR *line = GetCommandLineW();
    for (const WCHAR *p = line; *p; p++) {
        if (p[0] == L'-' && p[1] == L'c' && p[2] == L' ') {
            unsigned long long parent = parse(p + 3);
            LARGE_INTEGER now;
            QueryPerformanceCounter(&now);                  /* its first reading: not behind the parent's */
            Sleep(1);
            ExitProcess((unsigned long long)now.QuadPart >= parent ? 7 : 8);
        }
    }
    for (volatile int i = 0; i < 5000000; i++) {}          /* the parent's clock well past a new child's start-up */
    LARGE_INTEGER c;
    QueryPerformanceCounter(&c);
    WCHAR cmd[40] = L"probe.exe -c ";
    unsigned long long v = (unsigned long long)c.QuadPart;
    for (int i = 0; i < 16; i++) cmd[13 + i] = L"0123456789abcdef"[v >> (60 - 4 * i) & 15];
    cmd[29] = 0;
    STARTUPINFOW si = {0};
    si.cb = sizeof si;
    PROCESS_INFORMATION pi;
    if (!CreateProcessW(L"probe.exe", cmd, NULL, NULL, FALSE, 0, NULL, NULL, &si, &pi)) { say("create failed\n", 14); ExitProcess(1); }
    while (WaitForSingleObject(pi.hProcess, 0) != WAIT_OBJECT_0) {}
    DWORD code = 0;
    GetExitCodeProcess(pi.hProcess, &code);
    say(code == 7 ? "child ended\n" : code == 8 ? "clock behind\n" : "child bad code\n", code == 7 ? 12 : code == 8 ? 13 : 15);
    ExitProcess(0);
}
