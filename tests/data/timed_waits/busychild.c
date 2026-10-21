#include <windows.h>
/* A child process that spins and never ends; a worker thread sets an event 20 times and spins until the main thread,
 * waiting on that event, has taken its round. On two vCPUs the main thread's vCPU waits while the child spins, and
 * the main thread runs its rounds there only if the child's turn ends when the worker readies it. The child counts in
 * its own copy of rounds_done. */
static volatile LONG rounds_done;
static HANDLE go;
static void say(const char *s, DWORD n) { DWORD w; WriteFile(GetStdHandle(STD_OUTPUT_HANDLE), s, n, &w, NULL); }
static DWORD WINAPI worker(LPVOID p) {
    (void)p;
    for (int r = 0; r < 20; r++) {
        SetEvent(go);
        while (rounds_done <= r) {}
    }
    return 0;
}
void start(void) {
    const WCHAR *line = GetCommandLineW();
    for (const WCHAR *p = line; *p; p++) if (p[0] == L'-' && p[1] == L'c') for (;;) rounds_done++;
    go = CreateEventA(NULL, FALSE, FALSE, NULL);
    WCHAR cmd[] = L"probe.exe -c";
    STARTUPINFOW si = {0};
    si.cb = sizeof si;
    PROCESS_INFORMATION pi;
    CreateProcessW(L"probe.exe", cmd, NULL, NULL, FALSE, 0, NULL, NULL, &si, &pi);
    HANDLE h = CreateThread(NULL, 0, worker, NULL, 0, NULL);
    for (int r = 0; r < 20; r++) {
        WaitForSingleObject(go, INFINITE);
        rounds_done = r + 1;
    }
    WaitForSingleObject(h, INFINITE);
    say("done\n", 5);
    ExitProcess(0);
}
