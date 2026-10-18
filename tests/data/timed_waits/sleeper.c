#include <windows.h>
static volatile LONG stop, count;
static DWORD WINAPI spinner(LPVOID p) { (void)p; while (!stop) count++; return 0; }
static void say(const char *s, DWORD n) { DWORD w; WriteFile(GetStdHandle(STD_OUTPUT_HANDLE), s, n, &w, NULL); }
void start(void) {
    HANDLE h = CreateThread(NULL, 0, spinner, NULL, 0, NULL);
    while (!count) SwitchToThread();
    Sleep(20);                       /* the spinner never blocks: the sleep's end must still come */
    say("woke\n", 5);
    stop = 1;
    WaitForSingleObject(h, INFINITE);
    say("done\n", 5);
    ExitProcess(0);
}
