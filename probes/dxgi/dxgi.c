// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* dxgi.c — CreateDXGIFactory and the factory methods the game reaches (Sprint 063, group E).
 *
 * No C runtime. Game run 38206263976 seq 3298-3301: UnityPlayer calls MonitorFromWindow(its window,
 * MONITOR_DEFAULTTONEAREST), LoadLibraryW(L"dxgi.dll"), GetProcAddress(dxgi, "CreateDXGIFactory"),
 * CreateDXGIFactory(IID_IDXGIFactory {7b7166ec-21c7-44ae-b21a-c9ae321ae369}, &factory) -> S_OK, and FreeLibrary(dxgi).
 * Between the last two, UnityPlayer RVA 0x520700 (disassembled 2026-10-11) calls through the vtables, which the
 * capture cannot see: IDXGIFactory::EnumAdapters(i) until it fails; for each adapter IDXGIAdapter::EnumOutputs(j)
 * until it fails, IDXGIOutput::GetDesc, and a comparison of the description's Monitor with the HMONITOR it was given;
 * on a match IDXGIAdapter::GetDesc and its three memory sizes; Release of each object. This probe makes the same
 * calls on the runner (which has no GPU) and records each answer, the strings as text, with the boundary rows a
 * service would accept: another interface, a NULL out pointer, the reference counts, the enumerations' ends.
 */
#include <windows.h>

static const GUID IID_Factory = {0x7b7166ec, 0x21c7, 0x44ae, {0xb2, 0x1a, 0xc9, 0xae, 0x32, 0x1a, 0xe3, 0x69}};
static const GUID IID_Unknown = {0x00000000, 0x0000, 0x0000, {0xC0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x46}};
static const GUID IID_Nothing = {0x12345678, 0x1234, 0x5678, {0x9A, 0xBC, 0xDE, 0xF0, 0x12, 0x34, 0x56, 0x78}};

/* dxgi.h's vtable order: IUnknown, IDXGIObject (SetPrivateData, SetPrivateDataInterface, GetPrivateData, GetParent),
 * then each interface's own */
typedef struct ComObject { void **v; } ComObject;
#define SLOT(p, i) (((ComObject *)(p))->v[i])
typedef HRESULT (WINAPI *query_fn)(void *, const GUID *, void **);
typedef ULONG (WINAPI *ref_fn)(void *);
typedef HRESULT (WINAPI *enum_fn)(void *, UINT, void **);        /* EnumAdapters, EnumOutputs: slot 7 */
typedef HRESULT (WINAPI *desc_fn)(void *, void *);               /* GetDesc: slot 8 of an adapter, 7 of an output */
typedef HRESULT (WINAPI *create_fn)(const GUID *, void **);

typedef struct {
    WCHAR Description[128];
    UINT VendorId, DeviceId, SubSysId, Revision;
    SIZE_T DedicatedVideoMemory, DedicatedSystemMemory, SharedSystemMemory;
    LUID AdapterLuid;
} ADAPTER_DESC;
typedef struct {
    WCHAR DeviceName[32];
    RECT DesktopCoordinates;
    BOOL AttachedToDesktop;
    UINT Rotation;
    HMONITOR Monitor;
} OUTPUT_DESC;

static char out[16384];
static unsigned n;
static void put(const char *s) { while (*s && n < sizeof out - 64) out[n++] = *s++; }
static void putu(unsigned long long v) {
    char t[24];
    int k = 0;
    do { t[k++] = (char)('0' + v % 10); v /= 10; } while (v);
    while (k) out[n++] = t[--k];
}
static void row(const char *s) { put("{\"s\":\""); put(s); put("\""); }
#define NONE 0xFFFFFFFFu
static void rowi(const char *s, unsigned i, const char *rest, unsigned j) {   /* "s<i><rest><j>", j NONE: no j */
    put("{\"s\":\""); put(s); putu(i); put(rest); if (j != NONE) putu(j); put("\"");
}
static void field(const char *k, unsigned long long v) { put(",\""); put(k); put("\":"); putu(v); }
static void text(const char *k, const WCHAR *w, unsigned max) {
    static const char hex[] = "0123456789abcdef";
    put(",\""); put(k); put("\":\"");
    for (unsigned i = 0; i < max && w[i] && n < sizeof out - 80; i++) {
        WCHAR c = w[i];
        if (c >= 0x20 && c < 0x7F && c != '"' && c != '\\') { out[n++] = (char)c; continue; }
        put("\\u"); for (int sh = 12; sh >= 0; sh -= 4) out[n++] = hex[(c >> sh) & 15];
    }
    put("\"");
}
static void end(void) {
    put("}\n");
    DWORD written;
    WriteFile(GetStdHandle(STD_OUTPUT_HANDLE), out, n, &written, NULL);
    n = 0;
}

