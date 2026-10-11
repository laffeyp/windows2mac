// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* format_message.c — FormatMessageW from the system message table (Sprint 049, family "format message";
 * specs/work_list.json).
 *
 * No C runtime. The capture (store/game-runs/38113342549, 38113343972) holds mono's one form, 64 times:
 * FormatMessageW(ALLOCATE_BUFFER | FROM_SYSTEM | IGNORE_INSERTS, NULL, 2 or 3, LANG_USER_DEFAULT (0x400), &p, 0, NULL),
 * answering 44. Rows "form.*": return, last error (after SetLastError(0xDEAD)) and text for that form and its buffer,
 * language and id edges. Rows "dump": every id from 0 to 16383 that the game's form answers, with its text; they are
 * the system message table w2m carries (specs/system_messages.json), not contract rows.
 */
#include <windows.h>

#define SENTINEL 0xDEADu
#define GAME_FLAGS (FORMAT_MESSAGE_ALLOCATE_BUFFER | FORMAT_MESSAGE_FROM_SYSTEM | FORMAT_MESSAGE_IGNORE_INSERTS)

static char out[32768];
static unsigned n;
static WCHAR wbuf[1024];

static void put(const char *s) { while (*s && n < sizeof out - 64) out[n++] = *s++; }
static void putu(unsigned long long v) {
    char t[24];
    int k = 0;
    do { t[k++] = (char)('0' + v % 10); v /= 10; } while (v);
    while (k) out[n++] = t[--k];
}
static void row(const char *s) { put("{\"s\":\""); put(s); put("\""); }
static void field(const char *k, unsigned long long v) { put(",\""); put(k); put("\":"); putu(v); }
/* JSON text of `len` units: printable ASCII as is, every other unit as \uXXXX. */
static void text(const char *k, const WCHAR *s, unsigned len) {
    put(",\""); put(k); put("\":\"");
    for (unsigned i = 0; i < len && n < sizeof out - 80; i++) {
        WCHAR c = s[i];
        if (c == '"' || c == '\\') { out[n++] = '\\'; out[n++] = (char)c; }
        else if (c >= 0x20 && c < 0x7F) out[n++] = (char)c;
        else {
            put("\\u");
            for (int sh = 12; sh >= 0; sh -= 4) out[n++] = "0123456789abcdef"[(c >> sh) & 15];
        }
    }
    put("\"");
}
/* Each row is written when it ends, so a fault leaves the rows before it. */
static void end(void) {
    put("}\n");
    DWORD written;
    WriteFile(GetStdHandle(STD_OUTPUT_HANDLE), out, n, &written, NULL);
    n = 0;
}

/* The game's form: the buffer allocated by the call (LocalFree after). */
static void allocated(const char *name, DWORD id, DWORD lang, DWORD flags) {
    WCHAR *p = (WCHAR *)(ULONG_PTR)0xA5A5A5A5A5A5A5A5ull;
    SetLastError(SENTINEL);
    DWORD r = FormatMessageW(flags, NULL, id, lang, (LPWSTR)&p, 0, NULL);
    DWORD err = GetLastError();
    row(name); field("ret", r); field("err", err);
    if (r) { text("text", p, r); field("nul_after", p[r] == 0); LocalFree(p); }
    else field("pointer_left", p == (WCHAR *)(ULONG_PTR)0xA5A5A5A5A5A5A5A5ull);
    end();
}

/* The caller's buffer of `size` units. */
static void into(const char *name, DWORD id, DWORD size) {
    for (unsigned i = 0; i < 1024; i++) wbuf[i] = 0xA5A5;
    SetLastError(SENTINEL);
    DWORD r = FormatMessageW(FORMAT_MESSAGE_FROM_SYSTEM | FORMAT_MESSAGE_IGNORE_INSERTS, NULL, id, 0x400, wbuf, size, NULL);
    DWORD err = GetLastError();
    row(name); field("ret", r); field("err", err);
    if (r) { text("text", wbuf, r); field("nul_after", wbuf[r] == 0); }
    else field("first_unit", wbuf[0]);
    end();
}

void start(void) {
    allocated("form.game_2", 2, 0x400, GAME_FLAGS);
    allocated("form.game_3", 3, 0x400, GAME_FLAGS);
    allocated("form.id_0", 0, 0x400, GAME_FLAGS);
    allocated("form.insert_kept", 193, 0x400, GAME_FLAGS);          /* "%1 is not a valid Win32 application." */
    allocated("form.language_neutral", 2, 0, GAME_FLAGS);
    allocated("form.language_en_us", 2, 0x409, GAME_FLAGS);
    allocated("form.language_de_de", 2, 0x407, GAME_FLAGS);
    allocated("form.unknown_id", 0x3FFF, 0x400, GAME_FLAGS);
    allocated("form.hresult_win32", 0x80070002u, 0x400, GAME_FLAGS);
    allocated("form.max_width", 2, 0x400, GAME_FLAGS | FORMAT_MESSAGE_MAX_WIDTH_MASK);
    into("form.buffer", 2, 512);
    into("form.buffer_exact", 2, 45);                                 /* 44 units and the NUL */
    into("form.buffer_short", 2, 44);
    into("form.buffer_zero", 2, 0);

    for (DWORD id = 0; id < 16384; id++) {
        WCHAR *p = NULL;
        DWORD r = FormatMessageW(GAME_FLAGS, NULL, id, 0x400, (LPWSTR)&p, 0, NULL);
        if (!r) continue;
        row("dump"); field("id", id); text("text", p, r); end();
        LocalFree(p);
    }
    ExitProcess(0);
}
