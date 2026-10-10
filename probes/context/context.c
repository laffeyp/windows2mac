/* context.c — thread state at faults and suspensions, without the C runtime (Sprint 040).
 *
 * The checks of probes/guest_state (Sprint 034) that concern CONTEXT, rebuilt to run both on the
 * Windows runner and in w2m: entry point `start`, kernel32 imports only, no struct copies (no
 * memcpy without a C runtime). Prints one JSON object per line, the same format as guest_state:
 *   {"kind":"check","name":...,"offset":...,"observed":...,"expected":...,"equal":...}
 *   fault.*   a first vectored handler sees an access violation with known registers, moves RIP
 *             past the instruction and sets RCX; the probe checks both took effect
 *   suspend.* GetThreadContext/SetThreadContext on a suspended thread with known registers
 *   stores.*  a thread runs ADD [m], 1 then PUSH/POP N times while the main thread suspends and
 *             resumes it repeatedly; a suspension that repeated or lost a store would change the
 *             count (review 2026-10-09 progress N2: the forward rule)
 * Exit code 0 when every check is equal, 1 otherwise.
 */
#include <windows.h>

#define K_RBX 0x1111111111111111ULL
#define K_RSI 0x2222222222222222ULL
#define K_RDI 0x3333333333333333ULL
#define K_R12 0x4444444444444444ULL
#define K_R13 0x5555555555555555ULL
#define K_R14 0x6666666666666666ULL
#define K_R15 0x7777777777777777ULL
#define K_MARK 0x00000000C0FFEE00ULL
#define S_RBX 0x5151515151515151ULL
#define S_R12 0x1212121212121212ULL
#define S_NEW 0x0BADF00D0BADF00DULL
#define STORES 200000

static char out[8192];
static unsigned n;
static int failures;

static void put(const char *s) {
    while (*s) out[n++] = *s++;
}

static void putu(unsigned long long v) {
    char t[24];
    int k = 0;
    do {
        t[k++] = (char)('0' + v % 10);
        v /= 10;
    } while (v);
    while (k) out[n++] = t[--k];
}

static void puti(long long v) {
    if (v < 0) {
        out[n++] = '-';
        putu((unsigned long long)-v);
    } else {
        putu((unsigned long long)v);
    }
}

static void check(const char *name, long long offset, unsigned long long observed,
                  unsigned long long expected) {
    int eq = observed == expected;
    failures += !eq;
    put("{\"kind\":\"check\",\"name\":\"");
    put(name);
    put("\",\"offset\":");
    puti(offset);
    put(",\"observed\":");
    putu(observed);
    put(",\"expected\":");
    putu(expected);
    put(",\"equal\":");
    put(eq ? "true" : "false");
    put("}\n");
}

#define OFF(type, field) ((long long)__builtin_offsetof(type, field))

static volatile LONG handled;
static DWORD seen_code, seen_flags, seen_nparams;
static ULONG_PTR seen_address, seen_info0, seen_info1;
static DWORD64 seen_rip, seen_rax, seen_rbx, seen_rsi, seen_rdi, seen_r12, seen_r13, seen_r14, seen_r15;
static DWORD seen_mxcsr, seen_context_flags, seen_eflags, seen_fltsave_mxcsr;
static WORD seen_control_word;
extern char fault_insn[];

static LONG CALLBACK on_fault(EXCEPTION_POINTERS *ep) {
    EXCEPTION_RECORD *r = ep->ExceptionRecord;
    CONTEXT *c = ep->ContextRecord;
    if (r->ExceptionCode != EXCEPTION_ACCESS_VIOLATION || c->Rip != (DWORD64)(ULONG_PTR)fault_insn)
        return EXCEPTION_CONTINUE_SEARCH;
    seen_code = r->ExceptionCode;
    seen_flags = r->ExceptionFlags;
    seen_address = (ULONG_PTR)r->ExceptionAddress;
    seen_nparams = r->NumberParameters;
    seen_info0 = r->ExceptionInformation[0];
    seen_info1 = r->ExceptionInformation[1];
    seen_rip = c->Rip;
    seen_rax = c->Rax;
    seen_rbx = c->Rbx;
    seen_rsi = c->Rsi;
    seen_rdi = c->Rdi;
    seen_r12 = c->R12;
    seen_r13 = c->R13;
    seen_r14 = c->R14;
    seen_r15 = c->R15;
    seen_mxcsr = c->MxCsr;
    seen_fltsave_mxcsr = c->FltSave.MxCsr;
    seen_control_word = c->FltSave.ControlWord;
    seen_context_flags = c->ContextFlags;
    seen_eflags = c->EFlags;
    handled = 1;
    c->Rip += 3;                                       /* mov rcx, [rax]: 48 8b 08 */
    c->Rcx = K_MARK;
    return EXCEPTION_CONTINUE_EXECUTION;
}

