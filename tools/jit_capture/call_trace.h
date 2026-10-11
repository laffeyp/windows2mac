// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* call_trace.h — the import-call log and call trace, shared by jit_capture.exe (the game) and handler_trace.dll
 * (UnityCrashHandler64.exe; Sprint 055). Included once by each; the includer calls trace_init, hooks its images with
 * hook_image, and may set import_override to put its own function under an import's traced thunk.
 */
/* ---- import-call log (Sprint 038) -----------------------------------------------------------
 * Before Unity starts, every IAT slot of UnityPlayer.dll and mono-2.0-bdwgc.dll is pointed at a
 * thunk `lock inc qword [counter]; jmp qword [original]` (F0 48 FF 05 rel32; FF 25 rel32), so each
 * call to a Windows function through those images is counted. Their GetProcAddress slots go to
 * log_get_proc_address instead, which records each name resolved at run time (d3d11, dxgi, XInput
 * are reached that way, not through imports). A monitor thread rewrites <capture>.imports.tsv,
 * <capture>.dynamic.tsv and <capture>.threads.txt every 5 s (peak and distinct thread counts from
 * Toolhelp snapshots each second), so the data survives the process being stopped.
 * Images loaded later by Unity (plugins, steam_api64) are not hooked; the log says so.
 *
 * The call trace (Sprint 049): each thunk also stores its hook in a TLS slot of the TEB and jumps to
 * `traced`, a C function that calls the original with sixteen integer arguments (the four registers and
 * twelve stack slots, so a callee finds its own stack arguments where its caller put them) and, for the
 * first TRACE_CALLS calls of each function, records the call's sequence number (taken on entry, so
 * nested calls follow their caller), thread id, the four register and four stack arguments, the return
 * value and the last error after the call; it restores that last error before returning, so the logging
 * never shows. Being C, `traced` has unwind data, so exceptions and stack walks pass through it.
 * Functions that read or unwind their caller's context, or never return, keep a plain jump
 * (UNTRACED). Floating-point arguments pass in XMM registers `traced` does not touch before its call; a
 * floating-point return is not recorded. Names a Windows DLL answers through GetProcAddress get a traced
 * thunk too, named "<dll>!<name>". <capture>.trace.tsv is rewritten with the others: one line per call,
 * seq, tid, image, dll, function, then a0..a7, ret and error in hex. */
#include <tlhelp32.h>

#define MAX_HOOKS 4096
#define THUNK_SIZE 48
#define TRACE_CALLS 32
typedef struct { volatile LONG64 count; void *original; void *via; volatile LONG traced; char name[96];
                 unsigned wide, ansi; } hook_t;   /* wide, ansi: masks of arguments that are UTF-16 or ANSI strings */
typedef struct { volatile LONG ready; DWORD tid; LONG64 seq; int hook; DWORD err; uint64_t a[8]; uint64_t ret; } call_t;
typedef uint64_t (*fn16)(uint64_t, uint64_t, uint64_t, uint64_t, uint64_t, uint64_t, uint64_t, uint64_t,
                         uint64_t, uint64_t, uint64_t, uint64_t, uint64_t, uint64_t, uint64_t, uint64_t);
static hook_t *hooks;
static unsigned char *thunks;
static volatile LONG nhooks_live;
static int nhooks;
static call_t *calls;
static volatile LONG ncalls;
static volatile LONG64 sequence;
static DWORD slot_offset;                     /* TEB TlsSlots[i]: gs:[0x1480 + 8 i] */
static const char *const UNTRACED[] = {"RaiseException", "RtlUnwindEx", "RtlUnwind", "RtlCaptureContext",
    "RtlVirtualUnwind", "RtlLookupFunctionEntry", "RtlRestoreContext", "RtlCaptureStackBackTrace",
    "CaptureStackBackTrace", "ExitProcess", "ExitThread", "FreeLibraryAndExitThread", "TerminateProcess",
    "GetProcAddress", "SetUnhandledExceptionFilter", "UnhandledExceptionFilter", NULL};

