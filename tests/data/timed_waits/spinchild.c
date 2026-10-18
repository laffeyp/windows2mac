#include <windows.h>
/* A spinner thread, then a child process that waits forever on nothing it is given; the main thread sleeps 20 ms. */
static volatile LONG stop, count;
static DWORD WINAPI spinner(LPVOID p) { (void)p; while (!stop) count++; return 0; }
static void say(const char *s, DWORD n) { DWORD w; WriteFile(GetStdHandle(STD_OUTPUT_HANDLE), s, n, &w, NULL); }
void start(void) {
    const WCHAR *line = GetCommandLineW();
    for (const WCHAR *p = line; *p; p++) if (p[0] == L'-' && p[1] == L'c') { Sleep(INFINITE - 1); ExitProcess(0); }
    HANDLE h = CreateThread(NULL, 0, spinner, NULL, 0, NULL);
    while (!count) SwitchToThread();
    WCHAR cmd[] = L"probe.exe -c";
    STARTUPINFOW si = {0};
    si.cb = sizeof si;
    PROCESS_INFORMATION pi;
    CreateProcessW(L"probe.exe", cmd, NULL, NULL, FALSE, 0, NULL, NULL, &si, &pi);
    Sleep(20);
    stop = 1;
    WaitForSingleObject(h, INFINITE);
    say("done\n", 5);
    ExitProcess(0);
}
