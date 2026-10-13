# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""windows_machine.py — run a probe on the Windows reference machine, the laptop (Sprint 067 item B).

The controller's half of docs/windows-reference-machine.md §3. A probe job carries the committed tree
(`git archive HEAD`, store/ left out) and, for a probe with a `generate` step, substrate at substrate.commit; a
`binary` job runs a stored binary (store/binaries/<sha256>.exe) unchanged in place of the build (calibration, §6
step 2) and carries only its probe's folder. Each input is named by sha256 in job.json. The job goes to
C:\\w2m\\jobs\\<run>\\ over SSH (host `w2m-laptop` in ~/.ssh/config) with the committed executor scripts, and
run-job.ps1 runs inside w2m's signed-in desktop session (run-in-desktop.ps1; `--session ssh` runs it in SSH's
session 0 instead). The out\\ folder, with run-job.log in it and job.json beside it, comes back to
build/probe-runs/<run>/traces-<execution id>/, the layout scripts/store_probe_runs.py reads.

One job at a time: the laptop has one workspace (D:\\a\\windows2mac\\windows2mac), so the controller holds
build/windows_machine.lock for the whole job. The laptop's job folder is deleted only once the bundle is back; a run
whose executions number fewer than `repeat` fails.

Usage: uv run python scripts/windows_machine.py probe <probe> [--repeat 2] [--pin performance|efficiency]
       uv run python scripts/windows_machine.py binary <family> [--run <stored run id>] [--repeat 2] [--pin ...]
"""

from __future__ import annotations

import argparse
import fcntl
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
EXECUTOR = ("run-job.ps1", "profile.ps1", "cpusets.ps1", "run-in-desktop.ps1")


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def git(*args: str) -> bytes:
    return subprocess.run(["git", "-C", str(ROOT), *args], capture_output=True, check=True).stdout


def verdict(code: int, fetched: bool, bundle: Path, repeat: int) -> tuple[int, str]:
    """The run's exit status and its line: a missing bundle, then run-job's code, then a short count."""
    if not fetched:
        return code or 1, "bundle not copied back; the laptop's job folder is kept"
    n = len(list(bundle.glob("exec-*.json")))
    if code:
        return code, f"run-job exit {code}, {n} of {repeat} executions"
    return (0 if n == repeat else 1), f"{n} of {repeat} executions"


def run_job(name: str, repeat: int, binary: str | None = None, dll: str | None = None, session: str = "desktop",
            pin: str | None = None) -> int:
    if not (ROOT / "probes" / name).is_dir():
        print(f"no probes/{name}", file=sys.stderr)
        return 1
    head = git("rev-parse", "HEAD").decode().strip()
    (ROOT / "build").mkdir(exist_ok=True)
    with (ROOT / "build" / "windows_machine.lock").open("w") as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)              # one job at a time on the laptop's one workspace
        now = time.time()
        stamp = time.strftime("%Y%m%dT%H%M%S", time.gmtime(now)) + f"{int(now * 1000) % 1000:03d}Z"
        run = f"lap-{stamp}-{head[:8]}"
        execution = f"{name}-{stamp}"
        spec: dict[str, object] = {"kind": "binary" if binary else "probe", "probe": name, "execution_id": execution,
                                   "repeat": repeat, "commit": head, "session": session}
        if pin:
            spec["cpu_class"] = pin
        cfg = ROOT / "probes" / name / "probe.json"
        config = json.loads(cfg.read_text()) if cfg.is_file() else {}
        dst = ROOT / "build" / "probe-runs" / run / f"traces-{execution}"
        with tempfile.TemporaryDirectory() as d:
            job = Path(d) / run
            job.mkdir()
            paths = [f"probes/{name}"] if binary else [".", ":!store"]
            (job / "tree.tar").write_bytes(git("archive", "--format=tar", "HEAD", "--", *paths))
            for script in EXECUTOR:                   # the executor itself comes from the committed tree
                (job / script).write_bytes(git("show", f"HEAD:tools/windows_machine/{script}"))
            inputs = {"tree.tar": sha256(job / "tree.tar")}
            if binary:
                (job / "probe.exe").write_bytes((ROOT / "store" / "binaries" / f"{binary}.exe").read_bytes())
                inputs["probe.exe"] = binary
                if dll:
                    dll_name = f"{config['dll']['name']}.dll"
                    (job / dll_name).write_bytes((ROOT / "store" / "binaries" / f"{dll}.dll").read_bytes())
                    inputs[dll_name] = dll
            elif config.get("generate"):
                commit = (ROOT / "substrate.commit").read_text().strip()
                subprocess.run(["git", "-C", str(ROOT / "substrate"), "archive", "--format=tar",
                                "-o", str(job / "substrate.tar"), commit], check=True)
                inputs["substrate.tar"] = sha256(job / "substrate.tar")
            spec["inputs"] = inputs
            (job / "job.json").write_text(json.dumps(spec, indent=1))
            subprocess.run(["scp", "-q", "-r", str(job), f"{HOST}:{JOBS}/"], check=True)
            runner = "run-in-desktop.ps1" if session == "desktop" else "run-job.ps1"   # desktop: as the runners
            code = subprocess.run(["ssh", HOST, f"{JOBS}/{run}/{runner} -Job {JOBS}/{run}"]).returncode
            dst.parent.mkdir(parents=True, exist_ok=True)
            fetched = subprocess.run(["scp", "-q", "-r", f"{HOST}:{JOBS}/{run}/out", str(dst)]).returncode == 0
            if fetched:
                (dst / "job.json").write_text((job / "job.json").read_text())
                subprocess.run(["ssh", HOST, f"Remove-Item -Recurse -Force {JOBS}/{run}"])
    status, line = verdict(code, fetched, dst, repeat)
    print(f"{run}: {line}; bundle {dst.relative_to(ROOT)}")
    return status


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser()
    sub = ap.add_subparsers(dest="kind", required=True)
    for kind in ("probe", "binary"):
        p = sub.add_parser(kind)
        p.add_argument("probe" if kind == "probe" else "family")
        if kind == "binary":
            p.add_argument("--run", help="the stored run whose binary runs (default: the family's first)")
        p.add_argument("--repeat", type=int, default=2)
        p.add_argument("--session", choices=("desktop", "ssh"), default="desktop")
        p.add_argument("--pin", choices=("performance", "efficiency"), help="run on one core type only (G1)")
    a = ap.parse_args(argv)
    if a.kind == "probe":
        return run_job(a.probe, a.repeat, session=a.session, pin=a.pin)
    sys.path.insert(0, str(ROOT / "scripts"))
    import family_run
    fam = family_run.FAMILIES[a.family]
    chosen = fam.runs[a.run] if a.run else next(iter(fam.runs.values()))
    return run_job(fam.probe, a.repeat, chosen.binary, chosen.dll, a.session, a.pin)


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