/* Arguments that name something, recorded as text (Sprint 055): bit i of `wide` is a UTF-16 string argument i,
 * of `ansi` an ANSI one. An argument below 0x10000 is an atom or an id, not a string. */
static const struct { const char *fn; unsigned wide, ansi; } STRING_ARGS[] = {
    {"CreateEventW", 1u << 3, 0}, {"CreateEventA", 0, 1u << 3}, {"CreateEventExW", 1u << 1, 0},
    {"OpenEventW", 1u << 2, 0}, {"CreateMutexW", 1u << 2, 0}, {"CreateSemaphoreExW", 1u << 3, 0},
    {"CreateProcessW", 1u << 0 | 1u << 1 | 1u << 7, 0}, {"CreateProcessA", 0, 1u << 0 | 1u << 1 | 1u << 7},
    {"CreateFileW", 1u << 0, 0}, {"CreateFileA", 0, 1u << 0}, {"DeleteFileW", 1u << 0, 0},
    {"CreateDirectoryW", 1u << 0, 0}, {"RemoveDirectoryW", 1u << 0, 0}, {"GetFileAttributesExW", 1u << 0, 0},
    {"GetFileAttributesW", 1u << 0, 0}, {"GetFileAttributesA", 0, 1u << 0}, {"FindFirstFileExW", 1u << 0, 0},
    {"FindFirstFileW", 1u << 0, 0}, {"CopyFileExW", 1u << 0 | 1u << 1, 0}, {"MoveFileExW", 1u << 0 | 1u << 1, 0},
    {"SetCurrentDirectoryW", 1u << 0, 0}, {"GetFullPathNameW", 1u << 0, 0}, {"SHCreateDirectoryExW", 1u << 1, 0},
    {"LoadLibraryW", 1u << 0, 0}, {"LoadLibraryExW", 1u << 0, 0}, {"LoadLibraryA", 0, 1u << 0},
    {"GetModuleHandleW", 1u << 0, 0}, {"GetModuleHandleA", 0, 1u << 0},
    {"RegOpenKeyExW", 1u << 1, 0}, {"RegCreateKeyW", 1u << 1, 0}, {"RegSetValueExA", 0, 1u << 1},
    {"RegQueryValueExA", 0, 1u << 1}, {"RegQueryValueExW", 1u << 1, 0}, {"RegDeleteValueA", 0, 1u << 1},
    {"CreateWindowExW", 1u << 1 | 1u << 2, 0}, {"SetWindowTextW", 1u << 1, 0},
    {"OutputDebugStringA", 0, 1u << 0}, {"OutputDebugStringW", 1u << 0, 0},
    {"GetEnvironmentVariableW", 1u << 0, 0}, {"GetEnvironmentVariableA", 0, 1u << 0},
    {"SetEnvironmentVariableW", 1u << 0 | 1u << 1, 0}, {"CreateFileMappingW", 1u << 5, 0},
    {"OpenFileMappingW", 1u << 2, 0}, {"CreateNamedPipeW", 1u << 0, 0}, {"WaitNamedPipeW", 1u << 0, 0},
    {NULL, 0, 0}};

#define STRINGS 65536
#define STRING_CHARS 240
typedef struct { volatile LONG ready; int arg; LONG64 seq; char text[STRING_CHARS]; } string_t;
static string_t *strings;
static volatile LONG nstrings;

/* One string argument, as UTF-8 for ASCII and \uXXXX for the rest, cut at STRING_CHARS - 8 bytes. */
static void record_string(LONG64 seq, int arg, uint64_t p, int wide) {
    LONG k = InterlockedIncrement(&nstrings) - 1;
    if (k >= STRINGS) return;
    string_t *s = &strings[k];
    int n = 0;
    for (int i = 0; n < STRING_CHARS - 8; i++) {
        unsigned c = wide ? ((const unsigned short *)(uintptr_t)p)[i] : ((const unsigned char *)(uintptr_t)p)[i];
        if (!c) break;
        if (c >= 0x20 && c < 0x7F) s->text[n++] = (char)c;
        else n += snprintf(s->text + n, STRING_CHARS - n, "\\u%04x", c);
    }
    s->text[n] = 0;
    s->seq = seq;
    s->arg = arg;
    InterlockedExchange(&s->ready, 1);
}

