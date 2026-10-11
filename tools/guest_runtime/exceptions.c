// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* exceptions.c — structured exception dispatch and unwinding (Sprint 053, family "exceptions").
 *
 * An exception (RaiseException, or a fault from runtime.c's w2m_exception) is dispatched as Windows x64 does
 * (Microsoft Learn, "x64 exception handling"): the vectored handlers front to back, then the frames from the
 * exception's context outward, each found by its image's .pdata (or a growable table, function_tables.c) and
 * unwound by its UNWIND_INFO; a frame whose UNWIND_INFO names an exception handler has it called with a
 * DISPATCHER_CONTEXT. A handler that takes the exception unwinds to its frame with RtlUnwindEx, which walks the
 * same frames again calling each termination handler, then resumes the target frame. No taker: the top-level
 * filter (SetUnhandledExceptionFilter), then the process ends with the exception code.
 *
 * Handlers are x86 code. The runtime calls one by setting the guest's registers and sending the bridge to an x86
 * stub the host writes (W2M_HOST_EXCEPTION): `call rax`, then the handler's answer goes to the service
 * W2mExceptionResume, which continues where the dispatch left off. The dispatch keeps its state in a frame on the
 * guest's stack, below the exception's RSP, as Windows's dispatcher keeps its own; the stub's return point marks
 * it, so a walk that reaches the stub (RtlUnwindEx called from a handler) finds the dispatch it is inside. A
 * context resumes through a second stub that loads XMM0-15, MXCSR, the GPRs and RFLAGS from a CONTEXT and returns
 * to its RIP.
 *
 * As contracts/windows.exception.exceptions records Windows answering (Server 2025 and 2022) and
 * contracts/windows.exception.vectored_handlers' dispatch rows:
 *   - RaiseException's record has the code, the flags given (only EXCEPTION_NONCONTINUABLE kept) with 0x80
 *     (EXCEPTION_SOFTWARE_ORIGINATE), the arguments, and no chained record; the last error is left
 *   - a filter's EXCEPTION_EXECUTE_HANDLER runs each __finally between the raise and the __except (the C handler's
 *     unwind), then the __except with the code in RAX; CONTINUE_EXECUTION returns from RaiseException
 *   - CONTINUE_EXECUTION on a noncontinuable exception raises STATUS_NONCONTINUABLE_EXCEPTION (flags 0x81, no
 *     chained record), which the frames see from the start again
 *   - vectored handlers run before any frame's; an access violation's record has the address and two parameters
 *   - an exception no frame takes reaches the top-level filter
 * What the rows do not reach stops the run by name: a nested exception from a handler, a collided unwind,
 * STATUS_UNWIND_CONSOLIDATE (C++ catch blocks), an exit unwind, RtlVirtualUnwind's context pointers.
 */
#include "runtime.h"

#define EXCEPTION_NONCONTINUABLE 0x1u
#define EXCEPTION_UNWINDING 0x2u
#define EXCEPTION_EXIT_UNWIND 0x4u
#define EXCEPTION_STACK_INVALID 0x8u
#define EXCEPTION_TARGET_UNWIND 0x20u
#define EXCEPTION_COLLIDED_UNWIND 0x40u
#define EXCEPTION_SOFTWARE_ORIGINATE 0x80u
#define EXCEPTION_MAXIMUM_PARAMETERS 15u
#define STATUS_NONCONTINUABLE_EXCEPTION 0xC0000025u
#define STATUS_UNWIND 0xC0000027u
#define STATUS_UNWIND_CONSOLIDATE 0x80000029u
#define UNW_FLAG_EHANDLER 1u
#define UNW_FLAG_UHANDLER 2u
#define UNW_FLAG_CHAININFO 4u
#define CONTEXT_CAPTURED 0x10000Fu           /* CONTEXT_AMD64 | CONTROL | INTEGER | SEGMENTS | FLOATING_POINT */
#define DISPOSITION_CONTINUE_EXECUTION 0
#define DISPOSITION_CONTINUE_SEARCH 1
#define FILTER_CONTINUE_EXECUTION (-1)
#define RAX 0
#define RCX 1
#define RDX 2
#define RSP 4
#define R8 8
#define R9 9
#define MAX_FRAMES 100000                    /* a walk longer than this is a cycle */

