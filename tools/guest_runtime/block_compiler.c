// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* block_compiler.c — a dispatch miss compiled inside the VM (Sprint 083; BLACKBOARD Decision 2026-10-15, the inferred
 * compiler put in the VM). The dispatcher's miss stub calls w2m_vm_compile(target) on the runtime's stack before it
 * exits to the host. It compiles the block at `target` with the compiler in C (tools/vm_compiler) where every byte it
 * reads is on a page behind translated code (the code-page set the host keeps), puts it in the code cache, the
 * reconstruction table and the dispatch table, and logs it for the host (vm_backend.VM._vm_compiled). Anything it
 * refuses, it counts by cause and leaves to the host, which compiles it in Python as before.
 *
 * Addresses come from vm_backend.runtime_defines: W2M_VMC_VA (the words below, the counts, the log),
 * W2M_CODE_PAGE_SET_VA and W2M_CODE_PAGE_BITS, W2M_DISPATCH_VA and W2M_DISPATCH_BITS, W2M_RECON_VA,
 * W2M_RECON_COUNT_VA and W2M_RECON_ENTRIES, W2M_CODE_START and W2M_CODE_END, W2M_MONITOR_VA.
 */
#include "runtime.h"
#include "vm_compiler.h"

/* the words at W2M_VMC_VA (vm_backend.VMC_*) */
enum { VMC_ENABLED, VMC_CURSOR, VMC_LOG_COUNT, VMC_EXIT, VMC_SLICE, VMC_HOST_ONLY };
#define VMC_SYMBOLS_OFF 0x40            /* the table's symbols' addresses, in vmc_symbol_name's order (the host's) */
#define VMC_COUNTS_OFF 0x100
#define VMC_LOG_OFF 0x1000
#define VMC_LOG_ENTRY 0x80
#define VMC_LOG_PAGES 8

/* the counts (vm_backend.VMC_COUNT_NAMES) */
enum { C_COMPILED, C_NOT_CODE, C_HOST_ONLY, C_STATUS, C_SCAN_PAGES = C_STATUS + 7, C_LOG_FULL, C_CODE_FULL,
       C_RECON_FULL, C_CUT, C_NEEDS_SPLIT = C_CUT + 6, C_PROTECTED, C_PAUSE };
static const u8 CUT_INDEX[8] = {0, C_CUT + 0, C_CUT + 1, C_CUT + 2, C_CUT + 3, 0, C_CUT + 4, C_CUT + 5};

/* one log entry; the host reads it at W2M_VMC_VA + VMC_LOG_OFF + VMC_LOG_ENTRY * i */
struct log_entry {
    u64 start, end, code;           /* the block's x86 range and its code's address */
    u32 recon_first, nmarks;        /* its marks in the reconstruction table */
    u32 count;                      /* its instructions */
    s32 cut;                        /* 0, or the status of the instruction it stopped before */
    u32 live_out;                   /* the flags vmc_live_out found after the block; 0xFFFFFFFF when it was cut or did
                                       not end naturally (a branch or the count), and every flag is live at its end */
    u32 live_used;                  /* the flags live at its end as compiled */
    u32 npages;
    u32 pad;
    u64 pages[VMC_LOG_PAGES];       /* the pages the successor scan read, which the block joins the owners of */
};
_Static_assert(sizeof(struct log_entry) <= VMC_LOG_ENTRY, "log entry");

static volatile u64 *vmc(void) { return (volatile u64 *)W2M_VMC_VA; }
static void count(int i) { ((volatile u64 *)(W2M_VMC_VA + VMC_COUNTS_OFF))[i]++; }

/* `page` in the code-page set: open addressing, linear probing; 0 an empty slot, 1 a removed one (vm_backend
   _page_set_slot) */
static int code_page(u64 page) {
    const volatile u64 *set = (const volatile u64 *)W2M_CODE_PAGE_SET_VA;
    u64 slots = 1ull << W2M_CODE_PAGE_BITS;
    u64 i = ((page >> 12) * 0x9E3779B97F4A7C15ull) >> (64 - W2M_CODE_PAGE_BITS);
    for (u64 n = 0; n < slots; n++) {
        u64 v = set[i];
        if (v == page)
            return 1;
        if (v == 0)
            return 0;
        i = (i + 1) & (slots - 1);
    }
    return 0;
}

