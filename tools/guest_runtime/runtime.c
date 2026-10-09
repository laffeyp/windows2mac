/* runtime.c — kernel32 services run inside the managed VM (Sprint 035).
 *
 * Each k32_* function is entered from an AArch64 bridge stub that src/w2m/vm_backend.py generates
 * per import: the stub saves the guest's 16 GPRs in the state block, passes the Microsoft x64
 * arguments (RCX, RDX, R8, R9) in x0-x3 and the guest RSP in x4 (a fifth argument sits at
 * [RSP + 0x28]: return address, then 32 bytes of shadow space; Microsoft Learn, x64 calling
 * convention), switches to the runtime stack, and calls here. A function marked void in the
 * bridge table leaves RAX unchanged, as kernel32.py does.
 *
 * Each service cites the Microsoft Learn page it implements (as src/w2m/kernel32.py, retrieved
 * 2026-10-08). Host effects go through host_call; everything else stays in guest memory.
 */
#include "runtime.h"

#define STD_OUTPUT_HANDLE 0xFFFFFFF5u           /* (DWORD)-11, GetStdHandle */
#define STDOUT_HANDLE 0x74ull                   /* the runtime's handle value for standard output */
#define ERROR_INVALID_HANDLE 6u

static u64 host_call(u64 op, u64 a0, u64 a1) {
    volatile struct w2m_transport *t = (volatile struct w2m_transport *)W2M_TRANSPORT_VA;
    t->op = op;
    t->arg[0] = a0;
    t->arg[1] = a1;
    __asm__ volatile("hvc #8" ::: "memory");
    return t->result;
}

static void unimplemented(u64 api, u64 detail) {
    host_call(W2M_HOST_UNIMPLEMENTED, api, detail);     /* the host stops the run; no return */
    for (;;) {}
}

static u64 logged(u64 api, u64 arg0, u64 ret) {
    struct w2m_ring *r = (struct w2m_ring *)W2M_RING_VA;
    if (r->count < r->capacity) {
        r->rec[r->count].api = api;
        r->rec[r->count].arg0 = arg0;
        r->rec[r->count].ret = ret;
    }
    r->count++;
    return ret;
}

/* "Retrieves the thread identifier of the calling thread." */
u64 k32_GetCurrentThreadId(void) {
    return logged(W2M_API_GetCurrentThreadId, 0, (u32)w2m_teb()->client_thread);
}

/* "Sets the last-error code for the calling thread." Returns nothing: RAX is left as it was. */
u64 k32_SetLastError(u64 code) {
    w2m_set_last_error((u32)code);
    return logged(W2M_API_SetLastError, (u32)code, 0);
}

/* "Retrieves the calling thread's last-error code value." */
u64 k32_GetLastError(void) {
    return logged(W2M_API_GetLastError, 0, w2m_get_last_error());
}

u64 k32_GetStdHandle(u64 which) {
    if ((u32)which != STD_OUTPUT_HANDLE) unimplemented(W2M_API_GetStdHandle, (u32)which);
    return logged(W2M_API_GetStdHandle, (u32)which, STDOUT_HANDLE);
}

/* WriteFile(hFile, lpBuffer, nNumberOfBytesToWrite, lpNumberOfBytesWritten, lpOverlapped):
 * "sets this value to zero before doing any work"; nonzero (TRUE) on success. */
u64 k32_WriteFile(u64 handle, u64 buffer, u64 count, u64 written, u64 rsp) {
    u64 overlapped = *(u64 *)(rsp + 0x28);
    count &= 0xFFFFFFFFull;
    if (written == 0 || overlapped != 0) unimplemented(W2M_API_WriteFile, written);
    *(u32 *)written = 0;
    if (handle != STDOUT_HANDLE) {
        w2m_teb()->last_error_value = ERROR_INVALID_HANDLE;
        return logged(W2M_API_WriteFile, handle, 0);
    }
    host_call(W2M_HOST_WRITE, buffer, count);
    *(u32 *)written = (u32)count;
    return logged(W2M_API_WriteFile, handle, 1);
}

/* "Ends the calling process and all its threads." */
u64 k32_ExitProcess(u64 code) {
    logged(W2M_API_ExitProcess, (u32)code, 0);
    host_call(W2M_HOST_EXIT, (u32)code, 0);
    for (;;) {}
}

/* "Retrieves the current value of the performance counter"; nonzero on success. */
u64 k32_QueryPerformanceCounter(u64 out) {
    if (out == 0) unimplemented(W2M_API_QueryPerformanceCounter, 0);
    *(u64 *)out = host_call(W2M_HOST_CLOCK, 0, 0);
    return logged(W2M_API_QueryPerformanceCounter, out, 1);
}

/* The linker needs an entry symbol; nothing enters here. */
void w2m_image_entry(void) {}
