// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
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
typedef long long s64;
typedef int s32;

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

/* The running thread's TEB: the state block's gs_base (0xb0), which the host sets when it switches threads (Sprint
 * 051; before, one fixed TEB at W2M_TEB_VA, which stays the main thread's). */
#ifdef W2M_STATE_VA
static inline struct w2m_teb *w2m_teb(void) { return *(struct w2m_teb **)(W2M_STATE_VA + 0xb0); }
#else                                        /* a candidate built for the host (src/w2m/candidate.py): one TEB */
static inline struct w2m_teb *w2m_teb(void) { return (struct w2m_teb *)W2M_TEB_VA; }
#endif

/* w2m's own per-thread words, past the TEB's last Windows field (0x1838) in its 0x2000 bytes (Sprint 051): what a
 * blocked call keeps between its retries, and what ExitThread leaves for the end of the thread. */
struct w2m_thread_scratch {
    u64 wait_deadline;               /* the counter value a blocked wait times out at; 0: none, ~0: never */
    u64 wait_cv;                     /* SleepConditionVariableSRW: the variable waited on, while waiting */
    u64 wait_generation;             /* its wake count when the wait began */
    u32 exit_code;                   /* ExitThread's code, for the detach stub's end */
    u32 phase;                       /* a retried call's progress (SleepConditionVariableSRW: 1 released, 2 woken) */
    u32 com_inits;                   /* CoInitialize(Ex) calls not yet balanced by CoUninitialize (com.c; Sprint 052) */
    u32 com_model;                   /* their apartment: 1 multithreaded, 2 single-threaded */
    u64 progress;                    /* a retried WriteFile to a pipe: the bytes the pipe took so far (Sprint 058) */
};
#define W2M_BLOCKED (~0ull)          /* a service's helper blocked the thread: the call is retried, not returned */
#define W2M_THREAD_SCRATCH_OFF 0x1900
/* PEB.ProcessParameters (RTL_USER_PROCESS_PARAMETERS; the host writes it, vm_backend.py _init_parameters; Sprint
 * 059): Flags +8, the standard handles +0x20, CurrentDirectory.DosPath +0x38, CommandLine +0x70, Environment +0x80,
 * the window fields +0x88, WindowTitle +0xB0, DesktopInfo +0xC0, EnvironmentSize +0x3F0, EnvironmentVersion +0x3F8. */
static inline u8 *w2m_parameters(void) { return *(u8 *const volatile *)(W2M_PEB_VA + 0x20); }

static inline struct w2m_thread_scratch *w2m_thread_scratch(void) {
    return (struct w2m_thread_scratch *)((u8 *)w2m_teb() + W2M_THREAD_SCRATCH_OFF);
}

/* Host transport: the runtime fills a request, executes hvc #8, and reads the result. */
enum { W2M_HOST_WRITE = 1, W2M_HOST_EXIT = 2, W2M_HOST_CLOCK = 3, W2M_HOST_UNIMPLEMENTED = 4,
       W2M_HOST_RUNTIME_FAULT = 5, W2M_HOST_FILETIME = 14,     /* 6-13: the file primitives below */
       W2M_HOST_MODULE = 15, W2M_HOST_PROC = 16, W2M_HOST_MODULE_PATH = 17, W2M_HOST_MODULE_AT = 18,
       W2M_HOST_VIRTUAL_ALLOC = 19, W2M_HOST_VIRTUAL_FREE = 20, W2M_HOST_VIRTUAL_PROTECT = 21,   /* virtual_memory.py */
       W2M_HOST_VIRTUAL_QUERY = 22, W2M_HOST_RANDOM = 23 };          /* 23: random bytes (bcrypt.c; Sprint 047) */
struct w2m_transport {
    u64 op;
    u64 arg[6];
    u64 result;
};

/* Ring buffer: one record per service call, in order, for the first `capacity` calls; `per_api`
 * counts every call by API id, past capacity too (Sprint 044: the hot services make millions of
 * calls, and the totals must stay exact). The host reads both after the run, without an exit. */