static void __attribute__((noinline)) fault(void) {
    unsigned long long marker = 0;
    PVOID h = AddVectoredExceptionHandler(1, on_fault);
    __asm__ volatile(
        "movabsq $0x1111111111111111, %%rbx\n\t"
        "movabsq $0x2222222222222222, %%rsi\n\t"
        "movabsq $0x3333333333333333, %%rdi\n\t"
        "movabsq $0x4444444444444444, %%r12\n\t"
        "movabsq $0x5555555555555555, %%r13\n\t"
        "movabsq $0x6666666666666666, %%r14\n\t"
        "movabsq $0x7777777777777777, %%r15\n\t"
        "xorl %%eax, %%eax\n\t"
        "xorl %%ecx, %%ecx\n\t"
        ".globl fault_insn\n"
        "fault_insn:\n\t"
        "movq (%%rax), %%rcx\n\t"
        "movq %%rcx, %0\n\t"
        : "=m"(marker)
        :
        : "rax", "rbx", "rcx", "rsi", "rdi", "r12", "r13", "r14", "r15", "memory");
    check("fault.removed", -1, RemoveVectoredExceptionHandler(h) != 0, 1);
    check("fault.handled", -1, handled, 1);
    check("fault.code", OFF(EXCEPTION_RECORD, ExceptionCode), seen_code, EXCEPTION_ACCESS_VIOLATION);
    check("fault.flags", OFF(EXCEPTION_RECORD, ExceptionFlags), seen_flags, 0);
    check("fault.address", OFF(EXCEPTION_RECORD, ExceptionAddress), seen_address, (ULONG_PTR)fault_insn);
    check("fault.parameters", OFF(EXCEPTION_RECORD, NumberParameters), seen_nparams, 2);
    check("fault.info0_read", OFF(EXCEPTION_RECORD, ExceptionInformation), seen_info0, 0);
    check("fault.info1_address", OFF(EXCEPTION_RECORD, ExceptionInformation) + 8, seen_info1, 0);
    check("fault.rip", OFF(CONTEXT, Rip), seen_rip, (ULONG_PTR)fault_insn);
    check("fault.rax", OFF(CONTEXT, Rax), seen_rax, 0);
    check("fault.rbx", OFF(CONTEXT, Rbx), seen_rbx, K_RBX);
    check("fault.rsi", OFF(CONTEXT, Rsi), seen_rsi, K_RSI);
    check("fault.rdi", OFF(CONTEXT, Rdi), seen_rdi, K_RDI);
    check("fault.r12", OFF(CONTEXT, R12), seen_r12, K_R12);
    check("fault.r13", OFF(CONTEXT, R13), seen_r13, K_R13);
    check("fault.r14", OFF(CONTEXT, R14), seen_r14, K_R14);
    check("fault.r15", OFF(CONTEXT, R15), seen_r15, K_R15);
    check("fault.mxcsr", OFF(CONTEXT, MxCsr), seen_mxcsr, 0x1F80);
    check("fault.fltsave_mxcsr", OFF(CONTEXT, FltSave) + OFF(XMM_SAVE_AREA32, MxCsr), seen_fltsave_mxcsr, 0x1F80);
    check("fault.x87_control_word", OFF(CONTEXT, FltSave) + OFF(XMM_SAVE_AREA32, ControlWord), seen_control_word, 0x27F);
    check("fault.context_flags_full", OFF(CONTEXT, ContextFlags), (seen_context_flags & CONTEXT_FULL) == CONTEXT_FULL, 1);
    check("fault.eflags_if", OFF(CONTEXT, EFlags), (seen_eflags >> 9) & 1, 1);
    check("fault.write_back_rcx", -1, marker, K_MARK);
}

