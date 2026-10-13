# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""windows_machine.py — run a probe on the Windows reference machine, the laptop (Sprint 067 item B).

The controller's half of docs/windows-reference-machine.md §3. A job is the committed tree (`git archive HEAD`,
store/ left out) and, for a probe with a `generate` step, substrate at substrate.commit, each named by sha256 in
job.json. The job is copied to C:\\w2m\\jobs\\<run>\\ over SSH (host `w2m-laptop` in ~/.ssh/config) with the
committed tools/windows_machine/run-job.ps1 and profile.ps1, run by that run-job.ps1, and its out\\ folder copied
back to build/probe-runs/<run>/traces-<execution id>/, the layout scripts/store_probe_runs.py reads; the job folder
is then deleted on the laptop.

A `binary` job runs a stored binary (store/binaries/<sha256>.exe) unchanged in place of the build, for
calibration (§6 step 2): the family's first run names the binary and any DLL it imports.

Usage: uv run python scripts/windows_machine.py probe <probe> [--repeat 2]
       uv run python scripts/windows_machine.py binary <family> [--run <stored run id>] [--repeat 2]
"""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
import sys
import tempfile
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
HOST = "w2m-laptop"
JOBS = "C:/w2m/jobs"


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def ssh(command: str) -> int:
    return subprocess.run(["ssh", HOST, command]).returncode


def probe(name: str, repeat: int, binary: str | None = None, dll: str | None = None) -> int:
    if not (ROOT / "probes" / name).is_dir():
        print(f"no probes/{name}", file=sys.stderr)
        return 1
    head = subprocess.run(["git", "-C", str(ROOT), "rev-parse", "HEAD"], capture_output=True, text=True,
                          check=True).stdout.strip()
    now = time.time()
    stamp = time.strftime("%Y%m%dT%H%M%S", time.gmtime(now)) + f"{int(now * 1000) % 1000:03d}Z"   # ms: jobs run at once
    run = f"lap-{stamp}-{head[:8]}"
    execution = f"{name}-{stamp}"
    with tempfile.TemporaryDirectory() as d:
        job = Path(d) / run
        job.mkdir()
        subprocess.run(["git", "-C", str(ROOT), "archive", "--format=tar", "-o", str(job / "tree.tar"), "HEAD",
                        "--", ".", ":!store"], check=True)
        for script in ("run-job.ps1", "profile.ps1"):     # the executor itself comes from the committed tree
            shown = subprocess.run(["git", "-C", str(ROOT), "show", f"HEAD:tools/windows_machine/{script}"],
                                   capture_output=True, check=True)
            (job / script).write_bytes(shown.stdout)
        inputs = {"tree.tar": sha256(job / "tree.tar")}
        if binary:
            (job / "probe.exe").write_bytes((ROOT / "store" / "binaries" / f"{binary}.exe").read_bytes())
            inputs["probe.exe"] = binary
            if dll:
                cfgd = json.loads((ROOT / "probes" / name / "probe.json").read_text())["dll"]["name"]
                (job / f"{cfgd}.dll").write_bytes((ROOT / "store" / "binaries" / f"{dll}.dll").read_bytes())
                inputs[f"{cfgd}.dll"] = dll
        cfg = ROOT / "probes" / name / "probe.json"
        if cfg.is_file() and json.loads(cfg.read_text()).get("generate"):
            commit = (ROOT / "substrate.commit").read_text().strip()
            subprocess.run(["git", "-C", str(ROOT / "substrate"), "archive", "--format=tar",
                            "-o", str(job / "substrate.tar"), commit], check=True)
            inputs["substrate.tar"] = sha256(job / "substrate.tar")
        (job / "job.json").write_text(json.dumps({"kind": "binary" if binary else "probe", "probe": name, "execution_id": execution,
                                                  "repeat": repeat, "commit": head, "inputs": inputs}, indent=1))
        subprocess.run(["scp", "-q", "-r", str(job), f"{HOST}:{JOBS}/"], check=True)
    code = ssh(f"{JOBS}/{run}/run-job.ps1 -Job {JOBS}/{run}")
    dst = ROOT / "build" / "probe-runs" / run / f"traces-{execution}"
    dst.parent.mkdir(parents=True, exist_ok=True)
    fetched = subprocess.run(["scp", "-q", "-r", f"{HOST}:{JOBS}/{run}/out", str(dst)]).returncode
    ssh(f"Remove-Item -Recurse -Force {JOBS}/{run}")
    print(f"{run}: run-job exit {code}, bundle {'at ' + str(dst.relative_to(ROOT)) if fetched == 0 else 'missing'}")
    return code or fetched


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser()
    sub = ap.add_subparsers(dest="kind", required=True)
    p = sub.add_parser("probe")
    p.add_argument("probe")
    p.add_argument("--repeat", type=int, default=2)
    b = sub.add_parser("binary")
    b.add_argument("family")
    b.add_argument("--run", help="the stored run whose binary runs (default: the family's first)")
    b.add_argument("--repeat", type=int, default=2)
    a = ap.parse_args(argv)
    if a.kind == "probe":
        return probe(a.probe, a.repeat)
    sys.path.insert(0, str(ROOT / "scripts"))
    import family_run
    fam = family_run.FAMILIES[a.family]
    chosen = fam.runs[a.run] if a.run else next(iter(fam.runs.values()))
    return probe(fam.probe, a.repeat, chosen.binary, chosen.dll)


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
