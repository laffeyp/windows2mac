/* tools/guest_runtime/file.c — Windows file API for the w2m guest runtime. */

#include "runtime.h"

typedef unsigned short u16;
typedef long long s64;

#define INVALID_HANDLE_VALUE ((u64)(s64)-1)
#define STDOUT_HANDLE 0x74ull

/* Windows error codes used here. */
#define ERROR_SUCCESS 0
#define ERROR_FILE_NOT_FOUND 2
#define ERROR_PATH_NOT_FOUND 3
#define ERROR_ACCESS_DENIED 5
#define ERROR_INVALID_HANDLE 6
#define ERROR_NEGATIVE_SEEK 131
#define ERROR_ALREADY_EXISTS 183
#define ERROR_FILE_EXISTS 80
#define ERROR_SHARING_VIOLATION 32
#define ERROR_INVALID_PARAMETER 87
#define ERROR_TOO_MANY_OPEN_FILES 4

/* Windows constants. */
#define GENERIC_READ 0x80000000ull
#define GENERIC_WRITE 0x40000000ull
#define FILE_SHARE_READ 1
#define FILE_SHARE_WRITE 2
#define FILE_SHARE_DELETE 4
#define FILE_ATTRIBUTE_NORMAL 0x80
#define FILE_FLAG_BACKUP_SEMANTICS 0x02000000

#define CREATE_NEW 1
#define CREATE_ALWAYS 2
#define OPEN_EXISTING 3
#define OPEN_ALWAYS 4
#define TRUNCATE_EXISTING 5

#define FILE_BEGIN 0
#define FILE_CURRENT 1
#define FILE_END 2

#define MAX_HANDLES 256
#define MAX_PATH_CHARS 260

struct file_handle {
    u64 fd;          /* host descriptor, or 0 if free */
    u64 file_id;     /* host inode for sharing checks */
    u32 access;      /* desired access bits */
    u32 share;       /* share mode */
    u64 position;    /* current file pointer */
    int delete_pending; /* DeleteFileW was called while open with share delete */
};

static struct file_handle handles[MAX_HANDLES];

static void set_error(u32 code) {
    w2m_teb()->last_error_value = code;
}

static u32 get_error(void) {
    return w2m_teb()->last_error_value;
}

static u64 host_lookup(const u16 *name, u64 chars, u64 *file_id);
static u64 host_open(const u16 *name, u64 chars, int create, u64 *file_id);
static u64 host_unlink(const u16 *name, u64 chars);
static int path_ok(const u16 *name, u64 chars, char *out, u64 *out_len);

static u64 alloc_handle(void) {
    for (u64 i = 1; i < MAX_HANDLES; i++) {
        if (handles[i].fd == 0) {
            handles[i].fd = 1; /* reserve briefly */
            return i;
        }
    }
    return 0;
}

static void free_handle(u64 h) {
    if (h > 0 && h < MAX_HANDLES) {
        handles[h].fd = 0;
        handles[h].file_id = 0;
        handles[h].access = 0;
        handles[h].share = 0;
        handles[h].position = 0;
        handles[h].delete_pending = 0;
    }
}

static struct file_handle *get_handle(u64 h) {
    if (h == 0 || h >= MAX_HANDLES) return 0;
    if (handles[h].fd == 0 || handles[h].fd == 1) return 0;
    return &handles[h];
}

/* Convert UTF-16LE to ASCII, validate path, and case-fold ASCII letters.
   Returns 1 on success, 0 on unsupported path (calls unimplemented). */
