# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""A guest service that refuses an argument form stops the run as SERVICE_UNIMPLEMENTED says: the
service with its form, and the call's return address in the guest (Sprint 044; before, the frontier
reported such a stop as a fault)."""

from __future__ import annotations

from pathlib import Path

import pytest

from w2m.loader import load

ROOT = Path(__file__).resolve().parent.parent

BINARY = ROOT / "store" / "binaries" / "9395509bfe645a3c33c322e88efdd81285f919b9897020d7a134db5e8cbd45bf.exe"


@pytest.mark.hvf
def test_refused_argument_form_names_the_service_and_its_return_address() -> None:
    from w2m.vm_backend import VM
    source = (ROOT / "tools" / "guest_runtime" / "code_pages.c").read_text()
    old = "u64 k32_GetACP(void) { return w2m_logged(W2M_API_GetACP, 0, ANSI_CP); }"
    assert source.count(old) == 1
    planted = source.replace(old, "u64 k32_GetACP(void) { w2m_unimplemented(W2M_API_GetACP, 7); return 0; }")
    loaded = load(BINARY.read_bytes())
    vm = VM(loaded, image_name="probe.exe", runtime_sources={"code_pages.c": planted})
    try:
        r = vm.run(max_steps=1 << 30, timeout_ms=60_000)
    finally:
        vm.close()
    assert r.stop == "Unimplemented: GetACP argument form 0x7", r.stop
    assert vm.unimplemented is not None
    name, ret = vm.unimplemented
    base, size = loaded.image.image_base, loaded.image.size_of_image
    assert name == "GetACP argument form 0x7"
    assert base <= ret < base + size, hex(ret)                       # the probe's own code called GetACP


@pytest.mark.hvf
def test_an_api_set_with_no_observed_host_names_the_set_and_its_return_address() -> None:
    """Sprint 060: Sprint 059's frontier stopped in LoadLibraryExW on api-ms-win-appmodel-runtime-l1-1-2, a request
    the host refuses, and reported EXECUTION_COMPLETED (faulted) with the stop in `unobserved`; an unserved service is
    SERVICE_UNIMPLEMENTED, which needs the name and the guest's return address."""
    from w2m.vm_backend import VM
    source = (ROOT / "tools" / "guest_runtime" / "code_pages.c").read_text()
    old = "u64 k32_GetACP(void) { return w2m_logged(W2M_API_GetACP, 0, ANSI_CP); }"
    assert source.count(old) == 1
    name = "api-ms-win-appmodel-runtime-l1-1-2"
    units = ", ".join(str(ord(c)) for c in name)
    planted = source.replace(old, "u64 k32_GetACP(void) { static const u16 set[] = {" + units + ", 0}; "
                             f"return w2m_host(W2M_HOST_MODULE, (u64)set, {len(name)}, 1, 0, 0); }}")
    loaded = load(BINARY.read_bytes())
    vm = VM(loaded, image_name="probe.exe", runtime_sources={"code_pages.c": planted})
    try:
        r = vm.run(max_steps=1 << 30, timeout_ms=60_000)
    finally:
        vm.close()
    stop = f"API set {name}: its host module on Windows is not observed"
    assert r.stop == f"Unimplemented: {stop}", r.stop
    assert vm.unimplemented is not None
    service, ret = vm.unimplemented
    base, size = loaded.image.image_base, loaded.image.size_of_image
    assert service == stop
    assert base <= ret < base + size, hex(ret)


@pytest.mark.hvf
def test_the_pc_map_stays_sorted_as_blocks_are_compiled() -> None:
    """Sprint 062: _compile appends each block's (host address, x86 address) marks instead of sorting the whole map,
    because the code cache only grows; x86_address_at's bisect needs the map sorted."""
    from w2m.vm_backend import VM
    vm = VM(load(BINARY.read_bytes()), image_name="probe.exe")
    try:
        vm.run(max_steps=1 << 30, timeout_ms=60_000)
        marks = list(vm.pc_map)
    finally:
        vm.close()
    assert len(marks) > 1000 and marks == sorted(marks)


def test_the_appmodel_api_set_is_kernel_appcore_and_its_policy_is_served() -> None:
    """Sprint 063: the frontier stopped at LoadLibraryExW("api-ms-win-appmodel-runtime-l1-1-2"); on Windows the set is
    kernel.appcore.dll (game run 38206263976 seq 1764-1765) and AppPolicyGetThreadInitializationType answers 0."""
    from w2m.modules import API_SET_HOSTS
    from w2m.vm_backend import service_for
    assert API_SET_HOSTS["api-ms-win-appmodel-runtime-l1-1-2.dll"] == "kernel.appcore.dll"
    assert service_for("kernel.appcore.dll", "AppPolicyGetThreadInitializationType") == \
        ("k32_AppPolicyGetThreadInitializationType", True)
