#include <windows.h>
/* Two threads suspend each other at once, 20 rounds; the main thread resumes both and waits for them. */
static volatile LONG ready;
static HANDLE ha, hb;
static DWORD WINAPI a_fn(LPVOID p) { (void)p; InterlockedIncrement(&ready); while (ready < 2) {} SuspendThread(hb); return 0; }
static DWORD WINAPI b_fn(LPVOID p) { (void)p; InterlockedIncrement(&ready); while (ready < 2) {} SuspendThread(ha); return 0; }
static void say(const char *s, DWORD n) { DWORD w; WriteFile(GetStdHandle(STD_OUTPUT_HANDLE), s, n, &w, NULL); }
static void resume_all(HANDLE h) { while (ResumeThread(h) > 1) {} }
void start(void) {
    for (int round = 0; round < 20; round++) {
        ready = 0;
        ha = CreateThread(NULL, 0, a_fn, NULL, CREATE_SUSPENDED, NULL);
        hb = CreateThread(NULL, 0, b_fn, NULL, CREATE_SUSPENDED, NULL);
        ResumeThread(ha);
        ResumeThread(hb);
        Sleep(5);
        while (WaitForSingleObject(ha, 0) || WaitForSingleObject(hb, 0)) {
            resume_all(ha);
            resume_all(hb);
            Sleep(1);
        }
        CloseHandle(ha);
        CloseHandle(hb);
    }
    say("done\n", 5);
    ExitProcess(0);
}