static int path_ok(const u16 *name, u64 chars, char *out, u64 *out_len) {
    if (chars == 0) {
        set_error(ERROR_PATH_NOT_FOUND);
        return 0;
    }
    if (chars > MAX_PATH_CHARS) {
        w2m_unimplemented(W2M_API_CreateFileW, 1);
        return 0;
    }
    u64 j = 0;
    int first = 1;
    for (u64 i = 0; i < chars; i++) {
        u16 c = name[i];
        if (c == 0) break;
        if (c == '\\' || c == '/') {
            if (j == 0) {
                /* leading separator: absolute or drive-relative, unsupported */
                w2m_unimplemented(W2M_API_CreateFileW, 2);
                return 0;
            }
            if (out[j - 1] == '/') {
                /* empty component */
                w2m_unimplemented(W2M_API_CreateFileW, 3);
                return 0;
            }
            out[j++] = '/';
            first = 1;
            continue;
        }
        if (c == ':') {
            w2m_unimplemented(W2M_API_CreateFileW, 4);
            return 0;
        }
        if (c == '.') {
            if (first) {
                /* component starting with . : check for . or .. */
                if (i + 1 == chars || name[i + 1] == '\\' || name[i + 1] == '/' || name[i + 1] == 0) {
                    /* "." component */
                    w2m_unimplemented(W2M_API_CreateFileW, 5);
                    return 0;
                }
                if (name[i + 1] == '.' &&
                    (i + 2 == chars || name[i + 2] == '\\' || name[i + 2] == '/' || name[i + 2] == 0)) {
                    /* ".." component */
                    w2m_unimplemented(W2M_API_CreateFileW, 6);
                    return 0;
                }
            }
        }
        if (c > 0x7f) {
            w2m_unimplemented(W2M_API_CreateFileW, 7);
            return 0;
        }
        char ch = (char)c;
        if (ch >= 'A' && ch <= 'Z') ch = (char)(ch - 'A' + 'a');
        out[j++] = ch;
        first = 0;
    }
    /* Windows strips trailing spaces and dots from the final component. */
    while (j > 0 && (out[j - 1] == ' ' || out[j - 1] == '.')) j--;
    if (j == 0) {
        set_error(ERROR_PATH_NOT_FOUND);
        return 0;
    }
    out[j] = 0;
    *out_len = j;
    return 1;
}

static u64 host_lookup(const u16 *name, u64 chars, u64 *file_id) {
    char path[MAX_PATH_CHARS + 1];
    u64 len = 0;
    if (!path_ok(name, chars, path, &len)) {
        return (u64)(s64)-1;
    }
    u64 second = 0;
    u64 r = w2m_host(W2M_FILE_LOOKUP, (u64)path, len, 0, 0, &second);
    if (file_id) *file_id = second;
    return r;
}

static u64 host_open(const u16 *name, u64 chars, int create, u64 *file_id) {
    char path[MAX_PATH_CHARS + 1];
    u64 len = 0;
    if (!path_ok(name, chars, path, &len)) {
        return (u64)(s64)-1;
    }
    u64 second = 0;
    u64 r = w2m_host(W2M_FILE_OPEN, (u64)path, len, create ? 1 : 0, 0, &second);
    if (file_id) *file_id = second;
    return r;
}

static u64 host_unlink(const u16 *name, u64 chars) {
    char path[MAX_PATH_CHARS + 1];
    u64 len = 0;
    if (!path_ok(name, chars, path, &len)) {
        return (u64)(s64)-1;
    }
    u64 second = 0;
    return w2m_host(W2M_FILE_UNLINK, (u64)path, len, 0, 0, &second);
}

/* Check whether a new open with given access/share conflicts with an existing open of the same file. */
static int sharing_conflict(u64 file_id, u32 new_access, u32 new_share, u64 skip_handle) {
    for (u64 i = 1; i < MAX_HANDLES; i++) {
        if (i == skip_handle) continue;
        if (handles[i].fd == 0 || handles[i].fd == 1) continue;
        if (handles[i].file_id != file_id) continue;
        if (handles[i].delete_pending) return 1;

        u32 existing_access = handles[i].access;
        u32 existing_share = handles[i].share;

        /* New access must be allowed by existing share. */
        if ((new_access & GENERIC_READ) && !(existing_share & FILE_SHARE_READ)) return 1;
        if ((new_access & GENERIC_WRITE) && !(existing_share & FILE_SHARE_WRITE)) return 1;

        /* Existing access must be allowed by new share. */
        if ((existing_access & GENERIC_READ) && !(new_share & FILE_SHARE_READ)) return 1;
        if ((existing_access & GENERIC_WRITE) && !(new_share & FILE_SHARE_WRITE)) return 1;
    }
    return 0;
}

/* Map host errno (negative) to Windows error. */
static u32 errno_to_win(u64 err) {
    s64 e = (s64)err;
    if (e >= 0) return ERROR_SUCCESS;
    e = -e;
    switch ((int)e) {
        case 2: return ERROR_FILE_NOT_FOUND;
        case 13: return ERROR_ACCESS_DENIED;
        case 17: return ERROR_FILE_EXISTS;
        case 24: return ERROR_TOO_MANY_OPEN_FILES;
        case 20: return ERROR_FILE_NOT_FOUND;
        case 21: return ERROR_PATH_NOT_FOUND;
        default: return ERROR_ACCESS_DENIED;
    }
}

