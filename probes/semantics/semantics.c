/* semantics.c — run single x86-64 instructions from stated states on a real CPU (Sprint 037).
 *
 * Usage: semantics.exe <cases.bin> <results.bin>
 *
 * Fixed guest addresses, the same ones scripts/semantics_differential.py gives the model:
 *   code  0x20000000 (RWX)  the instruction, then `jmp [rip]` to collector_nt, then 14 bytes on
 *                           `jmp [rip]` to collector_t; at +0x100 another jump to collector_t
 *                           (the planted RET target)
 *   data  0x30000000 (RW)   64-byte window at +0x40
 *   stack 0x40000000 (RW, 64 KB)  64-byte window at +0x87C0; RSP lies in [+0x87E0, +0x87F8], so a
 *                           fault has 34 KB below RSP for Windows to dispatch the exception
 * All three sit below 4 GB so that 32-bit (67-prefixed) addresses reach them.
 *
 * run_case (assembly below) saves the harness's nonvolatile registers and RSP, loads RFLAGS from
 * its own small stack, loads all 16 GPRs (RSP last) and jumps to the instruction. A collector
 * saves RSP first, switches to its own stack, saves RFLAGS before any flag-writing instruction,
 * records which path ran (0 not taken, 1 taken), stores the other 15 GPRs, and returns to the
 * harness. A vectored handler turns any fault inside a case into outcome 2 with its code.
 *
 * Case record (little-endian): u8 length, u8 kind, 6 pad, code[32], regs[16] (RAX..R15 in
 * encoding order), rflags, data[64], stack[64]. File: "W2MS", u32 format 1, u32 count, then
 * records. Result record: regs[16], rflags, marker (0, 1, 2 = fault), fault code, data[64],
 * stack[64].
 */
#include <windows.h>
#include <stdio.h>
#include <string.h>

#define CODE_VA 0x20000000ULL
#define DATA_VA 0x30000000ULL
#define STACK_VA 0x40000000ULL
#define DATA_OFF 0x40
#define STACK_OFF 0x87C0
#define WINDOW 64
#define CODE_BYTES 32

typedef unsigned long long u64;

__attribute__((used)) u64 case_regs[16], case_rflags, out_regs[16], out_rflags, out_marker, out_fault;
__attribute__((used)) u64 h_rsp, h_rbx, h_rbp, h_rsi, h_rdi, h_r12, h_r13, h_r14, h_r15;
__attribute__((used)) void *code_ptr;
__attribute__((used, aligned(16))) unsigned char flag_stack[256], collect_stack[256];
static volatile LONG in_case;

void run_case(void);
void collector_nt(void);
void collector_t(void);
void fault_return(void);

__asm__(
    ".text\n"
    ".globl run_case\n"
    "run_case:\n"
    "  movq %rsp, h_rsp(%rip)\n  movq %rbx, h_rbx(%rip)\n  movq %rbp, h_rbp(%rip)\n"
    "  movq %rsi, h_rsi(%rip)\n  movq %rdi, h_rdi(%rip)\n  movq %r12, h_r12(%rip)\n"
    "  movq %r13, h_r13(%rip)\n  movq %r14, h_r14(%rip)\n  movq %r15, h_r15(%rip)\n"
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
    "restore:\n"
    "  movq h_rsp(%rip), %rsp\n  movq h_rbx(%rip), %rbx\n  movq h_rbp(%rip), %rbp\n"
    "  movq h_rsi(%rip), %rsi\n  movq h_rdi(%rip), %rdi\n  movq h_r12(%rip), %r12\n"
    "  movq h_r13(%rip), %r13\n  movq h_r14(%rip), %r14\n  movq h_r15(%rip), %r15\n"
    "  cld\n"
    "  retq\n"
    ".globl fault_return\n"
    "fault_return:\n"
    "  movq $2, out_marker(%rip)\n"
    "  jmp restore\n");

static LONG CALLBACK on_fault(EXCEPTION_POINTERS *ep) {
    if (!in_case) return EXCEPTION_CONTINUE_SEARCH;
    out_fault = ep->ExceptionRecord->ExceptionCode;
    ep->ContextRecord->Rip = (DWORD64)(ULONG_PTR)fault_return;
    ep->ContextRecord->EFlags &= ~0x400u;                      /* DF clear for the harness */
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
        format != 1 || fread(&count, 4, 1, in) != 1) {
        fprintf(stderr, "semantics: not a format 1 case file\n");
        return 2;
    }
    unsigned char *code = alloc_at(CODE_VA, 0x1000, PAGE_EXECUTE_READWRITE);
    unsigned char *data = alloc_at(DATA_VA, 0x1000, PAGE_READWRITE);
    unsigned char *stack = alloc_at(STACK_VA, 0x10000, PAGE_READWRITE);
    AddVectoredExceptionHandler(1, on_fault);
    code_ptr = code;
    unsigned int faults = 0;
    for (unsigned int n = 0; n < count; n++) {
        unsigned char head[8], bytes[CODE_BYTES];
        if (fread(head, 1, 8, in) != 8 || fread(bytes, 1, CODE_BYTES, in) != CODE_BYTES ||
            fread(case_regs, 8, 16, in) != 16 || fread(&case_rflags, 8, 1, in) != 1 ||
            fread(data + DATA_OFF, 1, WINDOW, in) != WINDOW || fread(stack + STACK_OFF, 1, WINDOW, in) != WINDOW) {
            fprintf(stderr, "semantics: case %u truncated\n", n);
            return 2;
        }
        unsigned len = head[0];
        memset(code, 0xCC, 0x1000);                            /* int3 everywhere else */
        memcpy(code, bytes, len);
        put_jump(code + len, (void *)collector_nt);
        put_jump(code + len + 14, (void *)collector_t);
        put_jump(code + 0x100, (void *)collector_t);
        FlushInstructionCache(GetCurrentProcess(), code, 0x1000);
        memset(out_regs, 0, sizeof out_regs);
        out_rflags = out_marker = out_fault = 0;
        in_case = 1;
        run_case();
        in_case = 0;
        faults += out_marker == 2;
        fwrite(out_regs, 8, 16, out);
        fwrite(&out_rflags, 8, 1, out);
        fwrite(&out_marker, 8, 1, out);
        fwrite(&out_fault, 8, 1, out);
        fwrite(data + DATA_OFF, 1, WINDOW, out);
        fwrite(stack + STACK_OFF, 1, WINDOW, out);
    }
    fclose(out);
    printf("cases %u, faults %u\n", count, faults);
    return 0;
}
