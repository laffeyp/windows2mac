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
