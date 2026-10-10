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

#define INVALID_HANDLE_VALUE    (~(u64)0)

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

/* Handle table: index 0 unused, 1 reserved for stdout. */
#define MAX_HANDLES 64
static struct {
    u64 fd;          /* host descriptor, 0 if free */
    u64 position;
    u32 access;      /* desired access the handle was opened with */
    u32 share;       /* share mode the handle was opened with */
    u64 file_id;     /* host file id for sharing checks */
} handles[MAX_HANDLES];

static u64 next_handle = 2;

static void set_error(u32 code) {
    w2m_teb()->last_error_value = code;
}

static u64 make_handle(u64 fd, u32 access, u32 share, u64 file_id) {
    u64 h;
    if (next_handle < MAX_HANDLES) {
        h = next_handle++;
    } else {
        /* Recycle a free slot if any. */
        h = 0;
        for (u64 i = 2; i < MAX_HANDLES; i++) {
            if (handles[i].fd == 0) {
                h = i;
                break;
            }
        }
        if (h == 0) return INVALID_HANDLE_VALUE;
    }
    handles[h].fd = fd;
    handles[h].position = 0;
    handles[h].access = access;
    handles[h].share = share;
    handles[h].file_id = file_id;
    return h;
}

static int handle_ok(u64 h) {
    return h != 0 && h < MAX_HANDLES && handles[h].fd != 0 && h != 0x74;
}

/* Convert UTF-16LE path to a host path string. Returns length in bytes of the
 * ASCII host path written into buf (not null-terminated; host primitive takes
 * a length). Handles backslash separators, '.' and '..' components, trailing
 * dots/spaces stripped per Windows behavior. Rejects absolute paths, drive
 * letters, and paths escaping the root. */
static u64 normalize_path(const u16 *src, u64 chars, u8 *buf, u64 buf_size) {
    u64 out_len = 0;
    u64 i = 0;

    if (chars == 0) {
        /* Empty path: treat as missing directory. */
        return 0;
    }

    /* Reject absolute paths and drive letters. */
    if (chars >= 2 && src[1] == ':') return ~(u64)0;
    if (chars >= 1 && (src[0] == '\\' || src[0] == '/')) return ~(u64)0;

    while (i < chars) {
        u64 start = i;
        while (i < chars && src[i] != '\\' && src[i] != '/') i++;
        u64 len = i - start;

        /* Strip trailing spaces and dots from the final component, matching
         * Windows behavior for names like "w2m-cf-a.txt." and "w2m-cf-a.txt ". */
        while (len > 0) {
            u16 c = src[start + len - 1];
            if (c == ' ' || c == '.') len--;
            else break;
        }

        if (len == 0) {
            /* Empty component: skip, but if it was a leading separator that's fine. */
            if (i < chars) i++;
            continue;
        }

        /* Check for special components. */
        if (len == 1 && src[start] == '.') {
            if (i < chars) i++;
            continue;
        }
        if (len == 2 && src[start] == '.' && src[start + 1] == '.') {
            /* Escape root: unimplemented. */
            return ~(u64)0;
        }

        /* Append separator and component. */
        if (out_len > 0) {
            if (out_len + 1 >= buf_size) return ~(u64)0;
            buf[out_len++] = '/';
        }
        for (u64 k = 0; k < len; k++) {
            u16 c = src[start + k];
            if (c >= 0x80) {
                /* Non-ASCII: outside scenario. */
                return ~(u64)0;
            }
            /* ASCII case-fold. */
            u8 ch = (u8)c;
            if (ch >= 'a' && ch <= 'z') ch -= 0x20;
            if (out_len >= buf_size) return ~(u64)0;
            buf[out_len++] = ch;
        }

        if (i < chars) i++;
    }

    (void)buf_size;
    return out_len;
}

/* Look up a path. Returns kind via result pointer; file_id via id pointer. */
static int lookup_path(const u8 *path, u64 len, u64 *kind, u64 *id) {
    u64 second = 0;
    u64 r = w2m_host(W2M_FILE_LOOKUP, (u64)path, len, 0, 0, &second);
    *kind = r;
    *id = second;
    return 0;
}

static int open_host(const u8 *path, u64 len, int create_excl, u64 *fd, u64 *id) {
    u64 second = 0;
    u64 r = w2m_host(W2M_FILE_OPEN, (u64)path, len, create_excl ? 1 : 0, 0, &second);
    if ((long long)r < 0) return (int)(-(long long)r);
    *fd = r;
    *id = second;
    return 0;
}

