#include <windows.h>
/* The child's main thread starts a spinner, says it is up, and waits on a named event; the parent's worker, on the
 * other vCPU, polls for the child's word without blocking, spins a little so the child is waiting, sets the event,
 * then waits for the child's answer on a second. The parent's main thread waits for the worker, so the parent's vCPU 0
 * has no thread and the Machine's thread runs the child, whose slice never ends: the child wakes only if the parent's
 * wake stops it. */
static volatile LONG stop;
static HANDLE up, asked, answered;
static void say(const char *s, DWORD n) { DWORD w; WriteFile(GetStdHandle(STD_OUTPUT_HANDLE), s, n, &w, NULL); }
static DWORD WINAPI spinner(LPVOID p) { (void)p; while (!stop) {} return 0; }
static DWORD WINAPI worker(LPVOID p) {
    (void)p;
    while (WaitForSingleObject(up, 0) != WAIT_OBJECT_0) {}
    for (volatile unsigned i = 0; i < 20000000; i++) {}    /* the child reaches its wait meanwhile */
    SetEvent(asked);
    WaitForSingleObject(answered, INFINITE);
    return 0;
}
void start(void) {
    up = CreateEventW(NULL, FALSE, FALSE, L"w2m-crosswake-up");
    asked = CreateEventW(NULL, FALSE, FALSE, L"w2m-crosswake-asked");
    answered = CreateEventW(NULL, FALSE, FALSE, L"w2m-crosswake-answered");
    const WCHAR *line = GetCommandLineW();
    for (const WCHAR *p = line; *p; p++)
        if (p[0] == L'-' && p[1] == L'c') {
            CreateThread(NULL, 0, spinner, NULL, 0, NULL);
            SetEvent(up);
            WaitForSingleObject(asked, INFINITE);
            SetEvent(answered);
            ExitProcess(0);
        }
    HANDLE h = CreateThread(NULL, 0, worker, NULL, 0, NULL);
    WCHAR cmd[] = L"probe.exe -c";
    STARTUPINFOW si = {0};
    si.cb = sizeof si;
    PROCESS_INFORMATION pi;
    CreateProcessW(L"probe.exe", cmd, NULL, NULL, FALSE, 0, NULL, NULL, &si, &pi);
    WaitForSingleObject(h, INFINITE);
    say("done\n", 5);
    ExitProcess(0);
}
