// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* semantics.c — run single x86-64 instructions from stated states on a real CPU (Sprint 037;
 * format 2 with MXCSR, XMM0-15, AF, DF, an edge window and the state at a fault, Sprint 039; format 3 adds
 * YMM0-15 bits 255:128, Sprint 049).
 *
 * Usage: semantics.exe <cases.bin> <results.bin>
 *
 * Fixed guest addresses, the same ones scripts/semantics_differential.py gives the model:
 *   code  0x20000000 (RWX)  the instruction, then `jmp [rip]` to collector_nt, then 14 bytes on
 *                           `jmp [rip]` to collector_t; at +0x100 another jump to collector_t
 *                           (the planted RET target)
 *   data  0x30000000 (RW, one page)  64-byte window at +0x40; 32-byte edge window at +0xFE0. The
 *                           page after it is not mapped, so an access straddling 0x30001000 faults
 *   stack 0x40000000 (RW, 64 KB)  64-byte window at +0x87C0; RSP lies in [+0x87E0, +0x87F8], so a
 *                           fault has 34 KB below RSP for Windows to dispatch the exception
 * All three sit below 4 GB so that 32-bit (67-prefixed) addresses reach them.
 *
 * run_case (assembly below) saves the harness's nonvolatile GPRs, XMM6-15 and MXCSR (Windows x64
 * nonvolatile state), loads the case's MXCSR, XMM0-15, RFLAGS and all 16 GPRs (RSP last) and jumps
 * to the instruction. A collector saves RSP first, switches to its own stack, saves RFLAGS before
 * any flag-writing instruction, records which path ran (0 not taken, 1 taken), stores the other
 * GPRs, MXCSR and XMM0-15, restores the harness and returns. A vectored handler turns a fault
 * inside a case into outcome 2: it copies the exception code, RIP, GPRs, EFlags, MxCsr and
 * XMM0-15 from the CONTEXT (the state at the fault) and resumes at fault_return.
 *
 * Case record (little-endian, 600 bytes): u8 length, u8 kind, 6 pad, code[32], regs[16] (RAX..R15
 * in encoding order), rflags, mxcsr (u64), xmm[16][16], data[64], stack[64], edge[32]. File:
 * "W2MS", u32 format 2, u32 count, then records. Result record (584 bytes): regs[16], rflags,
 * marker (0, 1, 2 = fault), fault code, fault rip, mxcsr, xmm[16][16], data[64], stack[64],
 * edge[32]. Format 3 (Sprint 049): each case record has ymm_hi[16][16] (bits 255:128 of YMM0-15) after xmm,
 * and each result record the same after its xmm; run_case inserts them with VINSERTF128 before the XMM loads
 * (legacy MOVDQU leaves bits 255:128 alone) and the collectors extract them with VEXTRACTF128; at a fault they
 * come from the CONTEXT's AVX state (LocateXStateFeature). Format 2 files run without any AVX instruction.
 * Format 4 (Sprint 065): each case record and each result record has a 96-byte x87 block after xmm: FCW, FSW
 * (u16 each), FXSAVE's abridged tag byte, 11 pad, ST0-ST7 in stack order (10 bytes each). run_case saves the
 * harness's FXSAVE image and loads the case's with FXRSTOR before MXCSR and the XMM loads (an image of the
 * harness's own state with the case's FCW, FSW, tags and registers, FOP, FIP and FDP zero); the collectors store
 * FXSAVE; at a fault the state comes from the CONTEXT's FltSave, which has the FXSAVE layout. restore reloads the
 * harness's image. Formats 2 and 3 run without FXSAVE or FXRSTOR.
 */
#include <windows.h>
#include <stdio.h>
#include <string.h>

#define CODE_VA 0x20000000ULL
#define DATA_VA 0x30000000ULL
#define STACK_VA 0x40000000ULL
#define DATA_OFF 0x40
#define EDGE_OFF 0xFE0
#define STACK_OFF 0x87C0
#define WINDOW 64
#define EDGE 32
#define CODE_BYTES 32
#define X87_BLOCK 96

