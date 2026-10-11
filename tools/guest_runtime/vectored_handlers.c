// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* vectored_handlers.c — the vectored exception handler list (Sprint 049, family "vectored handlers").
 *
 * As contracts/windows.exception.vectored_handlers records Windows answering (Server 2025 and 2022):
 *   - AddVectoredExceptionHandler answers a new handle each call, also for a handler already in the list; First
 *     nonzero puts it at the front, zero at the back; dispatch calls them front to back (rows dispatch.*)
 *   - RemoveVectoredExceptionHandler answers 1 for a handle in the list and removes it, 0 for any other value
 *   - neither touches the last error
 * The list is kept in order here for the dispatcher; dispatch itself (RaiseException, faults) is not built yet.
 */
#include "runtime.h"

#define HANDLERS 64

static struct vectored { u64 handler; int used; } slots[HANDLERS];
static int order[HANDLERS], count;                  /* slot indices, front to back */

/* AddVectoredExceptionHandler(First, Handler): a handle, the slot's address. */
u64 k32_AddVectoredExceptionHandler(u64 first, u64 handler) {
    if (!handler) w2m_unimplemented(W2M_API_AddVectoredExceptionHandler, 0);           /* not in the rows */
    for (int i = 0; i < HANDLERS; i++)
        if (!slots[i].used) {
            slots[i] = (struct vectored){handler, 1};
            if ((u32)first) {
                for (int k = count; k > 0; k--) order[k] = order[k - 1];
                order[0] = i;
            } else {
                order[count] = i;
            }
            count++;
            return w2m_logged(W2M_API_AddVectoredExceptionHandler, handler, (u64)&slots[i]);
        }
    w2m_unimplemented(W2M_API_AddVectoredExceptionHandler, HANDLERS);
    return 0;
}

/* RemoveVectoredExceptionHandler(Handle): 1 when it was in the list. */
u64 k32_RemoveVectoredExceptionHandler(u64 handle) {
    for (int k = 0; k < count; k++)
        if ((u64)&slots[order[k]] == handle) {
            slots[order[k]].used = 0;
            for (int j = k; j + 1 < count; j++) order[j] = order[j + 1];
            count--;
            return w2m_logged(W2M_API_RemoveVectoredExceptionHandler, handle, 1);
        }
    return w2m_logged(W2M_API_RemoveVectoredExceptionHandler, handle, 0);
}
