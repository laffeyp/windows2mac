/* tools/guest_runtime/file.c — Windows file API for the w2m guest runtime. */

#include "runtime.h"

/* Windows constants used by the probe. */
#define GENERIC_READ            0x80000000u
#define GENERIC_WRITE           0x40000000u

#define FILE_SHARE_READ         0x00000001u
#define FILE_SHARE_WRITE        0x00000002u
#define FILE_SHARE_DELETE       0x00000004u

#define CREATE_NEW              1u
#define CREATE_ALWAYS           2u
#define OPEN_EXISTING           3u
#define OPEN_ALWAYS             4u
#define TRUNCATE_EXISTING       5u

#define FILE_ATTRIBUTE_NORMAL   0x00000080u

#define FILE_BEGIN              0u
#define FILE_CURRENT            1u
#define FILE_END                2u

#define INVALID_HANDLE_VALUE    ((u64)(-1))

/* Windows error codes. */
#define ERROR_SUCCESS           0u
#define ERROR_FILE_NOT_FOUND    2u
#define ERROR_PATH_NOT_FOUND    3u
#define ERROR_ACCESS_DENIED     5u
#define ERROR_INVALID_HANDLE    6u
#define ERROR_NEGATIVE_SEEK     131u
#define ERROR_ALREADY_EXISTS    183u
#define ERROR_MORE_DATA         234u
#define ERROR_FILE_EXISTS       80u
#define ERROR_SHARING_VIOLATION 32u
#define ERROR_INVALID_PARAMETER 87u

/* Handle table: we keep a small fixed array. Index 0 is unused. */
#define MAX_HANDLES 64

struct file_handle {
    u64 fd;          /* host descriptor, or 0 if free */
    u64 position;    /* current file pointer */
    u32 access;      /* desired access we opened with */
    u32 share;       /* share mode we opened with */
    u64 file_id;     /* inode from host, for sharing checks */
};

static struct file_handle handles[MAX_HANDLES];

static u64 errno_to_win(u64 err)
{
    if (err == 0)
        return ERROR_SUCCESS;
    switch ((long long)err) {
    case -2:   return ERROR_FILE_NOT_FOUND;
    case -13:  return ERROR_ACCESS_DENIED;
    case -17:  return ERROR_FILE_EXISTS;
    case -20:  return ERROR_PATH_NOT_FOUND;
    case -21:  return ERROR_PATH_NOT_FOUND;
    case -22:  return ERROR_INVALID_PARAMETER;
    default:   return ERROR_ACCESS_DENIED;
    }
}

static void set_last_error(u32 code)
{
    w2m_teb()->last_error_value = code;
}

static u64 make_handle(u64 fd, u32 access, u32 share, u64 file_id)
{
    u64 i;
    if (fd == 0)
        return INVALID_HANDLE_VALUE;
    for (i = 1; i < MAX_HANDLES; i++) {
        if (handles[i].fd == 0) {
            handles[i].fd = fd;
            handles[i].position = 0;
            handles[i].access = access;
            handles[i].share = share;
            handles[i].file_id = file_id;
            /* Avoid colliding with the stdout pseudo-handle 0x74. */
            if (i == 0x74)
                return INVALID_HANDLE_VALUE; /* should not happen */
            return i;
        }
    }
    return INVALID_HANDLE_VALUE;
}

static struct file_handle *lookup(u64 h)
{
    if (h == 0 || h >= MAX_HANDLES || h == 0x74)
        return 0;
    if (handles[h].fd == 0)
        return 0;
    return &handles[h];
}

/* Convert UTF-16LE to a host path string in a small static buffer.
 * We only need relative paths with ASCII components for the probe.
 * Returns pointer to static buffer, or 0 if the path is unsupported. */
static char *normalize_path(u64 name, u64 chars)
{
    static char buf[512];
    u64 i, j;
    u8 *p = (u8 *)name;
    if (chars == 0 || chars >= 256)
        return 0;
    j = 0;
    for (i = 0; i < chars; i++) {
        u16 c = p[2 * i] | ((u16)p[2 * i + 1] << 8);
        if (c == 0)
            break;
        if (c == '\\' || c == '/')
            c = '/';
        else if (c >= 'A' && c <= 'Z')
            c = c - 'A' + 'a';
        else if (c > 0x7f)
            return 0; /* non-ASCII not needed */
        else if (c == ':' || c == 0)
            return 0;
        buf[j++] = (char)c;
    }
    buf[j] = 0;
    return buf;
}

