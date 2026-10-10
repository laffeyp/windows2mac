/* tools/guest_runtime/file.c — Windows file API for the w2m guest runtime. */

#include "runtime.h"

/* Windows constants used by the probe. */
#define GENERIC_READ        0x80000000u
#define GENERIC_WRITE       0x40000000u
#define FILE_SHARE_READ     0x00000001u
#define FILE_SHARE_WRITE    0x00000002u
#define FILE_SHARE_DELETE   0x00000004u
#define FILE_ATTRIBUTE_NORMAL 0x00000080u
#define CREATE_NEW          1u
#define CREATE_ALWAYS       2u
#define OPEN_EXISTING       3u
#define OPEN_ALWAYS         4u
#define TRUNCATE_EXISTING   5u
#define INVALID_HANDLE_VALUE ((u64)(-1))
#define ERROR_FILE_NOT_FOUND 2u
#define ERROR_PATH_NOT_FOUND 3u
#define ERROR_ACCESS_DENIED  5u
#define ERROR_INVALID_HANDLE 6u
#define ERROR_INVALID_PARAMETER 87u
#define ERROR_FILE_EXISTS    80u
#define ERROR_ALREADY_EXISTS 183u
#define ERROR_SHARING_VIOLATION 32u
#define ERROR_NEGATIVE_SEEK 131u

/* Handle table. Index 0 is reserved; stdout is handled in runtime.c and never reaches here. */
#define MAX_HANDLES 64

struct file_handle {
    u64 fd;          /* host descriptor, 0 if free */
    u64 file_id;     /* inode from host */
    u32 access;      /* desired access the handle was opened with */
    u32 share;       /* share mode the handle was opened with */
    u64 position;    /* current file pointer */
};

static struct file_handle handles[MAX_HANDLES];

static void set_error(u32 code) {
    w2m_teb()->last_error_value = code;
}

static u64 make_handle(int idx) {
    /* Avoid colliding with the magic stdout handle 0x74 used by runtime.c. */
    u64 h = (u64)(idx + 1);
    if (h == 0x74) h = 0x75;
    return h;
}

static int handle_index(u64 h) {
    if (h == 0x74) return -1;
    if (h == 0x75) return 0x74 - 1;
    if (h == 0 || h > MAX_HANDLES) return -1;
    return (int)h - 1;
}

static int alloc_handle(void) {
    for (int i = 0; i < MAX_HANDLES; i++) {
        if (handles[i].fd == 0) {
            handles[i].fd = (u64)(-1);
            return i;
        }
    }
    return -1;
}

static void free_handle(int idx) {
    handles[idx].fd = 0;
    handles[idx].file_id = 0;
    handles[idx].access = 0;
    handles[idx].share = 0;
    handles[idx].position = 0;
}

/* Convert a UTF-16LE path to a narrow path in a fixed buffer, performing ASCII case-folding.
 * Returns the length (excluding terminator) or -1 if the path is unsupported. */
static int normalize_path(const u16 *src, char *dst, int dst_cap) {
    int o = 0;
    int i = 0;
    for (;;) {
        u16 c = src[i++];
        if (c == 0) {
            if (o == 0) {
                /* Empty path: keep it empty; caller will reject. */
                if (o < dst_cap) dst[o] = 0;
                return 0;
            }
            if (o < dst_cap) dst[o] = 0;
            return o;
        }
        if (c == '\\' || c == '/') {
            /* Treat directory separators uniformly. */
            if (o > 0 && dst[o - 1] == '/') continue; /* collapse multiple separators */
            if (o >= dst_cap - 1) return -1;
            dst[o++] = '/';
            continue;
        }
        if (c == '.') {
            /* Strip trailing dot/space at end of final component. */
            int start = o;
            while (o < dst_cap - 1 && src[i] && src[i] != '\\' && src[i] != '/') {
                u16 d = src[i++];
                if (d >= 'A' && d <= 'Z') d += 32;
                if (d == 0) break;
                dst[o++] = (char)d;
            }
            /* Trailing dot/space removal: if component ends with . or space, drop it. */
            while (o > start && (dst[o - 1] == '.' || dst[o - 1] == ' ')) o--;
            continue;
        }
        if (c >= 'A' && c <= 'Z') c += 32;
        if (c == 0) continue;
        if (o >= dst_cap - 1) return -1;
        dst[o++] = (char)c;
    }
}

