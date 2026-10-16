# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""Sprint 089: symbolic templates (src/w2m/symbolic_lift.py) against lift().

Every instruction of the stencil samples (tests/data/stencil_samples.txt.gz: one per key of the game's record) and of
two random encodings per decoder spec row: the template for its class, shape and mode, filled with its values, equals
lift()'s ops. A plant that drops the immediate's sign extension from the symbolic operand reader makes them differ.
scripts/template_judge.py runs the same judge over the whole record and the class case sets.
"""

from __future__ import annotations

import gzip
import random
import sys
from pathlib import Path
from typing import Any

import pytest

from w2m import symbolic_lift
from w2m.lift import LiftError, lift

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "tests"))


def _instructions() -> list[tuple[bytes, int]]:
    import test_x86_decode_c
    out = []
    with gzip.open(ROOT / "tests" / "data" / "stencil_samples.txt.gz", "rt") as f:
        for line in f:
            code, address = line.split()[:2]
            out.append((bytes.fromhex(code), int(address, 16)))
    rng = random.Random(8902)
    out += [(raw, 0x1_4000_0000 + rng.randrange(1 << 24)) for raw in test_x86_decode_c.row_samples(rng, 2)]
    return out


def _judge() -> dict[str, int]:
    made: dict[Any, Any] = {}
    counts = {"equal": 0, "differ": 0, "lift refuses": 0}
    for code, address in _instructions():
        try:
            want = lift(code, address).ops
        except LiftError:
            counts["lift refuses"] += 1
            continue
        got = symbolic_lift.lifted_by_template(code + bytes(15), address, made)
        counts["equal" if got == want else "differ"] += 1
    return counts


def test_filled_templates_equal_lift() -> None:
    counts = _judge()
    assert counts["differ"] == 0 and counts["equal"] > 20_000, counts


def test_a_template_without_the_immediates_sign_extension_differs(monkeypatch: pytest.MonkeyPatch) -> None:
    real = symbolic_lift._sym_operand

    def planted(b: Any, s: str, next_ip: Any, width_hint: int | None = None) -> Any:
        return real(b, s, next_ip, None) if str(s).startswith("I:") else real(b, s, next_ip, width_hint)

    monkeypatch.setattr(symbolic_lift, "_sym_operand", planted)
    assert _judge()["differ"] > 0
