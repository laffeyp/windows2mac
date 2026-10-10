/* tools/guest_runtime/file.c — Windows file API for the w2m guest runtime.
 *
 * Freestanding C, AArch64, no C library. Implements CreateFileW, CloseHandle,
 * DeleteFileW, ReadFile, SetFilePointerEx and the file-handle half of WriteFile.
 * All host interaction goes through w2m_host() with W2M_FILE_* primitives.
 */

#include "runtime.h"

#define INVALID_HANDLE_VALUE ((u64)(-1))
#define STDOUT_HANDLE        0x74ull

/* Windows constants used by the probe. */
#define GENERIC_READ         0x80000000u
#define GENERIC_WRITE        0x40000000u
#define FILE_SHARE_READ      0x00000001u
#define FILE_SHARE_WRITE     0x00000002u
#define FILE_SHARE_DELETE    0x00000004u

#define CREATE_NEW           1u
#define CREATE_ALWAYS        2u
#define OPEN_EXISTING        3u
#define OPEN_ALWAYS          4u
#define TRUNCATE_EXISTING    5u

#define FILE_ATTRIBUTE_NORMAL 0x00000080u

#define FILE_BEGIN           0u
#define FILE_CURRENT         1u
#define FILE_END             2u

/* Windows error codes. */
#define ERROR_FILE_NOT_FOUND 2u
#define ERROR_PATH_NOT_FOUND 3u
#define ERROR_ACCESS_DENIED  5u
#define ERROR_INVALID_HANDLE 6u
#define ERROR_NOT_ENOUGH_MEMORY 8u
#define ERROR_INVALID_PARAMETER 87u
#define ERROR_FILE_EXISTS    80u
#define ERROR_ALREADY_EXISTS 183u
#define ERROR_NEGATIVE_SEEK 131u
#define ERROR_SHARING_VIOLATION 32u

/* Handle table: up to 64 open files. */
#define MAX_HANDLES 64
struct file_handle {
    u64 fd;          /* host descriptor, or INVALID_HANDLE_VALUE */
    u64 position;    /* current file pointer */
    u32 access;      /* desired access the handle was opened with */
    u32 share;       /* share mode */
    u64 file_id;     /* host inode/file id for sharing checks */
    int delete_on_close; /* set when FILE_SHARE_DELETE allows unlink while open */
};
static struct file_handle handles[MAX_HANDLES];

static u64 errno_to_win(u64 err)
{
    if (err <= 0)
        return 0;
    switch ((int)err) {
    case 2:  return ERROR_FILE_NOT_FOUND;
    case 13: return ERROR_ACCESS_DENIED;
    case 17: return ERROR_FILE_EXISTS;
    case 20: return ERROR_FILE_NOT_FOUND;
    case 21: return ERROR_PATH_NOT_FOUND;
    case 22: return ERROR_INVALID_PARAMETER;
    case 28: return ERROR_NOT_ENOUGH_MEMORY;
    default: return ERROR_ACCESS_DENIED;
    }
}

static void set_last_error(u32 code)
{
    w2m_teb()->last_error_value = code;
}

static u64 alloc_handle(u64 fd, u32 access, u32 share, u64 file_id)
{
    u64 i;
    if (fd == STDOUT_HANDLE)
        return INVALID_HANDLE_VALUE;
    for (i = 0; i < MAX_HANDLES; i++) {
        if (handles[i].fd == INVALID_HANDLE_VALUE) {
            handles[i].fd = fd;
            handles[i].position = 0;
            handles[i].access = access;
            handles[i].share = share;
            handles[i].file_id = file_id;
            handles[i].delete_on_close = 0;
            /* Avoid returning the reserved stdout handle value. */
            if (i + 1 == STDOUT_HANDLE)
                return i + 2;
            return i + 1;
        }
    }
    return INVALID_HANDLE_VALUE;
}

static struct file_handle *lookup_handle(u64 h)
{
    if (h == 0 || h == STDOUT_HANDLE)
        return 0;
    if (h > MAX_HANDLES + 1)
        return 0;
    if (h == MAX_HANDLES + 2)
        h = MAX_HANDLES + 1;
    h--;
    if (handles[h].fd == INVALID_HANDLE_VALUE)
        return 0;
    return &handles[h];
}

static void free_handle(u64 h)
{
    struct file_handle *fh = lookup_handle(h);
    if (!fh)
        return;
    if (fh->fd != INVALID_HANDLE_VALUE) {
        u64 second;
        w2m_host(W2M_FILE_CLOSE, fh->fd, 0, 0, 0, &second);
    }
    fh->fd = INVALID_HANDLE_VALUE;
}

