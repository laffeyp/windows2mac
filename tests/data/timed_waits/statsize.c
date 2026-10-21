#include <windows.h>
/* The main thread reads its own file's size, then spins so it keeps vCPU 0 while a worker, on vCPU 1, reads the size
 * again. GetFileAttributesExW's stat of the file reaches the host with a buffer on the runtime's stack, which each vCPU
 * has its own of: the worker's size is right only if the host writes the stat to vCPU 1's. */
static volatile LONG done;
static volatile DWORD sizes[2];
static void say(const char *s, DWORD n) { DWORD w; WriteFile(GetStdHandle(STD_OUTPUT_HANDLE), s, n, &w, NULL); }
static DWORD size_of_self(void) {
    WIN32_FILE_ATTRIBUTE_DATA data;
    if (!GetFileAttributesExW(L"probe.exe", GetFileExInfoStandard, &data)) return 0;
    return data.nFileSizeLow;
}
static DWORD WINAPI worker(LPVOID p) { (void)p; sizes[1] = size_of_self(); done = 1; return 0; }
void start(void) {
    sizes[0] = size_of_self();
    HANDLE h = CreateThread(NULL, 0, worker, NULL, 0, NULL);
    while (!done) {}
    WaitForSingleObject(h, INFINITE);
    if (sizes[0] == sizes[1] && sizes[0] > 0) say("done\n", 5);
    else say("sizes differ\n", 13);
    ExitProcess(0);
}
