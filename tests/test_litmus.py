# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""The five litmus tests as x86-64 code compiled by w2m, each thread on its own vCPU (Sprint 105; docs/design-several-
vcpus.md). x86-TSO forbids MP's r1=1 r2=0, LB's r1=r2=1, 2+2W's x=1 y=1 at the end and IRIW's readers disagreeing on
the order of two independent stores; it allows SB's r1=r2=0, which the store buffer makes (Owens, Sarkar and Sewell,
"A Better x86 Memory Model: x86-TSO", TPHOLs 2009). Iteration i uses the i-th word of each array, so no reset races
a late thread; the threads meet at an arrival counter before each iteration. Each thread ends by jumping to DONE,
which no block holds, so its vCPU exits to the harness with x17 = DONE. A thread runs once alone first (one arrival
needed), so every block is compiled before the threads run at once."""

from __future__ import annotations

import struct
import subprocess
import tempfile
import threading
from pathlib import Path
from typing import Any

import pytest

CODE, STACK, DATA = 0x2000_0000, 0x4000_0000, 0x3000_0000
DONE = 0x2FFF_0000
N = 100_000
A = 8 * N                                        # one array: a word an iteration
X, Y, ARRIVE, R = 0, A, 2 * A, 3 * A             # x, y, the arrival counters, then the result arrays R0..R3
DATA_SIZE = (R + 4 * A + 0xFFF) & ~0xFFF

# each thread: rdi the data, rcx the iterations, r9 the threads to wait for; rbx the iteration's byte offset
HEAD = """.intel_syntax noprefix
    xor ebx, ebx
1:  lock inc qword ptr [rdi + rbx + {ARRIVE}]
2:  cmp qword ptr [rdi + rbx + {ARRIVE}], r9
    jae 3f
    pause
    jmp 2b
3:
"""
TAIL = """    add rbx, 8
    dec rcx
    jnz 1b
    mov rax, {DONE}
    jmp rax
"""
STORE = "    mov qword ptr [rdi + rbx + {at}], {v}\n"
LOAD = "    mov rax, qword ptr [rdi + rbx + {at}]\n    mov qword ptr [rdi + rbx + {r}], rax\n"

TESTS: dict[str, list[str]] = {
    "MP": [STORE.format(at=X, v=1) + STORE.format(at=Y, v=1),
           LOAD.format(at=Y, r=R) + LOAD.format(at=X, r=R + A)],
    "SB": [STORE.format(at=X, v=1) + LOAD.format(at=Y, r=R),
           STORE.format(at=Y, v=1) + LOAD.format(at=X, r=R + A)],
    "LB": [LOAD.format(at=X, r=R) + STORE.format(at=Y, v=1),
           LOAD.format(at=Y, r=R + A) + STORE.format(at=X, v=1)],
    "2+2W": [STORE.format(at=X, v=1) + STORE.format(at=Y, v=2),
             STORE.format(at=Y, v=1) + STORE.format(at=X, v=2)],
    "IRIW": [STORE.format(at=X, v=1), STORE.format(at=Y, v=1),
             LOAD.format(at=X, r=R) + LOAD.format(at=Y, r=R + A),
             LOAD.format(at=Y, r=R + 2 * A) + LOAD.format(at=X, r=R + 3 * A)],
}


def forbidden(test: str, w: list[list[int]]) -> int:
    """Iterations with the outcome x86-TSO forbids (SB: the relaxed outcome it allows, counted instead)."""
    x, y, r = w[0], w[1], w[2:]
    if test == "MP":
        return sum(1 for a, b in zip(r[0], r[1]) if a == 1 and b == 0)
    if test == "SB":
        return sum(1 for a, b in zip(r[0], r[1]) if a == 0 and b == 0)
    if test == "LB":
        return sum(1 for a, b in zip(r[0], r[1]) if a == 1 and b == 1)
    if test == "2+2W":
        return sum(1 for a, b in zip(x, y) if a == 1 and b == 1)
    return sum(1 for a, b, c, d in zip(*r) if a == 1 and b == 0 and c == 1 and d == 0)


def assemble(text: str) -> bytes:
    """x86-64 bytes for `text`, by clang's assembler (the bytes w2m then compiles as any guest code)."""
    with tempfile.TemporaryDirectory() as d:
        (Path(d) / "t.s").write_text(text)
        subprocess.run(["clang", "-c", "-target", "x86_64-apple-macos", "t.s", "-o", "t.o"], cwd=d, check=True)
        out = subprocess.run(["otool", "-t", "-X", "t.o"], cwd=d, check=True, capture_output=True, text=True).stdout
    return bytes(int(b, 16) for line in out.splitlines() if "section" not in line and not line.endswith(":")
                 for b in line.split("\t", 1)[-1].split() if len(b) == 2)


