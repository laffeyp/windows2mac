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
from collections import Counter
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
NO_STENCIL, DECODE, GUARDED = -2, -1, -7


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


def left_to_host(ins: Any) -> bool:
    """An iclass lowered anew since the table was derived (vm_backend.HOST_LOWERED: PAUSE's yield, Sprint 096; the
    native PACKSSDW, PMULLD and FNSTCW, Sprint 098): the table leaves it out and the host compiles it."""
    return bool(ins.iclass in vm_backend.HOST_LOWERED)


def test_every_key_of_the_table_compiles_as_compile_insn(compiler: Compiler) -> None:
    rng = random.Random(7702)
    rows = 0
    for code, address, flags, nzcv_from in samples():
        ins = lift(code, address)
        if left_to_host(ins):
            continue
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
    assert rows > 21_000


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
        if n in (NO_STENCIL, GUARDED):                   # the first instruction goes to the host
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
    """A thunk at a block's third instruction ends it after two (the flags live there all of them, as the successor
    scan gives at a thunk), or sooner where an instruction has no stencil for its new flags; either way the words and
    marks are assemble_block's over the instructions taken."""
    ended_at_thunk = 0
    for start, _, mem in block_samples():
        insns = lifted(start, mem)
        if len(insns) < 3:
            continue
        thunk = insns[2].address
        n, bo, words, marks = compiler.block(start, mem, vm_backend.ALL_FLAGS, 0x10_0000, 0x20_0000, 0x30_0000,
                                             [thunk])
        if n < 0:
            continue
        assert bo.count <= 2 and (bo.count == 2 or bo.cut < 0)
        ended_at_thunk += bo.count == 2 and bo.end == thunk
        a, py_marks, _, _, _ = vm_backend.assemble_block(insns[:bo.count], 0x10_0000, 0x20_0000, 0x30_0000,
                                                         compiler.symbols, vm_backend.ALL_FLAGS)
        assert words == a.finish() and marks == py_marks, hex(start)
    assert ended_at_thunk > 1000


def test_the_case_sets_compile_as_compile_insn(compiler: Compiler) -> None:
    """Sprint 079, a judge held out from the table: the class case sets' instructions (Intel-checked, random operands,
    none drawn from the game record), each at two flag contexts. Where the C compiles one, its words equal
    compile_insn's; elsewhere it has no stencil, or a guard sends it to the host. On Sprint 078's table this found 144
    wrong compiles in seven SSE shift classes (a register taken for a constant)."""
    import class_run
    import semantics_differential
    rng = random.Random(7901)
    counts = {"equal": 0, "no stencil": 0, "guarded": 0}
    for k in [k for k in semantics_differential.CASE_SETS if k in class_run.REFERENCES]:
        for case in class_run._built(k):
            code = case.code[:x86dec.decode(case.code).length]
            address = 0x1_4000_0000 + rng.randrange(1 << 20) * 16
            ins = lift(code, address)
            if left_to_host(ins):
                continue
            for live in (vm_backend.ALL_FLAGS, 0):
                base = rng.randrange(1 << 30, 1 << 40) * 4
                exit_va = base + rng.randrange(-(1 << 24), 1 << 24) * 4
                got = compiler.compile(code, address, base, exit_va, Flags(0, live, 0))
                if got == NO_STENCIL:
                    counts["no stencil"] += 1
                    continue
                if got == GUARDED:
                    counts["guarded"] += 1
                    continue
                a = vm_backend.Assembler(base)
                vm_backend.compile_insn(a, ins, exit_va, compiler.symbols, None, live, False)
                assert got == a.finish(), (ins.iclass, code.hex())
                counts["equal"] += 1
    # the judge must compare many: 30,000 before Sprint 098, whose x87 FLD, FSTP and FSQRT (14,095 cases, two flag
    # contexts) are vm_backend.HOST_LOWERED, compiled by the host and judged by their class runs instead
    assert counts["equal"] > 28_000, counts


def test_canonical_flags_compile_as_the_raw_arguments() -> None:
    """Sprint 080 (the review's E2, pinned): an instruction compiled with flag arguments it cannot read
    (stencil_census.canonical_flags) gives the words of their canonical form."""
    import test_x86_decode_c
    rng = random.Random(8001)
    kinds = [None, *vm_backend.NZCV_CONDITIONS]
    codes = [c for c, _, _, _ in samples()[::8]] + test_x86_decode_c.row_samples(rng, 3)
    changed = 0
    for raw in codes:
        try:
            code = raw[:x86dec.decode(raw).length]
            ins = lift(code, 0x1_4000_1230)
        except Exception:
            continue
        for _ in range(3):
            args = (rng.choice(kinds), rng.randrange(64), rng.random() < 0.5)
            canonical = sc.canonical_flags(ins, *args)
            if canonical == args:
                continue
            changed += 1
            words = []
            for flags in (args, canonical):
                a = vm_backend.Assembler(0x10_0000_0000)
                vm_backend.compile_insn(a, ins, 0x10_0001_0000, {}, *flags)
                words.append(a.finish())
            assert words[0] == words[1], (ins.iclass, code.hex(), args)
    assert changed > 5_000


