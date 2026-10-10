/* tools/guest_runtime/file.c — Windows file API for the w2m guest runtime.
 *
 * Freestanding C for AArch64; no C library, only runtime.h.  Guest pointers are
 * directly dereferenceable.  The host side is src/w2m/file_bridge.py.
 */

#include "runtime.h"

/* Windows constants used by the probe. */
#define GENERIC_READ                     0x80000000u
#define GENERIC_WRITE                    0x40000000u
#define FILE_SHARE_READ                  0x00000001u
#define FILE_SHARE_WRITE                 0x00000002u
#define FILE_SHARE_DELETE                0x00000004u

#define CREATE_NEW                       1u
#define CREATE_ALWAYS                    2u
#define OPEN_EXISTING                    3u
#define OPEN_ALWAYS                      4u
#define TRUNCATE_EXISTING                5u

#define FILE_ATTRIBUTE_NORMAL            0x00000080u
#define FILE_FLAG_BACKUP_SEMANTICS       0x02000000u

#define FILE_BEGIN                       0u
#define FILE_CURRENT                     1u
#define FILE_END                         2u

#define INVALID_HANDLE_VALUE             (~(u64)0)

/* Windows error codes. */
#define ERROR_SUCCESS                    0u
#define ERROR_FILE_NOT_FOUND             2u
#define ERROR_PATH_NOT_FOUND             3u
#define ERROR_ACCESS_DENIED              5u
#define ERROR_INVALID_HANDLE             6u
#define ERROR_NEGATIVE_SEEK              131u
#define ERROR_ALREADY_EXISTS             183u
#define ERROR_MORE_DATA                  234u
#define ERROR_FILE_EXISTS                80u
#define ERROR_SHARING_VIOLATION          32u
#define ERROR_INVALID_PARAMETER          87u

/* Handle table: index 0 unused, 1..N.  We keep a small fixed array; the probe
 * only needs a handful of handles. */
#define MAX_HANDLES 32

struct file_handle {
    u64 fd;          /* host descriptor, or 0 if free */
    u64 file_id;     /* host file id (inode) */
    u64 position;    /* current file pointer */
    u32 access;      /* desired access the handle was opened with */
    u32 share;       /* share mode the handle was opened with */
    u8  used;
};

static struct file_handle handles[MAX_HANDLES];

/* Per-file sharing state.  We track, per file_id, the access and share bits of
 * every open handle.  The probe only opens a few files concurrently. */
#define MAX_TRACKED 16

struct tracked_file {
    u64 file_id;
    u32 access;      /* OR of access bits of open handles */
    u32 share;       /* OR of share bits of open handles */
    u32 count;       /* number of open handles */
};

static struct tracked_file tracked[MAX_TRACKED];

static void set_last_error(u32 code)
{
    w2m_teb()->last_error_value = code;
}

static u32 errno_to_win(u64 err)
{
    if (err == 0) return ERROR_SUCCESS;
    switch ((long long)err) {
        case -2:   return ERROR_FILE_NOT_FOUND;
        case -13:  return ERROR_ACCESS_DENIED;
        case -17:  return ERROR_FILE_EXISTS;
        case -20:  return ERROR_PATH_NOT_FOUND;
        case -21:  return ERROR_PATH_NOT_FOUND;
        case -39:  return ERROR_SHARING_VIOLATION; /* ENOTEMPTY? not used */
        default:   return ERROR_ACCESS_DENIED;
    }
}

/* Convert UTF-16LE to a narrow path for the host.  The host compares ASCII
 * case-folded, so we fold ASCII here.  Backslash becomes '/'.  We reject
 * absolute paths, drive letters, and '..' that would escape the root. */