/* The host's x86 stubs (src/w2m/exception_stubs.py), at the page W2M_EXCEPTION_STUBS answers. */
#define STUB_RETURN 0x00                     /* ret: the point RaiseException's context resumes at */
#define STUB_CALL 0x10                       /* call rax; mov rcx, rax; mov rax, W2mExceptionResume; jmp rax */
#define STUB_CALL_RETURN (STUB_CALL + 2)     /* the return address a handler sees */
#define STUB_RESTORE 0x40                    /* rcx = CONTEXT: XMM0-15, MXCSR, the GPRs, RFLAGS, then RIP */

struct runtime_function { u32 begin, end, unwind; };

struct dispatcher_context {                  /* winnt.h DISPATCHER_CONTEXT (x64), 80 bytes */
    u64 control_pc, image_base, function_entry, establisher_frame, target_ip, context_record;
    u64 language_handler, handler_data, history_table;
    u32 scope_index, fill0;
};
_Static_assert(sizeof(struct dispatcher_context) == 80, "DISPATCHER_CONTEXT size");

enum { KIND_DISPATCH = 1, KIND_UNWIND = 2 };
enum { STEP_VECTORED = 1, STEP_FRAME, STEP_TOP_FILTER };

/* A dispatch or an unwind in progress, on the guest's stack. */
struct frame {
    struct w2m_context ctx;              /* dispatch: the exception's context; unwind: the frame being unwound */
    struct w2m_context walk;             /* the context one frame out */
    struct w2m_exception_record rec;     /* dispatch: the record; unwind: its own when the caller gave none */
    struct dispatcher_context dc;
    u64 pointers[2];                     /* EXCEPTION_POINTERS: the record, the context */
    u64 magic, kind, step, next_vectored;
    u64 record;                          /* the record in use */
    u64 establisher;                     /* the frame whose handler runs */
    u64 target_frame, target_ip, return_value, history;
    u64 frames;                          /* frames walked */
};
#define FRAME_MAGIC 0x6863746170736964ull     /* "dispatch" */
#define FRAME_GAP 0x40                       /* bytes left free below the stack pointer the frame sits under */
_Static_assert(sizeof(struct w2m_context) % 16 == 0, "the walk context stays 16-aligned");

u64 w2m_vectored_handler(u64 k);                         /* vectored_handlers.c: the k-th, front to back; 0 past */
int w2m_table_function(u64 pc, u64 *base, u64 *entry);   /* function_tables.c: a growable table's range */
u32 w2m_mxcsr_now(void);                                 /* runtime.c */

static u64 stubs(void) {
    static u64 page;
    if (!page) page = w2m_host(W2M_HOST_EXCEPTION, W2M_EXCEPTION_STUBS, 0, 0, 0, 0);
    return page;
}

/* ---- finding a frame's unwind data ------------------------------------------------------------------------ */

/* The .pdata entry of an image holding pc: the image's exception directory, searched by BeginAddress. */
static const struct runtime_function *image_function(u64 pc, u64 *base) {
    u64 b = w2m_host(W2M_HOST_MODULE_AT, pc, 0, 0, 0, 0);
    if (!b) return 0;
    *base = b;
    const u8 *nt = (const u8 *)b + *(const u32 *)(b + 0x3c);
    const u8 *opt = nt + 24;
    if (*(const unsigned short *)opt != 0x20B) return 0;                 /* PE32+ only */
    if (*(const u32 *)(opt + 108) <= 3) return 0;                        /* NumberOfRvaAndSizes */
    u32 rva = *(const u32 *)(opt + 112 + 3 * 8), size = *(const u32 *)(opt + 112 + 3 * 8 + 4);
    const struct runtime_function *f = (const struct runtime_function *)(b + rva);
    u64 lo = 0, hi = size / sizeof *f, off = pc - b;
    while (lo < hi) {
        u64 mid = (lo + hi) / 2;
        if (off < f[mid].begin) hi = mid;
        else if (off >= f[mid].end) lo = mid + 1;
        else return &f[mid];
    }
    return 0;
}

/* For function_tables.c's RtlLookupFunctionEntry: an image's entry, its base in *base. 0: none. */
u64 w2m_image_function(u64 pc, u64 *base) {
    u64 b = 0;
    const struct runtime_function *f = image_function(pc, &b);
    if (f) *base = b;
    return (u64)f;
}