def _spec_row_judge(compiler: Compiler) -> dict[str, int]:
    """The review's E1, seeded: random encodings of every decoder spec row at unaligned addresses, under the samples'
    commonest flag contexts and the two defaults."""
    import collections
    import test_x86_decode_c
    rng = random.Random(424242)
    common = collections.Counter((f.nzcv_from, f.live_after, f.nzcv_out) for _, _, f, _ in samples())
    contexts = list(dict.fromkeys([c for c, _ in common.most_common(6)] + [(0, vm_backend.ALL_FLAGS, 0), (0, 0, 0)]))
    counts = {"equal": 0, "differ": 0, "no stencil": 0, "guarded": 0}
    for raw in test_x86_decode_c.row_samples(rng, 10):
        try:
            code = raw[:x86dec.decode(raw).length]
            address = 0x1_4000_0000 + rng.randrange(1 << 24)
            ins = lift(code, address)
        except Exception:
            continue
        if left_to_host(ins):
            continue
        for nf, live, no in contexts:
            base = rng.randrange(1 << 30, 1 << 40) * 4
            exit_va = base + rng.randrange(-(1 << 22), 1 << 22) * 4
            got = compiler.compile(code, address, base, exit_va, Flags(nf, live, no))
            if got in (NO_STENCIL, GUARDED):
                counts["no stencil" if got == NO_STENCIL else "guarded"] += 1
                continue
            a = vm_backend.Assembler(base)
            vm_backend.compile_insn(a, ins, exit_va, compiler.symbols, g.NZCV_FROM[nf], live, bool(no))
            counts["equal" if got == a.finish() else "differ"] += 1
    return counts


def test_spec_row_encodings_compile_as_compile_insn(compiler: Compiler) -> None:
    counts = _spec_row_judge(compiler)
    assert counts["differ"] == 0 and counts["equal"] > 4_000, counts      # 4,701 on the Sprint 080 table


GUARD_PLANT = [("            return VMC_GUARDED;\n    }\n    const struct vmc_stencil", "            ;\n    }\n    const struct vmc_stencil"),
               ("                        return VMC_GUARDED;\n", "                        ;\n")]


def test_without_its_guards_the_c_compiles_wrong_code(tmp_path: Path) -> None:
    """A plant: vm_compiler.c with its guards and its checked holes' agreement taken out. The judges outside the
    table's sources then find wrong words (the guards are what keep them at 0)."""
    assert CLANG is not None
    text = (SOURCE / "vm_compiler.c").read_text()
    for old, new in GUARD_PLANT:
        assert text.count(old) == 1, old
        text = text.replace(old, new)
    (tmp_path / "vm_compiler.c").write_text(text)
    (tmp_path / "harness.c").write_text(HARNESS)
    out = tmp_path / "planted.so"
    subprocess.run([CLANG, "-shared", "-fPIC", "-O2", "-std=c11", "-w", f"-I{tmp_path}", f"-I{SOURCE}", "-o", str(out),
                    str(tmp_path / "harness.c"), str(SOURCE / "x86_decode.c"), str(SOURCE / "a64_encoder.c")],
                   check=True, capture_output=True)
    assert _spec_row_judge(Compiler(out))["differ"] > 0


def _scan_judges(compiler: Compiler) -> tuple[Counter[str], Counter[str], list[str]]:
    """Sprint 082: the successor scan in C against VM._flags_live_in and _flags_live_out (successor_scan_judge), over
    the record's memory (tests/data/scan_samples.txt.gz) and over spec-row encodings laid out with jumps."""
    import successor_scan_judge as sj
    c = sj.CScan(compiler.lib)
    record, spec, examples = Counter[str](), Counter[str](), list[str]()
    with gzip.open(ROOT / "tests" / "data" / "scan_samples.txt.gz", "rt") as f:
        for line in f:
            model, scan = sj.sample_model(line)
            sj.judge(c, model, [scan], record, examples)
    rng = random.Random(8202)
    encodings = sj.spec_encodings(rng, 2)
    with_form = [e for e in encodings if c.decode(_one(e), 0x1_4000_0000) is not None]
    for _ in range(400):
        model, scans = sj.spec_layout(rng, with_form if rng.random() < 0.8 else encodings)
        sj.judge(c, model, scans, spec, examples)
    reaching = sj.process_area_layouts(c, with_form)
    assert len(reaching) >= 5, len(reaching)
    for model, scans in reaching:
        sj.judge(c, model, scans, spec, examples)
    return record, spec, examples