def run_test(test: str, monkeypatch: pytest.MonkeyPatch, entso: bool = True) -> int:
    from w2m import vm_backend
    from w2m.loader import Loaded, Memory
    monkeypatch.setattr(vm_backend, "TRANSLATION_CACHE_DIR", None)
    if not entso:
        monkeypatch.setattr(vm_backend, "ACTLR_ENTSO", 0)          # the plant
    bodies = TESTS[test]
    threads = len(bodies)
    code = b""
    entries = []
    for body in bodies:
        entries.append(CODE + 0x100 + len(code))
        code += assemble((HEAD + body + TAIL).format(ARRIVE=ARRIVE, DONE=DONE))
        code += b"\x90" * (-len(code) % 16)
    program = bytes.fromhex("c6042500000000 00" "ebfe")           # vCPU 0: mov byte [0], 0 (the end); jmp $
    mem = Memory()
    region = mem.map(CODE, 0x10000, "code")
    region.data[:len(program)] = program
    region.data[0x100:0x100 + len(code)] = code
    mem.map(STACK, 0x10000, "stack")
    mem.map(DATA, DATA_SIZE, "data")
    vm = vm_backend.VM(Loaded(None, mem, CODE, {}, STACK + 0x10000))   # type: ignore[arg-type]
    vm.vcpu_count = threads + 1
    found: dict[str, Any] = {}

    def run_thread(v: Any, k: int, n: int, wait_for: int) -> None:
        """Thread k on vCPU k + 1 to its DONE, servicing dispatch misses and slice ends as steps() does."""
        channel = v.vcpu_channels[k]
        x = [0] * 31
        x[1], x[7], x[9], x[17] = n, DATA, wait_for, entries[k]
        x[4] = STACK + 0x8000 - 0x100 * k
        x[vm_backend.REG_UNDEFINED_FLAGS] = 0x3F
        x[28] = vm_backend.MONITOR_VA + vm_backend.MONITOR_DATA
        x[vm_backend.REG_BUDGET] = 1 << 62
        pc = v.dispatch_va
        while True:
            e, _ = channel.run(pc, vm_backend.CPSR_EL1H, x, 60_000, None)
            x = e.x
            imm = e.syndrome & 0xFFFF
            if e.kind != "hvc":
                raise AssertionError(f"{test} thread {k}: {e.kind} at 0x{e.pc:x} esr 0x{e.esr:x}")
            if imm == 1 and x[17] == DONE:
                return
            if imm == 1:
                with lock:
                    va = v.blocks.get(x[17]) or v._compile(x[17])
                    v._dispatch_insert(x[17], va)
                pc = v.miss_va + 4
            elif imm == 4:
                x[vm_backend.REG_BUDGET] = 1 << 62
                pc = e.pc
            else:
                raise AssertionError(f"{test} thread {k}: hvc #{imm}")

    lock = threading.Lock()

    def on_dispatch(v: Any, x: list[int]) -> None:
        if found:
            return
        for k in range(threads):
            # no other thread is ready on a litmus vCPU (its private monitor page's SLICE_UNTIL, which a PAUSE reads;
            # 0 there made every barrier spin a slice end and a trip to the host); the scheduler sets it from 106
            struct.pack_into("<Q", v.mm, vm_backend.VCPU_AREA + (k + 1) * vm_backend.VCPU_STRIDE
                             + vm_backend.VS_MONITOR_DATA + vm_backend.SLICE_UNTIL, (1 << 64) - 1)
        for k in range(threads):                      # alone, once: every block compiled
            run_thread(v, k, 1, 1)
        v.write_bytes(DATA, bytes(DATA_SIZE))
        workers = [threading.Thread(target=run_thread, args=(v, k, N, threads)) for k in range(threads)]
        for w in workers:
            w.start()
        for w in workers:
            w.join()
        raw = v.read_bytes(DATA, R + 4 * A)
        found["words"] = [[int.from_bytes(raw[o + 8 * i:o + 8 * i + 8], "little") for i in range(N)]
                          for o in (X, Y, R, R + A, R + 2 * A, R + 3 * A)]

    try:
        vm.run(max_steps=1 << 20, timeout_ms=600_000, on_dispatch=on_dispatch)
    finally:
        vm.close()
    return forbidden(test, found["words"])


@pytest.mark.hvf
@pytest.mark.parametrize("test", ["MP", "LB", "2+2W", "IRIW"])
def test_x86_tso_forbidden_outcomes_never_appear(test: str, monkeypatch: pytest.MonkeyPatch) -> None:
    assert run_test(test, monkeypatch) == 0


@pytest.mark.hvf
def test_the_store_buffer_shows_in_sb(monkeypatch: pytest.MonkeyPatch) -> None:
    """SB's r1 = r2 = 0 is x86-TSO's one relaxation: seen here, the harness can see a reordering, so 0 elsewhere
    means the order held. It appeared 0 to 53 times in 100,000, and not at all in 3 runs of 100 (2026-10-17): up to
    five runs."""
    assert any(run_test("SB", monkeypatch) > 0 for _ in range(5))


@pytest.mark.hvf
def test_planted_entso_off_shows_mp_forbidden_outcome(monkeypatch: pytest.MonkeyPatch) -> None:
    """The plant: ACTLR_EL1.EnTSO clear on every vCPU. A run shows MP's forbidden outcome or does not: 3 of 25 runs on
    2026-10-17 showed none, the rest 1 to 11 in 100,000 (a run's count is not Poisson, so three runs, which Sprint 105
    took, missed about one time in 600 and failed the gates once, Sprint 110). Up to ten runs: the plant's own miss
    near 0.12 ** 10, six in ten billion; ten runs take about 4 s."""
    assert any(run_test("MP", monkeypatch, entso=False) > 0 for _ in range(10))
