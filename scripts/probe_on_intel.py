"""probe_on_intel.py — run a windows-probe workflow until an Intel CPU answers (Sprint 039).

GitHub's Windows runners are AMD EPYC or Intel Xeon machines, assigned per job; a workflow cannot
choose. Sprint 037 found the two vendors differ on defined behaviour (0F 0D register forms: NOP on
Intel, #UD on AMD), so a verdict that needs Intel's behaviour must come from an Intel run. Sprint
037 drew Intel in 2 of 5 windows-2025 jobs; Sprint 039's first 8 sequential jobs were all AMD.

This driver dispatches `windows-probe.yml` in batches (--batch jobs at once, alternating the
runner images in --runners), waits for every job of the batch, downloads each to
build/probe-runs/<run id>/, reads cpu.txt, and stops after the first batch that holds an Intel
run, up to --batches batches. Every run's CPU is printed and kept; none is discarded.

Usage: uv run python scripts/probe_on_intel.py <probe> [--repeat 3] [--batch 6] [--batches 4]
       [--runners windows-2025,windows-2022] [--ref main]
Exit 0 with each Intel run's directory on the last lines; 1 when no batch reached Intel.
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


def dispatch_batch(probe: str, repeat: int, ref: str, runners: list[str], size: int) -> list[int]:
    since = datetime.now(timezone.utc)
    for i in range(size):
        gh("workflow", "run", "windows-probe.yml", "--ref", ref, "-f", f"probe={probe}",
           "-f", f"execution_id={probe}-{int(time.time())}-{i}", "-f", f"repeat={repeat}",
           "-f", f"runner={runners[i % len(runners)]}")
    for _ in range(60):
        time.sleep(5)
        runs = json.loads(gh("run", "list", "--workflow", "windows-probe.yml", "--event", "workflow_dispatch",
                             "--limit", str(size + 10), "--json", "databaseId,createdAt"))
        new = [int(r["databaseId"]) for r in runs
               if datetime.fromisoformat(r["createdAt"].replace("Z", "+00:00")) >= since]
        if len(new) >= size:
            return sorted(new)
    raise RuntimeError(f"fewer than {size} dispatched runs appeared within 5 minutes")


def collect(run: int) -> tuple[str, str, Path | None]:
    subprocess.run(["gh", "run", "watch", str(run), "--exit-status", "--interval", "15"], cwd=ROOT,
                   capture_output=True, text=True)
    conclusion = json.loads(gh("run", "view", str(run), "--json", "conclusion"))["conclusion"]
    target = OUT / str(run)
    target.mkdir(parents=True, exist_ok=True)
    subprocess.run(["gh", "run", "download", str(run), "-D", str(target)], cwd=ROOT, check=False,
                   capture_output=True)
    cpu_files = list(target.rglob("cpu.txt"))
    if not cpu_files:
        return conclusion, "no cpu.txt", None
    return conclusion, cpu_files[0].read_text().strip(), cpu_files[0].parent


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("probe")
    ap.add_argument("--repeat", type=int, default=3)
    ap.add_argument("--batch", type=int, default=6)
    ap.add_argument("--batches", type=int, default=4)
    ap.add_argument("--runners", default="windows-2025,windows-2022")
    ap.add_argument("--ref", default="main")
    a = ap.parse_args(argv)
    runners = a.runners.split(",")
    for n in range(1, a.batches + 1):
        runs = dispatch_batch(a.probe, a.repeat, a.ref, runners, a.batch)
        intel = []
        results: list[tuple[str, str]] = []
        for run in runs:
            conclusion, cpu, where = collect(run)
            results.append((conclusion, cpu))
            print(f"batch {n}: run {run} {conclusion}; {cpu}", flush=True)
            if conclusion == "success" and where is not None and "intel" in cpu.lower():
                intel.append(where)
        if intel:
            for where in intel:
                print(where)
            return 0
        if not any(c == "success" for c, _ in results):
            print(f"batch {n}: every run failed; the probe or the workflow is broken, not the CPU draw", flush=True)
            return 2
    return 1


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