/* `page` added to the code-page set (vm_backend._page_set_add: the first removed slot on the probe, else the empty
   one that ends it) */
static void set_add(u64 page) {
    volatile u64 *set = (volatile u64 *)W2M_CODE_PAGE_SET_VA;
    u64 slots = 1ull << W2M_CODE_PAGE_BITS;
    u64 i = ((page >> 12) * 0x9E3779B97F4A7C15ull) >> (64 - W2M_CODE_PAGE_BITS), free = slots;
    for (u64 n = 0; n < slots; n++) {
        u64 v = set[i];
        if (v == page)
            return;
        if (v == 1 && free == slots)
            free = i;
        if (v == 0)
            break;
        i = (i + 1) & (slots - 1);
    }
    set[free == slots ? i : free] = page;
}

/* Sprint 084: a page made read-only here, as vm_backend._protect_code does on the host, so a guest write to it faults
   (HVC_CODE_WRITE) and drops the blocks over it. The stage-1 walk is _page_descriptor's, but a 2 MB or 1 GB block
   descriptor over the page is left to the host: replacing it with a table while it may be in a TLB needs
   break-before-make (Arm ARM D8), which the host's exit gives. A permission change needs none. */
enum { PAGE_OK, PAGE_UNMAPPED, PAGE_IN_BLOCK };
static volatile u64 *page_descriptor(u64 va, int *why) {
    u64 off = W2M_TRANSLATION_TABLES_OFF;
    for (int shift = 39; shift >= 21; shift -= 9) {
        u64 desc = ((volatile u64 *)(W2M_MONITOR_VA + off))[(va >> shift) & 0x1FF];
        if (!(desc & 1)) {
            *why = PAGE_UNMAPPED;
            return 0;
        }
        if (!(desc & 2)) {
            *why = PAGE_IN_BLOCK;
            return 0;
        }
        off = (desc & 0x0000FFFFFFFFF000ull) - W2M_IPA_BASE;
    }
    volatile u64 *at = (volatile u64 *)(W2M_MONITOR_VA + off) + ((va >> 12) & 0x1FF);
    *why = *at & 1 ? PAGE_OK : PAGE_UNMAPPED;
    return *why == PAGE_OK ? at : 0;
}

/* guest code may be here: below the process area and outside the monitor (whose bytes no guest instruction is) */
static int guest_page(u64 page) {
    return page < W2M_PROCESS_VA && !(page >= W2M_MONITOR_VA && page < W2M_MONITOR_VA + W2M_MONITOR_SIZE);
}

static void protect(volatile u64 *desc, u64 page) {
    *desc |= W2M_AP_READ_ONLY;
    __asm__ volatile("dsb ishst\n\ttlbi vaae1is, %0\n\tdsb ish\n\tisb" ::"r"(page >> 12) : "memory");
    set_add(page);
    count(C_PROTECTED);
}

struct scan { u64 pages[VMC_LOG_PAGES]; u32 npages; int overflow; };

static int code_bytes(void *ctx, u64 address, const u8 **bytes) {
    (void)ctx;
    u64 page = address & ~0xFFFull;
    if (!code_page(page))
        return 0;
    u64 n = page + 0x1000 - address;
    if (n < 15 && code_page(page + 0x1000))
        n += 0x1000;
    *bytes = (const u8 *)address;
    return n > 15 ? 15 : (int)n;
}

static void scanned(void *ctx, u64 page) {
    struct scan *s = ctx;
    for (u32 i = 0; i < s->npages; i++)
        if (s->pages[i] == page)
            return;
    if (s->npages == VMC_LOG_PAGES)
        s->overflow = 1;
    else
        s->pages[s->npages++] = page;
}

/* A dispatch pair stored whole, by one `stp` (Sprint 284): the dispatcher reads a pair by one `ldp`, single-copy atomic
 * with FEAT_LSE2, so another vCPU sees the old pair or the new, never a target with another pair's code. */
static void store_pair(volatile u64 *p, u64 target, u64 code) {
    __asm__ volatile("stp %0, %1, [%2]" ::"r"(target), "r"(code), "r"(p) : "memory");
}

/* the dispatch pair for `target` (vm_backend.dispatch_slot, _dispatch_insert): an empty slot or its own, else first,
 * the first way's pair moved to the second before the first is written */
