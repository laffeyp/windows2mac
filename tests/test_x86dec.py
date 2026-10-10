"""Sprint 015 grading surface: w2m's own decoder (generated from specs/x86_decode.json) against
the Capstone reference at every reachable instruction of the runner-built probes."""

from __future__ import annotations

from collections import Counter
from pathlib import Path

import pytest

from w2m.pe import parse
from w2m.x86dec import DecodeError, decode
from w2m.x86ref import walk

BINARIES = sorted((Path(__file__).resolve().parent.parent / "store" / "binaries").glob("*.exe"))


def disagreements(path: Path) -> tuple[int, Counter[tuple[str, ...]]]:
    data = path.read_bytes()
    img = parse(data)
    r = walk(img, data)
    out: Counter[tuple[str, ...]] = Counter()
    for ins in r.insns.values():
        try:
            got = decode(img.read_rva(data, ins.rva, 15))
        except DecodeError as e:
            out[("error", ins.mnemonic, str(e))] += 1
            continue
        if (got.length, got.mnemonic) != (ins.size, ins.mnemonic):
            out[("differs", ins.raw.hex(), f"{ins.mnemonic}/{ins.size}", f"{got.mnemonic}/{got.length}")] += 1
    return len(r.insns), out


@pytest.mark.parametrize("path", BINARIES, ids=lambda p: p.name[:12])
def test_own_decoder_agrees_with_reference(path: Path) -> None:
    n, d = disagreements(path)
    assert n > (20_000 if parse(path.read_bytes()).function_starts else 200)        # CRT-linked: has .pdata
    assert not d, d.most_common(10)


@pytest.mark.parametrize(("hexbytes", "length", "mnemonic", "iclass"), [
    ("90", 1, "nop", "NOP"), ("4190", 2, "xchg", "XCHG"), ("f390", 2, "pause", "PAUSE"),
    ("48b88877665544332211", 10, "movabs", "MOV"), ("b844332211", 5, "mov", "MOV"),
    ("f2410f114bd8", 6, "movsd", "MOVSD_XMM"),   # SSE movsd: F2 is mandatory, not repne
    ("a5", 1, "movsd", "MOVSD"),                 # the string move with the same mnemonic
    ("f348ab", 3, "rep stosq", "REP_STOSQ"), ("f0ff05aabbccdd", 7, "lock inc", "INC_LOCK"),
    ("c5f877", 3, "vzeroupper", "VZEROUPPER"), ("c5fc77", 3, "vzeroall", "VZEROALL"),
    ("4898", 2, "cdqe", "CDQE"), ("6698", 2, "cbw", "CBW"), ("0f01d0", 3, "xgetbv", "XGETBV"),
    ("e800000000", 5, "call", "CALL_NEAR"), ("7400", 2, "je", "JZ"),
    ("40498b4078", 5, "mov", "MOV"),            # two REX bytes: only the last applies
    ("a3790800837b0800ed", 9, "movabs", "MOV"),  # moffs64 operand
    ("f3a6", 2, "repe cmpsb", "REPE_CMPSB"),
    ("62e17e286f2557d10800", 10, "vmovdqu32", "VMOVDQU32"),   # EVEX, W0, F3
    ("62f1fd487f01", 6, "vmovdqa64", "VMOVDQA64"),             # EVEX, W1, 66
    ("62e3754825c496", 7, "vpternlogd", "VPTERNLOGD"),         # EVEX 0F3A, imm8
    ("8fe878c2e410", 6, "vprotd", "VPROTD"),                    # XOP map 8, imm8
    ("8f4878c3c838", 6, "vprotq", "VPROTQ"),
    ("8f00", 2, "pop", "POP"),                                  # 8F with map field < 8 is POP r/m
])
def test_known_encodings(hexbytes: str, length: int, mnemonic: str, iclass: str) -> None:
    got = decode(bytes.fromhex(hexbytes) + b"\0" * 15)
    assert (got.length, got.mnemonic, got.iclass) == (length, mnemonic, iclass)


@pytest.mark.parametrize(("features", "expected"), [
    (None, ["TZCNT", "LZCNT", "ENDBR64", "RDSSPQ"]),
    (frozenset({"BMI1", "LZCNT"}), ["TZCNT", "LZCNT", "NOP", "NOP"]),
    (frozenset(), ["BSF", "BSR", "NOP", "NOP"]),
])
def test_decoding_depends_on_cpu_features(features: frozenset[str] | None, expected: list[str]) -> None:
    cases = ["f30fbcc1", "f30fbdc1", "f30f1efa", "f3490f1eca"]
    assert [decode(bytes.fromhex(h) + b"\0" * 15, features).iclass for h in cases] == expected