def _one(code: bytes) -> Any:
    import successor_scan_judge as sj
    model = sj.Model()
    model.place(0x1_4000_0000, code)
    return model


def test_the_successor_scan_equals_the_hosts(compiler: Compiler) -> None:
    """Where the C finds a form for every instruction the host's scan reads, its flags and pages equal the host's;
    elsewhere its flags hold all the host's (it stops with the undecided ones live). Never short."""
    record, spec, examples = _scan_judges(compiler)
    for counts in (record, spec):
        assert counts["short"] == 0 and counts["differ"] == 0, (counts, examples[:5])
    assert record["equal"] > 3000 and spec["equal"] > 3000, (record, spec)
    assert record["equal at fill bytes"] < 600, record      # 465 of the 4,000 samples (review of Sprint 082)


SCAN_PLANTS = {
    "follows no jump": ("        addr = ins.values[f->target];\n", "        addr += (u64)ins.length;\n"),
    # review of Sprint 082, finding 1: the thunk and PROCESS_VA checks, which no judge reached before
    "sees no thunk": ("    return lo < nthunks && thunks[lo] == address;\n", "    return 0;\n"),
    # Sprint 084 (review of 083): seen through successor_scan_judge.process_area_layouts, a jump past PROCESS_VA. The
    # VM's code-page set never holds a process page (_protect_code), so in the VM this check is the second defense.
    "scans the process area": ("in_thunks(m->thunks, m->nthunks, addr) || addr >= VMC_PROCESS_VA)",
                               "in_thunks(m->thunks, m->nthunks, addr))"),
}
SCAN_PLANT = SCAN_PLANTS["follows no jump"]


@pytest.mark.parametrize("plant", sorted(SCAN_PLANTS))
def test_a_planted_scan_fault_is_caught(tmp_path: Path, plant: str) -> None:
    """Each plant changes what a scan returns, and the judges see it: the spec layouts place thunks (as jump targets
    and where runs fall through) and runs that cross PROCESS_VA; the record has neither."""
    assert CLANG is not None
    old, new = SCAN_PLANTS[plant]
    text = (SOURCE / "vm_compiler.c").read_text()
    assert text.count(old) == 1
    (tmp_path / "vm_compiler.c").write_text(text.replace(old, new))
    (tmp_path / "harness.c").write_text(HARNESS)
    out = tmp_path / "planted.so"
    subprocess.run([CLANG, "-shared", "-fPIC", "-O2", "-std=c11", "-w", f"-I{tmp_path}", f"-I{SOURCE}", "-o", str(out),
                    str(tmp_path / "harness.c"), str(SOURCE / "x86_decode.c"), str(SOURCE / "a64_encoder.c")],
                   check=True, capture_output=True)
    record, spec, _ = _scan_judges(Compiler(out))
    assert spec["short"] + spec["differ"] > 0, (plant, record, spec)
    if plant == "follows no jump":
        assert record["short"] + record["differ"] > 0, record


def test_every_setter_writes_the_flags_its_jcc_tests() -> None:
    """The invariant behind KIT_DIARY #60 (review of Sprint 082, finding 4): a Jcc that branches on its setter's NZCV
    tests only flags the setter writes, so the scan has decided them before it reaches the Jcc and the NZCV kind
    changes no scan's result. Over every recorded setter in the samples and every condition of its kind."""
    jcc = {cc: vm_backend.insn_flags(lift(bytes([0x0F, 0x80 + cc, 0, 0, 0, 0]), 0x1_4000_0000))[0] for cc in range(16)}
    conditions = {cc: next(op[1] for op in lift(bytes([0x0F, 0x80 + cc, 0, 0, 0, 0]), 0x1_4000_0000).ops
                           if op[0] == "br") for cc in range(16)}
    checked = 0
    for code, address, _, _ in samples()[::4]:
        ins = lift(code, address)
        kind = vm_backend.nzcv_setter(ins)
        if kind is None:
            continue
        written = vm_backend.insn_flags(ins)[1]
        for cc, cond in conditions.items():
            if cond in vm_backend.NZCV_CONDITIONS[kind]:
                assert jcc[cc] & ~written == 0, (ins.iclass, kind, cond)
                checked += 1
    assert checked > 1000, checked
