/* jit_capture.c — start a Unity Mono game with a hook that records every method Mono compiles
 * (Sprint 033).
 *
 * Built as "<game>.exe" in the game folder, so Unity finds "<game>_Data" by the executable name.
 * Before Unity starts, it loads the game's own MonoBleedingEdge/EmbedRuntime/mono-2.0-bdwgc.dll
 * by full path (UnityPlayer.dll later loads the same module) and registers a JIT-done hook:
 *   mono_profiler_create, mono_profiler_set_jit_done_callback — Unity-Technologies/mono,
 *   branch unity-2019.4-mbe, mono/metadata/profiler.c; jit_done in profiler-events.h passes
 *   (MonoProfiler *, MonoMethod *, MonoJitInfo *).
 * Then it calls UnityPlayer.dll's UnityMain with its own wWinMain arguments; the game's own
 * executable is a single jmp to UnityMain (0x140001000 in build 24247624).
 *
 * Capture file (path in env W2M_JIT_CAPTURE), one record per compiled method, little-endian:
 *   u64 code_start, u32 code_size, u32 name_len, name bytes (UTF-8), code bytes.
 * WriteFile goes straight to the OS, so records survive the process being killed.
 */
#include <windows.h>
#include <stdint.h>

typedef void *(*profiler_create_fn)(void *prof);
typedef void (*jit_done_cb)(void *prof, void *method, void *jinfo);
typedef void (*set_jit_done_fn)(void *handle, jit_done_cb cb);
typedef void *(*code_start_fn)(void *jinfo);
typedef int (*code_size_fn)(void *jinfo);
typedef char *(*full_name_fn)(void *method, int signature);
typedef void (*mono_free_fn)(void *p);
typedef int (*unity_main_fn)(HINSTANCE, HINSTANCE, LPWSTR, int);

static HANDLE out = INVALID_HANDLE_VALUE;
static CRITICAL_SECTION lock;
static code_start_fn code_start;
static code_size_fn code_size;
static full_name_fn full_name;
static mono_free_fn mono_free_p;

static void put(const void *p, DWORD n) {
    DWORD w;
    WriteFile(out, p, n, &w, NULL);
}

static void on_jit_done(void *prof, void *method, void *jinfo) {
    (void)prof;
    if (!jinfo) return;
    uint64_t start = (uint64_t)(uintptr_t)code_start(jinfo);
    uint32_t size = (uint32_t)code_size(jinfo);
    char *name = full_name(method, 1);
    uint32_t len = name ? (uint32_t)lstrlenA(name) : 0;
    EnterCriticalSection(&lock);
    put(&start, 8);
    put(&size, 4);
    put(&len, 4);
    if (len) put(name, len);
    if (size) put((void *)(uintptr_t)start, size);
    LeaveCriticalSection(&lock);
    if (name) mono_free_p(name);
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
    if (!GetEnvironmentVariableW(L"W2M_JIT_CAPTURE", cap, MAX_PATH)) fail("jit_capture: W2M_JIT_CAPTURE unset\n");
    out = CreateFileW(cap, GENERIC_WRITE, FILE_SHARE_READ, NULL, CREATE_ALWAYS, FILE_ATTRIBUTE_NORMAL, NULL);
    if (out == INVALID_HANDLE_VALUE) fail("jit_capture: cannot create capture file\n");
    InitializeCriticalSection(&lock);
    profiler_create_fn create = (profiler_create_fn)GetProcAddress(mono, "mono_profiler_create");
    set_jit_done_fn set_done = (set_jit_done_fn)GetProcAddress(mono, "mono_profiler_set_jit_done_callback");
    code_start = (code_start_fn)GetProcAddress(mono, "mono_jit_info_get_code_start");
    code_size = (code_size_fn)GetProcAddress(mono, "mono_jit_info_get_code_size");
    full_name = (full_name_fn)GetProcAddress(mono, "mono_method_full_name");
    mono_free_p = (mono_free_fn)GetProcAddress(mono, "mono_free");
    if (!create || !set_done || !code_start || !code_size || !full_name || !mono_free_p)
        fail("jit_capture: a Mono export is missing\n");
    set_done(create(NULL), on_jit_done);
    lstrcpyW(path, dir);
    lstrcatW(path, L"UnityPlayer.dll");
    HMODULE unity = LoadLibraryW(path);
    if (!unity) fail("jit_capture: cannot load UnityPlayer.dll\n");
    unity_main_fn unity_main = (unity_main_fn)GetProcAddress(unity, "UnityMain");
    if (!unity_main) fail("jit_capture: UnityMain missing\n");
    return unity_main(inst, prev, cmd, show);
}