/* Validate that the normalized path stays within the root directory. */
static int path_ok(const char *p) {
    if (p[0] == '/') return 0;          /* absolute or drive-letter path */
    int depth = 0;
    for (int i = 0; p[i];) {
        int j = i;
        while (p[j] && p[j] != '/') j++;
        int len = j - i;
        if (len == 2 && p[i] == '.' && p[i + 1] == '.') {
            depth--;
            if (depth < 0) return 0;   /* leaves root */
        } else if (!(len == 1 && p[i] == '.')) {
            depth++;
        }
        if (p[j] == '/') j++;
        i = j;
    }
    return 1;
}

/* Convert a guest UTF-16LE pointer to a validated narrow path. */
static int get_path(u64 name, char *buf, int cap) {
    const u16 *p = (const u16 *)name;
    int len = normalize_path(p, buf, cap);
    if (len < 0) return -1;
    if (!path_ok(buf)) return -1;
    return len;
}

/* Map host errno to Windows error code. */
static u32 errno_to_win(u64 err) {
    switch ((long long)err) {
        case -2: return ERROR_FILE_NOT_FOUND;
        case -13: return ERROR_ACCESS_DENIED;
        case -17: return ERROR_FILE_EXISTS;
        case -20: return ERROR_PATH_NOT_FOUND;
        case -21: return ERROR_PATH_NOT_FOUND;
        default: return ERROR_ACCESS_DENIED;
    }
}

/* Check whether two open handles conflict. Returns 1 if conflict, 0 if ok. */
static int share_conflict(const struct file_handle *existing, u32 new_access, u32 new_share) {
    /* Existing read needs new to not request write unless new shares write. */
    if ((existing->access & GENERIC_READ) && (new_access & GENERIC_WRITE) && !(new_share & FILE_SHARE_WRITE))
        return 1;
    /* Existing write needs new to not request anything unless new shares appropriately. */
    if ((existing->access & GENERIC_WRITE)) {
        if ((new_access & GENERIC_READ) && !(new_share & FILE_SHARE_READ)) return 1;
        if ((new_access & GENERIC_WRITE) && !(new_share & FILE_SHARE_WRITE)) return 1;
    }
    /* New read needs existing to not hold write unless existing shares write. */
    if ((new_access & GENERIC_READ) && (existing->access & GENERIC_WRITE) && !(existing->share & FILE_SHARE_READ))
        return 1;
    /* New write needs existing to not hold read unless existing shares write,
     * and not hold write unless existing shares write. */
    if ((new_access & GENERIC_WRITE)) {
        if ((existing->access & GENERIC_READ) && !(existing->share & FILE_SHARE_WRITE)) return 1;
        if ((existing->access & GENERIC_WRITE) && !(existing->share & FILE_SHARE_WRITE)) return 1;
    }
    return 0;
}

/* Find any open handle to the same file_id that conflicts with the requested access/share. */
static int find_conflict(u64 file_id, u32 access, u32 share) {
    for (int i = 0; i < MAX_HANDLES; i++) {
        if (handles[i].fd != 0 && handles[i].fd != (u64)(-1) && handles[i].file_id == file_id) {
            if (share_conflict(&handles[i], access, share)) return 1;
        }
    }
    return 0;
}

