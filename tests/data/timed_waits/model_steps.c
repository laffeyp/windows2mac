#include <windows.h>
/* Sprint 181: a guest whose code reaches the model, so the run loop's serving of hvc #10 and #11 has a judge. With
 * MXCSR's overflow exception unmasked (0x1b80), ADDSS leaves its fast path for the model (rules/_sse.py 330-335,
 * hvc #10); RCR is a whole model step (hvc #11). Sprint 185 (reviewer-2's Medium 5 of its review of 178-181) adds the
 * batch run's other model classes, each a whole step by hvc #11 (build/sprint-108-run.log 170-177): POPFQ, FSIN,
 * FCOS and XGETBV; and one ADDSS that overflows with the exception unmasked, raised as the guest's #XM, which a
 * vectored handler catches, masking the exception in the context MXCSR so the instruction runs again. 100 rounds,
 * the sum of the results and the handler's exception code printed. */
static void hex(unsigned long long v) { char b[18]; for (int i = 15; i >= 0; i--) { b[i] = "0123456789abcdef"[v & 15]; v >>= 4; } b[16] = '\n'; DWORD w; WriteFile(GetStdHandle(STD_OUTPUT_HANDLE), b, 17, &w, NULL); }
static volatile DWORD caught;
static LONG CALLBACK on_xm(EXCEPTION_POINTERS *e) {
    caught = e->ExceptionRecord->ExceptionCode;
    e->ContextRecord->MxCsr = 0x1f80; /* every exception masked, the flags cleared: ADDSS runs again to infinity */
    return EXCEPTION_CONTINUE_EXECUTION;
}
void start(void) {
    unsigned long long acc = 0;
    unsigned mxcsr = 0x1b80;
    __asm__ volatile("ldmxcsr %0" : : "m"(mxcsr));
    for (unsigned i = 0; i < 100; i++) {
        unsigned a = 0x3f800000u + (i << 16), b = 0x42480000u, sum; /* singles near 1.0, and 50.0 */
        __asm__ volatile("movd %1, %%xmm0\n\tmovd %2, %%xmm1\n\taddss %%xmm1, %%xmm0\n\tmovd %%xmm0, %0"
                         : "=r"(sum) : "r"(a), "r"(b) : "xmm0", "xmm1");
        unsigned r = i * 2654435761u;
        __asm__ volatile("stc\n\trcr $3, %0" : "+r"(r) : : "cc");
        unsigned long long f;                                  /* POPFQ: CF from bit 0 of i, then read by SETC */
        unsigned char c;
        __asm__ volatile("pushfq\n\tpopq %0\n\tandq $-2, %0\n\torq %2, %0\n\tpushq %0\n\tpopfq\n\tsetc %1"
                         : "=&r"(f), "=r"(c) : "r"((unsigned long long)(i & 1)) : "cc");
        double x = i * 0.0625, s, co;                          /* FSIN and FCOS on x87 */
        __asm__ volatile("fldl %2\n\tfsin\n\tfstpl %0\n\tfldl %2\n\tfcos\n\tfstpl %1" : "=m"(s), "=m"(co) : "m"(x));
        unsigned long long sb, cb;
        __builtin_memcpy(&sb, &s, 8);
        __builtin_memcpy(&cb, &co, 8);
        unsigned lo, hi;                                       /* XGETBV: XCR0 */
        __asm__ volatile("xgetbv" : "=a"(lo), "=d"(hi) : "c"(0));
        acc = acc * 31 + sum + r + c + sb + cb + lo + ((unsigned long long)hi << 32);
    }
    AddVectoredExceptionHandler(1, on_xm);
    unsigned big = 0x7f7fffffu, inf;                           /* the largest single, doubled: overflow */
    __asm__ volatile("movd %1, %%xmm0\n\taddss %%xmm0, %%xmm0\n\tmovd %%xmm0, %0" : "=r"(inf) : "r"(big) : "xmm0");
    hex(acc);
    hex(((unsigned long long)caught << 32) | inf);
    ExitProcess(0);
}
