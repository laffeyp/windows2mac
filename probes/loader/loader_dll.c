/* loader_dll.c — the DLL the loader probe imports (Sprint 046). No C runtime; KERNEL32 only. Its entry
 * logs every call (reason, whether lpReserved is NULL, the instance handle); the probe reads the log
 * through two exports. */
#include <windows.h>

struct call { DWORD reason; int reserved_null; HINSTANCE instance; };
static struct call calls[16];
static int count;

BOOL WINAPI dll_main(HINSTANCE instance, DWORD reason, LPVOID reserved) {
    if (count < 16) calls[count] = (struct call){reason, reserved == NULL, instance};
    count++;
    return TRUE;
}

__declspec(dllexport) int loader_dll_count(void) { return count; }

__declspec(dllexport) void loader_dll_call(int i, DWORD *reason, int *reserved_null, void **instance) {
    *reason = calls[i].reason;
    *reserved_null = calls[i].reserved_null;
    *instance = calls[i].instance;
}