static int untraced(const char *fn) {
    for (int i = 0; UNTRACED[i]; i++) if (!lstrcmpA(UNTRACED[i], fn)) return 1;
    return 0;
}

/* TEB.LastErrorValue (gs:[0x68]) written directly: clang 20 for x86_64-pc-windows-msvc declares __writegsdword
 * but links no definition (run 38113277206); volatile asm also keeps the call before it from becoming a tail call. */
static void set_teb_error(DWORD e) { __asm__ volatile("movl %0, %%gs:0x68" : : "r"(e) : "memory"); }

static uint64_t traced(uint64_t a0, uint64_t a1, uint64_t a2, uint64_t a3, uint64_t s4, uint64_t s5, uint64_t s6,
                       uint64_t s7, uint64_t s8, uint64_t s9, uint64_t s10, uint64_t s11, uint64_t s12, uint64_t s13,
                       uint64_t s14, uint64_t s15) {
    hook_t *h = (hook_t *)__readgsqword(slot_offset);
    fn16 f = (fn16)h->original;
    if (InterlockedIncrement(&h->traced) > TRACE_CALLS) {
        uint64_t r = f(a0, a1, a2, a3, s4, s5, s6, s7, s8, s9, s10, s11, s12, s13, s14, s15);
        set_teb_error(__readgsdword(0x68));               /* no tail call: the stack slots stay ours */
        return r;
    }
    LONG64 seq = InterlockedIncrement64(&sequence);
    if (h->wide | h->ansi) {
        uint64_t args[8] = {a0, a1, a2, a3, s4, s5, s6, s7};
        for (int i = 0; i < 8; i++)
            if (((h->wide | h->ansi) >> i & 1) && args[i] > 0xFFFF) record_string(seq, i, args[i], h->wide >> i & 1);
    }
    uint64_t r = f(a0, a1, a2, a3, s4, s5, s6, s7, s8, s9, s10, s11, s12, s13, s14, s15);
    DWORD err = __readgsdword(0x68);                       /* TEB.LastErrorValue */
    LONG k = InterlockedIncrement(&ncalls) - 1;
    if (k < MAX_HOOKS * TRACE_CALLS) {
        call_t *c = &calls[k];
        c->tid = __readgsdword(0x48);                      /* TEB.ClientId.UniqueThread */
        c->seq = seq;
        c->hook = (int)(h - hooks);
        c->err = err;
        c->a[0] = a0; c->a[1] = a1; c->a[2] = a2; c->a[3] = a3;
        c->a[4] = s4; c->a[5] = s5; c->a[6] = s6; c->a[7] = s7;
        c->ret = r;
        InterlockedExchange(&c->ready, 1);
    }
    set_teb_error(err);
    return r;
}

/* A thunk for hook h: lock inc [count]; mov rax, h; mov gs:[slot], rax; jmp [via]. */
static void *make_thunk(hook_t *h, int i) {
    unsigned char *t = thunks + THUNK_SIZE * i;
    LONG cdisp = (LONG)((unsigned char *)&h->count - (t + 8));
    t[0] = 0xF0; t[1] = 0x48; t[2] = 0xFF; t[3] = 0x05; memcpy(t + 4, &cdisp, 4);
    t[8] = 0x48; t[9] = 0xB8; uint64_t hp = (uint64_t)(uintptr_t)h; memcpy(t + 10, &hp, 8);
    t[18] = 0x65; t[19] = 0x48; t[20] = 0x89; t[21] = 0x04; t[22] = 0x25; memcpy(t + 23, &slot_offset, 4);
    LONG vdisp = (LONG)((unsigned char *)&h->via - (t + 33));
    t[27] = 0xFF; t[28] = 0x25; memcpy(t + 29, &vdisp, 4);
    return t;
}