typedef unsigned long long u64;

__attribute__((used)) u64 case_regs[16], case_rflags, out_regs[16], out_rflags, out_marker, out_fault, out_fault_rip;
__attribute__((used)) unsigned int case_mxcsr, out_mxcsr, h_mxcsr;
__attribute__((used, aligned(16))) u64 case_xmm[32], out_xmm[32], h_xmm[20], case_ymmhi[32], out_ymmhi[32];
__attribute__((used)) unsigned char use_ymm, use_x87;
__attribute__((used, aligned(16))) unsigned char case_fx[512], out_fx[512], h_fx[512];
__attribute__((used)) u64 h_rsp, h_rbx, h_rbp, h_rsi, h_rdi, h_r12, h_r13, h_r14, h_r15;
__attribute__((used)) void *code_ptr;
__attribute__((used, aligned(16))) unsigned char flag_stack[256], collect_stack[256];
static volatile LONG in_case;

void run_case(void);
void collector_nt(void);
void collector_t(void);
void fault_return(void);

#define X16(M) M(0) M(1) M(2) M(3) M(4) M(5) M(6) M(7) M(8) M(9) M(10) M(11) M(12) M(13) M(14) M(15)
#define LOAD_XMM(i) "  movdqu case_xmm+16*" #i "(%rip), %xmm" #i "\n"
#define STORE_XMM(i) "  movdqu %xmm" #i ", out_xmm+16*" #i "(%rip)\n"
#define LOAD_YMMHI(i) "  vinsertf128 $1, case_ymmhi+16*" #i "(%rip), %ymm" #i ", %ymm" #i "\n"
#define STORE_YMMHI(i) "  vextractf128 $1, %ymm" #i ", out_ymmhi+16*" #i "(%rip)\n"

