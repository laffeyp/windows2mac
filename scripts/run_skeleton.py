"""run_skeleton.py — the walking skeleton end to end (Sprint 029).

Input: a runner artifact directory from `windows-probe.yml` in stdout mode (body-<i>.jsonl,
exec-<i>.json, probe.exe, probe.sha256, profile.txt). The script
  1. wraps each runner body in GUEST_TRACE_OPENED/CLOSED and judges it (Windows reference);
  2. runs the same probe.exe on the Mac in w2m's model runtime (loader -> x86dec -> lift ->
     runtime with host kernel32), wraps its stdout body the same way, and judges it;
both through scripts/judge_traces.py, against one contract. The probe binary is copied into
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
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))
import judge_traces  # noqa: E402

from w2m.loader import load  # noqa: E402
from w2m.runtime import Runtime  # noqa: E402
from w2m.x86dec_tables import SPEC_VERSION  # noqa: E402


def wrap(body: bytes, execution_id: str, profile_id: str) -> tuple[bytes, dict[str, int]]:
    lines = body.splitlines()
    tid = json.loads(lines[0])["thread_id"]
    opened = json.dumps({"tag": "GUEST_TRACE_OPENED", "execution_id": execution_id,
                         "vocab_version": "0.5", "profile_id": profile_id,
                         "trace_format_version": "0.1"}, separators=(",", ":"))
    closed = json.dumps({"tag": "GUEST_TRACE_CLOSED", "line_count": len(lines) + 2,
                         "thread_count": len({json.loads(x)["thread_id"] for x in lines}),
                         "dropped_count": 0}, separators=(",", ":"))
    return b"\n".join([opened.encode(), *lines, closed.encode()]) + b"\n", {"main": tid}


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
        win, mac = Path(tmp) / "win", Path(tmp) / "mac"
        for d in (win, mac):
            d.mkdir()
            (d / "probe.sha256").write_text(digest)
        profile = (a.src / "profile.txt").read_text().strip()
        for body in sorted(a.src.glob("body-*.jsonl"), key=lambda p: int(p.stem.split("-")[1])):
            i = body.stem.split("-")[1]
            trace, roles = wrap(body.read_bytes(), f"{a.prefix}-{i}", profile)
            (win / f"trace-{i}.jsonl").write_bytes(trace)
            (win / f"roles-{i}.json").write_text(json.dumps(roles))
            shutil.copy(a.src / f"exec-{i}.json", win / f"exec-{i}.json")
        print("windows reference:")
        status |= judge_traces.main([str(win), "--run-id", a.run_id, "--contract", a.contract])

        mac_profile = (f"w2m-model-{sh('sysctl', '-n', 'hw.model').replace(',', '-').lower()}-"
                       f"macos-{sh('sw_vers', '-buildVersion').lower()}-spec-{SPEC_VERSION}")
        stamp = time.time_ns()
        for i in range(1, a.mac_runs + 1):
            t0 = time.monotonic()
            r = Runtime(load(exe)).run()
            elapsed = time.monotonic() - t0
            print(f"mac run {i}: stop {r.stop}; exit {r.exit_code}; {r.instructions_executed} "
                  f"instructions, {r.distinct_addresses} distinct; calls {r.calls}")
            if r.exit_code is None:
                return 1
            trace, roles = wrap(r.stdout, f"{a.prefix}-mac-{stamp}-{i}", mac_profile)
            (mac / f"trace-{i}.jsonl").write_bytes(trace)
            (mac / f"roles-{i}.json").write_text(json.dumps(roles))
            (mac / f"exec-{i}.json").write_text(json.dumps({
                "exit_code": r.exit_code, "elapsed_seconds": round(elapsed, 6),
                "profile_kind": "model", "instructions_executed": r.instructions_executed,
                "distinct_addresses": r.distinct_addresses}))
        print("mac model runtime:")
        status |= judge_traces.main([str(mac), "--run-id", f"mac-{stamp}", "--contract", a.contract])
    return status


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
