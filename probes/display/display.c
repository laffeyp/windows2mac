// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* display.c — the display as the game queries it (Sprint 054, family "display"; specs/work_list.json).
 *
 * No C runtime; linked with the game's manifest (PerMonitorV2 DPI awareness changes what the metrics say). The
 * batch trace holds the forms (seq 1531-1540, 1670-1671): GetSystemMetrics 76-79, EnumDisplayDevicesA(NULL, 0),
 * (the adapter's DeviceName, 0), (NULL, 1); EnumDisplayMonitors(NULL, NULL, proc, data) whose callback calls
 * GetMonitorInfoA with a MONITORINFOEXA (cbSize 72); MonitorFromWindow(window, MONITOR_DEFAULTTONEAREST); shcore's
 * GetDpiForMonitor(monitor, MDT_EFFECTIVE_DPI) through LoadLibraryW and GetProcAddress. The play run adds
 * GetSystemMetrics 23 and 94, GetMonitorInfoW, MonitorFromRect and EnumDisplaySettingsW. The rows read every
 * metric 0-99 and each answer whole, strings included.
 */
#include <windows.h>

#define SENTINEL 0xDEADu
#define MDT_EFFECTIVE_DPI 0
typedef HRESULT (WINAPI *dpi_fn)(HMONITOR, int, UINT *, UINT *);

static char out[16384];
static unsigned n;
static void put(const char *s) { while (*s && n < sizeof out - 64) out[n++] = *s++; }
static void puta(const char *s) { while (*s && n < sizeof out - 64) out[n++] = (*s == '"' || *s == '\\') ? '_' : *s, s++; }
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
static void text(const char *k, const char *v) { put(",\""); put(k); put("\":\""); puta(v); put("\""); }
static void wtext(const char *k, const WCHAR *v) { put(",\""); put(k); put("\":\""); putw(v); put("\""); }
static void rect(const char *k, RECT r) {
    char key[32];
    int i = 0;
    for (const char *p = k; *p; p++) key[i++] = *p;
    static const char *const side[] = {"_left", "_top", "_right", "_bottom"};
    long v[4] = {r.left, r.top, r.right, r.bottom};
    for (int j = 0; j < 4; j++) {
        int m = i;
        for (const char *p = side[j]; *p; p++) key[m++] = *p;
        key[m] = 0;
        sfield(key, v[j]);
    }
}
static void end(void) {
    put("}\n");
    DWORD written;
    WriteFile(GetStdHandle(STD_OUTPUT_HANDLE), out, n, &written, NULL);
    n = 0;
}
static void zero(void *p, unsigned size) { for (unsigned i = 0; i < size; i++) ((char *)p)[i] = 0; }
static void named(char *buf, const char *base, unsigned v) {
    int i = 0;
    for (const char *p = base; *p; p++) buf[i++] = *p;
    if (v >= 10) buf[i++] = (char)('0' + v / 10);
    buf[i++] = (char)('0' + v % 10);
    buf[i] = 0;
}

static HMONITOR first_monitor;
static int callbacks;

static BOOL CALLBACK monitor_proc(HMONITOR m, HDC dc, RECT *r, LPARAM data) {
    row("enum_monitors.callback"); field("index", (unsigned)callbacks); field("monitor", (ULONG_PTR)m);
    flag("dc_null", dc == NULL); rect("rect", *r); field("data", (ULONG_PTR)data); end();
    if (!callbacks++) first_monitor = m;
    MONITORINFOEXA info;
    zero(&info, sizeof info);
    info.cbSize = sizeof info;
    SetLastError(SENTINEL);
    BOOL ok = GetMonitorInfoA(m, (MONITORINFO *)&info);
    row("enum_monitors.callback_info"); field("ret", ok != 0); field("err", GetLastError()); field("size", info.cbSize);
    rect("monitor", info.rcMonitor); rect("work", info.rcWork); field("flags", info.dwFlags); text("device", info.szDevice);
    end();
    return TRUE;
}

static void device(const char *name, const char *adapter, DWORD index) {
    DISPLAY_DEVICEA d;
    zero(&d, sizeof d);
    d.cb = sizeof d;
    SetLastError(SENTINEL);
    BOOL ok = EnumDisplayDevicesA(adapter, index, &d, 0);
    row(name); field("ret", ok != 0); field("err", GetLastError());
    if (ok) {
        text("name", d.DeviceName); text("string", d.DeviceString); field("state", d.StateFlags);
        text("id", d.DeviceID); text("key", d.DeviceKey);
    }
    end();
}

static void from_point(const char *name, long x, long y, DWORD how) {
    POINT p = {x, y};
    HMONITOR m = MonitorFromPoint(p, how);
    row(name); flag("null", m == NULL); flag("first", m == first_monitor); end();
}

static void dpi(const char *name, dpi_fn f, HMONITOR m, int type) {
    UINT x = 0xA5A5, y = 0xA5A5;
    HRESULT hr = f(m, type, &x, &y);
    row(name); field("hr", (DWORD)hr); field("x", x); field("y", y); end();
}

static void settings(const char *name, DWORD mode) {
    DEVMODEW dm;
    zero(&dm, sizeof dm);
    dm.dmSize = sizeof dm;
    SetLastError(SENTINEL);
    BOOL ok = EnumDisplaySettingsW(NULL, mode, &dm);
    row(name); field("ret", ok != 0); field("err", GetLastError());
    if (ok) {
        wtext("device", dm.dmDeviceName); field("size", dm.dmSize); field("fields", dm.dmFields);
        field("width", dm.dmPelsWidth); field("height", dm.dmPelsHeight); field("bits", dm.dmBitsPerPel);
        field("frequency", dm.dmDisplayFrequency); field("flags", dm.dmDisplayFlags);
        sfield("x", dm.dmPosition.x); sfield("y", dm.dmPosition.y); field("orientation", dm.dmDisplayOrientation);
        field("fixed_output", dm.dmDisplayFixedOutput);
    }
    end();
}

void start(void) {
    char name[32];
    for (unsigned i = 0; i < 100; i++) {
        named(name, "metric.", i);
        row(name); sfield("value", GetSystemMetrics((int)i)); end();
    }
    row("metric.4096"); sfield("value", GetSystemMetrics(0x1000)); end();       /* SM_REMOTESESSION */
    row("metric.8193"); sfield("value", GetSystemMetrics(0x2001)); end();       /* SM_SHUTTINGDOWN */

    DISPLAY_DEVICEA adapter;
    zero(&adapter, sizeof adapter);
    adapter.cb = sizeof adapter;
    BOOL have = EnumDisplayDevicesA(NULL, 0, &adapter, 0);
    device("devices.adapter_0", NULL, 0);
    if (have) {
        device("devices.adapter_0_monitor_0", adapter.DeviceName, 0);
        device("devices.adapter_0_monitor_1", adapter.DeviceName, 1);
    }
    device("devices.adapter_1", NULL, 1);
    device("devices.adapter_2", NULL, 2);
    device("devices.unknown_adapter", "\\\\.\\DISPLAY9", 0);

    SetLastError(SENTINEL);
    BOOL ok = EnumDisplayMonitors(NULL, NULL, monitor_proc, 0x5A5A);
    row("enum_monitors"); field("ret", ok != 0); field("err", GetLastError()); field("callbacks", (unsigned)callbacks); end();

    MONITORINFOEXW wide;
    zero(&wide, sizeof wide);
    wide.cbSize = sizeof wide;
    ok = GetMonitorInfoW(first_monitor, (MONITORINFO *)&wide);
    row("info.wide"); field("ret", ok != 0); field("size", wide.cbSize); rect("monitor", wide.rcMonitor);
    rect("work", wide.rcWork); field("flags", wide.dwFlags); wtext("device", wide.szDevice); end();
    MONITORINFO plain;
    zero(&plain, sizeof plain);
    plain.cbSize = sizeof plain;
    ok = GetMonitorInfoA(first_monitor, &plain);
    row("info.plain"); field("ret", ok != 0); rect("monitor", plain.rcMonitor); field("flags", plain.dwFlags); end();
    zero(&plain, sizeof plain);
    plain.cbSize = 12;
    SetLastError(SENTINEL);
    ok = GetMonitorInfoA(first_monitor, &plain);
    row("info.bad_size"); field("ret", ok != 0); field("err", GetLastError()); end();
    zero(&plain, sizeof plain);
    plain.cbSize = sizeof plain;
    SetLastError(SENTINEL);
    ok = GetMonitorInfoA((HMONITOR)(ULONG_PTR)0x1234, &plain);
    row("info.not_a_monitor"); field("ret", ok != 0); field("err", GetLastError()); end();

    from_point("from_point.origin_null", 0, 0, MONITOR_DEFAULTTONULL);
    from_point("from_point.far_null", -10000, -10000, MONITOR_DEFAULTTONULL);
    from_point("from_point.far_nearest", -10000, -10000, MONITOR_DEFAULTTONEAREST);
    from_point("from_point.far_primary", 50000, 50000, MONITOR_DEFAULTTOPRIMARY);
    RECT r = {10, 10, 100, 100}, away = {-9000, -9000, -8000, -8000};
    HMONITOR m = MonitorFromRect(&r, MONITOR_DEFAULTTONULL);
    row("from_rect.inside"); flag("null", m == NULL); flag("first", m == first_monitor); end();
    m = MonitorFromRect(&away, MONITOR_DEFAULTTONULL);
    row("from_rect.far_null"); flag("null", m == NULL); end();
    m = MonitorFromRect(&away, MONITOR_DEFAULTTONEAREST);
    row("from_rect.far_nearest"); flag("first", m == first_monitor); end();
    m = MonitorFromWindow(NULL, MONITOR_DEFAULTTONULL);
    row("from_window.null_null"); flag("null", m == NULL); end();
    m = MonitorFromWindow(NULL, MONITOR_DEFAULTTOPRIMARY);
    row("from_window.null_primary"); flag("first", m == first_monitor); end();
    m = MonitorFromWindow(NULL, MONITOR_DEFAULTTONEAREST);
    row("from_window.null_nearest"); flag("null", m == NULL); flag("first", m == first_monitor); end();

    HMODULE shcore = LoadLibraryW(L"shcore.dll");
    dpi_fn f = shcore ? (dpi_fn)GetProcAddress(shcore, "GetDpiForMonitor") : NULL;
    row("dpi.found"); flag("library", shcore != NULL); flag("function", f != NULL); end();
    if (f) {
        dpi("dpi.effective", f, first_monitor, MDT_EFFECTIVE_DPI);
        dpi("dpi.angular", f, first_monitor, 1);
        dpi("dpi.raw", f, first_monitor, 2);
        dpi("dpi.bad_type", f, first_monitor, 3);
        dpi("dpi.not_a_monitor", f, (HMONITOR)(ULONG_PTR)0x1234, MDT_EFFECTIVE_DPI);
    }

    settings("settings.current", ENUM_CURRENT_SETTINGS);
    settings("settings.registry", ENUM_REGISTRY_SETTINGS);
    settings("settings.mode_0", 0);
    DEVMODEW dm;
    unsigned modes = 0;
    for (; modes < 2000; modes++) {
        zero(&dm, sizeof dm);
        dm.dmSize = sizeof dm;
        if (!EnumDisplaySettingsW(NULL, modes, &dm)) break;
    }
    row("settings.mode_count"); field("value", modes); end();
    ExitProcess(0);
}
