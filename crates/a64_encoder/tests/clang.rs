// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! Sprint 135's judge, carried from `tests/test_a64_encoder.py`: every case's word equals clang's assembly of its text
//! (`--target=aarch64-linux-gnu`, read back with llvm-objdump), the Arm ARM transcription checked by an independent
//! assembler. The cases are the Python test's, transcribed with the defaults it left out written in, then the fixed
//! words the monitor code uses that the Python test did not check. Runs where clang and llvm-objdump are found.

use std::path::PathBuf;
use std::process::Command;

use a64_encoder::*;

fn cases() -> Vec<(&'static str, u32)> {
    vec![
        ("movz x3, #0x1234", movz(3, 0x1234, 0)),
        ("movz x17, #0xffff, lsl #48", movz(17, 0xFFFF, 3)),
        ("movk x17, #0xbeef, lsl #16", movk(17, 0xBEEF, 1)),
        ("movk x0, #0x7ffe, lsl #32", movk(0, 0x7FFE, 2)),
        ("add x1, x2, x3", add(1, 2, 3, LSL, 0)),
        ("add x16, x16, x18, lsl #4", add(16, 16, 18, LSL, 4)),
        ("add x25, x5, x15, lsl #3", add(25, 5, 15, LSL, 3)),
        ("adds xzr, x25, x26", adds(31, 25, 26)),
        ("sub x4, x4, x30", sub(4, 4, 30)),
        ("subs xzr, x18, x17", subs(31, 18, 17)),
        ("and x1, x2, x29", and(1, 2, 29)),
        ("bic x24, x24, x16", bic(24, 24, 16)),
        ("orr x24, x24, x16", orr(24, 24, 16)),
        ("eor x16, x25, x26", eor(16, 25, 26, LSL, 0)),
        ("eor x16, x16, x16, lsr #4", eor(16, 16, 16, LSR, 4)),
        ("mov x7, x27", mov(7, 27)),
        ("mov w7, w27", mov32(7, 27)),
        ("add x16, x16, #0x8", addi(16, 16, 8)),
        ("add x17, x17, #4095", addi(17, 17, 4095)),
        ("sub x17, x16, #0x1", subi(17, 16, 1)),
        ("add w5, w9, #0x10", addi32(5, 9, 16)),
        ("add w30, w0, #4095", addi32(30, 0, 4095)),
        ("sub w5, w9, #0x10", subi32(5, 9, 16)),
        ("adds wzr, w25, w26", adds32(31, 25, 26, 0)),
        ("adds wzr, w16, w26, lsl #24", adds32(31, 16, 26, 24)),
        ("subs wzr, w18, w17", subs32(31, 18, 17, 0)),
        ("subs wzr, w16, w3, lsl #16", subs32(31, 16, 3, 16)),
        ("lsl w16, w25, #24", lsl32(16, 25, 24)),
        ("lsl w16, w2, #16", lsl32(16, 2, 16)),
        ("and x24, x24, #0x1", and_low(24, 24, 1)),
        ("and x3, x7, #0xff", and_low(3, 7, 8)),
        ("mov x24, xzr", mov(24, ZR)),
        ("cmp x16, #0x1", cmpi(16, 1)),
        ("cmp x25, #0", cmpi(25, 0)),
        ("cmp x18, x17", cmp(18, 17)),
        ("ubfx x21, x25, #31, #1", ubfx(21, 25, 31, 1)),
        ("ubfx x25, x3, #8, #8", ubfx(25, 3, 8, 8)),
        ("ubfx x25, x3, #0, #16", ubfx(25, 3, 0, 16)),
        ("ubfx x19, x16, #32, #1", ubfx(19, 16, 32, 1)),
        ("ubfx x22, x17, #63, #1", ubfx(22, 17, 63, 1)),
        ("lsr x19, x16, #8", lsri(19, 16, 8)),
        ("bfi x3, x25, #8, #8", bfi(3, 25, 8, 8)),
        ("bfi x3, x25, #0, #16", bfi(3, 25, 0, 16)),
        ("bfi x0, x26, #0, #8", bfi(0, 26, 0, 8)),
        ("sxtw x25, w26", sxtw(25, 26)),
        ("lsr x25, x17, x16", lsrv(25, 17, 16)),
        ("mul x27, x25, x26", mul(27, 25, 26)),
        ("lsl x25, x17, x16", lslv(25, 17, 16)),
        ("asr x25, x17, x16", asrv(25, 17, 16)),
        ("ror x25, x17, x16", rorv(25, 17, 16, 64)),
        ("ror w25, w17, w16", rorv(25, 17, 16, 32)),
        ("sbfx x25, x3, #0, #8", sbfx(25, 3, 0, 8)),
        ("sbfx x25, x3, #0, #32", sbfx(25, 3, 0, 32)),
        ("smulh x27, x25, x26", smulh(27, 25, 26)),
        ("udiv x18, x25, x26", udiv(18, 25, 26)),
        ("msub x27, x18, x26, x25", msub(27, 18, 26, 25)),
        ("rev x29, x30", rev(29, 30, 64)),
        ("rev w29, w30", rev(29, 30, 32)),
        ("clz x18, x25", clz(18, 25)),
        ("rbit x26, x27", rbit(26, 27)),
        ("orr v3.16b, v4.16b, v5.16b", orr_v(3, 4, 5)),
        ("sub v3.4s, v4.4s, v5.4s", sub_v(3, 4, 5, 2)),
        ("sub v3.2d, v4.2d, v31.2d", sub_v(3, 4, 31, 3)),
        ("ext v30.16b, v2.16b, v31.16b, #5", ext_v(30, 2, 31, 5)),
        ("zip1 v7.16b, v7.16b, v9.16b", zip1_v(7, 7, 9)),
        ("cmeq v1.16b, v2.16b, v3.16b", cmeq_v(1, 2, 3)),
        ("sxtl v30.2d, v6.2s", sxtl_2d(30, 6)),
        ("scvtf v6.2d, v30.2d", scvtf_2d(6, 30)),
        ("scvtf v6.4s, v30.4s", scvtf_4s(6, 30)),
        ("fcvtl v6.2d, v30.2s", fcvtl_2d(6, 30)),
        ("fcvtn v6.2s, v30.2d", fcvtn_2s(6, 30)),
        ("mov x16, v5.d[1]", umov_d(16, 5, 1)),
        ("mov v5.d[0], x17", ins_d(5, 0, 17)),
        ("mov v4.s[3], v30.s[1]", ins_s_element(4, 3, 30, 1)),
        ("movi v31.2d, #0", movi_zero(31)),
        ("ushr v2.2d, v2.2d, #13", ushr_2d(2, 2, 13)),
        ("ushr v2.2d, v3.2d, #64", ushr_2d(2, 3, 64)),
        ("dmb ishst", dmb_ishst()),
        ("dmb ishld", dmb_ishld()),
        ("dmb ish", dmb_ish()),
        ("ldaxrb w25, [x18]", ldaxr(1, 25, 18)),
        ("ldaxrh w25, [x18]", ldaxr(2, 25, 18)),
        ("ldaxr w25, [x18]", ldaxr(4, 25, 18)),
        ("ldaxr x25, [x18]", ldaxr(8, 25, 18)),
        ("stlxrb w16, w26, [x18]", stlxr(1, 16, 26, 18)),
        ("stlxr w16, x26, [x18]", stlxr(8, 16, 26, 18)),
        ("umulh x27, x25, x26", umulh(27, 25, 26)),
        ("mrs x0, esr_el1", mrs(0, ESR_EL1)),
        ("mrs x1, far_el1", mrs(1, FAR_EL1)),
        ("mrs x2, elr_el1", mrs(2, ELR_EL1)),
        ("msr elr_el1, x17", msr(ELR_EL1, 17)),
        ("msr spsr_el1, x17", msr(SPSR_EL1, 17)),
        ("eret", ERET),
        ("ldr s5, [x25]", ldr_v(4, 5, 25, 0)),
        ("ldr d5, [x25, #8]", ldr_v(8, 5, 25, 8)),
        ("ldr q15, [x26]", ldr_v(16, 15, 26, 0)),
        ("str s0, [x25]", str_v(4, 0, 25, 0)),
        ("str d1, [x18]", str_v(8, 1, 18, 0)),
        ("str q2, [x27, #16]", str_v(16, 2, 27, 16)),
        ("stp q0, q1, [x16, #32]", stp_q(0, 1, 16, 32)),
        ("ldp q14, q15, [x16, #224]", ldp_q(14, 15, 16, 224)),
        ("mov v3.16b, v7.16b", mov_v(3, 7)),
        ("mov v3.s[0], v31.s[0]", ins_lane0(4, 3, 31)),
        ("mov v3.d[0], v31.d[0]", ins_lane0(8, 3, 31)),
        ("eor v1.16b, v1.16b, v2.16b", eor_v(1, 1, 2)),
        ("and v1.16b, v1.16b, v2.16b", and_v(1, 1, 2)),
        ("fmov s4, w25", fmov_to_v(4, 4, 25)),
        ("fmov d4, x25", fmov_to_v(8, 4, 25)),
        ("fmov w25, s4", fmov_from_v(4, 25, 4)),
        ("fmov x25, d4", fmov_from_v(8, 25, 4)),
        ("fadd s31, s1, s2", fp_scalar("add", 4, 31, 1, 2)),
        ("fsub d31, d1, d2", fp_scalar("sub", 8, 31, 1, 2)),
        ("fmul s31, s1, s2", fp_scalar("mul", 4, 31, 1, 2)),
        ("fdiv d31, d1, d2", fp_scalar("div", 8, 31, 1, 2)),
        ("fmin s31, s1, s2", fp_scalar("min", 4, 31, 1, 2)),
        ("fmax d31, d1, d2", fp_scalar("max", 8, 31, 1, 2)),
        ("fadd v1.4s, v1.4s, v2.4s", fp_vector("add", 4, 1, 1, 2)),
        ("fsub v1.2d, v1.2d, v2.2d", fp_vector("sub", 8, 1, 1, 2)),
        ("fmul v1.4s, v1.4s, v2.4s", fp_vector("mul", 4, 1, 1, 2)),
        ("fdiv v1.2d, v1.2d, v2.2d", fp_vector("div", 8, 1, 1, 2)),
        ("fmin v1.4s, v1.4s, v2.4s", fp_vector("min", 4, 1, 1, 2)),
        ("fmax v1.2d, v1.2d, v2.2d", fp_vector("max", 8, 1, 1, 2)),
        ("fsqrt s31, s2", fsqrt_scalar(4, 31, 2)),
        ("fsqrt d31, d2", fsqrt_scalar(8, 31, 2)),
        ("fsqrt v1.4s, v2.4s", fsqrt_vector(4, 1, 2)),
        ("fsqrt v1.2d, v2.2d", fsqrt_vector(8, 1, 2)),
        ("fcmp s1, s2", fcmp(4, 1, 2, false)),
        ("fcmpe d1, d2", fcmp(8, 1, 2, true)),
        ("fcvtzs w25, s3", fcvtzs(32, 4, 25, 3)),
        ("fcvtzs x25, d3", fcvtzs(64, 8, 25, 3)),
        ("frintx s31, s3", frintx(4, 31, 3)),
        ("frintx d31, d3", frintx(8, 31, 3)),
        ("scvtf s31, w25", scvtf(4, 32, 31, 25)),
        ("scvtf d31, x25", scvtf(8, 64, 31, 25)),
        ("fcvt d31, s3", fcvt(8, 4, 31, 3)),
        ("fcvt s31, d3", fcvt(4, 8, 31, 3)),
        ("mrs x16, fpsr", mrs(16, FPSR)),
        ("msr fpsr, x16", msr(FPSR, 16)),
        ("msr fpcr, x16", msr(FPCR, 16)),
        ("strb w16, [x28, #56]", str(1, 16, 28, 56)),
        ("cset x19, cs", cset(19, "cs")),
        ("cset x19, lo", cset(19, "lo")),
        ("cset x20, eq", cset(20, "eq")),
        ("cset x16, ne", cset(16, "ne")),
        ("ldrb w25, [x26]", ldr(1, 25, 26, 0)),
        ("ldrh w25, [x26]", ldr(2, 25, 26, 0)),
        ("ldr w25, [x26]", ldr(4, 25, 26, 0)),
        ("ldr x25, [x26]", ldr(8, 25, 26, 0)),
        ("ldr x17, [x28, #8]", ldr(8, 17, 28, 8)),
        ("strb w25, [x26]", str(1, 25, 26, 0)),
        ("strh w25, [x26]", str(2, 25, 26, 0)),
        ("str w25, [x26]", str(4, 25, 26, 0)),
        ("str x17, [x16]", str(8, 17, 16, 0)),
        ("ldp x18, x16, [x16]", ldp(18, 16, 16, 0)),
        ("ldp x17, x18, [x28, #16]", ldp(17, 18, 28, 16)),
        ("b .+0x40", b(0x40)),
        ("cnt v31.8b, v31.8b", cnt_8b(31, 31)),
        ("uaddlv h30, v30.8b", uaddlv_8b(30, 30)),
        ("sxtl v5.2d, v30.2s", sxtl_2d(5, 30)),
        ("cmeq v1.2d, v2.2d, v3.2d", three_same("cmeq", 1, 2, 3, 3)),
        ("mov v28.s[0], w16", ins_s(28, 0, 16)),
        ("mov v28.s[3], w16", ins_s(28, 3, 16)),
        ("fcmeq v1.4s, v2.4s, v3.4s", fcm("eq", false, 1, 2, 3)),
        ("fcmge v1.4s, v2.4s, v3.4s", fcm("ge", false, 1, 2, 3)),
        ("fcmgt v1.4s, v2.4s, v3.4s", fcm("gt", false, 1, 2, 3)),
        ("fcmeq v30.2d, v31.2d, v0.2d", fcm("eq", true, 30, 31, 0)),
        ("fcmge v30.2d, v31.2d, v0.2d", fcm("ge", true, 30, 31, 0)),
        ("fcmgt v30.2d, v31.2d, v0.2d", fcm("gt", true, 30, 31, 0)),
        ("not v5.16b, v6.16b", not_16b(5, 6)),
        ("addv b31, v31.8b", addv_8b(31, 31)),
        ("bl .+0x40", bl(0x40)),
        ("bl .-0x1000", bl(-0x1000)),
        ("ret", ret()),
        ("b .-0x1000", b(-0x1000)),
        ("b.ne .+0xc", bcond("ne", 0xC)),
        ("b.ls .+8", bcond("ls", 8)),
        ("b.ne .-0x20", bcond("ne", -0x20)),
        ("cbz x16, .+0x18", cbz(16, 0x18)),
        ("cbnz x16, .+0x8", cbnz(16, 8)),
        ("cbnz x19, .-0x10", cbnz(19, -0x10)),
        ("tbz x24, #3, .+8", tbz(24, 3, 8)),
        ("tbz x24, #0, .+0x40", tbz(24, 0, 0x40)),
        ("br x16", br(16)),
        ("blr x16", blr(16)),
        ("stp x0, x1, [x16]", stp(0, 1, 16, 0)),
        ("stp x14, x15, [x16, #112]", stp(14, 15, 16, 112)),
        ("mov sp, x16", addi(31, 16, 0)),
        ("hvc #0x1", hvc(1)),
        ("hvc #0x33", hvc(0x33)),
        ("ic iallu", IC_IALLU),
        ("tlbi vmalle1", TLBI_VMALLE1),
        ("dsb ish", DSB_ISH),
        ("isb", ISB),
        ("add v3.4s, v5.4s, v30.4s", add_v(3, 5, 30, 2)),
        ("bic v3.16b, v31.16b, v3.16b", bic_v(3, 31, 3)),
        ("zip1 v3.4s, v3.4s, v31.4s", zip1_4s(3, 3, 31)),
        ("zip2 v3.4s, v3.4s, v31.4s", zip2_4s(3, 3, 31)),
        ("ushr v3.4s, v3.4s, #1", ushr_4s(3, 3, 1)),
        ("ushr v3.4s, v3.4s, #31", ushr_4s(3, 3, 31)),
        ("sshr v3.4s, v3.4s, #7", sshr_4s(3, 3, 7)),
        ("sshr v3.4s, v3.4s, #32", sshr_4s(3, 3, 32)),
        ("shl v3.4s, v3.4s, #0", shl_4s(3, 3, 0)),
        ("shl v3.4s, v3.4s, #31", shl_4s(3, 3, 31)),
        ("dup v30.4s, w16", dup_4s(30, 16)),
        ("ushl v3.4s, v3.4s, v30.4s", ushl_4s(3, 3, 30)),
        ("sshl v3.4s, v3.4s, v30.4s", sshl_4s(3, 3, 30)),
        ("mov w16, v3.s[2]", umov_s(16, 3, 2)),
        ("sdiv x16, x0, x3", sdiv(16, 0, 3)),
        ("ldaxp x16, x17, [x25]", ldaxp(16, 17, 25)),
        ("stlxp w30, x3, x1, [x25]", stlxp(30, 3, 1, 25)),
        ("add v3.8h, v3.8h, v30.8h", three_same("add", 3, 3, 30, 1)),
        ("add v3.2d, v3.2d, v30.2d", three_same("add", 3, 3, 30, 3)),
        ("sub v3.8h, v3.8h, v30.8h", three_same("sub", 3, 3, 30, 1)),
        ("cmeq v3.4s, v3.4s, v30.4s", three_same("cmeq", 3, 3, 30, 2)),
        ("cmgt v3.4s, v3.4s, v30.4s", three_same("cmgt", 3, 3, 30, 2)),
        ("uqsub v3.16b, v3.16b, v30.16b", three_same("uqsub", 3, 3, 30, 0)),
        ("sqsub v3.16b, v3.16b, v30.16b", three_same("sqsub", 3, 3, 30, 0)),
        ("sqadd v3.8h, v3.8h, v30.8h", three_same("sqadd", 3, 3, 30, 1)),
        ("umax v3.16b, v3.16b, v30.16b", three_same("umax", 3, 3, 30, 0)),
        ("mul v3.8h, v3.8h, v30.8h", three_same("mul", 3, 3, 30, 1)),
        ("zip1 v3.2d, v3.2d, v30.2d", three_same("zip1", 3, 3, 30, 3)),
        ("zip2 v3.8h, v3.8h, v30.8h", three_same("zip2", 3, 3, 30, 1)),
        ("zip2 v3.16b, v3.16b, v30.16b", three_same("zip2", 3, 3, 30, 0)),
        ("ushl v3.2d, v3.2d, v30.2d", three_same("ushl", 3, 3, 30, 3)),
        ("sshl v3.8h, v3.8h, v30.8h", three_same("sshl", 3, 3, 30, 1)),
        ("ushr v3.8h, v3.8h, #5", shift_right(3, 3, 5, 1, false)),
        ("sshr v3.8h, v3.8h, #16", shift_right(3, 3, 16, 1, true)),
        ("ushr v3.2d, v3.2d, #64", shift_right(3, 3, 64, 3, false)),
        ("shl v3.8h, v3.8h, #15", shift_left(3, 3, 15, 1)),
        ("shl v3.2d, v3.2d, #63", shift_left(3, 3, 63, 3)),
        ("dup v30.8h, w16", dup_gpr(30, 16, 1)),
        ("dup v30.2d, x16", dup_gpr(30, 16, 3)),
        ("sqxtun v30.8b, v3.8h", sqxtun(30, 3, false)),
        ("sqxtun2 v30.16b, v31.8h", sqxtun(30, 31, true)),
        ("sqxtn v30.8b, v3.8h", sqxtn(30, 3, false)),
        ("sqxtn2 v30.16b, v31.8h", sqxtn(30, 31, true)),
        ("smull v30.4s, v2.4h, v31.4h", smull(30, 2, 31, false)),
        ("smull2 v31.4s, v2.8h, v31.8h", smull(31, 2, 31, true)),
        ("uzp2 v2.8h, v30.8h, v31.8h", three_same("uzp2", 2, 30, 31, 1)),
        ("uqadd v5.16b, v5.16b, v31.16b", three_same("uqadd", 5, 5, 31, 0)),
        ("tbl v4.16b, { v4.16b }, v30.16b", tbl(4, 4, 30)),
        ("movi v30.16b, #0x8f", movi_bytes(30, 0x8F)),
        ("zip1 v30.8b, v30.8b, v31.8b", three_same_d("zip1", 30, 30, 31, 0)),
        ("zip2 v30.4h, v30.4h, v31.4h", three_same_d("zip2", 30, 30, 31, 1)),
        ("sqadd v30.4h, v30.4h, v31.4h", three_same_d("sqadd", 30, 30, 31, 1)),
        ("add v30.2s, v30.2s, v31.2s", three_same_d("add", 30, 30, 31, 2)),
        ("mov v3.h[5], v30.h[2]", ins_h_element(3, 5, 30, 2)),
        ("mov v3.h[7], w16", ins_h(3, 7, 16)),
        // the monitor code's fixed words (Sprint 135; ARMv8.1 for CASA)
        ("ic ialluis", IC_IALLUIS),
        ("tlbi vmalle1is", TLBI_VMALLE1IS),
        ("casa wzr, w18, [x16]", CASA_WZR_W18_X16),
        ("stlr wzr, [x16]", STLR_WZR_X16),
        ("mov x24, #0x10", mov_imm(24, 0x10)[0]),
        ("adr x16, .+12", adr(16, 12)),
        ("ldr x0, [x1, x2, lsl #3]", ldr_reg(8, 0, 1, 2, true)),
        ("str x0, [x1, x2, lsl #3]", str_reg(8, 0, 1, 2, true)),
        ("ldur x0, [x1, #-8]", ldur(8, 0, 1, -8)),
        ("stur x0, [x1, #-8]", stur(8, 0, 1, -8)),
    ]
}

