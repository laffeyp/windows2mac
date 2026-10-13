# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""store_probe_runs.py — keep downloaded probe runs in the store and print their family entries (Sprint 054).

For each run id, build/probe-runs/<run>/traces-<execution id>/ (scripts/probe_on_intel.py) is copied to
store/probe-runs/<run>/ (the bodies, exec, cpu, profile and probe.sha256) and its probe.exe to
store/binaries/<sha256>.exe; then one `Run(...)` line per run is printed for scripts/family_run.py, the runner
image read from profile.txt. A laptop run (scripts/windows_machine.py, Sprint 067: profile.txt `laptop-...`) keeps
its profile.json, job.json and run-job.log too, and its image is `laptop`.

Usage: uv run python scripts/store_probe_runs.py <run id> [<run id> ...]
"""

from __future__ import annotations

import shutil
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
KEPT = ("body-1.jsonl", "body-2.jsonl", "cpu.txt", "exec-1.json", "exec-2.json", "probe.sha256", "profile.txt",
        "profile.json", "job.json", "run-job.log")


def image(profile: str) -> str:
    """The machine a run came from, by its profile.txt."""
    if profile.startswith("laptop-"):
        return "laptop"
    return "windows-2025" if "windows-2025" in profile else "windows-2022"


def main(argv: list[str]) -> int:
    for run in argv:
        found = sorted((ROOT / "build" / "probe-runs" / run).glob("traces-*"))
        if len(found) != 1:
            print(f"{run}: {len(found)} trace folders under build/probe-runs/{run}", file=sys.stderr)
            return 1
        src = found[0]
        dst = ROOT / "store" / "probe-runs" / run
        dst.mkdir(parents=True, exist_ok=True)
        for name in KEPT:
            if (src / name).is_file():
                shutil.copyfile(src / name, dst / name)
        sha = (src / "probe.sha256").read_text().strip()
        binary = ROOT / "store" / "binaries" / f"{sha}.exe"
        if not binary.exists():
            shutil.copyfile(src / "probe.exe", binary)
        machine = image((src / "profile.txt").read_text())
        execution = src.name.removeprefix("traces-")
        cpu = (src / "cpu.txt").read_text().strip()
        print(f'              "{run}": Run("{machine}", "{sha}", "{execution}", "{cpu}"),')
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