static const struct runtime_function *lookup(u64 pc, u64 *base) {
    u64 entry = 0;
    if (w2m_table_function(pc, base, &entry)) return (const struct runtime_function *)entry;   /* 0: a leaf */
    return image_function(pc, base);
}

/* Code a walk may continue in: an image, a growable table's range, or the dispatch's own stubs. Past anything
 * else the walk has left the program's frames (a thread's start stub, or no code at all). */
static int known_code(u64 pc) {
    u64 base = 0, entry = 0;
    if (w2m_table_function(pc, &base, &entry)) return 1;
    if (w2m_host(W2M_HOST_MODULE_AT, pc, 0, 0, 0, 0)) return 1;
    return pc == stubs() + STUB_CALL_RETURN;
}

/* ---- the unwinder ----------------------------------------------------------------------------------------- */

static u64 *gpr(struct w2m_context *c, u32 r) { return &c->gpr[r & 15]; }
static u64 read64(u64 a) { return *(const u64 *)a; }
static void read_xmm(struct w2m_context *c, u32 r, u64 a) {
    for (int j = 0; j < 16; j++) c->flt_save.xmm_registers[r & 15][j] = ((const u8 *)a)[j];
}

/* Whether pc starts the rest of an epilogue (Microsoft Learn, "x64 prolog and epilog": an optional `add rsp,
 * constant` or `lea rsp, [frame register + constant]`, pops of nonvolatile registers, then `ret` or a jump out of
 * the function); if so, the epilogue's effect on c, which then holds the caller's context. */
static int unwind_epilogue(u64 pc, struct w2m_context *c, u32 frame_reg, u64 fbegin, u64 fend) {
    const u8 *p = (const u8 *)pc;
    u64 rsp = c->gpr[RSP];
    if (p[0] == 0x48 && p[1] == 0x83 && p[2] == 0xC4) { rsp += (s64)(signed char)p[3]; p += 4; }
    else if (p[0] == 0x48 && p[1] == 0x81 && p[2] == 0xC4) { rsp += (s64)*(const s32 *)(p + 3); p += 7; }
    else if ((p[0] & 0xFE) == 0x48 && p[1] == 0x8D && frame_reg) {            /* lea rsp, [fp + disp] */
        u32 rm = (p[2] & 7) | (p[0] & 1) << 3, mod = p[2] >> 6;
        if ((p[2] >> 3 & 7) != 4 || rm != frame_reg || (p[2] & 7) == 4) return 0;
        if (mod == 1) { rsp = c->gpr[rm] + (s64)(signed char)p[3]; p += 4; }
        else if (mod == 2) { rsp = c->gpr[rm] + (s64)*(const s32 *)(p + 3); p += 7; }
        else return 0;
    }
    const u8 *q = p;                                   /* check the pops reach a return before applying them */
    for (;;) {
        if (q[0] >= 0x58 && q[0] <= 0x5F) q += 1;
        else if (q[0] == 0x41 && q[1] >= 0x58 && q[1] <= 0x5F) q += 2;
        else break;
    }
    int out = q[0] == 0xC3 || (q[0] == 0xF3 && q[1] == 0xC3);
    if (q[0] == 0xE9) { u64 t = (u64)(q + 5) + (s64)*(const s32 *)(q + 1); out = t < fbegin || t >= fend; }
    if (q[0] == 0xEB) { u64 t = (u64)(q + 2) + (s64)(signed char)q[1]; out = t < fbegin || t >= fend; }
    if (q[0] == 0xFF && q[1] == 0x25) out = 1;                                 /* jmp [rip + disp] */
    if (q[0] == 0x48 && q[1] == 0xFF && q[2] == 0x25) out = 1;
    if (!out) return 0;
    for (;;) {
        if (p[0] >= 0x58 && p[0] <= 0x5F) { c->gpr[p[0] - 0x58] = read64(rsp); rsp += 8; p += 1; }
        else if (p[0] == 0x41 && p[1] >= 0x58 && p[1] <= 0x5F) { c->gpr[8 + p[1] - 0x58] = read64(rsp); rsp += 8; p += 2; }
        else break;
    }
    c->rip = read64(rsp);
    c->gpr[RSP] = rsp + 8;
    return 1;
}

