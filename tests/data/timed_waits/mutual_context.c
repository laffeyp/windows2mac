#include <windows.h>
/* Two threads suspend each other at once, 20 rounds; a also reads b's context while it holds b suspended, then resumes
 * it. The main thread resumes a until it ends (a resumes b), then b. */
static volatile LONG ready, read_ok;
static HANDLE ha, hb;
static DWORD WINAPI a_fn(LPVOID p) {
    (void)p;
    InterlockedIncrement(&ready);
    while (ready < 2) {}
    SuspendThread(hb);
    CONTEXT c;
    c.ContextFlags = CONTEXT_CONTROL;
    if (GetThreadContext(hb, &c)) InterlockedIncrement(&read_ok);
    ResumeThread(hb);
    return 0;
}
static DWORD WINAPI b_fn(LPVOID p) { (void)p; InterlockedIncrement(&ready); while (ready < 2) {} SuspendThread(ha); return 0; }
static void say(const char *s, DWORD n) { DWORD w; WriteFile(GetStdHandle(STD_OUTPUT_HANDLE), s, n, &w, NULL); }
static void until_ended(HANDLE h) { while (WaitForSingleObject(h, 1)) while (ResumeThread(h) > 1) {} }
void start(void) {
    for (int round = 0; round < 20; round++) {
        ready = 0;
        ha = CreateThread(NULL, 0, a_fn, NULL, CREATE_SUSPENDED, NULL);
        hb = CreateThread(NULL, 0, b_fn, NULL, CREATE_SUSPENDED, NULL);
        ResumeThread(ha);
        ResumeThread(hb);
        until_ended(ha);
        until_ended(hb);
        CloseHandle(ha);
        CloseHandle(hb);
    }
    say(read_ok == 20 ? "done\n" : "bad\n", read_ok == 20 ? 5 : 4);
    ExitProcess(0);
}
