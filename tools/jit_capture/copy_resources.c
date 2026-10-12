// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* copy_resources.c — every resource of the game's own executable into the launcher that takes its name (Sprint 061).
 *
 * The workflow runs the game through jit_capture.exe under the game executable's name, so a call on the process's
 * own module (GetModuleHandleW(NULL), the instance UnityMain gets) reads the launcher's resources. The game asks
 * for its icon: LoadIconA(its module, 103) answered NULL and ERROR_RESOURCE_TYPE_NOT_FOUND on the runner (game run
 * 38203364352 seq 1648) because the launcher had no RT_GROUP_ICON, while the game's exe has one. This copies every
 * type, name and language of the original (icons, icon groups, version, manifest) with UpdateResourceW, so the
 * launcher answers as the game's exe does.
 *
 * Usage: copy_resources.exe <original.exe> <launcher.exe>
 */
#include <windows.h>
#include <stdio.h>

static HMODULE source;
static HANDLE update;
static unsigned copied;
static int failed;

static BOOL CALLBACK language(HMODULE m, LPCWSTR type, LPCWSTR name, WORD lang, LONG_PTR unused) {
    (void)unused;
    HRSRC r = FindResourceExW(m, type, name, lang);
    HGLOBAL g = r ? LoadResource(m, r) : NULL;
    void *data = g ? LockResource(g) : NULL;
    DWORD size = r ? SizeofResource(m, r) : 0;
    if (!data || !UpdateResourceW(update, type, name, lang, data, size)) {
        fprintf(stderr, "copy_resources: a resource failed: error %lu\n", GetLastError());
        failed = 1;
        return FALSE;
    }
    copied++;
    return TRUE;
}

static BOOL CALLBACK named(HMODULE m, LPCWSTR type, LPWSTR name, LONG_PTR unused) {
    (void)unused;
    return EnumResourceLanguagesW(m, type, name, language, 0);
}

static BOOL CALLBACK typed(HMODULE m, LPWSTR type, LONG_PTR unused) {
    (void)unused;
    return EnumResourceNamesW(m, type, named, 0);
}

int wmain(int argc, wchar_t **argv) {
    if (argc != 3) {
        fprintf(stderr, "usage: copy_resources <original.exe> <launcher.exe>\n");
        return 2;
    }
    source = LoadLibraryExW(argv[1], NULL, LOAD_LIBRARY_AS_DATAFILE | LOAD_LIBRARY_AS_IMAGE_RESOURCE);
    if (!source) {
        fprintf(stderr, "copy_resources: cannot open the original: error %lu\n", GetLastError());
        return 1;
    }
    update = BeginUpdateResourceW(argv[2], FALSE);
    if (!update) {
        fprintf(stderr, "copy_resources: cannot open the launcher: error %lu\n", GetLastError());
        return 1;
    }
    EnumResourceTypesW(source, typed, 0);
    if (failed || !EndUpdateResourceW(update, failed)) {
        fprintf(stderr, "copy_resources: not written (error %lu)\n", GetLastError());
        return 1;
    }
    printf("copy_resources: %u resources copied\n", copied);
    return copied ? 0 : 1;
}