u64 k32_CreateFileW(u64 name, u64 desired_access, u64 share_mode, u64 security_attributes, u64 rsp) {
    (void)security_attributes;
    u32 creation_disposition = *(u32 *)(rsp + 0x28);
    u32 flags_and_attributes = *(u32 *)(rsp + 0x30);
    u64 template = *(u64 *)(rsp + 0x38);

    if (template != 0) {
        w2m_unimplemented(W2M_API_CreateFileW, 8);
        return INVALID_HANDLE_VALUE;
    }
    if (security_attributes != 0) {
        w2m_unimplemented(W2M_API_CreateFileW, 9);
        return INVALID_HANDLE_VALUE;
    }

    /* Validate access. */
    if ((desired_access & ~(GENERIC_READ | GENERIC_WRITE)) != 0) {
        w2m_unimplemented(W2M_API_CreateFileW, 10);
        return INVALID_HANDLE_VALUE;
    }

    /* Validate disposition. */
    if (creation_disposition == 0 || creation_disposition > 5) {
        set_error(ERROR_INVALID_PARAMETER);
        return w2m_logged(W2M_API_CreateFileW, name, INVALID_HANDLE_VALUE);
    }

    /* Validate share. */
    if (share_mode & ~(FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE)) {
        w2m_unimplemented(W2M_API_CreateFileW, 11);
        return INVALID_HANDLE_VALUE;
    }

    u64 chars = 0;
    while (((u16 *)name)[chars] != 0 && chars < 0x10000) chars++;

    u64 file_id = 0;
    u64 kind = host_lookup((const u16 *)name, chars, &file_id);

    if (kind == (u64)-1) {
        /* path_ok already set error or called unimplemented */
        return w2m_logged(W2M_API_CreateFileW, name, INVALID_HANDLE_VALUE);
    }

    int is_dir = (kind == 2);
    int exists = (kind == 1 || kind == 2);

    /* Opening "." directory without backup semantics fails with access denied. */
    if (is_dir) {
        if (!(flags_and_attributes & FILE_FLAG_BACKUP_SEMANTICS)) {
            set_error(ERROR_ACCESS_DENIED);
            return w2m_logged(W2M_API_CreateFileW, name, INVALID_HANDLE_VALUE);
        }
        /* Directory open not otherwise needed by probe; treat as unimplemented. */
        w2m_unimplemented(W2M_API_CreateFileW, 12);
        return INVALID_HANDLE_VALUE;
    }

    u64 h = alloc_handle();
    if (h == 0) {
        set_error(ERROR_TOO_MANY_OPEN_FILES);
        return w2m_logged(W2M_API_CreateFileW, name, INVALID_HANDLE_VALUE);
    }

    u64 fd = (u64)(s64)-1;
    u32 last_err = ERROR_SUCCESS;
    int created = 0;

    switch (creation_disposition) {
        case CREATE_NEW:
            if (exists) {
                last_err = ERROR_FILE_EXISTS;
                break;
            }
            fd = host_open((const u16 *)name, chars, 1, &file_id);
            if ((s64)fd < 0) {
                last_err = errno_to_win(fd);
            } else {
                created = 1;
            }
            break;
        case CREATE_ALWAYS:
            if (exists) {
                /* truncate existing */
                fd = host_open((const u16 *)name, chars, 0, &file_id);
                if ((s64)fd < 0) {
                    last_err = errno_to_win(fd);
                } else {
                    u64 tr = w2m_host(W2M_FILE_TRUNCATE, fd, 0, 0, 0, 0);
                    if ((s64)tr < 0) {
                        w2m_host(W2M_FILE_CLOSE, fd, 0, 0, 0, 0);
                        fd = (u64)(s64)-1;
                        last_err = errno_to_win(tr);
                    } else {
                        last_err = ERROR_ALREADY_EXISTS;
                    }
                }
            } else {
                fd = host_open((const u16 *)name, chars, 1, &file_id);
                if ((s64)fd < 0) {
                    last_err = errno_to_win(fd);
                } else {
                    created = 1;
                }
            }
            break;
        case OPEN_EXISTING:
            if (!exists) {
                last_err = (kind == 3) ? ERROR_PATH_NOT_FOUND : ERROR_FILE_NOT_FOUND;
                break;
            }
            fd = host_open((const u16 *)name, chars, 0, &file_id);
            if ((s64)fd < 0) last_err = errno_to_win(fd);
            break;
        case OPEN_ALWAYS:
            if (exists) {
                fd = host_open((const u16 *)name, chars, 0, &file_id);
                if ((s64)fd < 0) {
                    last_err = errno_to_win(fd);
                } else {
                    last_err = ERROR_ALREADY_EXISTS;
                }
            } else {
                fd = host_open((const u16 *)name, chars, 1, &file_id);
                if ((s64)fd < 0) {
                    last_err = errno_to_win(fd);
                } else {
                    created = 1;
                }
            }
            break;
        case TRUNCATE_EXISTING:
            if (!exists) {
                last_err = (kind == 3) ? ERROR_PATH_NOT_FOUND : ERROR_FILE_NOT_FOUND;
                break;
            }
            if (!(desired_access & GENERIC_WRITE)) {
                last_err = ERROR_INVALID_PARAMETER;
                break;
            }
            fd = host_open((const u16 *)name, chars, 0, &file_id);
            if ((s64)fd < 0) {
                last_err = errno_to_win(fd);
            } else {
                u64 tr = w2m_host(W2M_FILE_TRUNCATE, fd, 0, 0, 0, 0);
                if ((s64)tr < 0) {
                    w2m_host(W2M_FILE_CLOSE, fd, 0, 0, 0, 0);
                    fd = (u64)(s64)-1;
                    last_err = errno_to_win(tr);
                }
            }
            break;
    }

    if ((s64)fd < 0) {
        free_handle(h);
        set_error(last_err);
        return w2m_logged(W2M_API_CreateFileW, name, INVALID_HANDLE_VALUE);
    }

    /* Check sharing. */
    if (sharing_conflict(file_id, (u32)desired_access, (u32)share_mode, h)) {
        w2m_host(W2M_FILE_CLOSE, fd, 0, 0, 0, 0);
        free_handle(h);
        set_error(ERROR_SHARING_VIOLATION);
        return w2m_logged(W2M_API_CreateFileW, name, INVALID_HANDLE_VALUE);
    }

    handles[h].fd = fd;
    handles[h].file_id = file_id;
    handles[h].access = (u32)desired_access;
    handles[h].share = (u32)share_mode;
    handles[h].position = 0;
    handles[h].delete_pending = 0;

    set_error(last_err);
    return w2m_logged(W2M_API_CreateFileW, name, h);
}