#define W2M_RING_APIS 512                /* above every W2M_API_* id (tests/test_guest_runtime.py) */
struct w2m_ring_record {
    u64 api;
    u64 arg0;
    u64 ret;
};
struct w2m_ring {
    u64 count;                       /* records written, including any dropped past capacity */
    u64 capacity;
    u64 per_api[W2M_RING_APIS];      /* calls per W2M_API_* id */
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
    u8 reserved1[0xe8 - 0xe1];
    u64 redirect_rip;                /* 0x0e8 a service's next x86 address instead of its return (Sprint 051) */
    u32 schedule;                    /* 0x0f0 W2M_SCHEDULE_*: the bridge hands the vCPU to the host's scheduler */
    u8 reserved2[0x100 - 0xf4];
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
_Static_assert(__builtin_offsetof(struct w2m_state, redirect_rip) == 0xe8, "state.redirect_rip");
_Static_assert(__builtin_offsetof(struct w2m_state, schedule) == 0xf0, "state.schedule");
#ifdef W2M_STATE_VA
static inline struct w2m_state *w2m_state(void) { return (struct w2m_state *)W2M_STATE_VA; }
#endif

/* The bridge reads state.schedule after a service returns (Sprint 051; src/w2m/vm_backend.py _bridge):
 * YIELD   return to the caller, then let the scheduler run another thread (Sleep, SwitchToThread)
 * RETRY   do not return: the thread re-enters the same call when it next runs (a wait that cannot complete yet);
 *         the scheduler runs it again when an object is signaled or its TEB scratch's wait_deadline passes
 * END     the thread has ended (ExitThread's last step); it never runs again */
#define W2M_SCHEDULE_YIELD 1u
#define W2M_SCHEDULE_RETRY 2u
#define W2M_SCHEDULE_END 4u
#define W2M_SCHEDULE_ANSWER 8u               /* with YIELD: RAX = nonzero when another thread ran (SwitchToThread) */
void w2m_block(u64 deadline);                /* threads.c: RETRY until signaled or the counter passes deadline */
void w2m_signal(void);                       /* threads.c: a waitable thing changed; blocked threads re-check */
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
    u64 sw_code, sw_rip;             /* 0x118: an exception the host raised (an unmasked SIMD exception
                                        found by the slow path), or 0 for a hardware exception */
    u8 pad2[0x200 - 0x128];
    u8 q[16][16];                    /* 0x200: V0-V15, which hold XMM0-15 */
};
_Static_assert(__builtin_offsetof(struct w2m_save_area, sw_code) == 0x118, "save.sw_code");
_Static_assert(__builtin_offsetof(struct w2m_save_area, q) == 0x200, "save.q");

/* Where the runtime leaves an exception no handler took, which ends the process: the record and
 * the CONTEXT it was raised with, for the host to read (Sprint 040). A fault builds them here first;
 * the magic is written only when the exception ends the process (exceptions.c; Sprint 053). */
struct w2m_exception_area {
    u64 magic;                       /* W2M_EXCEPTION_MAGIC once written */
    u64 pad;
    struct w2m_exception_record record;   /* 16 */
    u8 pad2[0x100 - 16 - 152];
    struct w2m_context context;      /* 0x100 */
};
#define W2M_EXCEPTION_MAGIC 0x7832774E4F495445ull     /* "ETIONw2x" */

/* API identifiers in the ring: W2M_API_<Name> for every service the runtime's C files define and for the
 * internal entries Cpuid and Exception, 1 up in name order; src/w2m/guest_runtime.py writes them at build
 * (api_header) from the definitions themselves (Sprint 049), so a new service needs no line here. */
#include "api_ids.h"

/* Marks a service Windows declares void: the bridge leaves the guest's RAX as it was (vm_backend sets_rax). */
#define W2M_VOID

