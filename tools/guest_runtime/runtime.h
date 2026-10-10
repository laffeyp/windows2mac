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
enum { W2M_HOST_WRITE = 1, W2M_HOST_EXIT = 2, W2M_HOST_CLOCK = 3, W2M_HOST_UNIMPLEMENTED = 4,
       W2M_HOST_RUNTIME_FAULT = 5 };
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

/* The thread's state block (specs/guest_state.json 0.2.0 state_block; Sprint 040). Translated code
 * keeps the guest registers in host registers; the block holds them whenever the thread is not
 * running x86 code: in a service, at a fault, when stopped. */
struct w2m_state {
    u64 gpr[16];                     /* 0x000 rax rcx rdx rbx rsp rbp rsi rdi r8-r15 */
    u64 rip;                         /* 0x080 */
    u64 rflags;                      /* 0x088 materialized: CF PF AF ZF SF DF OF, IF and bit 1 */
    u64 flags_undefined;             /* 0x090 one bit per flag while undefined: C Z S O P A */
    u32 mxcsr;                       /* 0x098 */
    unsigned short x87_control_word; /* 0x09c */
    unsigned short x87_status_word;  /* 0x09e */
    u8 x87_tag_abridged;             /* 0x0a0 */
    u8 reserved0[7];
    u64 fs_base;                     /* 0x0a8 */
    u64 gs_base;                     /* 0x0b0 */
    u64 teb;                         /* 0x0b8 */
    u32 thread_state;                /* 0x0c0 */
    u32 suspend_count;               /* 0x0c4 */
    u64 af_operands[3];              /* 0x0c8 */
    u8 df;                           /* 0x0e0 */
    u8 reserved1[0x100 - 0xe1];
    u8 xmm[16][16];                  /* 0x100 */
    u8 ymm_high[16][16];             /* 0x200 */
    u8 st[8][16];                    /* 0x300 */
};
_Static_assert(sizeof(struct w2m_state) == 896, "state block size (spec 896)");
_Static_assert(__builtin_offsetof(struct w2m_state, rip) == 128, "state.rip");
_Static_assert(__builtin_offsetof(struct w2m_state, mxcsr) == 152, "state.mxcsr");
_Static_assert(__builtin_offsetof(struct w2m_state, af_operands) == 200, "state.af_operands");
_Static_assert(__builtin_offsetof(struct w2m_state, df) == 224, "state.df");
_Static_assert(__builtin_offsetof(struct w2m_state, xmm) == 256, "state.xmm0");
_Static_assert(__builtin_offsetof(struct w2m_state, st) == 768, "state.st0");

/* Windows x64 EXCEPTION_RECORD and CONTEXT, at the offsets probes/guest_state measured on the
 * runner (specs/guest_state.json windows.*). Only the fields the runtime writes are named. */
struct w2m_exception_record {
    u32 code;                        /* 0 */
    u32 flags;                       /* 4 */
    u64 record;                      /* 8 */
    u64 address;                     /* 16 */
    u32 number_parameters;           /* 24 */
    u32 pad;
    u64 information[15];             /* 32 */
};
_Static_assert(sizeof(struct w2m_exception_record) == 152, "EXCEPTION_RECORD size");

struct w2m_context {
    u64 p_home[6];                   /* 0 */
    u32 context_flags;               /* 48 */
    u32 mx_csr;                      /* 52 */
    unsigned short seg_cs, seg_ds, seg_es, seg_fs, seg_gs, seg_ss;   /* 56 */
    u32 eflags;                      /* 68 */
    u64 dr[6];                       /* 72 */
    u64 gpr[16];                     /* 120 Rax Rcx Rdx Rbx Rsp Rbp Rsi Rdi R8-R15 */
    u64 rip;                         /* 248 */
    struct {                         /* 256 FltSave: XMM_SAVE_AREA32 */
        unsigned short control_word, status_word;
        u8 tag_word, reserved1;
        unsigned short error_opcode;
        u32 error_offset;
        unsigned short error_selector, reserved2;
        u32 data_offset;
        unsigned short data_selector, reserved3;
        u32 mx_csr, mx_csr_mask;
        u8 float_registers[8][16];   /* 32 */
        u8 xmm_registers[16][16];    /* 160 */
        u8 reserved4[96];
    } flt_save;
    u8 vector_register[26][16];      /* 768 */
    u64 vector_control;              /* 1184 */
    u64 debug_control;               /* 1192 */
    u64 last_branch_to_rip, last_branch_from_rip, last_exception_to_rip, last_exception_from_rip;
};
_Static_assert(sizeof(struct w2m_context) == 1232, "CONTEXT size");
_Static_assert(__builtin_offsetof(struct w2m_context, gpr) == 120, "CONTEXT.Rax");
_Static_assert(__builtin_offsetof(struct w2m_context, rip) == 248, "CONTEXT.Rip");
_Static_assert(__builtin_offsetof(struct w2m_context, flt_save) == 256, "CONTEXT.FltSave");
_Static_assert(__builtin_offsetof(struct w2m_context, vector_control) == 1184, "CONTEXT.VectorControl");

/* What the EL1 exception vector saves (src/w2m/vm_backend.py exception entry). */
struct w2m_save_area {
    u64 x[31];
    u64 pad;
    u64 esr, far, elr;               /* 0x100 */
};

/* Where the runtime leaves an exception it could not hand to a handler: the record and the
 * CONTEXT it built, for the host to read (Sprint 040; VEH dispatch is Sprint 047). */
struct w2m_exception_area {
    u64 magic;                       /* W2M_EXCEPTION_MAGIC once written */
    u64 pad;
    struct w2m_exception_record record;   /* 16 */
    u8 pad2[0x100 - 16 - 152];
    struct w2m_context context;      /* 0x100 */
};
#define W2M_EXCEPTION_MAGIC 0x7832774E4F495445ull     /* "ETIONw2x" */

/* API identifiers in the ring; src/w2m/guest_runtime.py reads these lines. */
#define W2M_API_GetCurrentThreadId 1
#define W2M_API_SetLastError 2
#define W2M_API_GetLastError 3
#define W2M_API_GetStdHandle 4
#define W2M_API_WriteFile 5
#define W2M_API_ExitProcess 6
#define W2M_API_QueryPerformanceCounter 7
#define W2M_API_Exception 8
#define W2M_API_Cpuid 9

/* The contract surface a candidate may implement (contracts/windows.thread.last_error.set_get).
 * The default is tools/guest_runtime/last_error.c. */
void w2m_set_last_error(u32 code);
u32 w2m_get_last_error(void);

#endif
