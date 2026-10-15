# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""Sprint 075: the C decoder (tools/vm_compiler/x86_decode.c) against x86dec.decode and x86ops.operands.

For each byte string both give the same outcome: the same exception kind (InvalidEncoding, Truncated, DecodeError,
another), or the same length, iclass, spec row, every Fields member and every operand. The strings: samples built
for every spec row (its map, opcode, mandatory prefix, encoding, ModRM constraints, W and L, with random REX, ModRM,
SIB, displacement and immediate bytes after), random byte strings, and every truncation of a share of both.
"""

from __future__ import annotations

import ctypes
import random
import shutil
import subprocess
from pathlib import Path
from typing import Any

import pytest

from w2m import guest_runtime, x86dec, x86ops
from w2m.x86dec_tables import TABLE

ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / "tools" / "vm_compiler" / "x86_decode.c"
CLANG = shutil.which("clang")
pytestmark = pytest.mark.skipif(CLANG is None, reason="clang not available")

OK, INVALID, TRUNCATED, DECODE_ERROR, OTHER = range(5)
ENCODINGS = ["legacy", "vex", "evex", "xop"]


class Fields(ctypes.Structure):
    _fields_ = [("prefixes", ctypes.c_uint8 * 15), ("nprefixes", ctypes.c_uint8), ("rex", ctypes.c_uint8),
                ("enc", ctypes.c_uint8), ("opcode", ctypes.c_uint8), ("modrm", ctypes.c_int16),
                ("sib", ctypes.c_int16), ("disp", ctypes.c_int64), ("disp_size", ctypes.c_uint8),
                ("imm", ctypes.c_uint8 * 8), ("imm_size", ctypes.c_uint8), ("opsize", ctypes.c_uint8),
                ("asize", ctypes.c_uint8), ("ops", ctypes.c_uint16), ("vvvv", ctypes.c_int8), ("vl", ctypes.c_uint8)]


class Decoded(ctypes.Structure):
    _fields_ = [("length", ctypes.c_uint8), ("iclass", ctypes.c_uint16), ("row", ctypes.c_uint16), ("f", Fields)]


class Operand(ctypes.Structure):
    _fields_ = [("kind", ctypes.c_uint8), ("reg_class", ctypes.c_uint8), ("reg", ctypes.c_uint8),
                ("seg", ctypes.c_uint8), ("base_class", ctypes.c_uint8), ("base", ctypes.c_uint8),
                ("index_class", ctypes.c_uint8), ("index", ctypes.c_uint8), ("scale", ctypes.c_uint8),
                ("bytes", ctypes.c_uint8), ("imm_len", ctypes.c_uint8), ("disp", ctypes.c_int64),
                ("imm", ctypes.c_uint64)]


REGISTER_NAMES = {1: x86ops.GPR64, 2: x86ops.GPR32, 3: x86ops.GPR16, 4: x86ops.GPR8_REX, 5: x86ops.GPR8_LEGACY,
                  6: x86ops.XMM, 7: x86ops.YMM}


class CDecoder:
    def __init__(self, path: Path) -> None:
        self.lib = ctypes.CDLL(str(path))
        self.lib.x86_decode.argtypes = [ctypes.c_char_p, ctypes.c_int, ctypes.POINTER(Decoded)]
        self.lib.x86_operands.argtypes = [ctypes.POINTER(Decoded), ctypes.POINTER(Operand * 4)]
        n_ic = ctypes.c_int.in_dll(self.lib, "x86_iclasses").value
        n_rows = ctypes.c_int.in_dll(self.lib, "x86_rows").value
        n_t = ctypes.c_int.in_dll(self.lib, "x86_templates").value
        self.iclass = self.names("x86_iclass_name", n_ic)
        self.row = self.names("x86_row_name", n_rows)
        self.template = self.names("x86_template_name", n_t)

    def names(self, symbol: str, n: int) -> list[str]:
        return [(s or b"").decode() for s in (ctypes.c_char_p * n).in_dll(self.lib, symbol)]

    def decode(self, b: bytes) -> Any:
        d = Decoded()
        status = self.lib.x86_decode(b, len(b), ctypes.byref(d))
        if status:
            return status
        f = d.f
        fields = (tuple(f.prefixes[:f.nprefixes]), f.rex, ENCODINGS[f.enc], f.opcode,
                  None if f.modrm < 0 else f.modrm, None if f.sib < 0 else f.sib, f.disp, f.disp_size,
                  bytes(f.imm[:f.imm_size]), f.opsize, f.asize, None if f.ops == 0xFFFF else self.template[f.ops],
                  None if f.vvvv < 0 else f.vvvv, f.vl)
        ops = (Operand * 4)()
        n = self.lib.x86_operands(ctypes.byref(d), ctypes.byref(ops))
        return (d.length, self.iclass[d.iclass], self.row[d.row], fields,
                n if n < 0 else tuple(self.operand(o) for o in ops[:n]))

    @staticmethod
    def operand(o: Operand) -> str:
        def base(cls: int, n: int) -> str:
            return {0: "INVALID", 8: "RIP", 9: "EIP"}.get(cls) or str(REGISTER_NAMES[cls][n])
        kind = o.kind
        if kind == 0:
            return "R:" + str(REGISTER_NAMES[o.reg_class][o.reg])
        if kind == 1:
            seg = {0x64: "FS", 0x65: "GS"}.get(o.seg, "INVALID")
            return f"M:{seg}:{base(o.base_class, o.base)}:{base(o.index_class, o.index)}:{o.scale}:{o.disp}:{o.bytes}"
        if kind == 2:
            return f"A:{base(o.base_class, o.base)}:{base(o.index_class, o.index)}:{o.scale}:{o.disp}"
        if kind == 3:
            return f"I:{o.imm_len}:{o.imm:x}"
        if kind == 4:
            return f"J:{o.disp}"
        return f"{'MM' if kind == 5 else 'ST'}:{o.reg}"


def python_decode(b: bytes) -> Any:
    try:
        d = x86dec.decode(b)
    except x86dec.InvalidEncoding:
        return INVALID
    except x86dec.Truncated:
        return TRUNCATED
    except x86dec.DecodeError:
        return DECODE_ERROR
    except Exception:
        return OTHER
    f = d.fields
    assert f is not None
    fields = (f.prefixes, f.rex, f.enc, f.opcode, f.modrm, f.sib, f.disp, f.disp_size, f.imm, f.opsize, f.asize,
              f.ops, f.vvvv, f.vl)
    try:
        ops: Any = x86ops.operands(f)
    except x86ops.OperandError:
        ops = -1
    except (AssertionError, KeyError, IndexError):
        ops = -2
    return d.length, d.iclass, d.row, fields, ops


def build(tmp: Path, flags: tuple[str, ...]) -> Path:
    assert CLANG is not None
    out = tmp / "x86_decode.so"
    names = tmp / "names.c"                             # the counts as data, for ctypes
    names.write_text('#define X86_DECODE_NAMES\n#include "x86_decode.c"\nconst int x86_iclasses = X86_ICLASSES, '
                     'x86_rows = X86_ROWS, x86_templates = X86_TEMPLATES;\n')
    subprocess.run([CLANG, "-shared", "-fPIC", "-O2", "-std=c11", "-Wall", "-Werror", *flags,
                    f"-I{SOURCE.parent}", "-o", str(out), str(names)], check=True, capture_output=True)
    return out


@pytest.fixture(scope="module")
def decoder(tmp_path_factory: pytest.TempPathFactory) -> CDecoder:
    return CDecoder(build(tmp_path_factory.mktemp("x86c"), ()))


MAP_BYTES = {"1": b"", "0f": b"\x0f", "0f38": b"\x0f\x38", "0f3a": b"\x0f\x3a"}
PP = {"none": 0, "66": 1, "f3": 2, "f2": 3}


def row_samples(rng: random.Random, per_row: int) -> list[bytes]:
    """Byte strings aimed at each row: its encoding, map, opcode and mandatory prefix, the ModRM its reg, exact byte
    and form allow, its W and L, then random bytes."""
    out: list[bytes] = []
    for (mp, op, enc), rows in TABLE.items():
        for r in rows:
            pfx, reg, has_modrm, _imm, _mn, _s, mod, w, ll, form = r[:10]
            for _ in range(per_row):
                m = rng.randrange(256)
                if reg is not None:
                    m = m & ~0x38 | reg << 3
                if form == "reg":
                    m |= 0xC0
                elif form == "mem" and m >> 6 == 3:
                    m &= 0x3F
                if mod is not None:
                    m = int(mod, 16)
                tail = bytes([m] if has_modrm else []) + rng.randbytes(12)
                wbit = rng.randrange(2) if w is None else w
                lbit = rng.randrange(2) if ll is None else ll
                pp = PP[pfx]
                if enc == "legacy":
                    lead = {"none": b"", "66": b"\x66", "f2": b"\xf2", "f3": b"\xf3"}[pfx]
                    extra = bytes(rng.choice([0x66, 0x67, 0xF0, 0x64, 0x65, 0x2E]) for _ in range(rng.randrange(2)))
                    rex = bytes([0x40 | wbit << 3 | rng.randrange(8)]) if rng.random() < 0.6 or w else b""
                    out.append(extra + lead + rex + MAP_BYTES[mp] + bytes([op]) + tail)
                elif enc == "vex":
                    mmmmm = {"0f": 1, "0f38": 2, "0f3a": 3}[mp]
                    vvvv = 0xF if rng.random() < 0.7 else rng.randrange(16)
                    if mmmmm == 1 and not wbit and rng.random() < 0.5:
                        out.append(bytes([0xC5, rng.randrange(2) << 7 | vvvv << 3 | lbit << 2 | pp, op]) + tail)
                    else:
                        out.append(bytes([0xC4, rng.randrange(8) << 5 | mmmmm,
                                          wbit << 7 | vvvv << 3 | lbit << 2 | pp, op]) + tail)
                elif enc == "evex":
                    mm = {"0f": 1, "0f38": 2, "0f3a": 3}[mp]
                    out.append(bytes([0x62, rng.randrange(16) << 4 | mm, wbit << 7 | rng.randrange(16) << 3 | 4 | pp,
                                      rng.randrange(256), op]) + tail)
                else:
                    mmmmm = {"x8": 8, "x9": 9, "xa": 10}[mp]
                    out.append(bytes([0x8F, rng.randrange(8) << 5 | mmmmm,
                                      wbit << 7 | rng.randrange(16) << 3 | lbit << 2 | pp, op]) + tail)
    return out


def corpus() -> list[bytes]:
    rng = random.Random(7501)
    samples = row_samples(rng, 12)
    samples += [rng.randbytes(rng.randrange(1, 16)) for _ in range(20000)]
    samples += [bytes(rng.choice([0x66, 0x67, 0xF2, 0xF3, 0xF0, 0x48, 0x41, 0x0F, 0xC4, 0xC5, 0x62, 0x8F]) for _ in
                      range(rng.randrange(0, 4))) + rng.randbytes(8) for _ in range(20000)]
    cut = [s[:k] for s in rng.sample(samples, 3000) for k in range(len(s))]
    return samples + cut


def compare(decoder: CDecoder, strings: list[bytes]) -> tuple[dict[str, int], list[Any]]:
    outcomes: dict[str, int] = {}
    bad: list[Any] = []
    for b in strings:
        want, got = python_decode(b), decoder.decode(b)
        name = "decoded" if isinstance(want, tuple) else ["ok", "invalid", "truncated", "decode error", "other"][want]
        outcomes[name] = outcomes.get(name, 0) + 1
        if want != got:
            bad.append((b.hex(), want, got))
    return outcomes, bad


def test_builds_with_the_guest_runtime_flags(tmp_path: Path) -> None:
    assert CLANG is not None
    r = subprocess.run([CLANG, *guest_runtime.CFLAGS, "-c", "-o", str(tmp_path / "x86_decode.o"), str(SOURCE)],
                       capture_output=True, text=True)
    assert r.returncode == 0, r.stderr


def test_c_decoder_equals_python(decoder: CDecoder) -> None:
    outcomes, bad = compare(decoder, corpus())
    assert not bad, (len(bad), bad[:5])
    assert outcomes.get("decoded", 0) > 10000 and all(outcomes.get(k, 0) > 100 for k in
                                                      ("invalid", "truncated", "decode error")), outcomes


def test_every_row_decodes_in_c(decoder: CDecoder) -> None:
    """Each of the spec's rows is the row of at least one sample."""
    rows = {got[2] for got in map(decoder.decode, row_samples(random.Random(7502), 30)) if isinstance(got, tuple)}
    every = {r[18] for entries in TABLE.values() for r in entries}
    assert every <= rows, sorted(every - rows)[:10]
