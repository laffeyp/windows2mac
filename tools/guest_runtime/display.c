// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* display.c — the display as the game queries it (Sprint 054, family "display"; specs/work_list.json).
 *
 * From Sprint 383 the display is the laptop's (the Decision "Which machine-decided values w2m answers as": display
 * and dxgi the laptop's values, 1920x1080 at 120 DPI), as contracts/windows.display.display 0.3.0 records it under
 * the game's manifest (PerMonitorV2), laptop runs lap-20261013T231059422Z-136bec80-034 and -035 (the runners' two
 * binaries, all four bodies byte-equal): one monitor of 1920 x 1080 at an effective 120 DPI on "Intel(R) UHD
 * Graphics", the work area above the taskbar ending at 1020. Until Sprint 383 it was the runners' (Server 2025's
 * 1024 x 768 at 96 DPI on the Hyper-V adapter, contract 0.2.0). The game's window is not yet a Mac window (phase 6):
 *   - GetSystemMetrics: the laptop's value for each index 0-99 (17, SM_CYFULLSCREEN, is the work area's height less
 *     the caption, 29; 62, SM_CYMAXIMIZED, the work area's height and 18; 43, SM_CMOUSEBUTTONS, 2; 75,
 *     SM_MOUSEWHEELPRESENT, 0); index 93 has no SM_ name and answered 5852, 6552, 7696 and 8000 on the runners and
 *     1348 in the four laptop bodies, here 1348; SM_REMOTESESSION and SM_SHUTTINGDOWN 0
 *   - EnumDisplayDevicesA: adapter 0 is \\.\DISPLAY12, "Intel(R) UHD Graphics", state 5, with one monitor,
 *     \\.\DISPLAY12\Monitor0, "Generic PnP Monitor", state 3; no other monitor (FALSE, the last error left). The
 *     laptop answers adapters 1 and 2 too (\\.\DISPLAY13 and \\.\DISPLAY14, state 0), and the game, told TRUE at 1,
 *     goes on to 2, 3, 4 and 5, whose contents no stored row holds (3 and 4 TRUE on the laptop); so no adapter past
 *     0 answers TRUE here (FALSE, the last error left), as on the runners, waiting on a laptop probe that asks for
 *     them (BLACKBOARD Surfaced, Sprint 379's Smart App Control question). The probe's rows write a backslash as
 *     "_": the ids below take the PnP form the runners' Hyper-V ids had (VMBUS\{...}), PCI\VEN_8086&DEV_A7A8&...
 *   - EnumDisplayMonitors(NULL, NULL) calls back once with monitor 0x10001, a NULL DC, the screen and the data;
 *     GetMonitorInfoA and W fill MONITORINFO (40 bytes) or MONITORINFOEX (72, 104) with the screen, the work area,
 *     MONITORINFOF_PRIMARY and \\.\DISPLAY12; another size is ERROR_INVALID_PARAMETER, another monitor
 *     ERROR_INVALID_MONITOR_HANDLE (1461)
 *   - MonitorFrom{Point,Rect,Window}: the monitor for a point, rectangle or window on it, else NULL for
 *     MONITOR_DEFAULTTONULL and the monitor for the others; a NULL window as a point off every monitor
 *   - shcore's GetDpiForMonitor: S_OK with 120 x 120 for MDT_EFFECTIVE_DPI, 123 x 123 for MDT_ANGULAR_DPI and 141 x
 *     141 for MDT_RAW_DPI (the laptop's panel reports its size); another type HRESULT_FROM_WIN32(ERROR_BAD_ARGUMENTS)
 *     (0x800700A0), another monitor HRESULT_FROM_WIN32(ERROR_INVALID_HANDLE) (0x80070006), both leaving x and y
 *   - EnumDisplaySettingsW(NULL): the current and registry settings 1920 x 1080 x 32 at 144 Hz as "CDD", and 66 modes
 *     as "cdd": 1920 x 1080 at 144 and 60 Hz, then each of 16 smaller sizes at 144 and 60 Hz, each first with
 *     dmDisplayFixedOutput 0 and then 1 (DM_DISPLAYFIXEDOUTPUT in dmFields); dmSize answers 188
 *   - EnumDisplaySettingsW(L"\\.\DISPLAY12"), in any case, answers as NULL does (the game names the device
 *     MONITORINFOEXW.szDevice gave it); another device name, \\.\DISPLAY1 and the monitor's name included, answers
 *     FALSE and leaves the last error (rows settings.named.*: the laptop's adapter is not \\.\DISPLAY1)
 * The answers are the PerMonitorV2 view, the game's and the display probe's manifest's. A process without that
 * awareness sees the display scaled to 96 DPI (1536 x 864 here); no stored laptop row gives these calls' answers to one
 * (not measured), and no family run that makes them lacks the manifest; DXGI's output description is the one such
 * answer a row gives (dxgi.c). Other forms (a DC or clip rectangle, EDD_GET_DEVICE_INTERFACE_NAME, a callback that
 * stops the enumeration) are not in the rows and stop the run.
 */
#include "runtime.h"

typedef unsigned short u16;

#define MONITOR 0x10001ull
#define SCREEN_W 1920
#define SCREEN_H 1080
#define WORK_H 1020
#define ERROR_INVALID_PARAMETER 87u
#define ERROR_INVALID_MONITOR_HANDLE 1461u
#define MONITOR_DEFAULTTONULL 0u

/* The laptop's GetSystemMetrics(0..99) (rows metric.N, lap-20261013T231059422Z-136bec80-034 and -035) */
static const s32 METRICS[100] = {
    1920, 1080, 21, 21, 29, 1, 1, 3, 3, 21, 21, 40, 40, 32, 32, 25, 1920, WORK_H - 29, 0, 1,
    21, 21, 0, 0, 0, 0, 0, 0, 166, 47, 46, 28, 4, 4, 166, 47, 4, 4, 94, 94,
    0, 0, 0, 2, 0, 2, 2, 199, 34, 20, 20, 29, 28, 28, 24, 24, 8, 199, 34, 1942,
    1102, 1938, WORK_H + 18, 3, 0, 0, 0, 0, 4, 4, 0, 19, 19, 0, 0, 0, 0, 0, 1920, 1080,
    1, 1, 1, 1, 1, 4, 0, 0, 0, 0, 0, 0, 5, 1348, 0, 0, 0, 0, 0, 0};

static void set_error(u32 code) { w2m_teb()->last_error_value = code; }

int w2m_display_work_area(s32 r[4]) { r[0] = 0; r[1] = 0; r[2] = SCREEN_W; r[3] = WORK_H; return 1; }
u64 w2m_display_monitor(void) { return MONITOR; }

u64 user32_GetSystemMetrics(u64 index) {
    s32 i = (s32)index;
    s32 v;
    if (i >= 0 && i < 100) v = METRICS[i];
    else if (i == 0x1000 || i == 0x2001) v = 0;                     /* SM_REMOTESESSION, SM_SHUTTINGDOWN */
    else { w2m_unimplemented(W2M_API_GetSystemMetrics, index); v = 0; }
    return w2m_logged(W2M_API_GetSystemMetrics, index, (u64)(s64)v);
}

static void copy(char *to, const char *from, u32 room) {
    u32 i = 0;
    for (; from[i] && i < room - 1; i++) to[i] = from[i];
    for (; i < room; i++) to[i] = 0;
}

static int same(const char *a, const char *b) {
    while (*a && *a == *b) a++, b++;
    return *a == *b;
}

/* EnumDisplayDevicesA(lpDevice, iDevNum, lpDisplayDevice, dwFlags); DISPLAY_DEVICEA is 424 bytes: cb, DeviceName[32],
 * DeviceString[128], StateFlags, DeviceID[128], DeviceKey[128] */
u64 user32_EnumDisplayDevicesA(u64 device, u64 index, u64 out, u64 flags) {
    if ((u32)flags) w2m_unimplemented(W2M_API_EnumDisplayDevicesA, flags);       /* not in the rows */
    char *d = (char *)out;
    const char *dev = (const char *)device;
    if (!dev && (u32)index == 0) {
        copy(d + 4, "\\\\.\\DISPLAY12", 32);
        copy(d + 36, "Intel(R) UHD Graphics", 128);
        *(u32 *)(d + 164) = 5;                     /* DISPLAY_DEVICE_ATTACHED_TO_DESKTOP | PRIMARY_DEVICE */
        copy(d + 168, "PCI\\VEN_8086&DEV_A7A8&SUBSYS_8DCD103C&REV_04", 128);
        copy(d + 296, "\\Registry\\Machine\\System\\CurrentControlSet\\Control\\Video\\"
                      "{18AD10DA-71E4-11F1-A6D5-806E6F6E6963}\\0000", 128);
        return w2m_logged(W2M_API_EnumDisplayDevicesA, index, 1);
    }
    if (dev && same(dev, "\\\\.\\DISPLAY12") && (u32)index == 0) {
        copy(d + 4, "\\\\.\\DISPLAY12\\Monitor0", 32);
        copy(d + 36, "Generic PnP Monitor", 128);
        *(u32 *)(d + 164) = 3;                     /* DISPLAY_DEVICE_ACTIVE | ATTACHED */
        copy(d + 168, "MONITOR\\BOE0CFD\\{4d36e96e-e325-11ce-bfc1-08002be10318}\\0002", 128);
        copy(d + 296, "\\Registry\\Machine\\System\\CurrentControlSet\\Control\\Class\\"
                      "{4d36e96e-e325-11ce-bfc1-08002be10318}\\0002", 128);
        return w2m_logged(W2M_API_EnumDisplayDevicesA, index, 1);
    }
    /* adapters 1 to 5 of the laptop wait on a row for 3 and 4 (the header); no other device or monitor */
    return w2m_logged(W2M_API_EnumDisplayDevicesA, index, 0);
}

/* EnumDisplayMonitors(hdc, lprcClip, lpfnEnum, dwData): one callback, through guest_calls.c */
struct monitors_frame {
    struct w2m_guest_call call;
    u64 rsp;
    s32 rect[4];
};

static void monitors_done(struct w2m_guest_call *c, u64 answer) {
    struct monitors_frame *f = (struct monitors_frame *)c;
    if (!(u32)answer) w2m_unimplemented(W2M_API_EnumDisplayMonitors, 0);         /* a callback that stops: no row */
    w2m_return_to(f->rsp, 1);
}

W2M_VOID u64 user32_EnumDisplayMonitors(u64 dc, u64 clip, u64 proc, u64 data, u64 rsp) {
    if (dc || clip || !proc) w2m_unimplemented(W2M_API_EnumDisplayMonitors, dc | clip);
    w2m_logged(W2M_API_EnumDisplayMonitors, proc, data);
    struct monitors_frame *f = (struct monitors_frame *)w2m_guest_frame(rsp, sizeof *f, monitors_done);
    f->rsp = rsp;
    f->rect[2] = SCREEN_W;
    f->rect[3] = SCREEN_H;
    w2m_call_guest(&f->call, proc, MONITOR, 0, (u64)f->rect, data);
    return 0;
}

static const u16 DEVICE_W[] = {'\\', '\\', '.', '\\', 'D', 'I', 'S', 'P', 'L', 'A', 'Y', '1', '2', 0};

static u64 monitor_info(u64 api, u64 monitor, u64 info, int wide) {
    u8 *m = (u8 *)info;
    u32 size = *(u32 *)m;
    if (monitor != MONITOR) { set_error(ERROR_INVALID_MONITOR_HANDLE); return w2m_logged(api, monitor, 0); }
    if (size != 40 && size != (wide ? 104u : 72u)) { set_error(ERROR_INVALID_PARAMETER); return w2m_logged(api, size, 0); }
    s32 *r = (s32 *)(m + 4);                           /* rcMonitor, rcWork */
    r[0] = 0; r[1] = 0; r[2] = SCREEN_W; r[3] = SCREEN_H;
    r[4] = 0; r[5] = 0; r[6] = SCREEN_W; r[7] = WORK_H;
    *(u32 *)(m + 36) = 1;                              /* MONITORINFOF_PRIMARY */
    if (size > 40) {
        if (wide) for (int i = 0; i < 32; i++) ((u16 *)(m + 40))[i] = i < 14 ? DEVICE_W[i] : 0;
        else copy((char *)(m + 40), "\\\\.\\DISPLAY12", 32);
    }
    return w2m_logged(api, monitor, 1);
}

u64 user32_GetMonitorInfoA(u64 monitor, u64 info) { return monitor_info(W2M_API_GetMonitorInfoA, monitor, info, 0); }
u64 user32_GetMonitorInfoW(u64 monitor, u64 info) { return monitor_info(W2M_API_GetMonitorInfoW, monitor, info, 1); }

static u64 outside(u64 api, u32 flags) {
    if (flags > 2) w2m_unimplemented(api, flags);
    return w2m_logged(api, flags, flags == MONITOR_DEFAULTTONULL ? 0 : MONITOR);
}

/* MonitorFromPoint(pt, dwFlags): the POINT comes by value, x in the low half */
u64 user32_MonitorFromPoint(u64 point, u64 flags) {
    s32 x = (s32)point, y = (s32)(point >> 32);
    if (x >= 0 && y >= 0 && x < SCREEN_W && y < SCREEN_H) return w2m_logged(W2M_API_MonitorFromPoint, point, MONITOR);
    return outside(W2M_API_MonitorFromPoint, (u32)flags);
}

u64 user32_MonitorFromRect(u64 rect, u64 flags) {
    const s32 *r = (const s32 *)rect;
    if (r[0] < SCREEN_W && r[2] > 0 && r[1] < SCREEN_H && r[3] > 0) return w2m_logged(W2M_API_MonitorFromRect, rect, MONITOR);
    return outside(W2M_API_MonitorFromRect, (u32)flags);
}

int w2m_is_window(u64 window);                          /* windows.c */
int w2m_window_rect(u64 window, s32 r[4]);              /* windows.c */

/* A window's monitor is the one its rectangle meets (review of Sprint 054: every window was on the monitor) */
u64 user32_MonitorFromWindow(u64 window, u64 flags) {
    s32 r[4];
    if (window && w2m_window_rect(window, r)) {
        if (r[0] < SCREEN_W && r[2] > 0 && r[1] < SCREEN_H && r[3] > 0)
            return w2m_logged(W2M_API_MonitorFromWindow, window, MONITOR);
        return outside(W2M_API_MonitorFromWindow, (u32)flags);
    }
    if (window) w2m_unimplemented(W2M_API_MonitorFromWindow, window);           /* not a window: no row */
    return outside(W2M_API_MonitorFromWindow, (u32)flags);
}

/* GetDpiForMonitor(hmonitor, dpiType, dpiX, dpiY): MDT_EFFECTIVE_DPI, MDT_ANGULAR_DPI, MDT_RAW_DPI (rows dpi.*) */
u64 shcore_GetDpiForMonitor(u64 monitor, u64 type, u64 x, u64 y) {
    static const u32 DPI[3] = {120, 123, 141};
    if (monitor != MONITOR) return w2m_logged(W2M_API_GetDpiForMonitor, monitor, 0x80070006u);
    if ((u32)type > 2) return w2m_logged(W2M_API_GetDpiForMonitor, type, 0x800700A0u);
    *(u32 *)x = DPI[(u32)type]; *(u32 *)y = DPI[(u32)type];
    return w2m_logged(W2M_API_GetDpiForMonitor, type, 0);
}

/* The 16 sizes below the screen's, smallest first (rows settings.mode_2 to settings.mode_65) */
static const u16 SIZES[16][2] = {{640, 480}, {800, 600}, {1024, 768}, {1152, 864}, {1280, 600}, {1280, 720},
                                 {1280, 768}, {1280, 800}, {1280, 960}, {1280, 1024}, {1360, 768}, {1366, 768},
                                 {1400, 1050}, {1440, 900}, {1600, 900}, {1680, 1050}};
#define MODE_COUNT 66

/* The adapter's name, \\.\DISPLAY12, compared without regard to ASCII case (as the runners' \\.\DISPLAY1 was, row
 * settings.named_lower.current of runs 38273510163 and 38273513428) */
static int display_one(const u16 *name) {
    static const char want[] = "\\\\.\\DISPLAY12";
    for (int i = 0;; i++) {
        u16 c = name[i];
        if (c >= 'a' && c <= 'z') c = (u16)(c - 'a' + 'A');
        if (c != (u16)want[i]) return 0;
        if (!c) return 1;
    }
}

/* EnumDisplaySettingsW(lpszDeviceName, iModeNum, lpDevMode): DEVMODEW, the fields the rows hold */
u64 user32_EnumDisplaySettingsW(u64 device, u64 mode, u64 out) {
    if (device && !display_one((const u16 *)device)) {
        return w2m_logged(W2M_API_EnumDisplaySettingsW, mode, 0);              /* rows settings.other_device.current */
    }
    s32 m = (s32)mode;
    u8 *d = (u8 *)out;
    u32 width, height, fields, frequency = 144, fixed = 0;
    const char *name;
    if (m == -1 || m == -2) { width = SCREEN_W; height = SCREEN_H; fields = 0x207C00A0u; name = "CDD"; }
    else if (m >= 0 && m < 2) { width = SCREEN_W; height = SCREEN_H; frequency = m ? 60 : 144; fields = 0x7C0080u; name = "cdd"; }
    else if (m >= 2 && m < MODE_COUNT) {
        u32 k = (u32)(m - 2);                         /* size k / 4: 144 Hz twice, then 60 Hz twice, fixed output 0, 1 */
        width = SIZES[k / 4][0]; height = SIZES[k / 4][1];
        frequency = (k & 2) ? 60 : 144;
        fixed = k & 1;
        fields = fixed ? 0x207C0080u : 0x7C0080u;     /* DM_DISPLAYFIXEDOUTPUT with it */
        name = "cdd";
    }
    else if (m >= MODE_COUNT) return w2m_logged(W2M_API_EnumDisplaySettingsW, mode, 0);
    else { w2m_unimplemented(W2M_API_EnumDisplaySettingsW, mode); return 0; }
    for (int i = 0; i < 32; i++) ((u16 *)d)[i] = 0;
    for (int i = 0; name[i]; i++) ((u16 *)d)[i] = (u16)name[i];
    *(u16 *)(d + 68) = 188;                            /* dmSize */
    *(u32 *)(d + 72) = fields;
    *(s32 *)(d + 76) = 0; *(s32 *)(d + 80) = 0;        /* dmPosition */
    *(u32 *)(d + 84) = 0; *(u32 *)(d + 88) = fixed;    /* dmDisplayOrientation, dmDisplayFixedOutput */
    *(u32 *)(d + 168) = 32;                            /* dmBitsPerPel */
    *(u32 *)(d + 172) = width;
    *(u32 *)(d + 176) = height;
    *(u32 *)(d + 180) = 0;                             /* dmDisplayFlags */
    *(u32 *)(d + 184) = frequency;                     /* dmDisplayFrequency */
    return w2m_logged(W2M_API_EnumDisplaySettingsW, mode, 1);
}