/* A new hook named `name` over `original`; traced unless `fn` is UNTRACED. Caller holds dyn_lock or runs alone. */
static void *add_hook(const char *name, const char *fn, void *original) {
    if (nhooks >= MAX_HOOKS) return original;
    hook_t *h = &hooks[nhooks];
    lstrcpynA(h->name, name, sizeof h->name);
    h->original = original;
    h->via = untraced(fn) ? original : (void *)traced;
    h->wide = h->ansi = 0;
    for (int i = 0; STRING_ARGS[i].fn; i++)
        if (!lstrcmpA(STRING_ARGS[i].fn, fn)) { h->wide = STRING_ARGS[i].wide; h->ansi = STRING_ARGS[i].ansi; }
    void *t = make_thunk(h, nhooks);
    nhooks++;
    InterlockedExchange(&nhooks_live, nhooks);
    return t;
}
static CRITICAL_SECTION dyn_lock;
static char dynamic_names[2048][96];
static int ndynamic;
static FARPROC (WINAPI *real_get_proc_address)(HMODULE, LPCSTR);
static WCHAR log_base[MAX_PATH];
static int peak_threads;
static DWORD seen_tids[16384];
static int nseen;

static WCHAR system_dir[MAX_PATH];
static int system_dir_len;

/* Whether module m lives in the system directory (a Windows DLL, not the game's). */
static int windows_module(HMODULE m, char *stem, int cap) {
    WCHAR path[MAX_PATH];
    DWORD n = GetModuleFileNameW(m, path, MAX_PATH);
    if (!n || n <= (DWORD)system_dir_len || CompareStringOrdinal(path, system_dir_len, system_dir, system_dir_len, TRUE) != CSTR_EQUAL)
        return 0;
    const WCHAR *base = path + n;
    while (base > path && base[-1] != L'\\') base--;
    int k = 0;
    for (; base[k] && k < cap - 1; k++) stem[k] = (char)base[k];
    stem[k] = 0;
    return 1;
}

static FARPROC WINAPI log_get_proc_address(HMODULE m, LPCSTR name) {
    FARPROC p = real_get_proc_address(m, name);
    if ((ULONG_PTR)name > 0xFFFF) {
        DWORD err = GetLastError();
        char dll[64], full[96];
        int windows = p && windows_module(m, dll, sizeof dll);
        EnterCriticalSection(&dyn_lock);
        int known = 0;
        for (int i = 0; i < ndynamic && !known; i++) known = !lstrcmpA(dynamic_names[i], name);
        if (!known && ndynamic < 2048) lstrcpynA(dynamic_names[ndynamic++], name, 96);
        if (windows) {
            snprintf(full, sizeof full, "GetProcAddress\t%s\t%s", dll, name);
            void *t = NULL;
            for (int i = 0; i < nhooks && !t; i++)
                if (!lstrcmpA(hooks[i].name, full)) t = thunks + THUNK_SIZE * i;
            p = (FARPROC)(t ? t : add_hook(full, name, (void *)p));
        }
        LeaveCriticalSection(&dyn_lock);
        SetLastError(err);
    }
    return p;
}

/* Set by the includer: its own function to put under an import's thunk (the original passed in), or NULL. */
static void *(*import_override)(const char *fn, void *original);

