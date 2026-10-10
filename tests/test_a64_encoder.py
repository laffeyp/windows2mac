"""Sprint 030: every encoder form in src/w2m/a64_encoder.py equals clang's assembly of the same text
(the Arm ARM transcription checked by an independent assembler)."""

from __future__ import annotations

import re
import shutil
import subprocess
from pathlib import Path

import pytest

from w2m import a64_encoder as a64

OBJDUMP = shutil.which("llvm-objdump") or (
    subprocess.run(["xcrun", "--find", "llvm-objdump"], capture_output=True, text=True).stdout.strip()
    if shutil.which("xcrun") else "")
CLANG = shutil.which("clang")

CASES: list[tuple[str, int]] = [
    ("movz x3, #0x1234", a64.movz(3, 0x1234)),
    ("movz x17, #0xffff, lsl #48", a64.movz(17, 0xFFFF, 3)),
    ("movk x17, #0xbeef, lsl #16", a64.movk(17, 0xBEEF, 1)),
    ("movk x0, #0x7ffe, lsl #32", a64.movk(0, 0x7FFE, 2)),
    ("add x1, x2, x3", a64.add(1, 2, 3)),
    ("add x16, x16, x18, lsl #4", a64.add(16, 16, 18, a64.LSL, 4)),
    ("add x25, x5, x15, lsl #3", a64.add(25, 5, 15, a64.LSL, 3)),
    ("adds xzr, x25, x26", a64.adds(31, 25, 26)),
    ("sub x4, x4, x30", a64.sub(4, 4, 30)),
    ("subs xzr, x18, x17", a64.subs(31, 18, 17)),
    ("and x1, x2, x29", a64.and_(1, 2, 29)),
    ("bic x24, x24, x16", a64.bic(24, 24, 16)),
    ("orr x24, x24, x16", a64.orr(24, 24, 16)),
    ("eor x16, x25, x26", a64.eor(16, 25, 26)),
    ("eor x16, x16, x16, lsr #4", a64.eor(16, 16, 16, a64.LSR, 4)),
    ("mov x7, x27", a64.mov(7, 27)),
    ("mov w7, w27", a64.mov32(7, 27)),
    ("add x16, x16, #0x8", a64.addi(16, 16, 8)),
    ("add x17, x17, #4095", a64.addi(17, 17, 4095)),
    ("sub x17, x16, #0x1", a64.subi(17, 16, 1)),
    ("cmp x16, #0x1", a64.cmpi(16, 1)),
    ("cmp x25, #0", a64.cmpi(25, 0)),
    ("cmp x18, x17", a64.cmp(18, 17)),
    ("ubfx x21, x25, #31, #1", a64.ubfx(21, 25, 31, 1)),
    ("ubfx x25, x3, #8, #8", a64.ubfx(25, 3, 8, 8)),
    ("ubfx x25, x3, #0, #16", a64.ubfx(25, 3, 0, 16)),
    ("ubfx x19, x16, #32, #1", a64.ubfx(19, 16, 32, 1)),
    ("ubfx x22, x17, #63, #1", a64.ubfx(22, 17, 63, 1)),
    ("lsr x19, x16, #8", a64.lsri(19, 16, 8)),
    ("bfi x3, x25, #8, #8", a64.bfi(3, 25, 8, 8)),
    ("bfi x3, x25, #0, #16", a64.bfi(3, 25, 0, 16)),
    ("bfi x0, x26, #0, #8", a64.bfi(0, 26, 0, 8)),
    ("sxtw x25, w26", a64.sxtw(25, 26)),
    ("lsr x25, x17, x16", a64.lsrv(25, 17, 16)),
    ("mul x27, x25, x26", a64.mul(27, 25, 26)),
    ("lsl x25, x17, x16", a64.lslv(25, 17, 16)),
    ("asr x25, x17, x16", a64.asrv(25, 17, 16)),
    ("ror x25, x17, x16", a64.rorv(25, 17, 16)),
    ("ror w25, w17, w16", a64.rorv(25, 17, 16, 32)),
    ("sbfx x25, x3, #0, #8", a64.sbfx(25, 3, 0, 8)),
    ("sbfx x25, x3, #0, #32", a64.sbfx(25, 3, 0, 32)),
    ("smulh x27, x25, x26", a64.smulh(27, 25, 26)),
    ("udiv x18, x25, x26", a64.udiv(18, 25, 26)),
    ("msub x27, x18, x26, x25", a64.msub(27, 18, 26, 25)),
    ("rev x29, x30", a64.rev(29, 30)),
    ("rev w29, w30", a64.rev(29, 30, 32)),
    ("ldaxrb w25, [x18]", a64.ldaxr(1, 25, 18)),
    ("ldaxrh w25, [x18]", a64.ldaxr(2, 25, 18)),
    ("ldaxr w25, [x18]", a64.ldaxr(4, 25, 18)),
    ("ldaxr x25, [x18]", a64.ldaxr(8, 25, 18)),
    ("stlxrb w16, w26, [x18]", a64.stlxr(1, 16, 26, 18)),
    ("stlxr w16, x26, [x18]", a64.stlxr(8, 16, 26, 18)),
    ("umulh x27, x25, x26", a64.umulh(27, 25, 26)),
    ("mrs x0, esr_el1", a64.mrs(0, a64.ESR_EL1)),
    ("mrs x1, far_el1", a64.mrs(1, a64.FAR_EL1)),
    ("mrs x2, elr_el1", a64.mrs(2, a64.ELR_EL1)),
    ("msr elr_el1, x17", a64.msr(a64.ELR_EL1, 17)),
    ("msr spsr_el1, x17", a64.msr(a64.SPSR_EL1, 17)),
    ("eret", a64.ERET),
    ("ldr s5, [x25]", a64.ldr_v(4, 5, 25)),
    ("ldr d5, [x25, #8]", a64.ldr_v(8, 5, 25, 8)),
    ("ldr q15, [x26]", a64.ldr_v(16, 15, 26)),
    ("str s0, [x25]", a64.str_v(4, 0, 25)),
    ("str d1, [x18]", a64.str_v(8, 1, 18)),
    ("str q2, [x27, #16]", a64.str_v(16, 2, 27, 16)),
    ("stp q0, q1, [x16, #32]", a64.stp_q(0, 1, 16, 32)),
    ("ldp q14, q15, [x16, #224]", a64.ldp_q(14, 15, 16, 224)),
    ("mov v3.16b, v7.16b", a64.mov_v(3, 7)),
    ("mov v3.s[0], v31.s[0]", a64.ins_lane0(4, 3, 31)),
    ("mov v3.d[0], v31.d[0]", a64.ins_lane0(8, 3, 31)),
    ("eor v1.16b, v1.16b, v2.16b", a64.eor_v(1, 1, 2)),
    ("and v1.16b, v1.16b, v2.16b", a64.and_v(1, 1, 2)),
    ("fmov s4, w25", a64.fmov_to_v(4, 4, 25)),
    ("fmov d4, x25", a64.fmov_to_v(8, 4, 25)),
    ("fmov w25, s4", a64.fmov_from_v(4, 25, 4)),
    ("fmov x25, d4", a64.fmov_from_v(8, 25, 4)),
    ("fadd s31, s1, s2", a64.fp_scalar("add", 4, 31, 1, 2)),
    ("fsub d31, d1, d2", a64.fp_scalar("sub", 8, 31, 1, 2)),
    ("fmul s31, s1, s2", a64.fp_scalar("mul", 4, 31, 1, 2)),
    ("fdiv d31, d1, d2", a64.fp_scalar("div", 8, 31, 1, 2)),
    ("fmin s31, s1, s2", a64.fp_scalar("min", 4, 31, 1, 2)),
    ("fmax d31, d1, d2", a64.fp_scalar("max", 8, 31, 1, 2)),
    ("fadd v1.4s, v1.4s, v2.4s", a64.fp_vector("add", 4, 1, 1, 2)),
    ("fsub v1.2d, v1.2d, v2.2d", a64.fp_vector("sub", 8, 1, 1, 2)),
    ("fmul v1.4s, v1.4s, v2.4s", a64.fp_vector("mul", 4, 1, 1, 2)),
    ("fdiv v1.2d, v1.2d, v2.2d", a64.fp_vector("div", 8, 1, 1, 2)),
    ("fmin v1.4s, v1.4s, v2.4s", a64.fp_vector("min", 4, 1, 1, 2)),
    ("fmax v1.2d, v1.2d, v2.2d", a64.fp_vector("max", 8, 1, 1, 2)),
    ("fsqrt s31, s2", a64.fsqrt_scalar(4, 31, 2)),
    ("fsqrt d31, d2", a64.fsqrt_scalar(8, 31, 2)),
    ("fsqrt v1.4s, v2.4s", a64.fsqrt_vector(4, 1, 2)),
    ("fsqrt v1.2d, v2.2d", a64.fsqrt_vector(8, 1, 2)),
    ("fcmp s1, s2", a64.fcmp(4, 1, 2)),
    ("fcmpe d1, d2", a64.fcmp(8, 1, 2, True)),
    ("fcvtzs w25, s3", a64.fcvtzs(32, 4, 25, 3)),
    ("fcvtzs x25, d3", a64.fcvtzs(64, 8, 25, 3)),
    ("frintx s31, s3", a64.frintx(4, 31, 3)),
    ("frintx d31, d3", a64.frintx(8, 31, 3)),
    ("scvtf s31, w25", a64.scvtf(4, 32, 31, 25)),
    ("scvtf d31, x25", a64.scvtf(8, 64, 31, 25)),
    ("fcvt d31, s3", a64.fcvt(8, 4, 31, 3)),
    ("fcvt s31, d3", a64.fcvt(4, 8, 31, 3)),
    ("mrs x16, fpsr", a64.mrs(16, a64.FPSR)),
    ("msr fpsr, x16", a64.msr(a64.FPSR, 16)),
    ("msr fpcr, x16", a64.msr(a64.FPCR, 16)),
    ("strb w16, [x28, #56]", a64.str_(1, 16, 28, 56)),
    ("cset x19, cs", a64.cset(19, "cs")),
    ("cset x19, lo", a64.cset(19, "lo")),
    ("cset x20, eq", a64.cset(20, "eq")),
    ("cset x16, ne", a64.cset(16, "ne")),
    ("ldrb w25, [x26]", a64.ldr(1, 25, 26)),
    ("ldrh w25, [x26]", a64.ldr(2, 25, 26)),
    ("ldr w25, [x26]", a64.ldr(4, 25, 26)),
    ("ldr x25, [x26]", a64.ldr(8, 25, 26)),
    ("ldr x17, [x28, #8]", a64.ldr(8, 17, 28, 8)),
    ("strb w25, [x26]", a64.str_(1, 25, 26)),
    ("strh w25, [x26]", a64.str_(2, 25, 26)),
    ("str w25, [x26]", a64.str_(4, 25, 26)),
    ("str x17, [x16]", a64.str_(8, 17, 16)),
    ("ldp x18, x16, [x16]", a64.ldp(18, 16, 16)),
    ("ldp x17, x18, [x28, #16]", a64.ldp(17, 18, 28, 16)),
    ("b .+0x40", a64.b(0x40)),
    ("b .-0x1000", a64.b(-0x1000)),
    ("b.ne .+0xc", a64.bcond("ne", 0xC)),
    ("b.ls .+8", a64.bcond("ls", 8)),
    ("b.ne .-0x20", a64.bcond("ne", -0x20)),
    ("cbz x16, .+0x18", a64.cbz(16, 0x18)),
    ("cbnz x16, .+0x8", a64.cbnz(16, 8)),
    ("cbnz x19, .-0x10", a64.cbnz(19, -0x10)),
    ("tbz x24, #3, .+8", a64.tbz(24, 3, 8)),
    ("tbz x24, #0, .+0x40", a64.tbz(24, 0, 0x40)),
    ("br x16", a64.br(16)),
    ("blr x16", a64.blr(16)),
    ("stp x0, x1, [x16]", a64.stp(0, 1, 16)),
    ("stp x14, x15, [x16, #112]", a64.stp(14, 15, 16, 112)),
    ("mov sp, x16", a64.addi(31, 16, 0)),
    ("hvc #0x1", a64.hvc(1)),
    ("hvc #0x33", a64.hvc(0x33)),
    ("ic iallu", a64.IC_IALLU),
    ("dsb ish", a64.DSB_ISH),
    ("isb", a64.ISB),
]


