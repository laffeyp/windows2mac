#include <windows.h>
/* Sprint 181: a guest whose code reaches the model, so the run loop's serving of hvc #10 and #11 has a judge. With
 * MXCSR's overflow exception unmasked (0x1b80), ADDSS leaves its fast path for the model (rules/_sse.py 330-335,
 * hvc #10); RCR is a whole model step (hvc #11). 100 rounds of each, no overflow, and the sum of their results
 * printed. */
static void hex(unsigned long long v) { char b[18]; for (int i = 15; i >= 0; i--) { b[i] = "0123456789abcdef"[v & 15]; v >>= 4; } b[16] = '\n'; DWORD w; WriteFile(GetStdHandle(STD_OUTPUT_HANDLE), b, 17, &w, NULL); }
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
        acc = acc * 31 + sum + r;
    }
    hex(acc);
    ExitProcess(0);
}