static void dispatch_insert(u64 target, u64 code) {
    u64 slot = (target ^ target >> W2M_DISPATCH_BITS ^ target >> 2 * W2M_DISPATCH_BITS) &
               ((1ull << W2M_DISPATCH_BITS) - 2);
    volatile u64 *p = (volatile u64 *)(W2M_DISPATCH_VA + 16 * slot);
    if (p[0] == ~0ull || p[0] == target) {
        store_pair(p, target, code);
    } else if (p[2] == ~0ull || p[2] == target) {
        store_pair(p + 2, target, code);
    } else {
        /* way 0 read whole by one `ldp` before it is moved to way 1 (Sprint 286) */
        u64 t0, c0;
        __asm__ volatile("ldp %0, %1, [%2]" : "=r"(t0), "=r"(c0) : "r"(p) : "memory");
        store_pair(p + 2, t0, c0);
        store_pair(p, target, code);
    }
}

/* new code made visible to instruction fetch: clean to the point of unification, invalidate the instruction cache
   (the sequence of Linux's arch/arm64 caches_clean_inval_pou) */
static void sync_code(u64 start, u64 end) {
    u64 ctr;
    __asm__ volatile("mrs %0, ctr_el0" : "=r"(ctr));
    u64 dline = 4ull << (ctr >> 16 & 0xF), iline = 4ull << (ctr & 0xF);
    for (u64 a = start & ~(dline - 1); a < end; a += dline)
        __asm__ volatile("dc cvau, %0" ::"r"(a) : "memory");
    __asm__ volatile("dsb ish" ::: "memory");
    for (u64 a = start & ~(iline - 1); a < end; a += iline)
        __asm__ volatile("ic ivau, %0" ::"r"(a) : "memory");
    __asm__ volatile("dsb ish\n\tisb" ::: "memory");
}

/* The block's extent as vmc_block takes it (Monitor._compile's rule over the window's bytes): its last instruction,
   its end, its count, and whether it ended there naturally (a branch, or the count) rather than at the window's edge
   or bytes with no form. */
/* PAUSE (F3 90, F3 the last of any legacy prefixes or REX before 90): compile_insn yields the slice at one when another
 * thread is ready (Sprint 096), and the stencil table's PAUSE does not, so a block ends before it and a block that starts
 * at one goes to the host. A form this misses compiles here without the yield, which is correct code, only slower. */
static int is_pause(const u8 *p, int n) {
    if (n < 2 || p[n - 1] != 0x90) return 0;
    int f3 = 0;
    for (int i = 0; i < n - 1; i++) {
        u8 b = p[i];
        if (b == 0xF3) f3 = 1;
        else if (b == 0xF2) f3 = 0;
        else if (!((b >= 0x40 && b <= 0x4F) || b == 0x66 || b == 0x67 || b == 0x2E || b == 0x3E || b == 0x26 ||
                   b == 0x36 || b == 0x64 || b == 0x65 || b == 0xF0)) return 0;
    }
    return f3;
}

static int extent(u64 start, const u8 *mem, u64 base, u64 size, struct vmc_insn *last, u64 *end, int *natural) {
    u64 addr = start;
    int n = 0;
    *natural = 0;
    while (n < VMC_BLOCK_LIMIT) {
        if (addr - base >= size)
            return n;
        u64 avail = size - (addr - base);
        if (vmc_decode(mem + (addr - base), avail > 15 ? 15 : (int)avail, addr, last) < 0)
            return n;
        if (is_pause(mem + (addr - base), last->length))
            return n;                                    /* compile_insn's PAUSE, at the host (Sprint 096) */
        addr += (u64)last->length;
        *end = addr;
        n++;
        if (vmc_ends(last)) {
            *natural = 1;
            return n;
        }
    }
    *natural = 1;
    return n;
}