void start(void) {
    POINT zero = {0, 0};
    HMONITOR primary = MonitorFromPoint(zero, MONITOR_DEFAULTTOPRIMARY);
    HMODULE dxgi = LoadLibraryW(L"dxgi.dll");
    row("load"); field("module", dxgi != NULL); end();
    if (!dxgi) return;
    create_fn create = (create_fn)(void *)GetProcAddress(dxgi, "CreateDXGIFactory");
    row("proc"); field("found", create != NULL); end();
    if (!create) return;

    void *f = (void *)1;
    HRESULT hr = create(&IID_Nothing, &f);
    row("create.unknown_interface"); field("hr", (DWORD)hr); field("out_null", f == NULL);
    field("out_untouched", f == (void *)1); end();
    hr = create(&IID_Factory, NULL);
    row("create.null_out"); field("hr", (DWORD)hr); end();
    f = NULL;
    hr = create(&IID_Factory, &f);
    row("create.factory"); field("hr", (DWORD)hr); field("out_nonnull", f != NULL); end();
    if (!f) return;

    ULONG a = ((ref_fn)SLOT(f, 1))(f);
    ULONG r = ((ref_fn)SLOT(f, 2))(f);
    row("refcount"); field("add_ref", a); field("release", r); end();
    void *u = NULL;
    hr = ((query_fn)SLOT(f, 0))(f, &IID_Unknown, &u);
    row("query.unknown"); field("hr", (DWORD)hr); field("same", u == f); end();
    if (u) ((ref_fn)SLOT(u, 2))(u);
    u = (void *)1;
    hr = ((query_fn)SLOT(f, 0))(f, &IID_Nothing, &u);
    row("query.nothing"); field("hr", (DWORD)hr); field("out_null", u == NULL); end();

    /* the game's walk: every adapter, every output, the output whose Monitor is the window's, that adapter's desc */
    int matched = 0;
    for (UINT i = 0; i < 8; i++) {
        void *adapter = (void *)1;
        hr = ((enum_fn)SLOT(f, 7))(f, i, &adapter);
        if (hr != S_OK) {
            rowi("adapters.end.", i, "", NONE); field("hr", (DWORD)hr); field("out_null", adapter == NULL); end();
            break;
        }
        ADAPTER_DESC ad;
        for (unsigned k = 0; k < sizeof ad; k++) ((char *)&ad)[k] = (char)0xA5;
        HRESULT hd = ((desc_fn)SLOT(adapter, 8))(adapter, &ad);
        rowi("adapter.", i, "", NONE); field("hr", (DWORD)hr); field("desc_hr", (DWORD)hd); text("description", ad.Description, 128);
        field("vendor", ad.VendorId); field("device", ad.DeviceId); field("subsys", ad.SubSysId); field("revision", ad.Revision);
        field("dedicated_video", ad.DedicatedVideoMemory); field("dedicated_system", ad.DedicatedSystemMemory);
        field("shared_system", ad.SharedSystemMemory); field("luid_nonzero", ad.AdapterLuid.LowPart || ad.AdapterLuid.HighPart);
        end();
        for (UINT j = 0; j < 8; j++) {
            void *output = (void *)1;
            HRESULT ho = ((enum_fn)SLOT(adapter, 7))(adapter, j, &output);
            if (ho != S_OK) {
                rowi("adapter.", i, ".outputs.end.", j); field("hr", (DWORD)ho); field("out_null", output == NULL); end();
                break;
            }
            OUTPUT_DESC od;
            for (unsigned k = 0; k < sizeof od; k++) ((char *)&od)[k] = (char)0xA5;
            HRESULT hg = ((desc_fn)SLOT(output, 7))(output, &od);
            rowi("adapter.", i, ".output.", j); field("desc_hr", (DWORD)hg); text("device_name", od.DeviceName, 32);
            field("left", (DWORD)od.DesktopCoordinates.left); field("top", (DWORD)od.DesktopCoordinates.top);
            field("right", (DWORD)od.DesktopCoordinates.right); field("bottom", (DWORD)od.DesktopCoordinates.bottom);
            field("attached", od.AttachedToDesktop != 0); field("rotation", od.Rotation);
            field("monitor_is_primary", od.Monitor == primary); end();
            if (od.Monitor == primary) matched = 1;
            ((ref_fn)SLOT(output, 2))(output);
        }
        ULONG ra = ((ref_fn)SLOT(adapter, 2))(adapter);
        rowi("adapter.", i, ".release", NONE); field("count", ra); end();
    }
    row("game_walk"); field("matched", matched); end();

    ULONG last = ((ref_fn)SLOT(f, 2))(f);
    row("release.last"); field("count", last); end();
    BOOL freed = FreeLibrary(dxgi);
    row("free_library"); field("ret", freed != 0); end();
}
