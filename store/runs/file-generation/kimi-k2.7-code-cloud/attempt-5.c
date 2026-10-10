/* tools/guest_runtime/file.c — Windows file API for the w2m guest runtime. */

#include "runtime.h"

/* Windows constants used by the probe. */
#define GENERIC_READ        0x80000000u
#define GENERIC_WRITE       0x40000000u

#define FILE_SHARE_READ     0x00000001u
#define FILE_SHARE_WRITE    0x00000002u
#define FILE_SHARE_DELETE   0x00000004u

#define CREATE_NEW          1u
#define CREATE_ALWAYS       2u
#define OPEN_EXISTING       3u
#define OPEN_ALWAYS         4u
#define TRUNCATE_EXISTING   5u

#define FILE_ATTRIBUTE_NORMAL      0x00000080u
#define FILE_FLAG_BACKUP_SEMANTICS 0x02000000u

#define FILE_BEGIN          0u
#define FILE_CURRENT        1u
#define FILE_END            2u

#define INVALID_HANDLE_VALUE ((u64)(-1))

/* Windows error codes. */
#define ERROR_FILE_NOT_FOUND        2u
#define ERROR_PATH_NOT_FOUND        3u
#define ERROR_ACCESS_DENIED         5u
#define ERROR_INVALID_HANDLE        6u
#define ERROR_NEGATIVE_SEEK         131u
#define ERROR_ALREADY_EXISTS        183u
#define ERROR_FILE_EXISTS           80u
#define ERROR_SHARING_VIOLATION     32u
#define ERROR_INVALID_PARAMETER     87u

#define MAX_FILES 64

typedef long long s64;
typedef unsigned short u16;

struct file_entry {
    int used;
    u64 fd;
    u64 id;
    u32 access;
    u32 share;
    u64 position;
};

static struct file_entry files[MAX_FILES];

static void set_error(u32 code)
{
    w2m_teb()->last_error_value = code;
}

static struct file_entry *alloc_entry(void)
{
    for (unsigned i = 0; i < MAX_FILES; i++) {
        if (!files[i].used) {
            files[i].used = 1;
            files[i].fd = 0;
            files[i].id = 0;
            files[i].access = 0;
            files[i].share = 0;
            files[i].position = 0;
            return &files[i];
        }
    }
    return 0;
}

static void free_entry(struct file_entry *f)
{
    f->used = 0;
    f->fd = 0;
    f->id = 0;
    f->access = 0;
    f->share = 0;
    f->position = 0;
}

static unsigned handle_index(u64 h)
{
    if (h == 0 || h > MAX_FILES)
        return (unsigned)-1;
    return (unsigned)(h - 1);
}

static struct file_entry *lookup(u64 h)
{
    unsigned idx = handle_index(h);
    if (idx >= MAX_FILES)
        return 0;
    if (!files[idx].used)
        return 0;
    return &files[idx];
}

static int is_sep(u16 c)
{
    return c == '\\' || c == '/';
}

static u16 to_upper_ascii(u16 c)
{
    if (c >= 'a' && c <= 'z')
        return (u16)(c - 'a' + 'A');
    return c;
}

/* Validate and normalize a guest UTF-16LE path into a host UTF-16LE path.
 * Returns 0 on success, fills `out` with UTF-16LE code units and `out_len`
 * with the number of code units. `out` must have room for at least 512 u16.
 * Returns a Windows error code on failure (and sets last error).
 * Calls w2m_unimplemented with `api` for unsupported path forms.
 */
