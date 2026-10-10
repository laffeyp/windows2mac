/* file.c — the Windows file calls, inside the guest runtime (Sprint 042).
 *
 * CreateFileW, CloseHandle, DeleteFileW, ReadFile, SetFilePointerEx, and WriteFile on a file
 * handle. The behavior is contracts/windows.file.create_file.dispositions_sharing@0.1.0, whose rows
 * are Windows's own output (store/probe-runs/38031352273, Server 2025 build 26100; 38031354036,
 * Server 2022 build 20348; equal byte for byte). The Mac is reached only through the host file
 * primitives of src/w2m/file_bridge.py; every Windows rule is here.
 *
 * Rules the rows fix, beyond the Microsoft Learn pages:
 *   - a successful CreateFileW sets the last-error code: ERROR_ALREADY_EXISTS for CREATE_ALWAYS and
 *     OPEN_ALWAYS on an existing file, 0 otherwise; DeleteFileW, ReadFile, WriteFile and
 *     SetFilePointerEx leave it untouched on success
 *   - sharing (all 144 recorded pairs): a second open succeeds only when its access is within every
 *     open handle's share mode and every open handle's access is within its share mode;
 *     otherwise ERROR_SHARING_VIOLATION
 *   - DeleteFileW of a file whose open handles all share delete removes the name at once (POSIX
 *     semantics on both builds): OPEN_EXISTING then fails with ERROR_FILE_NOT_FOUND and CREATE_NEW
 *     makes a new file
 *   - a disposition outside 1-5, or TRUNCATE_EXISTING without GENERIC_WRITE: ERROR_INVALID_PARAMETER
 *   - a failed SetFilePointerEx leaves the position and *lpNewFilePointer unchanged
 * Argument forms no row covers stop the run (w2m_unimplemented) instead of being guessed.
 */
#include "runtime.h"

#define INVALID_HANDLE_VALUE 0xFFFFFFFFFFFFFFFFull
#define GENERIC_READ 0x80000000u
#define GENERIC_WRITE 0x40000000u
#define FILE_SHARE_READ 1u
#define FILE_SHARE_WRITE 2u
#define FILE_SHARE_DELETE 4u
#define CREATE_NEW 1u
#define CREATE_ALWAYS 2u
#define OPEN_EXISTING 3u
#define OPEN_ALWAYS 4u
#define TRUNCATE_EXISTING 5u
#define FILE_ATTRIBUTE_NORMAL 0x80u
#define FILE_BEGIN 0u
#define FILE_CURRENT 1u
#define FILE_END 2u

#define ERROR_FILE_NOT_FOUND 2u
#define ERROR_PATH_NOT_FOUND 3u
#define ERROR_ACCESS_DENIED 5u
#define ERROR_INVALID_HANDLE 6u
#define ERROR_SHARING_VIOLATION 32u
#define ERROR_FILE_EXISTS 80u
#define ERROR_INVALID_PARAMETER 87u
#define ERROR_NEGATIVE_SEEK 131u
#define ERROR_ALREADY_EXISTS 183u

enum { LOOKUP_ABSENT, LOOKUP_FILE, LOOKUP_DIRECTORY, LOOKUP_PARENT_MISSING };

/* Access and share as R=1, W=2 (share also D=4). */
#define R 1u
#define W 2u

#define HANDLES 64
#define HANDLE_BASE 0x1000ull          /* file handles: HANDLE_BASE + 4*slot, clear of the stdout value */
#define MAX_PATH_CHARS 259

struct open_file {
    u32 used;
    u32 access;
    u32 share;
    u32 pad;
    u64 fd;                        /* the host bridge's descriptor */
    u64 id;                        /* the host file id: handles on one file share it */
    long long pos;
};

static struct open_file files[HANDLES];
static unsigned short path[MAX_PATH_CHARS + 1];

static void set_error(u32 code) { w2m_teb()->last_error_value = code; }

static struct open_file *file_of(u64 handle) {
    if (handle < HANDLE_BASE || (handle - HANDLE_BASE) % 4) return 0;
    u64 slot = (handle - HANDLE_BASE) / 4;
    if (slot >= HANDLES || !files[slot].used) return 0;
    return &files[slot];
}

/* Copy the name into `path`, Win32-normalized: the last component loses trailing dots and spaces
 * unless it is "." or ".." (Microsoft Learn, Naming Files). Returns the length in characters. */
