/* runtime.h — the w2m guest runtime's view of guest memory (Sprint 035).
 *
 * Freestanding C compiled to AArch64 and run inside the managed VM (BLACKBOARD 2026-10-09, runtime
 * architecture). Addresses come from src/w2m/vm_backend.py as -D definitions: W2M_TEB_VA,
 * W2M_PEB_VA, W2M_TRANSPORT_VA, W2M_RING_VA. Offsets are specs/guest_state.json's, confirmed on
 * Windows in Sprint 034 (store/probe-runs/37989842529).
 */
#ifndef W2M_RUNTIME_H
#define W2M_RUNTIME_H

typedef unsigned long long u64;
typedef unsigned int u32;
typedef unsigned char u8;

/* TEB fields the runtime reads or writes (winternl.h and the Sprint 034 observations). */
struct w2m_teb {
    u8 reserved0[0x08];
    u64 stack_base;                  /* 0x08 NtTib.StackBase */
    u64 stack_limit;                 /* 0x10 NtTib.StackLimit */
    u8 reserved1[0x30 - 0x18];
    u64 self;                        /* 0x30 NtTib.Self */
    u8 reserved2[0x40 - 0x38];
    u64 client_process;              /* 0x40 ClientId.UniqueProcess */
    u64 client_thread;               /* 0x48 ClientId.UniqueThread */
    u8 reserved3[0x60 - 0x50];
    u64 process_environment_block;   /* 0x60 */
    u32 last_error_value;            /* 0x68 */
};
_Static_assert(__builtin_offsetof(struct w2m_teb, client_thread) == 0x48, "TEB ClientId.UniqueThread");
_Static_assert(__builtin_offsetof(struct w2m_teb, last_error_value) == 0x68, "TEB LastErrorValue");

static inline struct w2m_teb *w2m_teb(void) { return (struct w2m_teb *)W2M_TEB_VA; }

/* Host transport: the runtime fills a request, executes hvc #8, and reads the result. */
enum { W2M_HOST_WRITE = 1, W2M_HOST_EXIT = 2, W2M_HOST_CLOCK = 3, W2M_HOST_UNIMPLEMENTED = 4 };
struct w2m_transport {
    u64 op;
    u64 arg[6];
    u64 result;
};

/* Ring buffer: one record per service call, drained by the host after the run. */
struct w2m_ring_record {
    u64 api;
    u64 arg0;
    u64 ret;
};
struct w2m_ring {
    u64 count;                       /* records written, including any dropped past capacity */
    u64 capacity;
    struct w2m_ring_record rec[];
};

/* API identifiers in the ring; src/w2m/guest_runtime.py reads these lines. */
#define W2M_API_GetCurrentThreadId 1
#define W2M_API_SetLastError 2
#define W2M_API_GetLastError 3
#define W2M_API_GetStdHandle 4
#define W2M_API_WriteFile 5
#define W2M_API_ExitProcess 6
#define W2M_API_QueryPerformanceCounter 7

/* The contract surface a candidate may implement (contracts/windows.thread.last_error.set_get).
 * The default is tools/guest_runtime/last_error.c. */
void w2m_set_last_error(u32 code);
u32 w2m_get_last_error(void);

#endif