u64 k32_CreateFileW(u64 name, u64 desired_access, u64 share_mode, u64 security_attributes,
                    u64 rsp) {
    (void)security_attributes;
    u32 creation_disposition = *(u32 *)(rsp + 0x28);
    u32 flags_and_attributes = *(u32 *)(rsp + 0x30);
    u64 template = *(u64 *)(rsp + 0x38);

    if (template != 0) {
        w2m_unimplemented(W2M_API_CreateFileW, 1);
        return INVALID_HANDLE_VALUE;
    }
    if (security_attributes != 0) {
        w2m_unimplemented(W2M_API_CreateFileW, 2);
        return INVALID_HANDLE_VALUE;
    }
    if (flags_and_attributes != FILE_ATTRIBUTE_NORMAL) {
        w2m_unimplemented(W2M_API_CreateFileW, 3);
        return INVALID_HANDLE_VALUE;
    }

    /* Validate disposition. */
    if (creation_disposition == 0 || creation_disposition > 5) {
        set_error(ERROR_INVALID_PARAMETER);
        return w2m_logged(W2M_API_CreateFileW, name, INVALID_HANDLE_VALUE);
    }

    /* TRUNCATE_EXISTING requires GENERIC_WRITE. */
    if (creation_disposition == TRUNCATE_EXISTING && !(desired_access & GENERIC_WRITE)) {
        set_error(ERROR_INVALID_PARAMETER);
        return w2m_logged(W2M_API_CreateFileW, name, INVALID_HANDLE_VALUE);
    }

    char path[512];
    int plen = get_path(name, path, sizeof(path));
    if (plen < 0) {
        set_error(ERROR_PATH_NOT_FOUND);
        return w2m_logged(W2M_API_CreateFileW, name, INVALID_HANDLE_VALUE);
    }
    if (plen == 0) {
        set_error(ERROR_PATH_NOT_FOUND);
        return w2m_logged(W2M_API_CreateFileW, name, INVALID_HANDLE_VALUE);
    }

    /* Look up the path. */
    u64 second = 0;
    u64 kind = w2m_host(W2M_FILE_LOOKUP, (u64)path, (u64)(plen + 1), 0, 0, &second);
    u64 file_id = second;

    /* Opening "." as directory without FILE_FLAG_BACKUP_SEMANTICS is access denied. */
    if (plen == 1 && path[0] == '.') {
        set_error(ERROR_ACCESS_DENIED);
        return w2m_logged(W2M_API_CreateFileW, name, INVALID_HANDLE_VALUE);
    }

    int must_create = 0;
    int must_exist = 0;
    int truncate = 0;
    int set_already_exists = 0;

    switch (creation_disposition) {
        case CREATE_NEW:
            if (kind == 1 || kind == 2) {
                set_error(ERROR_FILE_EXISTS);
                return w2m_logged(W2M_API_CreateFileW, name, INVALID_HANDLE_VALUE);
            }
            must_create = 1;
            break;
        case CREATE_ALWAYS:
            if (kind == 1 || kind == 2) {
                set_already_exists = 1;
                truncate = 1;
            } else {
                must_create = 1;
            }
            break;
        case OPEN_EXISTING:
            if (kind != 1 && kind != 2) {
                if (kind == 3) set_error(ERROR_PATH_NOT_FOUND);
                else set_error(ERROR_FILE_NOT_FOUND);
                return w2m_logged(W2M_API_CreateFileW, name, INVALID_HANDLE_VALUE);
            }
            must_exist = 1;
            break;
        case OPEN_ALWAYS:
            if (kind == 1 || kind == 2) {
                set_already_exists = 1;
                must_exist = 1;
            } else {
                must_create = 1;
            }
            break;
        case TRUNCATE_EXISTING:
            if (kind != 1 && kind != 2) {
                if (kind == 3) set_error(ERROR_PATH_NOT_FOUND);
                else set_error(ERROR_FILE_NOT_FOUND);
                return w2m_logged(W2M_API_CreateFileW, name, INVALID_HANDLE_VALUE);
            }
            truncate = 1;
            must_exist = 1;
            break;
    }

    /* If opening existing directory as a file, treat as file not found. */
    if (must_exist && kind == 2) {
        set_error(ERROR_FILE_NOT_FOUND);
        return w2m_logged(W2M_API_CreateFileW, name, INVALID_HANDLE_VALUE);
    }

    /* Check sharing conflicts against already-open handles. */
    if (must_exist && find_conflict(file_id, (u32)desired_access, (u32)share_mode)) {
        set_error(ERROR_SHARING_VIOLATION);
        return w2m_logged(W2M_API_CreateFileW, name, INVALID_HANDLE_VALUE);
    }

    /* Open or create on the host. */
    u64 create_arg = must_create ? 1 : 0;
    u64 fd = w2m_host(W2M_FILE_OPEN, (u64)path, (u64)(plen + 1), create_arg, 0, &second);
    if ((long long)fd < 0) {
        set_error(errno_to_win(fd));
        return w2m_logged(W2M_API_CreateFileW, name, INVALID_HANDLE_VALUE);
    }
    file_id = second;

    /* If we are creating but a file already appeared, handle disposition semantics. */
    if (must_create && kind == 0 && set_already_exists == 0) {
        /* Created successfully. */
    } else if (must_create && (kind == 1 || kind == 2)) {
        /* Race: should have been caught by lookup, but treat as exists. */
        w2m_host(W2M_FILE_CLOSE, fd, 0, 0, 0, &second);
        set_error(ERROR_FILE_EXISTS);
        return w2m_logged(W2M_API_CreateFileW, name, INVALID_HANDLE_VALUE);
    }

    /* Truncate if needed. */
    if (truncate) {
        u64 tr = w2m_host(W2M_FILE_TRUNCATE, fd, 0, 0, 0, &second);
        if (tr != 0) {
            w2m_host(W2M_FILE_CLOSE, fd, 0, 0, 0, &second);
            set_error(ERROR_ACCESS_DENIED);
            return w2m_logged(W2M_API_CreateFileW, name, INVALID_HANDLE_VALUE);
        }
    }

    int idx = alloc_handle();
    if (idx < 0) {
        w2m_host(W2M_FILE_CLOSE, fd, 0, 0, 0, &second);
        set_error(ERROR_ACCESS_DENIED);
        return w2m_logged(W2M_API_CreateFileW, name, INVALID_HANDLE_VALUE);
    }

    handles[idx].fd = fd;
    handles[idx].file_id = file_id;
    handles[idx].access = (u32)desired_access;
    handles[idx].share = (u32)share_mode;
    handles[idx].position = 0;

    if (set_already_exists) set_error(ERROR_ALREADY_EXISTS);
    else set_error(0);

    return w2m_logged(W2M_API_CreateFileW, name, make_handle(idx));
}

