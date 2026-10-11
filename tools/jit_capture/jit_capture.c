// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* jit_capture.c — start a Unity Mono game with hooks that record the code Mono generates
 * (Sprint 033).
 *
 * Built as "<game>.exe" in the game folder, so Unity finds "<game>_Data" by the executable name.
 * Before Unity starts, it loads the game's own MonoBleedingEdge/EmbedRuntime/mono-2.0-bdwgc.dll
 * by full path (UnityPlayer.dll later loads the same module) and registers two hooks
 * (Unity-Technologies/mono, branch unity-2019.4-mbe, mono/metadata/profiler.c and
 * profiler-events.h):
 *   jit_done (MonoProfiler *, MonoMethod *, MonoJitInfo *) — each compiled method;
 *   jit_code_buffer (MonoProfiler *, const mono_byte *, uint64_t, MonoProfilerCodeBufferType,
 *     const void *) — trampolines, IMT, generic, delegate and exception stubs, which jit_done
 *     does not report (profiler.h, MonoProfilerCodeBufferType).
 * Then it calls UnityPlayer.dll's UnityMain with its own wWinMain arguments; the game's own
 * executable is a single jmp to UnityMain (0x140001000 in build 24247624).
 *
 * Capture file (path in env W2M_JIT_CAPTURE), little-endian. Header: "W2MJ", u32 format 2.
 * Records, each starting with u32 kind:
 *   kind 0 CPUID:  u32 count, then count x (u32 leaf, u32 subleaf, u32 eax, ebx, ecx, edx).
 *   kind 1 method: u64 code_start, u32 code_size, u32 name_len, u32 image_len, name, image,
 *                  code bytes. name is mono_method_full_name; image is the assembly image name
 *                  (mono_method_get_class -> mono_class_get_image -> mono_image_get_name).
 *   kind 2 buffer: u64 code_start, u32 code_size, u32 buffer_type, u32 desc_len, desc, code bytes.
 *                  desc is the method's full name for METHOD-typed buffers, the C string for
 *                  SPECIFIC_TRAMPOLINE, empty otherwise.
 * Bytes are copied when the hook fires, before Mono patches call sites in them.
 * WriteFile goes straight to the OS, so whole records survive the process being stopped. A
 * failed or short write stops all later writes, so the file stays a valid prefix, and writes
 * "<capture>.error" with the Windows error code.
 */
#include <windows.h>
#include <intrin.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

typedef void *(*profiler_create_fn)(void *prof);
typedef void (*jit_done_cb)(void *prof, void *method, void *jinfo);
typedef void (*code_buffer_cb)(void *prof, const uint8_t *buf, uint64_t size, int type, const void *data);
typedef void (*set_jit_done_fn)(void *handle, jit_done_cb cb);
typedef void (*set_code_buffer_fn)(void *handle, code_buffer_cb cb);
typedef void *(*code_start_fn)(void *jinfo);
typedef int (*code_size_fn)(void *jinfo);
typedef char *(*full_name_fn)(void *method, int signature);
typedef void *(*method_class_fn)(void *method);
typedef void *(*class_image_fn)(void *klass);
typedef const char *(*image_name_fn)(void *image);
typedef void (*mono_free_fn)(void *p);
typedef int (*unity_main_fn)(HINSTANCE, HINSTANCE, LPWSTR, int);

enum { KIND_CPUID = 0, KIND_METHOD = 1, KIND_BUFFER = 2 };
enum { BUFFER_METHOD = 0, BUFFER_SPECIFIC_TRAMPOLINE = 5 };

static HANDLE out = INVALID_HANDLE_VALUE;
static WCHAR error_path[MAX_PATH];
static volatile LONG broken;
static CRITICAL_SECTION lock;
static code_start_fn code_start;
static code_size_fn code_size;
static full_name_fn full_name;
static method_class_fn method_class;
static class_image_fn class_image;
static image_name_fn image_name;
static mono_free_fn mono_free_p;

