# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""gen_last_error_values.py — write probes/last_error_values/values.h (Sprint 032).

The probe picks SetLastError arguments at run time from this table. Each entry pairs a 32-bit
value with the args_sha256 of its canonical rendering `SetLastError(<decimal>)` (vocabulary 0.4
note), so the probe reports the true hash of every argument it passes without hashing in the
guest. 256 distinct values: five boundary values, then values from random.Random(SEED).

Usage: uv run python scripts/gen_last_error_values.py [--check]
"""

from __future__ import annotations

import hashlib
import random
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / "probes" / "last_error_values" / "values.h"
SEED = 2026_10_09
BOUNDARY = (0, 1, 0x7FFF_FFFF, 0x8000_0000, 0xFFFF_FFFF)
COUNT = 256


def values() -> list[int]:
    rng = random.Random(SEED)
    out = list(BOUNDARY)
    while len(out) < COUNT:
        v = rng.getrandbits(32)
        if v not in out:
            out.append(v)
    return out


def args_sha256(v: int) -> str:
    return hashlib.sha256(f"SetLastError({v})".encode()).hexdigest()


def render() -> str:
    vals = values()
    value_rows = "\n".join(f"    0x{v:08x}u," for v in vals)
    hash_rows = "\n".join(f'    "sha256:{args_sha256(v)}",' for v in vals)
    return ("// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0\n// Copyright (C) 2026 Peter Laffey\n"
            f"/* values.h — written by scripts/gen_last_error_values.py (seed {SEED}); do not edit.\n"
            f" * VALUE[i] is a SetLastError argument; ARGS_SHA256[i] is sha256 of\n"
            f" * \"SetLastError(<VALUE[i] in decimal>)\". Two arrays, so indexing needs no multiply. */\n"
            f"#define VALUE_COUNT {COUNT}\n"
            f"static const unsigned VALUE[VALUE_COUNT] = {{\n{value_rows}\n}};\n"
            f"static const char ARGS_SHA256[VALUE_COUNT][72] = {{\n{hash_rows}\n}};\n")


def main(argv: list[str]) -> int:
    text = render()
    if "--check" in argv:
        return 0 if OUT.read_text() == text else 1
    OUT.write_text(text)
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
