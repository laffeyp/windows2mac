/* static_tls_dll.c — a DLL with static TLS, the static_tls probe imports it (Sprint 046). No C runtime,
 * so the TLS directory is defined here, as the MSVC runtime's tlssup.c defines it: _tls_index, the
 * .tls section's start and end markers, an empty callback list, and _tls_used, which the linker makes
 * the image's TLS directory. Two thread variables: one initialized, one zero-filled. */
#include <windows.h>

ULONG _tls_index = 0xA5A5A5A5u;

__attribute__((section(".tls"))) char _tls_start = 0;
__attribute__((section(".tls$ZZZ"))) char _tls_end = 0;

__attribute__((section(".CRT$XLA"), used)) PIMAGE_TLS_CALLBACK __xl_a = 0;
__attribute__((section(".CRT$XLZ"), used)) PIMAGE_TLS_CALLBACK __xl_z = 0;

/* designated: the Windows SDK's IMAGE_TLS_DIRECTORY64 puts Characteristics in a union, mingw's does not */
const IMAGE_TLS_DIRECTORY64 _tls_used = {.StartAddressOfRawData = (ULONGLONG)&_tls_start,
                                         .EndAddressOfRawData = (ULONGLONG)&_tls_end,
                                         .AddressOfIndex = (ULONGLONG)&_tls_index,
                                         .AddressOfCallBacks = (ULONGLONG)(&__xl_a + 1), .SizeOfZeroFill = 0};

__declspec(thread) int counter = 41;
__declspec(thread) unsigned char zeros[64];

BOOL WINAPI dll_main(HINSTANCE instance, DWORD reason, LPVOID reserved) {
    (void)instance; (void)reason; (void)reserved;
    return TRUE;
}

__declspec(dllexport) ULONG tls_dll_index(void) { return _tls_index; }
__declspec(dllexport) int *tls_dll_counter(void) { return &counter; }
__declspec(dllexport) unsigned char *tls_dll_zeros(void) { return zeros; }
__declspec(dllexport) const IMAGE_TLS_DIRECTORY64 *tls_dll_directory(void) { return &_tls_used; }
