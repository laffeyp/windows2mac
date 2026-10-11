// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* com.c — COM apartments, CoCreateInstance of the audio device enumerator, and the small COM helpers (Sprint 052,
 * family "com"; specs/work_list.json).
 *
 * contracts/windows.com.com records Windows (Server 2025 and 2022, runs 38126872187, 38126873913; equal):
 *   - CoInitializeEx: S_OK, then S_FALSE in the same apartment, RPC_E_CHANGED_MODE for the other; CoUninitialize
 *     balances them; a thread with no apartment of its own uses the process's multithreaded one if any thread holds
 *     it, and with none, CoCreateInstance answers CO_E_NOTINITIALIZED with *ppv NULL
 *   - CoCreateInstance: the audio device enumerator (CLSID_MMDeviceEnumerator) is one object for the process, which
 *     COM keeps one reference to; an unknown class is REGDB_E_CLASSNOTREG, an interface the object lacks
 *     E_NOINTERFACE, both with *ppv NULL; a NULL ppv is E_INVALIDARG
 *   - the runners have no audio endpoint: every collection is empty and GetDefaultAudioEndpoint answers E_NOTFOUND
 *     with the device NULL; a notification client registers with S_OK and no AddRef, unregisters with S_OK, and a
 *     second unregister is E_NOTFOUND
 *   - PropVariantClear empties; CoCreateGuid gives version 4, variant 10; a BSTR holds its byte length in the four
 *     bytes before it and a NUL after; VariantClear frees a BSTR and empties the VARIANT; SysFreeString(NULL) is a no-op
 * The objects' methods are services; the host fills each class's vtable below with their thunks at the start
 * (vm_backend.py _com_vtables): the vtables live in the runtime image, so no guest allocation moves.
 */
#include "runtime.h"

#define S_OK 0u
#define S_FALSE 1u
#define E_NOINTERFACE 0x80004002u
#define E_INVALIDARG 0x80070057u
#define E_NOTFOUND 0x80070490u
#define REGDB_E_CLASSNOTREG 0x80040154u
#define CO_E_NOTINITIALIZED 0x800401F0u
#define RPC_E_CHANGED_MODE 0x80010106u
#define COINIT_APARTMENTTHREADED 0x2u
#define MTA 1u
#define STA 2u
#define CLSCTX_INPROC_SERVER 0x1u
#define CLSCTX_ALL 0x17u
#define VT_EMPTY 0u
#define VT_BSTR 8u
#define VT_LPWSTR 31u

/* IMMDeviceEnumerator's eight methods and IMMDeviceCollection's five, in vtable order (mmdeviceapi.h) */
volatile u64 w2m_enumerator_vtable[8];
volatile u64 w2m_collection_vtable[5];

static u64 process_mta;                         /* threads holding the multithreaded apartment */

typedef struct { u32 a; unsigned short b, c; u8 d[8]; } guid;
static const guid CLSID_MMDeviceEnumerator = {0xBCDE0395, 0xE52F, 0x467C, {0x8E, 0x3D, 0xC4, 0x57, 0x92, 0x91, 0x69, 0x2E}};
static const guid IID_IMMDeviceEnumerator = {0xA95664D2, 0x9614, 0x4F35, {0xA7, 0x46, 0xDE, 0x8D, 0xB6, 0x36, 0x17, 0xE6}};
static const guid IID_IMMDeviceCollection = {0x0BD7A1BE, 0x7A1A, 0x44DB, {0x83, 0x97, 0xCC, 0x53, 0x92, 0x38, 0x7B, 0x5E}};
static const guid IID_IUnknown = {0x00000000, 0x0000, 0x0000, {0xC0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x46}};

static int same(const guid *x, const guid *y) {
    const u8 *p = (const u8 *)x, *q = (const u8 *)y;
    for (int i = 0; i < 16; i++) if (p[i] != q[i]) return 0;
    return 1;
}

static u64 init(u64 api, u32 model) {
    struct w2m_thread_scratch *sc = w2m_thread_scratch();
    if (sc->com_inits) {
        if (sc->com_model != model) return w2m_logged(api, model, RPC_E_CHANGED_MODE);
        sc->com_inits++;
        return w2m_logged(api, model, S_FALSE);
    }
    sc->com_inits = 1;
    sc->com_model = model;
    if (model == MTA) process_mta++;
    return w2m_logged(api, model, S_OK);
}