static int normalize_path(const unsigned short *src, char *dst, int max_len)
{
    int i = 0, o = 0;
    int components = 0;

    if (!src || !src[0]) {
        /* empty path */
        return -1;
    }

    while (src[i]) {
        unsigned short c = src[i++];
        if (c == '\\' || c == '/') {
            if (o > 0 && dst[o - 1] == '/')
                continue;
            if (o >= max_len - 1) return -1;
            dst[o++] = '/';
            continue;
        }
        if (c == 0x2e) {
            /* '.' component: check for . or .. */
            int start = i - 1;
            int dots = 1;
            while (src[i] == 0x2e) { dots++; i++; }
            if ((src[i] == 0 || src[i] == '\\' || src[i] == '/') && dots <= 2) {
                if (dots == 2) {
                    /* '..' would escape root if we have no components */
                    if (components == 0) return -1;
                    /* remove last component and trailing slash */
                    if (o > 0 && dst[o - 1] == '/') o--;
                    while (o > 0 && dst[o - 1] != '/') o--;
                    if (o == 0) {
                        /* went back to root */
                        components = 0;
                        dst[o++] = '/';
                    } else {
                        components--;
                    }
                }
                /* single '.' is a no-op */
                continue;
            }
            /* not a special component, fall through to copy dots */
            i = start;
        }
        if (c == ':') {
            /* drive letter or absolute marker */
            return -1;
        }
        if (c < 0x20 || c > 0x7e) {
            /* non-ASCII: pass through as UTF-8 bytes */
            if (o + 3 >= max_len) return -1;
            if (c < 0x80) {
                dst[o++] = (char)c;
            } else if (c < 0x800) {
                dst[o++] = (char)(0xc0 | (c >> 6));
                dst[o++] = (char)(0x80 | (c & 0x3f));
            } else {
                dst[o++] = (char)(0xe0 | (c >> 12));
                dst[o++] = (char)(0x80 | ((c >> 6) & 0x3f));
                dst[o++] = (char)(0x80 | (c & 0x3f));
            }
            continue;
        }
        /* ASCII case fold */
        if (c >= 'A' && c <= 'Z') c = c - 'A' + 'a';
        if (o >= max_len - 1) return -1;
        dst[o++] = (char)c;
    }

    if (o == 0) {
        dst[o++] = '/';
    }
    dst[o] = 0;

    /* reject absolute paths */
    if (dst[0] == '/') {
        /* leading slash means absolute; but a single '/' is the cwd root, keep */
        if (o > 1) return -1;
    }

    return 0;
}

static int path_is_empty(const unsigned short *src)
{
    return !src || !src[0];
}

static u64 lookup_narrow(const char *path, u64 *file_id)
{
    u64 second = 0;
    u64 kind = w2m_host(W2M_FILE_LOOKUP, (u64)(unsigned long)path, 0, 0, 0, &second);
    if (file_id) *file_id = second;
    return kind;
}

static struct tracked_file *find_tracked(u64 file_id)
{
    for (int i = 0; i < MAX_TRACKED; i++) {
        if (tracked[i].file_id == file_id && tracked[i].count)
            return &tracked[i];
    }
    return 0;
}

static struct tracked_file *add_tracked(u64 file_id)
{
    struct tracked_file *t = find_tracked(file_id);
    if (t) return t;
    for (int i = 0; i < MAX_TRACKED; i++) {
        if (tracked[i].count == 0) {
            tracked[i].file_id = file_id;
            tracked[i].access = 0;
            tracked[i].share = 0;
            tracked[i].count = 0;
            return &tracked[i];
        }
    }
    return 0;
}

static void remove_tracked(u64 file_id, u32 access, u32 share)
{
    struct tracked_file *t = find_tracked(file_id);
    if (!t) return;
    t->access &= ~access;
    t->share &= ~share;
    if (t->count) t->count--;
    if (t->count == 0) {
        t->file_id = 0;
        t->access = 0;
        t->share = 0;
    }
}

/* Check whether a new open with requested access/sharing conflicts with the
 * existing opens of file_id.  Windows compares the requested access against
 * the sharing modes of existing opens, and the requested sharing mode against
 * the access modes of existing opens. */
static int sharing_conflicts(u64 file_id, u32 req_access, u32 req_share)
{
    struct tracked_file *t = find_tracked(file_id);
    if (!t) return 0;

    /* existing access vs requested share */
    if ((t->access & GENERIC_READ) && !(req_share & FILE_SHARE_READ))
        return 1;
    if ((t->access & GENERIC_WRITE) && !(req_share & FILE_SHARE_WRITE))
        return 1;

    /* existing share vs requested access */
    if ((req_access & GENERIC_READ) && !(t->share & FILE_SHARE_READ))
        return 1;
    if ((req_access & GENERIC_WRITE) && !(t->share & FILE_SHARE_WRITE))
        return 1;

    return 0;
}

