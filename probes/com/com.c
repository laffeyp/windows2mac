// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* com.c — COM apartments, the audio device enumerator and the small COM helpers (Sprint 052, family "com";
 * specs/work_list.json).
 *
 * No C runtime. The captures (store/game-runs/38113342549, 38113343972) hold the forms: mono's thread calls
 * CoInitializeEx(NULL, COINIT_MULTITHREADED); UnityPlayer calls CoInitialize(NULL), and six times
 * CoCreateInstance(CLSID_MMDeviceEnumerator, NULL, CLSCTX_ALL or CLSCTX_INPROC_SERVER, IID_IMMDeviceEnumerator, &p),
 * then the enumerator's methods (vtable calls the capture cannot see), PropVariantClear, CoUninitialize;
 * CoCreateGuid once. Rows: each call's HRESULT (and the last error after SetLastError(0xDEAD) where a row has it),
 * the enumerator's answers on the runner, reference counts, CoCreateGuid's version and variant bits, BSTR lengths.
 * The GUIDs are written here (no uuid library).
 */
#include <windows.h>

#define SENTINEL 0xDEADu

static const GUID CLSID_Enumerator = {0xBCDE0395, 0xE52F, 0x467C, {0x8E, 0x3D, 0xC4, 0x57, 0x92, 0x91, 0x69, 0x2E}};
static const GUID IID_Enumerator = {0xA95664D2, 0x9614, 0x4F35, {0xA7, 0x46, 0xDE, 0x8D, 0xB6, 0x36, 0x17, 0xE6}};
static const GUID IID_Unknown = {0x00000000, 0x0000, 0x0000, {0xC0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x46}};
static const GUID IID_Nothing = {0x12345678, 0x1234, 0x5678, {0x9A, 0xBC, 0xDE, 0xF0, 0x12, 0x34, 0x56, 0x78}};
static const GUID CLSID_Nothing = {0x87654321, 0x4321, 0x8765, {0x9A, 0xBC, 0xDE, 0xF0, 0x12, 0x34, 0x56, 0x78}};
/* PKEY_Device_FriendlyName: {a45c254e-df1c-4efd-8020-67d146a850e0}, 14 */
static const struct { GUID fmtid; DWORD pid; } PKEY_FriendlyName =
    {{0xa45c254e, 0xdf1c, 0x4efd, {0x80, 0x20, 0x67, 0xd1, 0x46, 0xa8, 0x50, 0xe0}}, 14};

/* the interfaces' vtables, as far as the probe calls them (mmdeviceapi.h, propsys.h order) */
typedef struct ComObject { struct UnknownVtbl *v; } ComObject;
struct UnknownVtbl {
    HRESULT (WINAPI *QueryInterface)(void *, const GUID *, void **);
    ULONG (WINAPI *AddRef)(void *);
    ULONG (WINAPI *Release)(void *);
};
struct EnumeratorVtbl {
    struct UnknownVtbl unknown;
    HRESULT (WINAPI *EnumAudioEndpoints)(void *, int flow, DWORD mask, void **collection);
    HRESULT (WINAPI *GetDefaultAudioEndpoint)(void *, int flow, int role, void **device);
    HRESULT (WINAPI *GetDevice)(void *, LPCWSTR, void **);
    HRESULT (WINAPI *RegisterEndpointNotificationCallback)(void *, void *);
    HRESULT (WINAPI *UnregisterEndpointNotificationCallback)(void *, void *);
};
struct CollectionVtbl {
    struct UnknownVtbl unknown;
    HRESULT (WINAPI *GetCount)(void *, UINT *);
    HRESULT (WINAPI *Item)(void *, UINT, void **);
};
struct DeviceVtbl {
    struct UnknownVtbl unknown;
    HRESULT (WINAPI *Activate)(void *, const GUID *, DWORD, void *, void **);
    HRESULT (WINAPI *OpenPropertyStore)(void *, DWORD, void **);
    HRESULT (WINAPI *GetId)(void *, LPWSTR *);
    HRESULT (WINAPI *GetState)(void *, DWORD *);
};
struct StoreVtbl {
    struct UnknownVtbl unknown;
    HRESULT (WINAPI *GetCount)(void *, DWORD *);
    HRESULT (WINAPI *GetAt)(void *, DWORD, void *);
    HRESULT (WINAPI *GetValue)(void *, const void *key, PROPVARIANT *);
};
#define VT(p, T) ((struct T *)((ComObject *)(p))->v)
#define DEVICE_STATE_ACTIVE 0x1u        /* mmdeviceapi.h */
#define DEVICE_STATEMASK_ALL 0xFu

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
static void field(const char *k, unsigned long long v) { put(",\""); put(k); put("\":"); putu(v); }
static void hr_field(HRESULT hr) { field("hr", (DWORD)hr); }
/* Each row is written when it ends, so a fault leaves the rows before it. */
static void end(void) {
    put("}\n");
    DWORD written;
    WriteFile(GetStdHandle(STD_OUTPUT_HANDLE), out, n, &written, NULL);
    n = 0;
}