extern char spin_begin[], spin_end[];
static volatile unsigned long long spin_rbx_after;

static DWORD WINAPI __attribute__((noinline)) spin(LPVOID p) {
    (void)p;
    unsigned long long after;
    __asm__ volatile(
        "movabsq $0x5151515151515151, %%rbx\n\t"
        "movabsq $0x1212121212121212, %%r12\n\t"
        "movabsq $0x5151515151515151, %%rax\n\t"
        ".globl spin_begin\n"
        "spin_begin:\n\t"
        "pause\n\t"
        "cmpq %%rax, %%rbx\n\t"
        "je spin_begin\n\t"
        ".globl spin_end\n"
        "spin_end:\n\t"
        "movq %%rbx, %0\n\t"
        : "=r"(after)
        :
        : "rax", "rbx", "r12", "memory", "cc");
    spin_rbx_after = after;
    return 0;
}

static CONTEXT ctx;                                     /* 16-byte aligned by its type */

static void suspend(void) {
    HANDLE t = CreateThread(NULL, 0, spin, NULL, 0, NULL);
    Sleep(200);
    DWORD prior = SuspendThread(t);
    check("suspend.prior_count", -1, prior, 0);
    ctx.ContextFlags = CONTEXT_FULL;
    BOOL ok = GetThreadContext(t, &ctx);
    check("suspend.get_ok", -1, ok != 0, 1);
    check("suspend.rbx", OFF(CONTEXT, Rbx), ctx.Rbx, S_RBX);
    check("suspend.r12", OFF(CONTEXT, R12), ctx.R12, S_R12);
    check("suspend.rip_in_loop", OFF(CONTEXT, Rip),
          ctx.Rip >= (DWORD64)(ULONG_PTR)spin_begin && ctx.Rip < (DWORD64)(ULONG_PTR)spin_end, 1);
    ctx.Rbx = S_NEW;
    check("suspend.set_ok", -1, SetThreadContext(t, &ctx) != 0, 1);
    check("suspend.resume_prior_count", -1, ResumeThread(t), 1);
    DWORD w = WaitForSingleObject(t, 5000);
    check("suspend.thread_exited", -1, w, WAIT_OBJECT_0);
    check("suspend.rbx_after_set", -1, spin_rbx_after, S_NEW);
    CloseHandle(t);
}

static volatile unsigned long long counter;
static volatile LONG stores_done;

static DWORD WINAPI __attribute__((noinline)) store_loop(LPVOID p) {
    (void)p;
    __asm__ volatile(
        "movl $200000, %%ecx\n\t"
        "1:\n\t"
        "addq $1, %0\n\t"
        "pushq %%rcx\n\t"
        "popq %%rcx\n\t"
        "decl %%ecx\n\t"
        "jnz 1b\n\t"
        : "+m"(counter)
        :
        : "rcx", "memory", "cc");
    stores_done = 1;
    return 0;
}

static void stores(void) {
    HANDLE t = CreateThread(NULL, 0, store_loop, NULL, 0, NULL);
    unsigned suspensions = 0;
    while (!stores_done && suspensions < 100000) {
        if (SuspendThread(t) != (DWORD)-1) {
            suspensions++;
            ResumeThread(t);
        }
        Sleep(0);
    }
    DWORD w = WaitForSingleObject(t, 10000);
    check("stores.thread_exited", -1, w, WAIT_OBJECT_0);
    check("stores.count", -1, counter, STORES);
    check("stores.suspended_at_least_once", -1, suspensions > 0, 1);
    CloseHandle(t);
}

void start(void) {
    fault();
    suspend();
    stores();
    put("{\"kind\":\"summary\",\"failures\":");
    putu((unsigned long long)failures);
    put("}\n");
    DWORD written;
    WriteFile(GetStdHandle(STD_OUTPUT_HANDLE), out, n, &written, NULL);
    ExitProcess(failures ? 1 : 0);
}