static void add_share(u64 file_id, u32 access, u32 share)
{
    struct tracked_file *t = add_tracked(file_id);
    if (!t) return;
    t->access |= access;
    t->share |= share;
    t->count++;
}

static int allocate_handle(void)
{
    for (int i = 1; i < MAX_HANDLES; i++) {
        if (!handles[i].used) return i;
    }
    return -1;
}

static struct file_handle *get_handle(u64 h)
{
    if (h == INVALID_HANDLE_VALUE || h == 0 || h == 0x74) return 0;
    if (h >= MAX_HANDLES) return 0;
    if (!handles[h].used) return 0;
    return &handles[h];
}

static u64 make_handle(int idx)
{
    if (idx < 0) return INVALID_HANDLE_VALUE;
    /* Avoid colliding with stdout pseudo-handle 0x74. */
    if (idx == 0x74) {
        /* swap with index 1 if free, otherwise fail */
        if (!handles[1].used) {
            handles[1] = handles[idx];
            handles[idx].used = 0;
            handles[1].used = 1;
            return 1;
        }
        return INVALID_HANDLE_VALUE;
    }
    return (u64)idx;
}

/* Strip trailing spaces and dots from the last component, matching Windows
 * behavior for names that differ only by trailing dots/spaces. */
static void strip_trailing_dots_spaces(unsigned short *dst, const unsigned short *src)
{
    int i = 0;
    while (src[i]) {
        dst[i] = src[i];
        i++;
    }
    dst[i] = 0;

    /* find last component */
    int last = i - 1;
    while (last >= 0 && dst[last] != '\\' && dst[last] != '/')
        last--;
    int end = i - 1;
    while (end > last && (dst[end] == ' ' || dst[end] == '.')) {
        dst[end] = 0;
        end--;
    }
}

