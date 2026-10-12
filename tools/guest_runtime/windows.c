// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* windows.c — window classes, the game's window and its procedure (Sprint 054, family "windows";
 * specs/work_list.json).
 *
 * contracts/windows.windows.windows records Windows (Server 2025 and 2022) under the game's manifest:
 *   - RegisterClassExW and RegisterClassW answer atoms from 0xC000 up, in order, the last error left; a name of 256
 *     units or more answers 0 with ERROR_INVALID_PARAMETER (Sprint 056), one of 255 registers whole; a class
 *     registered again by the same module answers 0 with ERROR_CLASS_ALREADY_EXISTS (1410); CreateWindowExW of a
 *     class never registered NULL with ERROR_CANNOT_FIND_WND_CLASS (1407)
 *   - CreateWindowExW sends the window procedure four messages, in order: WM_GETMINMAXINFO (the maximized size, the
 *     screen and 8 more each side, at -8, -8; the tracking sizes 136 x 39 and 1044 x 788), WM_NCCREATE and
 *     WM_CREATE with a CREATESTRUCTW holding the arguments (cx and cy as given, CW_USEDEFAULT included; x and y
 *     where the window lands; the style as given; WS_EX_WINDOWEDGE added to the extended style when the style has
 *     a caption), and between them WM_NCCALCSIZE with the window's rectangle; then returns the window, the last
 *     error left; DefWindowProcW answers 0, 1 (the title taken), the client rectangle (8 in at the sides and bottom,
 *     31 at the top) and 0
 *   - a window gets WS_CLIPSIBLINGS and WS_CAPTION and WS_EX_WINDOWEDGE; CW_USEDEFAULT places it on the desktop's
 *     cascade (steps of 26, from a start the desktop's earlier windows set) and sizes it 3/4 of the work area's
 *     width by 3/4 of its height less 21; w2m's desktop starts the cascade at 52, the lowest start the runners showed
 *   - SetWindowLongPtrW and GetWindowLongPtrW reach the class's extra bytes (ERROR_INVALID_INDEX, 1413, past them)
 *     and GWLP_USERDATA, the last error left; GetWindowLongPtrW gives the style, extended style, instance and
 *     procedure; GetClientRect and GetWindowRect the rectangles; the window is not visible
 *   - LoadIconA finds an icon group by id in a module (the last error 0), and answers NULL with
 *     ERROR_RESOURCE_TYPE_NOT_FOUND (1813) for an id it lacks (the batch capture's exe had no resources at all:
 *     ERROR_RESOURCE_DATA_NOT_FOUND, 1812); LoadIconA(NULL, IDI_APPLICATION) is 0x1002B and LoadCursorA(NULL,
 *     IDC_ARROW) 0x10003, the same handle each time, the last error left
 * Sprint 063 (contracts/windows.windows.windows 0.3.0; the game's calls in batch run 38206263976, seq 2592-3497):
 *   - a window belongs to the thread that made it; each new window has WM_DWMNCRENDERINGCHANGED (0x31F, wParam 1)
 *     posted to it; PeekMessageA takes a thread's posted messages in order, WM_QUIT (PostQuitMessage) after them,
 *     and answers 0 on an empty queue, which sets the last error to 0 once the thread has activated a window and
 *     leaves it before; GetMessageExtraInfo is 0, TranslateMessage 0 for these messages; DispatchMessageA answers
 *     the window procedure's result; PostMessageW queues a message; the last error is left by each
 *   - SetWindowLongPtrA sets GWLP_USERDATA as SetWindowLongPtrW does
 *   - RegisterDeviceNotificationW(window, a device interface filter, DEVICE_NOTIFY_ALL_INTERFACE_CLASSES) answers a
 *     handle and sets the last error to 0
 *   - GetFocus is NULL before; SetFocus on a window of the thread that is not active sends, in order: 0x46
 *     (WINDOWPOS: flags 3), WM_ACTIVATEAPP to it and then to the thread's other windows, WM_NCACTIVATE, WM_ACTIVATE,
 *     then WM_KILLFOCUS (to itself), WM_IME_SETCONTEXT 0 and 1, WM_SETFOCUS (from itself); it answers the window,
 *     last error 0. DefWindowProcW's WM_ACTIVATE sends from inside WM_IME_SETCONTEXT (1, 0xC000000F) and
 *     WM_SETFOCUS (from none), and the window's first WM_IME_SETCONTEXT (1) sends WM_IME_NOTIFY (2) from inside its
 *     DefWindowProcW: the rows finish 16, 15, 17, 14. On another thread's window: NULL, error 5
 *   - LoadImageW(NULL, a .cur file, IMAGE_CURSOR, 0, 0, LR_LOADFROMFILE | LR_DEFAULTSIZE) answers a new cursor each
 *     time, last error 0; a missing file NULL with 2; LoadImageW of an icon the module lacks NULL with 1813
 * A message the rows do not hold, a filter, a key message to translate, or a window of another thread stops the run.
 * A window here is a record in the guest runtime; nothing is drawn (milestone A runs the game in batch mode; a Mac
 * window is phase 6). Styles, messages and indexes the rows do not hold stop the run.
 */
#include "runtime.h"

typedef unsigned short u16;

#define ERROR_INVALID_INDEX 1413u
#define ERROR_INVALID_PARAMETER 87u
#define ERROR_CLASS_ALREADY_EXISTS 1410u
#define ERROR_CANNOT_FIND_WND_CLASS 1407u
#define ERROR_RESOURCE_DATA_NOT_FOUND 1812u
#define ERROR_RESOURCE_TYPE_NOT_FOUND 1813u
#define RT_GROUP_ICON 14u
#define IDI_APPLICATION 32512u
#define IDC_ARROW 32512u
#define CW_USEDEFAULT ((s32)0x80000000)
#define WS_CLIPSIBLINGS 0x04000000u
#define WS_CAPTION 0x00C00000u
#define WS_ALLOWED 0x00CA0000u              /* WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX: the play run's style */
#define WS_EX_WINDOWEDGE 0x100u
#define WM_CREATE 0x01u
#define WM_ACTIVATE 0x06u
#define WM_SETFOCUS 0x07u
#define WM_KILLFOCUS 0x08u
#define WM_QUIT 0x12u
#define WM_ACTIVATEAPP 0x1Cu
#define WM_WINDOWPOSCHANGING 0x46u
#define WM_NCACTIVATE 0x86u
#define WM_KEYFIRST 0x100u
#define WM_KEYLAST 0x109u
#define WM_IME_SETCONTEXT 0x281u
#define WM_IME_NOTIFY 0x282u
#define WM_DWMNCRENDERINGCHANGED 0x31Fu
#define WM_USER 0x400u
#define PM_REMOVE 1u
#define ISC_SHOWUI 0xC000000Full              /* WM_IME_SETCONTEXT's lParam on the runners */
#define IMN_OPENSTATUSWINDOW 2u
#define DBT_DEVTYP_DEVICEINTERFACE 5u
#define DEVICE_NOTIFY_ALL_INTERFACE_CLASSES 4u
#define IMAGE_ICON 1u
#define IMAGE_CURSOR 2u
#define LR_LOADFROMFILE 0x10u
#define LR_DEFAULTSIZE 0x40u
#define ERROR_ACCESS_DENIED 5u
#define NOTIFY_BASE 0x50000ull               /* a device notification: NOTIFY_BASE + 0x10 * its number */
#define CURSOR_BASE 0x60000ull               /* a cursor from a file: CURSOR_BASE + 0x10 * its number */
#define CURSOR_FILE (64 << 10)               /* the largest cursor file read; a larger one stops the run */
#define WM_GETMINMAXINFO 0x24u
#define WM_NCCREATE 0x81u
#define WM_NCCALCSIZE 0x83u
#define GWLP_WNDPROC (-4)
#define GWLP_HINSTANCE (-6)
#define GWL_STYLE (-16)
#define GWL_EXSTYLE (-20)
#define GWLP_USERDATA (-21)
#define FRAME_SIDE 8                         /* SM_CXFRAME 4 and SM_CXPADDEDBORDER 4 */
#define FRAME_TOP 31                         /* the caption, 23, and the frame */
#define CASCADE_START 52
#define CASCADE_STEP 26
#define CLASSES 16
#define WINDOWS 16
#define EXTRA 64
#define WINDOW_BASE 0x20010ull               /* window handles: WINDOW_BASE + 0x10*slot */
#define ICON_BASE 0x30000ull                 /* an icon from a module: ICON_BASE | its group id */

#define CLASS_NAME 255                       /* the longest class name: 255 units register, 256 answer 87 (rows class.name_*;
                                              * Microsoft Learn's "maximum length ... is 256" counts the NUL) */
#define TITLE 512                            /* a title kept for the window; a longer one stops the run */

static struct window_class {
    u16 atom;
    u16 name[CLASS_NAME + 1];
    u64 instance, procedure, icon, cursor, background;
    u32 style;
    s32 extra;
} classes[CLASSES];
static u16 next_atom = 0xC000;

static struct window {
    u64 used;
    int cls;
    u64 procedure, instance, user_data;
    u64 owner;                               /* the thread that made it (Sprint 063) */
    u64 ime_status;                          /* its IME status window opened (WM_IME_NOTIFY sent once) */
    u32 style, ex_style;
    s32 rect[4], client[4];                  /* screen coordinates */
    u8 extra[EXTRA];
    u16 title[TITLE];
} windows[WINDOWS];
static s32 cascade = CASCADE_START;

int w2m_display_work_area(s32 r[4]);         /* display.c */

static void set_error(u32 code) { w2m_teb()->last_error_value = code; }
static u16 upper(u16 c) { return c >= 'a' && c <= 'z' ? (u16)(c - 32) : c; }

static int same_name(const u16 *a, const u16 *b) {
    while (*a && upper(*a) == upper(*b)) a++, b++;
    return upper(*a) == upper(*b);
}

/* The class a name or atom (a pointer below 0x10000) means, or -1. */
static int find_class(u64 name) {
    for (int i = 0; i < CLASSES; i++) {
        if (!classes[i].atom) continue;
        if (name < 0x10000 ? classes[i].atom == (u16)name : same_name(classes[i].name, (const u16 *)name)) return i;
    }
    return -1;
}

static struct window *window_of(u64 h) {
    if (h < WINDOW_BASE || (h - WINDOW_BASE) % 0x10 || (h - WINDOW_BASE) / 0x10 >= WINDOWS) return 0;
    struct window *w = &windows[(h - WINDOW_BASE) / 0x10];
    return w->used ? w : 0;
}

int w2m_is_window(u64 h) { return window_of(h) != 0; }
int w2m_window_rect(u64 h, s32 r[4]) {
    struct window *w = window_of(h);
    if (!w) return 0;
    for (int i = 0; i < 4; i++) r[i] = w->rect[i];
    return 1;
}

static u64 register_class(u64 api, u32 style, u64 procedure, s32 class_extra, s32 extra, u64 instance, u64 icon,
                          u64 cursor, u64 background, u64 name) {
    if (name < 0x10000 || class_extra || extra < 0 || extra > EXTRA) w2m_unimplemented(api, name);   /* not in rows */
    int k = find_class(name);
    if (k >= 0) {
        if (classes[k].instance != instance) w2m_unimplemented(api, instance);        /* another module's: no row */
        set_error(ERROR_CLASS_ALREADY_EXISTS);
        return w2m_logged(api, name, 0);
    }
    for (int i = 0; i < CLASSES; i++)
        if (!classes[i].atom) {
            struct window_class *c = &classes[i];
            const u16 *s = (const u16 *)name;
            int n = 0;
            while (s[n]) n++;
            if (n > CLASS_NAME) {
                set_error(ERROR_INVALID_PARAMETER);
                return w2m_logged(api, (u64)n, 0);
            }
            for (int i = 0; i <= n; i++) c->name[i] = s[i];               /* whole: a cut name is never found again */
            c->atom = next_atom++;
            c->style = style; c->procedure = procedure; c->extra = extra; c->instance = instance;
            c->icon = icon; c->cursor = cursor; c->background = background;
            return w2m_logged(api, name, c->atom);
        }
    w2m_unimplemented(api, CLASSES);
    return 0;
}

/* RegisterClassExW(lpwcx): WNDCLASSEXW, 80 bytes */
u64 user32_RegisterClassExW(u64 wc) {
    const u8 *c = (const u8 *)wc;
    if (*(const u32 *)c != 80) w2m_unimplemented(W2M_API_RegisterClassExW, *(const u32 *)c);
    return register_class(W2M_API_RegisterClassExW, *(const u32 *)(c + 4), *(const u64 *)(c + 8), *(const s32 *)(c + 16),
                          *(const s32 *)(c + 20), *(const u64 *)(c + 24), *(const u64 *)(c + 32), *(const u64 *)(c + 40),
                          *(const u64 *)(c + 48), *(const u64 *)(c + 64));
}

/* RegisterClassW(lpWndClass): WNDCLASSW, 72 bytes */
u64 user32_RegisterClassW(u64 wc) {
    const u8 *c = (const u8 *)wc;
    return register_class(W2M_API_RegisterClassW, *(const u32 *)c, *(const u64 *)(c + 8), *(const s32 *)(c + 16),
                          *(const s32 *)(c + 20), *(const u64 *)(c + 24), *(const u64 *)(c + 32), *(const u64 *)(c + 40),
                          *(const u64 *)(c + 48), *(const u64 *)(c + 64));
}

/* The posted messages, all threads' in one list in the order they were posted (Sprint 063). */
#define POSTED 64
#define THREADS 64
static struct posted { u64 order, tid, window, msg, wparam, lparam; } posted[POSTED];
static u64 next_order = 1;
static struct thread_state { u64 tid, quit, quit_code, focus, active; } threads[THREADS];

static u64 tid_now(void) { return w2m_teb()->client_thread; }

static struct thread_state *thread_state(u64 tid) {
    for (int i = 0; i < THREADS; i++) if (threads[i].tid == tid) return &threads[i];
    for (int i = 0; i < THREADS; i++)
        if (!threads[i].tid) { threads[i].tid = tid; return &threads[i]; }
    w2m_unimplemented(W2M_API_PeekMessageA, THREADS);
    return 0;
}

static void post(u64 tid, u64 window, u64 msg, u64 wparam, u64 lparam) {
    for (int i = 0; i < POSTED; i++)
        if (!posted[i].order) {
            posted[i] = (struct posted){next_order++, tid, window, msg, wparam, lparam};
            return;
        }
    w2m_unimplemented(W2M_API_PostMessageW, POSTED);
}

/* CreateWindowExW: the four messages in turn, through guest_calls.c */
struct create_frame {
    struct w2m_guest_call call;
    u64 rsp, window, step;
    s32 minmax[10];                          /* MINMAXINFO: reserved, max size, max position, min and max track */
    u8 create[80];                           /* CREATESTRUCTW */
    s32 rect[4];                             /* WM_NCCALCSIZE's rectangle */
};

static void create_step(struct w2m_guest_call *c, u64 answer) {
    struct create_frame *f = (struct create_frame *)c;
    struct window *w = window_of(f->window);
    switch (f->step++) {
    case 0:                                                           /* WM_GETMINMAXINFO answered */
        w2m_call_guest(c, w->procedure, f->window, WM_NCCREATE, 0, (u64)f->create);
        return;
    case 1:                                                           /* WM_NCCREATE */
        if (!answer) w2m_unimplemented(W2M_API_CreateWindowExW, WM_NCCREATE);    /* refused: no row */
        for (int i = 0; i < 4; i++) f->rect[i] = w->rect[i];
        w2m_call_guest(c, w->procedure, f->window, WM_NCCALCSIZE, 0, (u64)f->rect);
        return;
    case 2:                                                           /* WM_NCCALCSIZE: the client rectangle */
        for (int i = 0; i < 4; i++) w->client[i] = f->rect[i];
        w2m_call_guest(c, w->procedure, f->window, WM_CREATE, 0, (u64)f->create);
        return;
    default:                                                          /* WM_CREATE */
        if ((s64)answer == -1) w2m_unimplemented(W2M_API_CreateWindowExW, WM_CREATE);   /* refused: no row */
        post(w->owner, f->window, WM_DWMNCRENDERINGCHANGED, 1, 0);   /* rows queue.first.*: one for each window */
        w2m_return_to(f->rsp, f->window);
    }
}

/* CreateWindowExW(dwExStyle, lpClassName, lpWindowName, dwStyle, X, Y, nWidth, nHeight, hWndParent, hMenu,
 * hInstance, lpParam) */
W2M_VOID u64 user32_CreateWindowExW(u64 ex_style, u64 class_name, u64 title, u64 style, u64 rsp) {
    const u64 *stack = (const u64 *)(rsp + 0x28);
    s32 x = (s32)stack[0], y = (s32)stack[1], width = (s32)stack[2], height = (s32)stack[3];
    u64 parent = stack[4], menu = stack[5], instance = stack[6], param = stack[7];
    if ((u32)ex_style || ((u32)style & ~WS_ALLOWED) || parent || menu)
        w2m_unimplemented(W2M_API_CreateWindowExW, style | ex_style << 32);    /* not in the rows */
    int k = find_class(class_name);
    if (k < 0) {
        set_error(ERROR_CANNOT_FIND_WND_CLASS);
        w2m_return_to(rsp, w2m_logged(W2M_API_CreateWindowExW, class_name, 0));
        return 0;
    }
    int slot = -1;
    for (int i = 0; i < WINDOWS; i++) if (!windows[i].used) { slot = i; break; }
    if (slot < 0) w2m_unimplemented(W2M_API_CreateWindowExW, WINDOWS);
    struct window *w = &windows[slot];
    u8 *b = (u8 *)w;
    for (u64 i = 0; i < sizeof *w; i++) b[i] = 0;
    w->used = 1;
    w->cls = k;
    w->procedure = classes[k].procedure;
    w->instance = instance;
    w->owner = tid_now();
    w->style = (u32)style | WS_CLIPSIBLINGS | WS_CAPTION;
    w->ex_style = WS_EX_WINDOWEDGE;
    if (x == CW_USEDEFAULT) {
        x = y = cascade;
        cascade += CASCADE_STEP;
    }
    s32 left = x, top = y, cx = width, cy = height;
    if (width == CW_USEDEFAULT) {
        s32 work[4];
        w2m_display_work_area(work);
        cx = (work[2] - work[0]) * 3 / 4;
        cy = (work[3] - work[1]) * 3 / 4 - 21;
    }
    w->rect[0] = left; w->rect[1] = top; w->rect[2] = left + cx; w->rect[3] = top + cy;
    for (int i = 0; i < 4; i++) w->client[i] = w->rect[i];
    u64 h = WINDOW_BASE + 0x10 * (u64)slot;
    w2m_logged(W2M_API_CreateWindowExW, class_name, h);

    struct create_frame *f = (struct create_frame *)w2m_guest_frame(rsp, sizeof *f, create_step);
    f->rsp = rsp;
    f->window = h;
    s32 *mm = f->minmax;                     /* the screen's, not the work area's (rows msg.*.0) */
    mm[2] = 1024 + 2 * FRAME_SIDE; mm[3] = 768 + 2 * FRAME_SIDE;
    mm[4] = -FRAME_SIDE; mm[5] = -FRAME_SIDE;
    mm[6] = 136; mm[7] = 39;                 /* SM_CXMINTRACK, SM_CYMINTRACK */
    mm[8] = 1044; mm[9] = 788;               /* SM_CXMAXTRACK, SM_CYMAXTRACK */
    u8 *cs = f->create;
    *(u64 *)(cs + 0) = param;
    *(u64 *)(cs + 8) = instance;
    *(u64 *)(cs + 16) = menu;
    *(u64 *)(cs + 24) = parent;
    *(s32 *)(cs + 32) = height;              /* as given */
    *(s32 *)(cs + 36) = width;
    *(s32 *)(cs + 40) = y;
    *(s32 *)(cs + 44) = x;
    *(s32 *)(cs + 48) = (s32)style;
    *(u64 *)(cs + 56) = title;
    *(u64 *)(cs + 64) = class_name;
    *(u32 *)(cs + 72) = ((u32)style & WS_CAPTION) == WS_CAPTION ? WS_EX_WINDOWEDGE : 0;
    w2m_call_guest(&f->call, w->procedure, h, WM_GETMINMAXINFO, 0, (u64)f->minmax);
    return 0;
}

/* DefWindowProcW's answer for a message it answers without sending others */
static u64 default_answer(struct window *w, u64 h, u64 msg, u64 wparam, u64 lparam);

/* DefWindowProcW's WM_ACTIVATE and WM_IME_SETCONTEXT: the messages they send from inside, through guest_calls.c */
struct default_frame { struct w2m_guest_call call; u64 rsp, window, step, msg; };

static void default_step(struct w2m_guest_call *c, u64 answer) {
    (void)answer;
    struct default_frame *f = (struct default_frame *)c;
    struct window *w = window_of(f->window);
    u64 k = f->step++;
    if (f->msg == WM_ACTIVATE && k == 0) {
        w2m_call_guest(c, w->procedure, f->window, WM_IME_SETCONTEXT, 1, ISC_SHOWUI);
        return;
    }
    if (f->msg == WM_ACTIVATE && k == 1) {
        thread_state(tid_now())->focus = f->window;                       /* the focus WM_ACTIVATE sets */
        w2m_call_guest(c, w->procedure, f->window, WM_SETFOCUS, 0, 0);
        return;
    }
    w2m_return_to(f->rsp, w2m_logged(W2M_API_DefWindowProcW, f->msg, 0));
}

/* DefWindowProcW(hWnd, Msg, wParam, lParam) */
W2M_VOID u64 user32_DefWindowProcW(u64 h, u64 msg, u64 wparam, u64 lparam, u64 rsp) {
    struct window *w = window_of(h);
    if (!w) w2m_unimplemented(W2M_API_DefWindowProcW, h);
    int sends = ((u32)msg == WM_ACTIVATE && (u32)wparam == 1) ||
                ((u32)msg == WM_IME_SETCONTEXT && (u32)wparam == 1 && !w->ime_status);
    if (!sends) {
        w2m_return_to(rsp, default_answer(w, h, msg, wparam, lparam));
        return 0;
    }
    struct default_frame *f = (struct default_frame *)w2m_guest_frame(rsp, sizeof *f, default_step);
    f->rsp = rsp;
    f->window = h;
    f->msg = (u32)msg;
    if ((u32)msg == WM_IME_SETCONTEXT) {                                   /* the first: the status window opens */
        w->ime_status = 1;
        f->step = 2;
        w2m_call_guest(&f->call, w->procedure, h, WM_IME_NOTIFY, IMN_OPENSTATUSWINDOW, 0);
        return 0;
    }
    default_step(&f->call, 0);
    return 0;
}

static u64 default_answer(struct window *w, u64 h, u64 msg, u64 wparam, u64 lparam) {
    (void)h;
    switch ((u32)msg) {
    case WM_GETMINMAXINFO:
    case WM_CREATE:
    case WM_ACTIVATE:                                          /* Sprint 063: the rows msg.game.* answers */
    case WM_SETFOCUS:
    case WM_KILLFOCUS:
    case WM_ACTIVATEAPP:
    case WM_WINDOWPOSCHANGING:
    case WM_IME_SETCONTEXT:
    case WM_IME_NOTIFY:
    case WM_DWMNCRENDERINGCHANGED:
    case WM_USER + 1:
        return w2m_logged(W2M_API_DefWindowProcW, msg, 0);
    case WM_NCACTIVATE:
        return w2m_logged(W2M_API_DefWindowProcW, msg, 1);
    case WM_NCCREATE: {
        const u16 *t = (const u16 *)*(const u64 *)(lparam + 56);
        int n = 0;
        if (t) while (t[n]) n++;
        if (n >= TITLE) w2m_unimplemented(W2M_API_DefWindowProcW, (u64)n);   /* kept whole or not at all */
        for (int i = 0; i < n; i++) w->title[i] = t[i];
        w->title[n] = 0;
        return w2m_logged(W2M_API_DefWindowProcW, msg, 1);
    }
    case WM_NCCALCSIZE: {
        if (wparam) w2m_unimplemented(W2M_API_DefWindowProcW, 0x100 | msg);        /* NCCALCSIZE_PARAMS: no row */
        s32 *r = (s32 *)lparam;
        if ((w->style & WS_CAPTION) == WS_CAPTION) {
            r[0] += FRAME_SIDE; r[1] += FRAME_TOP; r[2] -= FRAME_SIDE; r[3] -= FRAME_SIDE;
        }
        return w2m_logged(W2M_API_DefWindowProcW, msg, 0);
    }
    default:
        w2m_unimplemented(W2M_API_DefWindowProcW, msg);                            /* not in the rows */
        return 0;
    }
}

static u64 *long_slot(struct window *w, s64 index, u64 api, u64 *scratch) {
    if (index >= 0) {
        if (index + 8 > classes[w->cls].extra) return 0;
        return (u64 *)(w->extra + index);
    }
    switch (index) {
    case GWLP_USERDATA: return &w->user_data;
    case GWLP_WNDPROC: *scratch = w->procedure; return scratch;
    case GWLP_HINSTANCE: *scratch = w->instance; return scratch;
    case GWL_STYLE: *scratch = w->style; return scratch;
    case GWL_EXSTYLE: *scratch = w->ex_style; return scratch;
    default: w2m_unimplemented(api, (u64)index); return 0;
    }
}

u64 user32_GetWindowLongPtrW(u64 h, u64 index) {
    struct window *w = window_of(h);
    if (!w) w2m_unimplemented(W2M_API_GetWindowLongPtrW, h);
    u64 scratch = 0;
    u64 *p = long_slot(w, (s64)(s32)index, W2M_API_GetWindowLongPtrW, &scratch);
    if (!p) { set_error(ERROR_INVALID_INDEX); return w2m_logged(W2M_API_GetWindowLongPtrW, index, 0); }
    return w2m_logged(W2M_API_GetWindowLongPtrW, index, *p);
}

static u64 set_long(u64 api, u64 h, u64 index, u64 value) {
    struct window *w = window_of(h);
    if (!w) w2m_unimplemented(api, h);
    s64 i = (s64)(s32)index;
    if (i < 0 && i != GWLP_USERDATA) w2m_unimplemented(api, index);                     /* style, procedure */
    u64 scratch = 0;
    u64 *p = long_slot(w, i, api, &scratch);
    if (!p) { set_error(ERROR_INVALID_INDEX); return w2m_logged(api, index, 0); }
    u64 before = *p;
    *p = value;
    return w2m_logged(api, index, before);
}

u64 user32_SetWindowLongPtrW(u64 h, u64 index, u64 value) { return set_long(W2M_API_SetWindowLongPtrW, h, index, value); }
/* the A form: GWLP_USERDATA and extra bytes hold no text, so as the W form (row game.user_data_a) */
u64 user32_SetWindowLongPtrA(u64 h, u64 index, u64 value) { return set_long(W2M_API_SetWindowLongPtrA, h, index, value); }

u64 user32_GetClientRect(u64 h, u64 out) {
    struct window *w = window_of(h);
    if (!w) w2m_unimplemented(W2M_API_GetClientRect, h);
    s32 *r = (s32 *)out;
    r[0] = 0; r[1] = 0; r[2] = w->client[2] - w->client[0]; r[3] = w->client[3] - w->client[1];
    return w2m_logged(W2M_API_GetClientRect, h, 1);
}

u64 user32_GetWindowRect(u64 h, u64 out) {
    struct window *w = window_of(h);
    if (!w) w2m_unimplemented(W2M_API_GetWindowRect, h);
    s32 *r = (s32 *)out;
    for (int i = 0; i < 4; i++) r[i] = w->rect[i];
    return w2m_logged(W2M_API_GetWindowRect, h, 1);
}

u64 user32_IsWindowVisible(u64 h) {
    if (!window_of(h)) w2m_unimplemented(W2M_API_IsWindowVisible, h);
    return w2m_logged(W2M_API_IsWindowVisible, h, 0);                   /* no WS_VISIBLE, no ShowWindow yet */
}

/* LoadIconA(hInstance, lpIconName): an icon group of a module by id, or IDI_APPLICATION */
u64 user32_LoadIconA(u64 instance, u64 name) {
    if (name >= 0x10000) w2m_unimplemented(W2M_API_LoadIconA, name);                 /* by name: no row */
    if (!instance) {
        if ((u32)name != IDI_APPLICATION) w2m_unimplemented(W2M_API_LoadIconA, name);
        return w2m_logged(W2M_API_LoadIconA, name, 0x1002B);
    }
    u32 size = 0, missing = 0;
    if (!w2m_resource(instance, RT_GROUP_ICON, (u32)name, &size, &missing)) {
        set_error(missing == W2M_RESOURCE_NO_DIRECTORY ? ERROR_RESOURCE_DATA_NOT_FOUND : ERROR_RESOURCE_TYPE_NOT_FOUND);
        return w2m_logged(W2M_API_LoadIconA, name, 0);
    }
    set_error(0);
    return w2m_logged(W2M_API_LoadIconA, name, ICON_BASE | (u32)name);
}

u64 user32_LoadCursorA(u64 instance, u64 name) {
    if (instance || (u32)name != IDC_ARROW) w2m_unimplemented(W2M_API_LoadCursorA, name);   /* not in the rows */
    return w2m_logged(W2M_API_LoadCursorA, name, 0x10003);
}

/* ---- the message queue (Sprint 063) --------------------------------------------------------------------------- */

/* PeekMessageA(lpMsg, hWnd, wMsgFilterMin, wMsgFilterMax, wRemoveMsg): the calling thread's first posted message,
 * then WM_QUIT; no window or range filter is in the rows */
u64 user32_PeekMessageA(u64 out, u64 h, u64 low, u64 high, u64 rsp) {
    u32 remove = (u32)*(const u64 *)(rsp + 0x28);
    if (h || low || high || (remove & ~PM_REMOVE)) w2m_unimplemented(W2M_API_PeekMessageA, h | low << 32 | (u64)remove << 48);
    u64 tid = tid_now();
    struct thread_state *t = thread_state(tid);
    struct posted *first = 0;
    for (int i = 0; i < POSTED; i++)
        if (posted[i].order && posted[i].tid == tid && (!first || posted[i].order < first->order)) first = &posted[i];
    u8 *m = (u8 *)out;
    if (!first && !t->quit) {
        if (t->active) set_error(0);         /* queue.after_focus.00: 0 after activation on 4 of 4 executions (the rows keep it
                                              * only before, where it is left: the system's own messages vary it) */
        return w2m_logged(W2M_API_PeekMessageA, remove, 0);
    }
    for (int i = 0; i < 48; i++) m[i] = 0;   /* MSG: hwnd, message, wParam, lParam, time, pt, lPrivate */
    if (first) {
        *(u64 *)m = first->window; *(u32 *)(m + 8) = (u32)first->msg;
        *(u64 *)(m + 16) = first->wparam; *(u64 *)(m + 24) = first->lparam;
        if (remove & PM_REMOVE) first->order = 0;
    } else {
        *(u32 *)(m + 8) = WM_QUIT; *(u64 *)(m + 16) = t->quit_code;
        if (remove & PM_REMOVE) t->quit = 0;
    }
    return w2m_logged(W2M_API_PeekMessageA, remove, 1);
}

u64 user32_GetMessageExtraInfo(void) { return w2m_logged(W2M_API_GetMessageExtraInfo, 0, 0); }

/* TranslateMessage: only key messages make characters, and none is in the rows */
u64 user32_TranslateMessage(u64 msg) {
    u32 m = *(const u32 *)(msg + 8);
    if (m >= WM_KEYFIRST && m <= WM_KEYLAST) w2m_unimplemented(W2M_API_TranslateMessage, m);
    return w2m_logged(W2M_API_TranslateMessage, m, 0);
}

struct dispatch_frame { struct w2m_guest_call call; u64 rsp; };
static void dispatch_done(struct w2m_guest_call *c, u64 answer) {
    w2m_return_to(((struct dispatch_frame *)c)->rsp, w2m_logged(W2M_API_DispatchMessageA, 0, answer));
}

/* DispatchMessageA(lpMsg): the window procedure's answer. The messages the rows hold carry no text, so the A form
 * passes them as they are. */
W2M_VOID u64 user32_DispatchMessageA(u64 msg, u64 a1, u64 a2, u64 a3, u64 rsp) {
    (void)a1, (void)a2, (void)a3;
    const u8 *m = (const u8 *)msg;
    u64 h = *(const u64 *)m;
    u32 id = *(const u32 *)(m + 8);
    struct window *w = window_of(h);
    if (!w || w->owner != tid_now()) w2m_unimplemented(W2M_API_DispatchMessageA, h);   /* thread messages: no row */
    if (id != WM_DWMNCRENDERINGCHANGED && id < WM_USER) w2m_unimplemented(W2M_API_DispatchMessageA, id);
    struct dispatch_frame *f = (struct dispatch_frame *)w2m_guest_frame(rsp, sizeof *f, dispatch_done);
    f->rsp = rsp;
    w2m_call_guest(&f->call, w->procedure, h, id, *(const u64 *)(m + 16), *(const u64 *)(m + 24));
    return 0;
}

u64 user32_PostMessageW(u64 h, u64 msg, u64 wparam, u64 lparam) {
    struct window *w = window_of(h);
    if (!w || (u32)msg < WM_USER) w2m_unimplemented(W2M_API_PostMessageW, msg);   /* system messages: no row */
    post(w->owner, h, (u32)msg, wparam, lparam);
    return w2m_logged(W2M_API_PostMessageW, msg, 1);
}

W2M_VOID u64 user32_PostQuitMessage(u64 code) {
    struct thread_state *t = thread_state(tid_now());
    t->quit = 1;
    t->quit_code = (u64)(s64)(s32)code;
    return w2m_logged(W2M_API_PostQuitMessage, code, 0);
}

/* RegisterDeviceNotificationW(hRecipient, NotificationFilter, Flags) */
u64 user32_RegisterDeviceNotificationW(u64 recipient, u64 filter, u64 flags) {
    static u64 count;
    if (!window_of(recipient) || (u32)flags != DEVICE_NOTIFY_ALL_INTERFACE_CLASSES ||
        *(const u32 *)(filter + 4) != DBT_DEVTYP_DEVICEINTERFACE)
        w2m_unimplemented(W2M_API_RegisterDeviceNotificationW, flags);   /* another recipient or filter: no row */
    set_error(0);
    return w2m_logged(W2M_API_RegisterDeviceNotificationW, flags, NOTIFY_BASE + 0x10 * ++count);
}

u64 user32_GetFocus(void) { return w2m_logged(W2M_API_GetFocus, 0, thread_state(tid_now())->focus); }

/* SetFocus: the activation and focus messages in turn, through guest_calls.c */
#define FOCUS_STEPS (14 + WINDOWS)
struct focus_frame {
    struct w2m_guest_call call;
    u64 rsp, window, step, steps;
    u64 to[FOCUS_STEPS], msg[FOCUS_STEPS], wparam[FOCUS_STEPS], lparam[FOCUS_STEPS];
    u8 pos[40];                               /* WINDOWPOS: hwnd, hwndInsertAfter, x, y, cx, cy, flags */
};

static void focus_step(struct w2m_guest_call *c, u64 answer) {
    (void)answer;
    struct focus_frame *f = (struct focus_frame *)c;
    if (f->step == f->steps) {
        struct thread_state *t = thread_state(tid_now());
        t->active = t->focus = f->window;
        set_error(0);                                                      /* row focus.set */
        w2m_return_to(f->rsp, w2m_logged(W2M_API_SetFocus, f->window, f->window));   /* the focus WM_ACTIVATE set */
        return;
    }
    u64 k = f->step++;
    struct window *w = window_of(f->to[k]);
    w2m_call_guest(c, w->procedure, f->to[k], f->msg[k], f->wparam[k], f->lparam[k]);
}

W2M_VOID u64 user32_SetFocus(u64 h, u64 a1, u64 a2, u64 a3, u64 rsp) {
    (void)a1, (void)a2, (void)a3;
    struct window *w = window_of(h);
    if (!w) w2m_unimplemented(W2M_API_SetFocus, h);                         /* NULL or not a window: no row */
    u64 tid = tid_now();
    if (w->owner != tid) {                                                  /* row focus.other_thread */
        set_error(ERROR_ACCESS_DENIED);
        w2m_return_to(rsp, w2m_logged(W2M_API_SetFocus, h, 0));
        return 0;
    }
    if (thread_state(tid)->active) w2m_unimplemented(W2M_API_SetFocus, h);  /* an active window again: no row */
    struct focus_frame *f = (struct focus_frame *)w2m_guest_frame(rsp, sizeof *f, focus_step);
    f->rsp = rsp;
    f->window = h;
    *(u64 *)(f->pos + 0) = h;
    *(u64 *)(f->pos + 8) = 1;                /* hwndInsertAfter: not NULL (row msg.game.8); its value is not in the rows */
    *(u32 *)(f->pos + 32) = 3;               /* SWP_NOSIZE | SWP_NOMOVE */
    u64 n = 0;
#define SEND(T, M, W, L) (f->to[n] = (T), f->msg[n] = (M), f->wparam[n] = (W), f->lparam[n] = (L), n++)
    SEND(h, WM_WINDOWPOSCHANGING, 0, (u64)f->pos);
    SEND(h, WM_ACTIVATEAPP, 1, 1);           /* lParam: the deactivated program's thread, not NULL (rows) */
    for (int i = 0; i < WINDOWS; i++) {
        u64 other = WINDOW_BASE + 0x10 * (u64)i;
        if (windows[i].used && other != h && windows[i].owner == tid) SEND(other, WM_ACTIVATEAPP, 1, 1);
    }
    SEND(h, WM_NCACTIVATE, 1, 0);
    SEND(h, WM_ACTIVATE, 1, 0);              /* WA_ACTIVE: DefWindowProcW sends the IME pair and WM_SETFOCUS */
    SEND(h, WM_KILLFOCUS, h, 0);
    SEND(h, WM_IME_SETCONTEXT, 0, ISC_SHOWUI);
    SEND(h, WM_IME_SETCONTEXT, 1, ISC_SHOWUI);
    SEND(h, WM_SETFOCUS, h, 0);
#undef SEND
    f->steps = n;
    focus_step(&f->call, 0);
    return 0;
}

/* ---- LoadImageW (Sprint 063) ----------------------------------------------------------------------------------- */

/* A .cur file as the rows' was written: an ICONDIR of type 2 and its entries, each image inside the file. */
static int cursor_file(const u8 *b, u64 n) {
    if (n < 6 || *(const u16 *)b != 0 || *(const u16 *)(b + 2) != IMAGE_CURSOR || !*(const u16 *)(b + 4)) return 0;
    u64 count = *(const u16 *)(b + 4);
    if (6 + 16 * count > n) return 0;
    for (u64 i = 0; i < count; i++) {
        const u8 *e = b + 6 + 16 * i;
        u64 size = *(const u32 *)(e + 8), at = *(const u32 *)(e + 12);
        if (!size || at + size > n) return 0;
    }
    return 1;
}

/* LoadImageW(hInst, name, type, cx, cy, fuLoad) */
u64 user32_LoadImageW(u64 instance, u64 name, u64 type, u64 cx, u64 rsp) {
    static u8 file[CURSOR_FILE];
    static u64 cursors;
    u64 cy = *(const u64 *)(rsp + 0x28);
    u32 load = (u32)*(const u64 *)(rsp + 0x30);
    if (load & LR_LOADFROMFILE) {
        if (instance || (u32)type != IMAGE_CURSOR || (u32)cx || (u32)cy || (load & ~(LR_LOADFROMFILE | LR_DEFAULTSIZE)))
            w2m_unimplemented(W2M_API_LoadImageW, load | (u64)(u32)type << 32);   /* not in the rows */
        u64 size = 0;
        u32 error = w2m_file_read_all((const u16 *)name, file, sizeof file, &size);
        if (error) {
            set_error(error);                                                  /* row image.missing_file */
            return w2m_logged(W2M_API_LoadImageW, load, 0);
        }
        if (!cursor_file(file, size)) w2m_unimplemented(W2M_API_LoadImageW, size);   /* another file: no row */
        set_error(0);
        return w2m_logged(W2M_API_LoadImageW, load, CURSOR_BASE + 0x10 * ++cursors);  /* a new cursor each time */
    }
    if (!instance || name >= 0x10000 || (u32)type != IMAGE_ICON)
        w2m_unimplemented(W2M_API_LoadImageW, (u64)(u32)type << 32 | load);
    u32 size = 0, missing = 0;
    if (w2m_resource(instance, RT_GROUP_ICON, (u32)name, &size, &missing))
        w2m_unimplemented(W2M_API_LoadImageW, name);                         /* an icon the module has: no row */
    set_error(missing == W2M_RESOURCE_NO_DIRECTORY ? ERROR_RESOURCE_DATA_NOT_FOUND : ERROR_RESOURCE_TYPE_NOT_FOUND);
    return w2m_logged(W2M_API_LoadImageW, name, 0);                          /* row image.missing_resource */
}
