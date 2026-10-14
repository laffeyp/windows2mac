# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""The native PCMPISTRI (tools/guest_runtime/string_compare.c, 2026-10-13) against the model it replaces
(src/w2m/string_compare.py, judged against Intel's CPU in the class runs): built for the Mac and called through
ctypes, every imm8 on random operands with short strings, embedded zeros and high bytes, the same ECX and flags."""

from __future__ import annotations

import ctypes
import random
import shutil
import subprocess
import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))

from w2m import string_compare as sc  # noqa: E402


@pytest.mark.skipif(shutil.which("clang") is None, reason="clang builds the helper for the host")
def test_the_native_helper_equals_the_model_on_every_imm8(tmp_path: Path) -> None:
    lib_path = tmp_path / "libpcmpistri.so"
    subprocess.run(["clang", "-shared", "-fPIC", "-O2", "-std=c11", "-Wall", "-Werror", "-ffreestanding", "-o",
                    str(lib_path), str(ROOT / "tools" / "guest_runtime" / "string_compare.c")], check=True)
    lib = ctypes.CDLL(str(lib_path))
    lib.w2m_pcmpistri.restype = ctypes.c_uint64
    lib.w2m_pcmpistri.argtypes = [ctypes.c_char_p, ctypes.c_char_p, ctypes.c_uint64]
    rng = random.Random(7)
    alphabets = [b"\x00ab", b"\x00abc\xff\x80", bytes(range(256)), b"ab"]
    for imm in range(128):
        for _ in range(200):
            a, b = (bytes(rng.choice(rng.choice(alphabets)) for _ in range(16)) for _ in range(2))
            x, y = int.from_bytes(a, "little"), int.from_bytes(b, "little")
            res2, fl = sc.compare(x, y, imm, sc.implicit_length(x, imm), sc.implicit_length(y, imm))
            want = sc.index(res2, imm) | fl["C"] << 8 | fl["Z"] << 9 | fl["S"] << 10 | fl["O"] << 11
            assert lib.w2m_pcmpistri(a, b, imm) == want, (hex(imm), a.hex(), b.hex())
