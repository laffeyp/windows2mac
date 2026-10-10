# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""Sprint 027: property-based fuzzing of the two parsers that read shipped game bytes
(review 2026-10-08 §5: parsers of untrusted input). Hypothesis 6.168.5."""

from __future__ import annotations

from pathlib import Path

import pytest
from hypothesis import HealthCheck, given, settings
from hypothesis import strategies as st

from w2m.pe import PeError, parse
from w2m.x86dec import DecodeError, decode

BINARIES = sorted((Path(__file__).resolve().parent.parent / "store" / "binaries").glob("*.exe"))
LEGACY = [0x66, 0x67, 0xF0, 0xF2, 0xF3, 0x2E, 0x64, 0x65]


@settings(max_examples=5000, deadline=None)
@given(st.binary(min_size=0, max_size=20))
def test_decode_raises_only_decode_errors(b: bytes) -> None:
    try:
        d = decode(b)
    except DecodeError:
        return
    assert 1 <= d.length <= min(15, len(b))


@settings(max_examples=5000, deadline=None)
@given(st.lists(st.sampled_from(LEGACY + list(range(0x40, 0x50))), max_size=16),
       st.sampled_from([0x0F, 0x62, 0x8F, 0xC4, 0xC5, 0x38, 0x3A]), st.binary(max_size=12))
def test_decode_prefix_heavy_inputs(prefixes: list[int], esc: int, tail: bytes) -> None:
    """Biased toward prefix, REX and escape bytes, where length decoding is hardest."""
    b = bytes(prefixes) + bytes([esc]) + tail
    try:
        d = decode(b)
    except DecodeError:
        return
    assert 1 <= d.length <= min(15, len(b))


@pytest.mark.skipif(not BINARIES, reason="no probe binaries")
@settings(max_examples=3000, deadline=None, suppress_health_check=[HealthCheck.too_slow])
@given(st.data())
def test_parse_raises_only_pe_errors(data: st.DataObject) -> None:
    """Mutations of a real image: byte flips in the headers and directories, and truncation."""
    image = bytearray(BINARIES[0].read_bytes())
    for _ in range(data.draw(st.integers(1, 8))):
        i = data.draw(st.integers(0, 0x400 - 1))          # headers, section table
        image[i] = data.draw(st.integers(0, 255))
    cut = data.draw(st.one_of(st.none(), st.integers(0, len(image))))
    blob = bytes(image if cut is None else image[:cut])
    try:
        parse(blob)
    except PeError:
        pass