static void close_host(u64 fd) {
    u64 second = 0;
    w2m_host(W2M_FILE_CLOSE, fd, 0, 0, 0, &second);
}

static u64 file_size_host(u64 fd) {
    u64 second = 0;
    return w2m_host(W2M_FILE_SIZE, fd, 0, 0, 0, &second);
}

static u64 file_read_host(u64 fd, u64 buf, u64 n, u64 off) {
    u64 second = 0;
    return w2m_host(W2M_FILE_READ, fd, buf, n, off, &second);
}

static u64 file_write_host(u64 fd, u64 buf, u64 n, u64 off) {
    u64 second = 0;
    return w2m_host(W2M_FILE_WRITE, fd, buf, n, off, &second);
}

static int file_truncate_host(u64 fd, u64 size) {
    u64 second = 0;
    u64 r = w2m_host(W2M_FILE_TRUNCATE, fd, size, 0, 0, &second);
    return (int)(long long)r;
}

static int file_unlink_host(const u8 *path, u64 len) {
    u64 second = 0;
    u64 r = w2m_host(W2M_FILE_UNLINK, (u64)path, len, 0, 0, &second);
    if ((long long)r < 0) return (int)(-(long long)r);
    return 0;
}

/* Check whether a new open with (access, share) conflicts with existing opens
 * of the same file_id. */
static int sharing_conflict(u64 file_id, u32 new_access, u32 new_share, u64 skip_h) {
    for (u64 h = 2; h < MAX_HANDLES; h++) {
        if (h == skip_h) continue;
        if (handles[h].fd == 0) continue;
        if (handles[h].file_id != file_id) continue;

        u32 old_access = handles[h].access;
        u32 old_share = handles[h].share;

        /* New read conflicts if old did not share read. */
        if ((new_access & GENERIC_READ) && !(old_share & FILE_SHARE_READ))
            return 1;
        /* New write conflicts if old did not share write. */
        if ((new_access & GENERIC_WRITE) && !(old_share & FILE_SHARE_WRITE))
            return 1;
        /* Old read conflicts if new does not share read. */
        if ((old_access & GENERIC_READ) && !(new_share & FILE_SHARE_READ))
            return 1;
        /* Old write conflicts if new does not share write. */
        if ((old_access & GENERIC_WRITE) && !(new_share & FILE_SHARE_WRITE))
            return 1;
    }
    return 0;
}