static u32 normalize_path(u64 name, u16 *out, unsigned *out_len, u64 api)
{
    u16 buf[512];
    unsigned len = 0;
    unsigned i;

    if (name == 0) {
        set_error(ERROR_PATH_NOT_FOUND);
        return ERROR_PATH_NOT_FOUND;
    }

    for (i = 0; i < 511; i++) {
        u16 c = ((u16 *)name)[i];
        if (c == 0)
            break;
        buf[i] = c;
    }
    if (i == 511) {
        w2m_unimplemented(api, 1);
        return 0;
    }
    len = i;
    if (len == 0) {
        set_error(ERROR_PATH_NOT_FOUND);
        return ERROR_PATH_NOT_FOUND;
    }

    if (is_sep(buf[0])) {
        w2m_unimplemented(api, 2);
        return 0;
    }
    if (len >= 2 && buf[1] == ':') {
        w2m_unimplemented(api, 3);
        return 0;
    }

    /* Detect path that is only "." (with optional separators). */
    unsigned trim_start = 0;
    while (trim_start < len && is_sep(buf[trim_start])) trim_start++;
    unsigned trim_end = len;
    while (trim_end > trim_start && is_sep(buf[trim_end - 1])) trim_end--;
    int is_dot_path = (trim_end - trim_start == 1 && buf[trim_start] == '.');

    unsigned out_pos = 0;
    unsigned pos = 0;

    while (pos < len) {
        while (pos < len && is_sep(buf[pos]))
            pos++;
        if (pos >= len)
            break;

        unsigned start = pos;
        while (pos < len && !is_sep(buf[pos]))
            pos++;
        unsigned end = pos;

        /* Check for "." and ".." before stripping trailing dots/spaces. */
        int is_dot = 0, is_dotdot = 0;
        if (end - start == 1 && buf[start] == '.') is_dot = 1;
        else if (end - start == 2 && buf[start] == '.' && buf[start + 1] == '.') is_dotdot = 1;

        if (is_dot) {
            continue; /* skip "." */
        }
        if (is_dotdot) {
            if (out_pos == 0) {
                w2m_unimplemented(api, 5);
                return 0;
            }
            while (out_pos > 0 && out[out_pos - 1] != '/')
                out_pos--;
            if (out_pos > 0)
                out_pos--;
            continue;
        }

        /* Strip trailing spaces and dots. */
        unsigned stripped_end = end;
        while (stripped_end > start) {
            u16 c = buf[stripped_end - 1];
            if (c == ' ' || c == '.')
                stripped_end--;
            else
                break;
        }
        if (stripped_end == start)
            continue;

        if (out_pos > 0)
            out[out_pos++] = '/';
        for (i = start; i < stripped_end; i++) {
            u16 c = buf[i];
            if (c > 0x7f) {
                w2m_unimplemented(api, 4);
                return 0;
            }
            out[out_pos++] = to_upper_ascii(c);
        }
    }

    if (out_pos == 0) {
        if (is_dot_path) {
            out[0] = '.';
            out_pos = 1;
        } else {
            set_error(ERROR_PATH_NOT_FOUND);
            return ERROR_PATH_NOT_FOUND;
        }
    }

    *out_len = out_pos;
    return 0;
}

static u32 errno_to_win(u64 e)
{
    switch ((int)e) {
    case -2:    return ERROR_FILE_NOT_FOUND;
    case -13:   return ERROR_ACCESS_DENIED;
    case -17:   return ERROR_FILE_EXISTS;
    case -20:   return ERROR_PATH_NOT_FOUND;
    case -21:   return ERROR_PATH_NOT_FOUND;
    case -36:   return ERROR_FILE_EXISTS;
    case -1:
    default:    return ERROR_ACCESS_DENIED;
    }
}

static int sharing_conflicts(u32 acc1, u32 share1, u32 acc2, u32 share2)
{
    if ((acc2 & GENERIC_READ) && !(share1 & FILE_SHARE_READ))
        return 1;
    if ((acc2 & GENERIC_WRITE) && !(share1 & FILE_SHARE_WRITE))
        return 1;
    if ((acc1 & GENERIC_READ) && !(share2 & FILE_SHARE_READ))
        return 1;
    if ((acc1 & GENERIC_WRITE) && !(share2 & FILE_SHARE_WRITE))
        return 1;

    return 0;
}

