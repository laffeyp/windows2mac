// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
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
 *
 * Sprint 049 (contracts/windows.file.files_more, the game's forms in specs/work_list.json): GetFileInformationByHandleEx
 * for FileBasicInfo and FileStandardInfo, GetFileSize, SetHandleInformation and GetTempFileNameW. Their rows: a
 * success leaves the last error, but GetTempFileNameW's with uUnique 0 sets 0; a handle opened with no access
 * answers all three handle queries; a closed handle is ERROR_INVALID_HANDLE.
 *
 * Sprint 049 (contracts/windows.file.file_mapping): CreateFileMappingW and MapViewOfFile in mono's read-only form. A
 * mapping holds its own host descriptor, so its views outlive the file's handle and the mapping's; a view is the
 * file's bytes copied into a MEM_MAPPED region of the VirtualAlloc arena (virtual_memory.py). A copy is what a
 * read-only view needs; a write to the file through another handle after the view is made is not seen by it, and
 * writable views are not in the rows.
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
#define HANDLE_FLAG_INHERIT 1u
#define HANDLE_FLAG_PROTECT_FROM_CLOSE 2u

#define ERROR_FILE_NOT_FOUND 2u
#define ERROR_PATH_NOT_FOUND 3u
#define ERROR_ACCESS_DENIED 5u
#define ERROR_INVALID_HANDLE 6u
#define ERROR_BAD_LENGTH 24u
#define ERROR_DIRECTORY 267u
#define ERROR_SHARING_VIOLATION 32u
#define ERROR_FILE_EXISTS 80u
#define ERROR_INVALID_PARAMETER 87u
#define ERROR_NEGATIVE_SEEK 131u
#define ERROR_ALREADY_EXISTS 183u
#define ERROR_NOT_ENOUGH_MEMORY 8u
#define ERROR_FILE_INVALID 1006u

enum { LOOKUP_ABSENT, LOOKUP_FILE, LOOKUP_DIRECTORY, LOOKUP_PARENT_MISSING };

/* Access and share as R=1, W=2 (share also D=4). */
#define R 1u
#define W 2u
#define D 4u                         /* delete access; matches FILE_SHARE_DELETE (Sprint 048) */

#define HANDLES 64
#define HANDLE_BASE 0x1000ull          /* file handles: HANDLE_BASE + 4*slot, clear of the stdout value */
#define MAX_PATH_CHARS 259

struct open_file {
    u32 used;
    u32 access;
    u32 share;
    u32 flags;                     /* HANDLE_FLAG_INHERIT from SECURITY_ATTRIBUTES.bInheritHandle (Sprint 048);
                                      SetHandleInformation's INHERIT and PROTECT_FROM_CLOSE (Sprint 049) */
    u32 append_only;               /* FILE_APPEND_DATA without FILE_WRITE_DATA: every write goes to the end (Sprint 048) */
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

/* For sync_objects.c's DuplicateHandle (Sprint 049): whether h names an open file. */
int w2m_is_file_handle(u64 h) { return file_of(h) != 0; }

u64 w2m_full_path(const unsigned short *in, unsigned short *out, long *part);   /* paths.c (Sprint 048) */

/* The name made full into `path` as GetFullPathNameW makes it (paths.c: the working directory C:\w2m\app,
 * "." and "..", every component's trailing dots and spaces); before Sprint 048 the last component lost its
 * trailing dots and spaces and the name stayed relative. Returns the length in characters. */
static u64 normalize(u64 name) {
    static unsigned short full[1024];
    long part;
    if (!*(const unsigned short *)name) return 0;       /* the empty name: callers answer ERROR_PATH_NOT_FOUND (name.empty) */
    u64 n = w2m_full_path((const unsigned short *)name, full, &part);
    if (!n || n > MAX_PATH_CHARS) w2m_unimplemented(W2M_API_CreateFileW, n);    /* blanks only, or too long: not in the rows */
    for (u64 k = 0; k <= n; k++) path[k] = full[k];
    return n;
}

/* For paths.c (Sprint 048): FILE_STAT on a full path. */
u64 w2m_file_stat_full(const unsigned short *full, u64 out) {
    u64 n = 0;
    while (full[n]) n++;
    return w2m_host(W2M_FILE_STAT, (u64)full, n, out, 0, 0);
}

static u64 lookup(u64 chars, u64 *id) {
    return w2m_host(W2M_FILE_LOOKUP, (u64)path, chars, 0, 0, id);
}

/* For file_attributes.c (Sprint 047): the name normalized as CreateFileW's, then FILE_STAT (32 bytes at `out`:
 * size, birth, access and write times). Returns the kind. */
u64 w2m_file_stat(u64 name, u64 out) {
    u64 chars = normalize(name);
    return w2m_host(W2M_FILE_STAT, (u64)path, chars, out, 0, 0);
}

/* Data and delete access from a mask (contracts/windows.file.access, Sprint 048): reading with GENERIC_READ or
 * FILE_READ_DATA; writing with GENERIC_WRITE, FILE_WRITE_DATA or FILE_APPEND_DATA; GENERIC_ALL and MAXIMUM_ALLOWED
 * give both and DELETE, as FILE_ALL_ACCESS does through its bits. Attribute, extended-attribute, control and
 * SYNCHRONIZE rights give no data access and take no part in sharing. */
#define FILE_READ_DATA 0x1u
#define FILE_WRITE_DATA 0x2u
#define FILE_APPEND_DATA 0x4u
#define DELETE_ACCESS 0x10000u
#define GENERIC_ALL 0x10000000u
#define MAXIMUM_ALLOWED 0x02000000u
#define KNOWN_ACCESS (0x1FFu | 0x1F0000u | GENERIC_READ | GENERIC_WRITE | GENERIC_ALL | MAXIMUM_ALLOWED)
static u32 access_of(u32 desired) {
    u32 all = desired & (GENERIC_ALL | MAXIMUM_ALLOWED) ? R | W | D : 0;
    return all | (desired & (GENERIC_READ | FILE_READ_DATA) ? R : 0) |
           (desired & (GENERIC_WRITE | FILE_WRITE_DATA | FILE_APPEND_DATA) ? W : 0) | (desired & DELETE_ACCESS ? D : 0);
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
    if (desired & ~(u64)KNOWN_ACCESS) w2m_unimplemented(W2M_API_CreateFileW, desired);
    if (share & ~(u64)(FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE))
        w2m_unimplemented(W2M_API_CreateFileW, share);
    /* SECURITY_ATTRIBUTES (contracts/windows.file.security, Sprint 048): nLength is not checked (0, 12 and 1000 all
     * open); a descriptor is not in the rows; bInheritHandle sets the handle's inherit flag when nonzero */
    if (security && *(u64 *)(security + 8)) w2m_unimplemented(W2M_API_CreateFileW, security);
    u32 inherit = security ? *(u32 *)(security + 16) != 0 : 0;
    if (template_file || (flags != 0 && flags != FILE_ATTRIBUTE_NORMAL))
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
    f->flags = inherit;
    f->append_only = (desired & FILE_APPEND_DATA) && !(desired & (GENERIC_WRITE | FILE_WRITE_DATA | GENERIC_ALL | MAXIMUM_ALLOWED));
    f->fd = fd;
    f->id = id;
    f->pos = 0;
    set_error(!create && (disposition == CREATE_ALWAYS || disposition == OPEN_ALWAYS) ? ERROR_ALREADY_EXISTS : 0);
    return w2m_logged(W2M_API_CreateFileW, name, HANDLE_BASE + 4 * (u64)slot);
}

/* A handle CreateFileW gave out and CloseHandle has not closed (console.c's GetFileType). */
int w2m_file_is_open(u64 handle) { return file_of(handle) != 0; }

/* "Closes an open object handle." Nonzero on success. */
u64 k32_CloseHandle(u64 handle) {
    struct open_file *f = file_of(handle);
    if (f && (f->flags & HANDLE_FLAG_PROTECT_FROM_CLOSE)) w2m_unimplemented(W2M_API_CloseHandle, handle);  /* not in the rows */
    if (!f) {
        if (w2m_object_close(handle)) return w2m_logged(W2M_API_CloseHandle, handle, 1);   /* sync_objects.c */
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

static void note_size(u64 id, u64 size);           /* FileStandardInfo's allocation rule, below */

/* WriteFile on a file handle, after runtime.c zeroed *written. */
u64 w2m_file_write(u64 handle, u64 buffer, u64 count, u64 written) {
    struct open_file *f = file_of(handle);
    if (!f && w2m_pipe_write(handle, buffer, count, written) == 1) return 1;     /* processes.c (Sprint 055) */
    if (!f) {
        set_error(ERROR_INVALID_HANDLE);
        return 0;
    }
    if (!(f->access & W)) {
        set_error(ERROR_ACCESS_DENIED);
        return 0;
    }
    /* file-access-rights-constants: "write operations will not overwrite existing data if this flag is specified
     * without FILE_WRITE_DATA" */
    if (f->append_only) f->pos = (long long)w2m_host(W2M_FILE_SIZE, f->fd, 0, 0, 0, 0);
    u64 done = w2m_host(W2M_FILE_WRITE, f->fd, buffer, count, (u64)f->pos, 0);
    f->pos += (long long)done;
    note_size(f->id, (u64)f->pos);                     /* past the file record: clusters from now on */
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
        int p = w2m_pipe_read(handle, buffer, count, read);                      /* processes.c (Sprint 055) */
        if (p == -1) return 0;                                                   /* blocked: retried, no return */
        if (p >= 0) return w2m_logged(W2M_API_ReadFile, handle, (u64)p);
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

int w2m_object_is_open(u64 h);         /* sync_objects.c */
int w2m_is_std_handle(u64 h);          /* console.c */

/* A handle that is neither an open file nor any other live handle: what a closed file handle is. Live handles of
 * other kinds are not in the rows and stop the run. */
static struct open_file *file_or_stop(u64 api, u64 handle) {
    struct open_file *f = file_of(handle);
    if (!f && (w2m_object_is_open(handle) || w2m_is_std_handle(handle))) w2m_unimplemented(api, handle);
    if (!f) set_error(ERROR_INVALID_HANDLE);
    return f;
}

/* GetHandleInformation on a file handle (Sprint 048, contracts/windows.file.security): HANDLE_FLAG_INHERIT (1) as
 * CreateFileW's SECURITY_ATTRIBUTES set it, and both flags as SetHandleInformation set them (Sprint 049). */
u32 *w2m_object_flags(u64 h);          /* sync_objects.c (Sprint 055): a pipe's or an event's handle */

u64 k32_GetHandleInformation(u64 handle, u64 flags) {
    if (!file_of(handle) && w2m_is_pipe(handle)) {
        *(u32 *)flags = *w2m_object_flags(handle);
        return w2m_logged(W2M_API_GetHandleInformation, handle, 1);
    }
    struct open_file *f = file_or_stop(W2M_API_GetHandleInformation, handle);
    if (!f) return w2m_logged(W2M_API_GetHandleInformation, handle, 0);
    *(u32 *)flags = f->flags;
    return w2m_logged(W2M_API_GetHandleInformation, handle, 1);
}

/* SetHandleInformation(hObject, dwMask, dwFlags): the mask's bits of INHERIT and PROTECT_FROM_CLOSE take dwFlags'
 * values; other bits are ignored, and the answer is still TRUE (row set.unknown_bit). */
u64 k32_SetHandleInformation(u64 handle, u64 mask, u64 flags) {
    if (!file_of(handle) && w2m_is_pipe(handle)) {
        u32 m = (u32)mask & (HANDLE_FLAG_INHERIT | HANDLE_FLAG_PROTECT_FROM_CLOSE), *f = w2m_object_flags(handle);
        *f = (*f & ~m) | ((u32)flags & m);
        return w2m_logged(W2M_API_SetHandleInformation, handle, 1);
    }
    struct open_file *f = file_or_stop(W2M_API_SetHandleInformation, handle);
    if (!f) return w2m_logged(W2M_API_SetHandleInformation, handle, 0);
    u32 m = (u32)mask & (HANDLE_FLAG_INHERIT | HANDLE_FLAG_PROTECT_FROM_CLOSE);
    f->flags = (f->flags & ~m) | ((u32)flags & m);
    return w2m_logged(W2M_API_SetHandleInformation, handle, 1);
}

/* GetFileSize(hFile, lpFileSizeHigh): the low 32 bits; the high ones at lpFileSizeHigh when it is not NULL. A closed
 * handle answers INVALID_FILE_SIZE and leaves *lpFileSizeHigh. */
u64 k32_GetFileSize(u64 handle, u64 high) {
    struct open_file *f = file_or_stop(W2M_API_GetFileSize, handle);
    if (!f) return w2m_logged(W2M_API_GetFileSize, handle, 0xFFFFFFFFu);
    u64 size = w2m_host(W2M_FILE_SIZE, f->fd, 0, 0, 0, 0);
    if (high) *(u32 *)high = (u32)(size >> 32);
    return w2m_logged(W2M_API_GetFileSize, handle, (u32)size);
}

/* NTFS's AllocationSize (rows standard.sweep, Server 2025 and 2022): data up to 720 bytes lives in the file record,
 * in 8-byte units; from 740 bytes on it lives in 4096-byte clusters, and a file that has been there stays there when
 * it shrinks. CLUSTERED holds the ids of files w2m has seen past RESIDENT_MAX. */
#define RESIDENT_MAX 720u
#define CLUSTERED 256
static u64 clustered[CLUSTERED];

static int is_clustered(u64 id) {
    for (int i = 0; i < CLUSTERED; i++)
        if (clustered[i] == id) return 1;
    return 0;
}
static void note_size(u64 id, u64 size) {
    if (size <= RESIDENT_MAX || is_clustered(id)) return;
    for (int i = 0; i < CLUSTERED; i++)
        if (!clustered[i]) { clustered[i] = id; return; }
    w2m_unimplemented(W2M_API_GetFileInformationByHandleEx, CLUSTERED);
}
static u64 allocation_of(u64 id, u64 size) {
    note_size(id, size);
    return is_clustered(id) ? (size + 4095) & ~4095ull : (size + 7) & ~7ull;
}

#define FILE_BASIC_INFO_CLASS 0u
#define FILE_STANDARD_INFO_CLASS 1u
#define FILE_ATTRIBUTE_ARCHIVE 0x20u

/* GetFileInformationByHandleEx(hFile, FileInformationClass, lpFileInformation, dwBufferSize) for the two classes the
 * game asks (mono: FileBasicInfo then FileStandardInfo). A buffer under the structure's size, padding included, is
 * ERROR_BAD_LENGTH and nothing is written; a larger one gets the structure and nothing past it. */
u64 k32_GetFileInformationByHandleEx(u64 handle, u64 info_class, u64 out, u64 size) {
    struct open_file *f = file_or_stop(W2M_API_GetFileInformationByHandleEx, handle);
    if (!f) return w2m_logged(W2M_API_GetFileInformationByHandleEx, handle, 0);
    info_class = (u32)info_class;
    size = (u32)size;
    if (info_class != FILE_BASIC_INFO_CLASS && info_class != FILE_STANDARD_INFO_CLASS)
        w2m_unimplemented(W2M_API_GetFileInformationByHandleEx, info_class);
    if (size < (info_class == FILE_BASIC_INFO_CLASS ? 0x28u : 0x18u)) {
        set_error(ERROR_BAD_LENGTH);
        return w2m_logged(W2M_API_GetFileInformationByHandleEx, info_class, 0);
    }
    u64 st[5];                                        /* size, then birth, access, write and change as FILETIME */
    w2m_host(W2M_FILE_FSTAT, f->fd, (u64)st, 0, 0, 0);
    u64 *q = (u64 *)out;
    if (info_class == FILE_BASIC_INFO_CLASS) {        /* FILE_BASIC_INFO: four times, FileAttributes, 4 bytes of pad */
        q[0] = st[1]; q[1] = st[2]; q[2] = st[3]; q[3] = st[4];
        q[4] = FILE_ATTRIBUTE_ARCHIVE;
    } else {                                          /* FILE_STANDARD_INFO: one link, not deleting, not a directory */
        q[0] = allocation_of(f->id, st[0]);
        q[1] = st[0];
        q[2] = 1;
    }
    return w2m_logged(W2M_API_GetFileInformationByHandleEx, info_class, 1);
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

/* GetTempFileNameW(lpPathName, lpPrefixString, uUnique, lpTempFileName) (rows temp_name.*): the name is the directory
 * as given (not made full), one backslash, the prefix's first three characters, the number in upper-case hex without
 * leading zeros, and ".tmp". The directory must exist (ERROR_DIRECTORY otherwise, whatever uUnique is). A nonzero
 * uUnique gives its low 16 bits and creates nothing; 0 takes a number from the clock, steps past names that exist,
 * creates the empty file, and sets the last error to 0. */
#define TEMP_TRIES 65535u
u64 k32_GetTempFileNameW(u64 dir, u64 prefix, u64 unique, u64 out) {
    const unsigned short *d = (const unsigned short *)dir, *pre = (const unsigned short *)prefix;
    u64 chars = *d ? normalize(dir) : 0;
    if (!*d) w2m_unimplemented(W2M_API_GetTempFileNameW, 0);          /* the empty directory: not in the rows */
    u64 id = 0;
    if (lookup(chars, &id) != LOOKUP_DIRECTORY) {
        set_error(ERROR_DIRECTORY);
        return w2m_logged(W2M_API_GetTempFileNameW, unique, 0);
    }
    unsigned short *name = (unsigned short *)out;
    u32 n = 0;
    while (d[n]) { name[n] = d[n]; n++; }
    if (name[n - 1] != '\\') name[n++] = '\\';
    for (u32 k = 0; k < 3 && pre[k]; k++) name[n++] = pre[k];
    if (n + 8 > MAX_PATH_CHARS) w2m_unimplemented(W2M_API_GetTempFileNameW, n);   /* a name past MAX_PATH: not in the rows */
    u32 number = (u32)unique & 0xFFFF;
    if ((u32)unique && !number) w2m_unimplemented(W2M_API_GetTempFileNameW, unique);   /* 0x10000 and the like */
    for (u32 tries = 0;; tries++) {
        if (!(u32)unique) {
            if (tries == TEMP_TRIES) w2m_unimplemented(W2M_API_GetTempFileNameW, tries);
            if (!tries) number = (u32)(w2m_counter_now() / 10000) & 0xFFFF;     /* GetTickCount's milliseconds */
            else number = (number + 1) & 0xFFFF;
            if (!number) number = 1;
        }
        u32 m = n, digits = 1;
        while (digits < 4 && number >> (4 * digits)) digits++;
        for (u32 k = 0; k < digits; k++) name[m++] = "0123456789ABCDEF"[(number >> (4 * (digits - 1 - k))) & 15];
        name[m++] = '.'; name[m++] = 't'; name[m++] = 'm'; name[m++] = 'p'; name[m] = 0;
        if ((u32)unique) return w2m_logged(W2M_API_GetTempFileNameW, unique, number);
        u64 full = normalize(out);
        if (lookup(full, &id) != LOOKUP_ABSENT) continue;
        u64 fd = w2m_host(W2M_FILE_OPEN, (u64)path, full, 1, 0, &id);
        if ((long long)fd < 0) w2m_unimplemented(W2M_API_GetTempFileNameW, fd);
        w2m_host(W2M_FILE_CLOSE, fd, 0, 0, 0, 0);
        set_error(0);
        return w2m_logged(W2M_API_GetTempFileNameW, unique, number);
    }
}

#define PAGE_READONLY 0x02u
#define PAGE_READWRITE 0x04u
#define PAGE_WRITECOPY 0x08u
#define FILE_MAP_COPY 0x1u
#define FILE_MAP_WRITE 0x2u
#define FILE_MAP_READ 0x4u

u64 w2m_object_mapping(u64 api, u64 fd, u64 size);              /* sync_objects.c */
int w2m_object_mapping_of(u64 h, u64 *fd, u64 *size);

/* CreateFileMappingW(hFile, lpFileMappingAttributes, flProtect, dwMaximumSizeHigh, dwMaximumSizeLow, lpName) (rows
 * map.*): PAGE_READONLY on a handle with read access; the size 0 means the file's, and an empty file is
 * ERROR_FILE_INVALID; a size past the file's end is ERROR_NOT_ENOUGH_MEMORY for a read-only mapping; no read access,
 * or PAGE_READWRITE on a handle without write access, is ERROR_ACCESS_DENIED. Success sets the last error to 0. */
u64 k32_CreateFileMappingW(u64 file, u64 attributes, u64 protect, u64 high, u64 rsp) {
    u64 low = *(u32 *)(rsp + 0x28);
    u64 name = *(u64 *)(rsp + 0x30);
    if (attributes || name) w2m_unimplemented(W2M_API_CreateFileMappingW, name);    /* named or secured: not in the rows */
    if (file == INVALID_HANDLE_VALUE) w2m_unimplemented(W2M_API_CreateFileMappingW, file);   /* paging-file backed */
    struct open_file *f = file_or_stop(W2M_API_CreateFileMappingW, file);
    if (!f) return w2m_logged(W2M_API_CreateFileMappingW, file, 0);
    protect = (u32)protect;
    u32 error = 0;
    if (protect == PAGE_READWRITE) {
        if (!(f->access & W) || !(f->access & R)) error = ERROR_ACCESS_DENIED;
        else w2m_unimplemented(W2M_API_CreateFileMappingW, protect);              /* a writable mapping: not in the rows */
    } else if (protect != PAGE_READONLY) {
        w2m_unimplemented(W2M_API_CreateFileMappingW, protect);
    } else if (!(f->access & R)) {
        error = ERROR_ACCESS_DENIED;
    }
    u64 file_size = w2m_host(W2M_FILE_SIZE, f->fd, 0, 0, 0, 0);
    u64 size = (u64)(u32)high << 32 | low;
    if (!error && size == 0 && file_size == 0) error = ERROR_FILE_INVALID;
    if (!error && size > file_size) error = ERROR_NOT_ENOUGH_MEMORY;
    if (error) {
        set_error(error);
        return w2m_logged(W2M_API_CreateFileMappingW, file, 0);
    }
    u64 fd = w2m_host(W2M_FILE_DUP, f->fd, 0, 0, 0, 0);
    u64 h = w2m_object_mapping(W2M_API_CreateFileMappingW, fd, size ? size : file_size);
    set_error(0);
    return w2m_logged(W2M_API_CreateFileMappingW, file, h);
}

/* MapViewOfFile(hFileMappingObject, dwDesiredAccess, dwFileOffsetHigh, dwFileOffsetLow, dwNumberOfBytesToMap) (rows
 * view.*): FILE_MAP_READ, or FILE_MAP_COPY (a private copy), from offset 0; 0 bytes means the whole mapping; more
 * bytes than the mapping holds, or FILE_MAP_WRITE on a read-only mapping, is ERROR_ACCESS_DENIED. The view starts on
 * a 64 KB boundary and holds zeros from the file's end to its last page's end. Success leaves the last error. */
u64 k32_MapViewOfFile(u64 mapping, u64 access, u64 offset_high, u64 offset_low, u64 rsp) {
    u64 bytes = *(u64 *)(rsp + 0x28);
    u64 fd, size;
    if (!w2m_object_mapping_of(mapping, &fd, &size)) w2m_unimplemented(W2M_API_MapViewOfFile, mapping);
    if ((u32)offset_high || (u32)offset_low) w2m_unimplemented(W2M_API_MapViewOfFile, offset_low);
    access = (u32)access;
    if (access != FILE_MAP_READ && access != FILE_MAP_COPY && access != FILE_MAP_WRITE)
        w2m_unimplemented(W2M_API_MapViewOfFile, access);
    if (access == FILE_MAP_WRITE || bytes > size) {
        set_error(ERROR_ACCESS_DENIED);
        return w2m_logged(W2M_API_MapViewOfFile, mapping, 0);
    }
    u64 view = w2m_host(W2M_HOST_MAP_VIEW, bytes ? bytes : size, fd, access == FILE_MAP_COPY ? PAGE_WRITECOPY : PAGE_READONLY,
                        0, 0);
    return w2m_logged(W2M_API_MapViewOfFile, mapping, view);
}
