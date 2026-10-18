#include <windows.h>
/* The parent starts itself as a child and spins, polling the child's handle without blocking; the child sleeps 20 ms
 * and exits. The child's sleep must end while the parent spins. */
static void say(const char *s, DWORD n) { DWORD w; WriteFile(GetStdHandle(STD_OUTPUT_HANDLE), s, n, &w, NULL); }
void start(void) {
    const WCHAR *line = GetCommandLineW();
    int child = 0;
    for (const WCHAR *p = line; *p; p++) if (p[0] == L'-' && p[1] == L'c') child = 1;
    if (child) { Sleep(20); ExitProcess(7); }
    WCHAR cmd[] = L"probe.exe -c";
    STARTUPINFOW si = {0}; si.cb = sizeof si;
    PROCESS_INFORMATION pi;
    if (!CreateProcessW(L"probe.exe", cmd, NULL, NULL, FALSE, 0, NULL, NULL, &si, &pi)) { say("create failed\n", 14); ExitProcess(1); }
    volatile unsigned long long spins = 0;
    while (WaitForSingleObject(pi.hProcess, 0) != WAIT_OBJECT_0) spins++;
    DWORD code = 0; GetExitCodeProcess(pi.hProcess, &code);
    say(code == 7 ? "child ended\n" : "child bad code\n", code == 7 ? 12 : 15);
    ExitProcess(0);
}
