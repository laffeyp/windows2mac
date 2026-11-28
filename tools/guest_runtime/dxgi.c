// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* dxgi.c — CreateDXGIFactory and the factory, adapters and outputs the game walks (Sprint 063, family "dxgi").
 *
 * The game (UnityPlayer RVA 0x520700; game run 38206263976 seq 3298-3301) creates an IDXGIFactory, enumerates its
 * adapters and each adapter's outputs, finds the output on its window's monitor and reads that adapter's memory
 * sizes, then releases everything. From Sprint 383 contracts/windows.dxgi.dxgi 0.3.0 records the laptop answering
 * (laptop runs lap-20261013T231059422Z-136bec80-036 and -037, the runners' two binaries, all four bodies byte-equal;
 * until Sprint 383 the runners', 0.2.0: two Basic Render adapters, 8,587,180,032 bytes shared, \\.\DISPLAY1 at
 * 0,0-1024,768):
 *   - CreateDXGIFactory(IID_IDXGIFactory): S_OK and a new factory holding one reference; an interface it does not
 *     know: E_NOINTERFACE and *ppFactory NULL; a NULL ppFactory: E_POINTER
 *   - three adapters (ADAPTERS below): "Intel(R) UHD Graphics" with 128 MiB of dedicated video memory and one output,
 *     \\.\DISPLAY12, attached, rotation 1 (identity), on the primary monitor; "NVIDIA GeForce RTX 3050 6GB Laptop
 *     GPU" and "Microsoft Basic Render Driver" with none; each 16,985,460,736 bytes of shared system memory. The
 *     output's desktop coordinates are 0,0-1536,864 in the probe's rows, a program without a DPI awareness in its
 *     manifest seeing the 120 DPI desktop at 96; for a PerMonitorV2 program (the game) no stored row gives them: not
 *     measured, and w2m gives the monitor's rectangle as such a program sees it (display.c, 0,0-1920,1080)
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
#define SHARED_SYSTEM_MEMORY 16985460736ull  /* the laptop's (rows adapter.0, adapter.1, adapter.2) */

/* The laptop's adapters in EnumAdapters' order (rows adapter.<i>): DXGI_ADAPTER_DESC's fields but the LUID */
static const struct adapter {
    const char *name;
    u32 vendor, device, subsys, revision;
    u64 dedicated_video;
} ADAPTERS[3] = {
    {"Intel(R) UHD Graphics", 0x8086, 0xA7A8, 0x8DCD103C, 4, 134217728ull},
    {"NVIDIA GeForce RTX 3050 6GB Laptop GPU", 0x10DE, 0x25AC, 0x8DCD103C, 0xA1, 6292504576ull},
    {"Microsoft Basic Render Driver", 0x1414, 0x8C, 0, 0, 0},
};
#define ADAPTER_COUNT 3

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

