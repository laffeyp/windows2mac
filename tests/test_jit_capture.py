"""Sprint 033: the capture parser (both formats, duplicates, a torn tail, CPU features), the decoder's
row identity and LOCK CMPXCHG16B, and the lifter's refusal of forms without an operand template."""

from __future__ import annotations

import json
import struct
from pathlib import Path

import pytest

from w2m.jit_capture import CaptureError, cpu_features, parse
from w2m.lift import LiftError, lift
from w2m.x86dec import decode

ROOT = Path(__file__).resolve().parent.parent
RET = bytes.fromhex("c3")
BODY = bytes.fromhex("4883ec28e800000000c3")       # sub rsp,0x28; call; ret


def _method_1(start: int, name: str, code: bytes) -> bytes:
    n = name.encode()
    return struct.pack("<QII", start, len(code), len(n)) + n + code


def _method_2(start: int, name: str, image: str, code: bytes) -> bytes:
    n, i = name.encode(), image.encode()
    return struct.pack("<IQIII", 1, start, len(code), len(n), len(i)) + n + i + code


def _buffer_2(start: int, t: int, desc: str, code: bytes) -> bytes:
    d = desc.encode()
    return struct.pack("<IQIII", 2, start, len(code), t, len(d)) + d + code


def _cpuid_2(rows: list[tuple[int, int, int, int, int, int]]) -> bytes:
    return struct.pack("<II", 0, len(rows)) + b"".join(struct.pack("<6I", *r) for r in rows)


def test_format_1_with_duplicate_and_torn_tail() -> None:
    data = (_method_1(0x1000, "A:f ()", BODY) + _method_1(0x2000, "A:g ()", RET)
            + _method_1(0x1000, "A:f ()", BODY) + _method_1(0x3000, "A:h ()", BODY)[:-3])
    cap = parse(data)
    assert cap.format == 1 and len(cap.records) == 3
    assert [r.start for r in cap.regions()] == [0x1000, 0x2000]
    assert cap.torn_tail_bytes == len(_method_1(0x3000, "A:h ()", BODY)) - 3
    assert cpu_features(cap) == {}


def test_format_2_methods_buffers_and_cpuid() -> None:
    ecx = 1 << 0 | 1 << 19 | 1 << 20 | 1 << 28                 # sse3, sse4.1, sse4.2, avx
    edx = 1 << 26                                             # sse2
    data = (b"W2MJ" + struct.pack("<I", 2)
            + _cpuid_2([(1, 0, 0, 0, ecx, edx), (7, 0, 0, 1 << 5, 0, 0)])
            + _method_2(0x1000, "Intro_Script:Open_MenuLevel ()", "Assembly-CSharp", BODY)
            + _buffer_2(0x5000, 5, "generic_class_init", RET)
            + _buffer_2(0x6000, 3, "", RET))
    cap = parse(data)
    assert cap.format == 2 and cap.torn_tail_bytes == 0
    assert [(r.kind, r.image) for r in cap.records] == [
        ("method", "Assembly-CSharp"), ("specific_trampoline", ""), ("imt_trampoline", "")]
    f = cpu_features(cap)
    assert f["sse4_2"] and f["avx"] and f["avx2"] and not f["fma"] and not f["avx512f"]


def test_unknown_format_and_kind_are_refused() -> None:
    with pytest.raises(CaptureError, match="format 3"):
        parse(b"W2MJ" + struct.pack("<I", 3))
    with pytest.raises(CaptureError, match="record kind 9"):
        parse(b"W2MJ" + struct.pack("<II", 2, 9))


def test_lock_cmpxchg16b_matches_xed_and_names_its_confirmed_row() -> None:
    d = decode(bytes.fromhex("f0490fc708"))                  # lock cmpxchg16b [r8]; 25 sites in UnityPlayer
    assert (d.length, d.iclass) == (5, "CMPXCHG16B_LOCK")
    conf = json.loads((ROOT / "specs" / "x86_decode_confirmations.json").read_text())["by_row_sha256"]
    assert conf[d.row]["mn"] == "cmpxchg8b" and conf[d.row]["confirmed"]


def test_mov_moffs_is_refused_as_lift_error() -> None:
    with pytest.raises(LiftError, match="no operand template for MOV"):
        lift(bytes.fromhex("48a1") + bytes(8), 0x1000)       # 13 sites in UnityPlayer, 2 in mono