static void hook_image(HMODULE image, const char *label) {
    unsigned char *base = (unsigned char *)image;
    IMAGE_NT_HEADERS64 *nt = (IMAGE_NT_HEADERS64 *)(base + ((IMAGE_DOS_HEADER *)base)->e_lfanew);
    IMAGE_DATA_DIRECTORY dir = nt->OptionalHeader.DataDirectory[IMAGE_DIRECTORY_ENTRY_IMPORT];
    if (!dir.VirtualAddress) return;
    for (IMAGE_IMPORT_DESCRIPTOR *d = (IMAGE_IMPORT_DESCRIPTOR *)(base + dir.VirtualAddress); d->Name; d++) {
        const char *dll = (const char *)(base + d->Name);
        IMAGE_THUNK_DATA64 *names = (IMAGE_THUNK_DATA64 *)(base + (d->OriginalFirstThunk ? d->OriginalFirstThunk : d->FirstThunk));
        IMAGE_THUNK_DATA64 *slots = (IMAGE_THUNK_DATA64 *)(base + d->FirstThunk);
        for (; names->u1.AddressOfData; names++, slots++) {
            char fn[64];
            if (IMAGE_SNAP_BY_ORDINAL64(names->u1.Ordinal)) snprintf(fn, sizeof fn, "#%u", (unsigned)IMAGE_ORDINAL64(names->u1.Ordinal));
            else lstrcpynA(fn, ((IMAGE_IMPORT_BY_NAME *)(base + names->u1.AddressOfData))->Name, 64);
            DWORD old;
            VirtualProtect(&slots->u1.Function, 8, PAGE_READWRITE, &old);
            if (!lstrcmpA(fn, "GetProcAddress")) {
                real_get_proc_address = (FARPROC (WINAPI *)(HMODULE, LPCSTR))(ULONG_PTR)slots->u1.Function;
                slots->u1.Function = (ULONGLONG)(ULONG_PTR)log_get_proc_address;
            }
            char name[96];
            snprintf(name, sizeof name, "%s\t%s\t%s", label, dll, fn);
            void *original = (void *)(ULONG_PTR)slots->u1.Function;
            void *own = import_override ? import_override(fn, original) : NULL;
            void *t = add_hook(name, fn, own ? own : original);
            slots->u1.Function = (ULONGLONG)(ULONG_PTR)t;
            VirtualProtect(&slots->u1.Function, 8, old, &old);
        }
    }
}

static void write_logs(void) {
    WCHAR path[MAX_PATH];
    char line[256];
    DWORD w;
    lstrcpyW(path, log_base); lstrcatW(path, L".imports.tsv");
    HANDLE f = CreateFileW(path, GENERIC_WRITE, FILE_SHARE_READ, NULL, CREATE_ALWAYS, 0, NULL);
    if (f != INVALID_HANDLE_VALUE) {
        int live = nhooks_live;
        for (int i = 0; i < live; i++) {
            int n = snprintf(line, sizeof line, "%s\t%lld\n", hooks[i].name, (long long)hooks[i].count);
            WriteFile(f, line, n, &w, NULL);
        }
        CloseHandle(f);
    }
    lstrcpyW(path, log_base); lstrcatW(path, L".dynamic.tsv");
    f = CreateFileW(path, GENERIC_WRITE, FILE_SHARE_READ, NULL, CREATE_ALWAYS, 0, NULL);
    if (f != INVALID_HANDLE_VALUE) {
        EnterCriticalSection(&dyn_lock);
        for (int i = 0; i < ndynamic; i++) {
            int n = snprintf(line, sizeof line, "%s\n", dynamic_names[i]);
            WriteFile(f, line, n, &w, NULL);
        }
        LeaveCriticalSection(&dyn_lock);
        CloseHandle(f);
    }
    lstrcpyW(path, log_base); lstrcatW(path, L".trace.tsv");
    f = CreateFileW(path, GENERIC_WRITE, FILE_SHARE_READ, NULL, CREATE_ALWAYS, 0, NULL);
    if (f != INVALID_HANDLE_VALUE) {
        LONG total = ncalls;
        if (total > MAX_HOOKS * TRACE_CALLS) total = MAX_HOOKS * TRACE_CALLS;
        char big[512];
        for (LONG k = 0; k < total; k++) {
            call_t *c = &calls[k];
            if (!c->ready) continue;
            int n = snprintf(big, sizeof big, "%lld\t%lu\t%s", (long long)c->seq, (unsigned long)c->tid, hooks[c->hook].name);
            for (int j = 0; j < 8; j++) n += snprintf(big + n, sizeof big - n, "\t%llx", (unsigned long long)c->a[j]);
            n += snprintf(big + n, sizeof big - n, "\t%llx\t%lu\n", (unsigned long long)c->ret, (unsigned long)c->err);
            WriteFile(f, big, n, &w, NULL);
        }
        CloseHandle(f);
    }
    lstrcpyW(path, log_base); lstrcatW(path, L".strings.tsv");
    f = CreateFileW(path, GENERIC_WRITE, FILE_SHARE_READ, NULL, CREATE_ALWAYS, 0, NULL);
    if (f != INVALID_HANDLE_VALUE) {
        LONG total = nstrings;
        if (total > STRINGS) total = STRINGS;
        char big[STRING_CHARS + 64];
        for (LONG k = 0; k < total; k++) {
            string_t *s = &strings[k];
            if (!s->ready) continue;
            int n = snprintf(big, sizeof big, "%lld\t%d\t%s\n", (long long)s->seq, s->arg, s->text);
            WriteFile(f, big, n, &w, NULL);
        }
        CloseHandle(f);
    }
    lstrcpyW(path, log_base); lstrcatW(path, L".threads.txt");
    f = CreateFileW(path, GENERIC_WRITE, FILE_SHARE_READ, NULL, CREATE_ALWAYS, 0, NULL);
    if (f != INVALID_HANDLE_VALUE) {
        int n = snprintf(line, sizeof line, "peak %d\ndistinct %d\nhooked_imports %d\ntraced_calls %ld\n", peak_threads, nseen,
                         (int)nhooks_live, (long)ncalls);
        WriteFile(f, line, n, &w, NULL);
        CloseHandle(f);
    }
}