fn tool(name: &str) -> Option<PathBuf> {
    let out = Command::new("xcrun").args(["--find", name]).output().ok()?;
    let path = String::from_utf8(out.stdout).ok()?.trim().to_string();
    (out.status.success() && !path.is_empty()).then(|| PathBuf::from(path))
}

#[test]
fn every_encoding_equals_clangs_assembly() {
    let (Some(clang), Some(objdump)) = (tool("clang"), tool("llvm-objdump")) else {
        eprintln!("clang or llvm-objdump not found: the encodings are not checked here");
        return;
    };
    let cases = cases();
    let dir = std::env::temp_dir().join(format!("w2m-a64-encoder-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let src = dir.join("a.s");
    let body: String = cases.iter().map(|(t, _)| format!("  {t}\n")).collect();
    std::fs::write(&src, format!(".text\n{body}")).unwrap();
    let obj = dir.join("a.o");
    let built = Command::new(clang)
        .args([
            "-c",
            "--target=aarch64-linux-gnu",
            "-march=armv8.1-a",
            "-x",
            "assembler",
        ])
        .arg(&src)
        .arg("-o")
        .arg(&obj)
        .output()
        .unwrap();
    assert!(built.status.success(), "{}", String::from_utf8_lossy(&built.stderr));
    let dump = Command::new(objdump).arg("-d").arg(&obj).output().unwrap();
    let text = String::from_utf8_lossy(&dump.stdout);
    // each instruction line: "  <address>: <8 hex digits> <mnemonic>"
    let words: Vec<u32> = text
        .lines()
        .filter_map(|l| {
            let (addr, rest) = l.trim_start().split_once(':')?;
            u64::from_str_radix(addr, 16).ok()?;
            let hex = rest.split_whitespace().next()?;
            (hex.len() == 8).then(|| u32::from_str_radix(hex, 16).ok())?
        })
        .collect();
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(
        words.len(),
        cases.len(),
        "clang assembled {} words for {} cases",
        words.len(),
        cases.len()
    );
    let bad: Vec<String> = cases
        .iter()
        .zip(&words)
        .filter(|((_, ours), clangs)| ours != *clangs)
        .map(|((t, ours), clangs)| format!("{t}: ours {ours:08x}, clang {clangs:08x}"))
        .collect();
    assert!(bad.is_empty(), "{bad:#?}");
    println!("a64_encoder: {} encodings equal clang's", cases.len());
}
