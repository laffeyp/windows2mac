// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* a64_encoder.c — src/w2m/a64_encoder.py in C (Sprint 073). Each function follows its Python counterpart line for
 * line: the same checks (each a 0 return where the Python raises) and the same fields. Freestanding; built with the
 * guest runtime's flags. */
#include "a64_encoder.h"

#define NEED(c) do { if (!(c)) return 0; } while (0)
#define R1(a) NEED(reg(a))
#define R2(a, b) NEED(reg(a) && reg(b))
#define R3(a, b, c) NEED(reg(a) && reg(b) && reg(c))
#define R4(a, b, c, d) NEED(reg(a) && reg(b) && reg(c) && reg(d))

static int reg(int r) { return 0 <= r && r <= 31; }
static u32 U(int x) { return (u32)x; }
static int pmod(int a, int m) { int r = a % m; return r < 0 ? r + m : r; }   /* Python's %, m > 0 */

/* the access size's field (_SZ) and the size's floating-point type (_FTYPE): -1 for no entry */
static int size_field(int size) { return size == 1 ? 0 : size == 2 ? 1 : size == 4 ? 2 : size == 8 ? 3 : -1; }
static int ftype(int size) { return size == 4 ? 0 : size == 8 ? 1 : -1; }

u32 a64_movz(int rd, int imm16, int hw) { R1(rd); return 0xD2800000u | U(hw) << 21 | (U(imm16) & 0xFFFF) << 5 | U(rd); }
u32 a64_movk(int rd, int imm16, int hw) { R1(rd); return 0xF2800000u | U(hw) << 21 | (U(imm16) & 0xFFFF) << 5 | U(rd); }

int a64_mov_imm(u32 *out, int rd, u64 value) {
    int n = 0;
    u32 w = a64_movz(rd, (int)(value & 0xFFFF), 0);
    if (!w) return 0;
    out[n++] = w;
    for (int hw = 1; hw < 4; hw++) {
        int h = (int)(value >> (16 * hw) & 0xFFFF);
        if (h) out[n++] = a64_movk(rd, h, hw);
    }
    return n;
}

static u32 sreg(u32 base, int rd, int rn, int rm, int shift, int amount) {
    R3(rd, rn, rm);
    return base | U(shift) << 22 | U(rm) << 16 | (U(amount) & 63) << 10 | U(rn) << 5 | U(rd);
}

u32 a64_add(int rd, int rn, int rm, int shift, int amount) { return sreg(0x8B000000u, rd, rn, rm, shift, amount); }
u32 a64_adds(int rd, int rn, int rm) { return sreg(0xAB000000u, rd, rn, rm, 0, 0); }
u32 a64_sub(int rd, int rn, int rm) { return sreg(0xCB000000u, rd, rn, rm, 0, 0); }
u32 a64_subs(int rd, int rn, int rm) { return sreg(0xEB000000u, rd, rn, rm, 0, 0); }
u32 a64_and(int rd, int rn, int rm) { return sreg(0x8A000000u, rd, rn, rm, 0, 0); }
u32 a64_bic(int rd, int rn, int rm) { return sreg(0x8A200000u, rd, rn, rm, 0, 0); }
u32 a64_orr(int rd, int rn, int rm) { return sreg(0xAA000000u, rd, rn, rm, 0, 0); }
u32 a64_eor(int rd, int rn, int rm, int shift, int amount) { return sreg(0xCA000000u, rd, rn, rm, shift, amount); }
u32 a64_mov(int rd, int rm) { return a64_orr(rd, A64_ZR, rm); }
u32 a64_mov32(int rd, int rm) { R2(rd, rm); return 0x2A0003E0u | U(rm) << 16 | U(rd); }

static u32 imm12_form(u32 base, int rd, int rn, int imm12) {
    R2(rd, rn);
    NEED(0 <= imm12 && imm12 < 4096);
    return base | U(imm12) << 10 | U(rn) << 5 | U(rd);
}