u64 k32_CloseHandle(u64 handle) {
    int idx = handle_index(handle);
    if (idx < 0 || handles[idx].fd == 0) {
        set_error(ERROR_INVALID_HANDLE);
        return w2m_logged(W2M_API_CloseHandle, handle, 0);
    }
    u64 second = 0;
    w2m_host(W2M_FILE_CLOSE, handles[idx].fd, 0, 0, 0, &second);
    free_handle(idx);
    set_error(0);
    return w2m_logged(W2M_API_CloseHandle, handle, 1);
}

u64 k32_DeleteFileW(u64 name) {
    char path[512];
    int plen = get_path(name, path, sizeof(path));
    if (plen < 0) {
        set_error(ERROR_PATH_NOT_FOUND);
        return w2m_logged(W2M_API_DeleteFileW, name, 0);
    }
    if (plen == 0) {
        set_error(ERROR_PATH_NOT_FOUND);
        return w2m_logged(W2M_API_DeleteFileW, name, 0);
    }

    u64 second = 0;
    u64 kind = w2m_host(W2M_FILE_LOOKUP, (u64)path, (u64)(plen + 1), 0, 0, &second);
    if (kind != 1) {
        if (kind == 2) set_error(ERROR_ACCESS_DENIED);
        else if (kind == 3) set_error(ERROR_PATH_NOT_FOUND);
        else set_error(ERROR_FILE_NOT_FOUND);
        return w2m_logged(W2M_API_DeleteFileW, name, 0);
    }
    u64 file_id = second;

    /* If any open handle to this file does not share delete, fail. */
    for (int i = 0; i < MAX_HANDLES; i++) {
        if (handles[i].fd != 0 && handles[i].fd != (u64)(-1) && handles[i].file_id == file_id) {
            if (!(handles[i].share & FILE_SHARE_DELETE)) {
                set_error(ERROR_SHARING_VIOLATION);
                return w2m_logged(W2M_API_DeleteFileW, name, 0);
            }
        }
    }

    u64 ret = w2m_host(W2M_FILE_UNLINK, (u64)path, (u64)(plen + 1), 0, 0, &second);
    if ((long long)ret < 0) {
        set_error(errno_to_win(ret));
        return w2m_logged(W2M_API_DeleteFileW, name, 0);
    }
    set_error(0);
    return w2m_logged(W2M_API_DeleteFileW, name, 1);
}