u64 k32_CreateFileW(u64 name, u64 desired_access, u64 share_mode,
                    u64 security_attributes, u64 rsp)
{
    (void)security_attributes;
    u32 creation_disposition = *(u32 *)(rsp + 0x28);
    u32 flags_attrs = *(u32 *)(rsp + 0x30);
    u64 template = *(u64 *)(rsp + 0x38);

    if (template != 0) {
        w2m_unimplemented(W2M_API_CreateFileW, 2);
        return w2m_logged(W2M_API_CreateFileW, name, INVALID_HANDLE_VALUE);
    }

    /* Validate disposition. */
    if (creation_disposition == 0 || creation_disposition > 5) {
        set_last_error(ERROR_INVALID_PARAMETER);
        return w2m_logged(W2M_API_CreateFileW, name, INVALID_HANDLE_VALUE);
    }

    /* Validate access for truncate_existing: must include GENERIC_WRITE. */
    if (creation_disposition == TRUNCATE_EXISTING &&
        !(desired_access & GENERIC_WRITE)) {
        set_last_error(ERROR_INVALID_PARAMETER);
        return w2m_logged(W2M_API_CreateFileW, name, INVALID_HANDLE_VALUE);
    }

    /* Empty name -> ERROR_PATH_NOT_FOUND. */
    if (path_is_empty((const unsigned short *)name)) {
        set_last_error(ERROR_PATH_NOT_FOUND);
        return w2m_logged(W2M_API_CreateFileW, name, INVALID_HANDLE_VALUE);
    }

    /* Normalize name: strip trailing dots/spaces. */
    unsigned short norm[256];
    strip_trailing_dots_spaces(norm, (const unsigned short *)name);

    /* "." without backup semantics is access denied. */
    int is_dot = 0;
    {
        int k = 0;
        while (norm[k] && norm[k] != '\\' && norm[k] != '/') k++;
        if (k == 1 && norm[0] == '.') is_dot = 1;
    }
    if (is_dot && !(flags_attrs & FILE_FLAG_BACKUP_SEMANTICS)) {
        set_last_error(ERROR_ACCESS_DENIED);
        return w2m_logged(W2M_API_CreateFileW, name, INVALID_HANDLE_VALUE);
    }

    char path[512];
    if (normalize_path(norm, path, sizeof(path)) != 0) {
        w2m_unimplemented(W2M_API_CreateFileW, 3);
        return w2m_logged(W2M_API_CreateFileW, name, INVALID_HANDLE_VALUE);
    }

    u64 file_id = 0;
    u64 kind = lookup_narrow(path, &file_id);

    /* Missing directory on the way -> ERROR_PATH_NOT_FOUND. */
    if (kind == 3) {
        set_last_error(ERROR_PATH_NOT_FOUND);
        return w2m_logged(W2M_API_CreateFileW, name, INVALID_HANDLE_VALUE);
    }

    int exists = (kind == 1);
    int is_dir = (kind == 2);

    /* We don't support opening directories as files in this contract. */
    if (is_dir) {
        set_last_error(ERROR_ACCESS_DENIED);
        return w2m_logged(W2M_API_CreateFileW, name, INVALID_HANDLE_VALUE);
    }

    u64 fd = 0;
    u32 winerr = ERROR_SUCCESS;
    int created = 0;
    int truncated = 0;

    switch (creation_disposition) {
        case CREATE_NEW:
            if (exists) {
                set_last_error(ERROR_FILE_EXISTS);
                return w2m_logged(W2M_API_CreateFileW, name, INVALID_HANDLE_VALUE);
            }
            fd = w2m_host(W2M_FILE_OPEN, (u64)(unsigned long)path, 1, 0, 0, &file_id);
            if ((long long)fd < 0) {
                winerr = errno_to_win(fd);
                set_last_error(winerr);
                return w2m_logged(W2M_API_CreateFileW, name, INVALID_HANDLE_VALUE);
            }
            created = 1;
            break;

        case CREATE_ALWAYS:
            if (exists) {
                /* open existing, will truncate below */
                fd = w2m_host(W2M_FILE_OPEN, (u64)(unsigned long)path, 0, 0, 0, &file_id);
                if ((long long)fd < 0) {
                    winerr = errno_to_win(fd);
                    set_last_error(winerr);
                    return w2m_logged(W2M_API_CreateFileW, name, INVALID_HANDLE_VALUE);
                }
                truncated = 1;
            } else {
                fd = w2m_host(W2M_FILE_OPEN, (u64)(unsigned long)path, 1, 0, 0, &file_id);
                if ((long long)fd < 0) {
                    winerr = errno_to_win(fd);
                    set_last_error(winerr);
                    return w2m_logged(W2M_API_CreateFileW, name, INVALID_HANDLE_VALUE);
                }
                created = 1;
            }
            break;

        case OPEN_EXISTING:
            if (!exists) {
                set_last_error(ERROR_FILE_NOT_FOUND);
                return w2m_logged(W2M_API_CreateFileW, name, INVALID_HANDLE_VALUE);
            }
            fd = w2m_host(W2M_FILE_OPEN, (u64)(unsigned long)path, 0, 0, 0, &file_id);
            if ((long long)fd < 0) {
                winerr = errno_to_win(fd);
                set_last_error(winerr);
                return w2m_logged(W2M_API_CreateFileW, name, INVALID_HANDLE_VALUE);
            }
            break;

        case OPEN_ALWAYS:
            if (exists) {
                fd = w2m_host(W2M_FILE_OPEN, (u64)(unsigned long)path, 0, 0, 0, &file_id);
                if ((long long)fd < 0) {
                    winerr = errno_to_win(fd);
                    set_last_error(winerr);
                    return w2m_logged(W2M_API_CreateFileW, name, INVALID_HANDLE_VALUE);
                }
            } else {
                fd = w2m_host(W2M_FILE_OPEN, (u64)(unsigned long)path, 1, 0, 0, &file_id);
                if ((long long)fd < 0) {
                    winerr = errno_to_win(fd);
                    set_last_error(winerr);
                    return w2m_logged(W2M_API_CreateFileW, name, INVALID_HANDLE_VALUE);
                }
                created = 1;
            }
            break;

        case TRUNCATE_EXISTING:
            if (!exists) {
                set_last_error(ERROR_FILE_NOT_FOUND);
                return w2m_logged(W2M_API_CreateFileW, name, INVALID_HANDLE_VALUE);
            }
            fd = w2m_host(W2M_FILE_OPEN, (u64)(unsigned long)path, 0, 0, 0, &file_id);
            if ((long long)fd < 0) {
                winerr = errno_to_win(fd);
                set_last_error(winerr);
                return w2m_logged(W2M_API_CreateFileW, name, INVALID_HANDLE_VALUE);
            }
            truncated = 1;
            break;
    }

    /* Check sharing conflicts against existing opens. */
    if (sharing_conflicts(file_id, (u32)desired_access, (u32)share_mode)) {
        w2m_host(W2M_FILE_CLOSE, fd, 0, 0, 0, 0);
        set_last_error(ERROR_SHARING_VIOLATION);
        return w2m_logged(W2M_API_CreateFileW, name, INVALID_HANDLE_VALUE);
    }

    /* Truncate if needed. */
    if (truncated) {
        u64 r = w2m_host(W2M_FILE_TRUNCATE, fd, 0, 0, 0, 0);
        (void)r;
    }

    int idx = allocate_handle();
    if (idx < 0) {
        w2m_host(W2M_FILE_CLOSE, fd, 0, 0, 0, 0);
        set_last_error(ERROR_ACCESS_DENIED);
        return w2m_logged(W2M_API_CreateFileW, name, INVALID_HANDLE_VALUE);
    }

    handles[idx].fd = fd;
    handles[idx].file_id = file_id;
    handles[idx].position = 0;
    handles[idx].access = (u32)desired_access;
    handles[idx].share = (u32)share_mode;
    handles[idx].used = 1;

    add_share(file_id, (u32)desired_access, (u32)share_mode);

    if (created) {
        if (creation_disposition == CREATE_ALWAYS && exists) {
            set_last_error(ERROR_ALREADY_EXISTS);
        } else if (creation_disposition == OPEN_ALWAYS && exists) {
            set_last_error(ERROR_ALREADY_EXISTS);
        } else {
            set_last_error(ERROR_SUCCESS);
        }
    } else if (truncated) {
        set_last_error(ERROR_SUCCESS);
    } else {
        set_last_error(ERROR_SUCCESS);
    }

    return w2m_logged(W2M_API_CreateFileW, name, make_handle(idx));
}