u32 a64_addi(int rd, int rn, int imm12) { return imm12_form(0x91000000u, rd, rn, imm12); }
u32 a64_subi(int rd, int rn, int imm12) { return imm12_form(0xD1000000u, rd, rn, imm12); }
u32 a64_addi32(int rd, int rn, int imm12) { return imm12_form(0x11000000u, rd, rn, imm12); }
u32 a64_subi32(int rd, int rn, int imm12) { return imm12_form(0x51000000u, rd, rn, imm12); }

static u32 flags32_form(u32 base, int rd, int rn, int rm, int amount) {
    R3(rd, rn, rm);
    NEED(0 <= amount && amount < 32);
    return base | U(rm) << 16 | U(amount) << 10 | U(rn) << 5 | U(rd);
}

u32 a64_adds32(int rd, int rn, int rm, int amount) { return flags32_form(0x2B000000u, rd, rn, rm, amount); }
u32 a64_subs32(int rd, int rn, int rm, int amount) { return flags32_form(0x6B000000u, rd, rn, rm, amount); }

u32 a64_lsl32(int rd, int rn, int amount) {
    R2(rd, rn);
    NEED(0 <= amount && amount < 32);
    return 0x53000000u | U((32 - amount) % 32) << 16 | U(31 - amount) << 10 | U(rn) << 5 | U(rd);
}

u32 a64_and_low(int rd, int rn, int bits) {
    R2(rd, rn);
    NEED(1 <= bits && bits < 64);
    return 0x92400000u | U(bits - 1) << 10 | U(rn) << 5 | U(rd);
}

u32 a64_cmpi(int rn, int imm12) {
    R1(rn);
    NEED(0 <= imm12 && imm12 < 4096);
    return 0xF1000000u | U(imm12) << 10 | U(rn) << 5 | A64_ZR;
}

u32 a64_cmp(int rn, int rm) { return a64_subs(A64_ZR, rn, rm); }

u32 a64_ubfm(int rd, int rn, int immr, int imms) {
    R2(rd, rn);
    return 0xD3400000u | U(immr) << 16 | U(imms) << 10 | U(rn) << 5 | U(rd);
}

u32 a64_ubfx(int rd, int rn, int lsb, int width) {
    NEED(0 < width && lsb + width <= 64);
    return a64_ubfm(rd, rn, lsb, lsb + width - 1);
}

u32 a64_lsri(int rd, int rn, int shift) { return a64_ubfm(rd, rn, shift, 63); }

u32 a64_bfi(int rd, int rn, int lsb, int width) {
    R2(rd, rn);
    NEED(0 < width && lsb + width <= 64);
    return 0xB3400000u | U(pmod(-lsb, 64)) << 16 | U(width - 1) << 10 | U(rn) << 5 | U(rd);
}

u32 a64_sxtw(int rd, int rn) { R2(rd, rn); return 0x93407C00u | U(rn) << 5 | U(rd); }
u32 a64_udiv(int rd, int rn, int rm) { R3(rd, rn, rm); return 0x9AC00800u | U(rm) << 16 | U(rn) << 5 | U(rd); }
u32 a64_ldaxp(int rt, int rt2, int rn) { R3(rt, rt2, rn); return 0xC87F8000u | U(rt2) << 10 | U(rn) << 5 | U(rt); }

u32 a64_stlxp(int rs, int rt, int rt2, int rn) {
    R4(rs, rt, rt2, rn);
    return 0xC8208000u | U(rs) << 16 | U(rt2) << 10 | U(rn) << 5 | U(rt);
}

u32 a64_sdiv(int rd, int rn, int rm) { R3(rd, rn, rm); return 0x9AC00C00u | U(rm) << 16 | U(rn) << 5 | U(rd); }

u32 a64_msub(int rd, int rn, int rm, int ra) {
    R4(rd, rn, rm, ra);
    return 0x9B008000u | U(rm) << 16 | U(ra) << 10 | U(rn) << 5 | U(rd);
}