static u64 path_kind(u64 name, u64 chars, u64 *file_id)
{
    char *path = normalize_path(name, chars);
    u64 second = 0;
    u64 kind;
    if (!path)
        return (u64)(-1);
    kind = w2m_host(W2M_FILE_LOOKUP, (u64)path, chars, 0, 0, &second);
    if (file_id)
        *file_id = second;
    return kind;
}

static int is_directory_kind(u64 kind)
{
    return kind == 2;
}

static int is_file_kind(u64 kind)
{
    return kind == 1;
}

static int is_absent_kind(u64 kind)
{
    return kind == 0;
}

static int is_missing_dir_kind(u64 kind)
{
    return kind == 3;
}

/* Check whether a new open with (access, share) conflicts with existing opens
 * on the same file_id. */
static int sharing_conflict(u64 file_id, u32 access, u32 share)
{
    u64 i;
    for (i = 1; i < MAX_HANDLES; i++) {
        struct file_handle *h = &handles[i];
        if (h->fd == 0 || h->file_id != file_id)
            continue;
        /* Existing access vs new share, and new access vs existing share. */
        if ((access & GENERIC_READ) && !(h->share & FILE_SHARE_READ))
            return 1;
        if ((access & GENERIC_WRITE) && !(h->share & FILE_SHARE_WRITE))
            return 1;
        if ((h->access & GENERIC_READ) && !(share & FILE_SHARE_READ))
            return 1;
        if ((h->access & GENERIC_WRITE) && !(share & FILE_SHARE_WRITE))
            return 1;
    }
    return 0;
}

