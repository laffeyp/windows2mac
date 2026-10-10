# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""run_skeleton.py — the walking skeleton end to end (Sprint 029).

Input: a runner artifact directory from `windows-probe.yml` in stdout mode (body-<i>.jsonl,
exec-<i>.json, probe.exe, probe.sha256, profile.txt). The script
  1. wraps each runner body in GUEST_TRACE_OPENED/CLOSED and judges it (Windows reference);
  2. runs the same probe.exe on the Mac in w2m's model runtime (loader -> x86dec -> lift ->
     runtime with host kernel32), wraps its stdout body the same way, and judges it;
  3. (Sprint 030) runs it in the managed-VM back end (lift -> AArch64 in a Hypervisor.framework
     VM, kernel32 in the host), after lockstep and random-state comparison with the model, and
     judges those traces too;
all through scripts/judge_traces.py, against one contract. The probe binary is copied into
store/binaries/<sha256>.exe; the Mac runs execute those bytes.

Usage: uv run python scripts/run_skeleton.py <artifact dir> --run-id ID
           --contract windows.thread.last_error.set_get@0.1.0 [--mac-runs 3]
"""

from __future__ import annotations

import argparse
import hashlib
import json
import shutil
import subprocess
import sys
import tempfile
import time
from collections.abc import Callable
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))
import judge_traces  # noqa: E402

from w2m import VOCAB_VERSION  # noqa: E402
from w2m.loader import load  # noqa: E402
from w2m.runtime import Result, Runtime  # noqa: E402
from w2m.vm_backend import VM, lockstep, record_translations, state_differential  # noqa: E402
from w2m.x86dec_tables import SPEC_VERSION  # noqa: E402


def wrap(body: bytes, execution_id: str, profile_id: str) -> tuple[bytes, dict[str, int]]:
    lines = body.splitlines()
    tid = json.loads(lines[0])["thread_id"]
    opened = json.dumps({"tag": "GUEST_TRACE_OPENED", "execution_id": execution_id,
                         "vocab_version": VOCAB_VERSION, "profile_id": profile_id,
                         "trace_format_version": "0.1"}, separators=(",", ":"))
    closed = json.dumps({"tag": "GUEST_TRACE_CLOSED", "line_count": len(lines) + 2,
                         "thread_count": len({json.loads(x)["thread_id"] for x in lines}),
                         "dropped_count": 0}, separators=(",", ":"))
    return b"\n".join([opened.encode(), *lines, closed.encode()]) + b"\n", {"main": tid}


def stored_reference_ids(store: Path, probe_sha: str) -> dict[str, str]:
    """sha256 of each stored Windows reference body of `probe_sha` -> its execution id."""
    out = {}
    for m in (store / "executions").glob("*.json"):
        meta = json.loads(m.read_text())
        if meta.get("probe_sha256") != probe_sha or "profile_kind" in meta:
            continue
        lines = (store / "traces" / f"{m.stem}.jsonl").read_bytes().splitlines()
        body = b"\n".join(lines[1:-1]) + b"\n"
        out[hashlib.sha256(body).hexdigest()] = json.loads(lines[0])["execution_id"]
    return out


def sh(*cmd: str) -> str:
    return subprocess.run(cmd, capture_output=True, text=True, check=True).stdout.strip()


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("src", type=Path)
    ap.add_argument("--run-id", required=True)
    ap.add_argument("--contract", required=True)
    ap.add_argument("--prefix", default="skel")
    ap.add_argument("--mac-runs", type=int, default=3)
    a = ap.parse_args(argv)
    exe = (a.src / "probe.exe").read_bytes()
    digest = hashlib.sha256(exe).hexdigest()
    if digest != (a.src / "probe.sha256").read_text().strip().lower():
        raise SystemExit("probe.exe does not match probe.sha256")
    (ROOT / "store" / "binaries").mkdir(parents=True, exist_ok=True)
    (ROOT / "store" / "binaries" / f"{digest}.exe").write_bytes(exe)
    status = 0
    with tempfile.TemporaryDirectory() as tmp:
        win = Path(tmp) / "win"
        win.mkdir()
        (win / "probe.sha256").write_text(digest)
        profile = (a.src / "profile.txt").read_text().strip()
        stored = stored_reference_ids(ROOT / "store", f"sha256:{digest}")
        for body in sorted(a.src.glob("body-*.jsonl"), key=lambda p: int(p.stem.split("-")[1])):
            i = body.stem.split("-")[1]
            # a runner body already in the store keeps its execution id, so re-judging it adds no
            # second reference execution (Sprint 035: a new prefix doubled the 0.2.0 references)
            xid = stored.get(hashlib.sha256(body.read_bytes()).hexdigest(), f"{a.prefix}-{i}")
            trace, roles = wrap(body.read_bytes(), xid, profile)
            (win / f"trace-{i}.jsonl").write_bytes(trace)
            (win / f"roles-{i}.json").write_text(json.dumps(roles))
            shutil.copy(a.src / f"exec-{i}.json", win / f"exec-{i}.json")
        print("windows reference:")
        status |= judge_traces.main([str(win), "--run-id", a.run_id, "--contract", a.contract])

        host = (f"{sh('sysctl', '-n', 'hw.model').replace(',', '-').lower()}-"
                f"macos-{sh('sw_vers', '-buildVersion').lower()}-spec-{SPEC_VERSION}")
        ls = lockstep(exe)
        sd = state_differential(exe)
        print(f"lockstep: {ls.compared} comparisons, {len(ls.differences)} differences; "
              f"state differential: {sd.cases} cases over {sd.instructions} instructions, "
              f"{len(sd.differences)} differences")
        if ls.differences or sd.differences:
            print("\n".join(ls.differences + sd.differences))
            return 1

        translation_records: list[Path] = []
        finished_vms: list[VM] = []

        def run_vm() -> Result:
            vm = VM(load(exe))
            try:
                return vm.run()
            finally:
                finished_vms.append(vm)                # recorded and closed after timing

        backends: tuple[tuple[str, str, Callable[[], Result]], ...] = (
            ("model", "w2m-model", lambda: Runtime(load(exe)).run()),
                    ("managed_vm", "w2m-vm", run_vm))
        for kind, prefix, execute in backends:
            out = Path(tmp) / kind
            out.mkdir()
            (out / "probe.sha256").write_text(digest)
            stamp = time.time_ns()
            for i in range(1, a.mac_runs + 1):
                t0 = time.monotonic()
                r = execute()
                elapsed = time.monotonic() - t0
                print(f"{kind} run {i}: stop {r.stop}; exit {r.exit_code}; {r.instructions_executed} "
                      f"instructions, {r.distinct_addresses} distinct; {elapsed * 1000:.1f} ms")
                if r.exit_code is None:
                    return 1
                trace, roles = wrap(r.stdout, f"{a.prefix}-{kind}-{stamp}-{i}", f"{prefix}-{host}")
                (out / f"trace-{i}.jsonl").write_bytes(trace)
                (out / f"roles-{i}.json").write_text(json.dumps(roles))
                (out / f"exec-{i}.json").write_text(json.dumps({
                    "exit_code": r.exit_code, "elapsed_seconds": round(elapsed, 6),
                    "profile_kind": kind, "instructions_executed": r.instructions_executed,
                    "distinct_addresses": r.distinct_addresses}))
            if kind == "managed_vm":
                for vm in finished_vms:
                    try:
                        translation_records.append(record_translations(
                            vm, f"sha256:{digest}", ROOT / "runs", f"w2m-vm-{host}"))
                    finally:
                        vm.close()
                for rec in translation_records:
                    print(f"translations {rec}")
            print(f"mac {kind}:")
            status |= judge_traces.main([str(out), "--run-id", f"{kind}-{stamp}",
                                         "--contract", a.contract])
    return status


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