u32 a64_rev(int rd, int rn, int bits) { R2(rd, rn); return (bits == 64 ? 0xDAC00C00u : 0x5AC00800u) | U(rn) << 5 | U(rd); }
u32 a64_clz(int rd, int rn) { R2(rd, rn); return 0xDAC01000u | U(rn) << 5 | U(rd); }
u32 a64_rbit(int rd, int rn) { R2(rd, rn); return 0xDAC00000u | U(rn) << 5 | U(rd); }
u32 a64_lsrv(int rd, int rn, int rm) { R3(rd, rn, rm); return 0x9AC02400u | U(rm) << 16 | U(rn) << 5 | U(rd); }
u32 a64_lslv(int rd, int rn, int rm) { R3(rd, rn, rm); return 0x9AC02000u | U(rm) << 16 | U(rn) << 5 | U(rd); }
u32 a64_asrv(int rd, int rn, int rm) { R3(rd, rn, rm); return 0x9AC02800u | U(rm) << 16 | U(rn) << 5 | U(rd); }

u32 a64_rorv(int rd, int rn, int rm, int bits) {
    R3(rd, rn, rm);
    return (bits == 64 ? 0x9AC02C00u : 0x1AC02C00u) | U(rm) << 16 | U(rn) << 5 | U(rd);
}

u32 a64_sbfx(int rd, int rn, int lsb, int width) {
    R2(rd, rn);
    NEED(0 < width && lsb + width <= 64);
    return 0x93400000u | U(lsb) << 16 | U(lsb + width - 1) << 10 | U(rn) << 5 | U(rd);
}

u32 a64_smulh(int rd, int rn, int rm) { R3(rd, rn, rm); return 0x9B407C00u | U(rm) << 16 | U(rn) << 5 | U(rd); }

u32 a64_ldaxr(int size, int rt, int rn) {
    R2(rt, rn);
    NEED(size_field(size) >= 0);
    return 0x085FFC00u | U(size_field(size)) << 30 | U(rn) << 5 | U(rt);
}

u32 a64_stlxr(int size, int rs, int rt, int rn) {
    R3(rs, rt, rn);
    NEED(size_field(size) >= 0);
    return 0x0800FC00u | U(size_field(size)) << 30 | U(rs) << 16 | U(rn) << 5 | U(rt);
}

u32 a64_mul(int rd, int rn, int rm) { R3(rd, rn, rm); return 0x9B007C00u | U(rm) << 16 | U(rn) << 5 | U(rd); }
u32 a64_umulh(int rd, int rn, int rm) { R3(rd, rn, rm); return 0x9BC07C00u | U(rm) << 16 | U(rn) << 5 | U(rd); }

u32 a64_cset(int rd, int cond) {
    R1(rd);
    NEED(0 <= cond && cond <= A64_AL);
    return 0x9A9F07E0u | U(cond ^ 1) << 12 | U(rd);
}

/* the size's word in a table of the four access sizes 1, 2, 4, 8 */
static u32 by_size(int size, u32 b1, u32 b2, u32 b4, u32 b8) {
    return size == 1 ? b1 : size == 2 ? b2 : size == 4 ? b4 : size == 8 ? b8 : 0;
}

static u32 reg_offset_form(u32 base, int size, int rt, int rn, int rm, int scaled) {
    R3(rt, rn, rm);
    NEED(base);
    return base | U(rm) << 16 | (scaled && size > 1 ? 1u << 12 : 0) | U(rn) << 5 | U(rt);
}

u32 a64_ldr_reg(int size, int rt, int rn, int rm, int scaled) {
    return reg_offset_form(by_size(size, 0x38606800u, 0x78606800u, 0xB8606800u, 0xF8606800u), size, rt, rn, rm, scaled);
}

u32 a64_str_reg(int size, int rt, int rn, int rm, int scaled) {
    return reg_offset_form(by_size(size, 0x38206800u, 0x78206800u, 0xB8206800u, 0xF8206800u), size, rt, rn, rm, scaled);
}

static u32 unscaled_form(u32 base, int rt, int rn, int offset) {
    R2(rt, rn);
    NEED(-256 <= offset && offset <= 255);
    NEED(base);
    return base | (U(offset) & 0x1FF) << 12 | U(rn) << 5 | U(rt);
}

u32 a64_ldur(int size, int rt, int rn, int offset) {
    return unscaled_form(by_size(size, 0x38400000u, 0x78400000u, 0xB8400000u, 0xF8400000u), rt, rn, offset);
}