u64 k32_CloseHandle(u64 handle) {
    if (handle == STDOUT_HANDLE) {
        return w2m_logged(W2M_API_CloseHandle, handle, 1);
    }
    struct file_handle *fh = get_handle(handle);
    if (!fh) {
        set_error(ERROR_INVALID_HANDLE);
        return w2m_logged(W2M_API_CloseHandle, handle, 0);
    }
    w2m_host(W2M_FILE_CLOSE, fh->fd, 0, 0, 0, 0);
    free_handle(handle);
    set_error(ERROR_SUCCESS);
    return w2m_logged(W2M_API_CloseHandle, handle, 1);
}

u64 k32_DeleteFileW(u64 name) {
    u64 chars = 0;
    while (((u16 *)name)[chars] != 0 && chars < 0x10000) chars++;

    u64 file_id = 0;
    u64 kind = host_lookup((const u16 *)name, chars, &file_id);

    if (kind == (u64)-1) {
        return w2m_logged(W2M_API_DeleteFileW, name, 0);
    }

    if (kind == 0) {
        set_error(ERROR_FILE_NOT_FOUND);
        return w2m_logged(W2M_API_DeleteFileW, name, 0);
    }
    if (kind == 2) {
        set_error(ERROR_ACCESS_DENIED);
        return w2m_logged(W2M_API_DeleteFileW, name, 0);
    }

    /* Check for open handles without FILE_SHARE_DELETE. */
    int open_count = 0;
    int can_delete = 1;
    for (u64 i = 1; i < MAX_HANDLES; i++) {
        if (handles[i].fd == 0 || handles[i].fd == 1) continue;
        if (handles[i].file_id != file_id) continue;
        open_count++;
        if (!(handles[i].share & FILE_SHARE_DELETE)) {
            can_delete = 0;
        }
    }

    if (open_count > 0 && !can_delete) {
        set_error(ERROR_SHARING_VIOLATION);
        return w2m_logged(W2M_API_DeleteFileW, name, 0);
    }

    u64 r = host_unlink((const u16 *)name, chars);
    if ((s64)r < 0) {
        set_error(errno_to_win(r));
        return w2m_logged(W2M_API_DeleteFileW, name, 0);
    }

    /* Mark open handles as delete-pending so later opens fail. */
    for (u64 i = 1; i < MAX_HANDLES; i++) {
        if (handles[i].fd == 0 || handles[i].fd == 1) continue;
        if (handles[i].file_id == file_id) {
            handles[i].delete_pending = 1;
        }
    }

    set_error(ERROR_SUCCESS);
    return w2m_logged(W2M_API_DeleteFileW, name, 1);
}