u64 k32_CreateFileW(u64 name, u64 desired_access, u64 share_mode,
                    u64 security_attributes, u64 rsp) {
    u32 creation_disposition = *(u32 *)(rsp + 0x28);
    u32 flags_and_attrs = *(u32 *)(rsp + 0x30);
    u64 template_file = *(u64 *)(rsp + 0x38);
    u64 ret = INVALID_HANDLE_VALUE;
    u32 err = ERROR_SUCCESS;

    (void)security_attributes;

    /* Unsupported arguments. */
    if (template_file != 0) {
        w2m_unimplemented(W2M_API_CreateFileW, 1);
        return w2m_logged(W2M_API_CreateFileW, name, ret);
    }
    if (flags_and_attrs != FILE_ATTRIBUTE_NORMAL) {
        w2m_unimplemented(W2M_API_CreateFileW, 2);
        return w2m_logged(W2M_API_CreateFileW, name, ret);
    }

    /* Validate disposition. */
    if (creation_disposition == 0 || creation_disposition > TRUNCATE_EXISTING) {
        set_error(ERROR_INVALID_PARAMETER);
        return w2m_logged(W2M_API_CreateFileW, name,
                          w2m_logged(W2M_API_CreateFileW, name, INVALID_HANDLE_VALUE));
    }

    /* TRUNCATE_EXISTING requires GENERIC_WRITE. */
    if (creation_disposition == TRUNCATE_EXISTING && !(desired_access & GENERIC_WRITE)) {
        set_error(ERROR_INVALID_PARAMETER);
        return w2m_logged(W2M_API_CreateFileW, name, INVALID_HANDLE_VALUE);
    }

    u8 path_buf[512];
    u64 plen = normalize_path((const u16 *)name, 0x10000, path_buf, sizeof(path_buf));
    if (plen == ~(u64)0) {
        w2m_unimplemented(W2M_API_CreateFileW, 3);
        return w2m_logged(W2M_API_CreateFileW, name, ret);
    }

    /* Empty path -> ERROR_PATH_NOT_FOUND. */
    if (plen == 0) {
        set_error(ERROR_PATH_NOT_FOUND);
        return w2m_logged(W2M_API_CreateFileW, name, INVALID_HANDLE_VALUE);
    }

    u64 kind = 0, file_id = 0;
    lookup_path(path_buf, plen, &kind, &file_id);

    /* Opening "." as a directory without FILE_FLAG_BACKUP_SEMANTICS: Windows
     * returns ERROR_ACCESS_DENIED. Our normalized path becomes empty for ".",
     * so handle that explicitly. */
    {
        const u16 *p = (const u16 *)name;
        u64 chars = 0;
        while (chars < 0x10000 && p[chars]) chars++;
        if (chars == 1 && p[0] == '.') {
            set_error(ERROR_ACCESS_DENIED);
            return w2m_logged(W2M_API_CreateFileW, name, INVALID_HANDLE_VALUE);
        }
    }

    if (kind == 3) {
        /* Missing directory on the path. */
        set_error(ERROR_PATH_NOT_FOUND);
        return w2m_logged(W2M_API_CreateFileW, name, INVALID_HANDLE_VALUE);
    }

    int must_create = 0;
    int must_exist = 0;
    int truncate = 0;

    switch (creation_disposition) {
    case CREATE_NEW:
        must_create = 1;
        break;
    case CREATE_ALWAYS:
        must_create = 1;
        truncate = 1;
        break;
    case OPEN_EXISTING:
        must_exist = 1;
        break;
    case OPEN_ALWAYS:
        if (kind == 1) must_exist = 1;
        else must_create = 1;
        break;
    case TRUNCATE_EXISTING:
        must_exist = 1;
        truncate = 1;
        break;
    }

    if (must_exist && kind != 1) {
        set_error(ERROR_FILE_NOT_FOUND);
        return w2m_logged(W2M_API_CreateFileW, name, INVALID_HANDLE_VALUE);
    }

    if (must_create && kind == 2) {
        set_error(ERROR_ACCESS_DENIED);
        return w2m_logged(W2M_API_CreateFileW, name, INVALID_HANDLE_VALUE);
    }

    /* Sharing conflict check against existing opens. */
    if (kind == 1 && sharing_conflict(file_id, (u32)desired_access, (u32)share_mode, 0)) {
        set_error(ERROR_SHARING_VIOLATION);
        return w2m_logged(W2M_API_CreateFileW, name, INVALID_HANDLE_VALUE);
    }

    u64 fd = 0, new_id = 0;
    int host_err = 0;

    if (must_create) {
        host_err = open_host(path_buf, plen, 1, &fd, &new_id);
        if (host_err != 0) {
            if (kind == 1) {
                set_error(ERROR_FILE_EXISTS);
                return w2m_logged(W2M_API_CreateFileW, name, INVALID_HANDLE_VALUE);
            } else {
                set_error(ERROR_PATH_NOT_FOUND);
                return w2m_logged(W2M_API_CreateFileW, name, INVALID_HANDLE_VALUE);
            }
        }
        if (truncate) {
            file_truncate_host(fd, 0);
        }
        err = ERROR_SUCCESS;
    } else {
        host_err = open_host(path_buf, plen, 0, &fd, &new_id);
        if (host_err != 0) {
            set_error(ERROR_ACCESS_DENIED);
            return w2m_logged(W2M_API_CreateFileW, name, INVALID_HANDLE_VALUE);
        }
        if (truncate) {
            file_truncate_host(fd, 0);
        }
        if (creation_disposition == CREATE_ALWAYS || creation_disposition == OPEN_ALWAYS) {
            err = ERROR_ALREADY_EXISTS;
        } else {
            err = ERROR_SUCCESS;
        }
    }

    ret = make_handle(fd, (u32)desired_access, (u32)share_mode, new_id);
    if (ret == INVALID_HANDLE_VALUE) {
        close_host(fd);
        set_error(ERROR_TOO_MANY_OPEN_FILES);
        return w2m_logged(W2M_API_CreateFileW, name, INVALID_HANDLE_VALUE);
    }

    set_error(err);
    return w2m_logged(W2M_API_CreateFileW, name, ret);
}

u64 k32_CloseHandle(u64 handle) {
    if (!handle_ok(handle)) {
        set_error(ERROR_INVALID_HANDLE);
        return w2m_logged(W2M_API_CloseHandle, handle, 0);
    }
    close_host(handles[handle].fd);
    handles[handle].fd = 0;
    set_error(ERROR_SUCCESS);
    return w2m_logged(W2M_API_CloseHandle, handle, 1);
}