u32 a64_stur(int size, int rt, int rn, int offset) {
    return unscaled_form(by_size(size, 0x38000000u, 0x78000000u, 0xB8000000u, 0xF8000000u), rt, rn, offset);
}

/* the scaled 12-bit offset form: offset a multiple of size, offset / size in 0..4095 (a zero size raises in Python) */
static u32 scaled_form(u32 base, int size, int rt, int rn, int offset) {
    R2(rt, rn);
    NEED(size != 0 && offset % size == 0 && 0 <= offset / size && offset / size < 4096);
    NEED(base);
    return base | U(offset / size) << 10 | U(rn) << 5 | U(rt);
}

u32 a64_ldr(int size, int rt, int rn, int offset) {
    return scaled_form(by_size(size, 0x39400000u, 0x79400000u, 0xB9400000u, 0xF9400000u), size, rt, rn, offset);
}

u32 a64_str(int size, int rt, int rn, int offset) {
    return scaled_form(by_size(size, 0x39000000u, 0x79000000u, 0xB9000000u, 0xF9000000u), size, rt, rn, offset);
}

/* the signed 7-bit pair offset, scaled by `scale` */
static u32 pair_form(u32 base, int scale, int rt, int rt2, int rn, int offset) {
    R3(rt, rt2, rn);
    NEED(offset % scale == 0 && -64 <= offset / scale && offset / scale < 64);
    return base | (U(offset / scale) & 0x7F) << 15 | U(rt2) << 10 | U(rn) << 5 | U(rt);
}

u32 a64_ldp(int rt, int rt2, int rn, int offset) { return pair_form(0xA9400000u, 8, rt, rt2, rn, offset); }
u32 a64_stp(int rt, int rt2, int rn, int offset) { return pair_form(0xA9000000u, 8, rt, rt2, rn, offset); }

u32 a64_b(int offset) {
    NEED(offset % 4 == 0 && -(1 << 27) <= offset && offset < 1 << 27);
    return 0x14000000u | (U(offset >> 2) & 0x3FFFFFF);
}

u32 a64_bl(int offset) {
    NEED(offset % 4 == 0 && -(1 << 27) <= offset && offset < 1 << 27);
    return 0x94000000u | (U(offset >> 2) & 0x3FFFFFF);
}

u32 a64_ret(void) { return 0xD65F03C0u; }

u32 a64_bcond(int cond, int offset) {
    NEED(0 <= cond && cond <= A64_AL);
    NEED(offset % 4 == 0 && -(1 << 20) <= offset && offset < 1 << 20);
    return 0x54000000u | (U(offset >> 2) & 0x7FFFF) << 5 | U(cond);
}

u32 a64_cbz(int rt, int offset) {
    R1(rt);
    NEED(offset % 4 == 0 && -(1 << 20) <= offset && offset < 1 << 20);
    return 0xB4000000u | (U(offset >> 2) & 0x7FFFF) << 5 | U(rt);
}

u32 a64_cbnz(int rt, int offset) { u32 w = a64_cbz(rt, offset); return w ? w | 1u << 24 : 0; }

u32 a64_tbz(int rt, int bit, int offset) {
    R1(rt);
    NEED(offset % 4 == 0 && -(1 << 15) <= offset && offset < 1 << 15);
    return 0x36000000u | U(bit >> 5) << 31 | (U(bit) & 31) << 19 | (U(offset >> 2) & 0x3FFF) << 5 | U(rt);
}

u32 a64_br(int rn) { R1(rn); return 0xD61F0000u | U(rn) << 5; }
u32 a64_blr(int rn) { R1(rn); return 0xD63F0000u | U(rn) << 5; }
u32 a64_hvc(int imm16) { return 0xD4000002u | (U(imm16) & 0xFFFF) << 5; }

/* ---- SIMD and floating point ---- */

static u32 vector_scaled_form(int size, u32 b4, u32 b8, u32 b16, int vt, int rn, int offset) {
    R2(vt, rn);
    NEED(size != 0 && offset % size == 0 && 0 <= offset / size && offset / size < 4096);
    u32 base = size == 4 ? b4 : size == 8 ? b8 : size == 16 ? b16 : 0;
    NEED(base);
    return base | U(offset / size) << 10 | U(rn) << 5 | U(vt);
}