/* Called with the lock held. Returns 0 once any write has failed. */
static int put(const void *p, DWORD n) {
    DWORD w = 0;
    if (broken) return 0;
    if (n == 0) return 1;
    if (!WriteFile(out, p, n, &w, NULL) || w != n) {
        DWORD code = GetLastError();
        broken = 1;
        HANDLE e = CreateFileW(error_path, GENERIC_WRITE, 0, NULL, CREATE_ALWAYS, FILE_ATTRIBUTE_NORMAL, NULL);
        if (e != INVALID_HANDLE_VALUE) {
            DWORD x;
            WriteFile(e, &code, 4, &x, NULL);
            WriteFile(e, &w, 4, &x, NULL);
            CloseHandle(e);
        }
        return 0;
    }
    return 1;
}

static void on_jit_done(void *prof, void *method, void *jinfo) {
    (void)prof;
    if (!jinfo || broken) return;
    uint32_t kind = KIND_METHOD;
    uint64_t start = (uint64_t)(uintptr_t)code_start(jinfo);
    uint32_t size = (uint32_t)code_size(jinfo);
    char *name = full_name(method, 1);
    uint32_t name_len = name ? (uint32_t)lstrlenA(name) : 0;
    void *klass = method_class(method);
    const char *image = klass ? image_name(class_image(klass)) : NULL;
    uint32_t image_len = image ? (uint32_t)lstrlenA(image) : 0;
    EnterCriticalSection(&lock);
    put(&kind, 4) && put(&start, 8) && put(&size, 4) && put(&name_len, 4) && put(&image_len, 4) &&
        put(name, name_len) && put(image, image_len) && put((void *)(uintptr_t)start, size);
    LeaveCriticalSection(&lock);
    if (name) mono_free_p(name);
}

static void on_code_buffer(void *prof, const uint8_t *buf, uint64_t size64, int type, const void *data) {
    (void)prof;
    if (!buf || broken) return;
    uint32_t kind = KIND_BUFFER, size = (uint32_t)size64, t = (uint32_t)type, desc_len = 0;
    uint64_t start = (uint64_t)(uintptr_t)buf;
    char *owned = NULL;
    const char *desc = NULL;
    if (type == BUFFER_METHOD && data) desc = owned = full_name((void *)data, 1);
    else if (type == BUFFER_SPECIFIC_TRAMPOLINE && data) desc = (const char *)data;
    if (desc) desc_len = (uint32_t)lstrlenA(desc);
    EnterCriticalSection(&lock);
    put(&kind, 4) && put(&start, 8) && put(&size, 4) && put(&t, 4) && put(&desc_len, 4) &&
        put(desc, desc_len) && put(buf, size);
    LeaveCriticalSection(&lock);
    if (owned) mono_free_p(owned);
}

/* CPUID leaves that decide which instructions Mono's JIT selects (mini-amd64.c, cpu features). */
static void put_cpuid(void) {
    static const uint32_t leaves[][2] = {{0, 0}, {1, 0}, {7, 0}, {7, 1}, {0xD, 1}, {0x80000001u, 0}};
    uint32_t kind = KIND_CPUID, count = sizeof leaves / sizeof leaves[0];
    put(&kind, 4);
    put(&count, 4);
    for (uint32_t i = 0; i < count; i++) {
        int r[4];
        __cpuidex(r, (int)leaves[i][0], (int)leaves[i][1]);
        put(leaves[i], 8);
        put(r, 16);
    }
}


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
typedef struct { volatile LONG64 count; void *original; void *via; volatile LONG traced; char name[96]; } hook_t;
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
            void *t = add_hook(name, fn, (void *)(ULONG_PTR)slots->u1.Function);
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

static void fail(const char *what) {
    HANDLE e = GetStdHandle(STD_ERROR_HANDLE);
    DWORD w;
    WriteFile(e, what, lstrlenA(what), &w, NULL);
    ExitProcess(2);
}