static int delete_blocked(u64 id, struct file_entry *exclude)
{
    for (unsigned i = 0; i < MAX_FILES; i++) {
        if (&files[i] == exclude)
            continue;
        if (files[i].used && files[i].id == id) {
            if (!(files[i].share & FILE_SHARE_DELETE))
                return 1;
        }
    }
    return 0;
}

u64 k32_CreateFileW(u64 name, u64 desired_access, u64 share_mode, u64 security_attributes, u64 rsp)
{
    u32 creation_disposition = *(u32 *)(rsp + 0x28);
    u32 flags_and_attributes = *(u32 *)(rsp + 0x30);
    u64 template = *(u64 *)(rsp + 0x38);

    if (template != 0) {
        w2m_unimplemented(W2M_API_CreateFileW, 6);
        return 0;
    }
    if (security_attributes != 0) {
        w2m_unimplemented(W2M_API_CreateFileW, 7);
        return 0;
    }
    if (creation_disposition == 0 || creation_disposition > 5) {
        set_error(ERROR_INVALID_PARAMETER);
        return w2m_logged(W2M_API_CreateFileW, name, INVALID_HANDLE_VALUE);
    }
    if ((desired_access & ~(GENERIC_READ | GENERIC_WRITE)) != 0) {
        w2m_unimplemented(W2M_API_CreateFileW, 8);
        return 0;
    }
    if ((flags_and_attributes & ~(FILE_ATTRIBUTE_NORMAL | FILE_FLAG_BACKUP_SEMANTICS)) != 0) {
        w2m_unimplemented(W2M_API_CreateFileW, 9);
        return 0;
    }
    if (creation_disposition == TRUNCATE_EXISTING && !(desired_access & GENERIC_WRITE)) {
        set_error(ERROR_INVALID_PARAMETER);
        return w2m_logged(W2M_API_CreateFileW, name, INVALID_HANDLE_VALUE);
    }

    u16 path[512];
    unsigned path_len = 0;
    u32 err = normalize_path(name, path, &path_len, W2M_API_CreateFileW);
    if (err != 0) {
        return w2m_logged(W2M_API_CreateFileW, name, INVALID_HANDLE_VALUE);
    }

    if (path_len == 1 && path[0] == '.') {
        if (!(flags_and_attributes & FILE_FLAG_BACKUP_SEMANTICS)) {
            set_error(ERROR_ACCESS_DENIED);
            return w2m_logged(W2M_API_CreateFileW, name, INVALID_HANDLE_VALUE);
        }
        w2m_unimplemented(W2M_API_CreateFileW, 10);
        return 0;
    }

    u64 second = 0;
    u64 kind = w2m_host(W2M_FILE_LOOKUP, (u64)path, path_len, 0, 0, &second);
    u64 file_id = second;

    if (kind == 3) {
        set_error(ERROR_PATH_NOT_FOUND);
        return w2m_logged(W2M_API_CreateFileW, name, INVALID_HANDLE_VALUE);
    }
    if (kind == 2) {
        if (!(flags_and_attributes & FILE_FLAG_BACKUP_SEMANTICS)) {
            set_error(ERROR_ACCESS_DENIED);
            return w2m_logged(W2M_API_CreateFileW, name, INVALID_HANDLE_VALUE);
        }
        w2m_unimplemented(W2M_API_CreateFileW, 11);
        return 0;
    }

    int exists = (kind == 1);

    int must_create = 0, must_not_exist = 0, truncate = 0;
    switch (creation_disposition) {
    case CREATE_NEW:
        must_create = 1;
        must_not_exist = 1;
        break;
    case CREATE_ALWAYS:
        must_create = 1;
        truncate = 1;
        break;
    case OPEN_EXISTING:
        break;
    case OPEN_ALWAYS:
        must_create = 1;
        break;
    case TRUNCATE_EXISTING:
        truncate = 1;
        break;
    }

    if (!exists) {
        if (!must_create) {
            set_error(ERROR_FILE_NOT_FOUND);
            return w2m_logged(W2M_API_CreateFileW, name, INVALID_HANDLE_VALUE);
        }
    } else {
        if (must_not_exist) {
            set_error(ERROR_FILE_EXISTS);
            return w2m_logged(W2M_API_CreateFileW, name, INVALID_HANDLE_VALUE);
        }
    }

    for (unsigned i = 0; i < MAX_FILES; i++) {
        if (files[i].used && files[i].id == file_id) {
            if (sharing_conflicts(files[i].access, files[i].share,
                                  (u32)desired_access, (u32)share_mode)) {
                set_error(ERROR_SHARING_VIOLATION);
                return w2m_logged(W2M_API_CreateFileW, name, INVALID_HANDLE_VALUE);
            }
        }
    }

    u64 create_arg = (!exists && must_create) ? 1 : 0;
    u64 fd = w2m_host(W2M_FILE_OPEN, (u64)path, path_len, create_arg, 0, &second);
    file_id = second;

    if ((s64)fd < 0) {
        set_error(errno_to_win(fd));
        return w2m_logged(W2M_API_CreateFileW, name, INVALID_HANDLE_VALUE);
    }

    if (truncate && exists) {
        w2m_host(W2M_FILE_TRUNCATE, fd, 0, 0, 0, &second);
    }

    struct file_entry *f = alloc_entry();
    if (!f) {
        w2m_host(W2M_FILE_CLOSE, fd, 0, 0, 0, &second);
        set_error(ERROR_ACCESS_DENIED);
        return w2m_logged(W2M_API_CreateFileW, name, INVALID_HANDLE_VALUE);
    }

    f->fd = fd;
    f->id = file_id;
    f->access = (u32)desired_access;
    f->share = (u32)share_mode;
    f->position = 0;

    u64 h = (unsigned)(f - files) + 1;

    if (exists && (creation_disposition == OPEN_ALWAYS || creation_disposition == CREATE_ALWAYS)) {
        set_error(ERROR_ALREADY_EXISTS);
    } else {
        set_error(0);
    }

    return w2m_logged(W2M_API_CreateFileW, name, h);
}

