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
 *
 * Sprint 063 (batch run 38206263976, seq 2592-3497, the game's main thread): on a window of the batch class the game
 * sets GWLP_USERDATA through SetWindowLongPtrA, drains its queue with PeekMessageA (PM_NOREMOVE, then PM_REMOVE),
 * GetMessageExtraInfo, TranslateMessage and DispatchMessageA, registers a device notification (flags 4), asks
 * GetFocus and calls SetFocus on the window, and loads a cursor it wrote to a temporary file with LoadImageW
 * (IMAGE_CURSOR, LR_LOADFROMFILE | LR_DEFAULTSIZE). The probe does the same in that order on a window "game", then the
 * boundaries: a posted message, WM_QUIT, SetFocus on another thread's window, LoadImageW of a resource the module
 * lacks and of a file that is not there.
 */
#include <windows.h>
#include <dbt.h>

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
static void hexfield(const char *k, unsigned long long v) {
    static const char d[] = "0123456789abcdef";
    put(",\""); put(k); put("\":\"");
    char t[17];
    int m = 0;
    do { t[m++] = d[v & 15]; v >>= 4; } while (v);
    while (m) out[n++] = t[--m];
    put("\"");
}
static void end(void) {
    put("}\n");
    DWORD written;
    WriteFile(GetStdHandle(STD_OUTPUT_HANDLE), out, n, &written, NULL);
    n = 0;
}

static HINSTANCE self;
static HWND game_window;
static ATOM batch_atom_for_threads;
static const char *window_name;            /* "batch" or "play", the rows' prefix */
static HWND current;
static unsigned messages;

static void message_row(unsigned index) {
    char name[48];
    int m = 0;
    for (const char *p = "msg."; *p; p++) name[m++] = *p;
    for (const char *p = window_name; *p; p++) name[m++] = *p;
    name[m++] = '.';
    if (index >= 10) name[m++] = (char)('0' + index / 10);
    name[m++] = (char)('0' + index % 10);
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
    unsigned index = messages++;
    LRESULT r;
    /* a message DefWindowProcW answers by sending others (WM_ACTIVATE sends WM_SETFOCUS) gets its row after the
     * answer, so the rows of the messages it caused come first, each whole (Sprint 063) */
    switch (msg) {
    case WM_GETMINMAXINFO:
    case WM_NCCREATE:
    case WM_NCCALCSIZE:
    case WM_CREATE:
        message_row(index);
        field("msg", msg); flag("same_window", w == current);
        break;
    default:
        break;
    }
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
    case WM_SETFOCUS:
    case WM_KILLFOCUS:                         /* wParam: the window losing or gaining the focus */
        r = DefWindowProcW(w, msg, wp, lp);
        message_row(index);
        field("msg", msg); flag("same_window", w == current);
        flag("wparam_null", wp == 0); flag("wparam_is_self", (HWND)wp == w);
        flag("lparam_null", lp == 0);
        break;
    case WM_WINDOWPOSCHANGING:
    case WM_WINDOWPOSCHANGED: {                /* lParam: a WINDOWPOS */
        const WINDOWPOS *p = (const WINDOWPOS *)lp;
        int self_pos = p->hwnd == w, after_null = p->hwndInsertAfter == NULL;
        UINT flags = p->flags;
        r = DefWindowProcW(w, msg, wp, lp);
        message_row(index);
        field("msg", msg); flag("same_window", w == current);
        field("wparam", wp); flag("pos_hwnd_is_self", self_pos); field("pos_flags", flags); flag("after_null", after_null);
        break;
    }
    case WM_ACTIVATEAPP:                       /* lParam: the other application's thread id */
        r = DefWindowProcW(w, msg, wp, lp);
        message_row(index);
        field("msg", msg); flag("same_window", w == current);
        field("wparam", wp); flag("lparam_null", lp == 0);
        break;
    default:
        r = DefWindowProcW(w, msg, wp, lp);
        message_row(index);
        field("msg", msg); flag("same_window", w == current);
        field("wparam", wp); hexfield("lparam", (ULONG_PTR)lp);
    }
    sfield("result", r);
    end();
    return r;
}

