# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""The translation cache (recentering step 8): a block kept with the bytes it read is placed again only when those bytes
hash the same, its fixed-target branches re-encoded for its new place."""

from __future__ import annotations

import struct
from collections.abc import Callable
from pathlib import Path

import pytest

from w2m import a64_encoder as a64
from w2m.vm_backend import rebranch

RESULTS, CODE, STACK = 0x3001_0000, 0x2000_0000, 0x4000_0000


def test_rebranch_moves_every_branch_kind_it_takes() -> None:
    makers: list[Callable[[int], int]] = [a64.b, a64.bl, lambda off: a64.bcond("ne", off), lambda off: a64.cbz(3, off),
              lambda off: a64.cbnz(17, off)]
    for make in makers:
        for old, new in ((0x40, -0x1000), (-8, 0x7FFFC), (0x1234, 4)):
            assert rebranch(make(old), new) == make(new)
    assert rebranch(a64.movz(1, 5), 8) is None                # not a branch: such a block is not cached


def program(add: int) -> bytes:
    """Ten blocks: rax += `add` and a stored running total in each, a JMP from each to the next, then the end."""
    out = bytearray(b"\x48\x31\xc0")                            # xor rax, rax
    for k in range(10):
        out += b"\x48\x83\xc0" + bytes([add])                   # add rax, imm8
        out += b"\x48\x89\x04\x25" + struct.pack("<I", RESULTS + 8 * k)
        out += b"\xeb\x00"                                      # jmp to the next instruction: a block per step
    return bytes(out + b"\xc6\x04\x25\x00\x00\x00\x00\x00")


def run(code: bytes, monkeypatch: pytest.MonkeyPatch, cache: Path) -> tuple[list[int], int, int]:
    from w2m import vm_backend
    from w2m.loader import Loaded, Memory
    monkeypatch.setattr(vm_backend, "TRANSLATION_CACHE_DIR", cache)
    mem = Memory()
    mem.map(CODE, 0x1000, "code").data[:len(code)] = code
    mem.map(RESULTS, 0x1000, "results")
    mem.map(STACK, 0x10000, "stack")
    vm = vm_backend.VM(Loaded(None, mem, CODE, {}, STACK + 0x10000))   # type: ignore[arg-type]
    try:
        r = vm.run(max_steps=1 << 20, timeout_ms=10_000)
        got = [int.from_bytes(vm.mem.read(RESULTS + 8 * k, 8), "little") for k in range(10)]
        hits, compiled = vm.tcache_hits, len(vm.translations) - vm.tcache_hits
    finally:
        vm.close()
    assert r.stop.startswith("UnhandledException 0xc0000005"), r.stop
    return got, hits, compiled


@pytest.mark.hvf
def test_a_kept_block_runs_again_only_on_the_same_bytes(monkeypatch: pytest.MonkeyPatch, tmp_path: Path) -> None:
    first, hits, compiled = run(program(3), monkeypatch, tmp_path)
    assert first == [3 * (k + 1) for k in range(10)] and hits == 0 and compiled > 0
    again, hits, compiled = run(program(3), monkeypatch, tmp_path)
    assert again == first and compiled == 0 and hits > 0    # every block from the kept cache, the same results
    changed = bytearray(program(3))
    changed[3 + 3] = 5                                        # the first block's immediate: 3 -> 5
    other, hits, compiled = run(bytes(changed), monkeypatch, tmp_path)
    assert other == [5 + 3 * k for k in range(10)]            # the changed block compiled afresh, the rest kept
    assert compiled >= 1 and hits >= 1


B_PAGE, PHASE = CODE + 0x1000, RESULTS + 0x100


def rewriting_program() -> tuple[bytes, int]:
    """Block A (`cmp rax, rbx; jmp B`) whose flags are dead while B, on the next page, starts `add rcx, 1`; the program
    then rewrites B to `setb dl; nop` (a CF reader), jumps to A again, and stores RDX. A must leave its block when B's
    page is written (its liveness scan read B): RDX is 1 (1 < 2 sets CF). Also returns a second entry, a block on B's
    page past B that jumps to the first, so a run entered there places every kept block elsewhere."""
    def jmp(at: int, to: int) -> bytes:
        return b"\xe9" + struct.pack("<i", to - (at + 5))
    p1 = bytearray(0x1000)
    entry = b"\x48\xc7\xc0\x01\x00\x00\x00\x48\xc7\xc3\x02\x00\x00\x00\x48\x31\xd2"   # mov rax, 1; mov rbx, 2; xor rdx, rdx
    a_at = len(entry) + 5
    p1[:len(entry)] = entry
    pre = B_PAGE + 0x40                     # a block on B's page first, so A's scan reads B (a page behind code)
    p1[len(entry):a_at] = jmp(CODE + len(entry), pre)
    a_code = b"\x48\x39\xd8"                                                       # cmp rax, rbx
    p1[a_at:a_at + 3] = a_code
    p1[a_at + 3:a_at + 8] = jmp(CODE + a_at + 3, B_PAGE)
    back = 0x100
    tail = bytearray(b"\x48\x83\x3c\x25" + struct.pack("<I", PHASE) + b"\x00")       # cmp qword [PHASE], 0
    rewrite = (b"\x48\xc7\x04\x25" + struct.pack("<I", PHASE) + b"\x01\x00\x00\x00"   # mov qword [PHASE], 1
               + b"\xc7\x04\x25" + struct.pack("<I", B_PAGE) + bytes.fromhex("0f92c290"))   # B: setb dl; nop
    done = b"\x48\x89\x14\x25" + struct.pack("<I", RESULTS) + b"\xc6\x04\x25\x00\x00\x00\x00\x00"
    tail += b"\x75" + bytes([len(rewrite) + 5])                                      # jne done
    tail += rewrite
    tail += jmp(CODE + back + len(tail), CODE + a_at)
    tail += done
    p1[back:back + len(tail)] = tail
    p2 = bytearray(0x1000)
    p2[:4] = bytes.fromhex("4883c101")                                             # B: add rcx, 1
    p2[4:9] = jmp(B_PAGE + 4, CODE + back)
    p2[0x40:0x45] = jmp(pre, CODE + a_at)                                           # then A, in a block of its own
    second = 0x800                                                                 # the second entry: a jump in
    p2[second:second + 5] = jmp(B_PAGE + second, CODE)
    return bytes(p1 + p2), B_PAGE + second


def run_rewriting(monkeypatch: pytest.MonkeyPatch, cache: Path, entry: int, phase: int) -> tuple[int, int]:
    from w2m import vm_backend
    from w2m.loader import Loaded, Memory
    monkeypatch.setattr(vm_backend, "TRANSLATION_CACHE_DIR", cache)
    code, _ = rewriting_program()
    mem = Memory()
    mem.map(CODE, 0x2000, "code").data[:] = code
    mem.map(RESULTS, 0x1000, "results").data[PHASE - RESULTS] = phase   # 1: no rewrite, RDX stays 0
    mem.map(STACK, 0x10000, "stack")
    vm = vm_backend.VM(Loaded(None, mem, entry, {}, STACK + 0x10000))   # type: ignore[arg-type]
    try:
        r = vm.run(max_steps=1 << 20, timeout_ms=10_000)
        rdx, hits = int.from_bytes(vm.mem.read(RESULTS, 8), "little"), vm.tcache_hits
    finally:
        vm.close()
    assert r.stop.startswith("UnhandledException 0xc0000005"), r.stop
    return rdx, hits


@pytest.mark.hvf
def test_kept_blocks_move_and_leave_with_the_pages_their_scan_read(monkeypatch: pytest.MonkeyPatch,
                                                                   tmp_path: Path) -> None:
    _, second = rewriting_program()
    assert run_rewriting(monkeypatch, tmp_path, CODE, 1) == (0, 0)   # kept: A with its flags dead, B unrewritten
    # a new first block, so the kept ones land elsewhere; then B rewritten, and A must leave with B's page
    rdx, hits = run_rewriting(monkeypatch, tmp_path, second, 0)
    assert hits > 0 and rdx == 1