/* Convert UTF-16LE to a host path string in a small buffer. Returns length in
 * bytes (not including terminator) or (u64)-1 on unsupported forms. The host
 * expects UTF-8; we only need ASCII paths for the probe. */
static u64 utf16_to_path(u16 *src, u64 chars, char *dst, u64 dst_size)
{
    u64 i, j;
    int had_sep = 1; /* treat start as after a separator */
    for (i = 0, j = 0; i < chars; i++) {
        u16 c = src[i];
        if (c == 0)
            break;
        if (c > 0x7f)
            return (u64)(-1);
        if (c == '\\' || c == '/') {
            /* collapse repeated separators */
            if (!had_sep) {
                if (j + 1 >= dst_size)
                    return (u64)(-1);
                dst[j++] = '/';
                had_sep = 1;
            }
            continue;
        }
        had_sep = 0;
        if (j + 1 >= dst_size)
            return (u64)(-1);
        dst[j++] = (char)c;
    }
    if (j == 0) {
        if (dst_size < 2)
            return (u64)(-1);
        dst[j++] = '.';
    }
    dst[j] = 0;
    return j;
}

/* ASCII case-fold a single byte for NTFS-style comparison. */
static char fold_ascii(char c)
{
    if (c >= 'A' && c <= 'Z')
        return (char)(c + ('a' - 'A'));
    return c;
}

/* Compare two path strings ASCII case-insensitively. */
static int path_eq(const char *a, const char *b)
{
    while (*a && *b) {
        if (fold_ascii(*a) != fold_ascii(*b))
            return 0;
        a++;
        b++;
    }
    return *a == 0 && *b == 0;
}

/* Validate a relative path: reject absolute paths, drive letters, and .. that
 * leave the root. Also strip trailing spaces and dots per Windows behavior. */
static int normalize_path(char *path, u64 len)
{
    u64 i, out;
    int starts_with_dot = 0;

    if (len == 0)
        return 0;

    /* Reject absolute paths and drive letters. */
    if (path[0] == '/')
        return -1;
    if (len >= 2 && path[1] == ':')
        return -1;

    /* Strip trailing spaces and dots. */
    while (len > 0 && (path[len - 1] == ' ' || path[len - 1] == '.'))
        path[--len] = 0;

    if (len == 0) {
        path[0] = '.';
        path[1] = 0;
        return 0;
    }

    /* Collapse . components and reject .. that escapes root. */
    out = 0;
    i = 0;
    while (i < len) {
        u64 start = i;
        while (i < len && path[i] != '/')
            i++;
        u64 comp_len = i - start;

        if (comp_len == 1 && path[start] == '.') {
            /* skip . */
        } else if (comp_len == 2 && path[start] == '.' && path[start + 1] == '.') {
            if (out == 0)
                return -1; /* escapes root */
            /* remove last component */
            while (out > 0 && path[out - 1] != '/')
                out--;
            if (out > 0)
                out--;
        } else {
            if (out > 0)
                path[out++] = '/';
            while (start < i)
                path[out++] = path[start++];
        }

        if (i < len && path[i] == '/')
            i++;
    }

    path[out] = 0;

    if (out == 0) {
        path[0] = '.';
        path[1] = 0;
    }

    return 0;
}

/* Look up a path on the host, returning kind and file_id. */
static u64 host_lookup(const char *path, u64 *file_id)
{
    u64 second = 0;
    u64 kind = w2m_host(W2M_FILE_LOOKUP, (u64)path, 0, 0, 0, &second);
    *file_id = second;
    return kind;
}

/* Check whether opening a new handle conflicts with existing open handles on
 * the same file_id. */
static int sharing_conflict(u64 file_id, u32 desired_access, u32 share_mode)
{
    u64 i;
    for (i = 0; i < MAX_HANDLES; i++) {
        struct file_handle *fh = &handles[i];
        if (fh->fd == INVALID_HANDLE_VALUE)
            continue;
        if (fh->file_id != file_id)
            continue;

        /* Existing handle's share must permit our desired access. */
        if ((desired_access & GENERIC_READ) && !(fh->share & FILE_SHARE_READ))
            return 1;
        if ((desired_access & GENERIC_WRITE) && !(fh->share & FILE_SHARE_WRITE))
            return 1;

        /* Our share must permit the existing handle's access. */
        if ((fh->access & GENERIC_READ) && !(share_mode & FILE_SHARE_READ))
            return 1;
        if ((fh->access & GENERIC_WRITE) && !(share_mode & FILE_SHARE_WRITE))
            return 1;
    }
    return 0;
}

