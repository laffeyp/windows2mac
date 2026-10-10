/* loader.c — the Windows loader with two images (Sprint 046, family "loader").
 *
 * No C runtime; a console program that statically imports loader_dll.dll (built beside it, KERNEL32
 * only), as the launcher imports UnityPlayer.dll. Rows, each after SetLastError(0xDEAD) where a call
 * may set it:
 *   - the DLL's entry calls so far at the exe's entry: reason, lpReserved NULL or not, the instance
 *     equal to the DLL's module handle
 *   - GetModuleHandleW for the DLL by "loader_dll.dll", "LOADER_DLL.DLL" and "loader_dll"; the DLL
 *     loaded at its preferred base or not; GetModuleFileNameW in the exe's directory; GetProcAddress of
 *     an export equal to the imported address
 *   - the PEB loader lists in load, memory and initialization order: each entry's base name, and
 *     whether its DllBase is GetModuleHandleW(that name). Offsets: PEB.Ldr 0x18; PEB_LDR_DATA's
 *     InLoadOrderModuleList 0x10, InMemoryOrderModuleList 0x20, InInitializationOrderModuleList 0x30;
 *     LDR_DATA_TABLE_ENTRY links at 0x00, 0x10, 0x20, DllBase 0x30, BaseDllName 0x58 (winternl.h gives
 *     InMemoryOrderLinks at 0x10 and DllBase at 0x30; the probe checks the three lists hold the same set)
 */
#include <windows.h>

#define SENTINEL 0xDEADu

__declspec(dllimport) int loader_dll_count(void);
__declspec(dllimport) void loader_dll_call(int i, DWORD *reason, int *reserved_null, void **instance);

static char out[16384];
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

static unsigned char *peb(void) {
    unsigned char *p;
    __asm__ volatile("movq %%gs:0x60, %0" : "=r"(p));
    return p;
}

static void wide_item(const WCHAR *w, unsigned units) {
    out[n++] = '"';
    for (unsigned i = 0; i < units; i++) out[n++] = (w[i] >= 32 && w[i] < 127 && w[i] != '"' && w[i] != '\\') ? (char)w[i] : '?';
    out[n++] = '"';
}

/* One list: names in order, and how many entries' DllBase equals GetModuleHandleW(name). */
static void ldr_list(const char *name, unsigned head_off, unsigned link_off, unsigned *entries) {
    unsigned char *ldr = *(unsigned char **)(peb() + 0x18);
    LIST_ENTRY *head = (LIST_ENTRY *)(ldr + head_off);
    unsigned count = 0, bases_match = 0;
    row(name);
    put(",\"names\":[");
    for (LIST_ENTRY *e = head->Flink; e != head && count < 64; e = e->Flink, count++) {
        unsigned char *entry = (unsigned char *)e - link_off;
        void *base = *(void **)(entry + 0x30);
        unsigned short bytes = *(unsigned short *)(entry + 0x58);
        WCHAR *buffer = *(WCHAR **)(entry + 0x60);
        if (count) put(",");
        wide_item(buffer, bytes / 2);
        static WCHAR z[260];
        unsigned u = bytes / 2 < 259 ? bytes / 2 : 259;
        for (unsigned i = 0; i < u; i++) z[i] = buffer[i];
        z[u] = 0;
        bases_match += GetModuleHandleW(z) == (HMODULE)base;
    }
    put("]");
    field("count", count);
    field("bases_equal_module_handles", bases_match);
    end();
    *entries = count;
}

void start(void) {
    int calls = loader_dll_count();
    HMODULE dll = GetModuleHandleW(L"loader_dll.dll");
    row("entry.calls_at_exe_entry"); field("count", calls);
    put(",\"calls\":[");
    for (int i = 0; i < calls && i < 16; i++) {
        DWORD reason; int reserved_null; void *instance;
        loader_dll_call(i, &reason, &reserved_null, &instance);
        if (i) put(",");
        put("{\"reason\":"); putu(reason); put(",\"reserved_null\":"); put(reserved_null ? "true" : "false");
        put(",\"instance_is_module_handle\":"); put(instance == (void *)dll ? "true" : "false"); put("}");
    }
    put("]"); end();

    SetLastError(SENTINEL);
    HMODULE upper = GetModuleHandleW(L"LOADER_DLL.DLL");
    DWORD err_upper = GetLastError();
    SetLastError(SENTINEL);
    HMODULE bare = GetModuleHandleW(L"loader_dll");
    DWORD err_bare = GetLastError();
    row("module_handle"); flag("by_name_nonnull", dll != 0); flag("upper_case_same", upper == dll);
    field("upper_case_err", err_upper); flag("without_extension_same", bare == dll); field("without_extension_err", err_bare);
    end();

    unsigned char *base = (unsigned char *)dll;
    IMAGE_NT_HEADERS64 *nt = (IMAGE_NT_HEADERS64 *)(base + ((IMAGE_DOS_HEADER *)base)->e_lfanew);
    row("dll.base"); flag("at_preferred_base", (unsigned long long)base == nt->OptionalHeader.ImageBase);
    flag("dynamic_base", (nt->OptionalHeader.DllCharacteristics & IMAGE_DLLCHARACTERISTICS_DYNAMIC_BASE) != 0);
    flag("above_4gb", (unsigned long long)base > 0xFFFFFFFFull); flag("aligned_64k", ((unsigned long long)base & 0xFFFF) == 0);
    end();

    static WCHAR exe_path[600], dll_path[600];
    DWORD el = GetModuleFileNameW(NULL, exe_path, 600), dl = GetModuleFileNameW(dll, dll_path, 600);
    DWORD exe_dir = el, dll_dir = dl;
    while (exe_dir && exe_path[exe_dir - 1] != '\\') exe_dir--;
    while (dll_dir && dll_path[dll_dir - 1] != '\\') dll_dir--;
    int same_dir = exe_dir == dll_dir;
    for (DWORD i = 0; same_dir && i < exe_dir; i++) same_dir = exe_path[i] == dll_path[i];
    row("dll.file_name"); flag("in_exe_directory", same_dir); put(",\"leaf\":"); wide_item(dll_path + dll_dir, dl - dll_dir); end();

    SetLastError(SENTINEL);
    FARPROC p = GetProcAddress(dll, "loader_dll_count");
    DWORD err = GetLastError();
    row("dll.export"); flag("equals_import", p == (FARPROC)loader_dll_count); field("err", err);
    SetLastError(SENTINEL);
    p = GetProcAddress(dll, "no_such_export");
    err = GetLastError();
    flag("missing_null", p == 0); field("missing_err", err); end();

    unsigned a, b, c;
    ldr_list("ldr.load_order", 0x10, 0x00, &a);
    ldr_list("ldr.memory_order", 0x20, 0x10, &b);
    ldr_list("ldr.init_order", 0x30, 0x20, &c);
    unsigned char *ldr = *(unsigned char **)(peb() + 0x18);
    LIST_ENTRY *first = ((LIST_ENTRY *)(ldr + 0x10))->Flink;
    row("ldr.first_is_exe"); flag("value", *(void **)((unsigned char *)first + 0x30) == (void *)GetModuleHandleW(NULL)); end();

    DWORD written;
    WriteFile(GetStdHandle(STD_OUTPUT_HANDLE), out, n, &written, NULL);
    ExitProcess(0);
}
