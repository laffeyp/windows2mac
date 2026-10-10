```c
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
#define ERROR_MORE_DATA             234u
#define ERROR_FILE_EXISTS           80u
#define ERROR_SHARING_VIOLATION     32u
#define ERROR_INVALID_PARAMETER     87u

/* Handle table. Handle 0 is reserved (never issued); 0x74 is stdout and skipped. */
#define MAX_FILES 64

struct file_entry {
    u64 fd;          /* host descriptor, or 0 if free */
    u64 id;          /* file id (inode) from host */
    u32 access;      /* desired access we opened with */
    u32 share;       /* share mode we opened with */
    u64 position;    /* current file pointer */
};

static struct file_entry files[MAX_FILES];

static void set_error(u32 code)
{
    w2m_teb()->last_error_value = code;
}

static u64 make_handle(unsigned idx)
{
    u64 h = idx + 1;
    if (h == 0x74) {
        /* Skip the magic stdout handle value. */
        if (idx + 2 <= MAX_FILES)
            return idx + 2;
    }
    return h;
}

static unsigned handle_index(u64 h)
{
    if (h == 0x74)
        return (unsigned)-1;
    if (h == 0 || h > MAX_FILES + 1)
        return (unsigned)-1;
    return (unsigned)(h - 1);
}

static struct file_entry *lookup(u64 h)
{
    unsigned idx = handle_index(h);
    if (idx >= MAX_FILES)
        return 0;
    if (files[idx].fd == 0)
        return 0;
    return &files[idx];
}

/* UTF-16LE helpers. */
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

/* Validate and normalize a guest UTF-16LE path into an ASCII host path.
 * Returns 0 on success, fills `out` with a '/'-separated path relative to root.
 * Returns a Windows error code on failure.
 * `out` must have room for at least 512 bytes.
 *
 * Rules:
 * - empty path -> ERROR_PATH_NOT_FOUND
 * - absolute, drive letter, or ".." leaving root -> unimplemented (stops run)
 * - trailing spaces and dots are stripped per component
 * - components are ASCII case-folded to upper case
 * - separator is '/'
 */
static u32 normalize_path(u64 name, char *out)
{
    u16 buf[512];
    unsigned len = 0;
    unsigned i;

    if (name == 0) {
        set_error(ERROR_PATH_NOT_FOUND);
        return ERROR_PATH_NOT_FOUND;
    }

    /* Read the UTF-16LE string, checking for terminator. */
    for (i = 0; i < 511; i++) {
        u16 c = ((u16 *)name)[i];
        if (c == 0)
            break;
        buf[i] = c;
    }
    if (i == 511) {
        /* Path too long; treat as unimplemented. */
        w2m_unimplemented(W2M_API_CreateFileW, 1);
        return 0; /* not reached */
    }
    len = i;
    if (len == 0) {
        set_error(ERROR_PATH_NOT_FOUND);
        return ERROR_PATH_NOT_FOUND;
    }

    /* Reject absolute paths and drive letters. */
    if (is_sep(buf[0])) {
        w2m_unimplemented(W2M_API_CreateFileW, 2);
        return 0;
    }
    if (len >= 2 && buf[1] == ':') {
        w2m_unimplemented(W2M_API_CreateFileW, 3);
        return 0;
    }

    out[0] = 0;
    unsigned out_len = 0;
    unsigned pos = 0;

    while (pos < len) {
        /* Skip leading separators. */
        while (pos < len && is_sep(buf[pos]))
            pos++;
        if (pos >= len)
            break;

        /* Find end of component. */
        unsigned start = pos;
        while (pos < len && !is_sep(buf[pos]))
            pos++;

        /* Strip trailing spaces and dots. */
        unsigned end = pos;
        while (end > start) {
            u16 c = buf[end - 1];
            if (c == ' ' || c == '.')
                end--;
            else
                break;
        }

        if (end == start) {
            /* Component became empty after stripping; skip it. */
            continue;
        }

        /* Copy component, case-fold ASCII, check for "..". */
        char comp[512];
        unsigned cl = 0;
        int is_dotdot = 1;
        for (i = start; i < end; i++) {
            u16 c = buf[i];
            if (c > 0x7f) {
                /* Non-ASCII: not needed by contract, unimplemented. */
                w2m_unimplemented(W2M_API_CreateFileW, 4);
                return 0;
            }
            comp[cl] = (char)to_upper_ascii(c);
            if (cl < 2) {
                if (cl == 0 && comp[0] != '.') is_dotdot = 0;
                if (cl == 1 && comp[1] != '.') is_dotdot = 0;
            }
            cl++;
        }
        if (cl == 2 && is_dotdot) {
            /* ".." — check it doesn't leave root. */
            if (out_len == 0) {
                w2m_unimplemented(W2M_API_CreateFileW, 5);
                return 0;
            }
            /* Pop last component. */
            while (out_len > 0 && out[out_len - 1] != '/')
                out_len--;
            if (out_len > 0)
                out_len--; /* remove trailing '/' */
            out[out_len] = 0;
            continue;
        }

        if (out_len > 0) {
            out[out_len++] = '/';
        }
        for (i = 0; i < cl; i++)
            out[out_len++] = comp[i];
        out[out_len] = 0;
    }

    if (out_len == 0) {
        /* Path normalized to empty (e.g. "." or trailing separators). */
        set_error(ERROR_PATH_NOT_FOUND);
        return ERROR_PATH_NOT_FOUND;
    }

    return 0;
}

/* Convert a host errno (negative value from FILE_OPEN/UNLINK) to Windows error. */
static u32 errno_to_win(u64 e)
{
    switch ((int)e) {
    case -2:    return ERROR_FILE_NOT_FOUND;
    case -13:   return ERROR_ACCESS_DENIED;
    case -17:   return ERROR_FILE_EXISTS;
    case -20:   return ERROR_PATH_NOT_FOUND;
    case -21:   return ERROR_PATH_NOT_FOUND;
    case -36:   return ERROR_FILE_EXISTS; /* EEXIST */
    case -1:
    default:    return ERROR_ACCESS_DENIED;
    }
}

/* Allocate a file table entry. */
static struct file_entry *alloc_entry(void)
{
    for (unsigned i = 0; i < MAX_FILES; i++) {
        if (files[i].fd == 0) {
            files[i].id = 0;
            files[i].access = 0;
            files[i].share = 0;
            files[i].position = 0;
            return &files[i];
        }
    }
    return 0;
}

/* Free a file table entry. */
static void free_entry(struct file_entry *f)
{
    f->fd = 0;
    f->id = 0;
    f->access = 0;
    f->share = 0;
    f->position = 0;
}

/* Check whether two desired access / share mode combinations conflict.
 * Windows sharing rule: the second open's desired access must be allowed by
 * the first open's share mode, and the second open's share mode must allow
 * the first open's desired access. */
static int sharing_conflicts(u32 acc1, u32 share1, u32 acc2, u32 share2)
{
    /* Does share1 allow acc2? */
    if ((acc2 & GENERIC_READ) && !(share1 & FILE_SHARE_READ))
        return 1;
    if ((acc2 & GENERIC_WRITE) && !(share1 & FILE_SHARE_WRITE))
        return 1;

    /* Does share2 allow acc1? */
    if ((acc1 & GENERIC_READ) && !(share2 & FILE_SHARE_READ))
        return 1;
    if ((acc1 & GENERIC_WRITE) && !(share2 & FILE_SHARE_WRITE))
        return 1;

    return 0;
}

/* Count open handles to a given file id, optionally excluding one entry. */
static unsigned open_count(u64 id, struct file_entry *exclude)
{
    unsigned n = 0;
    for (unsigned i = 0; i < MAX_FILES; i++) {
        if (&files[i] == exclude)
            continue;
        if (files[i].fd != 0 && files[i].id == id)
            n++;
    }
    return n;
}

/* Check whether any open handle to id does not share delete. */
static int delete_blocked(u64 id, struct file_entry *exclude)
{
    for (unsigned i = 0; i < MAX_FILES; i++) {
        if (&files[i] == exclude)
            continue;
        if (files[i].fd != 0 && files[i].id == id) {
            if (!(files[i].share & FILE_SHARE_DELETE))
                return 1;
        }
    }
    return 0;
}

u64 k32_CreateFileW(u64 name, u64 desired_access, u64 share_mode, u64 security_attributes, u64 rsp)
{
    (void)security_attributes;

    u32 creation_disposition = *(u32 *)(rsp + 0x28);
    u32 flags_and_attributes = *(u32 *)(rsp + 0x30);
    u64 template = *(u64 *)(rsp + 0x38);

    if (template != 0) {
        w2m_unimplemented(W2M_API_CreateFileW, 6);
        return 0;
    }

    /* Validate disposition. */
    if (creation_disposition == 0 || creation_disposition > 5) {
        set_error(ERROR_INVALID_PARAMETER);
        return w2m_logged(W2M_API_CreateFileW, name, INVALID_HANDLE_VALUE);
    }

    /* Validate access. */
    if ((desired_access & ~(GENERIC_READ | GENERIC_WRITE)) != 0) {
        w2m_unimplemented(W2M_API_CreateFileW, 7);
        return 0;
    }

    /* TRUNCATE_EXISTING requires GENERIC_WRITE. */
    if (creation_disposition == TRUNCATE_EXISTING && !(desired_access & GENERIC_WRITE)) {
        set_error(ERROR_INVALID_PARAMETER);
        return w2m_logged(W2M_API_CreateFileW, name, INVALID_HANDLE_VALUE);
    }

    char path[512];
    u32 err = normalize_path(name, path);
    if (err != 0) {
        return w2m_logged(W2M_API_CreateFileW, name, INVALID_HANDLE_VALUE);
    }

    /* Special case: opening "." directory without backup semantics. */
    if (path[0] == '.' && path[1] == 0) {
        /* Only allowed with FILE_FLAG_BACKUP_SEMANTICS. */
        if (!(flags_and_attributes & FILE_FLAG_BACKUP_SEMANTICS)) {
            set_error(ERROR_ACCESS_DENIED);
            return w2m_logged(W2M_API_CreateFileW, name, INVALID_HANDLE_VALUE);
        }
        /* Directory open not otherwise needed by contract. */
        w2m_unimplemented(W2M_API_CreateFileW, 8);
        return 0;
    }

    /* Look up the path. */
    u64 second = 0;
    u64 kind = w2m_host(W2M_FILE_LOOKUP, (u64)path, 0, 0, 0, &second);
    u64 file_id = second;

    int exists = (kind == 1);

    if (kind == 3) {
        /* Missing directory on the way. */
        set_error(ERROR_PATH_NOT_FOUND);
        return w2m_logged(W2M_API_CreateFileW, name, INVALID_HANDLE_VALUE);
    }

    if (kind == 2) {
        /* It's a directory. Only allowed with backup semantics; otherwise treat as access denied. */
        if (!(flags_and_attributes & FILE_FLAG_BACKUP_SEMANTICS)) {
            set_error(ERROR_ACCESS_DENIED);
            return w2m_logged(W2M_API_CreateFileW, name, INVALID_HANDLE_VALUE);
        }
        w2m_unimplemented(W2M_API_CreateFileW, 9);
        return 0;
    }

    /* Determine whether we should create and whether we require absence. */
    int must_create = 0;
    int must_not_exist = 0;
    int truncate = 0;

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
        must_not_exist = 0;
        must_create = 0;
        break;
    case OPEN_ALWAYS:
        must_create = 1;
        break;
    case TRUNCATE_EXISTING:
        must_not_exist = 0;
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

    /* Check sharing against existing open handles to this file. */
    for (unsigned i = 0; i < MAX_FILES; i++) {
        if (files[i].fd != 0 && files[i].id == file_id) {
            if (sharing_conflicts(files[i].access, files[i].share,
                                  (u32)desired_access, (u32)share_mode)) {
                set_error(ERROR_SHARING_VIOLATION);
                return w2m_logged(W2M_API_CreateFileW, name, INVALID_HANDLE_VALUE);
            }
        }
    }

    /* Open or create on the host. */
    u64 create_arg = 0;
    if (!exists && must_create)
        create_arg = 1; /* O_CREAT|O_EXCL */
    else if (exists && (creation_disposition == CREATE_ALWAYS || creation_disposition == TRUNCATE_EXISTING))
        create_arg = 0; /* open existing, we truncate below */
    else
        create_arg = 0;

    u64 fd = w2m_host(W2M_FILE_OPEN, (u64)path, 0, create_arg, 0, &second);
    file_id = second;

    if ((s64)fd < 0) {
        set_error(errno_to_win(fd));
        return w2m_logged(W2M_API_CreateFileW, name, INVALID_HANDLE_VALUE);
    }

    /* Truncate if needed. */
    if (truncate && exists) {
        w2m_host(W2M_FILE_TRUNCATE, fd, 0, 0, 0, &second);
    }

    struct file_entry *f = alloc_entry();
    if (!f) {
        w2m_host(W2M_FILE_CLOSE, fd, 0, 0, 0, &second);
        set_error(ERROR_ACCESS_DENIED); /* out of handles */
        return w2m_logged(W2M_API_CreateFileW, name, INVALID_HANDLE_VALUE);
    }

    f->fd = fd;
    f->id = file_id;
    f->access = (u32)desired_access;
    f->share = (u32)share_mode;
    f->position = 0;

    u64 h = make_handle((unsigned)(f - files));

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
    char path[512];
    u32 err = normalize_path(name, path);
    if (err != 0) {
        return w2m_logged(W2M_API_DeleteFileW, name, 0);
    }

    u64 second = 0;
    u64 kind = w2m_host(W2M_FILE_LOOKUP, (u64)path, 0, 0, 0, &second);
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

    u64 ret = w2m_host(W2M_FILE_UNLINK, (u64)path, 0, 0, 0, &second);
    if ((s64)ret < 0) {
        set_error(errno_to_win(ret));
        return w2m_logged(W2M_API_DeleteFileW, name, 0);
    }

    set_error(0);
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

    set_error(0);
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

    /* distance is signed 64-bit. */
    s64 d = (s64)distance;
    s64 target = (s64)base + d;

    if (target < 0) {
        set_error(ERROR_NEGATIVE_SEEK);
        if (new_pointer)
            *(u64 *)new_pointer = (u64)(-7); /* probe's sentinel */
        return w2m_logged(W2M_API_SetFilePointerEx, handle, 0);
    }

    f->position = (u64)target;
    if (new_pointer)
        *(u64 *)new_pointer = (u64)target;

    set_error(0);
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

    set_error(0);
    return 1;
}
```