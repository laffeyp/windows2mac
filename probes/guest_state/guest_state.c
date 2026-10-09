/* guest_state.c — Windows-visible thread and process state, measured on Windows (Sprint 034).
 *
 * Prints one JSON object per line:
 *   {"kind":"layout","name":...,"value":...}  offsetof/sizeof from the SDK headers (winnt.h CONTEXT,
 *                                              winternl.h TEB and PEB)
 *   {"kind":"check","name":...,"offset":...,"observed":...,"expected":...,"equal":...}
 *       teb.*     the running thread's TEB read through gs: against the API reporting the same value
 *       fault.*   CONTEXT seen by a vectored handler (registered first, as Mono does) for an access
 *                 violation with known register values; the handler moves RIP past the faulting
 *                 instruction and sets RCX, and the probe checks both took effect
 *       suspend.* GetThreadContext/SetThreadContext on a suspended thread with known registers
 * Exit code 0 when every check is equal, 1 otherwise. Uses the C runtime: the probe runs only on
 * Windows, never in w2m.
 */
#include <windows.h>
#include <winternl.h>
#include <intrin.h>
#include <stddef.h>
#include <stdio.h>

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

static int failures;

static void layout(const char *name, size_t v) {
    printf("{\"kind\":\"layout\",\"name\":\"%s\",\"value\":%zu}\n", name, v);
}

static void check(const char *name, long long offset, unsigned long long observed,
                  unsigned long long expected) {
    int eq = observed == expected;
    failures += !eq;
    printf("{\"kind\":\"check\",\"name\":\"%s\",\"offset\":%lld,\"observed\":%llu,\"expected\":%llu,"
           "\"equal\":%s}\n", name, offset, observed, expected, eq ? "true" : "false");
}

#define L(type, field) layout(#type "." #field, offsetof(type, field))

static void layouts(void) {
    layout("CONTEXT.sizeof", sizeof(CONTEXT));
    L(CONTEXT, ContextFlags); L(CONTEXT, MxCsr); L(CONTEXT, SegCs); L(CONTEXT, SegGs);
    L(CONTEXT, EFlags); L(CONTEXT, Dr0); L(CONTEXT, Rax); L(CONTEXT, Rcx); L(CONTEXT, Rdx);
    L(CONTEXT, Rbx); L(CONTEXT, Rsp); L(CONTEXT, Rbp); L(CONTEXT, Rsi); L(CONTEXT, Rdi);
    L(CONTEXT, R8); L(CONTEXT, R9); L(CONTEXT, R10); L(CONTEXT, R11); L(CONTEXT, R12);
    L(CONTEXT, R13); L(CONTEXT, R14); L(CONTEXT, R15); L(CONTEXT, Rip); L(CONTEXT, FltSave);
    L(CONTEXT, Xmm0); L(CONTEXT, Xmm6); L(CONTEXT, Xmm15); L(CONTEXT, VectorRegister);
    L(CONTEXT, VectorControl); L(CONTEXT, DebugControl); L(CONTEXT, LastExceptionToRip);
    layout("XMM_SAVE_AREA32.sizeof", sizeof(XMM_SAVE_AREA32));
    L(XMM_SAVE_AREA32, ControlWord); L(XMM_SAVE_AREA32, StatusWord); L(XMM_SAVE_AREA32, TagWord);
    L(XMM_SAVE_AREA32, MxCsr); L(XMM_SAVE_AREA32, FloatRegisters); L(XMM_SAVE_AREA32, XmmRegisters);
    layout("EXCEPTION_RECORD.sizeof", sizeof(EXCEPTION_RECORD));
    L(EXCEPTION_RECORD, ExceptionCode); L(EXCEPTION_RECORD, ExceptionFlags);
    L(EXCEPTION_RECORD, ExceptionAddress); L(EXCEPTION_RECORD, NumberParameters);
    L(EXCEPTION_RECORD, ExceptionInformation);
    L(TEB, ProcessEnvironmentBlock); L(TEB, TlsSlots); L(TEB, TlsExpansionSlots);
    L(PEB, BeingDebugged); L(PEB, Ldr); L(PEB, ProcessParameters); L(PEB, SessionId);
}