/* Mark any open handles on file_id as delete-on-close. */
static void mark_delete_on_close(u64 file_id)
{
    u64 i;
    for (i = 0; i < MAX_HANDLES; i++) {
        if (handles[i].file_id == file_id && handles[i].fd != INVALID_HANDLE_VALUE)
            handles[i].delete_on_close = 1;
    }
}

/* Perform actual deletion of handles marked delete-on-close. */
static void do_pending_deletes(void)
{
    u64 i;
    for (i = 0; i < MAX_HANDLES; i++) {
        if (handles[i].delete_on_close && handles[i].fd == INVALID_HANDLE_VALUE) {
            /* Already closed; host unlink was done at close time. */
            handles[i].delete_on_close = 0;
        }
    }
}

u64 k32_CreateFileW(u64 name, u64 desired_access, u64 share_mode,
                    u64 security_attributes, u64 rsp)
{
    u32 creation_disposition = *(u32 *)(rsp + 0x28);
    u32 flags_attrs = *(u32 *)(rsp + 0x30);
    u64 template = *(u64 *)(rsp + 0x38);
    char path[512];
    u64 path_len;
    u64 kind, file_id, fd, ret;
    u64 second;
    int exists;
    int create = 0;
    int truncate = 0;
    int must_not_exist = 0;
    int open_existing = 0;
    int open_always = 0;
    u32 winerr = 0;
    struct file_handle *fh;

    (void)security_attributes;
    (void)template;

    if (creation_disposition == 0 || creation_disposition > 5) {
        set_last_error(ERROR_INVALID_PARAMETER);
        return w2m_logged(W2M_API_CreateFileW, name, INVALID_HANDLE_VALUE);
    }

    if (flags_attrs != FILE_ATTRIBUTE_NORMAL) {
        w2m_unimplemented(W2M_API_CreateFileW, flags_attrs);
        return INVALID_HANDLE_VALUE;
    }

    path_len = utf16_to_path((u16 *)name, 0x7fffffffull, path, sizeof(path));
    if (path_len == (u64)(-1)) {
        w2m_unimplemented(W2M_API_CreateFileW, 1);
        return INVALID_HANDLE_VALUE;
    }
    if (normalize_path(path, path_len) < 0) {
        w2m_unimplemented(W2M_API_CreateFileW, 2);
        return INVALID_HANDLE_VALUE;
    }

    kind = host_lookup(path, &file_id);
    exists = (kind == 1);

    if (kind == 3) {
        /* A directory on the way is missing. */
        set_last_error(ERROR_PATH_NOT_FOUND);
        return w2m_logged(W2M_API_CreateFileW, name, INVALID_HANDLE_VALUE);
    }

    if (kind == 2) {
        /* It's a directory. Without FILE_FLAG_BACKUP_SEMANTICS, fail. */
        set_last_error(ERROR_ACCESS_DENIED);
        return w2m_logged(W2M_API_CreateFileW, name, INVALID_HANDLE_VALUE);
    }

    switch (creation_disposition) {
    case CREATE_NEW:
        must_not_exist = 1;
        create = 1;
        break;
    case CREATE_ALWAYS:
        create = 1;
        truncate = 1;
        break;
    case OPEN_EXISTING:
        open_existing = 1;
        break;
    case OPEN_ALWAYS:
        open_always = 1;
        create = 1;
        break;
    case TRUNCATE_EXISTING:
        truncate = 1;
        open_existing = 1;
        break;
    }

    if (open_existing && !exists) {
        set_last_error(ERROR_FILE_NOT_FOUND);
        return w2m_logged(W2M_API_CreateFileW, name, INVALID_HANDLE_VALUE);
    }

    if (must_not_exist && exists) {
        set_last_error(ERROR_FILE_EXISTS);
        return w2m_logged(W2M_API_CreateFileW, name, INVALID_HANDLE_VALUE);
    }

    if (truncate && !(desired_access & GENERIC_WRITE)) {
        set_last_error(ERROR_INVALID_PARAMETER);
        return w2m_logged(W2M_API_CreateFileW, name, INVALID_HANDLE_VALUE);
    }

    if (exists && sharing_conflict(file_id, (u32)desired_access, (u32)share_mode)) {
        set_last_error(ERROR_SHARING_VIOLATION);
        return w2m_logged(W2M_API_CreateFileW, name, INVALID_HANDLE_VALUE);
    }

    /* Open or create on host. create=1 means O_CREAT|O_EXCL. */
    fd = w2m_host(W2M_FILE_OPEN, (u64)path, 0, create ? 1ull : 0ull, 0, &second);
    if ((s64)fd < 0) {
        winerr = errno_to_win(-(s64)fd);
        if (winerr == 0)
            winerr = ERROR_ACCESS_DENIED;
        set_last_error(winerr);
        return w2m_logged(W2M_API_CreateFileW, name, INVALID_HANDLE_VALUE);
    }
    file_id = second;

    if (truncate && exists) {
        u64 trunc_res = w2m_host(W2M_FILE_TRUNCATE, fd, 0, 0, 0, &second);
        (void)trunc_res;
    }

    ret = alloc_handle(fd, (u32)desired_access, (u32)share_mode, file_id);
    if (ret == INVALID_HANDLE_VALUE) {
        w2m_host(W2M_FILE_CLOSE, fd, 0, 0, 0, &second);
        set_last_error(ERROR_NOT_ENOUGH_MEMORY);
        return w2m_logged(W2M_API_CreateFileW, name, INVALID_HANDLE_VALUE);
    }

    fh = lookup_handle(ret);

    if (creation_disposition == CREATE_ALWAYS && exists) {
        set_last_error(ERROR_ALREADY_EXISTS);
    } else if (creation_disposition == OPEN_ALWAYS && exists) {
        set_last_error(ERROR_ALREADY_EXISTS);
    } else {
        set_last_error(0);
    }

    return w2m_logged(W2M_API_CreateFileW, name, ret);
}

