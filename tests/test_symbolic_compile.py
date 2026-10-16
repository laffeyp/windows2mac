# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""Sprint 090: stencils made by running compile_insn on symbolic templates (src/w2m/symbolic_compile.py), against
compile_insn, for a register move, a load through base and index, an ADD of an immediate (the live flags' forks),
a call and a Jcc (the symbolic exit), at three flag contexts each. scripts/stencil_from_rules_judge.py runs the same
judge over the record."""

from __future__ import annotations

import pytest

from w2m import vm_backend as vb
from w2m.symbolic_compile import Table, compile_words, words_by_stencil

SYMBOLS = {"w2m_cpuid": vb.MONITOR_VA + vb.RUNTIME_IMAGE + 0x2000,
           "w2m_approximate": vb.MONITOR_VA + vb.RUNTIME_IMAGE + 0x3000,
           "w2m_pcmpistri": vb.MONITOR_VA + vb.RUNTIME_IMAGE + 0x4000}


@pytest.mark.timeout(300)
def test_stencils_equal_compile_insn() -> None:
    table = Table()
    checked = 0
    for code in ("4889c8", "488b443008", "4883c0f0", "e800000000", "0f84f0ffffff"):
        raw = bytes.fromhex(code)
        for k, (nzcv_from, live, nzcv_out) in enumerate(((None, vb.ALL_FLAGS, False), (None, 0, False),
                                                         ("sub", 5, True))):
            base = vb.MONITOR_VA + vb.CODE_CACHE + 0x1000 * (k + 1)
            exit_va = vb.MONITOR_VA + vb.DISPATCHER
            got, why = words_by_stencil(table, raw + bytes(15), 0x1_4000_1000, base, exit_va, SYMBOLS, nzcv_from,
                                        live, nzcv_out)
            assert got == compile_words(raw, 0x1_4000_1000, base, exit_va, SYMBOLS, nzcv_from, live, nzcv_out), (code, why)
            checked += 1
    assert checked == 15
