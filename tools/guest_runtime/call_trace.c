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
 * the first W2M_TRACE_CALLS calls of each function under each label, a call gets a record: the sequence number taken on entry, the
 * thread id, the four register and four stack arguments, the return value and the last error after the call. A
 * call the service leaves unreturned (W2M_SCHEDULE_RETRY, a wait that cannot complete yet) gives its record back:
 * the thread re-enters the call, and that entry is the one recorded. The host reads the records after the run.
 */
#include "runtime.h"

#define W2M_TRACE_CALLS 32          /* call_trace.h TRACE_CALLS */
#define W2M_TRACE_THUNKS 4096
#define W2M_TRACE_LABELS 3          /* the two images, then GetProcAddress */
#define W2M_TRACE_DYNAMIC 2
#define W2M_TRACE_RECORDS_OFF 0xD100

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
    u32 taken[W2M_TRACE_THUNKS][W2M_TRACE_LABELS];   /* calls recorded per thunk and label */
    u8 owner[W2M_TRACE_THUNKS];      /* 1 + the traced image whose import slot holds the thunk; 0: none */
};
_Static_assert(__builtin_offsetof(struct w2m_trace, taken) == 0x80, "trace header (vm_backend.py)");
_Static_assert(__builtin_offsetof(struct w2m_trace, owner) == 0xC080, "trace owners (vm_backend.py TRACE_OWNER_OFF)");
_Static_assert(sizeof(struct w2m_trace) <= W2M_TRACE_RECORDS_OFF, "trace records follow the counters");

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
    int image = (int)t->owner[thunk] - 1;
    if (image < 0) {
        u64 caller = *(const u64 *)rsp;
        for (int i = 0; i < 2; i++)
            if (t->range[i][0] <= caller && caller < t->range[i][1])
                image = W2M_TRACE_DYNAMIC;
    }
    if (image < 0)
        return call(a0, a1, a2, a3, rsp);
    if (t->count >= t->capacity) {
        t->dropped++;
        return call(a0, a1, a2, a3, rsp);
    }
    if (t->taken[thunk][image] >= W2M_TRACE_CALLS)
        return call(a0, a1, a2, a3, rsp);
    struct w2m_trace_record *r = (struct w2m_trace_record *)(W2M_TRACE_VA + W2M_TRACE_RECORDS_OFF) + t->count++;
    t->taken[thunk][image]++;
    const u64 *stack = (const u64 *)rsp;
    r->ready = 0;
    r->seq = t->seq++;
    r->tid = w2m_teb()->client_thread;
    r->image = (u64)image;
    r->thunk = thunk;
    r->a[0] = a0, r->a[1] = a1, r->a[2] = a2, r->a[3] = a3;
    for (int i = 0; i < 4; i++)
        r->a[4 + i] = stack[5 + i];  /* above the return address and the 32-byte home area */
    u64 ret = call(a0, a1, a2, a3, rsp);
    if (w2m_state()->schedule & W2M_SCHEDULE_RETRY) {
        t->taken[thunk][image]--;    /* the slot stays, not ready: the re-entry is recorded */
        return ret;
    }
    r->ret = ret;
    r->error = w2m_get_last_error();
    r->ready = 1;
    return ret;
}
