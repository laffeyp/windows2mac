# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""Sprint 073: the C encoders (tools/vm_compiler/a64_encoder.c) against src/w2m/a64_encoder.py.

Every public function of the Python is called over a domain per argument, and its C counterpart with the same
arguments: where the Python returns a 32-bit word the C must return the same word, and where the Python raises the C
must return 0. Where the Python returns a value outside 32 bits (a field past its width) the C is not judged. The C is
also compiled with the guest runtime's flags, as the compiler inside the VM will be.
"""

from __future__ import annotations

import ctypes
import inspect
import itertools
import random
import shutil
import subprocess
from collections.abc import Callable, Iterator
from pathlib import Path
from typing import Any

import pytest

from w2m import a64_encoder as a64
from w2m import guest_runtime

ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / "tools" / "vm_compiler" / "a64_encoder.c"
CLANG = shutil.which("clang")
pytestmark = pytest.mark.skipif(CLANG is None, reason="clang not available")

REGISTERS = list(range(-1, 34))
SYSTEM_REGISTERS = [a64.ESR_EL1, a64.FAR_EL1, a64.ELR_EL1, a64.SPSR_EL1, a64.FPCR, a64.FPSR]
# the string tables, in the order of the C enums
NAMES: dict[str, dict[str, list[str]]] = {
    "cset": {"cond": list(a64.COND)}, "bcond": {"cond": list(a64.COND)},
    "fcm": {"cond": ["eq", "ge", "gt"]},
    "three_same": {"name": list(a64.THREE_SAME)}, "three_same_d": {"name": list(a64.THREE_SAME)},
    "fp_scalar": {"op": list(a64._FP2)}, "fp_vector": {"op": list(a64._FPV)}, "fp_fused": {"op": list(a64._FP3)},
}
BOOLEANS = {"scaled", "upper", "signed", "double", "signal"}


def _offsets() -> list[int]:
    out = set(range(-300, 301))
    for k in range(29):
        for d in (-17, -16, -9, -8, -5, -4, -1, 0, 1, 4, 8, 16):
            out |= {(1 << k) + d, -(1 << k) + d}
    for s in (1, 2, 4, 8, 16):
        out |= {s * 4095, s * 4096, s * 4096 - s, s * 4095 + 1, -s * 64, -s * 65, s * 63, s * 64}
    return sorted(out)


DOMAINS: dict[str, list[Any]] = {
    "imm12": [-1, 0, 1, 2, 7, 4095, 4096, 4097] + list(range(0, 4096, 37)),
    "imm16": [-1, 0, 1, 2, 0x7FFF, 0x8000, 0xFFFF, 0x10000, 0x12345],
    "imm8": list(range(-1, 258)),
    "hw": [-1, 0, 1, 2, 3, 4, 5],
    "shift": list(range(-1, 131)),
    "amount": list(range(-2, 70)),
    "offset": _offsets(),
    "size": [-1, 0, 1, 2, 3, 4, 5, 8, 16],
    "bits": [-1, 0, 1, 2, 31, 32, 33, 63, 64, 65],
    "lsb": list(range(-2, 67)), "width": list(range(-2, 67)),
    "immr": list(range(-1, 65)), "imms": list(range(-1, 65)),
    "bit": list(range(-1, 66)),
    "lane": list(range(-1, 18)), "src_lane": list(range(-1, 18)), "index": list(range(-1, 18)),
    "dst_size": [2, 4, 8, 16], "src_size": [2, 4, 8, 16],
    "reg": SYSTEM_REGISTERS,
}
SAMPLES = 20000


def _domain(fn: str, param: str) -> list[Any]:
    if fn in NAMES and param in NAMES[fn]:
        return NAMES[fn][param]
    if param in BOOLEANS:
        return [False, True]
    if param == "value":
        rng = random.Random(7301)
        halves = [0, 1, 0x8000, 0xFFFF]
        return [sum(h << 16 * i for i, h in enumerate(c)) for c in itertools.product(halves, repeat=4)] + [
            rng.getrandbits(64) for _ in range(200)]
    if param in DOMAINS:
        return DOMAINS[param]
    assert param.startswith(("r", "v")), f"{fn}: no domain for {param}"
    return REGISTERS


def _arguments(fn: str, params: list[str]) -> Iterator[tuple[Any, ...]]:
    """Every combination when they number SAMPLES or fewer; else each argument over its whole domain with the others
    drawn at random, then random combinations up to SAMPLES."""
    domains = [_domain(fn, p) for p in params]
    total = 1
    for d in domains:
        total *= len(d)
    if total <= SAMPLES:
        yield from itertools.product(*domains)
        return
    rng = random.Random(f"073 {fn}")
    for i, d in enumerate(domains):
        for v in d:
            for _ in range(3):
                yield tuple(v if j == i else rng.choice(e) for j, e in enumerate(domains))
    for _ in range(SAMPLES):
        yield tuple(rng.choice(d) for d in domains)


def _c_arguments(fn: str, params: list[str], args: tuple[Any, ...]) -> list[int]:
    out: list[int] = []
    for p, a in zip(params, args, strict=True):
        if fn in NAMES and p in NAMES[fn]:
            out.append(a64.COND[a] if p == "cond" and fn != "fcm" else NAMES[fn][p].index(a))
        elif p == "reg":
            out.extend(a)
        else:
            out.append(int(a))
    return out


def _functions() -> list[tuple[str, Callable[..., Any]]]:
    return sorted((n, f) for n, f in vars(a64).items()
                  if inspect.isfunction(f) and f.__module__ == a64.__name__ and not n.startswith("_"))


@pytest.fixture(scope="module")
def library(tmp_path_factory: pytest.TempPathFactory) -> ctypes.CDLL:
    assert CLANG is not None
    out = tmp_path_factory.mktemp("a64c") / "a64_encoder.so"
    subprocess.run([CLANG, "-shared", "-fPIC", "-O2", "-std=c11", "-Wall", "-Werror", "-o", str(out), str(SOURCE)],
                   check=True, capture_output=True)
    return ctypes.CDLL(str(out))


def test_builds_with_the_guest_runtime_flags(tmp_path: Path) -> None:
    assert CLANG is not None
    r = subprocess.run([CLANG, *guest_runtime.CFLAGS, "-c", "-o", str(tmp_path / "a64_encoder.o"), str(SOURCE)],
                       capture_output=True, text=True)
    assert r.returncode == 0, r.stderr


def test_every_python_encoder_has_a_c_function(library: ctypes.CDLL) -> None:
    missing = [n for n, _ in _functions() if not hasattr(library, "a64_" + n.rstrip("_"))]
    assert not missing, missing


def _call_python(f: Callable[..., Any], args: tuple[Any, ...]) -> int | None:
    """The word, or None where the Python raised."""
    try:
        return int(f(*args))
    except (AssertionError, ValueError, KeyError, ZeroDivisionError):
        return None


@pytest.mark.parametrize("name", [n for n, _ in _functions() if n != "mov_imm"])
def test_c_equals_python(library: ctypes.CDLL, name: str) -> None:
    f = getattr(a64, name)
    params = list(inspect.signature(f).parameters)
    c = getattr(library, "a64_" + name.rstrip("_"))
    c.restype = ctypes.c_uint32
    compared = rejected = 0
    bad: list[tuple[Any, ...]] = []
    for args in _arguments(name, params):
        want = _call_python(f, args)
        if want is not None and not 0 <= want < 1 << 32:
            continue
        got = c(*(ctypes.c_int(v) for v in _c_arguments(name, params, args)))
        if want is None:
            rejected += 1
            if got != 0:
                bad.append((args, "python raised", f"{got:08x}"))
        else:
            compared += 1
            if got != want:
                bad.append((args, f"{want:08x}", f"{got:08x}"))
    assert not bad, (len(bad), bad[:10])
    assert compared > 0, "no argument gave the python a word"


def test_mov_imm(library: ctypes.CDLL) -> None:
    c = library.a64_mov_imm
    c.restype = ctypes.c_int
    c.argtypes = [ctypes.POINTER(ctypes.c_uint32), ctypes.c_int, ctypes.c_uint64]
    buf = (ctypes.c_uint32 * 4)()
    bad = []
    for rd in REGISTERS:
        for value in _domain("mov_imm", "value"):
            try:
                want = a64.mov_imm(rd, value)
            except ValueError:
                want = []
            n = c(buf, rd, value)
            if list(buf[:n]) != want:
                bad.append((rd, hex(value), want, list(buf[:n])))
    assert not bad, bad[:10]