__asm__(
    ".text\n"
    ".globl run_case\n"
    "run_case:\n"
    "  movq %rsp, h_rsp(%rip)\n  movq %rbx, h_rbx(%rip)\n  movq %rbp, h_rbp(%rip)\n"
    "  movq %rsi, h_rsi(%rip)\n  movq %rdi, h_rdi(%rip)\n  movq %r12, h_r12(%rip)\n"
    "  movq %r13, h_r13(%rip)\n  movq %r14, h_r14(%rip)\n  movq %r15, h_r15(%rip)\n"
    "  stmxcsr h_mxcsr(%rip)\n"
    "  movdqu %xmm6, h_xmm+0(%rip)\n  movdqu %xmm7, h_xmm+16(%rip)\n  movdqu %xmm8, h_xmm+32(%rip)\n"
    "  movdqu %xmm9, h_xmm+48(%rip)\n  movdqu %xmm10, h_xmm+64(%rip)\n  movdqu %xmm11, h_xmm+80(%rip)\n"
    "  movdqu %xmm12, h_xmm+96(%rip)\n  movdqu %xmm13, h_xmm+112(%rip)\n  movdqu %xmm14, h_xmm+128(%rip)\n"
    "  movdqu %xmm15, h_xmm+144(%rip)\n"
    "  cmpb $0, use_x87(%rip)\n"
    "  je 2f\n"
    "  fxsave64 h_fx(%rip)\n"
    "  fxrstor64 case_fx(%rip)\n"
    "2:\n"
    "  ldmxcsr case_mxcsr(%rip)\n"
    "  cmpb $0, use_ymm(%rip)\n"
    "  je 1f\n"
    X16(LOAD_YMMHI)
    "1:\n"
    X16(LOAD_XMM)
    "  leaq flag_stack+256(%rip), %rsp\n"
    "  pushq case_rflags(%rip)\n"
    "  popfq\n"
    "  movq case_regs+0(%rip), %rax\n  movq case_regs+8(%rip), %rcx\n  movq case_regs+16(%rip), %rdx\n"
    "  movq case_regs+24(%rip), %rbx\n  movq case_regs+40(%rip), %rbp\n  movq case_regs+48(%rip), %rsi\n"
    "  movq case_regs+56(%rip), %rdi\n  movq case_regs+64(%rip), %r8\n  movq case_regs+72(%rip), %r9\n"
    "  movq case_regs+80(%rip), %r10\n  movq case_regs+88(%rip), %r11\n  movq case_regs+96(%rip), %r12\n"
    "  movq case_regs+104(%rip), %r13\n  movq case_regs+112(%rip), %r14\n  movq case_regs+120(%rip), %r15\n"
    "  movq case_regs+32(%rip), %rsp\n"
    "  jmpq *code_ptr(%rip)\n"
    ".globl collector_nt\n"
    "collector_nt:\n"
    "  movq %rsp, out_regs+32(%rip)\n"
    "  leaq collect_stack+256(%rip), %rsp\n"
    "  pushfq\n"
    "  movq $0, out_marker(%rip)\n"
    "  jmp collect_common\n"
    ".globl collector_t\n"
    "collector_t:\n"
    "  movq %rsp, out_regs+32(%rip)\n"
    "  leaq collect_stack+256(%rip), %rsp\n"
    "  pushfq\n"
    "  movq $1, out_marker(%rip)\n"
    "collect_common:\n"
    "  popq out_rflags(%rip)\n"
    "  movq %rax, out_regs+0(%rip)\n  movq %rcx, out_regs+8(%rip)\n  movq %rdx, out_regs+16(%rip)\n"
    "  movq %rbx, out_regs+24(%rip)\n  movq %rbp, out_regs+40(%rip)\n  movq %rsi, out_regs+48(%rip)\n"
    "  movq %rdi, out_regs+56(%rip)\n  movq %r8, out_regs+64(%rip)\n  movq %r9, out_regs+72(%rip)\n"
    "  movq %r10, out_regs+80(%rip)\n  movq %r11, out_regs+88(%rip)\n  movq %r12, out_regs+96(%rip)\n"
    "  movq %r13, out_regs+104(%rip)\n  movq %r14, out_regs+112(%rip)\n  movq %r15, out_regs+120(%rip)\n"
    "  stmxcsr out_mxcsr(%rip)\n"
    X16(STORE_XMM)
    "  cmpb $0, use_x87(%rip)\n"
    "  je 4f\n"
    "  fxsave64 out_fx(%rip)\n"
    "4:\n"
    "  cmpb $0, use_ymm(%rip)\n"
    "  je restore\n"
    X16(STORE_YMMHI)
    "restore:\n"
    "  cmpb $0, use_x87(%rip)\n"
    "  je 5f\n"
    "  fxrstor64 h_fx(%rip)\n"
    "5:\n"
    "  movq h_rsp(%rip), %rsp\n  movq h_rbx(%rip), %rbx\n  movq h_rbp(%rip), %rbp\n"
    "  movq h_rsi(%rip), %rsi\n  movq h_rdi(%rip), %rdi\n  movq h_r12(%rip), %r12\n"
    "  movq h_r13(%rip), %r13\n  movq h_r14(%rip), %r14\n  movq h_r15(%rip), %r15\n"
    "  ldmxcsr h_mxcsr(%rip)\n"
    "  movdqu h_xmm+0(%rip), %xmm6\n  movdqu h_xmm+16(%rip), %xmm7\n  movdqu h_xmm+32(%rip), %xmm8\n"
    "  movdqu h_xmm+48(%rip), %xmm9\n  movdqu h_xmm+64(%rip), %xmm10\n  movdqu h_xmm+80(%rip), %xmm11\n"
    "  movdqu h_xmm+96(%rip), %xmm12\n  movdqu h_xmm+112(%rip), %xmm13\n  movdqu h_xmm+128(%rip), %xmm14\n"
    "  movdqu h_xmm+144(%rip), %xmm15\n"
    "  cmpb $0, use_ymm(%rip)\n"
    "  je 3f\n"
    "  vzeroupper\n"
    "3:\n"
    "  cld\n"
    "  retq\n"
    ".globl fault_return\n"
    "fault_return:\n"
    "  movq $2, out_marker(%rip)\n"
    "  jmp restore\n");