/* CoInitializeEx(pvReserved, dwCoInit) */
u64 ole32_CoInitializeEx(u64 reserved, u64 flags) {
    if (reserved || ((u32)flags & ~(COINIT_APARTMENTTHREADED | 0x4u | 0x8u)))      /* DISABLE_OLE1DDE, SPEED_OVER_MEMORY */
        w2m_unimplemented(W2M_API_CoInitializeEx, flags);
    return init(W2M_API_CoInitializeEx, ((u32)flags & COINIT_APARTMENTTHREADED) ? STA : MTA);
}

u64 ole32_CoInitialize(u64 reserved) {
    if (reserved) w2m_unimplemented(W2M_API_CoInitialize, reserved);
    return init(W2M_API_CoInitialize, STA);
}

W2M_VOID u64 ole32_CoUninitialize(void) {
    struct w2m_thread_scratch *sc = w2m_thread_scratch();
    if (sc->com_inits && --sc->com_inits == 0 && sc->com_model == MTA) process_mta--;
    return w2m_logged(W2M_API_CoUninitialize, sc->com_inits, 0);
}

/* combase's RoInitialize(initType) (Sprint 055; contracts/windows.winrt.winrt): the COM apartment by another name,
 * RO_INIT_SINGLETHREADED (0) an STA, RO_INIT_MULTITHREADED (1) the MTA, S_OK then S_FALSE, RPC_E_CHANGED_MODE for the
 * other model; another type E_INVALIDARG. RoUninitialize balances it as CoUninitialize does. */
u64 combase_RoInitialize(u64 type) {
    if ((u32)type > 1) return w2m_logged(W2M_API_RoInitialize, type, E_INVALIDARG);
    return init(W2M_API_RoInitialize, (u32)type ? MTA : STA);
}

W2M_VOID u64 combase_RoUninitialize(void) {
    struct w2m_thread_scratch *sc = w2m_thread_scratch();
    if (sc->com_inits && --sc->com_inits == 0 && sc->com_model == MTA) process_mta--;
    return w2m_logged(W2M_API_RoUninitialize, sc->com_inits, 0);
}

/* The objects: a vtable, a reference count, and for the enumerator its registered notification clients. */
struct object { u64 vtable; s64 refs; };
static struct object enumerator;                /* one for the process (rows create.inproc: the same object) */
#define CLIENTS 16
static u64 clients[CLIENTS];

/* CoCreateInstance(rclsid, pUnkOuter, dwClsContext, riid, ppv) */
u64 ole32_CoCreateInstance(u64 clsid, u64 outer, u64 context, u64 iid, u64 rsp) {
    u64 out = *(u64 *)(rsp + 0x28);
    context = (u32)context;
    if (outer || (context != CLSCTX_ALL && context != CLSCTX_INPROC_SERVER))
        w2m_unimplemented(W2M_API_CoCreateInstance, outer ? outer : context);       /* not in the rows */
    if (!out) return w2m_logged(W2M_API_CoCreateInstance, clsid, E_INVALIDARG);
    *(u64 *)out = 0;
    if (!w2m_thread_scratch()->com_inits && !process_mta)
        return w2m_logged(W2M_API_CoCreateInstance, clsid, CO_E_NOTINITIALIZED);
    if (!same((const guid *)clsid, &CLSID_MMDeviceEnumerator))
        return w2m_logged(W2M_API_CoCreateInstance, clsid, REGDB_E_CLASSNOTREG);
    if (!same((const guid *)iid, &IID_IMMDeviceEnumerator) && !same((const guid *)iid, &IID_IUnknown))
        return w2m_logged(W2M_API_CoCreateInstance, clsid, E_NOINTERFACE);
    if (!enumerator.vtable) {
        enumerator.vtable = (u64)w2m_enumerator_vtable;
        enumerator.refs = 1;                    /* COM's own reference (rows refcount, release.last) */
    }
    enumerator.refs++;
    *(u64 *)out = (u64)&enumerator;
    return w2m_logged(W2M_API_CoCreateInstance, clsid, S_OK);
}

static u64 query(u64 api, struct object *o, u64 iid, u64 out, const guid *own) {
    if (same((const guid *)iid, own) || same((const guid *)iid, &IID_IUnknown)) {
        o->refs++;
        *(u64 *)out = (u64)o;
        return w2m_logged(api, iid, S_OK);
    }
    *(u64 *)out = 0;
    return w2m_logged(api, iid, E_NOINTERFACE);
}