u64 k32_CreateFileW(u64 name, u64 desired_access, u64 share_mode,
                    u64 security_attributes, u64 rsp)
{
    u32 creation_disposition = *(u32 *)(rsp + 0x28);
    u32 flags_and_attributes = *(u32 *)(rsp + 0x30);
    u64 template = *(u64 *)(rsp + 0x38);
    u64 chars = 0;
    char *path;
    u64 kind, file_id = 0;
    u64 fd, second = 0;
    u64 ret;
    int create = 0;
    int existed = 0;
    int truncate = 0;
    int open_existing = 0;
    (void)security_attributes;
    (void)template;
    (void)flags_and_attributes;

    /* Count UTF-16LE characters up to null. */
    {
        u8 *p = (u8 *)name;
        while (p[2 * chars] || p[2 * chars + 1])
            chars++;
    }

    path = normalize_path(name, chars);
    if (!path) {
        set_last_error(ERROR_PATH_NOT_FOUND);
        ret = INVALID_HANDLE_VALUE;
        goto log;
    }

    if (path[0] == 0) {
        set_last_error(ERROR_PATH_NOT_FOUND);
        ret = INVALID_HANDLE_VALUE;
        goto log;
    }

    /* Reject absolute paths and drive letters. */
    if (path[0] == '/' || path[0] == '\\') {
        w2m_unimplemented(W2M_API_CreateFileW, 1);
        return 0;
    }

    kind = w2m_host(W2M_FILE_LOOKUP, (u64)path, chars, 0, 0, &second);
    file_id = second;

    if (kind == (u64)(-1)) {
        set_last_error(ERROR_PATH_NOT_FOUND);
        ret = INVALID_HANDLE_VALUE;
        goto log;
    }

    if (creation_disposition == 0 || creation_disposition > 5) {
        set_last_error(ERROR_INVALID_PARAMETER);
        ret = INVALID_HANDLE_VALUE;
        goto log;
    }

    if (creation_disposition == CREATE_NEW) {
        if (kind != 0) {
            set_last_error(ERROR_FILE_EXISTS);
            ret = INVALID_HANDLE_VALUE;
            goto log;
        }
        create = 1;
    } else if (creation_disposition == CREATE_ALWAYS) {
        if (kind == 0) {
            create = 1;
        } else {
            existed = 1;
            truncate = 1;
            open_existing = 1;
        }
    } else if (creation_disposition == OPEN_EXISTING) {
        if (kind == 0) {
            set_last_error(ERROR_FILE_NOT_FOUND);
            ret = INVALID_HANDLE_VALUE;
            goto log;
        }
        open_existing = 1;
    } else if (creation_disposition == OPEN_ALWAYS) {
        if (kind == 0) {
            create = 1;
        } else {
            existed = 1;
            open_existing = 1;
        }
    } else if (creation_disposition == TRUNCATE_EXISTING) {
        if (kind == 0) {
            set_last_error(ERROR_FILE_NOT_FOUND);
            ret = INVALID_HANDLE_VALUE;
            goto log;
        }
        if (!(desired_access & GENERIC_WRITE)) {
            set_last_error(ERROR_INVALID_PARAMETER);
            ret = INVALID_HANDLE_VALUE;
            goto log;
        }
        truncate = 1;
        open_existing = 1;
    }

    if (kind == 2) {
        /* Opening a directory without FILE_FLAG_BACKUP_SEMANTICS is access denied. */
        set_last_error(ERROR_ACCESS_DENIED);
        ret = INVALID_HANDLE_VALUE;
        goto log;
    }

    if (open_existing && kind == 0) {
        set_last_error(ERROR_FILE_NOT_FOUND);
        ret = INVALID_HANDLE_VALUE;
        goto log;
    }

    if (open_existing && sharing_conflict(file_id, (u32)desired_access, (u32)share_mode)) {
        set_last_error(ERROR_SHARING_VIOLATION);
        ret = INVALID_HANDLE_VALUE;
        goto log;
    }

    if (create) {
        fd = w2m_host(W2M_FILE_OPEN, (u64)path, chars, 1, 0, &second);
        if ((long long)fd < 0) {
            set_last_error(errno_to_win(fd));
            ret = INVALID_HANDLE_VALUE;
            goto log;
        }
        file_id = second;
    } else {
        fd = w2m_host(W2M_FILE_OPEN, (u64)path, chars, 0, 0, &second);
        if ((long long)fd < 0) {
            set_last_error(errno_to_win(fd));
            ret = INVALID_HANDLE_VALUE;
            goto log;
        }
        file_id = second;
        if (truncate) {
            u64 tr = w2m_host(W2M_FILE_TRUNCATE, fd, 0, 0, 0, &second);
            if ((long long)tr < 0) {
                w2m_host(W2M_FILE_CLOSE, fd, 0, 0, 0, &second);
                set_last_error(errno_to_win(tr));
                ret = INVALID_HANDLE_VALUE;
                goto log;
            }
        }
    }

    ret = make_handle(fd, (u32)desired_access, (u32)share_mode, file_id);
    if (ret == INVALID_HANDLE_VALUE) {
        w2m_host(W2M_FILE_CLOSE, fd, 0, 0, 0, &second);
        set_last_error(ERROR_ACCESS_DENIED);
        goto log;
    }

    if (existed)
        set_last_error(ERROR_ALREADY_EXISTS);
    else
        set_last_error(ERROR_SUCCESS);

log:
    return w2m_logged(W2M_API_CreateFileW, name, ret);
}

u64 k32_CloseHandle(u64 handle)
{
    struct file_handle *h = lookup(handle);
    u64 ret = 0;
    u64 second = 0;
    if (!h) {
        set_last_error(ERROR_INVALID_HANDLE);
        return w2m_logged(W2M_API_CloseHandle, handle, 0);
    }
    w2m_host(W2M_FILE_CLOSE, h->fd, 0, 0, 0, &second);
    h->fd = 0;
    h->position = 0;
    h->access = 0;
    h->share = 0;
    h->file_id = 0;
    set_last_error(ERROR_SUCCESS);
    return w2m_logged(W2M_API_CloseHandle, handle, 1);
}

