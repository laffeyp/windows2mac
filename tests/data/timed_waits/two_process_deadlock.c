#include <windows.h>
/* Two processes, each waiting INFINITE on a named event only the other sets (Sprint 364, card 4f3): the parent starts
 * itself as a child and waits on W2mDeadlockParent, which the child sets only after its own wait on W2mDeadlockChild,
 * which the parent sets only after its wait. Neither wait ends: on one vCPU the machine finds every process waiting with
 * no timeout, and on several the request server gives the verdict across processes. "woken" is printed only if a wait
 * ended, which would be a fault. With "grandchild" in the parent's arguments its child first asks CreateProcessW for a
 * third process, a request the host refuses, so the run stops while the child waits on its answer. */
static void say(const char *s, DWORD n) { DWORD w; WriteFile(GetStdHandle(STD_OUTPUT_HANDLE), s, n, &w, NULL); }
static int has(const WCHAR *line, const WCHAR *word) {
    for (const WCHAR *p = line; *p; p++) {
        int k = 0;
        while (word[k] && p[k] == word[k]) k++;
        if (!word[k]) return 1;
    }
    return 0;
}
void start(void) {
    const WCHAR *line = GetCommandLineW();
    HANDLE parent = CreateEventW(NULL, FALSE, FALSE, L"W2mDeadlockParent");
    HANDLE child = CreateEventW(NULL, FALSE, FALSE, L"W2mDeadlockChild");
    STARTUPINFOW si = {0};
    si.cb = sizeof si;
    PROCESS_INFORMATION pi;
    if (has(line, L" -c")) {
        if (has(line, L" -g")) {
            WCHAR third[] = L"probe.exe -c";
            CreateProcessW(L"probe.exe", third, NULL, NULL, FALSE, 0, NULL, NULL, &si, &pi);
        }
        WaitForSingleObject(child, INFINITE);
        SetEvent(parent);
        ExitProcess(3);
    }
    WCHAR plain[] = L"probe.exe -c";
    WCHAR asking[] = L"probe.exe -c -g";
    if (!CreateProcessW(L"probe.exe", has(line, L" grandchild") ? asking : plain, NULL, NULL, FALSE, 0, NULL, NULL, &si,
                        &pi)) {
        say("create failed\n", 14);
        ExitProcess(1);
    }
    WaitForSingleObject(parent, INFINITE);
    SetEvent(child);
    say("woken\n", 6);
    ExitProcess(0);
}
