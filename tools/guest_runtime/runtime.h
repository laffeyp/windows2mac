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
       W2M_HOST_RUNTIME_FAULT = 5, W2M_HOST_FILETIME = 14,     /* 6-13: the file primitives below */
       W2M_HOST_MODULE = 15, W2M_HOST_PROC = 16, W2M_HOST_MODULE_PATH = 17, W2M_HOST_MODULE_AT = 18 };
struct w2m_transport {
    u64 op;
    u64 arg[6];
    u64 result;
};

/* Ring buffer: one record per service call, in order, for the first `capacity` calls; `per_api`
 * counts every call by API id, past capacity too (Sprint 044: the hot services make millions of
 * calls, and the totals must stay exact). The host reads both after the run, without an exit. */
#define W2M_RING_APIS 256                /* above every W2M_API_* id (tests/test_guest_runtime.py) */
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
    u64 sw_code, sw_rip;             /* 0x118: an exception the host raised (an unmasked SIMD exception
                                        found by the slow path), or 0 for a hardware exception */
    u8 pad2[0x200 - 0x128];
    u8 q[16][16];                    /* 0x200: V0-V15, which hold XMM0-15 */
};
_Static_assert(__builtin_offsetof(struct w2m_save_area, sw_code) == 0x118, "save.sw_code");
_Static_assert(__builtin_offsetof(struct w2m_save_area, q) == 0x200, "save.q");

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
#define W2M_API_CreateFileW 10
#define W2M_API_CloseHandle 11
#define W2M_API_DeleteFileW 12
#define W2M_API_ReadFile 13
#define W2M_API_SetFilePointerEx 14
#define W2M_API_GetSystemTimeAsFileTime 15
#define W2M_API_GetCurrentProcessId 16
#define W2M_API_QueryPerformanceFrequency 17
#define W2M_API_IsProcessorFeaturePresent 18
#define W2M_API_InitializeSListHead 19
#define W2M_API_GetStartupInfoW 20
#define W2M_API_IsDebuggerPresent 21
#define W2M_API_LoadLibraryExW 22
#define W2M_API_GetModuleHandleW 23
#define W2M_API_GetModuleHandleExW 24
#define W2M_API_GetModuleFileNameW 25
#define W2M_API_GetProcAddress 26
#define W2M_API_FreeLibrary 27
#define W2M_API_InitializeCriticalSectionEx 28
#define W2M_API_InitializeCriticalSectionAndSpinCount 29
#define W2M_API_InitializeCriticalSection 30
#define W2M_API_EnterCriticalSection 31
#define W2M_API_TryEnterCriticalSection 32
#define W2M_API_LeaveCriticalSection 33
#define W2M_API_DeleteCriticalSection 34
#define W2M_API_TlsAlloc 35
#define W2M_API_TlsGetValue 36
#define W2M_API_TlsSetValue 37
#define W2M_API_TlsFree 38
#define W2M_API_FlsAlloc 39
#define W2M_API_FlsGetValue 40
#define W2M_API_FlsSetValue 41
#define W2M_API_FlsFree 42
#define W2M_API_GetProcessHeap 43
#define W2M_API_HeapAlloc 44
#define W2M_API_HeapFree 45
#define W2M_API_HeapSize 46
#define W2M_API_HeapReAlloc 47
#define W2M_API_SetStdHandle 48
#define W2M_API_GetFileType 49
#define W2M_API_GetConsoleMode 50
#define W2M_API_GetConsoleCP 51
#define W2M_API_WriteConsoleW 52
#define W2M_API_FlushFileBuffers 53
#define W2M_API_GetCommandLineW 54
#define W2M_API_GetCommandLineA 55
#define W2M_API_GetACP 56
#define W2M_API_GetOEMCP 57
#define W2M_API_IsValidCodePage 58
#define W2M_API_GetCPInfo 59
#define W2M_API_MultiByteToWideChar 60
#define W2M_API_WideCharToMultiByte 61
#define W2M_API_GetStringTypeW 62
#define W2M_API_LCMapStringW 63
#define W2M_API_LCMapStringEx 64
#define W2M_API_GetEnvironmentStringsW 65
#define W2M_API_FreeEnvironmentStringsW 66
#define W2M_API_lstrlenW 67
#define W2M_API_SetUnhandledExceptionFilter 68
#define W2M_API_EncodePointer 69
#define W2M_API_DecodePointer 70
#define W2M_API_InitializeSRWLock 71
#define W2M_API_InitializeConditionVariable 72
#define W2M_API_AcquireSRWLockExclusive 73
#define W2M_API_TryAcquireSRWLockExclusive 74
#define W2M_API_ReleaseSRWLockExclusive 75
#define W2M_API_AcquireSRWLockShared 76
#define W2M_API_TryAcquireSRWLockShared 77
#define W2M_API_ReleaseSRWLockShared 78
#define W2M_API_SleepConditionVariableSRW 79
#define W2M_API_WakeConditionVariable 80
#define W2M_API_WakeAllConditionVariable 81
#define W2M_API_GetCurrentThread 82
#define W2M_API_GetThreadTimes 83
#define W2M_API_GetThreadPriority 84

/* Shared by runtime.c and file.c. */
u64 w2m_host(u64 op, u64 a0, u64 a1, u64 a2, u64 a3, u64 *second);
u64 w2m_logged(u64 api, u64 arg0, u64 ret);
void w2m_unimplemented(u64 api, u64 detail);
u64 w2m_file_write(u64 handle, u64 buffer, u64 count, u64 written);   /* file.c: WriteFile on a file handle */
void w2m_cpuid_query(u32 leaf, u32 sub, u32 r[4]);                     /* runtime.c: the guest CPUID profile */
int w2m_file_is_open(u64 handle);                                       /* file.c */
u64 w2m_heap_take(u64 size);                                            /* heap.c: a process-heap block, unlogged */
void w2m_heap_release(u64 api, u64 p);                                  /* heap.c: give it back */
u64 w2m_filetime_now(void);                                             /* process.c: the system time */
u64 w2m_filetime_start(void);                                           /* process.c: its first reading */
u64 w2m_unhandled_exception_filter(void);                               /* exception_filter.c: the current one */

/* Monitor data word: the process's start (console.c): 0 a console program with redirected standard
 * output (the probes), 1 a GUI program with no handles (the game as Steam starts it). */
#define W2M_START_MODE_OFF 0x68

/* Monitor data word: 0, the counter is the guest's CNTVCT_EL0; 1, the host's clock (lockstep, where
 * model and VM must read the same count). src/w2m/vm_backend.py CLOCK_MODE. */
#define W2M_CLOCK_MODE_OFF 0x60

/* Host file primitives (src/w2m/file_bridge.py). */
enum { W2M_FILE_LOOKUP = 6, W2M_FILE_OPEN, W2M_FILE_CLOSE, W2M_FILE_READ, W2M_FILE_WRITE, W2M_FILE_SIZE,
       W2M_FILE_TRUNCATE, W2M_FILE_UNLINK };

/* The contract surface a candidate may implement (contracts/windows.thread.last_error.set_get).
 * The default is tools/guest_runtime/last_error.c. */
void w2m_set_last_error(u32 code);
u32 w2m_get_last_error(void);

#endif
