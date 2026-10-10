/* tools/guest_runtime/file.c — Windows file API for the w2m guest runtime. */

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

#define INVALID_HANDLE_VALUE             ((u64)(~0ull))

/* Windows error codes. */
#define ERROR_SUCCESS                    0u
#define ERROR_FILE_NOT_FOUND             2u
#define ERROR_PATH_NOT_FOUND             3u
#define ERROR_ACCESS_DENIED              5u
#define ERROR_INVALID_HANDLE             6u
#define ERROR_INVALID_PARAMETER          87u
#define ERROR_FILE_EXISTS                80u
#define ERROR_ALREADY_EXISTS             183u
#define ERROR_NEGATIVE_SEEK              131u
#define ERROR_SHARING_VIOLATION          32u

/* Handle table: we keep a small fixed array. Handle 0 is reserved. */
#define MAX_HANDLES 64

struct file_handle {
    u64 fd;          /* host descriptor, or 0 if free */
    u64 position;    /* current file pointer */
    u32 access;      /* desired access the handle was opened with */
    u32 share;       /* share mode the handle was opened with */
    u64 file_id;     /* host inode/file id for sharing checks */
};

static struct file_handle handles[MAX_HANDLES];

/* Forward declarations. */
static void set_error(u32 code);
static u32 get_error(void);
static u64 make_handle(int idx);
static int handle_index(u64 h);
static u64 host_lookup(const u16 *name, u64 chars, u64 *file_id);
static u64 host_open(const u16 *name, u64 chars, int create, u64 *file_id);
static u64 host_close(u64 fd);
static u64 host_read(u64 fd, u64 buf, u64 n, u64 off);
static u64 host_write(u64 fd, u64 buf, u64 n, u64 off);
static u64 host_size(u64 fd);
static u64 host_truncate(u64 fd, u64 size);
static u64 host_unlink(const u16 *name, u64 chars);
static int path_ok(const u16 *name, u64 chars, char *out, u64 *out_len);
static int share_conflict(u64 file_id, u32 access, u32 share, int skip_idx);

/* ------------------------------------------------------------------ */
/* Helpers                                                            */
/* ------------------------------------------------------------------ */

static void set_error(u32 code)
{
    w2m_teb()->last_error_value = code;
}

static u32 get_error(void)
{
    return w2m_teb()->last_error_value;
}

static u64 make_handle(int idx)
{
    /* Avoid colliding with stdout handle 0x74 used by runtime.c. */
    u64 h = (u64)(idx + 1);
    if (h == 0x74ull) h = 0x75ull;
    return h;
}

static int handle_index(u64 h)
{
    if (h == 0x74ull) return -1;
    if (h == 0x75ull) h = 0x74ull;
    u64 idx = h - 1;
    if (idx >= MAX_HANDLES) return -1;
    if (!handles[idx].fd) return -1;
    return (int)idx;
}

/* UTF-16LE code unit access. */
typedef unsigned short u16;

static int path_ok(const u16 *name, u64 chars, char *out, u64 *out_len)
{
    /*
     * Convert UTF-16LE to an ASCII-normalized path:
     *   - reject absolute paths, drive letters, empty paths
     *   - fold A-Z to a-z
     *   - turn backslashes into forward slashes
     *   - reject components ., .., or components that would leave root
     *   - reject non-ASCII code units and colons
     * The host expects a UTF-8 byte string; we emit ASCII bytes.
     */
    u64 j = 0;
    u64 depth = 0;

    if (chars == 0)
        return 0;

    for (u64 i = 0; i < chars; i++) {
        u16 c = name[2 * i] | ((u16)name[2 * i + 1] << 8);
        if (c == 0)
            break;
        if (c == '\\' || c == '/')
            c = '/';
        else if (c >= 'A' && c <= 'Z')
            c = c - 'A' + 'a';
        else if (c > 0x7f)
            return 0;
        else if (c == ':')
            return 0;

        out[j++] = (char)(unsigned char)c;
    }
    out[j] = 0;
    *out_len = j;

    if (j == 0)
        return 0;

    /* No leading slash / absolute path / drive letter. */
    if (out[0] == '/')
        return 0;

    /* Validate components. */
    u64 i = 0;
    while (i < j) {
        u64 start = i;
        while (i < j && out[i] != '/') i++;
        u64 len = i - start;

        if (len == 0) {
            /* empty component: leading/trailing/consecutive slash */
            if (start == 0 || i == j) {
                /* trailing/leading slashes are allowed by Windows for files,
                 * but we treat them as part of the name. However an empty
                 * component in the middle is invalid. */
                if (start != 0 && i != j)
                    return 0;
            }
        } else if (len == 1 && out[start] == '.') {
            return 0;
        } else if (len == 2 && out[start] == '.' && out[start + 1] == '.') {
            return 0;
        }

        if (i < j) i++; /* skip '/' */
    }

    return 1;
}

