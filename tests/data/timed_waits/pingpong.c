#include <windows.h>
/* The parent's worker and the child's main thread take turns over two named events, 2,000 rounds: the worker sets
 * ping and waits on pong, the child waits on ping and sets pong. The parent's main thread waits for the worker, so on
 * two vCPUs the worker runs on vCPU 1 beside the Machine's thread, which runs the child: each process's wake reaches
 * the other from a thread that does not serve it. */
#define ROUNDS 2000
static HANDLE ping, pong;
static void say(const char *s, DWORD n) { DWORD w; WriteFile(GetStdHandle(STD_OUTPUT_HANDLE), s, n, &w, NULL); }
static DWORD WINAPI worker(LPVOID p) {
    (void)p;
    for (int r = 0; r < ROUNDS; r++) {
        SetEvent(ping);
        WaitForSingleObject(pong, INFINITE);
    }
    return 0;
}
void start(void) {
    ping = CreateEventW(NULL, FALSE, FALSE, L"w2m-pingpong-ping");
    pong = CreateEventW(NULL, FALSE, FALSE, L"w2m-pingpong-pong");
    const WCHAR *line = GetCommandLineW();
    for (const WCHAR *p = line; *p; p++)
        if (p[0] == L'-' && p[1] == L'c') {
            for (int r = 0; r < ROUNDS; r++) {
                WaitForSingleObject(ping, INFINITE);
                SetEvent(pong);
            }
            ExitProcess(0);
        }
    WCHAR cmd[] = L"probe.exe -c";
    STARTUPINFOW si = {0};
    si.cb = sizeof si;
    PROCESS_INFORMATION pi;
    CreateProcessW(L"probe.exe", cmd, NULL, NULL, FALSE, 0, NULL, NULL, &si, &pi);
    HANDLE h = CreateThread(NULL, 0, worker, NULL, 0, NULL);
    WaitForSingleObject(h, INFINITE);
    WaitForSingleObject(pi.hProcess, INFINITE);
    say("done\n", 5);
    ExitProcess(0);
}