static void teb(void) {
    unsigned char *self = (unsigned char *)__readgsqword(0x30);
    check("teb.self", 0x30, (ULONG_PTR)self, (ULONG_PTR)NtCurrentTeb());
    PEB *peb = (PEB *)__readgsqword(0x60);
    check("teb.peb", 0x60, (ULONG_PTR)peb, (ULONG_PTR)NtCurrentTeb()->ProcessEnvironmentBlock);
    check("teb.client_id.process", 0x40, __readgsqword(0x40), GetCurrentProcessId());
    check("teb.client_id.thread", 0x48, __readgsqword(0x48), GetCurrentThreadId());
    SetLastError(0x1234ABCD);
    check("teb.last_error", 0x68, __readgsdword(0x68), 0x1234ABCD);
    ULONG_PTR lo, hi;
    GetCurrentThreadStackLimits(&lo, &hi);
    check("teb.stack_base", 0x08, __readgsqword(0x08), hi);
    ULONG_PTR limit = __readgsqword(0x10);                      /* committed limit: within the reserve */
    check("teb.stack_limit_in_reserve", 0x10, limit >= lo && limit < hi, 1);
    DWORD low = TLS_OUT_OF_INDEXES, high = TLS_OUT_OF_INDEXES;
    for (int i = 0; i < 80 && high == TLS_OUT_OF_INDEXES; i++) {
        DWORD t = TlsAlloc();
        if (t < 64 && low == TLS_OUT_OF_INDEXES) low = t;
        if (t >= 64) high = t;
    }
    TlsSetValue(low, (LPVOID)0xA1A1A1A1A1A1ULL);
    TlsSetValue(high, (LPVOID)0xB2B2B2B2B2B2ULL);
    check("teb.tls_slot", 0x1480 + 8LL * low, __readgsqword(0x1480 + 8 * low), (ULONG_PTR)TlsGetValue(low));
    void **exp = (void **)__readgsqword(0x1780);
    check("teb.tls_expansion_slot", 0x1780, exp ? (ULONG_PTR)exp[high - 64] : 0, (ULONG_PTR)TlsGetValue(high));
    check("teb.tls_expansion_index_ge_64", -1, high >= 64, 1);
    check("peb.image_base", 0x10, *(ULONG_PTR *)((unsigned char *)peb + 0x10), (ULONG_PTR)GetModuleHandleW(NULL));
}

static volatile LONG handled;
static CONTEXT seen;
static EXCEPTION_RECORD seen_record;
extern char fault_insn[];