/* RtlVirtualUnwind's work: c from the frame of the function at pc to its caller's. Returns the frame's handler for
 * `type` (UNW_FLAG_EHANDLER or UNW_FLAG_UHANDLER) when pc is past the prolog and not in an epilogue, its data in
 * *data; *frame is the establisher frame (RSP after the prolog, or the frame register less its offset). */
static u64 virtual_unwind(u32 type, u64 base, u64 pc, const struct runtime_function *f, struct w2m_context *c,
                          u64 *data, u64 *frame) {
    while (f->unwind & 1) f = (const struct runtime_function *)(base + (f->unwind & ~1u));   /* an indirect entry */
    const u8 *info = (const u8 *)(base + f->unwind);
    u32 flags = info[0] >> 3, prolog = info[1], frame_reg = info[3] & 15, frame_off = info[3] >> 4;
    u64 off = pc - (base + f->begin);
    *data = 0;
    *frame = c->gpr[RSP];
    if (frame_reg && off >= prolog) *frame = c->gpr[frame_reg] - 16 * frame_off;
    if (off >= prolog && unwind_epilogue(pc, c, frame_reg, base + f->begin, base + f->end)) return 0;
    u64 handler = 0;
    if ((flags & type) && off >= prolog) {
        const unsigned short *codes = (const unsigned short *)(info + 4);
        const u32 *h = (const u32 *)&codes[(info[2] + 1) & ~1u];
        handler = base + h[0];
        *data = (u64)&h[1];
    }
    int machine_frame = 0, primary = 1;
    for (;;) {
        u32 version = info[0] & 7, count = info[2];
        flags = info[0] >> 3;
        frame_reg = info[3] & 15;
        frame_off = info[3] >> 4;
        const unsigned short *codes = (const unsigned short *)(info + 4);
        if (version != 1 && version != 2) w2m_unimplemented(W2M_API_RtlUnwindEx, version);
        for (u32 i = 0; i < count; i++) {
            u32 at = codes[i] & 0xFF, op = codes[i] >> 8 & 15, opinfo = codes[i] >> 12;
            u32 slots = op == 1 ? (opinfo ? 3 : 2) : op == 4 || op == 8 ? 2 : op == 5 || op == 9 ? 3 : 1;
            int applies = !primary || off >= prolog || at <= off;   /* in the prolog: only what has run */
            if (version == 2 && op == 6) applies = 0;               /* UWOP_EPILOG: where epilogues are */
            if (applies) {
                u64 rsp = c->gpr[RSP];
                switch (op) {
                case 0: *gpr(c, opinfo) = read64(rsp); c->gpr[RSP] = rsp + 8; break;              /* PUSH_NONVOL */
                case 1: c->gpr[RSP] = rsp + (opinfo ? *(const u32 *)&codes[i + 1] : 8u * codes[i + 1]); break;
                case 2: c->gpr[RSP] = rsp + 8 * opinfo + 8; break;                                 /* ALLOC_SMALL */
                case 3: c->gpr[RSP] = c->gpr[frame_reg] - 16 * frame_off; *frame = c->gpr[RSP]; break;
                case 4: *gpr(c, opinfo) = read64(*frame + 8u * codes[i + 1]); break;               /* SAVE_NONVOL */
                case 5: *gpr(c, opinfo) = read64(*frame + *(const u32 *)&codes[i + 1]); break;
                case 8: read_xmm(c, opinfo, *frame + 16u * codes[i + 1]); break;                   /* SAVE_XMM128 */
                case 9: read_xmm(c, opinfo, *frame + *(const u32 *)&codes[i + 1]); break;
                case 10:                                                                           /* PUSH_MACHFRAME */
                    c->rip = read64(rsp + 8 * opinfo);
                    c->gpr[RSP] = read64(rsp + 8 * opinfo + 24);
                    machine_frame = 1;
                    break;
                default: w2m_unimplemented(W2M_API_RtlUnwindEx, 0x100 | op);                      /* not in the rows */
                }
            }
            i += slots - 1;
        }
        if (!(flags & UNW_FLAG_CHAININFO)) break;
        f = (const struct runtime_function *)&codes[(count + 1) & ~1u];
        info = (const u8 *)(base + f->unwind);
        primary = 0;
    }
    if (!machine_frame) {
        c->rip = read64(c->gpr[RSP]);
        c->gpr[RSP] += 8;
    }
    return handler;
}