u32 a64_ldr_v(int size, int vt, int rn, int offset) {
    return vector_scaled_form(size, 0xBD400000u, 0xFD400000u, 0x3DC00000u, vt, rn, offset);
}

u32 a64_str_v(int size, int vt, int rn, int offset) {
    return vector_scaled_form(size, 0xBD000000u, 0xFD000000u, 0x3D800000u, vt, rn, offset);
}

u32 a64_stp_q(int vt, int vt2, int rn, int offset) { return pair_form(0xAD000000u, 16, vt, vt2, rn, offset); }
u32 a64_ldp_q(int vt, int vt2, int rn, int offset) { return pair_form(0xAD400000u, 16, vt, vt2, rn, offset); }

u32 a64_mov_v(int vd, int vn) { R2(vd, vn); return 0x4EA01C00u | U(vn) << 16 | U(vn) << 5 | U(vd); }

u32 a64_ins_lane0(int size, int vd, int vn) {
    R2(vd, vn);
    NEED(size == 4 || size == 8);
    return (size == 4 ? 0x6E040400u : 0x6E080400u) | U(vn) << 5 | U(vd);
}

static u32 three(u32 base, int vd, int vn, int vm) { R3(vd, vn, vm); return base | U(vm) << 16 | U(vn) << 5 | U(vd); }
static u32 two(u32 base, int vd, int vn) { R2(vd, vn); return base | U(vn) << 5 | U(vd); }

u32 a64_eor_v(int vd, int vn, int vm) { return three(0x6E201C00u, vd, vn, vm); }
u32 a64_and_v(int vd, int vn, int vm) { return three(0x4E201C00u, vd, vn, vm); }
u32 a64_orr_v(int vd, int vn, int vm) { return three(0x4EA01C00u, vd, vn, vm); }
u32 a64_sub_v(int vd, int vn, int vm, int size) { R3(vd, vn, vm); return three(0x6E208400u | U(size) << 22, vd, vn, vm); }

u32 a64_ext_v(int vd, int vn, int vm, int index) {
    R3(vd, vn, vm);
    NEED(0 <= index && index < 16);
    return 0x6E000000u | U(vm) << 16 | U(index) << 11 | U(vn) << 5 | U(vd);
}

u32 a64_zip1_v(int vd, int vn, int vm) { return three(0x4E003800u, vd, vn, vm); }
u32 a64_cmeq_v(int vd, int vn, int vm) { return three(0x6E208C00u, vd, vn, vm); }
u32 a64_sxtl_2d(int vd, int vn) { return two(0x0F20A400u, vd, vn); }
u32 a64_scvtf_2d(int vd, int vn) { return two(0x4E61D800u, vd, vn); }
u32 a64_scvtf_4s(int vd, int vn) { return two(0x4E21D800u, vd, vn); }
u32 a64_fcvtl_2d(int vd, int vn) { return two(0x0E617800u, vd, vn); }
u32 a64_fcvtn_2s(int vd, int vn) { return two(0x0E616800u, vd, vn); }

u32 a64_umov_d(int rd, int vn, int lane) { R2(rd, vn); return 0x4E003C00u | (U(lane) << 4 | 0x8) << 16 | U(vn) << 5 | U(rd); }
u32 a64_ins_d(int vd, int lane, int rn) { R2(vd, rn); return 0x4E001C00u | (U(lane) << 4 | 0x8) << 16 | U(rn) << 5 | U(vd); }

u32 a64_ins_s_element(int vd, int lane, int vn, int src_lane) {
    R2(vd, vn);
    return 0x6E000400u | (U(lane) << 3 | 0x4) << 16 | (U(src_lane) << 2) << 11 | U(vn) << 5 | U(vd);
}

u32 a64_movi_zero(int vd) { R1(vd); return 0x6F00E400u | U(vd); }

u32 a64_ushr_2d(int vd, int vn, int shift) {
    R2(vd, vn);
    NEED(1 <= shift && shift <= 64);
    return 0x6F400400u | U(128 - shift) << 16 | U(vn) << 5 | U(vd);
}