/* Shared by runtime.c and file.c. */
u64 w2m_host(u64 op, u64 a0, u64 a1, u64 a2, u64 a3, u64 *second);
u64 w2m_logged(u64 api, u64 arg0, u64 ret);
void w2m_unimplemented(u64 api, u64 detail);
u64 w2m_file_write(u64 handle, u64 buffer, u64 count, u64 written);   /* file.c: WriteFile on a file handle */
u32 w2m_file_read_all(const unsigned short *name, u8 *buf, u64 max, u64 *size);   /* file.c: a whole file (Sprint 063) */
void w2m_cpuid_query(u32 leaf, u32 sub, u32 r[4]);                     /* runtime.c: the guest CPUID profile */
int w2m_file_is_open(u64 handle);                                       /* file.c */
u64 w2m_heap_take(u64 size);                                            /* heap.c: a process-heap block, unlogged */
void w2m_heap_release(u64 api, u64 p);                                  /* heap.c: give it back */
int w2m_object_close(u64 handle);                                       /* sync_objects.c: CloseHandle on an object */
u64 w2m_counter_now(void);                                              /* process.c: the 10 MHz counter */
u64 w2m_filetime_now(void);                                             /* process.c: the system time */
u64 w2m_filetime_start(void);                                           /* process.c: its first reading */
u64 w2m_unhandled_exception_filter(void);                               /* exception_filter.c: the current one */
#define W2M_MANIFEST_WINDOWS_10 1u          /* the exe's manifest names Windows 10 among its supportedOS */
#define W2M_MANIFEST_PER_MONITOR_V2 2u      /* ... and the PerMonitorV2 DPI awareness */
u32 w2m_manifest(void);                                                 /* resources.c: the two, as flags */
enum { W2M_RESOURCE_NO_DIRECTORY = 1, W2M_RESOURCE_NO_TYPE, W2M_RESOURCE_NO_NAME };
u64 w2m_resource(u64 base, u32 type, u32 id, u32 *size, u32 *missing);  /* resources.c: the data, or 0 and why */
struct w2m_exception_record;
struct w2m_context;
int w2m_dispatch_fault(const struct w2m_exception_record *rec, const struct w2m_context *ctx);   /* exceptions.c */

/* Services that call the guest's x86 code and resume when it returns (guest_calls.c, Sprint 054). A frame starts
 * with this; the guest's answer goes to `resume`, which calls the guest again (w2m_call_guest) or returns to the
 * service's own caller (w2m_return_to). */
struct w2m_guest_call {
    u64 magic;
    void (*resume)(struct w2m_guest_call *, u64 answer);
};
#define W2M_GUEST_CALL_MAGIC 0x6c6c616374736567ull     /* "guestcal" */
#define W2M_STUB_RETURN 0x00                         /* guest_stubs.py: ret */
#define W2M_STUB_CALL 0x10                           /* call rax; then W2mGuestReturn with the answer in RCX */
#define W2M_STUB_CALL_RETURN 0x12                    /* the return address the called code sees */
#define W2M_STUB_RESTORE 0x40                        /* rcx = CONTEXT: the context resumed whole */
u64 w2m_guest_stubs(void);                                               /* the stubs' page */
void *w2m_guest_frame(u64 rsp, u64 size, void (*resume)(struct w2m_guest_call *, u64));   /* zeroed, below rsp */
void w2m_call_guest(struct w2m_guest_call *c, u64 fn, u64 a0, u64 a1, u64 a2, u64 a3);
void w2m_return_to(u64 rsp, u64 value);           /* the service entered with stack pointer rsp returns value */
struct w2m_guest_call *w2m_guest_call_at(u64 rsp);   /* the frame a stub call at rsp belongs to, or 0 */

/* Monitor data word: the process's start (console.c): 0 a console program with redirected standard
 * output (the probes), 1 a GUI program with no handles (the game as Steam starts it). */
#define W2M_START_MODE_OFF 0x68

/* Monitor data word: 0, the process takes the foreground when it activates a window (the probes); 1, another
 * process keeps it, so a window activates in the background (the game runs: game run 38275716702's hook saw a
 * Windows Terminal window hold it throughout). src/w2m/vm_backend.py FOREGROUND_MODE; windows.c SetFocus. */
#define W2M_FOREGROUND_MODE_OFF 0x80

/* Monitor data word: 0, the counter is the guest's CNTVCT_EL0; 1, the host's clock (lockstep, where
 * model and VM must read the same count). src/w2m/vm_backend.py CLOCK_MODE. */
