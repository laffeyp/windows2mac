"""Sprint 029 grading surface: the walking skeleton. The runner-built skeleton.exe (committed in
store/binaries) runs through loader -> x86dec -> lift -> runtime, and its trace is judged against
windows.thread.last_error.set_get@0.1.0. Planted faults must turn the verdict, or stop the run."""

from __future__ import annotations

import hashlib
import json
from pathlib import Path

import pytest

from w2m.compare import compare_allowed_outcome_set
from w2m.contract import load_contract
from w2m.lift import lift
from w2m.loader import load
from w2m.runtime import Runtime

ROOT = Path(__file__).resolve().parent.parent
SKELETON = ROOT / "store" / "binaries"
CONTRACT = ROOT / "contracts" / "windows.thread.last_error.set_get" / "0.1.0.json"


def _skeleton() -> bytes:
    for p in sorted(SKELETON.glob("*.exe")):
        data = p.read_bytes()
        if b"GUEST_THREAD_STARTED" in data and b"SetLastError" in data and b"bcrypt" not in data.lower():
            return data
    pytest.skip("skeleton.exe not in store/binaries")


def _verdict(stdout: bytes) -> str:
    lines = stdout.splitlines()
    tid = json.loads(lines[0])["thread_id"]
    opened = b'{"tag":"GUEST_TRACE_OPENED","execution_id":"t","vocab_version":"0.5","profile_id":"t","trace_format_version":"0.1"}'
    raw = b"\n".join([opened, *lines]) + b"\n"
    return compare_allowed_outcome_set(load_contract(CONTRACT), raw, {"main": tid}, "c", "r", "t").verdict


def test_skeleton_runs_and_conforms() -> None:
    r = Runtime(load(_skeleton())).run()
    assert (r.stop, r.exit_code, r.distinct_addresses) == ("ExitProcess", 0, 217)
    assert r.calls == ["GetCurrentThreadId", "SetLastError", "GetLastError", "GetStdHandle",
                       "WriteFile", "ExitProcess"]
    assert _verdict(r.stdout) == "conforms"


def test_planted_kernel32_fault_is_a_discrepancy() -> None:
    rt = Runtime(load(_skeleton()))
    rt.k32["SetLastError"] = lambda: None                      # forgets the value
    r = rt.run()
    assert r.exit_code == 1 and _verdict(r.stdout) == "discrepancy"


def test_planted_lift_fault_changes_the_trace() -> None:
    """mul_hi drives the decimal formatting (multiply by 0xcccccccccccccccd); breaking it must
    corrupt what the guest reports, not pass silently."""
    import w2m.runtime as rtmod
    good = Runtime(load(_skeleton())).run().stdout
    orig = rtmod.Runtime._exec

    def broken(self: rtmod.Runtime, ins):  # type: ignore[no-untyped-def]
        ops = tuple(("alu", o[1], "mul_lo", *o[3:]) if o[0] == "alu" and o[2] == "mul_hi" else o
                    for o in ins.ops)
        return orig(self, type(ins)(ins.address, ins.length, ins.iclass, ops))

    rtmod.Runtime._exec = broken  # type: ignore[method-assign]
    try:
        bad = Runtime(load(_skeleton())).run(max_steps=200_000)
    finally:
        rtmod.Runtime._exec = orig  # type: ignore[method-assign]
    assert bad.stdout != good


def test_unliftable_instruction_stops_the_run() -> None:
    from w2m.lift import LiftError
    with pytest.raises(LiftError):
        lift(bytes.fromhex("0fa2") + b"\0" * 13, 0x1000)           # CPUID: no lifting rule
    rt = Runtime(load(_skeleton()))
    rt.mem.write(rt.ld.entry, bytes.fromhex("0fa2"))
    r = rt.run()
    assert r.stop.startswith("LiftError") and r.exit_code is None


def test_executes_the_runner_built_bytes() -> None:
    data = _skeleton()
    assert (SKELETON / f"{hashlib.sha256(data).hexdigest()}.exe").exists()
