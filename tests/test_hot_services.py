# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""Hot services stay in the guest (Decision 2026-10-09; Sprint 044 observation contract): over a whole
probe run, QueryPerformanceCounter, GetCurrentThreadId and TlsGetValue make no host exit. The
transport's requests by op (vm.host_requests) are the exits."""

from __future__ import annotations

from pathlib import Path

import pytest

from w2m.loader import load

ROOT = Path(__file__).resolve().parent.parent
PROCESS_QUERIES = "39e501bce923469a19ac478de13b430949a8eb4719e2ca96a3785aba09468034"   # family process_queries
THREAD_STORAGE = "4426a716a6a15990383d970d2ee20c6b14b651128b6866fd5f70e4f7fda5a10c"                                                # family thread_storage


def _run(binary: str) -> tuple[dict[int, int], dict[str, int], str]:
    from w2m.vm_backend import VM
    vm = VM(load((ROOT / "store" / "binaries" / f"{binary}.exe").read_bytes()), image_name="probe.exe")
    try:
        r = vm.run(max_steps=1 << 40, timeout_ms=60_000)
        return dict(vm.host_requests), vm.calls_total(), r.stop
    finally:
        vm.close()


@pytest.mark.hvf
def test_hot_services_make_no_host_exit() -> None:
    from w2m.vm_backend import HOST_EXIT, HOST_FILETIME, HOST_UNIMPLEMENTED, HOST_WRITE
    requests, calls, stop = _run(PROCESS_QUERIES)
    assert stop == "ExitProcess", stop
    assert calls["QueryPerformanceCounter"] == 100_005 and calls["GetCurrentThreadId"] == 1
    # one exit each: the wall clock once (then the counter), the rows, the exit; GetStartupInfoW's title is
    # ProcessParameters' since Sprint 059, not a module-path request
    assert requests == {HOST_FILETIME: 1, HOST_WRITE: 1, HOST_EXIT: 1}, requests
    requests, calls, stop = _run(THREAD_STORAGE)
    assert stop == "Unimplemented: FlsFree argument form 0x1", stop        # the family's expected stop
    assert calls["TlsGetValue"] == 5
    assert requests == {HOST_WRITE: 1, HOST_UNIMPLEMENTED: 1}, requests