u64 k32_CloseHandle(u64 handle)
{
    struct file_entry *f = lookup(handle);
    if (!f) {
        set_error(ERROR_INVALID_HANDLE);
        return w2m_logged(W2M_API_CloseHandle, handle, 0);
    }

    u64 second = 0;
    w2m_host(W2M_FILE_CLOSE, f->fd, 0, 0, 0, &second);
    free_entry(f);

    set_error(0);
    return w2m_logged(W2M_API_CloseHandle, handle, 1);
}

u64 k32_DeleteFileW(u64 name)
{
    u16 path[512];
    unsigned path_len = 0;
    u32 err = normalize_path(name, path, &path_len, W2M_API_DeleteFileW);
    if (err != 0) {
        return w2m_logged(W2M_API_DeleteFileW, name, 0);
    }

    if (path_len == 1 && path[0] == '.') {
        set_error(ERROR_ACCESS_DENIED);
        return w2m_logged(W2M_API_DeleteFileW, name, 0);
    }

    u64 second = 0;
    u64 kind = w2m_host(W2M_FILE_LOOKUP, (u64)path, path_len, 0, 0, &second);
    u64 file_id = second;

    if (kind == 0) {
        set_error(ERROR_FILE_NOT_FOUND);
        return w2m_logged(W2M_API_DeleteFileW, name, 0);
    }
    if (kind != 1) {
        set_error(ERROR_ACCESS_DENIED);
        return w2m_logged(W2M_API_DeleteFileW, name, 0);
    }

    if (delete_blocked(file_id, 0)) {
        set_error(ERROR_SHARING_VIOLATION);
        return w2m_logged(W2M_API_DeleteFileW, name, 0);
    }

    u64 ret = w2m_host(W2M_FILE_UNLINK, (u64)path, path_len, 0, 0, &second);
    if ((s64)ret < 0) {
        set_error(errno_to_win(ret));
        return w2m_logged(W2M_API_DeleteFileW, name, 0);
    }

    /* DeleteFileW leaves last error untouched on success. */
    return w2m_logged(W2M_API_DeleteFileW, name, 1);
}