/* ---- calling the guest ----------------------------------------------------------------------------------- */

/* Call x86 function fn(a0, a1, a2, a3) with the frame's stack: its answer comes to W2mExceptionResume. */
static void call_guest(struct frame *f, u64 fn, u64 a0, u64 a1, u64 a2, u64 a3) {
    struct w2m_state *st = w2m_state();
    st->gpr[RAX] = fn;
    st->gpr[RCX] = a0;
    st->gpr[RDX] = a1;
    st->gpr[R8] = a2;
    st->gpr[R9] = a3;
    st->gpr[RSP] = (u64)f - 0x20;                      /* home space; `call` leaves RSP 8 mod 16 at entry */
    st->redirect_rip = stubs() + STUB_CALL;
}

static void resume_context(struct w2m_context *c) {
    struct w2m_state *st = w2m_state();
    st->gpr[RCX] = (u64)c;
    st->redirect_rip = stubs() + STUB_RESTORE;
}

static void end_process(u32 code) {
    w2m_host(W2M_HOST_EXIT, code, 0, 0, 0, 0);
    for (;;) {}
}

/* A frame for a new dispatch or unwind below stack pointer rsp, zeroed. */
static struct frame *place(u64 rsp) {
    struct frame *f = (struct frame *)((rsp - FRAME_GAP - sizeof(struct frame)) & ~15ull);
    u8 *b = (u8 *)f;
    for (u64 i = 0; i < sizeof *f; i++) b[i] = 0;
    f->magic = FRAME_MAGIC;
    return f;
}

static int in_stack(u64 a) {
    struct w2m_teb *t = w2m_teb();
    return a >= t->stack_limit && a < t->stack_base;
}

/* The dispatch a walk reached through a handler call's return point, or 0. */
static struct frame *outer_frame(const struct w2m_context *c) {
    if (c->rip != stubs() + STUB_CALL_RETURN) return 0;
    struct frame *o = (struct frame *)(c->gpr[RSP] + 0x20);
    if (o->magic != FRAME_MAGIC) w2m_unimplemented(W2M_API_RtlUnwindEx, c->gpr[RSP]);
    return o;
}

/* The caller's context at a service entry: the bridge saved the GPRs; RSP points at the return address. */
static void capture(struct w2m_context *c, u64 rsp, u64 rip) {
    const struct w2m_state *st = w2m_state();
    c->context_flags = CONTEXT_CAPTURED;
    c->seg_cs = 0x33;
    c->seg_ds = c->seg_es = c->seg_gs = c->seg_ss = 0x2B;
    c->seg_fs = 0x53;
    c->eflags = 0x202;                                 /* across a call the flags are the callee's to change */
    for (int i = 0; i < 16; i++) c->gpr[i] = st->gpr[i];
    c->gpr[RSP] = rsp;
    c->rip = rip;
    c->mx_csr = c->flt_save.mx_csr = w2m_mxcsr_now();
    c->flt_save.control_word = 0x27F;
    c->flt_save.mx_csr_mask = 0xFFFF;
    w2m_host(W2M_HOST_EXCEPTION, W2M_EXCEPTION_CAPTURE_XMM, (u64)c->flt_save.xmm_registers, 0, 0, 0);
}

/* ---- dispatch ---------------------------------------------------------------------------------------------- */

static void dispatch_begin(struct frame *f);
static void next_frame(struct frame *f);

/* The exception ends the process: its record and context go to the exception area for the host. */
static void end_unhandled(struct frame *f) {
    struct w2m_exception_area *e = (struct w2m_exception_area *)W2M_EXCEPTION_VA;
    e->record = f->rec;
    e->context = f->ctx;
    e->magic = W2M_EXCEPTION_MAGIC;
    end_process(f->rec.code);
}

static void unhandled(struct frame *f) {
    u64 filter = w2m_unhandled_exception_filter();
    if (filter && f->step != STEP_TOP_FILTER) {
        f->step = STEP_TOP_FILTER;
        call_guest(f, filter, (u64)f->pointers, 0, 0, 0);
        return;
    }
    end_unhandled(f);
}