u64 k32_ReadFile(u64 handle, u64 buffer, u64 count, u64 bytes_read, u64 rsp) {
    u64 overlapped = *(u64 *)(rsp + 0x28);
    if (overlapped != 0) {
        w2m_unimplemented(W2M_API_ReadFile, 1);
        return 0;
    }
    if (bytes_read == 0) {
        set_error(ERROR_INVALID_PARAMETER);
        return w2m_logged(W2M_API_ReadFile, handle, 0);
    }

    int idx = handle_index(handle);
    if (idx < 0 || handles[idx].fd == 0) {
        set_error(ERROR_INVALID_HANDLE);
        return w2m_logged(W2M_API_ReadFile, handle, 0);
    }
    if (!(handles[idx].access & GENERIC_READ)) {
        set_error(ERROR_ACCESS_DENIED);
        return w2m_logged(W2M_API_ReadFile, handle, 0);
    }

    u64 second = 0;
    u64 n = w2m_host(W2M_FILE_READ, handles[idx].fd, buffer, count, handles[idx].position, &second);
    if ((long long)n < 0) {
        set_error(ERROR_ACCESS_DENIED);
        *(u32 *)bytes_read = 0;
        return w2m_logged(W2M_API_ReadFile, handle, 0);
    }
    *(u32 *)bytes_read = (u32)n;
    handles[idx].position += n;
    set_error(0);
    return w2m_logged(W2M_API_ReadFile, handle, 1);
}

u64 k32_SetFilePointerEx(u64 handle, u64 distance, u64 new_pointer, u64 move_method) {
    int idx = handle_index(handle);
    if (idx < 0 || handles[idx].fd == 0) {
        set_error(ERROR_INVALID_HANDLE);
        if (new_pointer) *(u64 *)new_pointer = 0;
        return w2m_logged(W2M_API_SetFilePointerEx, handle, 0);
    }

    u64 second = 0;
    u64 size = w2m_host(W2M_FILE_SIZE, handles[idx].fd, 0, 0, 0, &second);
    if ((long long)size < 0) size = 0;

    long long dist = (long long)distance;
    u64 pos;
    switch (move_method) {
        case 0: /* FILE_BEGIN */
            pos = (u64)dist;
            break;
        case 1: /* FILE_CURRENT */
            pos = (u64)((long long)handles[idx].position + dist);
            break;
        case 2: /* FILE_END */
            pos = (u64)((long long)size + dist);
            break;
        default:
            set_error(ERROR_INVALID_PARAMETER);
            if (new_pointer) *(u64 *)new_pointer = 0;
            return w2m_logged(W2M_API_SetFilePointerEx, handle, 0);
    }

    if ((long long)pos < 0) {
        set_error(ERROR_NEGATIVE_SEEK);
        if (new_pointer) *(u64 *)new_pointer = (u64)(-7);
        return w2m_logged(W2M_API_SetFilePointerEx, handle, 0);
    }

    handles[idx].position = pos;
    if (new_pointer) *(u64 *)new_pointer = pos;
    set_error(0);
    return w2m_logged(W2M_API_SetFilePointerEx, handle, 1);
}

u64 w2m_file_write(u64 handle, u64 buffer, u64 count, u64 written) {
    int idx = handle_index(handle);
    if (idx < 0 || handles[idx].fd == 0) {
        set_error(ERROR_INVALID_HANDLE);
        *(u32 *)written = 0;
        return 0;
    }
    if (!(handles[idx].access & GENERIC_WRITE)) {
        set_error(ERROR_ACCESS_DENIED);
        *(u32 *)written = 0;
        return 0;
    }

    u64 second = 0;
    u64 n = w2m_host(W2M_FILE_WRITE, handles[idx].fd, buffer, count, handles[idx].position, &second);
    if ((long long)n < 0) {
        set_error(ERROR_ACCESS_DENIED);
        *(u32 *)written = 0;
        return 0;
    }
    *(u32 *)written = (u32)n;
    handles[idx].position += n;
    set_error(0);
    return 1;
}