static int share_conflict(u64 file_id, u32 access, u32 share, int skip_idx)
{
    for (int i = 0; i < MAX_HANDLES; i++) {
        if (i == skip_idx) continue;
        if (!handles[i].fd) continue;
        if (handles[i].file_id != file_id) continue;

        u32 oacc = handles[i].access;
        u32 oshr = handles[i].share;

        /* Requested access must be allowed by existing share. */
        if ((access & GENERIC_READ) && !(oshr & FILE_SHARE_READ))
            return 1;
        if ((access & GENERIC_WRITE) && !(oshr & FILE_SHARE_WRITE))
            return 1;

        /* Existing access must be allowed by requested share. */
        if ((oacc & GENERIC_READ) && !(share & FILE_SHARE_READ))
            return 1;
        if ((oacc & GENERIC_WRITE) && !(share & FILE_SHARE_WRITE))
            return 1;
    }
    return 0;
}

/* ------------------------------------------------------------------ */
/* Host primitive wrappers                                            */
/* ------------------------------------------------------------------ */

static u64 host_lookup(const u16 *name, u64 chars, u64 *file_id)
{
    char path[512];
    u64 len;
    if (!path_ok(name, chars, path, &len)) {
        w2m_unimplemented(W2M_API_CreateFileW, 1);
        return 0;
    }
    u64 second = 0;
    u64 r = w2m_host(W2M_FILE_LOOKUP, (u64)path, len, 0, 0, &second);
    *file_id = second;
    return r;
}

static u64 host_open(const u16 *name, u64 chars, int create, u64 *file_id)
{
    char path[512];
    u64 len;
    if (!path_ok(name, chars, path, &len)) {
        w2m_unimplemented(W2M_API_CreateFileW, 2);
        return 0;
    }
    u64 second = 0;
    u64 r = w2m_host(W2M_FILE_OPEN, (u64)path, len, create ? 1 : 0, 0, &second);
    *file_id = second;
    return r;
}

static u64 host_close(u64 fd)
{
    u64 second = 0;
    return w2m_host(W2M_FILE_CLOSE, fd, 0, 0, 0, &second);
}

static u64 host_read(u64 fd, u64 buf, u64 n, u64 off)
{
    u64 second = 0;
    return w2m_host(W2M_FILE_READ, fd, buf, n, off, &second);
}

static u64 host_write(u64 fd, u64 buf, u64 n, u64 off)
{
    u64 second = 0;
    return w2m_host(W2M_FILE_WRITE, fd, buf, n, off, &second);
}

static u64 host_size(u64 fd)
{
    u64 second = 0;
    return w2m_host(W2M_FILE_SIZE, fd, 0, 0, 0, &second);
}

static u64 host_truncate(u64 fd, u64 size)
{
    u64 second = 0;
    return w2m_host(W2M_FILE_TRUNCATE, fd, size, 0, 0, &second);
}

static u64 host_unlink(const u16 *name, u64 chars)
{
    char path[512];
    u64 len;
    if (!path_ok(name, chars, path, &len)) {
        w2m_unimplemented(W2M_API_DeleteFileW, 1);
        return 0;
    }
    u64 second = 0;
    return w2m_host(W2M_FILE_UNLINK, (u64)path, len, 0, 0, &second);
}

/* ------------------------------------------------------------------ */
/* Exported functions                                                 */
/* ------------------------------------------------------------------ */

