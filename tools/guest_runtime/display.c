// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* display.c — the display as the game queries it (Sprint 054, family "display"; specs/work_list.json).
 *
 * contracts/windows.display.display records Windows (Server 2025 and 2022) under the game's manifest
 * (PerMonitorV2): one monitor of 1024 x 768 at 96 DPI on the Hyper-V video adapter, the runners' display. w2m's
 * display is that one, with Server 2025's taskbar (the work area ends at 720; 2022's at 728), until the game's
 * window is a Mac window (phase 6; BLACKBOARD Decision 2026-10-11):
 *   - GetSystemMetrics: Server 2025's value for each index 0-99 (17, SM_CYFULLSCREEN, is the work area's height less
 *     the caption, 23; 62, SM_CYMAXIMIZED, the work area's height and 16); index 93 has no SM_ name and answers a
 *     value that differs from run to run (5852, 6552, 7696), here 0; SM_REMOTESESSION and SM_SHUTTINGDOWN 0
 *   - EnumDisplayDevicesA: adapter 0 is \\.\DISPLAY1, "Microsoft Hyper-V Video", state 5, with one monitor,
 *     \\.\DISPLAY1\Monitor0, "Generic PnP Monitor", state 3; no adapter 1 and no other monitor (FALSE, the last
 *     error left); the adapter's registry key carries a GUID that differs by machine (Server 2025's here)
 *   - EnumDisplayMonitors(NULL, NULL) calls back once with monitor 0x10001, a NULL DC, the screen and the data;
 *     GetMonitorInfoA and W fill MONITORINFO (40 bytes) or MONITORINFOEX (72, 104) with the screen, the work area,
 *     MONITORINFOF_PRIMARY and \\.\DISPLAY1; another size is ERROR_INVALID_PARAMETER, another monitor
 *     ERROR_INVALID_MONITOR_HANDLE (1461)
 *   - MonitorFrom{Point,Rect,Window}: the monitor for a point or rectangle on it, else NULL for
 *     MONITOR_DEFAULTTONULL and the monitor for the others; a NULL window as a point off every monitor
 *   - shcore's GetDpiForMonitor: 96 x 96 for MDT_EFFECTIVE_DPI and MDT_ANGULAR_DPI; MDT_RAW_DPI answers
 *     HRESULT_FROM_WIN32(ERROR_NOT_SUPPORTED) with 0 x 0 (the Hyper-V monitor reports no size); another type
 *     E_INVALIDARG's WIN32 form (0x800700A0), another monitor 0x80070006, both leaving x and y
 *   - EnumDisplaySettingsW(NULL): the current and registry settings 1024 x 768 x 32 at 64 Hz as "CDD", and 13 modes
 *     from 640 x 480 to 1920 x 1080 as "cdd"; dmSize answers 188
 * Other forms (a DC or clip rectangle, EDD_GET_DEVICE_INTERFACE_NAME, a callback that stops the enumeration, a named
 * device for the settings) are not in the rows and stop the run.
 */
#include "runtime.h"

typedef unsigned short u16;

#define MONITOR 0x10001ull
#define SCREEN_W 1024
#define SCREEN_H 768
#define WORK_H 720
#define ERROR_INVALID_PARAMETER 87u
#define ERROR_INVALID_MONITOR_HANDLE 1461u
#define MONITOR_DEFAULTTONULL 0u

/* Server 2025's GetSystemMetrics(0..99) (row metric.N, run 38174316653), 93 excepted */
static const s32 METRICS[100] = {
    1024, 768, 17, 17, 23, 1, 1, 3, 3, 17, 17, 32, 32, 32, 32, 20, 1024, WORK_H - 23, 0, 1,
    17, 17, 0, 0, 0, 0, 0, 0, 136, 39, 36, 22, 4, 4, 136, 39, 4, 4, 75, 75,
    0, 0, 0, 5, 0, 2, 2, 160, 28, 16, 16, 23, 22, 22, 19, 19, 8, 160, 28, 1044,
    788, 1040, WORK_H + 16, 3, 0, 0, 0, 0, 4, 4, 0, 15, 15, 0, 0, 1, 0, 0, 1024, 768,
    1, 1, 1, 1, 1, 4, 0, 0, 0, 0, 0, 0, 4, 0, 0, 0, 0, 0, 0, 0};

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
        copy(d + 4, "\\\\.\\DISPLAY1", 32);
        copy(d + 36, "Microsoft Hyper-V Video", 128);
        *(u32 *)(d + 164) = 5;                     /* DISPLAY_DEVICE_ATTACHED_TO_DESKTOP | PRIMARY_DEVICE */
        copy(d + 168, "VMBUS\\{da0a7802-e377-4aac-8e77-0558eb1073f8}", 128);
        copy(d + 296, "\\Registry\\Machine\\System\\CurrentControlSet\\Control\\Video\\"
                      "{36FB1401-A953-11F1-85AF-806E6F6E6963}\\0000", 128);
        return w2m_logged(W2M_API_EnumDisplayDevicesA, index, 1);
    }
    if (dev && same(dev, "\\\\.\\DISPLAY1") && (u32)index == 0) {
        copy(d + 4, "\\\\.\\DISPLAY1\\Monitor0", 32);
        copy(d + 36, "Generic PnP Monitor", 128);
        *(u32 *)(d + 164) = 3;                     /* DISPLAY_DEVICE_ACTIVE | ATTACHED */
        copy(d + 168, "MONITOR\\MSH062E\\{4d36e96e-e325-11ce-bfc1-08002be10318}\\0000", 128);
        copy(d + 296, "\\Registry\\Machine\\System\\CurrentControlSet\\Control\\Class\\"
                      "{4d36e96e-e325-11ce-bfc1-08002be10318}\\0000", 128);
        return w2m_logged(W2M_API_EnumDisplayDevicesA, index, 1);
    }
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

