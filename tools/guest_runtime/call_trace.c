// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* call_trace.c — the game's calls as w2m answers them, recorded as tools/jit_capture/call_trace.h records them on
 * Windows (Sprint 060), so the two can be compared call by call (scripts/call_compare.py).
 *
 * When the host turns the trace on (VM call_trace), every import bridge with a service calls w2m_traced with the
 * service's address and the thunk's index instead of the service. The capture's labels follow its hooks: each
 * traced image's import slots (UnityPlayer.dll, mono-2.0-bdwgc.dll: the images call_trace.h hooks) hold thunks of
 * their own (`owner`: 1 + the image), and a call through one is that image's; a call through any other thunk, as
 * GetProcAddress hands out, from code in a traced image is the "GetProcAddress" hook's (W2M_TRACE_DYNAMIC). For
 * the first W2M_TRACE_CALLS calls of each function under each label on each thread, a call gets a record: the sequence number taken on entry, the
 * thread id, the four register and four stack arguments, the return value and the last error after the call. A
 * call the service leaves unreturned (W2M_SCHEDULE_RETRY, a wait that cannot complete yet) gives its record back:
 * the thread re-enters the call, and that entry is the one recorded. A call whose service runs guest code first
 * (guest_calls.c: CreateWindowExW's window procedure, EnumDisplayMonitors' callback) returns to its caller later,
 * through w2m_return_to; its record waits in `pending`, keyed by the call's stack pointer, and is completed there
 * with the value the caller gets (Sprint 061: the first record held the service's early 0). The host reads the
 * records after the run.
 */
#include "runtime.h"

typedef unsigned short u16;

#define W2M_TRACE_CALLS 32          /* call_trace.h TRACE_CALLS */
#define W2M_TRACE_THUNKS 16384      /* Sprint 064: 4096 left a frontier run's later thunks untraced */
#define W2M_TRACE_LABELS 3          /* the two images, then GetProcAddress */
#define W2M_TRACE_DYNAMIC 2
#define W2M_TRACE_PENDING 8
#define W2M_TRACE_THREADS 64
#define W2M_TRACE_STRINGS 2048
#define W2M_TRACE_STRING_UNITS 60
#define W2M_TRACE_RECORDS_OFF 0x34D000

struct w2m_trace_record {
    u64 ready, seq, tid, image, thunk;
    u64 a[8];
    u64 ret, error;
};
_Static_assert(sizeof(struct w2m_trace_record) == 120, "trace record (vm_backend.py TRACE_RECORD)");

struct w2m_trace {
    u64 on, seq, count, capacity, dropped;
    u64 range[2][2];                 /* [image] = (base, end): UnityPlayer.dll, mono-2.0-bdwgc.dll; 0 until loaded */
    u64 pad[7];
    u8 owner[W2M_TRACE_THUNKS];      /* 1 + the traced image whose import slot holds the thunk; 0: none; 0xFF: never */
    u64 pending[W2M_TRACE_PENDING][2];   /* (the call's stack pointer, 1 + its record) for calls not yet returned */
    u64 thread[W2M_TRACE_THREADS];   /* the thread ids `taken` is kept for, in order of their first traced call */
    u8 wide[W2M_TRACE_THUNKS];       /* bit i: argument i is a UTF-16 string the record keeps (call_trace.h STRING_ARGS) */
    /* calls recorded per thread, thunk and label. The capture keeps a function's first 32 calls over all threads, so
     * each Windows thread's recorded calls are a prefix of its own; keeping 32 a thread here holds that prefix for
     * every thread whatever the schedule (Sprint 061: a count over all threads kept other threads' calls than
     * Windows kept). */
    u8 taken[W2M_TRACE_THREADS][W2M_TRACE_THUNKS][W2M_TRACE_LABELS];
    u64 strings;                     /* the strings kept, each the text of one string argument of a recorded call */
    u64 pad2;
    struct { u64 seq, arg; u16 text[W2M_TRACE_STRING_UNITS]; } string[W2M_TRACE_STRINGS];
};
_Static_assert(__builtin_offsetof(struct w2m_trace, owner) == 0x80, "trace owners (vm_backend.py TRACE_OWNER_OFF)");
_Static_assert(__builtin_offsetof(struct w2m_trace, wide) == 0x4300, "trace string masks (vm_backend.py TRACE_WIDE_OFF)");
_Static_assert(__builtin_offsetof(struct w2m_trace, strings) == 0x308300, "trace strings (vm_backend.py TRACE_STRINGS_OFF)");
_Static_assert(sizeof(struct w2m_trace) <= W2M_TRACE_RECORDS_OFF, "trace records follow the counters");

static void complete(struct w2m_trace_record *r, u64 ret) {
    r->ret = ret;
    r->error = w2m_get_last_error();
    r->ready = 1;
}

/* guest_calls.c's w2m_return_to: the caller at `rsp` gets `value` now. */
void w2m_trace_return(u64 rsp, u64 value) {
    struct w2m_trace *t = (struct w2m_trace *)W2M_TRACE_VA;
    for (int i = 0; i < W2M_TRACE_PENDING; i++)
        if (t->pending[i][1] && t->pending[i][0] == rsp) {
            complete((struct w2m_trace_record *)(W2M_TRACE_VA + W2M_TRACE_RECORDS_OFF) + (t->pending[i][1] - 1), value);
            t->pending[i][1] = 0;
            return;
        }
}

typedef u64 (*w2m_service)(u64, u64, u64, u64, u64);

u64 w2m_traced(u64 a0, u64 a1, u64 a2, u64 a3, u64 rsp, u64 service, u64 thunk) {
    struct w2m_trace *t = (struct w2m_trace *)W2M_TRACE_VA;
    w2m_service call = (w2m_service)service;
    if (!t->on)
        return call(a0, a1, a2, a3, rsp);
    if (thunk >= W2M_TRACE_THUNKS) {
        t->dropped++;
        return call(a0, a1, a2, a3, rsp);
    }
    if (t->owner[thunk] == 0xFF)     /* resolved while the trace was closed: the real function on Windows */
        return call(a0, a1, a2, a3, rsp);
    int image = (int)t->owner[thunk] - 1;
    if (image < 0) {
        u64 caller = *(const u64 *)rsp;
        for (int i = 0; i < 2; i++)
            if (t->range[i][0] <= caller && caller < t->range[i][1])
                image = W2M_TRACE_DYNAMIC;
    }
    if (image < 0)
        return call(a0, a1, a2, a3, rsp);
    u64 tid = w2m_teb()->client_thread;
    int slot = 0;
    while (slot < W2M_TRACE_THREADS && t->thread[slot] && t->thread[slot] != tid) slot++;
    if (t->count >= t->capacity || slot == W2M_TRACE_THREADS) {
        t->dropped++;
        return call(a0, a1, a2, a3, rsp);
    }
    t->thread[slot] = tid;
    u8 *taken = &t->taken[slot][thunk][image];
    if (*taken >= W2M_TRACE_CALLS)
        return call(a0, a1, a2, a3, rsp);
    struct w2m_trace_record *r = (struct w2m_trace_record *)(W2M_TRACE_VA + W2M_TRACE_RECORDS_OFF) + t->count++;
    (*taken)++;
    const u64 *stack = (const u64 *)rsp;
    r->ready = 0;
    r->seq = t->seq++;
    r->tid = tid;
    r->image = (u64)image;
    r->thunk = thunk;
    r->a[0] = a0, r->a[1] = a1, r->a[2] = a2, r->a[3] = a3;
    for (int i = 0; i < 4; i++)
        r->a[4 + i] = stack[5 + i];  /* above the return address and the 32-byte home area */
    for (int i = 0; i < 8; i++)      /* a string argument's text, as the capture keeps it (Sprint 064: thread names) */
        if (t->wide[thunk] >> i & 1 && r->a[i] >= 0x10000 && t->strings < W2M_TRACE_STRINGS) {
            const u16 *w = (const u16 *)r->a[i];
            u64 k = t->strings++, n = 0;
            t->string[k].seq = r->seq;
            t->string[k].arg = (u64)i;
            while (n < W2M_TRACE_STRING_UNITS - 1 && w[n]) { t->string[k].text[n] = w[n]; n++; }
            t->string[k].text[n] = 0;
        }
    u64 ret = call(a0, a1, a2, a3, rsp);
    if (w2m_state()->schedule & W2M_SCHEDULE_RETRY) {
        (*taken)--;                  /* the re-entry is recorded */
        if (r == (struct w2m_trace_record *)(W2M_TRACE_VA + W2M_TRACE_RECORDS_OFF) + t->count - 1)
            t->count--;              /* the slot back (Sprint 064: a long wait's retries filled the area) */
        return ret;
    }
    struct w2m_state *st = w2m_state();
    if (st->redirect_rip) {
        if (st->redirect_rip == stack[0] && st->gpr[4] == rsp + 8) {     /* returned at once through w2m_return_to */
            complete(r, st->gpr[0]);
            return ret;
        }
        for (int i = 0; i < W2M_TRACE_PENDING; i++)
            if (!t->pending[i][1]) {
                t->pending[i][0] = rsp;
                t->pending[i][1] = (u64)(r - (struct w2m_trace_record *)(W2M_TRACE_VA + W2M_TRACE_RECORDS_OFF)) + 1;
                return ret;
            }
        t->dropped++;                /* more calls in guest code at once than `pending` holds: the record stays unready */
        return ret;
    }
    complete(r, ret);
    return ret;
}
