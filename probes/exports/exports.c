// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* exports.c — the names kernel32.dll and kernelbase.dll export on this Windows (Sprint 044).
 *
 * No C runtime. Walks each module's export directory in memory (PE/COFF, "The .edata Section") and
 * prints one JSON row per name, in the directory's order (sorted by the linker): {"m":module,
 * "n":name,"f":forwarded}. GetProcAddress in the guest answers NULL with ERROR_PROC_NOT_FOUND only
 * for a name its module does not export; an exported name w2m does not serve yet gets an address
 * that stops the run by name. Then every *.dll name in the system directory, {"system32":name}:
 * LoadLibrary answers ERROR_MOD_NOT_FOUND only for a name found nowhere on the search path.
 */
#include <windows.h>

static char out[1 << 21];
static unsigned n;

static void put(const char *s) { while (*s) out[n++] = *s++; }

static void dump(const WCHAR *wname, const char *name) {
    unsigned char *base = (unsigned char *)GetModuleHandleW(wname);
    IMAGE_NT_HEADERS64 *nt = (IMAGE_NT_HEADERS64 *)(base + ((IMAGE_DOS_HEADER *)base)->e_lfanew);
    IMAGE_DATA_DIRECTORY d = nt->OptionalHeader.DataDirectory[IMAGE_DIRECTORY_ENTRY_EXPORT];
    IMAGE_EXPORT_DIRECTORY *e = (IMAGE_EXPORT_DIRECTORY *)(base + d.VirtualAddress);
    DWORD *names = (DWORD *)(base + e->AddressOfNames);
    WORD *ords = (WORD *)(base + e->AddressOfNameOrdinals);
    DWORD *funcs = (DWORD *)(base + e->AddressOfFunctions);
    for (DWORD i = 0; i < e->NumberOfNames; i++) {
        DWORD rva = funcs[ords[i]];
        int forwarded = rva >= d.VirtualAddress && rva < d.VirtualAddress + d.Size;
        put("{\"m\":\""); put(name); put("\",\"n\":\""); put((const char *)(base + names[i]));
        put(forwarded ? "\",\"f\":true}\n" : "\",\"f\":false}\n");
    }
}

static void system32(void) {
    WIN32_FIND_DATAW f;
    HANDLE h = FindFirstFileW(L"C:\\Windows\\System32\\*.dll", &f);
    if (h == INVALID_HANDLE_VALUE) return;
    do {
        put("{\"system32\":\"");
        for (WCHAR *c = f.cFileName; *c; c++) out[n++] = *c < 128 ? (char)*c : '?';
        put("\"}\n");
    } while (FindNextFileW(h, &f));
    FindClose(h);
}

void start(void) {
    dump(L"kernel32.dll", "kernel32");
    dump(L"kernelbase.dll", "kernelbase");
    system32();
    DWORD written;
    WriteFile(GetStdHandle(STD_OUTPUT_HANDLE), out, n, &written, NULL);
    ExitProcess(0);
}