static LONG CALLBACK on_fault(EXCEPTION_POINTERS *ep) {
    if (!in_case) return EXCEPTION_CONTINUE_SEARCH;
    CONTEXT *c = ep->ContextRecord;
    out_fault = ep->ExceptionRecord->ExceptionCode;
    out_fault_rip = c->Rip;
    memcpy(out_regs, &c->Rax, sizeof out_regs);              /* Rax..R15: encoding order in CONTEXT */
    out_rflags = c->EFlags;
    out_mxcsr = c->MxCsr;
    for (int i = 0; i < 16; i++) {
        out_xmm[2 * i] = c->FltSave.XmmRegisters[i].Low;
        out_xmm[2 * i + 1] = (u64)c->FltSave.XmmRegisters[i].High;
    }
    if (use_x87) memcpy(out_fx, &c->FltSave, sizeof out_fx);      /* XSAVE_FORMAT: the FXSAVE image at the fault */
    if (use_ymm) {
        DWORD len = 0;
        void *avx = LocateXStateFeature(c, XSTATE_AVX, &len);
        if (avx && len >= sizeof out_ymmhi) memcpy(out_ymmhi, avx, sizeof out_ymmhi);
        else memset(out_ymmhi, 0xEE, sizeof out_ymmhi);           /* no AVX state in the CONTEXT: a visible mark */
    }
    c->Rip = (DWORD64)(ULONG_PTR)fault_return;
    c->EFlags &= ~0x400u;                                     /* DF clear for the harness */
    return EXCEPTION_CONTINUE_EXECUTION;
}

static void put_jump(unsigned char *at, void *target) {
    static const unsigned char jmp_rip[6] = {0xFF, 0x25, 0, 0, 0, 0};   /* jmp qword [rip+0] */
    memcpy(at, jmp_rip, 6);
    u64 t = (u64)(ULONG_PTR)target;
    memcpy(at + 6, &t, 8);
}

static void *alloc_at(u64 va, SIZE_T size, DWORD protect) {
    void *p = VirtualAlloc((void *)(ULONG_PTR)va, size, MEM_RESERVE | MEM_COMMIT, protect);
    if (p != (void *)(ULONG_PTR)va) {
        fprintf(stderr, "semantics: cannot map 0x%llx (error %lu)\n", va, GetLastError());
        ExitProcess(3);
    }
    return p;
}

