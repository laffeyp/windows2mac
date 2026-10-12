# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""Each VM compiles calls into its own runtime image (Sprint 058, the review of Sprint 055: defect 7). The symbol
addresses were one table for the host, rewritten by each VM built, so a VM built first called the CPUID handler at the
address the last-built VM's image put it."""

from __future__ import annotations

import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))

import family_run  # noqa: E402


@pytest.mark.hvf
def test_a_vm_calls_its_own_images_cpuid_handler() -> None:
    from w2m.loader import load
    from w2m.vm_backend import VM
    f = family_run.FAMILIES["system_info"]
    binary = (ROOT / "store" / "binaries" / f"{next(iter(f.runs.values())).binary}.exe").read_bytes()
    text = (family_run.GUEST / "runtime.c").read_text()
    anchor = '#include "runtime.h"\n'
    assert text.count(anchor) == 1
    shifted = {"runtime.c": text.replace(anchor, anchor + '__asm__(".text\\n.space 4096\\n");\n')}
    first = VM(load(binary), image_name="probe.exe", runtime_sources=shifted)
    second = VM(load(binary), image_name="probe.exe")
    try:
        assert first.runtime.symbols["w2m_cpuid"] != second.runtime.symbols["w2m_cpuid"]
        r = first.run(max_steps=1 << 30, timeout_ms=60_000)
    finally:
        first.close()
        second.close()
    assert r.stop == "ExitProcess", r.stop
    assert family_run.unequal_rows(f, bytes(first.out)) == []