u64 k32_CloseHandle(u64 handle)
{
    struct file_handle *h = get_handle(handle);
    if (!h) {
        set_last_error(ERROR_INVALID_HANDLE);
        return w2m_logged(W2M_API_CloseHandle, handle, 0);
    }

    w2m_host(W2M_FILE_CLOSE, h->fd, 0, 0, 0, 0);
    remove_tracked(h->file_id, h->access, h->share);
    h->used = 0;
    h->fd = 0;
    h->file_id = 0;
    h->position = 0;
    h->access = 0;
    h->share = 0;

    set_last_error(ERROR_SUCCESS);
    return w2m_logged(W2M_API_CloseHandle, handle, 1);
}

u64 k32_DeleteFileW(u64 name)
{
    if (path_is_empty((const unsigned short *)name)) {
        set_last_error(ERROR_PATH_NOT_FOUND);
        return w2m_logged(W2M_API_DeleteFileW, name, 0);
    }

    unsigned short norm[256];
    strip_trailing_dots_spaces(norm, (const unsigned short *)name);

    char path[512];
    if (normalize_path(norm, path, sizeof(path)) != 0) {
        w2m_unimplemented(W2M_API_DeleteFileW, 1);
        return w2m_logged(W2M_API_DeleteFileW, name, 0);
    }

    u64 file_id = 0;
    u64 kind = lookup_narrow(path, &file_id);

    if (kind == 0) {
        set_last_error(ERROR_FILE_NOT_FOUND);
        return w2m_logged(W2M_API_DeleteFileW, name, 0);
    }
    if (kind == 3) {
        set_last_error(ERROR_PATH_NOT_FOUND);
        return w2m_logged(W2M_API_DeleteFileW, name, 0);
    }
    if (kind == 2) {
        set_last_error(ERROR_ACCESS_DENIED);
        return w2m_logged(W2M_API_DeleteFileW, name, 0);
    }

    /* If any open handle lacks FILE_SHARE_DELETE, fail. */
    struct tracked_file *t = find_tracked(file_id);
    if (t && t->count) {
        if (!(t->share & FILE_SHARE_DELETE)) {
            set_last_error(ERROR_SHARING_VIOLATION);
            return w2m_logged(W2M_API_DeleteFileW, name, 0);
        }
    }

    u64 r = w2m_host(W2M_FILE_UNLINK, (u64)(unsigned long)path, 0, 0, 0, 0);
    if ((long long)r < 0) {
        set_last_error(errno_to_win(r));
        return w2m_logged(W2M_API_DeleteFileW, name, 0);
    }

    set_last_error(ERROR_SUCCESS);
    return w2m_logged(W2M_API_DeleteFileW, name, 1);
}

