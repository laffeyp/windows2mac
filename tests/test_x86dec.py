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
    assert n > 20_000
    assert not d, d.most_common(10)


@pytest.mark.parametrize(("hexbytes", "length", "mnemonic"), [
    ("90", 1, "nop"), ("4190", 2, "xchg"), ("f390", 2, "pause"),
    ("48b88877665544332211", 10, "movabs"), ("b844332211", 5, "mov"),
    ("f2410f114bd8", 6, "movsd"),          # SSE movsd: F2 is mandatory, not repne
    ("f348ab", 3, "rep stosq"), ("f0ff05aabbccdd", 7, "lock inc"),
    ("c5f877", 3, "vzeroupper"), ("4898", 2, "cdqe"), ("6698", 2, "cbw"),
    ("0f01d0", 3, "xgetbv"), ("e800000000", 5, "call"), ("7400", 2, "je"),
])
def test_known_encodings(hexbytes: str, length: int, mnemonic: str) -> None:
    got = decode(bytes.fromhex(hexbytes) + b"\0" * 15)
    assert (got.length, got.mnemonic) == (length, mnemonic)


def test_unknown_opcode_is_reported() -> None:
    with pytest.raises(DecodeError, match="no entry"):
        decode(bytes.fromhex("0f0e") + b"\0" * 15)        # femms (3DNow!) is not in the spec