static const u16 DEVICE_W[] = {'\\', '\\', '.', '\\', 'D', 'I', 'S', 'P', 'L', 'A', 'Y', '1', 0};

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
        if (wide) for (int i = 0; i < 32; i++) ((u16 *)(m + 40))[i] = i < 13 ? DEVICE_W[i] : 0;
        else copy((char *)(m + 40), "\\\\.\\DISPLAY1", 32);
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

u64 user32_MonitorFromWindow(u64 window, u64 flags) {
    if (window && w2m_is_window(window)) return w2m_logged(W2M_API_MonitorFromWindow, window, MONITOR);
    if (window) w2m_unimplemented(W2M_API_MonitorFromWindow, window);           /* not a window: no row */
    return outside(W2M_API_MonitorFromWindow, (u32)flags);
}

/* GetDpiForMonitor(hmonitor, dpiType, dpiX, dpiY) */
u64 shcore_GetDpiForMonitor(u64 monitor, u64 type, u64 x, u64 y) {
    if (monitor != MONITOR) return w2m_logged(W2M_API_GetDpiForMonitor, monitor, 0x80070006u);
    if ((u32)type > 2) return w2m_logged(W2M_API_GetDpiForMonitor, type, 0x800700A0u);
    if ((u32)type == 2) {
        *(u32 *)x = 0; *(u32 *)y = 0;
        return w2m_logged(W2M_API_GetDpiForMonitor, type, 0x80070032u);
    }
    *(u32 *)x = 96; *(u32 *)y = 96;
    return w2m_logged(W2M_API_GetDpiForMonitor, type, 0);
}

static const u16 MODES[13][2] = {{640, 480}, {800, 600}, {1024, 768}, {1152, 864}, {1280, 720}, {1280, 800},
                                 {1280, 1024}, {1366, 768}, {1440, 900}, {1600, 900}, {1600, 1200}, {1680, 1050},
                                 {1920, 1080}};

/* EnumDisplaySettingsW(lpszDeviceName, iModeNum, lpDevMode): DEVMODEW, the fields the rows hold */
u64 user32_EnumDisplaySettingsW(u64 device, u64 mode, u64 out) {
    if (device) w2m_unimplemented(W2M_API_EnumDisplaySettingsW, device);         /* a named device: no row */
    s32 m = (s32)mode;
    u8 *d = (u8 *)out;
    u32 width, height, fields;
    const char *name;
    if (m == -1 || m == -2) { width = SCREEN_W; height = SCREEN_H; fields = 0x207C00A0u; name = "CDD"; }
    else if (m >= 0 && m < 13) { width = MODES[m][0]; height = MODES[m][1]; fields = 0x7C0080u; name = "cdd"; }
    else if (m >= 13) return w2m_logged(W2M_API_EnumDisplaySettingsW, mode, 0);
    else { w2m_unimplemented(W2M_API_EnumDisplaySettingsW, mode); return 0; }
    for (int i = 0; i < 32; i++) ((u16 *)d)[i] = 0;
    for (int i = 0; name[i]; i++) ((u16 *)d)[i] = (u16)name[i];
    *(u16 *)(d + 68) = 188;                            /* dmSize */
    *(u32 *)(d + 72) = fields;
    *(s32 *)(d + 76) = 0; *(s32 *)(d + 80) = 0;        /* dmPosition */
    *(u32 *)(d + 84) = 0; *(u32 *)(d + 88) = 0;        /* dmDisplayOrientation, dmDisplayFixedOutput */
    *(u32 *)(d + 168) = 32;                            /* dmBitsPerPel */
    *(u32 *)(d + 172) = width;
    *(u32 *)(d + 176) = height;
    *(u32 *)(d + 180) = 0;                             /* dmDisplayFlags */
    *(u32 *)(d + 184) = 64;                            /* dmDisplayFrequency */
    return w2m_logged(W2M_API_EnumDisplaySettingsW, mode, 1);
}