def test_unknown_opcode_is_reported() -> None:
    with pytest.raises(DecodeError, match="no entry"):
        decode(bytes.fromhex("0f04") + b"\0" * 15)        # 0F 04 is undefined (XED refuses it too)


@pytest.mark.parametrize(("hexbytes", "why"), [
    ("f04188a4d185e3ed61", "LOCK on MOV"),
    ("f00303", "LOCK with register-destination form 03 (add eax, [rbx])"),
    ("f001c0", "LOCK on register form of ADD"),
    ("c5e877c0", "VZEROUPPER needs VEX.vvvv=1111"),
    ("c5fd73348574", "VEX shift-by-immediate group is register-only"),
    ("f2660f6f00", "F2 takes precedence over 66 on 0F 6F, and F2 0F 6F has no row (not MOVDQA)"),
    ("660f5200", "RSQRTPS is NP: 66 0F 52 is not an instruction"),
    ("660f01d0", "XGETBV is NP"),
    ("8dc0", "LEA with a register operand (mod = 3) is #UD"),
    ("f30f38f000", "MOVBE refuses F3 (F3 has no row there and is not ignored)"),
])
def test_invalid_encodings_are_refused(hexbytes: str, why: str) -> None:
    from w2m.x86dec import DecodeError
    with pytest.raises(DecodeError):
        decode(bytes.fromhex(hexbytes) + b"\0" * 15)


@pytest.mark.parametrize(("hexbytes", "iclass"), [
    ("f30f1234", "MOVSLDUP"), ("660f38f000", "MOVBE"), ("f20fbc00", "BSF"), ("f30fc70f", "CMPXCHG8B"),
    ("66cf", "IRET"), ("67e300", "JECXZ"), ("f0490fc708", "CMPXCHG16B_LOCK"),
    ("f00100", "ADD_LOCK"), ("f0870a", "XCHG"), ("c0f401", "SHL"), ("f2a4", "REP_MOVSB"),
    ("f2a6", "REPNE_CMPSB"), ("f34390", "PAUSE"), ("0f18c0", "NOP"), ("660fb6c0", "MOVZX"),
])
def test_edge_rules_from_generated_differential(hexbytes: str, iclass: str) -> None:
    assert decode(bytes.fromhex(hexbytes) + b"\0" * 15).iclass == iclass


def test_generated_encodings_never_disagree_on_length() -> None:
    from w2m.encgen import cases
    from w2m.x86dec import DecodeError
    from w2m.xedref import ROOT, XED_KIT_GLOB, decode_many
    if not list(ROOT.glob(XED_KIT_GLOB)):
        pytest.skip("XED kit not built")
    cs = list(cases(per_row=8))
    bad = []
    for c, x in zip(cs, decode_many([c.raw for c in cs])):
        try:
            g = decode(c.raw)
        except DecodeError:
            continue
        if x.ok and g.length != x.length:
            bad.append((c.raw.hex(), c.note, x.length, g.length))
    assert not bad, bad[:5]


# Sprint 023 (project review 2026-10-08, items 4.3 and 4.4). XED v2026.08.23 verdicts in comments.
@pytest.mark.parametrize("hexbytes", [
    "66c5f877",          # BAD_LEGACY_PREFIX: 66 before VEX
    "f3c5f877",          # BAD_LEGACY_PREFIX: F3 before VEX
    "f2c4e17d58c0",      # BAD_LEGACY_PREFIX: F2 before 3-byte VEX
    "6662f17c4858c0",    # BAD_LEGACY_PREFIX: 66 before EVEX
    "40c5f877",          # BAD_REX_PREFIX: REX before VEX
    "66" * 15,           # INSTR_TOO_LONG
])
def test_prefix_and_length_faults_are_invalid_encodings(hexbytes: str) -> None:
    from w2m.x86dec import InvalidEncoding
    with pytest.raises(InvalidEncoding):
        decode(bytes.fromhex(hexbytes))


def test_segment_and_address_size_prefixes_before_vex_are_allowed() -> None:
    assert decode(bytes.fromhex("2ec5f877")).iclass == "VZEROUPPER"   # XED: VZEROUPPER, length 4
    assert decode(bytes.fromhex("67c5f877")).length == 4


@pytest.mark.parametrize("hexbytes", ["", "66", "0f", "48", "c5", "e8000000", "488b0425"])
def test_short_input_is_truncated_not_indexerror(hexbytes: str) -> None:
    from w2m.x86dec import Truncated
    with pytest.raises(Truncated):
        decode(bytes.fromhex(hexbytes))