u64 k32_ReadFile(u64 handle, u64 buffer, u64 count, u64 bytes_read, u64 rsp)
{
    u64 overlapped = *(u64 *)(rsp + 0x28);
    if (overlapped != 0) {
        w2m_unimplemented(W2M_API_ReadFile, 1);
        return 0;
    }

    struct file_entry *f = lookup(handle);
    if (!f) {
        set_error(ERROR_INVALID_HANDLE);
        if (bytes_read)
            *(u32 *)bytes_read = 0;
        return w2m_logged(W2M_API_ReadFile, handle, 0);
    }

    if (!(f->access & GENERIC_READ)) {
        set_error(ERROR_ACCESS_DENIED);
        if (bytes_read)
            *(u32 *)bytes_read = 0;
        return w2m_logged(W2M_API_ReadFile, handle, 0);
    }

    u64 second = 0;
    u64 n = w2m_host(W2M_FILE_READ, f->fd, buffer, count, f->position, &second);
    if ((s64)n < 0) {
        set_error(ERROR_ACCESS_DENIED);
        if (bytes_read)
            *(u32 *)bytes_read = 0;
        return w2m_logged(W2M_API_ReadFile, handle, 0);
    }

    f->position += n;
    if (bytes_read)
        *(u32 *)bytes_read = (u32)n;

    /* ReadFile leaves last error untouched on success. */
    return w2m_logged(W2M_API_ReadFile, handle, 1);
}

u64 k32_SetFilePointerEx(u64 handle, u64 distance, u64 new_pointer, u64 move_method)
{
    struct file_entry *f = lookup(handle);
    if (!f) {
        set_error(ERROR_INVALID_HANDLE);
        if (new_pointer)
            *(u64 *)new_pointer = (u64)(-1);
        return w2m_logged(W2M_API_SetFilePointerEx, handle, 0);
    }

    u64 second = 0;
    u64 size = w2m_host(W2M_FILE_SIZE, f->fd, 0, 0, 0, &second);
    u64 base;

    switch (move_method) {
    case FILE_BEGIN:
        base = 0;
        break;
    case FILE_CURRENT:
        base = f->position;
        break;
    case FILE_END:
        base = size;
        break;
    default:
        set_error(ERROR_INVALID_PARAMETER);
        if (new_pointer)
            *(u64 *)new_pointer = (u64)(-1);
        return w2m_logged(W2M_API_SetFilePointerEx, handle, 0);
    }

    s64 d = (s64)distance;
    s64 target = (s64)base + d;

    if (target < 0) {
        set_error(ERROR_NEGATIVE_SEEK);
        if (new_pointer)
            *(u64 *)new_pointer = (u64)(-7);
        return w2m_logged(W2M_API_SetFilePointerEx, handle, 0);
    }

    f->position = (u64)target;
    if (new_pointer)
        *(u64 *)new_pointer = (u64)target;

    /* SetFilePointerEx leaves last error untouched on success. */
    return w2m_logged(W2M_API_SetFilePointerEx, handle, 1);
}

u64 w2m_file_write(u64 handle, u64 buffer, u64 count, u64 written)
{
    struct file_entry *f = lookup(handle);
    if (!f) {
        set_error(ERROR_INVALID_HANDLE);
        if (written)
            *(u32 *)written = 0;
        return 0;
    }

    if (!(f->access & GENERIC_WRITE)) {
        set_error(ERROR_ACCESS_DENIED);
        if (written)
            *(u32 *)written = 0;
        return 0;
    }

    u64 second = 0;
    u64 n = w2m_host(W2M_FILE_WRITE, f->fd, buffer, count, f->position, &second);
    if ((s64)n < 0) {
        set_error(ERROR_ACCESS_DENIED);
        if (written)
            *(u32 *)written = 0;
        return 0;
    }

    f->position += n;
    if (written)
        *(u32 *)written = (u32)n;

    /* WriteFile leaves last error untouched on success. */
    return 1;
}
