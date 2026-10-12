// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* dxgi.c — CreateDXGIFactory and the factory, adapters and outputs the game walks (Sprint 063, family "dxgi").
 *
 * The game (UnityPlayer RVA 0x520700; game run 38206263976 seq 3298-3301) creates an IDXGIFactory, enumerates its
 * adapters and each adapter's outputs, finds the output on its window's monitor and reads that adapter's memory
 * sizes, then releases everything. contracts/windows.dxgi.dxgi records the runners answering (no GPU; Server 2025 and
 * 2022, Intel and AMD, identical rows):
 *   - CreateDXGIFactory(IID_IDXGIFactory): S_OK and a new factory holding one reference; an interface it does not
 *     know: E_NOINTERFACE and *ppFactory NULL; a NULL ppFactory: E_POINTER
 *   - two adapters, both "Microsoft Basic Render Driver" (vendor 0x1414, device 0x8C), no dedicated memory and
 *     8,587,180,032 bytes of shared system memory; the first has one output, \\.\DISPLAY1 at 0,0-1024,768, attached,
 *     rotation 1 (identity), on the primary monitor; the second has none
 *   - an enumeration past its end: DXGI_ERROR_NOT_FOUND and the out pointer NULL; each enumerated object is new and
 *     holds one reference, so Release answers 0
 *   - QueryInterface on the factory for IUnknown or IDXGIFactory: S_OK, the same object; another interface:
 *     E_NOINTERFACE and NULL
 * The objects' vtables live in the runtime image and the host fills each slot with the thunk
 * dxgi.dll!W2m<Factory|Adapter|Output><Method> (vm_backend.py COM_VTABLES); a method without a service here stops
 * the run by that name: the rows hold no other call. The interfaces DXGI also answers (IDXGIFactory1 and 2) stop
 * the run: no row shows their objects.
 */
#include "runtime.h"

typedef unsigned short u16;
typedef struct { u32 a; u16 b, c; u8 d[8]; } guid;

#define S_OK 0u
#define E_NOINTERFACE 0x80004002u
#define E_POINTER 0x80004003u
#define DXGI_ERROR_NOT_FOUND 0x887A0002u
#define MONITOR 0x10001ull                   /* display.c's one monitor */
#define SHARED_SYSTEM_MEMORY 8587180032ull   /* the runners' (rows adapter.0, adapter.1) */

static const guid IID_IUnknown = {0x00000000, 0x0000, 0x0000, {0xC0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x46}};
static const guid IID_IDXGIFactory = {0x7b7166ec, 0x21c7, 0x44ae, {0xb2, 0x1a, 0xc9, 0xae, 0x32, 0x1a, 0xe3, 0x69}};
static const guid IID_IDXGIFactory1 = {0x770aae78, 0xf26f, 0x4dba, {0xa8, 0x29, 0x25, 0x3c, 0x83, 0xd1, 0xb3, 0x87}};
static const guid IID_IDXGIFactory2 = {0x50c83a1c, 0xe072, 0x4c48, {0x87, 0xb0, 0x36, 0x30, 0xfa, 0x36, 0xa6, 0xd0}};

/* dxgi.h's vtable order: IUnknown (3), IDXGIObject (SetPrivateData, SetPrivateDataInterface, GetPrivateData,
 * GetParent), then IDXGIFactory's 5, IDXGIAdapter's 3, IDXGIOutput's 12 */
volatile u64 w2m_dxgi_factory_vtable[12];
volatile u64 w2m_dxgi_adapter_vtable[10];
volatile u64 w2m_dxgi_output_vtable[19];

static int same(const guid *x, const guid *y) {
    const u8 *p = (const u8 *)x, *q = (const u8 *)y;
    for (int i = 0; i < 16; i++)
        if (p[i] != q[i]) return 0;
    return 1;
}

/* An object: its vtable, its references, and for an adapter its index. */
struct object { u64 vtable; s64 refs; u32 index; };

static struct object *made(u64 api, volatile u64 *vtable, u32 index) {
    struct object *o = (struct object *)w2m_heap_take(sizeof *o);
    if (!o) w2m_unimplemented(api, sizeof *o);                       /* the heap's arena is full */
    o->vtable = (u64)vtable;
    o->refs = 1;
    o->index = index;
    return o;
}

static u64 released(u64 api, u64 self) {
    struct object *o = (struct object *)self;
    s64 left = --o->refs;
    if (left == 0) w2m_heap_release(api, self);
    return w2m_logged(api, self, (u32)left);
}

/* CreateDXGIFactory(riid, ppFactory) */
u64 dxgi_CreateDXGIFactory(u64 iid, u64 out) {
    if (!out) return w2m_logged(W2M_API_CreateDXGIFactory, iid, E_POINTER);
    const guid *g = (const guid *)iid;
    if (same(g, &IID_IUnknown) || same(g, &IID_IDXGIFactory1) || same(g, &IID_IDXGIFactory2))
        w2m_unimplemented(W2M_API_CreateDXGIFactory, iid);           /* answered by DXGI; no row shows them */
    if (!same(g, &IID_IDXGIFactory)) {
        *(u64 *)out = 0;
        return w2m_logged(W2M_API_CreateDXGIFactory, iid, E_NOINTERFACE);
    }
    *(u64 *)out = (u64)made(W2M_API_CreateDXGIFactory, w2m_dxgi_factory_vtable, 0);
    return w2m_logged(W2M_API_CreateDXGIFactory, iid, S_OK);
}