static char name[48];
static void game_calls(void);
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
    batch_atom_for_threads = hidden_atom;
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
    /* Sprint 056 (review of 054): class names of 64, 256 and 257 units, and a window off the screen */
    static WCHAR long_class[300];
    static const int lengths[5] = {64, 254, 255, 256, 257};
    static const char *const names[5][2] = {{"class.name_64", "class.name_64_again"}, {"class.name_254", "class.name_254_again"},
                                            {"class.name_255", "class.name_255_again"}, {"class.name_256", "class.name_256_again"},
                                            {"class.name_257", "class.name_257_again"}};
    for (int j = 0; j < 5; j++) {
        for (int i = 0; i < lengths[j]; i++) long_class[i] = (WCHAR)(L'A' + j);
        long_class[lengths[j]] = 0;
        hidden_class.lpszClassName = long_class;
        for (int again = 0; again < 2; again++) {
            SetLastError(SENTINEL);
            ATOM a = RegisterClassW(&hidden_class);
            row(names[j][again]); flag("atom_in_range", a >= 0xC000); field("err", GetLastError()); end();
        }
    }
    window_name = "offscreen";
    current = NULL;
    messages = 0;
    SetLastError(SENTINEL);
    HWND away = CreateWindowExW(0, L"W2mProbeMain", L"away", 0xCA0000, 2000, 2000, 320, 240, NULL, NULL, self, NULL);
    row("create.offscreen"); flag("null", away == NULL); field("messages", messages); field("err", GetLastError()); end();
    if (away) {
        POINT zero = {0, 0};
        HMONITOR primary = MonitorFromPoint(zero, MONITOR_DEFAULTTOPRIMARY);
        row("offscreen.monitor"); flag("null_null", MonitorFromWindow(away, MONITOR_DEFAULTTONULL) == NULL);
        flag("primary_primary", MonitorFromWindow(away, MONITOR_DEFAULTTOPRIMARY) == primary);
        flag("nearest_primary", MonitorFromWindow(away, MONITOR_DEFAULTTONEAREST) == primary); end();
    }
    game_calls();
    ExitProcess(0);
}

/* ---- Sprint 063: the game's later calls on its window, then the boundaries ---------------------------------------- */

static void drain(const char *prefix) {
    MSG m;
    for (int i = 0; i < 12; i++) {
        char key[48];
        int k = 0;
        for (const char *p = prefix; *p; p++) key[k++] = *p;
        key[k++] = '.'; key[k++] = (char)('0' + i / 10); key[k++] = (char)('0' + i % 10); key[k] = 0;
        for (unsigned j = 0; j < sizeof m; j++) ((char *)&m)[j] = 0;
        SetLastError(SENTINEL);
        BOOL looked = PeekMessageA(&m, NULL, 0, 0, PM_NOREMOVE);
        DWORD e0 = GetLastError();
        row(key); field("noremove", looked != 0); field("noremove_err", e0);
        if (!looked) { end(); return; }
        UINT seen = m.message;
        SetLastError(SENTINEL);
        BOOL got = PeekMessageA(&m, NULL, 0, 0, PM_REMOVE);
        field("remove", got != 0); field("err", GetLastError()); field("same", m.message == seen);
        field("msg", m.message); flag("hwnd_game", m.hwnd == game_window); flag("hwnd_null", m.hwnd == NULL);
        field("wparam", m.wParam); hexfield("lparam", (ULONG_PTR)m.lParam);
        field("extra_info", (ULONG_PTR)GetMessageExtraInfo());
        if (m.message == WM_QUIT) { end(); return; }
        SetLastError(SENTINEL);
        BOOL translated = TranslateMessage(&m);
        field("translated", translated != 0); field("translate_err", GetLastError());
        end();
        SetLastError(SENTINEL);
        LRESULT dispatched = DispatchMessageA(&m);
        DWORD e2 = GetLastError();
        row(key); flag("after", 1); sfield("dispatched", dispatched); field("dispatch_err", e2); end();
    }
}

