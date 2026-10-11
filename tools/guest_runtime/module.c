// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* module.c — the module calls (Sprint 044, family "modules").
 *
 * LoadLibraryExW, GetModuleHandleW, GetModuleHandleExW, GetModuleFileNameW, GetProcAddress and
 * FreeLibrary, as contracts/windows.module.queries/0.1.0.json records Windows answering on six
 * runners (store/probe-runs/380392*). Which module a name means is the host's (src/w2m/modules.py);
 * every Windows rule is here:
 *   - success leaves the last-error code untouched, except GetModuleFileNameW, which sets 0
 *   - a module found nowhere: ERROR_MOD_NOT_FOUND; a name a module does not export: ERROR_PROC_NOT_FOUND
 *   - GetModuleFileNameW into a buffer too small: nSize - 1 characters and a NUL, returns nSize,
 *     ERROR_INSUFFICIENT_BUFFER
 * Argument forms no row covers stop the run (w2m_unimplemented).
 */
#include "runtime.h"

#define ERROR_MOD_NOT_FOUND 126u
#define ERROR_PROC_NOT_FOUND 127u
#define ERROR_INSUFFICIENT_BUFFER 122u
#define LOAD_LIBRARY_SEARCH_SYSTEM32 0x800u
#define FLAG_PIN 1u
#define FLAG_UNCHANGED_REFCOUNT 2u
#define FLAG_FROM_ADDRESS 4u
#define PATH_CHARS 520

static unsigned short path[PATH_CHARS];

static void set_error(u32 code) { w2m_teb()->last_error_value = code; }

static u64 wide_length(u64 s) {
    u64 n = 0;
    while (((const unsigned short *)s)[n]) n++;
    return n;
}

static u64 byte_length(u64 s) {
    u64 n = 0;
    while (((const u8 *)s)[n]) n++;
    return n;
}

static u64 image_base(void) {
    return *(const volatile u64 *)(W2M_PEB_VA + 0x10);      /* PEB.ImageBaseAddress */
}

static u64 find(u64 name, u64 loading) {
    return w2m_host(W2M_HOST_MODULE, name, wide_length(name), loading, 0, 0);
}

/* LoadLibrary's search. A DLL the host maps now (Sprint 049) comes with a stub that runs its DllMain: the caller's
 * return address goes into the stub at W2M_INIT_STUB_SLOT and the stub becomes the return address, so the service
 * returns into DllMain, and DllMain's return into the caller with RAX the handle. */
static u64 load(u64 name, u64 rsp) {
    u64 init = 0;
    u64 h = w2m_host(W2M_HOST_MODULE, name, wide_length(name), 1, 0, &init);
    if (init) {
        *(u64 *)(init + W2M_INIT_STUB_SLOT) = *(u64 *)rsp;
        *(u64 *)rsp = init;
    }
    return h;
}

/* LoadLibraryExW(lpLibFileName, hFile, dwFlags): the module's handle, or NULL with the error. */
u64 k32_LoadLibraryExW(u64 name, u64 file, u64 flags, u64 unused, u64 rsp) {
    (void)unused;
    flags = (u32)flags;
    if (name == 0 || file != 0 || (flags != 0 && flags != LOAD_LIBRARY_SEARCH_SYSTEM32))
        w2m_unimplemented(W2M_API_LoadLibraryExW, flags);
    u64 h = load(name, rsp);
    if (!h) set_error(ERROR_MOD_NOT_FOUND);
    return w2m_logged(W2M_API_LoadLibraryExW, name, h);
}

/* LoadLibraryW(lpLibFileName): LoadLibraryExW with no file and no flags (Sprint 048; contracts/windows.module.system:
 * a system DLL the program imports statically is found under any spelling, as GetModuleHandleW finds it). */
u64 k32_LoadLibraryW(u64 name, u64 unused1, u64 unused2, u64 unused3, u64 rsp) {
    (void)unused1; (void)unused2; (void)unused3;
    if (name == 0) w2m_unimplemented(W2M_API_LoadLibraryW, 0);
    u64 h = load(name, rsp);
    if (!h) set_error(ERROR_MOD_NOT_FOUND);
    return w2m_logged(W2M_API_LoadLibraryW, name, h);
}

/* GetModuleHandleW(lpModuleName): NULL means the image; a module not loaded: NULL, the error. */
u64 k32_GetModuleHandleW(u64 name) {
    u64 h = name ? find(name, 0) : image_base();
    if (!h) set_error(ERROR_MOD_NOT_FOUND);
    return w2m_logged(W2M_API_GetModuleHandleW, name, h);
}

