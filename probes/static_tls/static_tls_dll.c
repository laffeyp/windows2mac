/* static_tls_dll.c — a DLL with static TLS, the static_tls probe imports it (Sprint 046). Built with the
 * static C runtime, whose tlssup supplies the TLS directory, as UnityPlayer.dll is built: a hand-made
 * directory (no C runtime) faulted on every runner while w2m ran it, so the probe takes the ordinary
 * route. Two thread variables: one initialized, one zero-filled. */
#include <windows.h>

__declspec(thread) int counter = 41;
__declspec(thread) unsigned char zeros[64];

extern IMAGE_TLS_DIRECTORY64 _tls_used;            /* the C runtime's (tlssup) */
extern ULONG _tls_index;

BOOL WINAPI DllMain(HINSTANCE instance, DWORD reason, LPVOID reserved) {
    (void)instance; (void)reason; (void)reserved;
    return TRUE;
}

__declspec(dllexport) ULONG tls_dll_index(void) { return _tls_index; }
__declspec(dllexport) int *tls_dll_counter(void) { return &counter; }
__declspec(dllexport) unsigned char *tls_dll_zeros(void) { return zeros; }
__declspec(dllexport) const IMAGE_TLS_DIRECTORY64 *tls_dll_directory(void) { return &_tls_used; }