static DWORD WINAPI monitor(LPVOID p) {
    (void)p;
    DWORD pid = GetCurrentProcessId();
    for (int tick = 0;; tick++) {
        HANDLE snap = CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0);
        if (snap != INVALID_HANDLE_VALUE) {
            THREADENTRY32 te;
            te.dwSize = sizeof te;
            int now = 0;
            for (BOOL ok = Thread32First(snap, &te); ok; ok = Thread32Next(snap, &te)) {
                if (te.th32OwnerProcessID != pid) continue;
                now++;
                int known = 0;
                for (int i = 0; i < nseen && !known; i++) known = seen_tids[i] == te.th32ThreadID;
                if (!known && nseen < 16384) seen_tids[nseen++] = te.th32ThreadID;
            }
            CloseHandle(snap);
            if (now > peak_threads) peak_threads = now;
        }
        if (tick % 5 == 4) write_logs();
        Sleep(1000);
    }
    return 0;
}

/* Allocations, the TLS slot and the log base for the files above; 0 and a message on failure. */
static const char *trace_init(const WCHAR *base) {
    InitializeCriticalSection(&dyn_lock);
    /* one allocation: each thunk reaches its counter and target through a rip-relative rel32 */
    thunks = VirtualAlloc(NULL, THUNK_SIZE * MAX_HOOKS + sizeof(hook_t) * MAX_HOOKS, MEM_COMMIT | MEM_RESERVE,
                          PAGE_EXECUTE_READWRITE);
    if (!thunks) return "cannot allocate hooks";
    hooks = (hook_t *)(thunks + THUNK_SIZE * MAX_HOOKS);
    calls = VirtualAlloc(NULL, sizeof(call_t) * MAX_HOOKS * TRACE_CALLS, MEM_COMMIT | MEM_RESERVE, PAGE_READWRITE);
    strings = VirtualAlloc(NULL, sizeof(string_t) * STRINGS, MEM_COMMIT | MEM_RESERVE, PAGE_READWRITE);
    if (!calls || !strings) return "cannot allocate the trace";
    DWORD slot = TlsAlloc();
    if (slot == TLS_OUT_OF_INDEXES || slot >= 64) return "no TLS slot below 64";
    slot_offset = 0x1480 + 8 * slot;
    system_dir_len = (int)GetSystemDirectoryW(system_dir, MAX_PATH);
    if (!system_dir_len) return "no system directory";
    system_dir[system_dir_len++] = L'\\';
    system_dir[system_dir_len] = 0;
    lstrcpyW(log_base, base);
    return 0;
}