int main(int argc, char **argv) {
    if (argc != 3) {
        fprintf(stderr, "usage: semantics.exe <cases.bin> <results.bin>\n");
        return 2;
    }
    FILE *in = fopen(argv[1], "rb"), *out = fopen(argv[2], "wb");
    if (!in || !out) {
        fprintf(stderr, "semantics: cannot open files\n");
        return 2;
    }
    char magic[4];
    unsigned int format, count;
    if (fread(magic, 1, 4, in) != 4 || memcmp(magic, "W2MS", 4) || fread(&format, 4, 1, in) != 1 ||
        format < 2 || format > 4 || fread(&count, 4, 1, in) != 1) {
        fprintf(stderr, "semantics: not a format 2, 3 or 4 case file\n");
        return 2;
    }
    use_ymm = format == 3;
    use_x87 = format == 4;
    static __attribute__((aligned(16))) unsigned char template_fx[512];
    __asm__ volatile("fxsave64 %0" : "=m"(template_fx));             /* the harness's state: MXCSR and the rest valid */
    unsigned char *code = alloc_at(CODE_VA, 0x1000, PAGE_EXECUTE_READWRITE);
    unsigned char *data = alloc_at(DATA_VA, 0x1000, PAGE_READWRITE);
    unsigned char *stack = alloc_at(STACK_VA, 0x10000, PAGE_READWRITE);
    MEMORY_BASIC_INFORMATION mbi;
    if (VirtualQuery((void *)(ULONG_PTR)(DATA_VA + 0x1000), &mbi, sizeof mbi) == 0 || mbi.State == MEM_COMMIT) {
        fprintf(stderr, "semantics: the page after the data page is committed\n");
        return 3;
    }
    AddVectoredExceptionHandler(1, on_fault);
    code_ptr = code;
    unsigned int faults = 0;
    for (unsigned int n = 0; n < count; n++) {
        unsigned char head[8], bytes[CODE_BYTES];
        u64 mxcsr;
        unsigned char x87[X87_BLOCK];
        if (fread(head, 1, 8, in) != 8 || fread(bytes, 1, CODE_BYTES, in) != CODE_BYTES ||
            fread(case_regs, 8, 16, in) != 16 || fread(&case_rflags, 8, 1, in) != 1 ||
            fread(&mxcsr, 8, 1, in) != 1 || fread(case_xmm, 8, 32, in) != 32 ||
            (use_ymm && fread(case_ymmhi, 8, 32, in) != 32) || (use_x87 && fread(x87, 1, X87_BLOCK, in) != X87_BLOCK) ||
            fread(data + DATA_OFF, 1, WINDOW, in) != WINDOW || fread(stack + STACK_OFF, 1, WINDOW, in) != WINDOW ||
            fread(data + EDGE_OFF, 1, EDGE, in) != EDGE) {
            fprintf(stderr, "semantics: case %u truncated\n", n);
            return 2;
        }
        case_mxcsr = (unsigned int)mxcsr;
        if (use_x87) {
            memcpy(case_fx, template_fx, sizeof case_fx);
            memcpy(case_fx + 0, x87 + 0, 2);                       /* FCW */
            memcpy(case_fx + 2, x87 + 2, 2);                       /* FSW */
            case_fx[4] = x87[4];                                   /* abridged FTW */
            memset(case_fx + 6, 0, 18);                            /* FOP, FIP, FDP */
            for (int i = 0; i < 8; i++) {
                memset(case_fx + 32 + 16 * i, 0, 16);
                memcpy(case_fx + 32 + 16 * i, x87 + 16 + 10 * i, 10);
            }
        }
        unsigned len = head[0];
        memset(code, 0xCC, 0x1000);                            /* int3 everywhere else */
        memcpy(code, bytes, len);
        put_jump(code + len, (void *)collector_nt);
        put_jump(code + len + 14, (void *)collector_t);
        put_jump(code + 0x100, (void *)collector_t);
        FlushInstructionCache(GetCurrentProcess(), code, 0x1000);
        memset(out_regs, 0, sizeof out_regs);
        memset(out_xmm, 0, sizeof out_xmm);
        memset(out_ymmhi, 0, sizeof out_ymmhi);
        memset(out_fx, 0, sizeof out_fx);
        out_rflags = out_marker = out_fault = out_fault_rip = 0;
        out_mxcsr = 0;
        in_case = 1;
        run_case();
        in_case = 0;
        faults += out_marker == 2;
        u64 mx = out_mxcsr;
        fwrite(out_regs, 8, 16, out);
        fwrite(&out_rflags, 8, 1, out);
        fwrite(&out_marker, 8, 1, out);
        fwrite(&out_fault, 8, 1, out);
        fwrite(&out_fault_rip, 8, 1, out);
        fwrite(&mx, 8, 1, out);
        fwrite(out_xmm, 8, 32, out);
        if (use_ymm) fwrite(out_ymmhi, 8, 32, out);
        if (use_x87) {
            unsigned char block[X87_BLOCK] = {0};
            memcpy(block, out_fx, 5);                              /* FCW, FSW, abridged FTW */
            for (int i = 0; i < 8; i++) memcpy(block + 16 + 10 * i, out_fx + 32 + 16 * i, 10);
            fwrite(block, 1, X87_BLOCK, out);
        }
        fwrite(data + DATA_OFF, 1, WINDOW, out);
        fwrite(stack + STACK_OFF, 1, WINDOW, out);
        fwrite(data + EDGE_OFF, 1, EDGE, out);
        fflush(out);                                          /* a crash leaves every finished case */
    }
    fclose(out);
    printf("cases %u, faults %u\n", count, faults);
    return 0;
}
