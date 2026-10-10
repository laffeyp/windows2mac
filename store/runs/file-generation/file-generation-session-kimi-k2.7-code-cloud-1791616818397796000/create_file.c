/* create_file.c — what Windows does with CreateFileW and the file calls the game makes (Sprint 042).
 *
 * No C runtime: entry point `start`, kernel32 imports only, so the same binary runs on the Windows
 * runner and in w2m. Calls: CreateFileW, CloseHandle, DeleteFileW, ReadFile, WriteFile,
 * SetFilePointerEx, SetLastError, GetLastError (play run 38006665419's file calls). Every path is
 * relative to the working directory; the probe deletes its files before each scenario and at the end.
 *
 * One JSON row per scenario on standard output:
 *   {"s":<scenario>, ... fields of the calls it made ..., "exists":..., "size":..., "head":...}
 * `err` is GetLastError after the call, with SetLastError(0xDEAD) just before it, so a call that
 * leaves the code untouched reads 57005. `exists`, `size` and `head` (the first 8 bytes, hex) are
 * measured after the scenario's handles are closed, by opening the file again for reading.
 * Exit code 0; the rows are the result.
 */
#include <windows.h>

#define SENTINEL 0xDEADu
#define A L"w2m-cf-a.txt"
#define B L"w2m-cf-b.txt"

static char out[65536];
static unsigned n;

static void put(const char *s) {
    while (*s) out[n++] = *s++;
}

static void putu(unsigned long long v) {
    char t[24];
    int k = 0;
    do {
        t[k++] = (char)('0' + v % 10);
        v /= 10;
    } while (v);
    while (k) out[n++] = t[--k];
}

static void puti(long long v) {
    if (v < 0) {
        out[n++] = '-';
        putu((unsigned long long)-v);
    } else {
        putu((unsigned long long)v);
    }
}

static void row(const char *s) {
    put("{\"s\":\"");
    put(s);
    put("\"");
}

static void field(const char *k, long long v) {
    put(",\"");
    put(k);
    put("\":");
    puti(v);
}

static void flag(const char *k, int v) {
    put(",\"");
    put(k);
    put(v ? "\":true" : "\":false");
}

static void end(void) {
    put("}\n");
}

static int valid(HANDLE h) {
    return h != INVALID_HANDLE_VALUE;
}

static HANDLE open_(const WCHAR *name, DWORD access, DWORD share, DWORD disposition) {
    return CreateFileW(name, access, share, NULL, disposition, FILE_ATTRIBUTE_NORMAL, NULL);
}

/* The call under test, with the sentinel before it; the code after it goes to *err. */
static HANDLE create(const WCHAR *name, DWORD access, DWORD share, DWORD disposition, DWORD *err) {
    SetLastError(SENTINEL);
    HANDLE h = open_(name, access, share, disposition);
    *err = GetLastError();
    return h;
}

static void reset(void) {
    DeleteFileW(A);
    DeleteFileW(B);
}

static void make(const WCHAR *name, const char *bytes, DWORD count) {
    HANDLE h = open_(name, GENERIC_WRITE, 0, CREATE_ALWAYS);
    DWORD w;
    WriteFile(h, bytes, count, &w, NULL);
    CloseHandle(h);
}

/* exists, size and head of `name`, appended to the current row. */
static void measure(const WCHAR *name) {
    HANDLE h = open_(name, GENERIC_READ, FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE, OPEN_EXISTING);
    flag("exists", valid(h));
    if (!valid(h)) return;
    LARGE_INTEGER zero, size;
    zero.QuadPart = 0;
    SetFilePointerEx(h, zero, &size, FILE_END);
    SetFilePointerEx(h, zero, NULL, FILE_BEGIN);
    unsigned char b[8];
    DWORD got = 0;
    ReadFile(h, b, sizeof b, &got, NULL);
    CloseHandle(h);
    field("size", size.QuadPart);
    put(",\"head\":\"");
    for (DWORD i = 0; i < got; i++) {
        out[n++] = "0123456789abcdef"[b[i] >> 4];
        out[n++] = "0123456789abcdef"[b[i] & 15];
    }
    put("\"");
}

static const DWORD DISPOSITION[] = {CREATE_NEW, CREATE_ALWAYS, OPEN_EXISTING, OPEN_ALWAYS, TRUNCATE_EXISTING};
static const char *const DISPOSITION_NAME[] = {"create_new", "create_always", "open_existing", "open_always",
                                               "truncate_existing"};
static const DWORD ACCESS[] = {GENERIC_READ, GENERIC_WRITE, GENERIC_READ | GENERIC_WRITE};
static const char *const ACCESS_NAME[] = {"r", "w", "rw"};
static const DWORD SHARE[] = {0, FILE_SHARE_READ, FILE_SHARE_WRITE, FILE_SHARE_READ | FILE_SHARE_WRITE};
static const char *const SHARE_NAME[] = {"0", "r", "w", "rw"};

