# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""Sprint 077: the compiler inside the VM (tools/vm_compiler/vm_compiler.c) against vm_backend.compile_insn.

For one recorded instruction per key and stencil of the committed table (tests/data/stencil_samples.txt.gz, written
by scripts/gen_stencil_table.py --samples), the C key equals stencil_census.decoded_form's key as key_bytes writes
it, the C values equal the Python's, and the C words equal compile_insn's at a base and exit drawn at random. The
hole expressions and features equal the Python's over a spread of values; the C builds with the guest runtime's
flags.
"""

from __future__ import annotations

import ctypes
import gzip
import random
import shutil
import subprocess
import sys
from pathlib import Path
from typing import Any

import pytest

from w2m import guest_runtime, vm_backend, x86dec
from w2m.lift import lift

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))

import gen_stencil_table as g  # noqa: E402
import stencil_census as sc  # noqa: E402
import stencil_table as st  # noqa: E402

SOURCE = ROOT / "tools" / "vm_compiler"
CLANG = shutil.which("clang")
pytestmark = pytest.mark.skipif(CLANG is None, reason="clang not available")
NO_STENCIL, DECODE = -2, -1


class Flags(ctypes.Structure):
    _fields_ = [("nzcv_from", ctypes.c_int), ("live_after", ctypes.c_int), ("nzcv_out", ctypes.c_int)]


U64, U32, INT = ctypes.c_uint64, ctypes.c_uint32, ctypes.c_int


class BlockIn(ctypes.Structure):
    _fields_ = [("start", U64), ("mem", ctypes.c_char_p), ("mem_base", U64), ("mem_size", U64),
                ("thunks", ctypes.POINTER(U64)), ("nthunks", INT), ("base", U64), ("exit_va", U64),
                ("slice_va", U64), ("symbols", ctypes.POINTER(U64)), ("live_out", INT)]


class Mark(ctypes.Structure):
    _fields_ = [("host", U64), ("guest", U64)]


class BlockOut(ctypes.Structure):
    _fields_ = [("words", ctypes.POINTER(U32)), ("max_words", INT), ("marks", ctypes.POINTER(Mark)),
                ("nwords", INT), ("nmarks", INT), ("count", INT), ("cut", INT), ("end", U64)]


HARNESS = """#define VMC_NAMES
#include "vm_compiler.c"
const int vmc_symbols = VMC_SYMBOLS, vmc_expressions = VMC_EXPRESSIONS, vmc_features = VMC_FEATURES;
u64 vmc_test_expression(int e, u64 u) { return vmc_expression(e, u); }
u64 vmc_test_feature(int f, u64 u) { return vmc_feature(f, u); }
u32 vmc_test_implicit(int row) { return VMC_IMPLICIT[row]; }
"""


class Compiler:
    def __init__(self, path: Path) -> None:
        lib = self.lib = ctypes.CDLL(str(path))
        lib.vmc_key.argtypes = [ctypes.c_char_p, ctypes.c_int, ctypes.c_uint64, ctypes.POINTER(Flags),
                                ctypes.POINTER(ctypes.c_uint8), ctypes.POINTER(ctypes.c_uint64),
                                ctypes.POINTER(ctypes.c_int), ctypes.POINTER(ctypes.c_int)]
        lib.vmc_compile.argtypes = [ctypes.c_char_p, ctypes.c_int, ctypes.c_uint64, ctypes.c_uint64, ctypes.c_uint64,
                                    ctypes.POINTER(Flags), ctypes.POINTER(ctypes.c_uint64),
                                    ctypes.POINTER(ctypes.c_uint32), ctypes.c_int]
        lib.vmc_test_expression.restype = lib.vmc_test_feature.restype = ctypes.c_uint64
        lib.vmc_test_expression.argtypes = lib.vmc_test_feature.argtypes = [ctypes.c_int, ctypes.c_uint64]
        lib.vmc_test_implicit.restype = ctypes.c_uint32
        lib.vmc_block.argtypes = [ctypes.POINTER(BlockIn), ctypes.POINTER(BlockOut)]
        n = ctypes.c_int.in_dll(lib, "vmc_symbols").value
        self.symbol_names = [(s or b"").decode() for s in (ctypes.c_char_p * n).in_dll(lib, "vmc_symbol_name")]
        self.symbols = {name: 0x1_2340_0000 + 0x1000 * i for i, name in enumerate(self.symbol_names)}
        self.c_symbols = (ctypes.c_uint64 * max(n, 1))(*self.symbols.values())
        rows = g.names_of("x86_row_name")
        self.implicit = {}
        for i, row in enumerate(rows):
            m = lib.vmc_test_implicit(i)
            self.implicit[row] = frozenset(b if b < 16 else 1000 + b - 16 for b in range(32) if m >> b & 1)

    def key(self, code: bytes, address: int, flags: Flags) -> tuple[int, bytes, list[int]]:
        key = (ctypes.c_uint8 * 160)()
        values = (ctypes.c_uint64 * 24)()
        n, length = ctypes.c_int(), ctypes.c_int()
        k = self.lib.vmc_key(code, len(code), address, ctypes.byref(flags), key, values, ctypes.byref(n),
                             ctypes.byref(length))
        return k, bytes(key[:max(k, 0)]), list(values[:n.value])

    def block(self, start: int, mem: bytes, live_out: int, base: int, exit_va: int, slice_va: int,
              thunks: list[int] = []) -> tuple[int, BlockOut, list[int], list[tuple[int, int]]]:
        words, marks = (U32 * 8192)(), (Mark * 80)()
        t = (U64 * max(len(thunks), 1))(*sorted(thunks))
        bi = BlockIn(start, mem, start, len(mem), t, len(thunks), base, exit_va, slice_va, self.c_symbols, live_out)
        bo = BlockOut(words, 8192, marks, 0, 0, 0, 0, 0)
        n = self.lib.vmc_block(ctypes.byref(bi), ctypes.byref(bo))
        return n, bo, list(words[:max(n, 0)]), [(m.host, m.guest) for m in marks[:bo.nmarks]]

    def compile(self, code: bytes, address: int, base: int, exit_va: int, flags: Flags) -> list[int] | int:
        out = (ctypes.c_uint32 * 512)()
        n = self.lib.vmc_compile(code, len(code), address, base, exit_va, ctypes.byref(flags), self.c_symbols, out,
                                 512)
        return n if n < 0 else list(out[:n])


@pytest.fixture(scope="module")
def compiler(tmp_path_factory: pytest.TempPathFactory) -> Compiler:
    assert CLANG is not None
    tmp = tmp_path_factory.mktemp("vmc")
    (tmp / "harness.c").write_text(HARNESS)
    out = tmp / "vm_compiler.so"
    subprocess.run([CLANG, "-shared", "-fPIC", "-O2", "-std=c11", "-Wall", "-Werror", f"-I{SOURCE}", "-o", str(out),
                    str(tmp / "harness.c"), str(SOURCE / "x86_decode.c"), str(SOURCE / "a64_encoder.c")],
                   check=True, capture_output=True)
    return Compiler(out)


def samples() -> list[tuple[bytes, int, Flags, Any]]:
    out = []
    with gzip.open(ROOT / "tests" / "data" / "stencil_samples.txt.gz", "rt") as f:
        for line in f:
            code, address, nzcv_from, live_after, nzcv_out = line.split()
            out.append((bytes.fromhex(code), int(address, 16),
                        Flags(int(nzcv_from), int(live_after), int(nzcv_out)), g.NZCV_FROM[int(nzcv_from)]))
    return out


def test_builds_with_the_guest_runtime_flags(tmp_path: Path) -> None:
    assert CLANG is not None
    r = subprocess.run([CLANG, *guest_runtime.CFLAGS, "-c", "-o", str(tmp_path / "vm_compiler.o"),
                        str(SOURCE / "vm_compiler.c")], capture_output=True, text=True)
    assert r.returncode == 0, r.stderr


def test_the_expressions_and_features_equal_the_python(compiler: Compiler) -> None:
    rng = random.Random(7701)
    values = list(range(-300, 1100)) + [rng.getrandbits(64) for _ in range(500)] + [
        (1 << k) + d for k in range(64) for d in (-1, 0, 1)] + [-(1 << k) for k in range(64)]
    mask = (1 << 64) - 1
    for i, (_, name, fn) in enumerate(sc.EXPRESSIONS):
        for v in values:
            sv = v - (1 << 64) if v >= 1 << 63 else v           # the value as decoded_form gives it: signed
            assert compiler.lib.vmc_test_expression(i, v & mask) == fn(sv) & mask, (name, v)
    for i, (name, fn) in enumerate(st.FEATURES):
        for v in values:
            assert compiler.lib.vmc_test_feature(i, v & mask) == int(fn(v & mask)), (name, v)


def test_every_key_of_the_table_compiles_as_compile_insn(compiler: Compiler) -> None:
    rng = random.Random(7702)
    rows = 0
    for code, address, flags, nzcv_from in samples():
        ins = lift(code, address)
        call = (ins, 0, nzcv_from, flags.live_after, bool(flags.nzcv_out))
        row = x86dec.decode(code).row or ""
        key, values, _ = sc.decoded_form(call, code, compiler.implicit.get(row, frozenset()))
        k, kb, cvalues = compiler.key(code, address, flags)
        assert k > 0 and kb == g.key_bytes(key), code.hex()
        assert cvalues == [v & (1 << 64) - 1 for v in values], code.hex()
        base = rng.randrange(1 << 30, 1 << 40) * 4
        exit_va = base + rng.randrange(-(1 << 24), 1 << 24) * 4
        a = vm_backend.Assembler(base)
        vm_backend.compile_insn(a, ins, exit_va, compiler.symbols, nzcv_from, flags.live_after, bool(flags.nzcv_out))
        assert compiler.compile(code, address, base, exit_va, flags) == a.finish(), code.hex()
        rows += 1
    assert rows > 38_000


def test_a_form_outside_the_table_goes_to_the_host(compiler: Compiler) -> None:
    add = bytes.fromhex("4801d8")                                   # add rax, rbx
    assert compiler.compile(add, 0x1_4000_1000, 0x10_0000, 0x20_0000, Flags(0, 0x3E, 1)) == NO_STENCIL
    assert compiler.compile(bytes.fromhex("48"), 0x1_4000_1000, 0x10_0000, 0x20_0000, Flags(0, 0, 0)) == DECODE


def block_samples() -> list[tuple[int, int, bytes]]:
    with gzip.open(ROOT / "tests" / "data" / "block_samples.txt.gz", "rt") as f:
        return [(int(a, 16), int(live), bytes.fromhex(code)) for a, live, code in (line.split() for line in f)]


def lifted(start: int, mem: bytes) -> list[Any]:
    """The block's instructions as Monitor._compile takes them: up to and including a branch."""
    out: list[Any] = []
    off = 0
    while off < len(mem) and len(out) < vm_backend.MAX_BLOCK_INSNS:
        n = x86dec.decode(mem[off:]).length
        ins = lift(mem[off:off + n], start + off)
        out.append(ins)
        off += n
        if any(op[0] == "br" for op in ins.ops):
            break
    return out


