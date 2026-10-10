/* virtual_memory.c — VirtualAlloc, VirtualFree, VirtualProtect, VirtualQuery (Sprint 046, family
 * "virtual memory").
 *
 * The host keeps the address space (src/w2m/virtual_memory.py): each call is one host request, and a
 * failure comes back with its error code in the transport's second word, which becomes the last error.
 * contracts/windows.memory.virtual records what Windows answers; the host module's docstring says what
 * w2m does where the rows are silent. Not in the rows, so the run stops: a NULL lpflOldProtect, a
 * VirtualQuery buffer shorter than MEMORY_BASIC_INFORMATION.
 */
#include "runtime.h"

#define MBI_SIZE 48u

static void set_error(u32 code) { w2m_teb()->last_error_value = code; }

u64 k32_VirtualAlloc(u64 address, u64 size, u64 type, u64 protect) {
    u64 err = 0;
    u64 p = w2m_host(W2M_HOST_VIRTUAL_ALLOC, address, size, (u32)type, (u32)protect, &err);
    if (!p) set_error((u32)err);
    return w2m_logged(W2M_API_VirtualAlloc, size, p);
}

u64 k32_VirtualFree(u64 address, u64 size, u64 type) {
    u64 err = 0;
    u64 ok = w2m_host(W2M_HOST_VIRTUAL_FREE, address, size, (u32)type, 0, &err);
    if (!ok) set_error((u32)err);
    return w2m_logged(W2M_API_VirtualFree, address, ok);
}

u64 k32_VirtualProtect(u64 address, u64 size, u64 protect, u64 old) {
    if (!old) w2m_unimplemented(W2M_API_VirtualProtect, 0);
    u64 second = 0;
    u64 ok = w2m_host(W2M_HOST_VIRTUAL_PROTECT, address, size, (u32)protect, 0, &second);
    *(u32 *)old = (u32)second;                     /* written on failure too (row protect.reserved_page) */
    if (!ok) set_error((u32)(second >> 32));
    return w2m_logged(W2M_API_VirtualProtect, address, ok);
}

u64 k32_VirtualQuery(u64 address, u64 buffer, u64 length) {
    if (length < MBI_SIZE) w2m_unimplemented(W2M_API_VirtualQuery, length);
    u64 err = 0;
    u64 n = w2m_host(W2M_HOST_VIRTUAL_QUERY, address, buffer, 0, 0, &err);
    if (!n) set_error((u32)err);
    return w2m_logged(W2M_API_VirtualQuery, address, n);
}