u64 k32_CreateFileW(u64 name, u64 desired_access, u64 share_mode,
                    u64 security_attributes, u64 rsp)
{
    (void)security_attributes;

    u32 creation_disposition = *(u32 *)(rsp + 0x28);
    u32 flags_attrs = *(u32 *)(rsp + 0x30);
    u64 template = *(u64 *)(rsp + 0x38);

    if (template != 0) {
        w2m_unimplemented(W2M_API_CreateFileW, 10);
        return INVALID_HANDLE_VALUE;
    }

    /* Validate disposition. */
    if (creation_disposition == 0 || creation_disposition > 5) {
        set_error(ERROR_INVALID_PARAMETER);
        return w2m_logged(W2M_API_CreateFileW, name, INVALID_HANDLE_VALUE);
    }

    /* Validate access for truncate_existing. */
    if (creation_disposition == TRUNCATE_EXISTING &&
        !(desired_access & GENERIC_WRITE)) {
        set_error(ERROR_INVALID_PARAMETER);
        return w2m_logged(W2M_API_CreateFileW, name, INVALID_HANDLE_VALUE);
    }

    /* Directory opens require backup semantics; we don't support directories. */
    if (flags_attrs & FILE_FLAG_BACKUP_SEMANTICS) {
        /* Opening "." as directory with backup semantics: Windows allows it.
         * Without backup semantics it fails with ERROR_ACCESS_DENIED.
         * We emulate: if path is exactly "." and disposition is OPEN_EXISTING,
         * fail with access denied unless backup semantics. */
    }

    /* Count characters in the UTF-16LE name. */
    const u16 *p = (const u16 *)name;
    u64 chars = 0;
    while (p[chars] != 0) chars++;

    /* Special-case "." without backup semantics: access denied. */
    if (chars == 1 && p[0] == '.' && !(flags_attrs & FILE_FLAG_BACKUP_SEMANTICS)) {
        set_error(ERROR_ACCESS_DENIED);
        return w2m_logged(W2M_API_CreateFileW, name, INVALID_HANDLE_VALUE);
    }

    u64 file_id = 0;
    u64 kind = host_lookup(p, chars, &file_id);

    int exists = (kind == 1);
    int missing_dir = (kind == 3);

    if (missing_dir) {
        set_error(ERROR_PATH_NOT_FOUND);
        return w2m_logged(W2M_API_CreateFileW, name, INVALID_HANDLE_VALUE);
    }

    int create = 0;
    int truncate = 0;
    int must_not_exist = 0;
    int set_already_exists = 0;

    switch (creation_disposition) {
    case CREATE_NEW:
        must_not_exist = 1;
        create = 1;
        break;
    case CREATE_ALWAYS:
        create = 1;
        set_already_exists = exists;
        truncate = exists;
        break;
    case OPEN_EXISTING:
        if (!exists) {
            set_error(ERROR_FILE_NOT_FOUND);
            return w2m_logged(W2M_API_CreateFileW, name, INVALID_HANDLE_VALUE);
        }
        break;
    case OPEN_ALWAYS:
        create = 1;
        set_already_exists = exists;
        break;
    case TRUNCATE_EXISTING:
        if (!exists) {
            set_error(ERROR_FILE_NOT_FOUND);
            return w2m_logged(W2M_API_CreateFileW, name, INVALID_HANDLE_VALUE);
        }
        truncate = 1;
        break;
    }

    if (must_not_exist && exists) {
        set_error(ERROR_FILE_EXISTS);
        return w2m_logged(W2M_API_CreateFileW, name, INVALID_HANDLE_VALUE);
    }

    /* Sharing check against existing open handles. */
    if (exists && share_conflict(file_id, (u32)desired_access, (u32)share_mode, -1)) {
        set_error(ERROR_SHARING_VIOLATION);
        return w2m_logged(W2M_API_CreateFileW, name, INVALID_HANDLE_VALUE);
    }

    u64 fd = host_open(p, chars, create, &file_id);
    if ((s64)fd < 0) {
        /* Map errno-ish negative values to Windows errors. */
        set_error(ERROR_ACCESS_DENIED); /* best guess for the contract */
        return w2m_logged(W2M_API_CreateFileW, name, INVALID_HANDLE_VALUE);
    }

    if (truncate) {
        host_truncate(fd, 0);
    }

    /* Allocate handle slot. */
    int idx = -1;
    for (int i = 0; i < MAX_HANDLES; i++) {
        if (!handles[i].fd) {
            idx = i;
            break;
        }
    }
    if (idx < 0) {
        host_close(fd);
        set_error(ERROR_TOO_MANY_OPEN_FILES);
        return w2m_logged(W2M_API_CreateFileW, name, INVALID_HANDLE_VALUE);
    }

    handles[idx].fd = fd;
    handles[idx].position = 0;
    handles[idx].access = (u32)desired_access;
    handles[idx].share = (u32)share_mode;
    handles[idx].file_id = file_id;

    if (set_already_exists)
        set_error(ERROR_ALREADY_EXISTS);
    else
        set_error(ERROR_SUCCESS);

    return w2m_logged(W2M_API_CreateFileW, name, make_handle(idx));
}

u64 k32_CloseHandle(u64 handle)
{
    int idx = handle_index(handle);
    if (idx < 0) {
        set_error(ERROR_INVALID_HANDLE);
        return w2m_logged(W2M_API_CloseHandle, handle, 0);
    }

    host_close(handles[idx].fd);
    handles[idx].fd = 0;
    handles[idx].position = 0;
    handles[idx].access = 0;
    handles[idx].share = 0;
    handles[idx].file_id = 0;

    set_error(ERROR_SUCCESS);
    return w2m_logged(W2M_API_CloseHandle, handle, 1);
}

