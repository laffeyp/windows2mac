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
 * <capture>.activation.tsv (Sprint 065): a WH_CALLWNDPROC hook on the main thread, installed before UnityMain, writes
 * one line for each WM_WINDOWPOSCHANGING, WM_ACTIVATEAPP, WM_NCACTIVATE, WM_ACTIVATE, WM_SETFOCUS and WM_KILLFOCUS
 * Windows sends: the message, wParam, lParam, the window, the foreground window, its thread and process, its class,
 * and this thread and process; a first line "start" holds the same foreground fields and
 * SPI_GETFOREGROUNDLOCKTIMEOUT. Game run 38216081440 has WM_ACTIVATEAPP's lParam 0 and WM_NCACTIVATE's wParam 0
 * where seventeen ways of starting a probe give a thread and 1 (probe activation): the lines say what Windows sent.
 * WriteFile goes straight to the OS, so whole records survive the process being stopped. A
 * failed or short write stops all later writes, so the file stays a valid prefix, and writes
 * "<capture>.error" with the Windows error code.
 */
#include <windows.h>
#pragma comment(lib, "user32.lib")      /* the activation hook (Sprint 065) */
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


#include "call_trace.h"

/* UnityPlayer starts UnityCrashHandler64.exe (Sprint 055): the child starts suspended, with LoadLibraryW of
 * handler_trace.dll (beside this exe) queued as an APC on its main thread, which runs once the loader has bound the
 * handler's imports and before its entry point; then it resumes, unless the caller asked for it suspended. The
 * child inherits W2M_JIT_CAPTURE and writes its files under "<capture>.handler". */
typedef BOOL (WINAPI *create_process_fn)(LPCWSTR, LPWSTR, LPSECURITY_ATTRIBUTES, LPSECURITY_ATTRIBUTES, BOOL, DWORD,
                                         LPVOID, LPCWSTR, LPSTARTUPINFOW, LPPROCESS_INFORMATION);
static create_process_fn real_create_process;
static WCHAR handler_dll[MAX_PATH];

static int names_handler(LPCWSTR app, LPWSTR cmd) {
    const WCHAR *s = app ? app : cmd;
    if (!s) return 0;
    static const WCHAR needle[] = L"UnityCrashHandler64.exe";
    for (; *s; s++) {
        int k = 0;
        while (needle[k] && (s[k] | 0x20) == (needle[k] | 0x20)) k++;
        if (!needle[k]) return 1;
    }
    return 0;
}

static BOOL WINAPI create_process_traced(LPCWSTR app, LPWSTR cmd, LPSECURITY_ATTRIBUTES pa, LPSECURITY_ATTRIBUTES ta,
                                         BOOL inherit, DWORD flags, LPVOID env, LPCWSTR dir, LPSTARTUPINFOW si,
                                         LPPROCESS_INFORMATION pi) {
    if (!names_handler(app, cmd)) return real_create_process(app, cmd, pa, ta, inherit, flags, env, dir, si, pi);
    BOOL ok = real_create_process(app, cmd, pa, ta, inherit, flags | CREATE_SUSPENDED, env, dir, si, pi);
    if (!ok) return ok;
    DWORD err = GetLastError();
    SIZE_T bytes = (lstrlenW(handler_dll) + 1) * sizeof(WCHAR);
    void *remote = VirtualAllocEx(pi->hProcess, NULL, bytes, MEM_COMMIT | MEM_RESERVE, PAGE_READWRITE);
    if (remote && WriteProcessMemory(pi->hProcess, remote, handler_dll, bytes, NULL))
        QueueUserAPC((PAPCFUNC)(void *)LoadLibraryW, pi->hThread, (ULONG_PTR)remote);
    if (!(flags & CREATE_SUSPENDED)) ResumeThread(pi->hThread);
    SetLastError(err);
    return ok;
}

static void *jit_override(const char *fn, void *original) {
    if (lstrcmpA(fn, "CreateProcessW")) return NULL;
    real_create_process = (create_process_fn)original;
    return (void *)create_process_traced;
}

static void fail(const char *what) {
    HANDLE e = GetStdHandle(STD_ERROR_HANDLE);
    DWORD w;
    WriteFile(e, what, lstrlenA(what), &w, NULL);
    ExitProcess(2);
}

static HANDLE activation = INVALID_HANDLE_VALUE;

static void activation_line(const char *what, UINT msg, WPARAM wp, LPARAM lp, HWND w) {
    char line[512];
    WCHAR cls[64];
    char cls8[64];
    HWND fg = GetForegroundWindow();
    DWORD fg_pid = 0, fg_tid = fg ? GetWindowThreadProcessId(fg, &fg_pid) : 0;
    cls[0] = 0;
    if (fg) GetClassNameW(fg, cls, 64);
    int i = 0;
    for (; cls[i] && i < 63; i++) cls8[i] = cls[i] < 128 ? (char)cls[i] : '?';
    cls8[i] = 0;
    int n = snprintf(line, sizeof line, "%s\t%x\t%llx\t%llx\t%llx\t%llx\t%lu\t%lu\t%s\t%lu\t%lu\n", what, msg,
                     (unsigned long long)wp, (unsigned long long)lp, (unsigned long long)(ULONG_PTR)w,
                     (unsigned long long)(ULONG_PTR)fg, fg_tid, fg_pid, cls8, GetCurrentThreadId(), GetCurrentProcessId());
    DWORD written;
    if (n > 0) WriteFile(activation, line, (DWORD)n, &written, NULL);
}

static LRESULT CALLBACK activation_hook(int code, WPARAM wp, LPARAM lp) {
    const CWPSTRUCT *c = (const CWPSTRUCT *)lp;
    if (code == HC_ACTION && (c->message == WM_WINDOWPOSCHANGING || c->message == WM_ACTIVATEAPP ||
                              c->message == WM_NCACTIVATE || c->message == WM_ACTIVATE || c->message == WM_SETFOCUS ||
                              c->message == WM_KILLFOCUS))
        activation_line("msg", c->message, c->wParam, c->lParam, c->hwnd);
    return CallNextHookEx(NULL, code, wp, lp);
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
    const char *why = trace_init(cap);
    if (why) fail(why);
    lstrcpyW(handler_dll, dir);
    lstrcatW(handler_dll, L"handler_trace.dll");
    import_override = jit_override;
    hook_image(unity, "UnityPlayer.dll");
    hook_image(mono, "mono-2.0-bdwgc.dll");
    CreateThread(NULL, 0, monitor, NULL, 0, NULL);
    lstrcpyW(path, cap);
    lstrcatW(path, L".activation.tsv");
    activation = CreateFileW(path, GENERIC_WRITE, FILE_SHARE_READ, NULL, CREATE_ALWAYS, FILE_ATTRIBUTE_NORMAL, NULL);
    if (activation != INVALID_HANDLE_VALUE) {
        UINT timeout = 0;
        SystemParametersInfoW(SPI_GETFOREGROUNDLOCKTIMEOUT, 0, &timeout, 0);
        activation_line("start", timeout, 0, 0, NULL);
        SetWindowsHookExW(WH_CALLWNDPROC, activation_hook, NULL, GetCurrentThreadId());
    }
    unity_main_fn unity_main = (unity_main_fn)GetProcAddress(unity, "UnityMain");
    if (!unity_main) fail("jit_capture: UnityMain missing\n");
    return unity_main(inst, prev, cmd, show);
}
