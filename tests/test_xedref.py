"""Sprint 017 grading surface: Intel XED as the primary reference decoder."""

from __future__ import annotations

from pathlib import Path

import pytest

from w2m.pe import parse
from w2m.x86dec import DecodeError, decode
from w2m.xedref import ROOT, XED_KIT_GLOB, decode_many

pytestmark = pytest.mark.skipif(not list(ROOT.glob(XED_KIT_GLOB)), reason="XED kit not built")
BINARIES = sorted((ROOT / "store" / "binaries").glob("*.exe"))


def test_xed_settles_the_disputed_cases() -> None:
    cases = {"f20f100d93890100": ("MOVSD_XMM", 8), "a5": ("MOVSD", 1), "f3c3": ("RET_NEAR", 2),
             "c5fc77": ("VZEROALL", 3), "c5f877": ("VZEROUPPER", 3), "f348ab": ("REP_STOSQ", 3),
             "f0ff05aabbccdd": ("INC_LOCK", 7)}
    got = decode_many([bytes.fromhex(h) for h in cases])
    assert [(x.iclass, x.length) for x in got] == list(cases.values())


def test_embedded_text_is_not_code() -> None:
    # OpenSSL credit strings inside libcrypto's .text, reached by static traversal
    x = decode_many([b"Camellia", b"by <appr"])
    assert not any(r.ok for r in x)


@pytest.mark.parametrize("path", BINARIES, ids=lambda p: p.name[:12])
def test_lengths_match_xed_on_probes(path: Path) -> None:
    from w2m.x86ref import walk
    data = path.read_bytes()
    img = parse(data)
    insns = list(walk(img, data).insns.values())
    raws = [img.read_rva(data, i.rva, 15) for i in insns]
    for ins, raw, x in zip(insns, raws, decode_many(raws)):
        assert x.ok and x.length == ins.size, (hex(ins.rva), ins.raw.hex(), x)
        try:
            assert decode(raw).length == x.length, (hex(ins.rva), raw.hex())
        except DecodeError:
            pytest.fail(f"spec gap in probe at 0x{ins.rva:x}")
