# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""The runtime lock (Sprint 106; docs/design-several-vcpus.md, one lock first). Two x86 threads on two vCPUs call
HeapAlloc and HeapFree through their bridges at once, 20,000 times each: each writes its own mark at both ends of its
48 bytes, spins, and counts a mark that is not its own. heap.c keeps its free lists in file-scope variables and makes no
host request, so only the lock keeps two vCPUs out of them at once. Each thread runs once alone first, so its blocks
exist before the two run together (tests/test_litmus.py's harness)."""

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
N = 20_000
THREAD = """.intel_syntax noprefix
    sub rsp, 40
    call rsi
    mov rbp, rax
1:  mov rcx, rbp
    xor edx, edx
    mov r8d, 48
    call r15
    test rax, rax
    jz 5f
    mov rdi, rax
    mov qword ptr [rdi], r13
    mov qword ptr [rdi + 40], r13
    mov eax, 64
2:  dec eax
    jnz 2b
    cmp qword ptr [rdi], r13
    jne 3f
    cmp qword ptr [rdi + 40], r13
    je 4f
3:  lock inc qword ptr [r14]
4:  mov rcx, rbp
    xor edx, edx
    mov r8, rdi
    call rbx
    jmp 6f
5:  lock inc qword ptr [r14 + 8]
6:  dec r12
    jnz 1b
    mov rax, {DONE}
    jmp rax
"""


def assemble(text: str) -> bytes:
    with tempfile.TemporaryDirectory() as d:
        (Path(d) / "t.s").write_text(text)
        subprocess.run(["clang", "-c", "-target", "x86_64-apple-macos", "t.s", "-o", "t.o"], cwd=d, check=True)
        out = subprocess.run(["otool", "-t", "-X", "t.o"], cwd=d, check=True, capture_output=True, text=True).stdout
    return bytes(int(b, 16) for line in out.splitlines() if "section" not in line and not line.endswith(":")
                 for b in line.split("\t", 1)[-1].split() if len(b) == 2)


def run_two(monkeypatch: pytest.MonkeyPatch, locked: bool = True) -> dict[str, Any]:
    from w2m import vm_backend
    from w2m.loader import THUNK_BASE, Loaded, Memory
    monkeypatch.setattr(vm_backend, "TRANSLATION_CACHE_DIR", None)
    if not locked:                                                   # the plant: no lock words anywhere
        monkeypatch.setattr(vm_backend, "lock_runtime", lambda a: None)
        monkeypatch.setattr(vm_backend, "unlock_runtime", lambda a: None)
    thunks = {THUNK_BASE: "KERNEL32.dll!GetProcessHeap", THUNK_BASE + 16: "KERNEL32.dll!HeapAlloc",
              THUNK_BASE + 32: "KERNEL32.dll!HeapFree"}
    code = assemble(THREAD.format(DONE=DONE))
    program = bytes.fromhex("c6042500000000 00" "ebfe")           # vCPU 0: mov byte [0], 0 (the end); jmp $
    mem = Memory()
    region = mem.map(CODE, 0x10000, "code")
    region.data[:len(program)] = program
    region.data[0x100:0x100 + len(code)] = code
    mem.map(STACK, 0x20000, "stack")
    mem.map(DATA, 0x1000, "data")
    vm = vm_backend.VM(Loaded(None, mem, CODE, thunks, STACK + 0x20000))   # type: ignore[arg-type]
    vm.vcpu_count = 3
    found: dict[str, Any] = {"failures": []}
    lock = threading.Lock()

    def run_thread(v: Any, k: int, n: int) -> None:
        channel = v.vcpus[k + 1].channel
        x = [0] * 31
        x[3], x[6], x[15] = THUNK_BASE + 32, THUNK_BASE, THUNK_BASE + 16          # rbx, rsi, r15
        x[12], x[13], x[14] = n, 0x1111_0000_0000_0000 * (k + 1) | k, DATA + 16 * k   # r12, r13, r14
        x[17] = CODE + 0x100
        x[4] = STACK + 0x10000 * (k + 1) - 0x100
        x[vm_backend.REG_UNDEFINED_FLAGS] = 0x3F
        x[28] = vm_backend.MONITOR_VA + vm_backend.MONITOR_DATA
        x[vm_backend.REG_BUDGET] = 1 << 62
        pc = v.dispatch_va
        while True:
            e, _ = channel.run(pc, vm_backend.CPSR_EL1H, x, 20_000, None)
            x = e.x
            imm = e.syndrome & 0xFFFF
            if e.kind != "hvc" or imm not in (1, 4):
                found["failures"].append(f"thread {k}: {e.kind} #{imm} at 0x{e.pc:x} esr 0x{e.esr:x} far 0x{e.far:x}")
                return
            if imm == 1 and x[17] == DONE:
                return
            if imm == 1:
                with lock:
                    va = v.bridges.get(x[17]) or v.blocks.get(x[17]) or v._compile(x[17])
                    v._dispatch_insert(x[17], va)
                pc = v.miss_va + 4
            else:
                x[vm_backend.REG_BUDGET] = 1 << 62
                pc = e.pc

    def on_dispatch(v: Any, x: list[int]) -> None:
        if "counts" in found:
            return
        for k in range(2):
            struct.pack_into("<Q", v.mm, vm_backend.VCPU_AREA + (k + 1) * vm_backend.VCPU_STRIDE
                             + vm_backend.VS_MONITOR_DATA + vm_backend.SLICE_UNTIL, (1 << 64) - 1)
        for k in range(2):
            run_thread(v, k, 1)
        v.write_bytes(DATA, bytes(64))
        workers = [threading.Thread(target=run_thread, args=(v, k, N)) for k in range(2)]
        for w in workers:
            w.start()
        for w in workers:
            w.join()
        raw = v.read_bytes(DATA, 32)
        found["counts"] = struct.unpack("<4Q", raw)            # thread 0 foreign, null; thread 1 foreign, null

    try:
        vm.run(max_steps=1 << 20, timeout_ms=600_000, on_dispatch=on_dispatch)
    finally:
        vm.close()
    return found


@pytest.mark.hvf
def test_two_vcpus_allocate_through_the_runtime_one_at_a_time(monkeypatch: pytest.MonkeyPatch) -> None:
    found = run_two(monkeypatch)
    assert found["failures"] == [] and found["counts"] == (0, 0, 0, 0), found


@pytest.mark.hvf
def test_planted_no_lock_lets_two_vcpus_corrupt_the_heap(monkeypatch: pytest.MonkeyPatch) -> None:
    """The plant: no lock words. On 2026-10-17 three runs showed 45 and 141, 16 and 12, 14 and 7 foreign marks, two
    of them a fault at a mark read as a free-list pointer, and an earlier one a torn record in the service ring."""
    try:
        found = run_two(monkeypatch, locked=False)
    except (KeyError, RuntimeError, AssertionError):            # the host meets the corruption first
        return
    assert found["failures"] or any(found.get("counts", (1,))), found
