/* file.c — w2m guest runtime file system (Sprint 042). */
#include "runtime.h"

typedef unsigned short u16;
typedef long long s64;

#define INVALID_HANDLE_VALUE ((u64)(-1))
#define STDOUT_HANDLE 0x74

/* Windows constants used by the probe. */
#define GENERIC_READ  0x80000000u
#define GENERIC_WRITE 0x40000000u
#define FILE_SHARE_READ   1
#define FILE_SHARE_WRITE  2
#define FILE_SHARE_DELETE 4
#define FILE_ATTRIBUTE_NORMAL 0x80
#define CREATE_NEW        1
#define CREATE_ALWAYS     2
#define OPEN_EXISTING     3
#define OPEN_ALWAYS       4
#define TRUNCATE_EXISTING 5
#define FILE_BEGIN   0
#define FILE_CURRENT 1
#define FILE_END     2

/* Windows error codes. */
#define ERROR_FILE_NOT_FOUND    2
#define ERROR_PATH_NOT_FOUND    3
#define ERROR_ACCESS_DENIED     5
#define ERROR_INVALID_HANDLE    6
#define ERROR_INVALID_PARAMETER 87
#define ERROR_FILE_EXISTS       80
#define ERROR_ALREADY_EXISTS    183
#define ERROR_NEGATIVE_SEEK     131
#define ERROR_SHARING_VIOLATION 32

/* Handle table. */
#define MAX_HANDLES 128
struct handle {
    u64 fd;
    u64 file_id;
    u32 access;
    u32 share;
    u64 position;
    u8 used;
    u8 deleted; /* file was unlinked while handle open */
};
static struct handle handles[MAX_HANDLES];

static void set_error(u32 code) {
    w2m_teb()->last_error_value = code;
}

static u64 make_handle(struct handle *h) {
    u64 idx = (u64)(h - handles) + 1;
    /* Avoid colliding with the stdout pseudo-handle 0x74. */
    if (idx == STDOUT_HANDLE) {
        if (MAX_HANDLES > STDOUT_HANDLE) {
            struct handle *alt = &handles[STDOUT_HANDLE];
            if (!alt->used) {
                *alt = *h;
                h->used = 0;
                return STDOUT_HANDLE + 1;
            }
        }
    }
    return idx;
}

static struct handle *lookup(u64 h) {
    if (h == 0 || h > MAX_HANDLES) return 0;
    struct handle *p = &handles[h - 1];
    if (!p->used) return 0;
    return p;
}

static u64 utf16le_len(const u16 *s) {
    u64 n = 0;
    while (s[n]) n++;
    return n;
}

/* Validate a UTF-16LE path and return the number of code units to hand to the
 * host.  The host does ASCII case-folding and component splitting; we only
 * need to strip trailing spaces/dots from the final component, like NTFS. */
static u64 prepare_path(const u16 *src, u64 chars, const u16 **out_path) {
    if (chars == 0) return (u64)(-1);

    /* Reject embedded NULs and non-ASCII characters (outside this scenario). */
    for (u64 i = 0; i < chars; i++) {
        if (src[i] == 0) return (u64)(-1);
        if (src[i] > 0x7f) return (u64)(-1);
    }

    /* Trim trailing spaces and dots from the final component. */
    u64 end = chars;
    while (end > 0 && (src[end - 1] == ' ' || src[end - 1] == '.'))
        end--;
    /* A name that was only dots/spaces refers to the directory ("."). */
    if (end == 0) end = 1;

    *out_path = src;
    return end;
}

static int path_is_absolute(const u16 *src, u64 chars) {
    if (chars >= 2 && src[1] == ':') return 1;
    if (chars >= 1 && (src[0] == '\\' || src[0] == '/')) return 1;
    return 0;
}