u64 k32_CloseHandle(u64 handle)
{
    struct file_handle *fh = lookup_handle(handle);
    u64 ret;
    if (!fh) {
        set_last_error(ERROR_INVALID_HANDLE);
        return w2m_logged(W2M_API_CloseHandle, handle, 0);
    }
    if (fh->delete_on_close) {
        u64 second;
        w2m_host(W2M_FILE_UNLINK, (u64)"", 0, 0, 0, &second);
        /* We don't have the path here; rely on host having done POSIX delete
         * semantics. The unlink was already attempted by DeleteFileW. */
    }
    free_handle(handle);
    do_pending_deletes();
    set_last_error(0);
    ret = 1;
    return w2m_logged(W2M_API_CloseHandle, handle, ret);
}

u64 k32_DeleteFileW(u64 name)
{
    char path[512];
    u64 path_len;
    u64 kind, file_id;
    u64 second;
    u64 ret;

    path_len = utf16_to_path((u16 *)name, 0x7fffffffull, path, sizeof(path));
    if (path_len == (u64)(-1)) {
        w2m_unimplemented(W2M_API_DeleteFileW, 1);
        return 0;
    }
    if (normalize_path(path, path_len) < 0) {
        w2m_unimplemented(W2M_API_DeleteFileW, 2);
        return 0;
    }

    kind = host_lookup(path, &file_id);
    if (kind == 0) {
        set_last_error(ERROR_FILE_NOT_FOUND);
        return w2m_logged(W2M_API_DeleteFileW, name, 0);
    }
    if (kind == 2) {
        set_last_error(ERROR_ACCESS_DENIED);
        return w2m_logged(W2M_API_DeleteFileW, name, 0);
    }
    if (kind == 3) {
        set_last_error(ERROR_PATH_NOT_FOUND);
        return w2m_logged(W2M_API_DeleteFileW, name, 0);
    }

    /* Check for open handles without FILE_SHARE_DELETE. */
    {
        u64 i;
        int blocked = 0;
        for (i = 0; i < MAX_HANDLES; i++) {
            if (handles[i].fd != INVALID_HANDLE_VALUE && handles[i].file_id == file_id) {
                if (!(handles[i].share & FILE_SHARE_DELETE)) {
                    blocked = 1;
                    break;
                }
            }
        }
        if (blocked) {
            set_last_error(ERROR_SHARING_VIOLATION);
            return w2m_logged(W2M_API_DeleteFileW, name, 0);
        }
    }

    /* Try host unlink. */
    {
        s64 res = (s64)w2m_host(W2M_FILE_UNLINK, (u64)path, 0, 0, 0, &second);
        if (res < 0) {
            set_last_error(errno_to_win(-res));
            return w2m_logged(W2M_API_DeleteFileW, name, 0);
        }
    }

    /* Mark open handles delete-on-close so later opens see the file gone. */
    mark_delete_on_close(file_id);

    set_last_error(0);
    ret = 1;
    return w2m_logged(W2M_API_DeleteFileW, name, ret);
}