u32 a64_add_v(int vd, int vn, int vm, int size) { R3(vd, vn, vm); return three(0x4E208400u | U(size) << 22, vd, vn, vm); }
u32 a64_bic_v(int vd, int vn, int vm) { return three(0x4E601C00u, vd, vn, vm); }
u32 a64_zip1_4s(int vd, int vn, int vm) { return three(0x4E803800u, vd, vn, vm); }
u32 a64_zip2_4s(int vd, int vn, int vm) { return three(0x4E807800u, vd, vn, vm); }

u32 a64_ushr_4s(int vd, int vn, int shift) {
    R2(vd, vn);
    NEED(1 <= shift && shift <= 32);
    return 0x6F000400u | U(64 - shift) << 16 | U(vn) << 5 | U(vd);
}

u32 a64_sshr_4s(int vd, int vn, int shift) {
    R2(vd, vn);
    NEED(1 <= shift && shift <= 32);
    return 0x4F000400u | U(64 - shift) << 16 | U(vn) << 5 | U(vd);
}

u32 a64_shl_4s(int vd, int vn, int shift) {
    R2(vd, vn);
    NEED(0 <= shift && shift <= 31);
    return 0x4F005400u | U(32 + shift) << 16 | U(vn) << 5 | U(vd);
}

u32 a64_dup_4s(int vd, int rn) { return two(0x4E040C00u, vd, rn); }
u32 a64_ushl_4s(int vd, int vn, int vm) { return three(0x6EA04400u, vd, vn, vm); }
u32 a64_sshl_4s(int vd, int vn, int vm) { return three(0x4EA04400u, vd, vn, vm); }
u32 a64_umov_s(int rd, int vn, int lane) { R2(rd, vn); return 0x0E003C00u | (U(lane) << 3 | 0x4) << 16 | U(vn) << 5 | U(rd); }

static const u32 THREE_SAME[A64_TS_COUNT] = {
    0x4E208400u, 0x6E208400u, 0x6E208C00u, 0x4E203400u, 0x6E202C00u, 0x4E202C00u, 0x4E200C00u, 0x6E206400u,
    0x4E209C00u, 0x4E003800u, 0x4E007800u, 0x6E204400u, 0x4E204400u, 0x6E200C00u, 0x4E005800u, 0x6E207400u,
};

u32 a64_three_same(int name, int vd, int vn, int vm, int size) {
    R3(vd, vn, vm);
    NEED(0 <= name && name < A64_TS_COUNT);
    return THREE_SAME[name] | U(size) << 22 | U(vm) << 16 | U(vn) << 5 | U(vd);
}

u32 a64_three_same_d(int name, int vd, int vn, int vm, int size) {
    NEED(size < 3);
    u32 w = a64_three_same(name, vd, vn, vm, size);
    return w & ~(1u << 30);
}

/* 8 << size, the lane's bits; a negative size raises in Python, and one past 24 has no u32 result there */
static int lane_bits(int size) { return 0 <= size && size <= 24 ? 8 << size : 0; }

u32 a64_shift_right(int vd, int vn, int shift, int size, int is_signed) {
    R2(vd, vn);
    int bits = lane_bits(size);
    NEED(bits && 1 <= shift && shift <= bits && size >= 1);
    return (is_signed ? 0x4F000400u : 0x6F000400u) | U(2 * bits - shift) << 16 | U(vn) << 5 | U(vd);
}

u32 a64_shift_left(int vd, int vn, int shift, int size) {
    R2(vd, vn);
    int bits = lane_bits(size);
    NEED(bits && 0 <= shift && shift < bits && size >= 1);
    return 0x4F005400u | U(bits + shift) << 16 | U(vn) << 5 | U(vd);
}

u32 a64_dup_gpr(int vd, int rn, int size) {
    R2(vd, rn);
    NEED(0 <= size && size < 32);
    return 0x4E000C00u | (1u << size) << 16 | U(rn) << 5 | U(vd);
}

u32 a64_sqxtun(int vd, int vn, int upper) { return two(upper ? 0x6E212800u : 0x2E212800u, vd, vn); }
u32 a64_sqxtn(int vd, int vn, int upper) { return two(upper ? 0x4E214800u : 0x0E214800u, vd, vn); }