static int path_has_dotdot(const u16 *src, u64 chars) {
    u64 i = 0;
    while (i < chars) {
        u64 start = i;
        while (i < chars && src[i] != '\\' && src[i] != '/') i++;
        u64 len = i - start;
        if (len == 2 && src[start] == '.' && src[start + 1] == '.')
            return 1;
        if (i < chars) i++;
    }
    return 0;
}

static u64 lookup_kind(const u16 *name, u64 chars, u64 *file_id) {
    u64 second = 0;
    u64 kind = w2m_host(W2M_FILE_LOOKUP, (u64)name, chars, 0, 0, &second);
    if (file_id) *file_id = second;
    return kind;
}

static u64 host_open(const u16 *name, u64 chars, int create, u64 *file_id) {
    u64 second = 0;
    u64 fd = w2m_host(W2M_FILE_OPEN, (u64)name, chars, (u64)create, 0, &second);
    if (file_id) *file_id = second;
    return fd;
}

static u64 host_unlink(const u16 *name, u64 chars) {
    return w2m_host(W2M_FILE_UNLINK, (u64)name, chars, 0, 0, 0);
}

static struct handle *alloc_handle(void) {
    for (int i = 0; i < MAX_HANDLES; i++) {
        if (!handles[i].used) {
            handles[i].used = 1;
            handles[i].fd = (u64)(-1);
            handles[i].file_id = 0;
            handles[i].access = 0;
            handles[i].share = 0;
            handles[i].position = 0;
            handles[i].deleted = 0;
            return &handles[i];
        }
    }
    return 0;
}

/* Windows sharing check: compare the requested access/share against the
 * access/share of every existing open handle on the same file. */
static int sharing_conflict(u32 new_access, u32 new_share,
                            u32 existing_access, u32 existing_share) {
    if ((new_access & GENERIC_READ) && !(existing_share & FILE_SHARE_READ))
        return 1;
    if ((new_access & GENERIC_WRITE) && !(existing_share & FILE_SHARE_WRITE))
        return 1;
    if ((existing_access & GENERIC_READ) && !(new_share & FILE_SHARE_READ))
        return 1;
    if ((existing_access & GENERIC_WRITE) && !(new_share & FILE_SHARE_WRITE))
        return 1;
    return 0;
}