u64 k32_ReadFile(u64 handle, u64 buffer, u64 count, u64 bytes_read, u64 rsp)
{
    u64 overlapped = *(u64 *)(rsp + 0x28);
    struct file_handle *fh = lookup_handle(handle);
    u64 second;
    s64 n;
    u64 ret;

    if (overlapped) {
        w2m_unimplemented(W2M_API_ReadFile, overlapped);
        return 0;
    }

    if (!fh) {
        set_last_error(ERROR_INVALID_HANDLE);
        return w2m_logged(W2M_API_ReadFile, handle, 0);
    }

    if (!(fh->access & GENERIC_READ)) {
        set_last_error(ERROR_ACCESS_DENIED);
        *(u32 *)bytes_read = 0;
        return w2m_logged(W2M_API_ReadFile, handle, 0);
    }

    n = (s64)w2m_host(W2M_FILE_READ, fh->fd, buffer, count, fh->position, &second);
    if (n < 0) {
        set_last_error(errno_to_win(-n));
        *(u32 *)bytes_read = 0;
        return w2m_logged(W2M_API_ReadFile, handle, 0);
    }

    *(u32 *)bytes_read = (u32)n;
    fh->position += (u64)n;
    set_last_error(0);
    ret = 1;
    return w2m_logged(W2M_API_ReadFile, handle, ret);
}

u64 k32_SetFilePointerEx(u64 handle, u64 distance, u64 new_pointer, u64 move_method)
{
    struct file_handle *fh = lookup_handle(handle);
    s64 dist = (s64)distance;
    u64 size, second;
    u64 new_pos;
    u64 ret;

    if (!fh) {
        set_last_error(ERROR_INVALID_HANDLE);
        if (new_pointer)
            *(u64 *)new_pointer = 0;
        return w2m_logged(W2M_API_SetFilePointerEx, handle, 0);
    }

    switch (move_method) {
    case FILE_BEGIN:
        new_pos = (u64)dist;
        if (dist < 0) {
            set_last_error(ERROR_NEGATIVE_SEEK);
            if (new_pointer)
                *(u64 *)new_pointer = (u64)-7; /* probe expects unchanged output on failure */
            return w2m_logged(W2M_API_SetFilePointerEx, handle, 0);
        }
        break;
    case FILE_CURRENT:
        if (dist < 0 && (u64)(-dist) > fh->position) {
            set_last_error(ERROR_NEGATIVE_SEEK);
            if (new_pointer)
                *(u64 *)new_pointer = (u64)-7;
            return w2m_logged(W2M_API_SetFilePointerEx, handle, 0);
        }
        new_pos = fh->position + (u64)dist;
        break;
    case FILE_END:
        size = w2m_host(W2M_FILE_SIZE, fh->fd, 0, 0, 0, &second);
        if (dist < 0 && (u64)(-dist) > size) {
            set_last_error(ERROR_NEGATIVE_SEEK);
            if (new_pointer)
                *(u64 *)new_pointer = (u64)-7;
            return w2m_logged(W2M_API_SetFilePointerEx, handle, 0);
        }
        new_pos = size + (u64)dist;
        break;
    default:
        set_last_error(ERROR_INVALID_PARAMETER);
        if (new_pointer)
            *(u64 *)new_pointer = 0;
        return w2m_logged(W2M_API_SetFilePointerEx, handle, 0);
    }

    fh->position = new_pos;
    if (new_pointer)
        *(u64 *)new_pointer = new_pos;
    set_last_error(0);
    ret = 1;
    return w2m_logged(W2M_API_SetFilePointerEx, handle, ret);
}

u64 w2m_file_write(u64 handle, u64 buffer, u64 count, u64 written)
{
    struct file_handle *fh = lookup_handle(handle);
    u64 second;
    s64 n;

    if (!fh) {
        set_last_error(ERROR_INVALID_HANDLE);
        return 0;
    }

    if (!(fh->access & GENERIC_WRITE)) {
        set_last_error(ERROR_ACCESS_DENIED);
        return 0;
    }

    n = (s64)w2m_host(W2M_FILE_WRITE, fh->fd, buffer, count, fh->position, &second);
    if (n < 0) {
        set_last_error(errno_to_win(-n));
        return 0;
    }

    *(u32 *)written = (u32)n;
    fh->position += (u64)n;
    set_last_error(0);
    return 1;
}