u32 a64_adr(int rd, int offset) {
    R1(rd);
    NEED(-(1 << 20) <= offset && offset < 1 << 20);
    return 0x10000000u | (U(offset) & 3) << 29 | (U(offset >> 2) & 0x7FFFF) << 5 | U(rd);
}

u32 a64_sqxtn_h(int vd, int vn, int upper) { return two(upper ? 0x4E614800u : 0x0E614800u, vd, vn); }

u32 a64_add_pairwise_long(int vd, int vn, int size, int is_signed) {
    R2(vd, vn);
    NEED(size < 3);
    return (is_signed ? 0x4E202800u : 0x6E202800u) | U(size) << 22 | U(vn) << 5 | U(vd);
}

u32 a64_extend_bytes(int vd, int vn, int upper, int is_signed) {
    return two((is_signed ? 0x0F08A400u : 0x2F08A400u) | (upper ? 1u << 30 : 0), vd, vn);
}

u32 a64_umov_h(int rd, int vn, int lane) {
    R2(rd, vn);
    NEED(0 <= lane && lane < 8);
    return 0x0E003C00u | (U(lane) << 2 | 0x2) << 16 | U(vn) << 5 | U(rd);
}

u32 a64_smull(int vd, int vn, int vm, int upper) { return three(upper ? 0x4E60C000u : 0x0E60C000u, vd, vn, vm); }
u32 a64_tbl(int vd, int vn, int vm) { return three(0x4E000000u, vd, vn, vm); }

u32 a64_movi_bytes(int vd, int imm8) {
    R1(vd);
    return 0x4F00E400u | U(imm8 >> 5) << 16 | (U(imm8) & 31) << 5 | U(vd);
}

u32 a64_ins_h_element(int vd, int lane, int vn, int src_lane) {
    R2(vd, vn);
    return 0x6E000400u | (U(lane) << 2 | 0x2) << 16 | (U(src_lane) << 1) << 11 | U(vn) << 5 | U(vd);
}

u32 a64_ins_h(int vd, int lane, int rn) { R2(vd, rn); return 0x4E001C00u | (U(lane) << 2 | 0x2) << 16 | U(rn) << 5 | U(vd); }

u32 a64_dmb_ishst(void) { return 0xD5033ABFu; }
u32 a64_dmb_ishld(void) { return 0xD50339BFu; }
u32 a64_dmb_ish(void) { return 0xD5033BBFu; }

u32 a64_fmov_to_v(int size, int vd, int rn) {
    R2(vd, rn);
    NEED(size == 4 || size == 8);
    return (size == 4 ? 0x1E270000u : 0x9E670000u) | U(rn) << 5 | U(vd);
}

static const u32 FCM[A64_FCM_COUNT] = {0x4E20E400u, 0x6E20E400u, 0x6EA0E400u};

u32 a64_fcm(int cond, int is_double, int vd, int vn, int vm) {
    R3(vd, vn, vm);
    NEED(0 <= cond && cond < A64_FCM_COUNT);
    return FCM[cond] | U(is_double != 0) << 22 | U(vm) << 16 | U(vn) << 5 | U(vd);
}

u32 a64_not_16b(int vd, int vn) { return two(0x6E205800u, vd, vn); }

u32 a64_ins_s(int vd, int lane, int rn) {
    R2(vd, rn);
    NEED(0 <= lane && lane < 4);
    return 0x4E001C00u | (U(lane) << 3 | 4) << 16 | U(rn) << 5 | U(vd);
}

u32 a64_uaddlv_8b(int vd, int vn) { return two(0x2E303800u, vd, vn); }
u32 a64_cnt_8b(int vd, int vn) { return two(0x0E205800u, vd, vn); }
u32 a64_addv_8b(int vd, int vn) { return two(0x0E31B800u, vd, vn); }

u32 a64_fmov_from_v(int size, int rd, int vn) {
    R2(rd, vn);
    NEED(size == 4 || size == 8);
    return (size == 4 ? 0x1E260000u : 0x9E660000u) | U(vn) << 5 | U(rd);
}

