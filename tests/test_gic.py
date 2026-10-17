# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""The GICv3 is made before any vCPU (Sprint 097, roadmap step 2). hv_gic.h: "The hv_gic_create() API must only be
called after a virtual machine has been created. It must also be done before vCPU's have been created", and "vCPU's
must set affinity values in their MPIDR_EL1 register". The monitor places the distributor and the redistributor region
at the top of the 40-bit IPA space, above the guest's memory, and reports them with its `gic` command."""

from __future__ import annotations

import subprocess
import tempfile

import pytest

SIZE = 0x10_0000


@pytest.mark.hvf
def test_the_gic_is_made_before_the_vcpu_above_the_guests_memory() -> None:
    from w2m import vm_backend
    with tempfile.NamedTemporaryFile() as f:
        f.truncate(SIZE)
        proc = subprocess.run([str(vm_backend.build_monitor()), f.name, f"{vm_backend.IPA_BASE:x}", f"{SIZE:x}"],
                              input="gic\n", capture_output=True, text=True, timeout=30)
    assert proc.returncode == 0, proc.stderr
    word, distributor, redistributor, region, mpidr = proc.stdout.split()
    d, r, n, m = (int(v, 16) for v in (distributor, redistributor, region, mpidr))
    assert word == "gic"
    assert vm_backend.IPA_BASE + SIZE <= d < r and r + n <= 1 << 40      # above the guest, within the IPA space
    assert r % 0x10000 == 0 and d % 0x10000 == 0                         # 64 KB frames
    assert n >= 2 * 0x10000                                               # two frames for each vCPU, at least one
    assert m == 1 << 31                                                   # affinity 0.0.0.0, bit 31 RES1