static u64 normalize(u64 name) {
    const unsigned short *s = (const unsigned short *)name;
    u64 n = 0;
    while (s[n]) {
        if (n == MAX_PATH_CHARS) w2m_unimplemented(W2M_API_CreateFileW, n);
        path[n] = s[n];
        n++;
    }
    u64 start = n;
    while (start && path[start - 1] != '\\' && path[start - 1] != '/') start--;
    u64 len = n - start;
    int dots = (len == 1 && path[start] == '.') || (len == 2 && path[start] == '.' && path[start + 1] == '.');
    if (!dots)
        while (n > start && (path[n - 1] == '.' || path[n - 1] == ' ')) n--;
    path[n] = 0;
    return n;
}

static u64 lookup(u64 chars, u64 *id) {
    return w2m_host(W2M_FILE_LOOKUP, (u64)path, chars, 0, 0, id);
}

static u32 access_of(u32 desired) {
    return (desired & GENERIC_READ ? R : 0) | (desired & GENERIC_WRITE ? W : 0);
}

/* The sharing rule over every handle open on file `id`. */
static int shares(u64 id, u32 access, u32 share) {
    for (int i = 0; i < HANDLES; i++) {
        const struct open_file *f = &files[i];
        if (!f->used || f->id != id) continue;
        if ((access & ~f->share) || (f->access & ~share)) return 0;
    }
    return 1;
}

/* CreateFileW(lpFileName, dwDesiredAccess, dwShareMode, lpSecurityAttributes,
 * dwCreationDisposition, dwFlagsAndAttributes, hTemplateFile). */
u64 k32_CreateFileW(u64 name, u64 desired, u64 share, u64 security, u64 rsp) {
    u32 disposition = *(u32 *)(rsp + 0x28);
    u32 flags = *(u32 *)(rsp + 0x30);
    u64 template_file = *(u64 *)(rsp + 0x38);
    desired = (u32)desired;
    share = (u32)share;
    if (desired == 0 || (desired & ~(u64)(GENERIC_READ | GENERIC_WRITE)))
        w2m_unimplemented(W2M_API_CreateFileW, desired);
    if (share & ~(u64)(FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE))
        w2m_unimplemented(W2M_API_CreateFileW, share);
    if (security || template_file || (flags != 0 && flags != FILE_ATTRIBUTE_NORMAL))
        w2m_unimplemented(W2M_API_CreateFileW, flags);

    u32 access = access_of((u32)desired);
    if (disposition < CREATE_NEW || disposition > TRUNCATE_EXISTING ||
        (disposition == TRUNCATE_EXISTING && !(access & W))) {
        set_error(ERROR_INVALID_PARAMETER);
        return w2m_logged(W2M_API_CreateFileW, name, INVALID_HANDLE_VALUE);
    }
    int slot = 0;
    while (slot < HANDLES && files[slot].used) slot++;
    if (slot == HANDLES) w2m_unimplemented(W2M_API_CreateFileW, HANDLES);

    u64 chars = normalize(name);
    u64 id = 0;
    u64 kind = chars ? lookup(chars, &id) : LOOKUP_PARENT_MISSING;
    u32 error = 0;
    int create = 0, truncate = 0;
    if (kind == LOOKUP_PARENT_MISSING) error = ERROR_PATH_NOT_FOUND;
    else if (kind == LOOKUP_DIRECTORY) error = ERROR_ACCESS_DENIED;
    else if (kind == LOOKUP_ABSENT) {
        if (disposition == OPEN_EXISTING || disposition == TRUNCATE_EXISTING) error = ERROR_FILE_NOT_FOUND;
        else create = 1;
    } else {
        if (disposition == CREATE_NEW) error = ERROR_FILE_EXISTS;
        else if (!shares(id, access, (u32)share)) error = ERROR_SHARING_VIOLATION;
        truncate = disposition == CREATE_ALWAYS || disposition == TRUNCATE_EXISTING;
    }
    if (error) {
        set_error(error);
        return w2m_logged(W2M_API_CreateFileW, name, INVALID_HANDLE_VALUE);
    }
    u64 fd = w2m_host(W2M_FILE_OPEN, (u64)path, chars, (u64)create, 0, &id);
    if ((long long)fd < 0) w2m_unimplemented(W2M_API_CreateFileW, fd);
    if (truncate) w2m_host(W2M_FILE_TRUNCATE, fd, 0, 0, 0, 0);
    struct open_file *f = &files[slot];
    f->used = 1;
    f->access = access;
    f->share = (u32)share;
    f->fd = fd;
    f->id = id;
    f->pos = 0;
    set_error(!create && (disposition == CREATE_ALWAYS || disposition == OPEN_ALWAYS) ? ERROR_ALREADY_EXISTS : 0);
    return w2m_logged(W2M_API_CreateFileW, name, HANDLE_BASE + 4 * (u64)slot);
}