static void next_vectored(struct frame *f) {
    u64 h = w2m_vectored_handler(f->next_vectored++);
    if (h) {
        f->step = STEP_VECTORED;
        call_guest(f, h, (u64)f->pointers, 0, 0, 0);
        return;
    }
    f->step = STEP_FRAME;
    f->walk = f->ctx;
    next_frame(f);
}

static void dispatch_begin(struct frame *f) {
    f->kind = KIND_DISPATCH;
    f->record = (u64)&f->rec;
    f->pointers[0] = (u64)&f->rec;
    f->pointers[1] = (u64)&f->ctx;
    next_vectored(f);
}

/* Walk out from f->walk to the next frame with an exception handler and call it; none: unhandled. */
static void next_frame(struct frame *f) {
    while (f->frames++ < MAX_FRAMES) {
        u64 pc = f->walk.rip, base = 0, data = 0, establisher = 0;
        if (outer_frame(&f->walk)) w2m_unimplemented(W2M_API_RaiseException, pc);   /* nested: not in the rows */
        const struct runtime_function *fe = lookup(pc, &base);
        if (!fe) {                                     /* a leaf: its return address is at RSP */
            f->walk.rip = read64(f->walk.gpr[RSP]);
            f->walk.gpr[RSP] += 8;
            if (!known_code(f->walk.rip) || !in_stack(f->walk.gpr[RSP])) break;
            continue;
        }
        u64 handler = virtual_unwind(UNW_FLAG_EHANDLER, base, pc, fe, &f->walk, &data, &establisher);
        if (!in_stack(establisher) || (establisher & 7)) {
            f->rec.flags |= EXCEPTION_STACK_INVALID;
            break;
        }
        if (handler) {
            f->dc = (struct dispatcher_context){pc, base, (u64)fe, establisher, 0, (u64)&f->walk, handler, data, 0, 0, 0};
            f->establisher = establisher;
            call_guest(f, handler, f->record, establisher, (u64)&f->ctx, (u64)&f->dc);
            return;
        }
        if (!in_stack(f->walk.gpr[RSP]) || !known_code(f->walk.rip)) break;
    }
    unhandled(f);
}

/* CONTINUE_EXECUTION on a noncontinuable exception: STATUS_NONCONTINUABLE_EXCEPTION raised from the same context,
 * noncontinuable and software-originated, no parameters and no chained record (rows except.noncontinuable). */
static void raise_noncontinuable(struct frame *f) {
    struct frame *g = place((u64)f);
    g->rec.code = STATUS_NONCONTINUABLE_EXCEPTION;
    g->rec.flags = EXCEPTION_NONCONTINUABLE | EXCEPTION_SOFTWARE_ORIGINATE;
    g->rec.address = f->rec.address;
    g->ctx = f->ctx;
    dispatch_begin(g);
}

static void continue_execution(struct frame *f) {
    if (f->rec.flags & EXCEPTION_NONCONTINUABLE) raise_noncontinuable(f);
    else resume_context(&f->ctx);
}

/* ---- unwind ------------------------------------------------------------------------------------------------ */

static void unwind_finish(struct frame *f) {
    struct w2m_exception_record *rec = (struct w2m_exception_record *)f->record;
    if (rec->code == STATUS_UNWIND_CONSOLIDATE) w2m_unimplemented(W2M_API_RtlUnwindEx, rec->code);   /* C++ */
    f->ctx.gpr[RAX] = f->return_value;
    f->ctx.rip = f->target_ip;
    resume_context(&f->ctx);
}

