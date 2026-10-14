# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""The code census (recentering step 3): with BLOCK_CENSUS each block counts its executions, and the VM weighs each x86
instruction's arm64 words by them. A loop of `dec ecx; jnz` run 1,000 times counts 1,000 of each, every one at the words
its lowering compiled to."""

from __future__ import annotations

import pytest


@pytest.mark.hvf
def test_the_census_counts_each_block_execution(monkeypatch: pytest.MonkeyPatch) -> None:
    from w2m import vm_backend
    from w2m.lift import lift
    from w2m.loader import Loaded, Memory
    monkeypatch.setattr(vm_backend, "BLOCK_CENSUS", True)
    code_va, stack_va = 0x2000_0000, 0x4000_0000
    mem = Memory()
    code = bytes.fromhex("b9e8030000" "ffc9" "75fc" "c6042500000000 00")   # mov ecx, 1000; l: dec ecx; jnz l; the end
    mem.map(code_va, 0x1000, "code").data[:len(code)] = code
    mem.map(stack_va, 0x10000, "stack")
    vm = vm_backend.VM(Loaded(None, mem, code_va, {}, stack_va + 0x10000))   # type: ignore[arg-type]
    try:
        vm.run(max_steps=1 << 20, timeout_ms=10_000)
        _x86, _body, _prologue, by = vm.code_census()
    finally:
        vm.close()

    def words(hexcode: str, after: str | None = None) -> int:
        """The words `hexcode` compiles to, after the instruction `after` in its block (a jump after a setter
        branches on its NZCV)."""
        setter = vm_backend.nzcv_setter(lift(bytes.fromhex(after), code_va)) if after else None
        a = vm_backend.Assembler(0)
        vm_backend.compile_insn(a, lift(bytes.fromhex(hexcode), code_va), 0x8000, None, setter)
        return len(a.finish())
    assert by["DEC"] == (1000, 1000 * words("ffc9"))
    assert by["JNZ"] == (1000, 1000 * words("75fc", after="ffc9"))