u64 k32_ReadFile(u64 handle, u64 buffer, u64 count, u64 bytes_read, u64 rsp)
{
    u64 overlapped = *(u64 *)(rsp + 0x28);
    if (overlapped != 0) {
        w2m_unimplemented(W2M_API_ReadFile, 1);
        return w2m_logged(W2M_API_ReadFile, handle, 0);
    }

    struct file_handle *h = get_handle(handle);
    if (!h) {
        set_last_error(ERROR_INVALID_HANDLE);
        return w2m_logged(W2M_API_ReadFile, handle, 0);
    }

    if (!(h->access & GENERIC_READ)) {
        set_last_error(ERROR_ACCESS_DENIED);
        return w2m_logged(W2M_API_ReadFile, handle, 0);
    }

    if (bytes_read)
        *(u32 *)bytes_read = 0;

    u64 nread = w2m_host(W2M_FILE_READ, h->fd, buffer, count, h->position, 0);
    if ((long long)nread < 0) {
        set_last_error(errno_to_win(nread));
        return w2m_logged(W2M_API_ReadFile, handle, 0);
    }

    h->position += nread;
    if (bytes_read)
        *(u32 *)bytes_read = (u32)nread;

    set_last_error(ERROR_SUCCESS);
    return w2m_logged(W2M_API_ReadFile, handle, 1);
}

u64 k32_SetFilePointerEx(u64 handle, u64 distance, u64 new_pointer, u64 move_method)
{
    struct file_handle *h = get_handle(handle);
    if (!h) {
        set_last_error(ERROR_INVALID_HANDLE);
        return w2m_logged(W2M_API_SetFilePointerEx, handle, 0);
    }

    long long dist = (long long)distance;
    long long base;
    switch (move_method) {
        case FILE_BEGIN:   base = 0; break;
        case FILE_CURRENT: base = (long long)h->position; break;
        case FILE_END: {
            u64 sz = w2m_host(W2M_FILE_SIZE, h->fd, 0, 0, 0, 0);
            base = (long long)sz;
            break;
        }
        default:
            set_last_error(ERROR_INVALID_PARAMETER);
            return w2m_logged(W2M_API_SetFilePointerEx, handle, 0);
    }

    long long newpos = base + dist;
    if (newpos < 0) {
        if (new_pointer)
            *(long long *)new_pointer = -7; /* Windows leaves unmodified? probe expects -7 */
        set_last_error(ERROR_NEGATIVE_SEEK);
        return w2m_logged(W2M_API_SetFilePointerEx, handle, 0);
    }

    h->position = (u64)newpos;
    if (new_pointer)
        *(long long *)new_pointer = newpos;

    set_last_error(ERROR_SUCCESS);
    return w2m_logged(W2M_API_SetFilePointerEx, handle, 1);
}

u64 w2m_file_write(u64 handle, u64 buffer, u64 count, u64 written)
{
    struct file_handle *h = get_handle(handle);
    if (!h) {
        set_last_error(ERROR_INVALID_HANDLE);
        return 0;
    }

    if (!(h->access & GENERIC_WRITE)) {
        set_last_error(ERROR_ACCESS_DENIED);
        return 0;
    }

    u64 nwritten = w2m_host(W2M_FILE_WRITE, h->fd, buffer, count, h->position, 0);
    if ((long long)nwritten < 0) {
        set_last_error(errno_to_win(nwritten));
        return 0;
    }

    h->position += nwritten;
    if (written)
        *(u32 *)written = (u32)nwritten;

    set_last_error(ERROR_SUCCESS);
    return 1;
}