static char name[64];

static const char *join(const char *a, const char *b, const char *c, const char *d, const char *e) {
    const char *parts[] = {a, b, c, d, e};
    unsigned k = 0;
    for (int i = 0; i < 5; i++) {
        if (!parts[i]) continue;
        if (k) name[k++] = '.';
        for (const char *p = parts[i]; *p; p++) name[k++] = *p;
    }
    name[k] = 0;
    return name;
}

static void dispositions(void) {
    DWORD err;
    for (int d = 0; d < 5; d++)
        for (int existing = 0; existing < 2; existing++) {
            reset();
            if (existing) make(A, "hello", 5);
            HANDLE h = create(A, GENERIC_READ | GENERIC_WRITE, 0, DISPOSITION[d], &err);
            if (valid(h)) CloseHandle(h);
            row(join("disp", DISPOSITION_NAME[d], existing ? "existing" : "absent", 0, 0));
            flag("valid", valid(h));
            field("err", err);
            measure(A);
            end();
        }
    reset();
    make(A, "hello", 5);
    HANDLE h = create(A, GENERIC_READ, 0, TRUNCATE_EXISTING, &err);
    if (valid(h)) CloseHandle(h);
    row("disp.truncate_existing.read_only");
    flag("valid", valid(h));
    field("err", err);
    measure(A);
    end();
    h = create(A, GENERIC_READ, 0, 0, &err);
    if (valid(h)) CloseHandle(h);
    row("disp.zero");
    flag("valid", valid(h));
    field("err", err);
    end();
    h = create(A, GENERIC_READ, 0, 6, &err);
    if (valid(h)) CloseHandle(h);
    row("disp.six");
    flag("valid", valid(h));
    field("err", err);
    end();
}

static void sharing(void) {
    reset();
    make(A, "hello", 5);
    for (int a1 = 0; a1 < 3; a1++)
        for (int s1 = 0; s1 < 4; s1++)
            for (int a2 = 0; a2 < 3; a2++)
                for (int s2 = 0; s2 < 4; s2++) {
                    DWORD err1, err2;
                    HANDLE h1 = create(A, ACCESS[a1], SHARE[s1], OPEN_EXISTING, &err1);
                    HANDLE h2 = create(A, ACCESS[a2], SHARE[s2], OPEN_EXISTING, &err2);
                    if (valid(h2)) CloseHandle(h2);
                    if (valid(h1)) CloseHandle(h1);
                    row(join("share", ACCESS_NAME[a1], SHARE_NAME[s1], ACCESS_NAME[a2], SHARE_NAME[s2]));
                    flag("first", valid(h1));
                    flag("valid", valid(h2));
                    field("err", err2);
                    end();
                }
}

static void deleted(const char *s, int before) {
    SetLastError(SENTINEL);
    BOOL ok = DeleteFileW(A);
    DWORD err = GetLastError();
    row(s);
    field("ret", ok);
    field("err", err);
    if (before) measure(A);
}

static void deletes(void) {
    reset();
    deleted("del.absent", 1);
    end();
    make(A, "hello", 5);
    deleted("del.closed", 1);
    end();

    make(A, "hello", 5);
    HANDLE h = open_(A, GENERIC_READ, FILE_SHARE_READ, OPEN_EXISTING);
    deleted("del.open.no_share_delete", 0);
    CloseHandle(h);
    measure(A);
    end();

    reset();
    make(A, "hello", 5);
    h = open_(A, GENERIC_READ, FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE, OPEN_EXISTING);
    deleted("del.open.share_delete", 0);
    end();
    DWORD err;
    HANDLE h2 = create(A, GENERIC_READ, FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE, OPEN_EXISTING, &err);
    row("del.pending.open_existing");
    flag("valid", valid(h2));
    field("err", err);
    end();
    if (valid(h2)) CloseHandle(h2);
    h2 = create(A, GENERIC_WRITE, 0, CREATE_NEW, &err);
    row("del.pending.create_new");
    flag("valid", valid(h2));
    field("err", err);
    end();
    if (valid(h2)) CloseHandle(h2);
    CloseHandle(h);
    row("del.pending.after_close");
    measure(A);
    end();
}

static void named(const char *s, const WCHAR *path, DWORD access, DWORD disposition, DWORD flags) {
    SetLastError(SENTINEL);
    HANDLE h = CreateFileW(path, access, FILE_SHARE_READ | FILE_SHARE_WRITE, NULL, disposition, flags, NULL);
    DWORD err = GetLastError();
    if (valid(h)) CloseHandle(h);
    row(s);
    flag("valid", valid(h));
    field("err", err);
    end();
}