/* Walk out from f->ctx calling each frame's termination handler, up to the target frame. */
static void next_unwind(struct frame *f) {
    struct w2m_exception_record *rec = (struct w2m_exception_record *)f->record;
    while (f->frames++ < MAX_FRAMES) {
        struct frame *o = outer_frame(&f->ctx);
        if (o) {                                       /* the dispatch whose handler called us: on from its raise */
            if (o->kind != KIND_DISPATCH) w2m_unimplemented(W2M_API_RtlUnwindEx, EXCEPTION_COLLIDED_UNWIND);
            f->ctx = o->ctx;
            continue;
        }
        u64 pc = f->ctx.rip, base = 0, data = 0, establisher = 0;
        const struct runtime_function *fe = lookup(pc, &base);
        if (!fe) {
            f->ctx.rip = read64(f->ctx.gpr[RSP]);
            f->ctx.gpr[RSP] += 8;
            if (!in_stack(f->ctx.gpr[RSP])) break;
            continue;
        }
        f->walk = f->ctx;
        u64 handler = virtual_unwind(UNW_FLAG_UHANDLER, base, pc, fe, &f->walk, &data, &establisher);
        if (!in_stack(establisher) || (establisher & 7) || establisher > f->target_frame) break;
        f->establisher = establisher;
        if (handler) {
            if (establisher == f->target_frame) rec->flags |= EXCEPTION_TARGET_UNWIND;
            f->dc = (struct dispatcher_context){pc, base, (u64)fe, establisher, f->target_ip, (u64)&f->ctx, handler,
                                                data, f->history, 0, 0};
            call_guest(f, handler, f->record, establisher, (u64)&f->ctx, (u64)&f->dc);
            return;
        }
        if (establisher == f->target_frame) {
            unwind_finish(f);
            return;
        }
        f->ctx = f->walk;
    }
    w2m_unimplemented(W2M_API_RtlUnwindEx, f->target_frame);     /* the target is not on the stack */
}

/* ---- services ---------------------------------------------------------------------------------------------- */

/* RaiseException(dwExceptionCode, dwExceptionFlags, nNumberOfArguments, lpArguments): dispatches; returns only
 * when a handler continues execution. Its context is that of its own return point (a `ret` stub): RSP at the
 * return address, as Windows's points inside KERNELBASE!RaiseException. */
W2M_VOID u64 k32_RaiseException(u64 code, u64 flags, u64 count, u64 args, u64 rsp) {
    if ((u32)flags & ~EXCEPTION_NONCONTINUABLE) w2m_unimplemented(W2M_API_RaiseException, flags);   /* not in the rows */
    if ((u32)count > EXCEPTION_MAXIMUM_PARAMETERS || ((u32)count && !args))
        w2m_unimplemented(W2M_API_RaiseException, count);
    w2m_logged(W2M_API_RaiseException, (u32)code, (u32)count);
    struct frame *f = place(rsp);
    f->rec.code = (u32)code;
    f->rec.flags = ((u32)flags & EXCEPTION_NONCONTINUABLE) | EXCEPTION_SOFTWARE_ORIGINATE;
    f->rec.address = stubs() + STUB_RETURN;
    f->rec.number_parameters = (u32)count;
    for (u32 i = 0; i < (u32)count; i++) f->rec.information[i] = ((const u64 *)args)[i];
    capture(&f->ctx, rsp, stubs() + STUB_RETURN);
    dispatch_begin(f);
    return 0;
}

/* A fault (runtime.c): its record and context, built at the faulting instruction, dispatched from a frame
 * below its RSP. 0: the stack cannot hold the frame (Windows ends such a process). */
int w2m_dispatch_fault(const struct w2m_exception_record *rec, const struct w2m_context *ctx) {
    u64 rsp = ctx->gpr[RSP];
    if (!in_stack(rsp) || !in_stack(rsp - FRAME_GAP - sizeof(struct frame) - 0x40)) return 0;
    struct frame *f = place(rsp);
    f->rec = *rec;
    f->ctx = *ctx;
    dispatch_begin(f);
    return 1;
}

