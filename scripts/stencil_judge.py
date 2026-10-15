# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""stencil_judge.py — the compiler in C (tools/vm_compiler) judged against the Python compiler, as a record
(Sprint 080; review 2026-10-15 H3: the judges of Sprints 077-079 ran as scratch drivers and left no record).

Four judges, each a COMPARISON_COMPLETED of a session whose candidate is the generated table (its sha256):

- `instructions`: every distinct compile call of the stencil record (store/stencil/record.txt.gz): the C key and
  values equal to stencil_census.decoded_form's, the C words equal to compile_insn's at a base and exit drawn at
  random.
- `blocks`: a block from every recorded address (its recorded successors on, up to a branch or MAX_BLOCK_INSNS), the
  flags live at its end drawn from those its last instruction was recorded with: vmc_block's words and marks equal to
  assemble_block's over the instructions the C took.
- `cases`: the class case sets (Intel-checked, random operands, none from the record), at two flag contexts.
- `spec`: random encodings of every decoder spec row (tests/test_x86_decode_c.row_samples; the reviewer's E1), at
  unaligned addresses, under the record's commonest flag contexts.

The record and the table are the only sources of the table; the cases and the spec rows are neither. A judge passes
with 0 differing; an instruction with no stencil, or refused by a guard, is counted, not judged.

Usage: uv run python scripts/stencil_judge.py [--judges instructions,blocks,cases,spec] [--runs store/runs]
"""

from __future__ import annotations

import argparse
import ctypes
import hashlib
import multiprocessing
import random
import subprocess
import sys
import tempfile
import time
from collections import Counter, defaultdict
from collections.abc import Iterator
from pathlib import Path
from typing import Any

import msgspec

ROOT = Path(__file__).resolve().parent.parent
sys.path[:0] = [str(ROOT / "src"), str(ROOT / "scripts"), str(ROOT / "tests")]

import gen_stencil_table as g  # noqa: E402
import stencil_census as sc  # noqa: E402
import stencil_table as st  # noqa: E402
from w2m import events, vm_backend, x86dec  # noqa: E402
from w2m.lift import lift  # noqa: E402
from w2m.record import one_step_run  # noqa: E402

SOURCE = ROOT / "tools" / "vm_compiler"
NO_STENCIL, GUARDED = -2, -7
CONTRACT, VERSION = "w2m.vm_compiler.stencil_table", "0.80.0"
U64, U32, INT = ctypes.c_uint64, ctypes.c_uint32, ctypes.c_int


class Flags(ctypes.Structure):
    _fields_ = [("nzcv_from", INT), ("live_after", INT), ("nzcv_out", INT)]


class BlockIn(ctypes.Structure):
    _fields_ = [("start", U64), ("mem", ctypes.c_char_p), ("mem_base", U64), ("mem_size", U64),
                ("thunks", ctypes.POINTER(U64)), ("nthunks", INT), ("base", U64), ("exit_va", U64),
                ("slice_va", U64), ("symbols", ctypes.POINTER(U64)), ("live_out", INT)]


class Mark(ctypes.Structure):
    _fields_ = [("host", U64), ("guest", U64)]


class BlockOut(ctypes.Structure):
    _fields_ = [("words", ctypes.POINTER(U32)), ("max_words", INT), ("marks", ctypes.POINTER(Mark)),
                ("nwords", INT), ("nmarks", INT), ("count", INT), ("cut", INT), ("end", U64)]


def build(out: Path) -> Path:
    src = out.parent / "stencil_judge.c"
    src.write_text('#define VMC_NAMES\n#include "vm_compiler.c"\nconst int vmc_symbols = VMC_SYMBOLS;\n')
    subprocess.run(["clang", "-shared", "-fPIC", "-O2", "-std=c11", "-Wall", "-Werror", f"-I{SOURCE}", "-o", str(out),
                    str(src), str(SOURCE / "x86_decode.c"), str(SOURCE / "a64_encoder.c")], check=True)
    return out


_ST: dict[str, Any] = {}


def _init(lib_path: str, symbols: dict[str, int], implicit: dict[str, frozenset[int]],
          by_addr: dict[int, tuple[Any, bytes]], lives: dict[int, set[int]]) -> None:
    lib = ctypes.CDLL(lib_path)
    n = ctypes.c_int.in_dll(lib, "vmc_symbols").value
    names = [(s or b"").decode() for s in (ctypes.c_char_p * n).in_dll(lib, "vmc_symbol_name")]
    _ST.update(lib=lib, symbols=symbols, implicit=implicit, by_addr=by_addr, lives=lives,
               csyms=(U64 * max(n, 1))(*[symbols[x] for x in names]))


def _compile_c(code: bytes, address: int, base: int, exit_va: int, flags: Flags) -> list[int] | int:
    out = (U32 * 512)()
    n = _ST["lib"].vmc_compile(code, len(code), U64(address), U64(base), U64(exit_va), ctypes.byref(flags),
                               _ST["csyms"], out, 512)
    return n if n < 0 else list(out[:n])


def _compile_py(ins: Any, base: int, exit_va: int, nzcv_from: str | None, live: int, nzcv_out: bool) -> list[int]:
    a = vm_backend.Assembler(base)
    vm_backend.compile_insn(a, ins, exit_va, _ST["symbols"], nzcv_from, live, nzcv_out)
    return a.finish()


def _place(rng: random.Random) -> tuple[int, int]:
    base = rng.randrange(1 << 30, 1 << 40) * 4
    return base, base + rng.randrange(-(1 << 24), 1 << 24) * 4


def _judge_instructions(job: tuple[list[tuple[Any, bytes]], int]) -> tuple[Counter[str], list[str]]:
    calls, seed = job
    rng, c = random.Random(seed), Counter[str]()
    ex: list[str] = []
    key, vals = (ctypes.c_uint8 * 160)(), (U64 * 24)()
    nv, ln = ctypes.c_int(), ctypes.c_int()
    for call, code in calls:
        ins, _, nzcv_from, live, nzcv_out = call
        row = x86dec.decode(code).row or ""
        pk, pvals, _ = sc.decoded_form(call, code, _ST["implicit"].get(row, frozenset()))
        cn = sc.canonical_flags(ins, nzcv_from, live, nzcv_out)
        k = _ST["lib"].vmc_key(code, len(code), U64(ins.address), ctypes.byref(Flags(g.NZCV_FROM.index(cn[0]), cn[1],
                               int(cn[2]))), key, vals, ctypes.byref(nv), ctypes.byref(ln))
        if k < 0 or bytes(key[:k]) != g.key_bytes(pk) or list(vals[:nv.value]) != [v & (1 << 64) - 1 for v in pvals]:
            c["differ (key)"] += 1
            ex.append(f"key {code.hex()}")
            continue
        base, exit_va = _place(rng)
        got = _compile_c(code, ins.address, base, exit_va, Flags(g.NZCV_FROM.index(nzcv_from), live, int(nzcv_out)))
        _count(c, ex, got, lambda: _compile_py(ins, base, exit_va, nzcv_from, live, nzcv_out), ins.iclass, code)
    return c, ex


def _count(c: Counter[str], ex: list[str], got: list[int] | int, py: Any, iclass: str, code: bytes) -> None:
    if got == NO_STENCIL:
        c["no stencil"] += 1
    elif got == GUARDED:
        c["guarded"] += 1
    elif isinstance(got, int):
        c[f"C status {got}"] += 1
    elif got == py():
        c["equal"] += 1
    else:
        c["differ"] += 1
        ex.append(f"{iclass} {code.hex()}")


def _walk(start: int) -> tuple[list[Any], bytes, bool]:
    by, addr, ended = _ST["by_addr"], start, False
    insns: list[Any] = []
    codes: list[bytes] = []
    while len(insns) < vm_backend.MAX_BLOCK_INSNS and addr in by:
        ins, code = by[addr]
        insns.append(ins)
        codes.append(code)
        addr += ins.length
        if any(op[0] == "br" for op in ins.ops):
            ended = True
            break
    else:
        ended = len(insns) == vm_backend.MAX_BLOCK_INSNS
    return insns, b"".join(codes), ended


def _judge_blocks(job: tuple[list[int], int]) -> tuple[Counter[str], list[str]]:
    starts, seed = job
    rng, c = random.Random(seed), Counter[str]()
    ex: list[str] = []
    words, marks = (U32 * 8192)(), (Mark * 80)()
    for start in starts:
        insns, mem, ended = _walk(start)
        live_out = rng.choice(sorted(_ST["lives"][insns[-1].address])) if ended else vm_backend.ALL_FLAGS
        base, exit_va = _place(rng)
        slice_va = base + rng.randrange(-(1 << 24), 1 << 24) * 4
        bi = BlockIn(start, mem, start, len(mem), None, 0, base, exit_va, slice_va, _ST["csyms"], live_out)
        bo = BlockOut(words, 8192, marks, 0, 0, 0, 0, 0)
        n = _ST["lib"].vmc_block(ctypes.byref(bi), ctypes.byref(bo))
        if n < 0:
            c[f"to the host ({n})"] += 1
            continue
        k = bo.count
        if k < len(insns):
            c[f"cut short ({bo.cut})"] += 1
        a, py_marks, _, _, _ = vm_backend.assemble_block(insns[:k], base, exit_va, slice_va, _ST["symbols"],
                                                         live_out if k == len(insns) else vm_backend.ALL_FLAGS)
        if list(words[:n]) == a.finish() and [(m.host, m.guest) for m in marks[:bo.nmarks]] == py_marks:
            c["equal"] += 1
        else:
            c["differ"] += 1
            ex.append(f"block {start:x}")
    return c, ex


def _judge_codes(job: tuple[list[bytes], int, list[tuple[int, int, int]], bool]) -> tuple[Counter[str], list[str]]:
    """Encodings from outside the table's sources, each at `contexts` (and two addresses where `aligned` is false)."""
    codes, seed, contexts, unaligned = job
    rng, c = random.Random(seed), Counter[str]()
    ex: list[str] = []
    for raw in codes:
        try:
            code = raw[:x86dec.decode(raw).length]
        except Exception:
            c["not decoded"] += 1
            continue
        addresses = ([0x1_4000_0000 + rng.randrange(1 << 24), 0x7FF7_0000_0000 + rng.randrange(1 << 20) * 16]
                     if unaligned else [0x1_4000_0000 + rng.randrange(1 << 20) * 16])
        for address in addresses:
            try:
                ins = lift(code, address)
            except Exception:
                c["not lifted"] += 1
                break
            for nf, live, no in contexts:
                base, exit_va = _place(rng)
                got = _compile_c(code, address, base, exit_va, Flags(nf, live, no))
                _count(c, ex, got, lambda: _compile_py(ins, base, exit_va, g.NZCV_FROM[nf], live, bool(no)),
                       ins.iclass, code)
    return c, ex


def _pool(fn: Any, jobs: list[Any], init: tuple[Any, ...], workers: int) -> tuple[Counter[str], list[str]]:
    total, examples = Counter[str](), []
    with multiprocessing.get_context("fork").Pool(workers, _init, init) as pool:
        for c, ex in pool.imap_unordered(fn, jobs):
            total.update(c)
            examples += ex
    return total, examples


def judges(names: list[str], workers: int) -> Iterator[tuple[str, Counter[str], list[str], str]]:
    """(judge, counts, examples of differences, its data source) for each judge named."""
    record = sc.read_record(g.RECORD)
    table = st.read_table(g.TABLE)
    with tempfile.TemporaryDirectory() as tmp:
        lib = str(build(Path(tmp) / "stencil_judge.so"))
        by_addr: dict[int, tuple[Any, bytes]] = {}
        for ins, codes in record["bytes"].items():
            for code in codes:
                by_addr.setdefault(ins.address, (ins, code))
        lives: dict[int, set[int]] = defaultdict(set)
        for call in record["calls"]:
            lives[call[0].address].add(call[3])
        init = (lib, record["symbols"], table["implicit"], by_addr, dict(lives))
        if "instructions" in names:
            calls = [(call, next(iter(record["bytes"][call[0]]))) for call in record["calls"]]
            jobs: list[Any] = [(calls[i:i + 2000], 77_000 + i) for i in range(0, len(calls), 2000)]
            yield ("instructions", *_pool(_judge_instructions, jobs, init, workers), str(g.RECORD))
        if "blocks" in names:
            starts = sorted(by_addr)
            jobs = [(starts[i:i + 1000], 78_000 + i) for i in range(0, len(starts), 1000)]
            yield ("blocks", *_pool(_judge_blocks, jobs, init, workers), str(g.RECORD))
        if "cases" in names:
            import class_run
            import semantics_differential
            codes = [case.code for k in semantics_differential.CASE_SETS if k in class_run.REFERENCES
                     for case in class_run._built(k)]
            contexts = [(0, vm_backend.ALL_FLAGS, 0), (0, 0, 0)]
            jobs = [(codes[i:i + 5000], 79_000 + i, contexts, False) for i in range(0, len(codes), 5000)]
            yield ("cases", *_pool(_judge_codes, jobs, init, workers), "the class case sets (semantics_differential)")
        if "spec" in names:
            import test_x86_decode_c
            codes = test_x86_decode_c.row_samples(random.Random(424242), 40)
            common = Counter((g.NZCV_FROM.index(k[9]), k[10], int(k[11])) for k in table["table"])
            contexts = list(dict.fromkeys([c for c, _ in common.most_common(12)]
                                          + [(0, vm_backend.ALL_FLAGS, 0), (0, 0, 0)]))
            jobs = [(codes[i:i + 2000], 80_000 + i, contexts, True) for i in range(0, len(codes), 2000)]
            yield ("spec", *_pool(_judge_codes, jobs, init, workers), "decoder spec rows (test_x86_decode_c.row_samples)")


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--judges", default="instructions,blocks,cases,spec")
    ap.add_argument("--runs", type=Path, default=ROOT / "store" / "runs")
    ap.add_argument("--workers", type=int, default=max(1, (multiprocessing.cpu_count() or 2) - 1))
    a = ap.parse_args(argv)
    header = g.OUT.read_bytes()
    candidate = "sha256:" + hashlib.sha256(header).hexdigest()
    cand_id = f"stencil-table-{candidate[7:19]}"
    results: list[tuple[str, Counter[str], list[str], str]] = []

    def work() -> Iterator[msgspec.Struct]:
        yield events.CANDIDATE_GENERATED(candidate_id=cand_id, contract_id=CONTRACT, contract_version=VERSION,
                                         generator_ref="scripts/gen_stencil_table.py: tools/vm_compiler/stencil_table.h",
                                         attempt=1, candidate_sha256=candidate)
        for name, counts, examples, source in judges(a.judges.split(","), a.workers):
            results.append((name, counts, examples, source))
            print(f"{name}: {dict(sorted(counts.items()))}" + (f"; e.g. {examples[:5]}" if examples else ""))
            differ = sum(n for k, n in counts.items() if k.startswith("differ"))
            yield events.COMPARISON_COMPLETED(
                comparison_id=f"cmp-{cand_id}-{name}", contract_id=CONTRACT, contract_version=VERSION,
                relation="exact", reference_ref=f"vm_backend.compile_insn / assemble_block over {source}",
                target_execution_id=f"vm_compiler-{name}", verdict="discrepancy" if differ else "conforms",
                rule_ref="; ".join(f"{k} {n}" for k, n in sorted(counts.items()))[:500],
                first_divergence=examples[0][:500] if examples else None)
        if all(not any(k.startswith("differ") for k in c) for _, c, _, _ in results):
            yield events.CANDIDATE_ACCEPTED(candidate_id=cand_id, contract_id=CONTRACT, contract_version=VERSION,
                                            comparison_ids=tuple(f"cmp-{cand_id}-{n}" for n, _, _, _ in results))

    t0 = time.monotonic()
    root, kinds = one_step_run(a.runs / f"stencil-judge-{time.time_ns()}", profile="host clang build of tools/vm_compiler",
                               contract_version=VERSION, purpose="conformance", work=work,
                               baseline={"candidate_sha256": candidate, "judges": a.judges},
                               outcome=lambda evs: "completed" if any(type(e).__name__ == "CANDIDATE_ACCEPTED"
                                                                      for e in evs) else "completed_with_discrepancies")
    print(f"record {root} in {time.monotonic() - t0:.0f} s: {dict(kinds)}")
    return 0 if kinds.get("CANDIDATE_ACCEPTED") else 1


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