u64 k32_ReadFile(u64 handle, u64 buffer, u64 count, u64 bytes_read, u64 rsp) {
    u64 overlapped = *(u64 *)(rsp + 0x28);
    if (overlapped != 0) {
        w2m_unimplemented(W2M_API_ReadFile, 1);
        return 0;
    }
    if (bytes_read == 0) {
        w2m_unimplemented(W2M_API_ReadFile, 2);
        return 0;
    }

    struct file_handle *fh = get_handle(handle);
    if (!fh) {
        *(u32 *)bytes_read = 0;
        set_error(ERROR_INVALID_HANDLE);
        return w2m_logged(W2M_API_ReadFile, handle, 0);
    }
    if (!(fh->access & GENERIC_READ)) {
        *(u32 *)bytes_read = 0;
        set_error(ERROR_ACCESS_DENIED);
        return w2m_logged(W2M_API_ReadFile, handle, 0);
    }

    u64 n = w2m_host(W2M_FILE_READ, fh->fd, buffer, count, fh->position, 0);
    if ((s64)n < 0) {
        *(u32 *)bytes_read = 0;
        set_error(errno_to_win(n));
        return w2m_logged(W2M_API_ReadFile, handle, 0);
    }
    *(u32 *)bytes_read = (u32)n;
    fh->position += n;
    set_error(ERROR_SUCCESS);
    return w2m_logged(W2M_API_ReadFile, handle, 1);
}

u64 k32_SetFilePointerEx(u64 handle, u64 distance, u64 new_pointer, u64 move_method) {
    struct file_handle *fh = get_handle(handle);
    if (!fh) {
        if (new_pointer) *(u64 *)new_pointer = 0;
        set_error(ERROR_INVALID_HANDLE);
        return w2m_logged(W2M_API_SetFilePointerEx, handle, 0);
    }

    s64 dist = (s64)distance;
    u64 size = w2m_host(W2M_FILE_SIZE, fh->fd, 0, 0, 0, 0);
    u64 pos = 0;

    switch (move_method) {
        case FILE_BEGIN:
            if (dist < 0) {
                if (new_pointer) *(u64 *)new_pointer = (u64)-1;
                set_error(ERROR_NEGATIVE_SEEK);
                return w2m_logged(W2M_API_SetFilePointerEx, handle, 0);
            }
            pos = (u64)dist;
            break;
        case FILE_CURRENT:
            if (dist < 0 && (u64)-dist > fh->position) {
                if (new_pointer) *(u64 *)new_pointer = (u64)-1;
                set_error(ERROR_NEGATIVE_SEEK);
                return w2m_logged(W2M_API_SetFilePointerEx, handle, 0);
            }
            pos = fh->position + (u64)dist;
            break;
        case FILE_END:
            if (dist < 0 && (u64)-dist > size) {
                if (new_pointer) *(u64 *)new_pointer = (u64)-1;
                set_error(ERROR_NEGATIVE_SEEK);
                return w2m_logged(W2M_API_SetFilePointerEx, handle, 0);
            }
            pos = size + (u64)dist;
            break;
        default:
            if (new_pointer) *(u64 *)new_pointer = (u64)-1;
            set_error(ERROR_INVALID_PARAMETER);
            return w2m_logged(W2M_API_SetFilePointerEx, handle, 0);
    }

    fh->position = pos;
    if (new_pointer) *(u64 *)new_pointer = pos;
    set_error(ERROR_SUCCESS);
    return w2m_logged(W2M_API_SetFilePointerEx, handle, 1);
}

u64 w2m_file_write(u64 handle, u64 buffer, u64 count, u64 written) {
    struct file_handle *fh = get_handle(handle);
    if (!fh) {
        set_error(ERROR_INVALID_HANDLE);
        return 0;
    }
    if (!(fh->access & GENERIC_WRITE)) {
        *(u32 *)written = 0;
        set_error(ERROR_ACCESS_DENIED);
        return 0;
    }

    u64 n = w2m_host(W2M_FILE_WRITE, fh->fd, buffer, count, fh->position, 0);
    if ((s64)n < 0) {
        *(u32 *)written = 0;
        set_error(errno_to_win(n));
        return 0;
    }
    *(u32 *)written = (u32)n;
    fh->position += n;
    set_error(ERROR_SUCCESS);
    return 1;
}
