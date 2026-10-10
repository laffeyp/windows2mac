"""probe_on_intel.py — run a windows-probe workflow until an Intel CPU answers (Sprint 039).

GitHub's windows-2025 runners are AMD EPYC or Intel Xeon machines, assigned per job; a workflow
cannot choose. Sprint 037 found the two vendors differ on defined behaviour (0F 0D register forms:
NOP on Intel, #UD on AMD), so a verdict that needs Intel's behaviour must come from an Intel run.
This driver dispatches `windows-probe.yml`, waits for the run, downloads its output to
build/probe-runs/<run id>/, reads cpu.txt, and dispatches again while the CPU is not Intel, up to
--attempts times. Every run's CPU is printed and kept; none is discarded.

Usage: uv run python scripts/probe_on_intel.py <probe> [--repeat 3] [--attempts 8] [--ref BRANCH]
Exit 0 with the Intel run's directory on the last line; 1 when no attempt reached Intel.
"""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
import time
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / "build" / "probe-runs"


def gh(*args: str) -> str:
    return subprocess.run(["gh", *args], check=True, capture_output=True, text=True, cwd=ROOT).stdout


def dispatch(probe: str, execution_id: str, repeat: int, ref: str) -> int:
    since = datetime.now(timezone.utc)
    gh("workflow", "run", "windows-probe.yml", "--ref", ref, "-f", f"probe={probe}",
       "-f", f"execution_id={execution_id}", "-f", f"repeat={repeat}")
    for _ in range(60):
        time.sleep(5)
        runs = json.loads(gh("run", "list", "--workflow", "windows-probe.yml", "--event", "workflow_dispatch",
                             "--limit", "10", "--json", "databaseId,createdAt"))
        new = [r for r in runs if datetime.fromisoformat(r["createdAt"].replace("Z", "+00:00")) >= since]
        if new:
            return int(min(new, key=lambda r: r["createdAt"])["databaseId"])
    raise RuntimeError("the dispatched run did not appear within 5 minutes")


def wait(run: int) -> str:
    subprocess.run(["gh", "run", "watch", str(run), "--exit-status", "--interval", "15"], cwd=ROOT,
                   capture_output=True, text=True)
    return json.loads(gh("run", "view", str(run), "--json", "conclusion"))["conclusion"]


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("probe")
    ap.add_argument("--repeat", type=int, default=3)
    ap.add_argument("--attempts", type=int, default=8)
    ap.add_argument("--ref", default="main")
    a = ap.parse_args(argv)
    for attempt in range(1, a.attempts + 1):
        run = dispatch(a.probe, f"{a.probe}-{int(time.time())}", a.repeat, a.ref)
        conclusion = wait(run)
        target = OUT / str(run)
        target.mkdir(parents=True, exist_ok=True)
        subprocess.run(["gh", "run", "download", str(run), "-D", str(target)], cwd=ROOT, check=False,
                       capture_output=True)
        cpu_files = list(target.rglob("cpu.txt"))
        cpu = cpu_files[0].read_text().strip() if cpu_files else "no cpu.txt"
        print(f"attempt {attempt}: run {run} {conclusion}; {cpu}", flush=True)
        if conclusion != "success":
            print(f"run {run} failed; see `gh run view {run} --log-failed`", file=sys.stderr)
            return 1
        if "Intel" in cpu:
            print(cpu_files[0].parent)
            return 0
    return 1


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