u64 k32_CreateFileW(u64 name, u64 desired_access, u64 share_mode,
                    u64 security_attributes, u64 rsp) {
    (void)security_attributes;
    u32 creation_disposition = *(u32 *)(rsp + 0x28);
    u32 flags_and_attributes = *(u32 *)(rsp + 0x30);
    u64 template = *(u64 *)(rsp + 0x38);
    (void)flags_and_attributes;
    (void)template;

    const u16 *path = (const u16 *)name;
    u64 chars = utf16le_len(path);

    /* Validate disposition. */
    if (creation_disposition < CREATE_NEW || creation_disposition > TRUNCATE_EXISTING) {
        set_error(ERROR_INVALID_PARAMETER);
        return w2m_logged(W2M_API_CreateFileW, name, INVALID_HANDLE_VALUE);
    }

    /* Empty path. */
    if (chars == 0) {
        set_error(ERROR_PATH_NOT_FOUND);
        return w2m_logged(W2M_API_CreateFileW, name, INVALID_HANDLE_VALUE);
    }

    /* Absolute paths and drive letters are outside this scenario. */
    if (path_is_absolute(path, chars)) {
        w2m_unimplemented(W2M_API_CreateFileW, 1);
        return INVALID_HANDLE_VALUE;
    }

    /* Components that leave the root are not supported. */
    if (path_has_dotdot(path, chars)) {
        w2m_unimplemented(W2M_API_CreateFileW, 2);
        return INVALID_HANDLE_VALUE;
    }

    const u16 *host_path = 0;
    u64 host_chars = prepare_path(path, chars, &host_path);
    if (host_chars == (u64)(-1)) {
        set_error(ERROR_PATH_NOT_FOUND);
        return w2m_logged(W2M_API_CreateFileW, name, INVALID_HANDLE_VALUE);
    }

    u64 file_id = 0;
    u64 kind = lookup_kind(host_path, host_chars, &file_id);
    if (kind == (u64)(-1)) {
        set_error(ERROR_PATH_NOT_FOUND);
        return w2m_logged(W2M_API_CreateFileW, name, INVALID_HANDLE_VALUE);
    }

    int must_exist = (creation_disposition == OPEN_EXISTING ||
                      creation_disposition == TRUNCATE_EXISTING);
    int must_not_exist = (creation_disposition == CREATE_NEW);
    int truncate = (creation_disposition == TRUNCATE_EXISTING ||
                    creation_disposition == CREATE_ALWAYS);

    if (kind == 2) {
        /* Directory: only allowed with FILE_FLAG_BACKUP_SEMANTICS, which the
         * probe does not use for ".". */
        set_error(ERROR_ACCESS_DENIED);
        return w2m_logged(W2M_API_CreateFileW, name, INVALID_HANDLE_VALUE);
    }

    if (kind == 3) {
        set_error(ERROR_PATH_NOT_FOUND);
        return w2m_logged(W2M_API_CreateFileW, name, INVALID_HANDLE_VALUE);
    }

    int exists = (kind == 1);

    if (!exists && must_exist) {
        set_error(ERROR_FILE_NOT_FOUND);
        return w2m_logged(W2M_API_CreateFileW, name, INVALID_HANDLE_VALUE);
    }

    if (exists && must_not_exist) {
        set_error(ERROR_FILE_EXISTS);
        return w2m_logged(W2M_API_CreateFileW, name, INVALID_HANDLE_VALUE);
    }

    /* TRUNCATE_EXISTING requires write access. */
    if (exists && creation_disposition == TRUNCATE_EXISTING &&
        !(desired_access & GENERIC_WRITE)) {
        set_error(ERROR_INVALID_PARAMETER);
        return w2m_logged(W2M_API_CreateFileW, name, INVALID_HANDLE_VALUE);
    }

    /* Check sharing against existing open handles. */
    for (int i = 0; i < MAX_HANDLES; i++) {
        if (!handles[i].used) continue;
        if (handles[i].file_id != file_id) continue;
        if (handles[i].deleted) continue;
        if (sharing_conflict((u32)desired_access, (u32)share_mode,
                             handles[i].access, handles[i].share)) {
            set_error(ERROR_SHARING_VIOLATION);
            return w2m_logged(W2M_API_CreateFileW, name, INVALID_HANDLE_VALUE);
        }
    }

    /* Open/create on host. create=1 means O_CREAT|O_EXCL. */
    int create_arg = (!exists && (creation_disposition == CREATE_NEW ||
                                   creation_disposition == CREATE_ALWAYS ||
                                   creation_disposition == OPEN_ALWAYS)) ? 1 : 0;
    u64 new_file_id = 0;
    u64 fd = host_open(host_path, host_chars, create_arg, &new_file_id);
    if ((s64)fd < 0) {
        s64 err = (s64)fd;
        if (err == -17) set_error(ERROR_FILE_EXISTS);
        else if (err == -2) set_error(ERROR_FILE_NOT_FOUND);
        else if (err == -13) set_error(ERROR_ACCESS_DENIED);
        else set_error(ERROR_ACCESS_DENIED);
        return w2m_logged(W2M_API_CreateFileW, name, INVALID_HANDLE_VALUE);
    }

    struct handle *h = alloc_handle();
    if (!h) {
        w2m_host(W2M_FILE_CLOSE, fd, 0, 0, 0, 0);
        set_error(ERROR_ACCESS_DENIED);
        return w2m_logged(W2M_API_CreateFileW, name, INVALID_HANDLE_VALUE);
    }

    h->fd = fd;
    h->file_id = new_file_id ? new_file_id : file_id;
    h->access = (u32)desired_access;
    h->share = (u32)share_mode;
    h->position = 0;
    h->deleted = 0;

    if (exists && truncate) {
        w2m_host(W2M_FILE_TRUNCATE, fd, 0, 0, 0, 0);
    }

    u64 ret = make_handle(h);
    u32 errcode = 0;
    if (exists && (creation_disposition == CREATE_ALWAYS ||
                   creation_disposition == OPEN_ALWAYS)) {
        errcode = ERROR_ALREADY_EXISTS;
    }
    set_error(errcode);
    return w2m_logged(W2M_API_CreateFileW, name, ret);
}

