#include <windows.h>
/* A spinner thread, then a child process that sets a named event and ends; the main thread waits on that event, then
 * stops the spinner. On two vCPUs the main thread's vCPU has no thread while the spinner runs on the other, and the
 * main thread wakes only if the child runs meanwhile. */
static volatile LONG stop, count;
static DWORD WINAPI spinner(LPVOID p) { (void)p; while (!stop) count++; return 0; }
static void say(const char *s, DWORD n) { DWORD w; WriteFile(GetStdHandle(STD_OUTPUT_HANDLE), s, n, &w, NULL); }
void start(void) {
    HANDLE ev = CreateEventW(NULL, FALSE, FALSE, L"w2m-handshake");
    const WCHAR *line = GetCommandLineW();
    for (const WCHAR *p = line; *p; p++) if (p[0] == L'-' && p[1] == L'c') { SetEvent(ev); ExitProcess(0); }
    HANDLE h = CreateThread(NULL, 0, spinner, NULL, 0, NULL);
    while (!count) SwitchToThread();
    WCHAR cmd[] = L"probe.exe -c";
    STARTUPINFOW si = {0};
    si.cb = sizeof si;
    PROCESS_INFORMATION pi;
    CreateProcessW(L"probe.exe", cmd, NULL, NULL, FALSE, 0, NULL, NULL, &si, &pi);
    WaitForSingleObject(ev, INFINITE);
    stop = 1;
    WaitForSingleObject(h, INFINITE);
    say("done\n", 5);
    ExitProcess(0);
}