static void endpoints(const char *name, void *e, int flow, DWORD mask) {
    void *c = NULL;
    HRESULT hr = VT(e, EnumeratorVtbl)->EnumAudioEndpoints(e, flow, mask, &c);
    UINT count = 0xA5A5;
    HRESULT hc = c ? VT(c, CollectionVtbl)->GetCount(c, &count) : 0;
    row(name); hr_field(hr); field("collection", c != NULL); field("count_hr", (DWORD)hc); field("count", count); end();
    if (c) VT(c, UnknownVtbl)->Release(c);
}

static void default_endpoint(const char *name, void *e, int flow, int role) {
    void *d = (void *)1;
    HRESULT hr = VT(e, EnumeratorVtbl)->GetDefaultAudioEndpoint(e, flow, role, &d);
    row(name); hr_field(hr); field("device_null", d == NULL); field("device_untouched", d == (void *)1);
    if (hr == S_OK && d) {
        DWORD state = 0;
        VT(d, DeviceVtbl)->GetState(d, &state);
        field("state", state);
        void *ps = NULL;
        HRESULT hp = VT(d, DeviceVtbl)->OpenPropertyStore(d, STGM_READ, &ps);
        field("store_hr", (DWORD)hp);
        if (ps) {
            PROPVARIANT pv;
            for (unsigned i = 0; i < sizeof pv; i++) ((char *)&pv)[i] = 0;
            HRESULT hv = VT(ps, StoreVtbl)->GetValue(ps, &PKEY_FriendlyName, &pv);
            field("value_hr", (DWORD)hv); field("vt", pv.vt);
            field("clear_hr", (DWORD)PropVariantClear(&pv)); field("vt_after", pv.vt);
            VT(ps, UnknownVtbl)->Release(ps);
        }
        VT(d, UnknownVtbl)->Release(d);
    }
    end();
}

/* a notification client (IMMNotificationClient: IUnknown and five notifications), counting its references */
static volatile LONG client_refs = 1;
static HRESULT WINAPI client_query(void *self, const GUID *iid, void **out) { (void)iid; *out = self; client_refs++; return S_OK; }
static ULONG WINAPI client_add_ref(void *self) { (void)self; return (ULONG)++client_refs; }
static ULONG WINAPI client_release(void *self) { (void)self; return (ULONG)--client_refs; }
static HRESULT WINAPI client_notify(void *self) { (void)self; return S_OK; }
static void *client_vtbl[8] = {(void *)client_query, (void *)client_add_ref, (void *)client_release, (void *)client_notify,
                               (void *)client_notify, (void *)client_notify, (void *)client_notify, (void *)client_notify};
static struct { void **v; } client = {client_vtbl};

static volatile HRESULT worker_hr;
static DWORD WINAPI worker(LPVOID p) {
    (void)p;
    void *e = NULL;
    worker_hr = CoCreateInstance(&CLSID_Enumerator, NULL, CLSCTX_ALL, &IID_Enumerator, &e);
    if (e) VT(e, UnknownVtbl)->Release(e);
    return 0;
}