u64 w2m_vm_compile(u64 target) {
    volatile u64 *v = vmc();
    if (target == v[VMC_HOST_ONLY]) {
        count(C_HOST_ONLY);
        return 0;
    }
    u64 page = target & ~0xFFFull;
    volatile u64 *unprotected = 0;                       /* the page's descriptor, to protect once the block compiles */
    if (!code_page(page)) {
        int why = PAGE_UNMAPPED;
        unprotected = guest_page(page) ? page_descriptor(page, &why) : 0;
        if (!unprotected) {
            count(why == PAGE_IN_BLOCK ? C_NEEDS_SPLIT : C_NOT_CODE);
            return 0;
        }
    }
    u64 n_log = v[VMC_LOG_COUNT];
    if (n_log >= W2M_VMC_LOG_ENTRIES) {
        count(C_LOG_FULL);
        return 0;
    }
    u64 size = code_page(page + 0x1000) ? 0x2000 : 0x1000;
    const u8 *mem = (const u8 *)page;
    struct vmc_insn last;
    u64 end = target;
    int natural;
    int extent_count = extent(target, mem, page, size, &last, &end, &natural);
    if (!extent_count) {
        struct vmc_insn first;
        u64 avail = page + size - target;
        int s = vmc_decode((const u8 *)target, avail > 15 ? 15 : (int)avail, target, &first);
        if (s >= 0 && is_pause((const u8 *)target, first.length)) {
            count(C_PAUSE);
            return 0;
        }
        count(C_STATUS + (s < 0 && s >= -7 ? -s - 1 : 0));
        return 0;
    }
    struct scan scan = {{0}, 0, 0};
    int live_out = -1;
    if (natural) {
        struct vmc_memory m = {code_bytes, scanned, &scan, 0, 0};
        live_out = vmc_live_out(&m, &last, end);
        if (scan.overflow) {
            count(C_SCAN_PAGES);
            return 0;
        }
    }
    u64 cursor = v[VMC_CURSOR];
    u64 code = W2M_MONITOR_VA + cursor;
    u64 room = (W2M_CODE_END - code) / 4;
    struct vmc_mark marks[VMC_BLOCK_LIMIT + 2];
    struct vmc_block_in in = {target, mem, page, size, 0, 0, code, v[VMC_EXIT], v[VMC_SLICE],
                              (const u64 *)(W2M_VMC_VA + VMC_SYMBOLS_OFF), live_out < 0 ? VMC_FLAGS_ALL : live_out};
    struct vmc_block_out out = {(u32 *)code, room > 8192 ? 8192 : (int)room, marks, 0, 0, 0, 0, 0};
    int words = vmc_block(&in, &out);
    if (words < 0) {
        count(words == VMC_FULL ? C_CODE_FULL : C_STATUS + (words >= -7 ? -words - 1 : 0));
        return 0;
    }
    volatile u64 *recon_count = (volatile u64 *)W2M_RECON_COUNT_VA;
    u64 first = *recon_count;
    if (first + (u64)out.nmarks > W2M_RECON_ENTRIES) {
        count(C_RECON_FULL);
        return 0;
    }
    u64 *recon = (u64 *)W2M_RECON_VA;
    for (int i = 0; i < out.nmarks; i++) {
        recon[2 * (first + i)] = marks[i].host;
        recon[2 * (first + i) + 1] = marks[i].guest;
    }
    sync_code(code, code + 4 * (u64)words);
    if (unprotected)
        protect(unprotected, page);                      /* the replay gives it its code_pages entry */
    struct log_entry *e = (struct log_entry *)(W2M_VMC_VA + VMC_LOG_OFF + VMC_LOG_ENTRY * n_log);
    e->start = target;
    e->end = out.end;
    e->code = code;
    e->recon_first = (u32)first;
    e->nmarks = (u32)out.nmarks;
    e->count = (u32)out.count;
    e->cut = out.cut;
    int whole = natural && out.count == extent_count;   /* not cut: the scan's flags are the block's */
    e->live_out = whole ? (u32)live_out : 0xFFFFFFFFu;
    e->live_used = whole ? (u32)live_out : (u32)VMC_FLAGS_ALL;
    e->npages = scan.npages;
    for (u32 i = 0; i < scan.npages; i++)
        e->pages[i] = scan.pages[i];
    if (out.cut < 0 && out.cut >= -7)
        count(CUT_INDEX[-out.cut]);
    /* the counts move last: a compile abandoned before here left words past the cursor and nothing else */
    *recon_count = first + (u64)out.nmarks;
    v[VMC_CURSOR] = cursor + 4 * (u64)words;
    v[VMC_LOG_COUNT] = n_log + 1;
    count(C_COMPILED);
    dispatch_insert(target, code);
    return code;
}