u64 k32_DeleteFileW(u64 name) {
    u8 path_buf[512];
    u64 plen = normalize_path((const u16 *)name, 0x10000, path_buf, sizeof(path_buf));
    if (plen == ~(u64)0) {
        w2m_unimplemented(W2M_API_DeleteFileW, 1);
        return w2m_logged(W2M_API_DeleteFileW, name, 0);
    }
    if (plen == 0) {
        set_error(ERROR_PATH_NOT_FOUND);
        return w2m_logged(W2M_API_DeleteFileW, name, 0);
    }

    u64 kind = 0, file_id = 0;
    lookup_path(path_buf, plen, &kind, &file_id);

    if (kind == 0) {
        set_error(ERROR_FILE_NOT_FOUND);
        return w2m_logged(W2M_API_DeleteFileW, name, 0);
    }
    if (kind == 2) {
        set_error(ERROR_ACCESS_DENIED);
        return w2m_logged(W2M_API_DeleteFileW, name, 0);
    }

    /* Check for open handles that did not allow delete sharing. */
    for (u64 h = 2; h < MAX_HANDLES; h++) {
        if (handles[h].fd == 0) continue;
        if (handles[h].file_id != file_id) continue;
        if (!(handles[h].share & FILE_SHARE_DELETE)) {
            set_error(ERROR_SHARING_VIOLATION);
            return w2m_logged(W2M_API_DeleteFileW, name, 0);
        }
    }

    int host_err = file_unlink_host(path_buf, plen);
    if (host_err != 0) {
        /* If any handle is open with share delete, POSIX delete should have
         * succeeded; if it failed, assume access denied. */
        set_error(ERROR_ACCESS_DENIED);
        return w2m_logged(W2M_API_DeleteFileW, name, 0);
    }

    set_error(ERROR_SUCCESS);
    return w2m_logged(W2M_API_DeleteFileW, name, 1);
}

u64 k32_ReadFile(u64 handle, u64 buffer, u64 count, u64 bytes_read, u64 rsp) {
    u64 overlapped = *(u64 *)(rsp + 0x28);
    (void)overlapped;

    if (!handle_ok(handle)) {
        set_error(ERROR_INVALID_HANDLE);
        return w2m_logged(W2M_API_ReadFile, handle, 0);
    }

    if (!(handles[handle].access & GENERIC_READ)) {
        set_error(ERROR_ACCESS_DENIED);
        return w2m_logged(W2M_API_ReadFile, handle, 0);
    }

    u64 off = handles[handle].position;
    u64 got = file_read_host(handles[handle].fd, buffer, count, off);
    handles[handle].position += got;
    *(u32 *)bytes_read = (u32)got;
    set_error(ERROR_SUCCESS);
    return w2m_logged(W2M_API_ReadFile, handle, 1);
}

u64 k32_SetFilePointerEx(u64 handle, u64 distance, u64 new_pointer, u64 move_method) {
    if (!handle_ok(handle)) {
        set_error(ERROR_INVALID_HANDLE);
        return w2m_logged(W2M_API_SetFilePointerEx, handle, 0);
    }

    i64 dist = (i64)distance;
    i64 base = 0;
    i64 size = (i64)file_size_host(handles[handle].fd);

    switch (move_method) {
    case FILE_BEGIN:
        base = 0;
        break;
    case FILE_CURRENT:
        base = (i64)handles[handle].position;
        break;
    case FILE_END:
        base = size;
        break;
    default:
        set_error(ERROR_INVALID_PARAMETER);
        return w2m_logged(W2M_API_SetFilePointerEx, handle, 0);
    }

    i64 pos = base + dist;
    if (pos < 0) {
        if (new_pointer) *(i64 *)new_pointer = -7;
        set_error(ERROR_NEGATIVE_SEEK);
        return w2m_logged(W2M_API_SetFilePointerEx, handle, 0);
    }

    handles[handle].position = (u64)pos;
    if (new_pointer) *(i64 *)new_pointer = pos;
    set_error(ERROR_SUCCESS);
    return w2m_logged(W2M_API_SetFilePointerEx, handle, 1);
}

u64 w2m_file_write(u64 handle, u64 buffer, u64 count, u64 written) {
    if (!handle_ok(handle)) {
        set_error(ERROR_INVALID_HANDLE);
        return 0;
    }

    if (!(handles[handle].access & GENERIC_WRITE)) {
        set_error(ERROR_ACCESS_DENIED);
        return 0;
    }

    u64 off = handles[handle].position;
    u64 done = file_write_host(handles[handle].fd, buffer, count, off);
    handles[handle].position += done;
    *(u32 *)written = (u32)done;
    set_error(ERROR_SUCCESS);
    return 1;
}