static HANDLE other_ready, other_done;
static HWND other_window;
static DWORD WINAPI other_thread(LPVOID unused) {
    (void)unused;
    other_window = CreateWindowExW(0, (LPCWSTR)(ULONG_PTR)batch_atom_for_threads, L"other", 0, CW_USEDEFAULT,
                                   CW_USEDEFAULT, CW_USEDEFAULT, CW_USEDEFAULT, NULL, NULL, self, NULL);
    SetEvent(other_ready);
    WaitForSingleObject(other_done, INFINITE);
    return 0;
}

static void game_calls(void) {
    window_name = "game";
    current = NULL;
    messages = 0;
    SetLastError(SENTINEL);
    game_window = CreateWindowExW(0, (LPCWSTR)(ULONG_PTR)batch_atom_for_threads, L"Hail to the Rainbow", 0,
                                  CW_USEDEFAULT, CW_USEDEFAULT, CW_USEDEFAULT, CW_USEDEFAULT, NULL, NULL, self, NULL);
    row("create.game"); flag("null", game_window == NULL); field("messages", messages); field("err", GetLastError()); end();
    if (!game_window) return;

    /* seq 2592: SetWindowLongPtrA(window, GWLP_USERDATA, 1) answered 0 */
    SetLastError(SENTINEL);
    LONG_PTR before = SetWindowLongPtrA(game_window, GWLP_USERDATA, 1);
    DWORD e = GetLastError();
    row("game.user_data_a"); field("before", (ULONG_PTR)before); field("err", e);
    field("after", (ULONG_PTR)GetWindowLongPtrW(game_window, GWLP_USERDATA)); end();

    /* seq 2945-2952: the queue as the game first drains it */
    drain("queue.first");

    /* seq 3382: RegisterDeviceNotificationW(window, a device interface filter, DEVICE_NOTIFY_ALL_INTERFACE_CLASSES) */
    DEV_BROADCAST_DEVICEINTERFACE_W filter;
    for (unsigned j = 0; j < sizeof filter; j++) ((char *)&filter)[j] = 0;
    filter.dbcc_size = sizeof filter;
    filter.dbcc_devicetype = DBT_DEVTYP_DEVICEINTERFACE;
    SetLastError(SENTINEL);
    HDEVNOTIFY note = RegisterDeviceNotificationW(game_window, &filter,
                                                  DEVICE_NOTIFY_WINDOW_HANDLE | DEVICE_NOTIFY_ALL_INTERFACE_CLASSES);
    row("device_notification"); flag("null", note == NULL); field("err", GetLastError()); end();

    /* seq 3383-3395: GetFocus, then SetFocus on the window, and what the procedure is sent */
    SetLastError(SENTINEL);
    HWND focus = GetFocus();
    DWORD ef = GetLastError();
    row("focus.before"); flag("null", focus == NULL); flag("game", focus == game_window); field("err", ef); end();
    unsigned before_set = messages;
    SetLastError(SENTINEL);
    HWND previous = SetFocus(game_window);
    DWORD es = GetLastError();
    row("focus.set"); flag("previous_null", previous == NULL); flag("previous_game", previous == game_window);
    field("err", es); field("messages", messages - before_set); end();
    row("focus.after"); flag("game", GetFocus() == game_window); end();

    /* what the activation left in the queue (seq 3491-3493 dispatched one more message) */
    drain("queue.after_focus");

    /* seq 3423-3427: the game writes a cursor to a temporary file and loads it */
    WCHAR dir[MAX_PATH], path[MAX_PATH];
    GetTempPathW(MAX_PATH, dir);
    GetTempFileNameW(dir, L"w2m", 0, path);
    static unsigned char cur[22 + 40 + 32 * 32 * 4 + 32 * 32 / 8];
    unsigned short *h16 = (unsigned short *)cur;
    h16[0] = 0; h16[1] = 2; h16[2] = 1;                      /* ICONDIR: a cursor file, one image */
    unsigned char *entry = cur + 6;
    entry[0] = 32; entry[1] = 32;                            /* width, height */
    *(unsigned short *)(entry + 4) = 3; *(unsigned short *)(entry + 6) = 5;   /* the hot spot */
    *(DWORD *)(entry + 8) = sizeof cur - 22; *(DWORD *)(entry + 12) = 22;
    BITMAPINFOHEADER *bih = (BITMAPINFOHEADER *)(cur + 22);
    bih->biSize = 40; bih->biWidth = 32; bih->biHeight = 64; bih->biPlanes = 1; bih->biBitCount = 32;
    for (unsigned j = 22 + 40; j < 22 + 40 + 32 * 32 * 4; j += 4) cur[j + 3] = 0xFF;   /* opaque black */
    HANDLE f = CreateFileW(path, GENERIC_WRITE, 0, NULL, CREATE_ALWAYS, FILE_ATTRIBUTE_NORMAL, NULL);
    DWORD wrote = 0;
    WriteFile(f, cur, sizeof cur, &wrote, NULL);
    CloseHandle(f);
    SetLastError(SENTINEL);
    HANDLE cursor = LoadImageW(NULL, path, IMAGE_CURSOR, 0, 0, LR_LOADFROMFILE | LR_DEFAULTSIZE);
    DWORD ec = GetLastError();
    row("image.cursor_file"); flag("null", cursor == NULL); field("err", ec); field("written", wrote); end();
    SetLastError(SENTINEL);
    HANDLE again = LoadImageW(NULL, path, IMAGE_CURSOR, 0, 0, LR_LOADFROMFILE | LR_DEFAULTSIZE);
    DWORD ea = GetLastError();
    row("image.cursor_file_again"); flag("null", again == NULL); flag("same", again == cursor); field("err", ea); end();
    DeleteFileW(path);
    SetLastError(SENTINEL);
    HANDLE gone = LoadImageW(NULL, path, IMAGE_CURSOR, 0, 0, LR_LOADFROMFILE | LR_DEFAULTSIZE);
    DWORD eg = GetLastError();
    row("image.missing_file"); flag("null", gone == NULL); field("err", eg); end();
    SetLastError(SENTINEL);
    HANDLE no_resource = LoadImageW(self, MAKEINTRESOURCEW(999), IMAGE_ICON, 0, 0, LR_DEFAULTSIZE);
    DWORD er = GetLastError();
    row("image.missing_resource"); flag("null", no_resource == NULL); field("err", er); end();

    /* a message posted to the window, then WM_QUIT */
    SetLastError(SENTINEL);
    BOOL posted = PostMessageW(game_window, WM_USER + 1, 2, 3);
    DWORD ep = GetLastError();
    row("post.user"); field("ret", posted != 0); field("err", ep); end();
    drain("queue.posted");
    PostQuitMessage(5);
    drain("queue.quit");

    /* SetFocus on a window another thread made */
    other_ready = CreateEventW(NULL, TRUE, FALSE, NULL);
    other_done = CreateEventW(NULL, TRUE, FALSE, NULL);
    window_name = "other";
    current = NULL;
    messages = 0;
    HANDLE t = CreateThread(NULL, 0, other_thread, NULL, 0, NULL);
    WaitForSingleObject(other_ready, INFINITE);
    row("create.other"); flag("null", other_window == NULL); field("messages", messages); end();
    window_name = "game";
    SetLastError(SENTINEL);
    HWND cross = SetFocus(other_window);
    DWORD ex = GetLastError();
    row("focus.other_thread"); flag("previous_null", cross == NULL); flag("previous_game", cross == game_window);
    field("err", ex); flag("focus_still_game", GetFocus() == game_window); end();
    SetEvent(other_done);
    WaitForSingleObject(t, INFINITE);
}
