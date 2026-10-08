"""Sprint 014 grading surface: Capstone reference traversal of the runner-built probes, and an
independent cross-check against LLVM's disassembler (llvm-objdump) at every reachable instruction."""

from __future__ import annotations

import shutil
import subprocess
from pathlib import Path

import pytest

from w2m.pe import parse
from w2m.x86ref import text_coverage, walk

BINARIES = sorted((Path(__file__).resolve().parent.parent / "store" / "binaries").glob("*.exe"))
OBJDUMP = shutil.which("llvm-objdump") or (
    subprocess.run(["xcrun", "--find", "llvm-objdump"], capture_output=True, text=True).stdout.strip()
    if shutil.which("xcrun") else "")
REP = {"rep", "repe", "repne", "repz", "repnz"}


def llvm_insns(path: Path, image_base: int) -> dict[int, tuple[str, int]]:
    """rva -> (mnemonic, length). llvm-objdump prints `lock` on its own line (folded here) and
    `rep` with its string instruction as the next token on the same line."""
    out = subprocess.run([OBJDUMP, "-d", "--no-show-raw-insn", "--x86-asm-syntax=intel", str(path)],
                         capture_output=True, text=True, check=True).stdout
    rows: list[tuple[int, str]] = []
    for line in out.splitlines():
        head, sep, rest = line.partition(":")
        if not sep or not all(c in "0123456789abcdef" for c in head) or not head:
            continue
        toks = rest.split()
        if not toks:
            continue
        mn = " ".join(toks[:2]) if toks[0] in REP and len(toks) > 1 else toks[0]
        addr = int(head, 16) - image_base
        if rows and rows[-1][1] == "lock":
            rows[-1] = (rows[-1][0], f"lock {mn}")
        else:
            rows.append((addr, mn))
    return {a: (mn, b - a) for (a, mn), (b, _) in zip(rows, rows[1:])}


@pytest.mark.parametrize("path", BINARIES, ids=lambda p: p.name[:12])
def test_reachable_code(path: Path) -> None:
    data = path.read_bytes()
    img = parse(data)
    r = walk(img, data)
    assert not r.undecodable
    assert len(r.insns) > 20_000
    assert r.roots["pdata"] == len(img.function_starts) > 400
    # every import is referenced; some are loaded into a register, not called through the slot
    assert set(r.imports_referenced) == set(img.iat.values())
    assert set(r.imports_called) <= set(r.imports_referenced)
    assert text_coverage(img, r) > 0.9


@pytest.mark.skipif(not OBJDUMP, reason="llvm-objdump not available")
@pytest.mark.parametrize("path", BINARIES, ids=lambda p: p.name[:12])
def test_capstone_agrees_with_llvm(path: Path) -> None:
    data = path.read_bytes()
    img = parse(data)
    r = walk(img, data)
    llvm = llvm_insns(path, img.image_base)
    missing = [hex(i.rva) for i in r.insns.values() if i.rva not in llvm]
    assert not missing, missing[:10]
    length = [(hex(i.rva), i.size, llvm[i.rva]) for i in r.insns.values() if llvm[i.rva][1] != i.size]
    assert not length, length[:10]
    spelling = [(hex(i.rva), i.mnemonic, llvm[i.rva][0]) for i in r.insns.values()
                if llvm[i.rva][0] != i.mnemonic]
    assert not spelling, spelling[:10]