u64 k32_CloseHandle(u64 handle) {
    struct handle *h = lookup(handle);
    if (!h) {
        set_error(ERROR_INVALID_HANDLE);
        return w2m_logged(W2M_API_CloseHandle, handle, 0);
    }
    w2m_host(W2M_FILE_CLOSE, h->fd, 0, 0, 0, 0);
    h->used = 0;
    set_error(0);
    return w2m_logged(W2M_API_CloseHandle, handle, 1);
}

u64 k32_DeleteFileW(u64 name) {
    const u16 *path = (const u16 *)name;
    u64 chars = utf16le_len(path);

    if (chars == 0) {
        set_error(ERROR_PATH_NOT_FOUND);
        return w2m_logged(W2M_API_DeleteFileW, name, 0);
    }

    if (path_is_absolute(path, chars)) {
        w2m_unimplemented(W2M_API_DeleteFileW, 1);
        return 0;
    }
    if (path_has_dotdot(path, chars)) {
        w2m_unimplemented(W2M_API_DeleteFileW, 2);
        return 0;
    }

    const u16 *host_path = 0;
    u64 host_chars = prepare_path(path, chars, &host_path);
    if (host_chars == (u64)(-1)) {
        set_error(ERROR_PATH_NOT_FOUND);
        return w2m_logged(W2M_API_DeleteFileW, name, 0);
    }

    u64 file_id = 0;
    u64 kind = lookup_kind(host_path, host_chars, &file_id);
    if (kind == (u64)(-1) || kind == 0) {
        set_error(ERROR_FILE_NOT_FOUND);
        return w2m_logged(W2M_API_DeleteFileW, name, 0);
    }
    if (kind == 2) {
        set_error(ERROR_ACCESS_DENIED);
        return w2m_logged(W2M_API_DeleteFileW, name, 0);
    }

    /* Check for open handles without FILE_SHARE_DELETE. */
    for (int i = 0; i < MAX_HANDLES; i++) {
        if (!handles[i].used) continue;
        if (handles[i].file_id != file_id) continue;
        if (handles[i].deleted) continue;
        if (!(handles[i].share & FILE_SHARE_DELETE)) {
            set_error(ERROR_SHARING_VIOLATION);
            return w2m_logged(W2M_API_DeleteFileW, name, 0);
        }
    }

    /* POSIX delete: remove immediately. */
    u64 ret = host_unlink(host_path, host_chars);
    if ((s64)ret < 0) {
        s64 err = (s64)ret;
        if (err == -2) set_error(ERROR_FILE_NOT_FOUND);
        else if (err == -13 || err == -30) set_error(ERROR_ACCESS_DENIED);
        else if (err == -26) set_error(ERROR_SHARING_VIOLATION);
        else set_error(ERROR_ACCESS_DENIED);
        return w2m_logged(W2M_API_DeleteFileW, name, 0);
    }

    /* Mark open handles as deleted. */
    for (int i = 0; i < MAX_HANDLES; i++) {
        if (handles[i].used && handles[i].file_id == file_id)
            handles[i].deleted = 1;
    }

    set_error(0xDEAD); /* untouched sentinel */
    return w2m_logged(W2M_API_DeleteFileW, name, 1);
}