static LONG CALLBACK on_fault(EXCEPTION_POINTERS *ep) {
    if (ep->ExceptionRecord->ExceptionCode != EXCEPTION_ACCESS_VIOLATION ||
        ep->ContextRecord->Rip != (DWORD64)(ULONG_PTR)fault_insn)
        return EXCEPTION_CONTINUE_SEARCH;
    seen = *ep->ContextRecord;
    seen_record = *ep->ExceptionRecord;
    handled = 1;
    ep->ContextRecord->Rip += 3;                                /* mov rcx, [rax]: 48 8b 08 */
    ep->ContextRecord->Rcx = K_MARK;
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
        "movq %%rbx, %%xmm6\n\t"
        "xorl %%eax, %%eax\n\t"
        "xorl %%ecx, %%ecx\n\t"
        ".globl fault_insn\n"
        "fault_insn:\n\t"
        "movq (%%rax), %%rcx\n\t"
        "movq %%rcx, %0\n\t"
        : "=m"(marker)
        :
        : "rax", "rbx", "rcx", "rsi", "rdi", "r12", "r13", "r14", "r15", "xmm6", "memory");
    RemoveVectoredExceptionHandler(h);
    check("fault.handled", -1, handled, 1);
    check("fault.code", offsetof(EXCEPTION_RECORD, ExceptionCode), seen_record.ExceptionCode, EXCEPTION_ACCESS_VIOLATION);
    check("fault.address", offsetof(EXCEPTION_RECORD, ExceptionAddress), (ULONG_PTR)seen_record.ExceptionAddress, (ULONG_PTR)fault_insn);
    check("fault.parameters", offsetof(EXCEPTION_RECORD, NumberParameters), seen_record.NumberParameters, 2);
    check("fault.info0_read", offsetof(EXCEPTION_RECORD, ExceptionInformation), seen_record.ExceptionInformation[0], 0);
    check("fault.info1_address", offsetof(EXCEPTION_RECORD, ExceptionInformation) + 8, seen_record.ExceptionInformation[1], 0);
    check("fault.rip", offsetof(CONTEXT, Rip), seen.Rip, (ULONG_PTR)fault_insn);
    check("fault.rax", offsetof(CONTEXT, Rax), seen.Rax, 0);
    check("fault.rbx", offsetof(CONTEXT, Rbx), seen.Rbx, K_RBX);
    check("fault.rsi", offsetof(CONTEXT, Rsi), seen.Rsi, K_RSI);
    check("fault.rdi", offsetof(CONTEXT, Rdi), seen.Rdi, K_RDI);
    check("fault.r12", offsetof(CONTEXT, R12), seen.R12, K_R12);
    check("fault.r13", offsetof(CONTEXT, R13), seen.R13, K_R13);
    check("fault.r14", offsetof(CONTEXT, R14), seen.R14, K_R14);
    check("fault.r15", offsetof(CONTEXT, R15), seen.R15, K_R15);
    check("fault.xmm6_low", offsetof(CONTEXT, Xmm6), seen.Xmm6.Low, K_RBX);
    check("fault.mxcsr", offsetof(CONTEXT, MxCsr), seen.MxCsr, 0x1F80);
    check("fault.fltsave_mxcsr", offsetof(CONTEXT, FltSave) + offsetof(XMM_SAVE_AREA32, MxCsr), seen.FltSave.MxCsr, 0x1F80);
    check("fault.x87_control_word", offsetof(CONTEXT, FltSave) + offsetof(XMM_SAVE_AREA32, ControlWord), seen.FltSave.ControlWord, 0x27F);
    check("fault.context_flags_full", offsetof(CONTEXT, ContextFlags), (seen.ContextFlags & CONTEXT_FULL) == CONTEXT_FULL, 1);
    check("fault.eflags_if", offsetof(CONTEXT, EFlags), (seen.EFlags >> 9) & 1, 1);
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

static void suspend(void) {
    HANDLE t = CreateThread(NULL, 0, spin, NULL, 0, NULL);
    Sleep(200);
    DWORD prior = SuspendThread(t);
    check("suspend.prior_count", -1, prior, 0);
    CONTEXT c;
    c.ContextFlags = CONTEXT_FULL;
    BOOL ok = GetThreadContext(t, &c);
    check("suspend.get_ok", -1, ok != 0, 1);
    check("suspend.rbx", offsetof(CONTEXT, Rbx), c.Rbx, S_RBX);
    check("suspend.r12", offsetof(CONTEXT, R12), c.R12, S_R12);
    check("suspend.rip_in_loop", offsetof(CONTEXT, Rip),
          c.Rip >= (DWORD64)(ULONG_PTR)spin_begin && c.Rip < (DWORD64)(ULONG_PTR)spin_end, 1);
    c.Rbx = S_NEW;
    check("suspend.set_ok", -1, SetThreadContext(t, &c) != 0, 1);
    ResumeThread(t);
    DWORD w = WaitForSingleObject(t, 5000);
    check("suspend.thread_exited", -1, w, WAIT_OBJECT_0);
    check("suspend.rbx_after_set", -1, spin_rbx_after, S_NEW);
    CloseHandle(t);
}

int main(void) {
    layouts();
    teb();
    fault();
    suspend();
    printf("{\"kind\":\"summary\",\"failures\":%d}\n", failures);
    return failures ? 1 : 0;
}
