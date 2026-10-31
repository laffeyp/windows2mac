// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* x86_decode.h — src/w2m/x86dec.py's decode and src/w2m/x86ops.py's operands in C, for the compiler inside the VM
 * (Sprint 075; BLACKBOARD Decision 2026-10-14, the stencil key). The tables are generated from specs/x86_decode.json
 * by scripts/gen_decoder.py (x86_decode_tables.h). tests/test_x86_decode_c.py holds the C equal to the Python.
 */
#ifndef W2M_X86_DECODE_H
#define W2M_X86_DECODE_H

typedef unsigned char u8;
typedef unsigned short u16;
typedef unsigned int u32;
typedef unsigned long long u64;
typedef signed char s8;
typedef short s16;
typedef long long s64;

/* x86_decode's result: 0, or the exception x86dec.decode raises */
enum x86_status { X86_OK, X86_INVALID_ENCODING, X86_TRUNCATED, X86_DECODE_ERROR, X86_OTHER_ERROR };
enum x86_encoding { X86_LEGACY, X86_VEX, X86_EVEX, X86_XOP };
#define X86_NO_TEMPLATE 0xFFFF

/* x86dec.Fields */
struct x86_fields {
    u8 prefixes[15];
    u8 nprefixes;
    u8 rex;                 /* REX, or VEX's W, R, X, B as a REX byte */
    u8 enc;                 /* enum x86_encoding */
    u8 opcode;
    s16 modrm;              /* -1: none */
    s16 sib;                /* -1: none */
    s64 disp;
    u8 disp_size;
    u8 imm[8];
    u8 imm_size;
    u8 opsize;              /* 16, 32 or 64 */
    u8 asize;               /* 32 or 64 */
    u16 ops;                /* the operand template's index; X86_NO_TEMPLATE */
    s8 vvvv;                /* as encoded; -1: none */
    u8 vl;
};

struct x86_decoded {
    u8 length;
    u16 iclass;             /* an index into the generated iclass names */
    u16 row;                /* the spec row's index in the generated tables */
    struct x86_fields f;
};

/* one operand, as x86ops writes it: R:<reg> M:<seg>:<base>:<index>:<scale>:<disp>:<bytes> A:<base>:<index>:<scale>:<disp>
 * I:<width>:<hex> J:<disp> MM:<n> ST:<n> */
enum x86_operand_kind { X86_OP_R, X86_OP_M, X86_OP_A, X86_OP_I, X86_OP_J, X86_OP_MM, X86_OP_ST };
/* register classes: x86ops' GPR64, GPR32, GPR16, GPR8_REX, GPR8_LEGACY, XMM, YMM; RIP and EIP as a base */
enum x86_register_class { X86_NONE, X86_Q, X86_D, X86_W, X86_B_REX, X86_B_LEGACY, X86_XMM, X86_YMM, X86_RIP, X86_EIP };

struct x86_operand {
    u8 kind;                /* enum x86_operand_kind */
    u8 reg_class, reg;      /* R; MM and ST use reg */
    u8 seg;                 /* M: 0x64 FS, 0x65 GS, 0 none */
    u8 base_class, base;    /* M, A */
    u8 index_class, index;
    u8 scale;
    u8 bytes;               /* M */
    u8 imm_len;             /* I */
    s64 disp;               /* M, A; J's displacement */
    u64 imm;                /* I */
};

/* the template tokens (scripts/gen_decoder.py TOKENS, in x86ops.operands' order of tests) */
enum x86_token { X86_T_MF, X86_T_STI, X86_T_P, X86_T_NQ, X86_T_VX, X86_T_B, X86_T_H, X86_T_HX, X86_T_WX, X86_T_V,
                 X86_T_EDB, X86_T_W, X86_T_E, X86_T_G, X86_T_Z, X86_T_M, X86_T_MCL, X86_T_MQDQ, X86_T_I, X86_T_J,
                 X86_T_ONE, X86_T_AL, X86_T_CL, X86_T_RAX };
/* the size letters (gen_decoder.py LETTERS) */
enum x86_letter { X86_L_B, X86_L_W, X86_L_D, X86_L_Q, X86_L_DQ, X86_L_Z, X86_L_Y, X86_L_V };

struct x86_row {
    u8 pfx, has_modrm, imm, form, lock_ok, vvvv_ones, osz64, np;
    s8 reg, w, l;
    s16 modrm_byte;
    u8 reg_rule;
    u16 reg_valid;
    u16 ops;
    u16 iclass[3];          /* by operand size 16, 32, 64 */
    u8 mn_flags[3];
};

struct x86_template {
    u8 count;
    u8 kind[4];
    u8 arg[4];
};

/* Decode the instruction at the start of b[0..n). */
int x86_decode(const u8 *b, int n, struct x86_decoded *out);
/* Its explicit operands: their count, -1 where the row has no template (OperandError), -2 where x86ops raises
 * otherwise (an assertion or a size it has no entry for). */
int x86_operands(const struct x86_decoded *d, struct x86_operand out[4]);
/* Whether the instruction is an XCHG, the iclass the tables keep apart (`X86_ICLASS_XCHG`; Sprint 272). */
int x86_is_xchg(const struct x86_decoded *d);

#endif