@pytest.mark.skipif(not (OBJDUMP and CLANG), reason="clang or llvm-objdump not available")
def test_encodings_match_clang(tmp_path: Path) -> None:
    src = tmp_path / "a.s"
    src.write_text(".text\n" + "".join(f"  {t}\n" for t, _ in CASES))
    obj = tmp_path / "a.o"
    assert CLANG is not None
    subprocess.run([CLANG, "-c", "--target=aarch64-linux-gnu", "-x", "assembler", str(src),
                    "-o", str(obj)], check=True, capture_output=True)
    out = subprocess.run([OBJDUMP, "-d", str(obj)], capture_output=True, text=True, check=True).stdout
    words = [int(m.group(1), 16) for m in re.finditer(r"^\s*[0-9a-f]+:\s+([0-9a-f]{8})\s", out, re.M)]
    assert len(words) == len(CASES)
    bad = [(t, f"{w:08x}", f"{c:08x}") for (t, w), c in zip(CASES, words, strict=True) if w != c]
    assert not bad, bad


def test_mov_imm_halfwords() -> None:
    assert a64.mov_imm(17, 0x7FFE_0000_0010) == [a64.movz(17, 0x10), a64.movk(17, 0x7FFE, 2)]
    assert a64.mov_imm(0, 0) == [a64.movz(0, 0)]
    assert len(a64.mov_imm(0, (1 << 64) - 1)) == 4