/* IMMDeviceEnumerator */
u64 mmdevapi_W2mEnumeratorQueryInterface(u64 self, u64 iid, u64 out) {
    return query(W2M_API_W2mEnumeratorQueryInterface, (struct object *)self, iid, out, &IID_IMMDeviceEnumerator);
}
u64 mmdevapi_W2mEnumeratorAddRef(u64 self) {
    return w2m_logged(W2M_API_W2mEnumeratorAddRef, self, (u32)++((struct object *)self)->refs);
}
u64 mmdevapi_W2mEnumeratorRelease(u64 self) {
    struct object *o = (struct object *)self;
    if (o->refs > 1) o->refs--;                  /* COM's own reference stays */
    return w2m_logged(W2M_API_W2mEnumeratorRelease, self, (u32)o->refs);
}

/* EnumAudioEndpoints(dataFlow, dwStateMask, ppDevices): an empty collection, a new object each time */
u64 mmdevapi_W2mEnumeratorEnumAudioEndpoints(u64 self, u64 flow, u64 mask, u64 out) {
    (void)self;
    if ((u32)flow > 2 || ((u32)mask & ~0xFu) || !(u32)mask) w2m_unimplemented(W2M_API_W2mEnumeratorEnumAudioEndpoints, mask);
    struct object *c = (struct object *)w2m_heap_take(sizeof(struct object));
    if (!c) w2m_unimplemented(W2M_API_W2mEnumeratorEnumAudioEndpoints, 0);
    c->vtable = (u64)w2m_collection_vtable;
    c->refs = 1;
    *(u64 *)out = (u64)c;
    return w2m_logged(W2M_API_W2mEnumeratorEnumAudioEndpoints, flow, S_OK);
}

/* GetDefaultAudioEndpoint(dataFlow, role, ppEndpoint): no endpoint (rows default.*) */
u64 mmdevapi_W2mEnumeratorGetDefaultAudioEndpoint(u64 self, u64 flow, u64 role, u64 out) {
    (void)self;
    if ((u32)flow > 1 || (u32)role > 2) w2m_unimplemented(W2M_API_W2mEnumeratorGetDefaultAudioEndpoint, flow);
    *(u64 *)out = 0;
    return w2m_logged(W2M_API_W2mEnumeratorGetDefaultAudioEndpoint, flow, E_NOTFOUND);
}

u64 mmdevapi_W2mEnumeratorGetDevice(u64 self, u64 id, u64 out) {
    (void)self; (void)out;
    w2m_unimplemented(W2M_API_W2mEnumeratorGetDevice, id);                         /* no device has an id */
    return 0;
}

u64 mmdevapi_W2mEnumeratorRegisterEndpointNotificationCallback(u64 self, u64 client) {
    (void)self;
    for (int i = 0; i < CLIENTS; i++)
        if (!clients[i]) {
            clients[i] = client;                /* no AddRef (row notification) */
            return w2m_logged(W2M_API_W2mEnumeratorRegisterEndpointNotificationCallback, client, S_OK);
        }
    w2m_unimplemented(W2M_API_W2mEnumeratorRegisterEndpointNotificationCallback, CLIENTS);
    return 0;
}

u64 mmdevapi_W2mEnumeratorUnregisterEndpointNotificationCallback(u64 self, u64 client) {
    (void)self;
    for (int i = 0; i < CLIENTS; i++)
        if (clients[i] == client) {
            clients[i] = 0;
            return w2m_logged(W2M_API_W2mEnumeratorUnregisterEndpointNotificationCallback, client, S_OK);
        }
    return w2m_logged(W2M_API_W2mEnumeratorUnregisterEndpointNotificationCallback, client, E_NOTFOUND);
}

/* IMMDeviceCollection: always empty */
u64 mmdevapi_W2mCollectionQueryInterface(u64 self, u64 iid, u64 out) {
    return query(W2M_API_W2mCollectionQueryInterface, (struct object *)self, iid, out, &IID_IMMDeviceCollection);
}
u64 mmdevapi_W2mCollectionAddRef(u64 self) {
    return w2m_logged(W2M_API_W2mCollectionAddRef, self, (u32)++((struct object *)self)->refs);
}
u64 mmdevapi_W2mCollectionRelease(u64 self) {
    struct object *o = (struct object *)self;
    s64 left = --o->refs;
    if (left == 0) w2m_heap_release(W2M_API_W2mCollectionRelease, self);
    return w2m_logged(W2M_API_W2mCollectionRelease, self, (u32)left);
}
u64 mmdevapi_W2mCollectionGetCount(u64 self, u64 count) {
    (void)self;
    *(u32 *)count = 0;
    return w2m_logged(W2M_API_W2mCollectionGetCount, self, S_OK);
}
u64 mmdevapi_W2mCollectionItem(u64 self, u64 index, u64 out) {
    (void)self; (void)out;
    w2m_unimplemented(W2M_API_W2mCollectionItem, index);                          /* an empty collection */
    return 0;
}