void start(void) {
    void *e = NULL;
    HRESULT hr = CoCreateInstance(&CLSID_Enumerator, NULL, CLSCTX_ALL, &IID_Enumerator, &e);
    row("create.before_init"); hr_field(hr); field("out_null", e == NULL); end();

    hr = CoInitializeEx(NULL, COINIT_MULTITHREADED);
    row("init.mta"); hr_field(hr); end();
    hr = CoInitializeEx(NULL, COINIT_MULTITHREADED);
    row("init.mta_again"); hr_field(hr); end();
    hr = CoInitialize(NULL);
    row("init.sta_after_mta"); hr_field(hr); end();

    /* another thread, no apartment of its own, the process's MTA present */
    HANDLE h = CreateThread(NULL, 0, worker, NULL, 0, NULL);
    WaitForSingleObject(h, INFINITE);
    CloseHandle(h);
    row("create.implicit_mta"); hr_field(worker_hr); end();

    hr = CoCreateInstance(&CLSID_Enumerator, NULL, CLSCTX_ALL, &IID_Enumerator, &e);
    row("create.enumerator"); hr_field(hr); field("out_nonnull", e != NULL); end();
    void *e2 = NULL;
    hr = CoCreateInstance(&CLSID_Enumerator, NULL, CLSCTX_INPROC_SERVER, &IID_Enumerator, &e2);
    row("create.inproc"); hr_field(hr); field("out_nonnull", e2 != NULL); field("same_object", e2 == e); end();
    if (e2) VT(e2, UnknownVtbl)->Release(e2);
    void *x = (void *)1;
    hr = CoCreateInstance(&CLSID_Nothing, NULL, CLSCTX_ALL, &IID_Unknown, &x);
    row("create.unknown_class"); hr_field(hr); field("out_null", x == NULL); end();
    hr = CoCreateInstance(&CLSID_Enumerator, NULL, CLSCTX_ALL, &IID_Enumerator, NULL);
    row("create.null_out"); hr_field(hr); end();
    x = (void *)1;
    hr = CoCreateInstance(&CLSID_Enumerator, NULL, CLSCTX_ALL, &IID_Nothing, &x);
    row("create.unknown_interface"); hr_field(hr); field("out_null", x == NULL); end();

    if (e) {
        ULONG a = VT(e, UnknownVtbl)->AddRef(e);
        ULONG r = VT(e, UnknownVtbl)->Release(e);
        row("refcount"); field("add_ref", a); field("release", r); end();
        void *u = NULL;
        hr = VT(e, UnknownVtbl)->QueryInterface(e, &IID_Unknown, &u);
        row("query.unknown"); hr_field(hr); field("same", u == e); end();
        if (u) VT(u, UnknownVtbl)->Release(u);
        u = (void *)1;
        hr = VT(e, UnknownVtbl)->QueryInterface(e, &IID_Nothing, &u);
        row("query.nothing"); hr_field(hr); field("out_null", u == NULL); end();
        endpoints("endpoints.render_active", e, 0, DEVICE_STATE_ACTIVE);
        endpoints("endpoints.capture_active", e, 1, DEVICE_STATE_ACTIVE);
        endpoints("endpoints.render_all", e, 0, DEVICE_STATEMASK_ALL);
        default_endpoint("default.render_console", e, 0, 0);
        default_endpoint("default.render_multimedia", e, 0, 1);
        default_endpoint("default.capture_console", e, 1, 0);
        hr = VT(e, EnumeratorVtbl)->RegisterEndpointNotificationCallback(e, &client);
        LONG refs_registered = client_refs;
        HRESULT hu = VT(e, EnumeratorVtbl)->UnregisterEndpointNotificationCallback(e, &client);
        row("notification"); field("register_hr", (DWORD)hr); field("refs_registered", (DWORD)refs_registered);
        field("unregister_hr", (DWORD)hu); field("refs_after", (DWORD)client_refs); end();
        hu = VT(e, EnumeratorVtbl)->UnregisterEndpointNotificationCallback(e, &client);
        row("notification.unregister_again"); field("hr", (DWORD)hu); end();
        ULONG last = VT(e, UnknownVtbl)->Release(e);
        row("release.last"); field("count", last); end();
    }

    PROPVARIANT pv;
    for (unsigned i = 0; i < sizeof pv; i++) ((char *)&pv)[i] = 0;
    hr = PropVariantClear(&pv);
    row("propvariant.clear_empty"); hr_field(hr); field("vt", pv.vt); end();

    GUID g1, g2;
    HRESULT h1 = CoCreateGuid(&g1), h2 = CoCreateGuid(&g2);
    int differ = 0;
    for (unsigned i = 0; i < sizeof g1; i++) if (((char *)&g1)[i] != ((char *)&g2)[i]) differ = 1;
    row("guid"); field("hr1", (DWORD)h1); field("hr2", (DWORD)h2); field("version", g1.Data3 >> 12);
    field("variant", g1.Data4[0] >> 6); field("differ", differ); end();

    BSTR b = SysAllocString(L"abc");
    row("bstr"); field("nonnull", b != NULL); field("len", SysStringLen(b)); field("byte_len", ((DWORD *)b)[-1]);
    field("nul", b[3] == 0); end();
    VARIANT v;
    VariantInit(&v);
    v.vt = VT_BSTR;
    v.bstrVal = b;
    hr = VariantClear(&v);
    row("variant.clear_bstr"); hr_field(hr); field("vt", v.vt); end();
    BSTR empty = SysAllocString(L"");
    row("bstr.empty"); field("nonnull", empty != NULL); field("len", SysStringLen(empty)); end();
    SysFreeString(empty);
    SysFreeString(NULL);
    row("bstr.free_null"); field("ok", 1); end();

    CoUninitialize();
    CoUninitialize();
    e = (void *)1;
    hr = CoCreateInstance(&CLSID_Enumerator, NULL, CLSCTX_ALL, &IID_Enumerator, &e);
    row("create.after_uninit"); hr_field(hr); field("out_null", e == NULL); end();
    ExitProcess(0);
}
