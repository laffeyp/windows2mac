// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* static_tls.c — static TLS in a DLL the program imports (Sprint 046, family "static TLS").
 *
 * No C runtime; a console program without TLS that imports static_tls_dll.dll, which has static TLS, as
 * the launcher (no TLS) imports UnityPlayer.dll (static TLS: it reads gs:[0x58] at 154,660 instructions
 * under w2m, frontier 2026-10-10). Rows: the DLL's TLS index as the loader wrote it; whether
 * TEB.ThreadLocalStoragePointer (gs:[0x58]) is set and its slot at that index holds the block the
 * thread variables live in; the initialized variable's value and the zero-filled one's bytes; a write
 * landing in the thread's copy, not the image's template; the image's TLS directory as linked.
 */
#include <windows.h>

__declspec(dllimport) ULONG tls_dll_index(void);
__declspec(dllimport) int *tls_dll_counter(void);
__declspec(dllimport) unsigned char *tls_dll_zeros(void);
__declspec(dllimport) const IMAGE_TLS_DIRECTORY64 *tls_dll_directory(void);

static char out[4096];
static unsigned n;

static void put(const char *s) { while (*s) out[n++] = *s++; }
static void putu(unsigned long long v) {
    char t[24];
    int k = 0;
    do { t[k++] = (char)('0' + v % 10); v /= 10; } while (v);
    while (k) out[n++] = t[--k];
}
static void row(const char *s) { put("{\"s\":\""); put(s); put("\""); }
static void field(const char *k, unsigned long long v) { put(",\""); put(k); put("\":"); putu(v); }
static void flag(const char *k, int v) { put(",\""); put(k); put(v ? "\":true" : "\":false"); }
static void end(void) { put("}\n"); }

void start(void) {
    void **array;
    __asm__ volatile("movq %%gs:0x58, %0" : "=r"(array));
    ULONG index = tls_dll_index();
    int *counter = tls_dll_counter();
    unsigned char *zeros = tls_dll_zeros();
    const IMAGE_TLS_DIRECTORY64 *dir = tls_dll_directory();
    unsigned long long size = dir->EndAddressOfRawData - dir->StartAddressOfRawData;
    unsigned char *block = array ? (unsigned char *)array[index] : 0;

    row("tls.index"); field("value", index); end();
    row("tls.array"); flag("set", array != 0); flag("slot_holds_block", block && (unsigned char *)counter >= block &&
                                                       (unsigned char *)counter < block + size + dir->SizeOfZeroFill + 4096);
    end();
    int z = 1;
    for (int i = 0; i < 64; i++) z &= zeros[i] == 0;
    row("tls.values"); field("counter", (unsigned long long)*counter); flag("zeros_zero", z); end();
    *counter += 1;
    unsigned char *template_start = (unsigned char *)dir->StartAddressOfRawData;
    unsigned long long offset = (unsigned long long)((unsigned char *)counter - block);
    row("tls.write"); field("counter_after", (unsigned long long)*counter);
    flag("template_unchanged", offset + 4 <= size && *(int *)(template_start + offset) == 41); end();
    row("tls.directory"); flag("callbacks_empty", *(void **)dir->AddressOfCallBacks == 0);
    field("zero_fill", dir->SizeOfZeroFill); flag("index_address_in_image", dir->AddressOfIndex != 0); end();

    /* the DLL's static C runtime asks it at start-up (w2m, 2026-10-10); the probe asks too */
    row("file_apis_ansi"); flag("value", AreFileApisANSI() != 0); end();

    DWORD written;
    WriteFile(GetStdHandle(STD_OUTPUT_HANDLE), out, n, &written, NULL);
    ExitProcess(0);
}