def test_blocks_compile_as_assemble_block(compiler: Compiler) -> None:
    """Sprint 078: blocks made from the record, each compiled by vmc_block and by assemble_block over the
    instructions the C took; the flags live at the end are the sample's where the C took them all, else every flag
    (the C stopped before an instruction with no stencil for its flags)."""
    rng = random.Random(7801)
    counts: dict[str, int] = {"equal": 0, "first has no stencil": 0, "cut": 0}
    for start, live_out, mem in block_samples():
        insns = lifted(start, mem)
        base = rng.randrange(1 << 30, 1 << 40) * 4
        exit_va, slice_va = (base + rng.randrange(-(1 << 24), 1 << 24) * 4 for _ in range(2))
        n, bo, words, marks = compiler.block(start, mem, live_out, base, exit_va, slice_va)
        if n == NO_STENCIL:
            counts["first has no stencil"] += 1
            continue
        assert n > 0, (hex(start), n)
        k = bo.count
        counts["cut"] += k < len(insns)
        a, py_marks, _, _, _ = vm_backend.assemble_block(insns[:k], base, exit_va, slice_va, compiler.symbols,
                                                         live_out if k == len(insns) else vm_backend.ALL_FLAGS)
        assert words == a.finish() and marks == py_marks, hex(start)
        assert bo.end == insns[k - 1].address + insns[k - 1].length
        counts["equal"] += 1
    assert counts["equal"] > 2900, counts


def test_a_thunk_ends_the_block(compiler: Compiler) -> None:
    start, _, mem = next((s, lv, m) for s, lv, m in block_samples() if len(lifted(s, m)) >= 3
                         and compiler.block(s, m, vm_backend.ALL_FLAGS, 0x10_0000, 0x20_0000, 0x30_0000)[0] > 0)
    insns = lifted(start, mem)
    thunk = insns[2].address
    n, bo, words, marks = compiler.block(start, mem, vm_backend.ALL_FLAGS, 0x10_0000, 0x20_0000, 0x30_0000, [thunk])
    assert n > 0 and bo.count == 2 and bo.end == thunk
    a, py_marks, _, _, _ = vm_backend.assemble_block(insns[:2], 0x10_0000, 0x20_0000, 0x30_0000, compiler.symbols,
                                                     vm_backend.ALL_FLAGS)
    assert words == a.finish() and marks == py_marks