int WINAPI wWinMain(HINSTANCE inst, HINSTANCE prev, LPWSTR cmd, int show) {
    WCHAR dir[MAX_PATH], path[MAX_PATH], cap[MAX_PATH];
    DWORD n = GetModuleFileNameW(NULL, dir, MAX_PATH);
    while (n && dir[n - 1] != L'\\') n--;
    dir[n] = 0;
    lstrcpyW(path, dir);
    lstrcatW(path, L"MonoBleedingEdge\\EmbedRuntime\\mono-2.0-bdwgc.dll");
    HMODULE mono = LoadLibraryW(path);
    if (!mono) fail("jit_capture: cannot load mono-2.0-bdwgc.dll\n");
    if (!GetEnvironmentVariableW(L"W2M_JIT_CAPTURE", cap, MAX_PATH - 8)) fail("jit_capture: W2M_JIT_CAPTURE unset\n");
    lstrcpyW(error_path, cap);
    lstrcatW(error_path, L".error");
    out = CreateFileW(cap, GENERIC_WRITE, FILE_SHARE_READ, NULL, CREATE_ALWAYS, FILE_ATTRIBUTE_NORMAL, NULL);
    if (out == INVALID_HANDLE_VALUE) fail("jit_capture: cannot create capture file\n");
    InitializeCriticalSection(&lock);
    profiler_create_fn create = (profiler_create_fn)GetProcAddress(mono, "mono_profiler_create");
    set_jit_done_fn set_done = (set_jit_done_fn)GetProcAddress(mono, "mono_profiler_set_jit_done_callback");
    set_code_buffer_fn set_buffer = (set_code_buffer_fn)GetProcAddress(mono, "mono_profiler_set_jit_code_buffer_callback");
    code_start = (code_start_fn)GetProcAddress(mono, "mono_jit_info_get_code_start");
    code_size = (code_size_fn)GetProcAddress(mono, "mono_jit_info_get_code_size");
    full_name = (full_name_fn)GetProcAddress(mono, "mono_method_full_name");
    method_class = (method_class_fn)GetProcAddress(mono, "mono_method_get_class");
    class_image = (class_image_fn)GetProcAddress(mono, "mono_class_get_image");
    image_name = (image_name_fn)GetProcAddress(mono, "mono_image_get_name");
    mono_free_p = (mono_free_fn)GetProcAddress(mono, "mono_free");
    if (!create || !set_done || !set_buffer || !code_start || !code_size || !full_name ||
        !method_class || !class_image || !image_name || !mono_free_p)
        fail("jit_capture: a Mono export is missing\n");
    static const char magic[4] = {'W', '2', 'M', 'J'};
    uint32_t format = 2;
    if (!(put(magic, 4) && put(&format, 4))) fail("jit_capture: cannot write capture header\n");
    put_cpuid();
    void *handle = create(NULL);
    set_done(handle, on_jit_done);
    set_buffer(handle, on_code_buffer);
    lstrcpyW(path, dir);
    lstrcatW(path, L"UnityPlayer.dll");
    HMODULE unity = LoadLibraryW(path);
    if (!unity) fail("jit_capture: cannot load UnityPlayer.dll\n");
    InitializeCriticalSection(&dyn_lock);
    /* one allocation: each thunk reaches its counter and target through a rip-relative rel32 */
    thunks = VirtualAlloc(NULL, THUNK_SIZE * MAX_HOOKS + sizeof(hook_t) * MAX_HOOKS, MEM_COMMIT | MEM_RESERVE,
                          PAGE_EXECUTE_READWRITE);
    if (!thunks) fail("jit_capture: cannot allocate hooks\n");
    hooks = (hook_t *)(thunks + THUNK_SIZE * MAX_HOOKS);
    calls = VirtualAlloc(NULL, sizeof(call_t) * MAX_HOOKS * TRACE_CALLS, MEM_COMMIT | MEM_RESERVE, PAGE_READWRITE);
    if (!calls) fail("jit_capture: cannot allocate the trace\n");
    DWORD slot = TlsAlloc();
    if (slot == TLS_OUT_OF_INDEXES || slot >= 64) fail("jit_capture: no TLS slot below 64\n");
    slot_offset = 0x1480 + 8 * slot;
    system_dir_len = (int)GetSystemDirectoryW(system_dir, MAX_PATH);
    if (!system_dir_len) fail("jit_capture: no system directory\n");
    system_dir[system_dir_len++] = L'\\';
    system_dir[system_dir_len] = 0;
    lstrcpyW(log_base, cap);
    hook_image(unity, "UnityPlayer.dll");
    hook_image(mono, "mono-2.0-bdwgc.dll");
    CreateThread(NULL, 0, monitor, NULL, 0, NULL);
    unity_main_fn unity_main = (unity_main_fn)GetProcAddress(unity, "UnityMain");
    if (!unity_main) fail("jit_capture: UnityMain missing\n");
    return unity_main(inst, prev, cmd, show);
}