u64 k32_ReadFile(u64 handle, u64 buffer, u64 count, u64 bytes_read, u64 rsp) {
    (void)rsp;
    struct handle *h = lookup(handle);
    if (!h) {
        set_error(ERROR_INVALID_HANDLE);
        *(u32 *)bytes_read = 0;
        return w2m_logged(W2M_API_ReadFile, handle, 0);
    }
    if (!(h->access & GENERIC_READ)) {
        set_error(ERROR_ACCESS_DENIED);
        *(u32 *)bytes_read = 0;
        return w2m_logged(W2M_API_ReadFile, handle, 0);
    }
    if (count == 0) {
        *(u32 *)bytes_read = 0;
        set_error(0);
        return w2m_logged(W2M_API_ReadFile, handle, 1);
    }
    u64 second = 0;
    u64 n = w2m_host(W2M_FILE_READ, h->fd, buffer, count, h->position, &second);
    if ((s64)n < 0) {
        *(u32 *)bytes_read = 0;
        set_error(ERROR_ACCESS_DENIED);
        return w2m_logged(W2M_API_ReadFile, handle, 0);
    }
    *(u32 *)bytes_read = (u32)n;
    h->position += n;
    /* Windows leaves last-error untouched on a successful ReadFile. */
    return w2m_logged(W2M_API_ReadFile, handle, 1);
}

u64 k32_SetFilePointerEx(u64 handle, u64 distance, u64 new_pointer, u64 move_method) {
    struct handle *h = lookup(handle);
    if (!h) {
        set_error(ERROR_INVALID_HANDLE);
        if (new_pointer) *(u64 *)new_pointer = 0;
        return w2m_logged(W2M_API_SetFilePointerEx, handle, 0);
    }

    s64 dist = (s64)distance;
    s64 pos = (s64)h->position;
    if (move_method == FILE_BEGIN) {
        pos = dist;
    } else if (move_method == FILE_CURRENT) {
        pos += dist;
    } else if (move_method == FILE_END) {
        u64 sz = w2m_host(W2M_FILE_SIZE, h->fd, 0, 0, 0, 0);
        pos = (s64)sz + dist;
    } else {
        set_error(ERROR_INVALID_PARAMETER);
        if (new_pointer) *(u64 *)new_pointer = 0;
        return w2m_logged(W2M_API_SetFilePointerEx, handle, 0);
    }

    if (pos < 0) {
        set_error(ERROR_NEGATIVE_SEEK);
        if (new_pointer) *(u64 *)new_pointer = (u64)(-7);
        return w2m_logged(W2M_API_SetFilePointerEx, handle, 0);
    }

    h->position = (u64)pos;
    if (new_pointer) *(u64 *)new_pointer = (u64)pos;
    /* Windows leaves last-error untouched on a successful SetFilePointerEx. */
    return w2m_logged(W2M_API_SetFilePointerEx, handle, 1);
}

u64 w2m_file_write(u64 handle, u64 buffer, u64 count, u64 written) {
    struct handle *h = lookup(handle);
    if (!h) {
        set_error(ERROR_INVALID_HANDLE);
        *(u32 *)written = 0;
        return 0;
    }
    if (!(h->access & GENERIC_WRITE)) {
        set_error(ERROR_ACCESS_DENIED);
        *(u32 *)written = 0;
        return 0;
    }
    if (count == 0) {
        *(u32 *)written = 0;
        set_error(0);
        return 1;
    }
    u64 second = 0;
    u64 n = w2m_host(W2M_FILE_WRITE, h->fd, buffer, count, h->position, &second);
    if ((s64)n < 0 || n != count) {
        *(u32 *)written = (u32)(n > 0 ? n : 0);
        set_error(ERROR_ACCESS_DENIED);
        return 0;
    }
    *(u32 *)written = (u32)n;
    h->position += n;
    /* Windows WriteFile leaves last-error untouched on success. */
    return 1;
}
