#include <windows.h>
/* Sprint 350 (reviewer-4's nineteenth review, Medium 1): a worker waits for all of a named event, which is the host's,
 * and an unnamed auto-reset event; the main thread closes the named event's only handle while the worker waits, writes
 * "closed", sets the unnamed event and writes "done", and ends the process with the worker still waiting. The host's
 * close of the named event (W2M_KERNEL_CLOSE) comes at CloseHandle of its last handle, before "closed", though the
 * worker's wait still holds the object; and the set, which offers the wait its objects, asks nothing of the closed one. */
static HANDLE shared, local, ready;
static void line(const char *s, DWORD n) { DWORD w; WriteFile(GetStdHandle(STD_OUTPUT_HANDLE), s, n, &w, NULL); }
static DWORD WINAPI worker(LPVOID p) {
    (void)p;
    HANDLE both[2] = {shared, local};
    SetEvent(ready);
    WaitForMultipleObjects(2, both, TRUE, INFINITE);
    line("worker woke\n", 12);
    return 0;
}
void start(void) {
    DWORD tid;
    shared = CreateEventW(NULL, TRUE, FALSE, L"w2m_shared_close");
    local = CreateEventW(NULL, FALSE, FALSE, NULL);
    ready = CreateEventW(NULL, FALSE, FALSE, NULL);
    CreateThread(NULL, 0, worker, NULL, 0, &tid);
    WaitForSingleObject(ready, INFINITE);
    Sleep(20);
    CloseHandle(shared);
    line("closed\n", 7);
    SetEvent(local);
    Sleep(20);
    line("done\n", 5);
    ExitProcess(0);
}