#define W2M_CLOCK_MODE_OFF 0x60

/* Host file primitives (src/w2m/file_bridge.py). */
enum { W2M_FILE_LOOKUP = 6, W2M_FILE_OPEN, W2M_FILE_CLOSE, W2M_FILE_READ, W2M_FILE_WRITE, W2M_FILE_SIZE,
       W2M_FILE_TRUNCATE, W2M_FILE_UNLINK, W2M_FILE_STAT = 24, W2M_FILE_MKDIR, W2M_FILE_RMDIR,      /* 24: Sprint 047; 25, 26: 048 */
       W2M_FILE_FSTAT, W2M_FILE_DUP, W2M_HOST_MAP_VIEW,                                           /* 27-29: Sprint 049 */
       W2M_HOST_THREAD,                                                                           /* 30: Sprint 051 */
       W2M_HOST_EXCEPTION,                                                                        /* 31: Sprint 053 */
       W2M_FILE_LIST,                                                                             /* 32: Sprint 054 */
       W2M_HOST_KERNEL };                                                                         /* 33: Sprint 055 */

/* W2M_HOST_KERNEL's sub-operations (src/w2m/kernel.py): the objects processes share. */
enum { W2M_KERNEL_EVENT_OPEN = 1, W2M_KERNEL_EVENT_STATE, W2M_KERNEL_EVENT_TAKE, W2M_KERNEL_EVENT_SET,
       W2M_KERNEL_EVENT_RESET, W2M_KERNEL_CLOSE, W2M_KERNEL_PIPE_CREATE, W2M_KERNEL_PIPE_READ, W2M_KERNEL_PIPE_WRITE,
       W2M_KERNEL_PROCESS_CREATE, W2M_KERNEL_PROCESS_STATE, W2M_KERNEL_REF };
/* sync_objects.c's object kinds for the host's objects (kernel.py KIND_*; PROCESS is 7) */
#define W2M_OBJECT_SHARED_EVENT 9
#define W2M_OBJECT_PIPE_READ 10
#define W2M_OBJECT_PIPE_WRITE 11
#define W2M_OBJECT_REMOTE_THREAD 12
#define W2M_INHERIT_STANDARD 100              /* an inherited-list entry naming a standard handle */
#define W2M_INHERIT_ERROR_MODE 101            /* ... the parent's error mode, which the child inherits */
#define W2M_INHERIT_DIRECTORY 102             /* ... the parent's current directory: (characters, kind, address) */
u32 w2m_error_mode(void);                                               /* time_and_error_mode.c */
int w2m_pipe_read(u64 handle, u64 buffer, u64 count, u64 read);        /* processes.c */
int w2m_pipe_write(u64 handle, u64 buffer, u64 count, u64 written);
int w2m_is_pipe(u64 handle);

/* W2M_HOST_EXCEPTION's first argument (exceptions.c): the x86 stubs' page; XMM0-15 copied to a CONTEXT's. */
enum { W2M_EXCEPTION_STUBS = 1, W2M_EXCEPTION_CAPTURE_XMM };

/* W2M_HOST_THREAD's first argument (src/w2m/scheduler.py). */
enum { W2M_THREAD_CREATE = 1, W2M_THREAD_EXIT_STUB, W2M_THREAD_END, W2M_THREAD_SUSPEND, W2M_THREAD_RESUME,
       W2M_THREAD_CONTEXT, W2M_THREAD_EXIT_CODE, W2M_THREAD_SIGNAL };

/* The contract surface a candidate may implement (contracts/windows.thread.last_error.set_get).
 * The default is tools/guest_runtime/last_error.c. */
void w2m_set_last_error(u32 code);
u32 w2m_get_last_error(void);
/* The machine's processors (Sprint 061: the runners', row machine of windows.system.processor_topology): four on two
 * cores with SMT. system_info.c, topology.c and the PEB (vm_backend.py PROCESSORS) report them. */
#define W2M_PROCESSORS 4u
#define W2M_PROCESSOR_MASK 0xFull
void w2m_trace_return(u64 rsp, u64 value);   /* call_trace.c: a traced call that ran guest code returns (Sprint 061) */

#endif
