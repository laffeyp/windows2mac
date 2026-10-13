// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* create_file_flags.c — CreateFileW with FILE_FLAG_SEQUENTIAL_SCAN | FILE_ATTRIBUTE_NORMAL (0x08000080) beside
 * FILE_ATTRIBUTE_NORMAL (0x80) alone (Sprint 065).
 *
 * The frontier of 2026-10-12 stops at CreateFileW's form 0x08000080 after 50,766,965,231 instructions. For each flags
 * value, each disposition (CREATE_NEW, CREATE_ALWAYS, OPEN_EXISTING, OPEN_ALWAYS, TRUNCATE_EXISTING) on an existing
 * and an absent file, with GENERIC_READ | GENERIC_WRITE and no sharing: the handle's validity and the last error after
 * SetLastError(0xDEAD); on success the attributes GetFileAttributesW reads, the size, a 4-byte write, a read back
 * from the start and a second open of the file (GENERIC_READ, FILE_SHARE_READ) with its error. One JSON line each.
 */
#include <windows.h>
#include <stdio.h>

static const DWORD DISP[] = {CREATE_NEW, CREATE_ALWAYS, OPEN_EXISTING, OPEN_ALWAYS, TRUNCATE_EXISTING};
static const char *DISP_NAME[] = {"create_new", "create_always", "open_existing", "open_always", "truncate_existing"};

static void prepare(const WCHAR *name, int existing) {
    DeleteFileW(name);
    if (existing) {
        HANDLE h = CreateFileW(name, GENERIC_WRITE, 0, NULL, CREATE_NEW, FILE_ATTRIBUTE_NORMAL, NULL);
        DWORD w;
        WriteFile(h, "0123456789", 10, &w, NULL);
        CloseHandle(h);
    }
}

int main(void) {
    const WCHAR *name = L"flags_probe.dat";
    const DWORD flags[] = {FILE_ATTRIBUTE_NORMAL, FILE_FLAG_SEQUENTIAL_SCAN | FILE_ATTRIBUTE_NORMAL};
    for (int f = 0; f < 2; f++)
        for (int d = 0; d < 5; d++)
            for (int existing = 0; existing < 2; existing++) {
                prepare(name, existing);
                SetLastError(0xDEAD);
                HANDLE h = CreateFileW(name, GENERIC_READ | GENERIC_WRITE, 0, NULL, DISP[d], flags[f], NULL);
                DWORD err = GetLastError();
                printf("{\"flags\": %lu, \"disp\": \"%s\", \"existing\": %d, \"ok\": %d, \"err\": %lu", flags[f],
                       DISP_NAME[d], existing, h != INVALID_HANDLE_VALUE, err);
                if (h != INVALID_HANDLE_VALUE) {
                    LARGE_INTEGER size;
                    GetFileSizeEx(h, &size);
                    DWORD w = 0, r = 0;
                    char back[8] = {0};
                    BOOL wrote = WriteFile(h, "abcd", 4, &w, NULL);
                    SetFilePointer(h, 0, NULL, FILE_BEGIN);
                    BOOL read = ReadFile(h, back, 6, &r, NULL);
                    SetLastError(0xDEAD);
                    HANDLE h2 = CreateFileW(name, GENERIC_READ, FILE_SHARE_READ, NULL, OPEN_EXISTING, flags[f], NULL);
                    DWORD err2 = GetLastError();
                    if (h2 != INVALID_HANDLE_VALUE) CloseHandle(h2);
                    printf(", \"attributes\": %lu, \"size\": %lld, \"wrote\": %d, \"written\": %lu, \"read\": %d, "
                           "\"got\": \"%.6s\", \"second_ok\": %d, \"second_err\": %lu",
                           GetFileAttributesW(name), size.QuadPart, wrote, w, read, back, h2 != INVALID_HANDLE_VALUE, err2);
                    CloseHandle(h);
                }
                printf("}\n");
            }
    DeleteFileW(name);
    return 0;
}