/* EnumAdapters(Adapter, ppAdapter): three adapters, each a new object */
u64 dxgi_W2mFactoryEnumAdapters(u64 self, u64 index, u64 out) {
    (void)self;
    if (!out) w2m_unimplemented(W2M_API_W2mFactoryEnumAdapters, index);   /* not in the rows */
    if ((u32)index >= ADAPTER_COUNT) {
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

/* EnumOutputs(Output, ppOutput): the first adapter has \\.\DISPLAY12, the others none */
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
    const struct adapter *a = &ADAPTERS[((struct object *)self)->index];
    u8 *p = (u8 *)desc;
    for (u32 i = 0; i < 0x130; i++) p[i] = 0;
    for (u32 i = 0; a->name[i]; i++) ((u16 *)p)[i] = (u16)a->name[i];   /* Description: WCHAR[128] */
    *(u32 *)(p + 0x100) = a->vendor;                                 /* VendorId */
    *(u32 *)(p + 0x104) = a->device;                                 /* DeviceId */
    *(u32 *)(p + 0x108) = a->subsys;                                 /* SubSysId */
    *(u32 *)(p + 0x10C) = a->revision;                               /* Revision */
    *(u64 *)(p + 0x110) = a->dedicated_video;                        /* DedicatedVideoMemory; DedicatedSystemMemory 0 */
    *(u64 *)(p + 0x120) = SHARED_SYSTEM_MEMORY;                      /* SharedSystemMemory */
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
    static const char NAME[] = "\\\\.\\DISPLAY12";
    u8 *p = (u8 *)desc;
    for (u32 i = 0; i < 0x60; i++) p[i] = 0;
    for (u32 i = 0; NAME[i]; i++) ((u16 *)p)[i] = (u16)NAME[i];      /* DeviceName: WCHAR[32] */
    int aware = (w2m_manifest() & W2M_MANIFEST_PER_MONITOR_V2) != 0;
    *(u32 *)(p + 0x48) = aware ? 1920 : 1536;                        /* DesktopCoordinates: 0, 0 to these (the header) */
    *(u32 *)(p + 0x4C) = aware ? 1080 : 864;
    *(u32 *)(p + 0x50) = 1;                                          /* AttachedToDesktop */
    *(u32 *)(p + 0x54) = 1;                                          /* Rotation: DXGI_MODE_ROTATION_IDENTITY */
    *(u64 *)(p + 0x58) = MONITOR;                                    /* Monitor */
    return w2m_logged(W2M_API_W2mOutputGetDesc, desc, S_OK);
}

/* GetDisplayModeList(EnumFormat, Flags, pNumModes, pDesc) (Sprint 065; UnityPlayer RVA 0xdc5084 asks for
 * DXGI_FORMAT_R8G8B8A8_UNORM, flags 0, the count first). The laptop's rows (Sprint 383; runs
 * lap-20261013T231059422Z-136bec80-036 and -037): for each of six formats 34 modes under flags 0 and INTERLACED and 66
 * under SCALING and both; for format 28 under flags 0 the 17 sizes, 640x480 to 1920x1080, each at 14712000/245196
 * and then 35309000/245196 Hz (60 and 144), progressive (1), scaling unspecified (0), the format asked for; under
 * flags 2 each of the 16 smaller sizes four times, at 60 Hz with scaling 0 and 2 (stretched), then at 144 Hz with
 * the two, and 1920x1080 at 60 and 144 Hz with scaling 0; a buffer short of the count: DXGI_ERROR_MORE_DATA, the
 * first *pNumModes written and the count left; a longer one: S_OK and the count; a NULL pNumModes:
 * DXGI_ERROR_INVALID_CALL. Other formats and flags, and the lists the rows lack, stop the run. (The runners': 13
 * sizes at 64/1 Hz under any flags, 0.2.0.) */
#define DXGI_ERROR_INVALID_CALL 0x887A0001u
#define DXGI_ERROR_MORE_DATA 0x887A0003u
static const u16 SIZES[17][2] = {{640, 480}, {800, 600}, {1024, 768}, {1152, 864}, {1280, 600}, {1280, 720},
                                 {1280, 768}, {1280, 800}, {1280, 960}, {1280, 1024}, {1360, 768}, {1366, 768},
                                 {1400, 1050}, {1440, 900}, {1600, 900}, {1680, 1050}, {1920, 1080}};
#define HZ_60 14712000u
#define HZ_144 35309000u
#define HZ_DENOMINATOR 245196u
#define SCALING_STRETCHED 2u

u64 dxgi_W2mOutputGetDisplayModeList(u64 self, u64 format, u64 flags, u64 count, u64 rsp) {
    (void)self;
    u64 desc = *(u64 *)(rsp + 0x28);
    format = (u32)format;
    flags = (u32)flags;
    if (!(format == 28 || format == 29 || format == 87 || format == 91 || format == 24 || format == 10) || flags > 3)
        w2m_unimplemented(W2M_API_W2mOutputGetDisplayModeList, format);
    if (!count) return w2m_logged(W2M_API_W2mOutputGetDisplayModeList, format, DXGI_ERROR_INVALID_CALL);
    u32 *n = (u32 *)count;
    u32 modes = (flags & 2) ? 66 : 34;
    if (!desc) {
        *n = modes;
        return w2m_logged(W2M_API_W2mOutputGetDisplayModeList, format, S_OK);
    }
    if (format != 28 || (flags != 0 && flags != 2)) w2m_unimplemented(W2M_API_W2mOutputGetDisplayModeList, format);
    u32 room = *n < modes ? *n : modes;
    for (u32 i = 0; i < room; i++) {                                 /* DXGI_MODE_DESC: 7 UINTs */
        u32 size, hz, scaling = 0;
        if (!flags) { size = i / 2; hz = (i & 1) ? HZ_144 : HZ_60; }
        else if (i < 64) { size = i / 4; hz = (i & 2) ? HZ_144 : HZ_60; scaling = (i & 1) ? SCALING_STRETCHED : 0; }
        else { size = 16; hz = i == 64 ? HZ_60 : HZ_144; }
        u32 *m = (u32 *)desc + 7 * i;
        m[0] = SIZES[size][0];
        m[1] = SIZES[size][1];
        m[2] = hz;
        m[3] = HZ_DENOMINATOR;
        m[4] = (u32)format;
        m[5] = 1;                                                    /* DXGI_MODE_SCANLINE_ORDER_PROGRESSIVE */
        m[6] = scaling;
    }
    if (*n < modes) return w2m_logged(W2M_API_W2mOutputGetDisplayModeList, format, DXGI_ERROR_MORE_DATA);
    *n = modes;
    return w2m_logged(W2M_API_W2mOutputGetDisplayModeList, format, S_OK);
}
