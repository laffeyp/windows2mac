// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* windows.c — window classes, the game's window and its procedure (Sprint 054, family "windows";
 * specs/work_list.json).
 *
 * contracts/windows.windows.windows records Windows (Server 2025 and 2022) under the game's manifest:
 *   - RegisterClassExW and RegisterClassW answer atoms from 0xC000 up, in order, the last error left; a class
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
 * A window here is a record in the guest runtime; nothing is drawn (milestone A runs the game in batch mode; a Mac
 * window is phase 6). Styles, messages and indexes the rows do not hold stop the run.
 */
#include "runtime.h"

typedef unsigned short u16;

#define ERROR_INVALID_INDEX 1413u
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

static struct window_class {
    u16 atom;
    u16 name[64];
    u64 instance, procedure, icon, cursor, background;
    u32 style;
    s32 extra;
} classes[CLASSES];
static u16 next_atom = 0xC000;

static struct window {
    u64 used;
    int cls;
    u64 procedure, instance, user_data;
    u32 style, ex_style;
    s32 rect[4], client[4];                  /* screen coordinates */
    u8 extra[EXTRA];
    u16 title[128];
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
            for (; s[n] && n < 63; n++) c->name[n] = s[n];
            c->name[n] = 0;
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

/* DefWindowProcW(hWnd, Msg, wParam, lParam): the four creation messages */
u64 user32_DefWindowProcW(u64 h, u64 msg, u64 wparam, u64 lparam) {
    struct window *w = window_of(h);
    if (!w) w2m_unimplemented(W2M_API_DefWindowProcW, h);
    switch ((u32)msg) {
    case WM_GETMINMAXINFO:
    case WM_CREATE:
        return w2m_logged(W2M_API_DefWindowProcW, msg, 0);
    case WM_NCCREATE: {
        const u16 *t = (const u16 *)*(const u64 *)(lparam + 56);
        int n = 0;
        if (t) for (; t[n] && n < 127; n++) w->title[n] = t[n];
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

u64 user32_SetWindowLongPtrW(u64 h, u64 index, u64 value) {
    struct window *w = window_of(h);
    if (!w) w2m_unimplemented(W2M_API_SetWindowLongPtrW, h);
    s64 i = (s64)(s32)index;
    if (i < 0 && i != GWLP_USERDATA) w2m_unimplemented(W2M_API_SetWindowLongPtrW, index);   /* style, procedure */
    u64 scratch = 0;
    u64 *p = long_slot(w, i, W2M_API_SetWindowLongPtrW, &scratch);
    if (!p) { set_error(ERROR_INVALID_INDEX); return w2m_logged(W2M_API_SetWindowLongPtrW, index, 0); }
    u64 before = *p;
    *p = value;
    return w2m_logged(W2M_API_SetWindowLongPtrW, index, before);
}

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