static const u32 FP2[A64_FP_COUNT] = {0x1E202800u, 0x1E203800u, 0x1E200800u, 0x1E201800u, 0x1E205800u, 0x1E204800u};

u32 a64_fp_scalar(int op, int size, int vd, int vn, int vm) {
    R3(vd, vn, vm);
    NEED(0 <= op && op < A64_FP_COUNT && ftype(size) >= 0);
    return FP2[op] | U(ftype(size)) << 22 | U(vm) << 16 | U(vn) << 5 | U(vd);
}

u32 a64_fp_fused(int op, int size, int vd, int vn, int vm, int va) {
    R4(vd, vn, vm, va);
    NEED(0 <= op && op < A64_FUSED_COUNT && ftype(size) >= 0);
    u32 o1 = U(op) >> 1, o0 = U(op) & 1;
    return 0x1F000000u | U(ftype(size)) << 22 | o1 << 21 | U(vm) << 16 | o0 << 15 | U(va) << 10 | U(vn) << 5 | U(vd);
}

static const u32 FPV[A64_FP_COUNT] = {0x4E20D400u, 0x4EA0D400u, 0x6E20DC00u, 0x6E20FC00u, 0x4EA0F400u, 0x4E20F400u};

u32 a64_fp_vector(int op, int size, int vd, int vn, int vm) {
    R3(vd, vn, vm);
    NEED(0 <= op && op < A64_FP_COUNT);
    return FPV[op] | U(size == 8) << 22 | U(vm) << 16 | U(vn) << 5 | U(vd);
}

u32 a64_fsqrt_scalar(int size, int vd, int vn) {
    R2(vd, vn);
    NEED(ftype(size) >= 0);
    return 0x1E21C000u | U(ftype(size)) << 22 | U(vn) << 5 | U(vd);
}

u32 a64_fsqrt_vector(int size, int vd, int vn) { R2(vd, vn); return 0x6EA1F800u | U(size == 8) << 22 | U(vn) << 5 | U(vd); }

u32 a64_fcmp(int size, int vn, int vm, int signal) {
    R2(vn, vm);
    NEED(ftype(size) >= 0);
    return 0x1E202000u | U(ftype(size)) << 22 | U(vm) << 16 | U(vn) << 5 | (signal ? 0x10u : 0);
}

u32 a64_fcvtzs(int bits, int size, int rd, int vn) {
    R2(rd, vn);
    NEED(ftype(size) >= 0);
    return 0x1E380000u | U(bits == 64) << 31 | U(ftype(size)) << 22 | U(vn) << 5 | U(rd);
}

u32 a64_frintx(int size, int vd, int vn) {
    R2(vd, vn);
    NEED(ftype(size) >= 0);
    return 0x1E274000u | U(ftype(size)) << 22 | U(vn) << 5 | U(vd);
}

u32 a64_scvtf(int size, int bits, int vd, int rn) {
    R2(vd, rn);
    NEED(ftype(size) >= 0);
    return 0x1E220000u | U(bits == 64) << 31 | U(ftype(size)) << 22 | U(rn) << 5 | U(vd);
}

u32 a64_fcvt(int dst_size, int src_size, int vd, int vn) {
    R2(vd, vn);
    if (dst_size == 8 && src_size == 4) return 0x1E22C000u | U(vn) << 5 | U(vd);
    if (dst_size == 4 && src_size == 8) return 0x1E624000u | U(vn) << 5 | U(vd);
    return 0;
}

static u32 sysreg(int op0, int op1, int crn, int crm, int op2) {
    return U(op0 - 2) << 19 | U(op1) << 16 | U(crn) << 12 | U(crm) << 8 | U(op2) << 5;
}

u32 a64_mrs(int rt, int op0, int op1, int crn, int crm, int op2) {
    R1(rt);
    return 0xD5300000u | sysreg(op0, op1, crn, crm, op2) | U(rt);
}

u32 a64_msr(int op0, int op1, int crn, int crm, int op2, int rt) {
    R1(rt);
    return 0xD5100000u | sysreg(op0, op1, crn, crm, op2) | U(rt);
}