/* "Closes an open object handle." Nonzero on success. */
u64 k32_CloseHandle(u64 handle) {
    struct open_file *f = file_of(handle);
    if (!f) {
        set_error(ERROR_INVALID_HANDLE);
        return w2m_logged(W2M_API_CloseHandle, handle, 0);
    }
    w2m_host(W2M_FILE_CLOSE, f->fd, 0, 0, 0, 0);
    f->used = 0;
    return w2m_logged(W2M_API_CloseHandle, handle, 1);
}

/* "Deletes an existing file." Nonzero on success. */
u64 k32_DeleteFileW(u64 name) {
    u64 chars = normalize(name);
    u64 id = 0;
    u64 kind = chars ? lookup(chars, &id) : LOOKUP_PARENT_MISSING;
    u32 error = 0;
    if (kind == LOOKUP_PARENT_MISSING) error = ERROR_PATH_NOT_FOUND;
    else if (kind == LOOKUP_DIRECTORY) error = ERROR_ACCESS_DENIED;
    else if (kind == LOOKUP_ABSENT) error = ERROR_FILE_NOT_FOUND;
    else
        for (int i = 0; i < HANDLES; i++)
            if (files[i].used && files[i].id == id && !(files[i].share & FILE_SHARE_DELETE))
                error = ERROR_SHARING_VIOLATION;
    if (error) {
        set_error(error);
        return w2m_logged(W2M_API_DeleteFileW, name, 0);
    }
    if ((long long)w2m_host(W2M_FILE_UNLINK, (u64)path, chars, 0, 0, 0) < 0)
        w2m_unimplemented(W2M_API_DeleteFileW, chars);
    return w2m_logged(W2M_API_DeleteFileW, name, 1);
}

/* WriteFile on a file handle, after runtime.c zeroed *written. */
u64 w2m_file_write(u64 handle, u64 buffer, u64 count, u64 written) {
    struct open_file *f = file_of(handle);
    if (!f) {
        set_error(ERROR_INVALID_HANDLE);
        return 0;
    }
    if (!(f->access & W)) {
        set_error(ERROR_ACCESS_DENIED);
        return 0;
    }
    u64 done = w2m_host(W2M_FILE_WRITE, f->fd, buffer, count, (u64)f->pos, 0);
    f->pos += (long long)done;
    *(u32 *)written = (u32)done;
    return 1;
}

/* ReadFile(hFile, lpBuffer, nNumberOfBytesToRead, lpNumberOfBytesRead, lpOverlapped): "sets this
 * value to zero before doing any work"; at end of file it succeeds with zero bytes. */
u64 k32_ReadFile(u64 handle, u64 buffer, u64 count, u64 read, u64 rsp) {
    u64 overlapped = *(u64 *)(rsp + 0x28);
    if (read == 0 || overlapped != 0) w2m_unimplemented(W2M_API_ReadFile, read);
    *(u32 *)read = 0;
    struct open_file *f = file_of(handle);
    if (!f) {
        set_error(ERROR_INVALID_HANDLE);
        return w2m_logged(W2M_API_ReadFile, handle, 0);
    }
    if (!(f->access & R)) {
        set_error(ERROR_ACCESS_DENIED);
        return w2m_logged(W2M_API_ReadFile, handle, 0);
    }
    u64 done = w2m_host(W2M_FILE_READ, f->fd, buffer, count & 0xFFFFFFFFull, (u64)f->pos, 0);
    f->pos += (long long)done;
    *(u32 *)read = (u32)done;
    return w2m_logged(W2M_API_ReadFile, handle, 1);
}

/* SetFilePointerEx(hFile, liDistanceToMove, lpNewFilePointer, dwMoveMethod): nonzero on success; a
 * position before the start is ERROR_NEGATIVE_SEEK. */
u64 k32_SetFilePointerEx(u64 handle, u64 distance, u64 new_pointer, u64 method) {
    struct open_file *f = file_of(handle);
    if (!f) {
        set_error(ERROR_INVALID_HANDLE);
        return w2m_logged(W2M_API_SetFilePointerEx, handle, 0);
    }
    method = (u32)method;
    if (method > FILE_END) w2m_unimplemented(W2M_API_SetFilePointerEx, method);
    long long base = method == FILE_BEGIN ? 0
                   : method == FILE_CURRENT ? f->pos
                   : (long long)w2m_host(W2M_FILE_SIZE, f->fd, 0, 0, 0, 0);
    long long to = base + (long long)distance;
    if (to < 0) {
        set_error(ERROR_NEGATIVE_SEEK);
        return w2m_logged(W2M_API_SetFilePointerEx, handle, 0);
    }
    f->pos = to;
    if (new_pointer) *(long long *)new_pointer = to;
    return w2m_logged(W2M_API_SetFilePointerEx, handle, 1);
}