/* PropVariantClear(pvar): a PROPVARIANT (24 bytes) emptied; VT_LPWSTR's string freed (CoTaskMemFree) */
u64 ole32_PropVariantClear(u64 pv) {
    unsigned short vt = *(unsigned short *)pv;
    if (vt == VT_LPWSTR) {
        u64 s = *(u64 *)(pv + 8);
        if (s) w2m_heap_release(W2M_API_PropVariantClear, s);
    } else if (vt != VT_EMPTY && !(vt >= 2 && vt <= 7) && vt != 11 && !(vt >= 16 && vt <= 23)) {
        w2m_unimplemented(W2M_API_PropVariantClear, vt);                           /* not in the rows */
    }
    for (int i = 0; i < 24; i++) ((u8 *)pv)[i] = 0;
    return w2m_logged(W2M_API_PropVariantClear, vt, S_OK);
}

/* CoCreateGuid(pguid): 16 random bytes, version 4 and the 10 variant (RFC 4122 4.4) */
u64 ole32_CoCreateGuid(u64 out) {
    u8 *g = (u8 *)out;
    w2m_host(W2M_HOST_RANDOM, 16, out, 0, 0, 0);
    g[7] = (u8)((g[7] & 0x0F) | 0x40);           /* Data3's high nibble (little-endian u16 at 6) */
    g[8] = (u8)((g[8] & 0x3F) | 0x80);
    return w2m_logged(W2M_API_CoCreateGuid, out, S_OK);
}

/* BSTRs: the length in bytes in the four bytes before the string, a NUL after it; the block starts 8 bytes before */
static u64 bstr(u64 api, const unsigned short *s, u32 chars) {
    u8 *block = (u8 *)w2m_heap_take(8 + 2 * (u64)chars + 2);
    if (!block) w2m_unimplemented(api, chars);
    unsigned short *p = (unsigned short *)(block + 8);
    *(u32 *)(block + 4) = 2 * chars;
    for (u32 i = 0; i < chars; i++) p[i] = s ? s[i] : 0;
    p[chars] = 0;
    return (u64)p;
}

u64 oleaut32_SysAllocString(u64 s) {
    if (!s) return w2m_logged(W2M_API_SysAllocString, 0, 0);
    u32 n = 0;
    while (((const unsigned short *)s)[n]) n++;
    return w2m_logged(W2M_API_SysAllocString, n, bstr(W2M_API_SysAllocString, (const unsigned short *)s, n));
}

W2M_VOID u64 oleaut32_SysFreeString(u64 b) {
    if (b) w2m_heap_release(W2M_API_SysFreeString, b - 8);
    return w2m_logged(W2M_API_SysFreeString, b, 0);
}

u64 oleaut32_SysStringLen(u64 b) {
    return w2m_logged(W2M_API_SysStringLen, b, b ? *(u32 *)(b - 4) / 2 : 0);
}

W2M_VOID u64 oleaut32_VariantInit(u64 v) {
    *(unsigned short *)v = VT_EMPTY;
    return w2m_logged(W2M_API_VariantInit, v, 0);
}

/* VariantClear(pvarg): a BSTR freed; a scalar left; the VARIANT's type VT_EMPTY */
u64 oleaut32_VariantClear(u64 v) {
    unsigned short vt = *(unsigned short *)v;
    if (vt == VT_BSTR) {
        u64 b = *(u64 *)(v + 8);
        if (b) w2m_heap_release(W2M_API_VariantClear, b - 8);
    } else if (vt != VT_EMPTY && vt != 1 && !(vt >= 2 && vt <= 7) && vt != 11 && !(vt >= 16 && vt <= 23)) {
        w2m_unimplemented(W2M_API_VariantClear, vt);                               /* objects, arrays: not in the rows */
    }
    *(unsigned short *)v = VT_EMPTY;
    return w2m_logged(W2M_API_VariantClear, vt, S_OK);
}
