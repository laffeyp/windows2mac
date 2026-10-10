# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""Sprint 022 grading surface: operands resolved from SDM Appendix A templates equal XED's."""

from __future__ import annotations

import pytest

from w2m.x86dec import decode
from w2m.x86ops import OperandError, normalize, operands
from w2m.xedref import ROOT, XED_KIT_GLOB, decode_many

# Expected strings are XED v2026.08.23 output (xedshim), normalized.
CASES = {
    "4889d8": "R:RAX;R:RBX",
    "488b4c2408": "R:RCX;M:-:RSP:INVALID:-:8:8",
    "8b0d93890100": "R:ECX;M:-:RIP:INVALID:-:100755:4",
    "4183c0f0": "R:R8D;I:1:f0",
    "488d0c8500000000": "R:RCX;A:INVALID:RAX:4:0",
    "d1e0": "R:EAX;I:1:1",
    "400fb6c7": "R:EAX;R:DIL",          # REX present: register 7 is DIL
    "0fb6c4": "R:EAX;R:AH",             # no REX: register 4 is AH
    "67488b00": "R:RAX;M:-:EAX:INVALID:-:0:8",
    "648b042528000000": "R:EAX;M:FS:INVALID:INVALID:-:40:4",
    "48b8efcdab8967452301": "R:RAX;I:8:123456789abcdef",
    "41ff24c4": "M:-:R12:RAX:8:0:8",
    "66ffa217cf2204": "M:-:RDX:INVALID:-:69390103:8",   # FF /4 is f64: 66 ignored
    "6663f0": "R:SI;R:AX",              # MOVSXD source is Ez
    "4887d1": "R:RCX;R:RDX",
}


@pytest.mark.parametrize(("hexbytes", "expected"), CASES.items())
def test_operands_match_recorded_xed_output(hexbytes: str, expected: str) -> None:
    g = decode(bytes.fromhex(hexbytes) + b"\0" * 15)
    assert g.fields is not None
    assert ";".join(map(normalize, operands(g.fields))) == expected


def test_row_without_template_refuses() -> None:
    g = decode(bytes.fromhex("c3"))
    assert g.fields is not None
    with pytest.raises(OperandError):
        operands(g.fields)


@pytest.mark.skipif(not list(ROOT.glob(XED_KIT_GLOB)), reason="XED kit not built")
def test_recorded_strings_are_still_xed_output() -> None:
    got = decode_many([bytes.fromhex(h) for h in CASES])
    assert [";".join(map(normalize, x.operands)) for x in got] == list(CASES.values())