/* IDXGIFactory */
u64 dxgi_W2mFactoryQueryInterface(u64 self, u64 iid, u64 out) {
    const guid *g = (const guid *)iid;
    if (same(g, &IID_IUnknown) || same(g, &IID_IDXGIFactory)) {
        ((struct object *)self)->refs++;
        *(u64 *)out = self;
        return w2m_logged(W2M_API_W2mFactoryQueryInterface, iid, S_OK);
    }
    if (same(g, &IID_IDXGIFactory1) || same(g, &IID_IDXGIFactory2))
        w2m_unimplemented(W2M_API_W2mFactoryQueryInterface, iid);
    *(u64 *)out = 0;
    return w2m_logged(W2M_API_W2mFactoryQueryInterface, iid, E_NOINTERFACE);
}
u64 dxgi_W2mFactoryAddRef(u64 self) {
    return w2m_logged(W2M_API_W2mFactoryAddRef, self, (u32)++((struct object *)self)->refs);
}
u64 dxgi_W2mFactoryRelease(u64 self) { return released(W2M_API_W2mFactoryRelease, self); }

/* EnumAdapters(Adapter, ppAdapter): two adapters, each a new object */
u64 dxgi_W2mFactoryEnumAdapters(u64 self, u64 index, u64 out) {
    (void)self;
    if (!out) w2m_unimplemented(W2M_API_W2mFactoryEnumAdapters, index);   /* not in the rows */
    if ((u32)index >= 2) {
        *(u64 *)out = 0;
        return w2m_logged(W2M_API_W2mFactoryEnumAdapters, index, DXGI_ERROR_NOT_FOUND);
    }
    *(u64 *)out = (u64)made(W2M_API_W2mFactoryEnumAdapters, w2m_dxgi_adapter_vtable, (u32)index);
    return w2m_logged(W2M_API_W2mFactoryEnumAdapters, index, S_OK);
}

/* IDXGIAdapter */
u64 dxgi_W2mAdapterAddRef(u64 self) {
    return w2m_logged(W2M_API_W2mAdapterAddRef, self, (u32)++((struct object *)self)->refs);
}
u64 dxgi_W2mAdapterRelease(u64 self) { return released(W2M_API_W2mAdapterRelease, self); }

/* EnumOutputs(Output, ppOutput): the first adapter has \\.\DISPLAY1, the second none */
u64 dxgi_W2mAdapterEnumOutputs(u64 self, u64 index, u64 out) {
    if (!out) w2m_unimplemented(W2M_API_W2mAdapterEnumOutputs, index);
    if (((struct object *)self)->index != 0 || (u32)index >= 1) {
        *(u64 *)out = 0;
        return w2m_logged(W2M_API_W2mAdapterEnumOutputs, index, DXGI_ERROR_NOT_FOUND);
    }
    *(u64 *)out = (u64)made(W2M_API_W2mAdapterEnumOutputs, w2m_dxgi_output_vtable, 0);
    return w2m_logged(W2M_API_W2mAdapterEnumOutputs, index, S_OK);
}

/* GetDesc(pDesc): DXGI_ADAPTER_DESC, 0x130 bytes */
u64 dxgi_W2mAdapterGetDesc(u64 self, u64 desc) {
    if (!desc) w2m_unimplemented(W2M_API_W2mAdapterGetDesc, 0);
    static const char NAME[] = "Microsoft Basic Render Driver";
    u8 *p = (u8 *)desc;
    for (u32 i = 0; i < 0x130; i++) p[i] = 0;
    for (u32 i = 0; NAME[i]; i++) ((u16 *)p)[i] = (u16)NAME[i];      /* Description: WCHAR[128] */
    *(u32 *)(p + 0x100) = 0x1414;                                    /* VendorId: Microsoft */
    *(u32 *)(p + 0x104) = 0x8C;                                      /* DeviceId */
    *(u64 *)(p + 0x120) = SHARED_SYSTEM_MEMORY;                      /* SharedSystemMemory; dedicated ones 0 */
    *(u32 *)(p + 0x128) = 0x1000 + ((struct object *)self)->index;   /* AdapterLuid.LowPart: nonzero, one each */
    return w2m_logged(W2M_API_W2mAdapterGetDesc, desc, S_OK);
}

/* IDXGIOutput */
u64 dxgi_W2mOutputAddRef(u64 self) {
    return w2m_logged(W2M_API_W2mOutputAddRef, self, (u32)++((struct object *)self)->refs);
}
u64 dxgi_W2mOutputRelease(u64 self) { return released(W2M_API_W2mOutputRelease, self); }

/* GetDesc(pDesc): DXGI_OUTPUT_DESC, 0x60 bytes */
u64 dxgi_W2mOutputGetDesc(u64 self, u64 desc) {
    (void)self;
    if (!desc) w2m_unimplemented(W2M_API_W2mOutputGetDesc, 0);
    static const char NAME[] = "\\\\.\\DISPLAY1";
    u8 *p = (u8 *)desc;
    for (u32 i = 0; i < 0x60; i++) p[i] = 0;
    for (u32 i = 0; NAME[i]; i++) ((u16 *)p)[i] = (u16)NAME[i];      /* DeviceName: WCHAR[32] */
    *(u32 *)(p + 0x48) = 1024;                                       /* DesktopCoordinates: 0, 0, 1024, 768 */
    *(u32 *)(p + 0x4C) = 768;
    *(u32 *)(p + 0x50) = 1;                                          /* AttachedToDesktop */
    *(u32 *)(p + 0x54) = 1;                                          /* Rotation: DXGI_MODE_ROTATION_IDENTITY */
    *(u64 *)(p + 0x58) = MONITOR;                                    /* Monitor */
    return w2m_logged(W2M_API_W2mOutputGetDesc, desc, S_OK);
}