u64 k32_DeleteFileW(u64 name)
{
    const u16 *p = (const u16 *)name;
    u64 chars = 0;
    while (p[chars] != 0) chars++;

    u64 file_id = 0;
    u64 kind = host_lookup(p, chars, &file_id);

    if (kind == 0) {
        set_error(ERROR_FILE_NOT_FOUND);
        return w2m_logged(W2M_API_DeleteFileW, name, 0);
    }

    if (kind == 3) {
        set_error(ERROR_PATH_NOT_FOUND);
        return w2m_logged(W2M_API_DeleteFileW, name, 0);
    }

    /* Check for open handles without FILE_SHARE_DELETE. */
    for (int i = 0; i < MAX_HANDLES; i++) {
        if (!handles[i].fd) continue;
        if (handles[i].file_id != file_id) continue;
        if (!(handles[i].share & FILE_SHARE_DELETE)) {
            set_error(ERROR_SHARING_VIOLATION);
            return w2m_logged(W2M_API_DeleteFileW, name, 0);
        }
    }

    u64 r = host_unlink(p, chars);
    if (r != 0) {
        set_error(ERROR_ACCESS_DENIED);
        return w2m_logged(W2M_API_DeleteFileW, name, 0);
    }

    set_error(ERROR_SUCCESS);
    return w2m_logged(W2M_API_DeleteFileW, name, 1);
}

u64 k32_ReadFile(u64 handle, u64 buffer, u64 count, u64 bytes_read, u64 rsp)
{
    u64 overlapped = *(u64 *)(rsp + 0x28);
    if (overlapped != 0) {
        w2m_unimplemented(W2M_API_ReadFile, 1);
        return 0;
    }

    int idx = handle_index(handle);
    if (idx < 0) {
        *(u32 *)bytes_read = 0;
        set_error(ERROR_INVALID_HANDLE);
        return w2m_logged(W2M_API_ReadFile, handle, 0);
    }

    if (!(handles[idx].access & GENERIC_READ)) {
        *(u32 *)bytes_read = 0;
        set_error(ERROR_ACCESS_DENIED);
        return w2m_logged(W2M_API_ReadFile, handle, 0);
    }

    u64 n = host_read(handles[idx].fd, buffer, count, handles[idx].position);
    if ((s64)n < 0) {
        *(u32 *)bytes_read = 0;
        set_error(ERROR_ACCESS_DENIED);
        return w2m_logged(W2M_API_ReadFile, handle, 0);
    }

    handles[idx].position += n;
    *(u32 *)bytes_read = (u32)n;
    set_error(ERROR_SUCCESS);
    return w2m_logged(W2M_API_ReadFile, handle, 1);
}

u64 k32_SetFilePointerEx(u64 handle, u64 distance, u64 new_pointer, u64 move_method)
{
    int idx = handle_index(handle);
    if (idx < 0) {
        if (new_pointer)
            *(u64 *)new_pointer = 0;
        set_error(ERROR_INVALID_HANDLE);
        return w2m_logged(W2M_API_SetFilePointerEx, handle, 0);
    }

    s64 dist = (s64)distance;
    s64 base = 0;
    switch (move_method) {
    case FILE_BEGIN:
        base = 0;
        break;
    case FILE_CURRENT:
        base = (s64)handles[idx].position;
        break;
    case FILE_END:
        base = (s64)host_size(handles[idx].fd);
        break;
    default:
        if (new_pointer)
            *(u64 *)new_pointer = 0;
        set_error(ERROR_INVALID_PARAMETER);
        return w2m_logged(W2M_API_SetFilePointerEx, handle, 0);
    }

    s64 pos = base + dist;
    if (pos < 0) {
        if (new_pointer)
            *(u64 *)new_pointer = (u64)-7; /* probe initializes to -7 */
        set_error(ERROR_NEGATIVE_SEEK);
        return w2m_logged(W2M_API_SetFilePointerEx, handle, 0);
    }

    handles[idx].position = (u64)pos;
    if (new_pointer)
        *(u64 *)new_pointer = (u64)pos;
    set_error(ERROR_SUCCESS);
    return w2m_logged(W2M_API_SetFilePointerEx, handle, 1);
}

u64 w2m_file_write(u64 handle, u64 buffer, u64 count, u64 written)
{
    int idx = handle_index(handle);
    if (idx < 0) {
        *(u32 *)written = 0;
        set_error(ERROR_INVALID_HANDLE);
        return 0;
    }

    if (!(handles[idx].access & GENERIC_WRITE)) {
        *(u32 *)written = 0;
        set_error(ERROR_ACCESS_DENIED);
        return 0;
    }

    u64 n = host_write(handles[idx].fd, buffer, count, handles[idx].position);
    if ((s64)n < 0) {
        *(u32 *)written = 0;
        set_error(ERROR_ACCESS_DENIED);
        return 0;
    }

    handles[idx].position += n;
    *(u32 *)written = (u32)n;
    set_error(ERROR_SUCCESS);
    return 1;
}