/* GetModuleHandleExW(dwFlags, lpModuleName, phModule): nonzero on success. */
u64 k32_GetModuleHandleExW(u64 flags, u64 name, u64 out) {
    flags = (u32)flags;
    if (out == 0 || (flags & ~(u64)(FLAG_PIN | FLAG_UNCHANGED_REFCOUNT | FLAG_FROM_ADDRESS)))
        w2m_unimplemented(W2M_API_GetModuleHandleExW, flags);
    u64 h;
    if (flags & FLAG_FROM_ADDRESS) h = w2m_host(W2M_HOST_MODULE_AT, name, 0, 0, 0, 0);
    else h = name ? find(name, 0) : image_base();
    *(u64 *)out = h;
    if (!h) {
        set_error(ERROR_MOD_NOT_FOUND);
        return w2m_logged(W2M_API_GetModuleHandleExW, flags, 0);
    }
    return w2m_logged(W2M_API_GetModuleHandleExW, flags, 1);
}

/* GetModuleFileNameW(hModule, lpFilename, nSize): characters copied, not counting the NUL. */
u64 k32_GetModuleFileNameW(u64 module, u64 buffer, u64 size) {
    size = (u32)size;
    if (size == 0 || buffer == 0) w2m_unimplemented(W2M_API_GetModuleFileNameW, size);
    u64 h = module ? module : image_base();
    u64 n = w2m_host(W2M_HOST_MODULE_PATH, h, (u64)path, PATH_CHARS, 0, 0);
    if (n == ~0ull) w2m_unimplemented(W2M_API_GetModuleFileNameW, module);
    unsigned short *out = (unsigned short *)buffer;
    if (n < size) {
        for (u64 i = 0; i < n; i++) out[i] = path[i];
        out[n] = 0;
        set_error(0);
        return w2m_logged(W2M_API_GetModuleFileNameW, module, n);
    }
    for (u64 i = 0; i + 1 < size; i++) out[i] = path[i];
    out[size - 1] = 0;
    set_error(ERROR_INSUFFICIENT_BUFFER);
    return w2m_logged(W2M_API_GetModuleFileNameW, module, size);
}

/* GetModuleFileNameA(hModule, lpFilename, nSize): GetModuleFileNameW's answer narrowed (contracts/windows.module.
 * names_ansi, Sprint 049: the same length, the same short-buffer form and errors). w2m's module paths are ASCII;
 * another character stops the run rather than guess the code page's mapping. */
u64 k32_GetModuleFileNameA(u64 module, u64 buffer, u64 size) {
    size = (u32)size;
    if (size == 0 || buffer == 0) w2m_unimplemented(W2M_API_GetModuleFileNameA, size);
    u64 h = module ? module : image_base();
    u64 n = w2m_host(W2M_HOST_MODULE_PATH, h, (u64)path, PATH_CHARS, 0, 0);
    if (n == ~0ull) w2m_unimplemented(W2M_API_GetModuleFileNameA, module);
    for (u64 i = 0; i < n; i++) if (path[i] > 0x7F) w2m_unimplemented(W2M_API_GetModuleFileNameA, path[i]);
    char *out = (char *)buffer;
    if (n < size) {
        for (u64 i = 0; i < n; i++) out[i] = (char)path[i];
        out[n] = 0;
        set_error(0);
        return w2m_logged(W2M_API_GetModuleFileNameA, module, n);
    }
    for (u64 i = 0; i + 1 < size; i++) out[i] = (char)path[i];
    out[size - 1] = 0;
    set_error(ERROR_INSUFFICIENT_BUFFER);
    return w2m_logged(W2M_API_GetModuleFileNameA, module, size);
}

/* GetModuleHandleA(lpModuleName): GetModuleHandleW on the name widened (ASCII names; another byte stops the run). */
u64 k32_GetModuleHandleA(u64 name) {
    u64 h;
    if (!name) h = image_base();
    else {
        u64 n = byte_length(name);
        if (n >= PATH_CHARS) w2m_unimplemented(W2M_API_GetModuleHandleA, n);
        for (u64 i = 0; i < n; i++) {
            u8 c = ((const u8 *)name)[i];
            if (c > 0x7F) w2m_unimplemented(W2M_API_GetModuleHandleA, c);
            path[i] = c;
        }
        path[n] = 0;
        h = find((u64)path, 0);
    }
    if (!h) set_error(ERROR_MOD_NOT_FOUND);
    return w2m_logged(W2M_API_GetModuleHandleA, name, h);
}

/* GetProcAddress(hModule, lpProcName): the function's address, or NULL with the error. */
u64 k32_GetProcAddress(u64 module, u64 name) {
    if (module == 0 || name < 0x10000) w2m_unimplemented(W2M_API_GetProcAddress, name);   /* ordinals */
    u64 p = w2m_host(W2M_HOST_PROC, module, name, byte_length(name), 0, 0);
    if (!p) set_error(ERROR_PROC_NOT_FOUND);
    return w2m_logged(W2M_API_GetProcAddress, module, p);
}

/* FreeLibrary(hLibModule): nonzero. The modules w2m holds are never unloaded. */
u64 k32_FreeLibrary(u64 module) {
    if (w2m_host(W2M_HOST_MODULE_PATH, module, 0, 0, 0, 0) == ~0ull) w2m_unimplemented(W2M_API_FreeLibrary, module);
    return w2m_logged(W2M_API_FreeLibrary, module, 1);
}