u64 k32_DeleteFileW(u64 name)
{
    u64 chars = 0;
    char *path;
    u64 kind, file_id = 0;
    u64 ret;
    u64 second = 0;
    u64 i;

    {
        u8 *p = (u8 *)name;
        while (p[2 * chars] || p[2 * chars + 1])
            chars++;
    }

    path = normalize_path(name, chars);
    if (!path) {
        set_last_error(ERROR_PATH_NOT_FOUND);
        ret = 0;
        goto log;
    }

    kind = w2m_host(W2M_FILE_LOOKUP, (u64)path, chars, 0, 0, &second);
    file_id = second;

    if (kind == 0) {
        set_last_error(ERROR_FILE_NOT_FOUND);
        ret = 0;
        goto log;
    }

    if (kind == 2) {
        set_last_error(ERROR_ACCESS_DENIED);
        ret = 0;
        goto log;
    }

    /* If any open handle lacks FILE_SHARE_DELETE, fail. */
    for (i = 1; i < MAX_HANDLES; i++) {
        if (handles[i].fd && handles[i].file_id == file_id &&
            !(handles[i].share & FILE_SHARE_DELETE)) {
            set_last_error(ERROR_SHARING_VIOLATION);
            ret = 0;
            goto log;
        }
    }

    {
        u64 r = w2m_host(W2M_FILE_UNLINK, (u64)path, chars, 0, 0, &second);
        if ((long long)r < 0) {
            set_last_error(errno_to_win(r));
            ret = 0;
            goto log;
        }
    }

    set_last_error(ERROR_SUCCESS);
    ret = 1;

log:
    return w2m_logged(W2M_API_DeleteFileW, name, ret);
}

u64 k32_ReadFile(u64 handle, u64 buffer, u64 count, u64 bytes_read, u64 rsp)
{
    u64 overlapped = *(u64 *)(rsp + 0x28);
    struct file_handle *h = lookup(handle);
    u64 ret = 0;
    u64 second = 0;
    u64 n;

    *(u32 *)bytes_read = 0;

    if (overlapped) {
        w2m_unimplemented(W2M_API_ReadFile, 1);
        return 0;
    }

    if (!h) {
        set_last_error(ERROR_INVALID_HANDLE);
        return w2m_logged(W2M_API_ReadFile, handle, 0);
    }

    if (!(h->access & GENERIC_READ)) {
        set_last_error(ERROR_ACCESS_DENIED);
        return w2m_logged(W2M_API_ReadFile, handle, 0);
    }

    n = w2m_host(W2M_FILE_READ, h->fd, buffer, count, h->position, &second);
    if ((long long)n < 0) {
        set_last_error(errno_to_win(n));
        return w2m_logged(W2M_API_ReadFile, handle, 0);
    }

    *(u32 *)bytes_read = (u32)n;
    h->position += n;
    set_last_error(ERROR_SUCCESS);
    return w2m_logged(W2M_API_ReadFile, handle, 1);
}

u64 k32_SetFilePointerEx(u64 handle, u64 distance, u64 new_pointer, u64 move_method)
{
    struct file_handle *h = lookup(handle);
    u64 second = 0;
    u64 target;
    u64 size;
    u64 ret;

    if (new_pointer)
        *(u64 *)new_pointer = (u64)(-1);

    if (!h) {
        set_last_error(ERROR_INVALID_HANDLE);
        return w2m_logged(W2M_API_SetFilePointerEx, handle, 0);
    }

    if (move_method > 2) {
        set_last_error(ERROR_INVALID_PARAMETER);
        return w2m_logged(W2M_API_SetFilePointerEx, handle, 0);
    }

    if (move_method == FILE_BEGIN) {
        target = distance;
    } else if (move_method == FILE_CURRENT) {
        target = h->position + (long long)distance;
    } else {
        size = w2m_host(W2M_FILE_SIZE, h->fd, 0, 0, 0, &second);
        target = size + (long long)distance;
    }

    /* Windows rejects a negative resulting position. */
    if ((long long)target < 0) {
        set_last_error(ERROR_NEGATIVE_SEEK);
        return w2m_logged(W2M_API_SetFilePointerEx, handle, 0);
    }

    h->position = target;
    if (new_pointer)
        *(u64 *)new_pointer = target;
    set_last_error(ERROR_SUCCESS);
    return w2m_logged(W2M_API_SetFilePointerEx, handle, 1);
}

u64 w2m_file_write(u64 handle, u64 buffer, u64 count, u64 written)
{
    struct file_handle *h = lookup(handle);
    u64 second = 0;
    u64 n;

    if (!h) {
        set_last_error(ERROR_INVALID_HANDLE);
        return 0;
    }

    if (!(h->access & GENERIC_WRITE)) {
        set_last_error(ERROR_ACCESS_DENIED);
        return 0;
    }

    n = w2m_host(W2M_FILE_WRITE, h->fd, buffer, count, h->position, &second);
    if ((long long)n < 0) {
        set_last_error(errno_to_win(n));
        return 0;
    }

    *(u32 *)written = (u32)n;
    h->position += n;
    set_last_error(ERROR_SUCCESS);
    return 1;
}