static void names(void) {
    reset();
    named("name.missing_dir.open_existing", L"w2m-cf-nodir\\a.txt", GENERIC_READ, OPEN_EXISTING, FILE_ATTRIBUTE_NORMAL);
    named("name.missing_dir.create_new", L"w2m-cf-nodir\\a.txt", GENERIC_WRITE, CREATE_NEW, FILE_ATTRIBUTE_NORMAL);
    named("name.dot.no_backup_semantics", L".", GENERIC_READ, OPEN_EXISTING, FILE_ATTRIBUTE_NORMAL);
    named("name.empty", L"", GENERIC_READ, OPEN_EXISTING, FILE_ATTRIBUTE_NORMAL);
    make(A, "hello", 5);
    named("name.case.open_existing", L"W2M-CF-A.TXT", GENERIC_READ, OPEN_EXISTING, FILE_ATTRIBUTE_NORMAL);
    named("name.case.create_new", L"W2M-CF-A.TXT", GENERIC_WRITE, CREATE_NEW, FILE_ATTRIBUTE_NORMAL);
    named("name.trailing_dot", L"w2m-cf-a.txt.", GENERIC_READ, OPEN_EXISTING, FILE_ATTRIBUTE_NORMAL);
    named("name.trailing_space", L"w2m-cf-a.txt ", GENERIC_READ, OPEN_EXISTING, FILE_ATTRIBUTE_NORMAL);
    named("name.other_absent", B, GENERIC_READ, OPEN_EXISTING, FILE_ATTRIBUTE_NORMAL);
}

static void moved(const char *s, HANDLE h, long long distance, DWORD method) {
    LARGE_INTEGER d, pos;
    d.QuadPart = distance;
    pos.QuadPart = -7;
    SetLastError(SENTINEL);
    BOOL ok = SetFilePointerEx(h, d, &pos, method);
    DWORD err = GetLastError();
    row(s);
    field("ret", ok);
    field("err", err);
    field("pos", pos.QuadPart);
    end();
}

static void transferred(const char *s, HANDLE h, int write, const char *bytes, DWORD count) {
    static unsigned char b[16];
    DWORD done = 77;
    SetLastError(SENTINEL);
    BOOL ok = write ? WriteFile(h, bytes, count, &done, NULL) : ReadFile(h, b, count, &done, NULL);
    DWORD err = GetLastError();
    row(s);
    field("ret", ok);
    field("err", err);
    field("done", done);
    if (!write && ok) {
        put(",\"data\":\"");
        for (DWORD i = 0; i < done; i++) {
            out[n++] = "0123456789abcdef"[b[i] >> 4];
            out[n++] = "0123456789abcdef"[b[i] & 15];
        }
        put("\"");
    }
    end();
}

static void handles(void) {
    reset();
    HANDLE h = open_(A, GENERIC_READ | GENERIC_WRITE, 0, CREATE_NEW);
    transferred("io.write", h, 1, "hello world", 11);
    moved("io.seek.current_0", h, 0, FILE_CURRENT);
    moved("io.seek.end_minus_5", h, -5, FILE_END);
    transferred("io.read_8", h, 0, 0, 8);
    transferred("io.read_at_eof", h, 0, 0, 8);
    moved("io.seek.begin_minus_1", h, -1, FILE_BEGIN);
    moved("io.seek.current_after_failure", h, 0, FILE_CURRENT);
    moved("io.seek.begin_100", h, 100, FILE_BEGIN);
    transferred("io.write_at_100", h, 1, "!", 1);
    moved("io.seek.begin_9", h, 9, FILE_BEGIN);
    transferred("io.read_gap", h, 0, 0, 4);
    CloseHandle(h);
    row("io.after_close");
    measure(A);
    end();

    h = open_(A, GENERIC_READ, FILE_SHARE_READ, OPEN_EXISTING);
    transferred("io.write_on_read_handle", h, 1, "x", 1);
    CloseHandle(h);
    h = open_(A, GENERIC_WRITE, 0, OPEN_EXISTING);
    transferred("io.read_on_write_handle", h, 0, 0, 4);
    CloseHandle(h);
    SetLastError(SENTINEL);
    BOOL ok = CloseHandle(h);
    DWORD err = GetLastError();
    row("close.twice");
    field("ret", ok);
    field("err", err);
    end();
}

void start(void) {
    dispositions();
    sharing();
    deletes();
    names();
    handles();
    reset();
    DWORD written;
    WriteFile(GetStdHandle(STD_OUTPUT_HANDLE), out, n, &written, NULL);
    ExitProcess(0);
}