/* Where a handler called through the stub returns, its answer in RCX: the dispatch or unwind goes on. */
W2M_VOID u64 k32_W2mExceptionResume(u64 answer, u64 a1, u64 a2, u64 a3, u64 rsp) {
    (void)a1, (void)a2, (void)a3;
    struct frame *f = (struct frame *)(rsp + 0x20);
    if (f->magic != FRAME_MAGIC) w2m_unimplemented(W2M_API_W2mExceptionResume, rsp);
    struct w2m_exception_record *rec = (struct w2m_exception_record *)f->record;
    w2m_logged(W2M_API_W2mExceptionResume, f->step, (u32)answer);
    if (f->kind == KIND_UNWIND) {
        rec->flags &= ~(EXCEPTION_TARGET_UNWIND | EXCEPTION_COLLIDED_UNWIND);
        if ((u32)answer != DISPOSITION_CONTINUE_SEARCH) w2m_unimplemented(W2M_API_RtlUnwindEx, (u32)answer);
        if (f->establisher == f->target_frame) unwind_finish(f);
        else {
            f->ctx = f->walk;
            next_unwind(f);
        }
        return 0;
    }
    if (f->step == STEP_VECTORED) {
        if ((s32)answer == FILTER_CONTINUE_EXECUTION) continue_execution(f);
        else next_vectored(f);
    } else if (f->step == STEP_FRAME) {
        if ((u32)answer == DISPOSITION_CONTINUE_EXECUTION) continue_execution(f);
        else if ((u32)answer == DISPOSITION_CONTINUE_SEARCH) next_frame(f);
        else w2m_unimplemented(W2M_API_RaiseException, 0x100 | (u32)answer);  /* nested, collided: not in the rows */
    } else {
        if ((s32)answer == FILTER_CONTINUE_EXECUTION) continue_execution(f);
        else end_unhandled(f);
    }
    return 0;
}

/* RtlUnwindEx(TargetFrame, TargetIp, ExceptionRecord, ReturnValue, ContextRecord, HistoryTable): never returns;
 * the target frame resumes at TargetIp with RAX = ReturnValue. The walk keeps its own context; ContextRecord is
 * the caller's scratch, as the C handler passes it. */
W2M_VOID u64 ntdll_RtlUnwindEx(u64 target_frame, u64 target_ip, u64 record, u64 value, u64 rsp) {
    u64 history = *(const u64 *)(rsp + 0x30);
    if (!target_frame) w2m_unimplemented(W2M_API_RtlUnwindEx, 0);              /* an exit unwind: not in the rows */
    w2m_logged(W2M_API_RtlUnwindEx, target_frame, target_ip);
    struct frame *f = place(rsp);
    f->kind = KIND_UNWIND;
    capture(&f->ctx, rsp, stubs() + STUB_RETURN);
    if (!record) {
        f->rec.code = STATUS_UNWIND;
        f->rec.address = f->ctx.rip;
        record = (u64)&f->rec;
    }
    ((struct w2m_exception_record *)record)->flags |= EXCEPTION_UNWINDING;
    f->record = record;
    f->target_frame = target_frame;
    f->target_ip = target_ip;
    f->return_value = value;
    f->history = history;
    next_unwind(f);
    return 0;
}

/* RtlCaptureContext(ContextRecord): the caller's registers as they are after the call returns. */
W2M_VOID u64 ntdll_RtlCaptureContext(u64 context, u64 a1, u64 a2, u64 a3, u64 rsp) {
    (void)a1, (void)a2, (void)a3;
    struct w2m_context *c = (struct w2m_context *)context;
    u8 *b = (u8 *)c;
    for (u64 i = 0; i < sizeof *c; i++) b[i] = 0;
    capture(c, rsp + 8, read64(rsp));
    return w2m_logged(W2M_API_RtlCaptureContext, context, 0);
}

/* RtlVirtualUnwind(HandlerType, ImageBase, ControlPc, FunctionEntry, ContextRecord, HandlerData,
 * EstablisherFrame, ContextPointers) */
u64 ntdll_RtlVirtualUnwind(u64 type, u64 base, u64 pc, u64 entry, u64 rsp) {
    u64 context = read64(rsp + 0x28), data = read64(rsp + 0x30), frame = read64(rsp + 0x38);
    if (read64(rsp + 0x40)) w2m_unimplemented(W2M_API_RtlVirtualUnwind, 1);   /* context pointers: not in the rows */
    u64 d = 0, e = 0;
    u64 h = virtual_unwind((u32)type, base, pc, (const struct runtime_function *)entry,
                           (struct w2m_context *)context, &d, &e);
    *(u64 *)data = d;
    *(u64 *)frame = e;
    return w2m_logged(W2M_API_RtlVirtualUnwind, pc, h);
}

/* RtlPcToFileHeader(PcValue, BaseOfImage): the image holding the address, or NULL in both. */
u64 ntdll_RtlPcToFileHeader(u64 pc, u64 out) {
    u64 b = w2m_host(W2M_HOST_MODULE_AT, pc, 0, 0, 0, 0);
    *(u64 *)out = b;
    return w2m_logged(W2M_API_RtlPcToFileHeader, pc, b);
}
