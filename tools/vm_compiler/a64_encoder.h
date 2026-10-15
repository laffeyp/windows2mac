// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* a64_encoder.h — the AArch64 encoders of src/w2m/a64_encoder.py in C, for the compiler inside the VM (Sprint 073;
 * BLACKBOARD Decision 2026-10-14, copy-and-patch at the encoder level).
 *
 * One function per public function of a64_encoder.py, with its arguments in the same order. Each returns the
 * instruction word, or 0 where the Python raises (a register outside 0-31, a failed assertion, a size or name with
 * no entry); no encoder makes the word 0 (UDF #0). An argument the Python names by a string takes its index in the
 * Python table's order (the enums below); a system register is its five fields. tests/test_a64_encoder_c.py holds
 * every function equal to the Python over a domain per argument.
 */
#ifndef W2M_A64_ENCODER_H
#define W2M_A64_ENCODER_H

typedef unsigned int u32;
typedef unsigned long long u64;

#define A64_ZR 31
#define A64_LSL 0
#define A64_LSR 1

/* COND's values: eq ne cs(hs) cc(lo) mi pl vs vc hi ls ge lt gt le al */
enum a64_cond { A64_EQ, A64_NE, A64_CS, A64_CC, A64_MI, A64_PL, A64_VS, A64_VC, A64_HI, A64_LS, A64_GE, A64_LT,
                A64_GT, A64_LE, A64_AL };
/* THREE_SAME's order */
enum a64_three_same { A64_TS_ADD, A64_TS_SUB, A64_TS_CMEQ, A64_TS_CMGT, A64_TS_UQSUB, A64_TS_SQSUB, A64_TS_SQADD,
                      A64_TS_UMAX, A64_TS_MUL, A64_TS_ZIP1, A64_TS_ZIP2, A64_TS_USHL, A64_TS_SSHL, A64_TS_UQADD,
                      A64_TS_UZP2, A64_TS_UABD, A64_TS_COUNT };
/* _FP2's and _FPV's order */
enum a64_fp_op { A64_FP_ADD, A64_FP_SUB, A64_FP_MUL, A64_FP_DIV, A64_FP_MIN, A64_FP_MAX, A64_FP_COUNT };
/* _FP3's order */
enum a64_fp_fused { A64_FMADD, A64_FMSUB, A64_FNMADD, A64_FNMSUB, A64_FUSED_COUNT };
/* fcm's conditions */
enum a64_fcm { A64_FCM_EQ, A64_FCM_GE, A64_FCM_GT, A64_FCM_COUNT };

int a64_mov_imm(u32 *out, int rd, u64 value);        /* MOVZ, then MOVK per further nonzero halfword: the count */

u32 a64_movz(int rd, int imm16, int hw);
u32 a64_movk(int rd, int imm16, int hw);
u32 a64_add(int rd, int rn, int rm, int shift, int amount);
u32 a64_adds(int rd, int rn, int rm);
u32 a64_sub(int rd, int rn, int rm);
u32 a64_subs(int rd, int rn, int rm);
u32 a64_and(int rd, int rn, int rm);
u32 a64_bic(int rd, int rn, int rm);
u32 a64_orr(int rd, int rn, int rm);
u32 a64_eor(int rd, int rn, int rm, int shift, int amount);
u32 a64_mov(int rd, int rm);
u32 a64_mov32(int rd, int rm);
u32 a64_addi(int rd, int rn, int imm12);
u32 a64_subi(int rd, int rn, int imm12);
u32 a64_addi32(int rd, int rn, int imm12);
u32 a64_subi32(int rd, int rn, int imm12);
u32 a64_adds32(int rd, int rn, int rm, int amount);
u32 a64_subs32(int rd, int rn, int rm, int amount);
u32 a64_lsl32(int rd, int rn, int amount);
u32 a64_and_low(int rd, int rn, int bits);
u32 a64_cmpi(int rn, int imm12);
u32 a64_cmp(int rn, int rm);
u32 a64_ubfm(int rd, int rn, int immr, int imms);
u32 a64_ubfx(int rd, int rn, int lsb, int width);
u32 a64_lsri(int rd, int rn, int shift);
u32 a64_bfi(int rd, int rn, int lsb, int width);
u32 a64_sxtw(int rd, int rn);
u32 a64_udiv(int rd, int rn, int rm);
u32 a64_ldaxp(int rt, int rt2, int rn);
u32 a64_stlxp(int rs, int rt, int rt2, int rn);
u32 a64_sdiv(int rd, int rn, int rm);
u32 a64_msub(int rd, int rn, int rm, int ra);
u32 a64_rev(int rd, int rn, int bits);
u32 a64_clz(int rd, int rn);
u32 a64_rbit(int rd, int rn);
u32 a64_lsrv(int rd, int rn, int rm);
u32 a64_lslv(int rd, int rn, int rm);
u32 a64_asrv(int rd, int rn, int rm);
u32 a64_rorv(int rd, int rn, int rm, int bits);
u32 a64_sbfx(int rd, int rn, int lsb, int width);
u32 a64_smulh(int rd, int rn, int rm);
u32 a64_ldaxr(int size, int rt, int rn);
u32 a64_stlxr(int size, int rs, int rt, int rn);
u32 a64_mul(int rd, int rn, int rm);
u32 a64_umulh(int rd, int rn, int rm);
u32 a64_cset(int rd, int cond);
u32 a64_ldr_reg(int size, int rt, int rn, int rm, int scaled);
u32 a64_str_reg(int size, int rt, int rn, int rm, int scaled);
u32 a64_ldur(int size, int rt, int rn, int offset);
u32 a64_stur(int size, int rt, int rn, int offset);
u32 a64_ldr(int size, int rt, int rn, int offset);
u32 a64_str(int size, int rt, int rn, int offset);
u32 a64_ldp(int rt, int rt2, int rn, int offset);
u32 a64_stp(int rt, int rt2, int rn, int offset);
u32 a64_b(int offset);
u32 a64_bl(int offset);
u32 a64_ret(void);
u32 a64_bcond(int cond, int offset);
u32 a64_cbz(int rt, int offset);
u32 a64_cbnz(int rt, int offset);
u32 a64_tbz(int rt, int bit, int offset);
u32 a64_br(int rn);
u32 a64_blr(int rn);
u32 a64_hvc(int imm16);

u32 a64_ldr_v(int size, int vt, int rn, int offset);
u32 a64_str_v(int size, int vt, int rn, int offset);
u32 a64_stp_q(int vt, int vt2, int rn, int offset);
u32 a64_ldp_q(int vt, int vt2, int rn, int offset);
u32 a64_mov_v(int vd, int vn);
u32 a64_ins_lane0(int size, int vd, int vn);
u32 a64_eor_v(int vd, int vn, int vm);
u32 a64_and_v(int vd, int vn, int vm);
u32 a64_orr_v(int vd, int vn, int vm);
u32 a64_sub_v(int vd, int vn, int vm, int size);
u32 a64_ext_v(int vd, int vn, int vm, int index);
u32 a64_zip1_v(int vd, int vn, int vm);
u32 a64_cmeq_v(int vd, int vn, int vm);
u32 a64_sxtl_2d(int vd, int vn);
u32 a64_scvtf_2d(int vd, int vn);
u32 a64_scvtf_4s(int vd, int vn);
u32 a64_fcvtl_2d(int vd, int vn);
u32 a64_fcvtn_2s(int vd, int vn);
u32 a64_umov_d(int rd, int vn, int lane);
u32 a64_ins_d(int vd, int lane, int rn);
u32 a64_ins_s_element(int vd, int lane, int vn, int src_lane);
u32 a64_movi_zero(int vd);
u32 a64_ushr_2d(int vd, int vn, int shift);
u32 a64_add_v(int vd, int vn, int vm, int size);
u32 a64_bic_v(int vd, int vn, int vm);
u32 a64_zip1_4s(int vd, int vn, int vm);
u32 a64_zip2_4s(int vd, int vn, int vm);
u32 a64_ushr_4s(int vd, int vn, int shift);
u32 a64_sshr_4s(int vd, int vn, int shift);
u32 a64_shl_4s(int vd, int vn, int shift);
u32 a64_dup_4s(int vd, int rn);
u32 a64_ushl_4s(int vd, int vn, int vm);
u32 a64_sshl_4s(int vd, int vn, int vm);
u32 a64_umov_s(int rd, int vn, int lane);
u32 a64_three_same(int name, int vd, int vn, int vm, int size);
u32 a64_three_same_d(int name, int vd, int vn, int vm, int size);
u32 a64_shift_right(int vd, int vn, int shift, int size, int is_signed);
u32 a64_shift_left(int vd, int vn, int shift, int size);
u32 a64_dup_gpr(int vd, int rn, int size);
u32 a64_sqxtun(int vd, int vn, int upper);
u32 a64_sqxtn(int vd, int vn, int upper);
u32 a64_adr(int rd, int offset);
u32 a64_sqxtn_h(int vd, int vn, int upper);
u32 a64_add_pairwise_long(int vd, int vn, int size, int is_signed);
u32 a64_extend_bytes(int vd, int vn, int upper, int is_signed);
u32 a64_umov_h(int rd, int vn, int lane);
u32 a64_smull(int vd, int vn, int vm, int upper);
u32 a64_tbl(int vd, int vn, int vm);
u32 a64_movi_bytes(int vd, int imm8);
u32 a64_ins_h_element(int vd, int lane, int vn, int src_lane);
u32 a64_ins_h(int vd, int lane, int rn);
u32 a64_dmb_ishst(void);
u32 a64_dmb_ishld(void);
u32 a64_dmb_ish(void);
u32 a64_fmov_to_v(int size, int vd, int rn);
u32 a64_fcm(int cond, int is_double, int vd, int vn, int vm);
u32 a64_not_16b(int vd, int vn);
u32 a64_ins_s(int vd, int lane, int rn);
u32 a64_uaddlv_8b(int vd, int vn);
u32 a64_cnt_8b(int vd, int vn);
u32 a64_addv_8b(int vd, int vn);
u32 a64_fmov_from_v(int size, int rd, int vn);
u32 a64_fp_scalar(int op, int size, int vd, int vn, int vm);
u32 a64_fp_fused(int op, int size, int vd, int vn, int vm, int va);
u32 a64_fp_vector(int op, int size, int vd, int vn, int vm);
u32 a64_fsqrt_scalar(int size, int vd, int vn);
u32 a64_fsqrt_vector(int size, int vd, int vn);
u32 a64_fcmp(int size, int vn, int vm, int signal);
u32 a64_fcvtzs(int bits, int size, int rd, int vn);
u32 a64_frintx(int size, int vd, int vn);
u32 a64_scvtf(int size, int bits, int vd, int rn);
u32 a64_fcvt(int dst_size, int src_size, int vd, int vn);
u32 a64_mrs(int rt, int op0, int op1, int crn, int crm, int op2);
u32 a64_msr(int op0, int op1, int crn, int crm, int op2, int rt);

#define A64_ERET 0xD69F03E0u
#define A64_IC_IALLU 0xD508751Fu
#define A64_TLBI_VMALLE1 0xD508871Fu
#define A64_DSB_ISH 0xD5033B9Fu
#define A64_ISB 0xD5033FDFu

#endif
