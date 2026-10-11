// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* windows.c — the game's window classes, its window and what the window procedure sees (Sprint 054, family
 * "windows"; specs/work_list.json).
 *
 * No C runtime; linked with the game's manifest and an icon group with the game's id (103). The traces hold the
 * forms. Batch run (store/game-runs/38113342549, seq 1656-1681): LoadIconA(the exe, 103), RegisterClassExW,
 * RegisterClassW, CreateWindowExW(0, the RegisterClassW atom, a title, style 0, CW_USEDEFAULT x4, ...), inside it
 * DefWindowProcW for WM_GETMINMAXINFO, WM_NCCREATE and WM_NCCALCSIZE and SetWindowLongPtrW(window, 0, pointer) from
 * the procedure's WM_CREATE; then GetClientRect, MonitorFromWindow and LoadCursorA(NULL, IDC_ARROW). Play run
 * (38113343972, seq 1656-1674): CreateWindowExW(0, a class name, a title, 0xCA0000, CW_USEDEFAULT, CW_USEDEFAULT,
 * 320, 240, ...). The procedure here records each message it receives with its parameters, passes it to
 * DefWindowProcW (WM_CREATE excepted, as UnityPlayer's procedure keeps it) and records the answer; after
 * CreateWindowExW returns the window's rectangles, styles and stored values are read back.
 */
#include <windows.h>

#define SENTINEL 0xDEADu
#define STORED ((LONG_PTR)0x5A5A5A5A12345678ll)

static char out[32768];
static unsigned n;
static void put(const char *s) { while (*s && n < sizeof out - 64) out[n++] = *s++; }
static void putw(const WCHAR *s) { while (*s && n < sizeof out - 64) out[n++] = *s < 128 && *s != '"' && *s != '\\' ? (char)*s : '_', s++; }
static void putu(unsigned long long v) {
    char t[24];
    int k = 0;
    do { t[k++] = (char)('0' + v % 10); v /= 10; } while (v);
    while (k) out[n++] = t[--k];
}
static void row(const char *s) { put("{\"s\":\""); put(s); put("\""); }
static void field(const char *k, unsigned long long v) { put(",\""); put(k); put("\":"); putu(v); }
static void sfield(const char *k, long long v) { put(",\""); put(k); put("\":"); if (v < 0) { put("-"); v = -v; } putu((unsigned long long)v); }
static void flag(const char *k, int v) { put(",\""); put(k); put(v ? "\":true" : "\":false"); }
static void wtext(const char *k, const WCHAR *v) { put(",\""); put(k); put("\":\""); putw(v); put("\""); }
static void rect(const char *k, const RECT *r) {
    char key[40];
    static const char *const side[] = {"_left", "_top", "_right", "_bottom"};
    long v[4] = {r->left, r->top, r->right, r->bottom};
    for (int j = 0; j < 4; j++) {
        int m = 0;
        for (const char *p = k; *p; p++) key[m++] = *p;
        for (const char *p = side[j]; *p; p++) key[m++] = *p;
        key[m] = 0;
        sfield(key, v[j]);
    }
}
static void point(const char *k, POINT p) {
    char key[40];
    int m = 0;
    for (const char *q = k; *q; q++) key[m++] = *q;
    key[m] = '_'; key[m + 1] = 'x'; key[m + 2] = 0;
    sfield(key, p.x);
    key[m + 1] = 'y';
    sfield(key, p.y);
}
static void end(void) {
    put("}\n");
    DWORD written;
    WriteFile(GetStdHandle(STD_OUTPUT_HANDLE), out, n, &written, NULL);
    n = 0;
}

static HINSTANCE self;
static const char *window_name;            /* "batch" or "play", the rows' prefix */
static HWND current;
static unsigned messages;

static void message_row(void) {
    char name[48];
    int m = 0;
    for (const char *p = "msg."; *p; p++) name[m++] = *p;
    for (const char *p = window_name; *p; p++) name[m++] = *p;
    name[m++] = '.';
    if (messages >= 10) name[m++] = (char)('0' + messages / 10);
    name[m++] = (char)('0' + messages % 10);
    name[m] = 0;
    row(name);
}

static void create_struct(const CREATESTRUCTW *c) {
    flag("params_null", c->lpCreateParams == NULL); flag("instance_is_self", c->hInstance == self);
    flag("menu_null", c->hMenu == NULL); flag("parent_null", c->hwndParent == NULL);
    sfield("cx", c->cx); sfield("cy", c->cy); sfield("x", c->x); sfield("y", c->y);
    field("style", (DWORD)c->style); field("ex_style", c->dwExStyle);
    if (c->lpszName) wtext("name", c->lpszName);
    flag("class_is_atom", ((ULONG_PTR)c->lpszClass >> 16) == 0);
    if (((ULONG_PTR)c->lpszClass >> 16) != 0) wtext("class", c->lpszClass);
}

static LRESULT CALLBACK procedure(HWND w, UINT msg, WPARAM wp, LPARAM lp) {
    if (!current) current = w;
    message_row();
    messages++;
    field("msg", msg); flag("same_window", w == current);
    LRESULT r;
    switch (msg) {
    case WM_GETMINMAXINFO: {
        MINMAXINFO *mm = (MINMAXINFO *)lp;
        field("wparam", wp);
        point("max_size_in", mm->ptMaxSize); point("max_position_in", mm->ptMaxPosition);
        point("min_track_in", mm->ptMinTrackSize); point("max_track_in", mm->ptMaxTrackSize);
        r = DefWindowProcW(w, msg, wp, lp);
        point("max_size", mm->ptMaxSize); point("max_position", mm->ptMaxPosition);
        point("min_track", mm->ptMinTrackSize); point("max_track", mm->ptMaxTrackSize);
        break;
    }
    case WM_NCCREATE:
        field("wparam", wp);
        create_struct((const CREATESTRUCTW *)lp);
        r = DefWindowProcW(w, msg, wp, lp);
        break;
    case WM_NCCALCSIZE: {
        RECT *rc = (RECT *)lp;
        field("wparam", wp);
        rect("in", rc);
        r = DefWindowProcW(w, msg, wp, lp);
        rect("out", rc);
        break;
    }
    case WM_CREATE: {
        field("wparam", wp);
        create_struct((const CREATESTRUCTW *)lp);
        SetLastError(SENTINEL);
        LONG_PTR before = SetWindowLongPtrW(w, 0, STORED);      /* UnityPlayer's call, from its WM_CREATE */
        field("set_long_ptr", (ULONG_PTR)before); field("set_long_ptr_err", GetLastError());
        r = 0;
        break;
    }
    default:
        field("wparam", wp); field("lparam", (ULONG_PTR)lp);
        r = DefWindowProcW(w, msg, wp, lp);
    }
    sfield("result", r);
    end();
    return r;
}

static char name[48];
static void named(const char *prefix, const char *s) {
    int m = 0;
    for (const char *p = prefix; *p; p++) name[m++] = *p;
    for (const char *p = s; *p; p++) name[m++] = *p;
    name[m] = 0;
}

static void after(const char *prefix, HWND w) {
    named(prefix, ".long_ptr_0");
    SetLastError(SENTINEL);
    LONG_PTR v = GetWindowLongPtrW(w, 0);
    row(name); flag("stored", v == STORED); field("err", GetLastError()); end();
    named(prefix, ".long_ptr_8");
    SetLastError(SENTINEL);
    v = SetWindowLongPtrW(w, 8, 1);
    row(name); field("ret", (ULONG_PTR)v); field("err", GetLastError()); end();
    named(prefix, ".user_data");
    SetLastError(SENTINEL);
    LONG_PTR first = SetWindowLongPtrW(w, GWLP_USERDATA, 77);
    DWORD e1 = GetLastError();
    LONG_PTR second = GetWindowLongPtrW(w, GWLP_USERDATA);
    row(name); field("before", (ULONG_PTR)first); field("err", e1); field("after", (ULONG_PTR)second); end();
    named(prefix, ".styles");
    row(name); field("style", (DWORD)GetWindowLongPtrW(w, GWL_STYLE)); field("ex_style", (DWORD)GetWindowLongPtrW(w, GWL_EXSTYLE));
    flag("instance_is_self", (HINSTANCE)GetWindowLongPtrW(w, GWLP_HINSTANCE) == self);
    flag("procedure_is_ours", (WNDPROC)GetWindowLongPtrW(w, GWLP_WNDPROC) == procedure); end();
    RECT r;
    named(prefix, ".client_rect");
    SetLastError(SENTINEL);
    BOOL ok = GetClientRect(w, &r);
    row(name); field("ret", ok != 0); field("err", GetLastError()); rect("rect", &r); end();
    named(prefix, ".window_rect");
    ok = GetWindowRect(w, &r);
    row(name); field("ret", ok != 0); rect("rect", &r); end();
    named(prefix, ".visible");
    row(name); flag("value", IsWindowVisible(w) != 0); end();
    named(prefix, ".monitor");
    HMONITOR m = MonitorFromWindow(w, MONITOR_DEFAULTTONEAREST);
    POINT origin = {0, 0};
    row(name); flag("primary", m == MonitorFromPoint(origin, MONITOR_DEFAULTTOPRIMARY)); end();
}

void start(void) {
    self = GetModuleHandleW(NULL);
    MONITORINFO mi;
    for (unsigned i = 0; i < sizeof mi; i++) ((char *)&mi)[i] = 0;
    mi.cbSize = sizeof mi;
    POINT zero = {0, 0};
    GetMonitorInfoW(MonitorFromPoint(zero, MONITOR_DEFAULTTOPRIMARY), &mi);
    row("screen.work_area"); rect("rect", &mi.rcWork); end();     /* the runners' taskbars differ (rows display) */
    SetLastError(SENTINEL);
    HICON icon = LoadIconA(self, MAKEINTRESOURCEA(103));
    row("icon.own_103"); flag("non_null", icon != NULL); field("err", GetLastError()); end();
    SetLastError(SENTINEL);
    HICON missing = LoadIconA(self, MAKEINTRESOURCEA(999));
    row("icon.own_999"); flag("non_null", missing != NULL); field("err", GetLastError()); end();
    SetLastError(SENTINEL);
    HICON application = LoadIconA(NULL, (LPCSTR)IDI_APPLICATION);
    row("icon.system_application"); field("handle", (ULONG_PTR)application); field("err", GetLastError()); end();
    SetLastError(SENTINEL);
    HCURSOR arrow = LoadCursorA(NULL, (LPCSTR)IDC_ARROW);
    row("cursor.arrow"); field("handle", (ULONG_PTR)arrow); field("err", GetLastError()); end();
    HCURSOR arrow_again = LoadCursorA(NULL, (LPCSTR)IDC_ARROW);
    row("cursor.arrow_again"); flag("same", arrow_again == arrow); end();

    WNDCLASSEXW main_class;
    for (unsigned i = 0; i < sizeof main_class; i++) ((char *)&main_class)[i] = 0;
    main_class.cbSize = sizeof main_class;
    main_class.style = CS_HREDRAW | CS_VREDRAW | CS_DBLCLKS;
    main_class.lpfnWndProc = procedure;
    main_class.cbWndExtra = 8;
    main_class.hInstance = self;
    main_class.hIcon = icon;
    main_class.hCursor = arrow;
    main_class.lpszClassName = L"W2mProbeMain";
    SetLastError(SENTINEL);
    ATOM main_atom = RegisterClassExW(&main_class);
    row("class.register_ex"); flag("atom_in_range", main_atom >= 0xC000); field("err", GetLastError()); end();
    SetLastError(SENTINEL);
    ATOM again = RegisterClassExW(&main_class);
    row("class.register_ex_again"); field("atom", again); field("err", GetLastError()); end();

    WNDCLASSW hidden_class;
    for (unsigned i = 0; i < sizeof hidden_class; i++) ((char *)&hidden_class)[i] = 0;
    hidden_class.lpfnWndProc = procedure;
    hidden_class.cbWndExtra = 8;
    hidden_class.hInstance = self;
    hidden_class.lpszClassName = L"W2mProbeHidden";
    SetLastError(SENTINEL);
    ATOM hidden_atom = RegisterClassW(&hidden_class);
    row("class.register"); flag("atom_in_range", hidden_atom >= 0xC000); flag("after_first", hidden_atom > main_atom);
    field("err", GetLastError()); end();

    SetLastError(SENTINEL);
    HWND none = CreateWindowExW(0, L"W2mProbeNoSuchClass", L"x", 0, CW_USEDEFAULT, CW_USEDEFAULT, CW_USEDEFAULT,
                                CW_USEDEFAULT, NULL, NULL, self, NULL);
    row("create.unknown_class"); flag("null", none == NULL); field("err", GetLastError()); end();

    /* the batch run's window: the RegisterClassW class by atom, style 0, CW_USEDEFAULT four times */
    window_name = "batch";
    SetLastError(SENTINEL);
    HWND batch = CreateWindowExW(0, (LPCWSTR)(ULONG_PTR)hidden_atom, L"Hail to the Rainbow", 0, CW_USEDEFAULT,
                                 CW_USEDEFAULT, CW_USEDEFAULT, CW_USEDEFAULT, NULL, NULL, self, NULL);
    row("create.batch"); flag("null", batch == NULL); flag("first_message_window", batch == current);
    field("messages", messages); field("err", GetLastError()); end();
    if (batch) after("batch", batch);

    /* the play run's window: the RegisterClassExW class by name, WS_OVERLAPPED|WS_CAPTION|WS_SYSMENU|WS_MINIMIZEBOX,
     * CW_USEDEFAULT position, 320 x 240 */
    window_name = "play";
    current = NULL;
    messages = 0;
    SetLastError(SENTINEL);
    HWND play = CreateWindowExW(0, L"W2mProbeMain", L"Hail to the Rainbow", 0xCA0000, CW_USEDEFAULT, CW_USEDEFAULT,
                                320, 240, NULL, NULL, self, NULL);
    row("create.play"); flag("null", play == NULL); flag("first_message_window", play == current);
    flag("differs_from_batch", play != batch); field("messages", messages); field("err", GetLastError()); end();
    if (play) after("play", play);
    ExitProcess(0);
}
