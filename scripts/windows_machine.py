# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
r"""windows_machine.py — run a probe on the Windows reference machine, the laptop (Sprint 067 item B).

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

A `batch` job runs many stored binaries in one job (calibration): one profile, one copy each way, one workspace
lock, a fresh workspace per binary. Its bundle is split into one run per binary, lap-<stamp>-<commit>-<n>, each with
a batch.txt naming the batch; the batch's profile.json, run-job.log and job.json stay once, in build/batches/<batch>/.

Usage: uv run python scripts/windows_machine.py probe <probe> [--repeat 2] [--pin performance|efficiency]
       uv run python scripts/windows_machine.py binary <family> [--run <stored run id>] [--repeat 2] [--pin ...]
       uv run python scripts/windows_machine.py batch [<family> ...] [--repeat 2] [--pin ...]   (no family: all)
       uv run python scripts/windows_machine.py game [--executions 10] [--seconds 180] [--mode batch|play] [--audio on|off]

A `game` job runs the game as windows-game.yml does, `executions` times in one job (the game tree is the laptop's
copy at C:\w2m\games\<game>, sent once); each execution comes back as build/game-runs/<run>-<i>/, the layout
store/game-runs/ holds.
"""

from __future__ import annotations

import argparse
import fcntl
import hashlib
import json
import shutil
import subprocess
import sys
import tempfile
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
HOST = "w2m-laptop"
JOBS = "C:/w2m/jobs"
EXECUTOR = ("run-job.ps1", "profile.ps1", "cpusets.ps1", "run-in-desktop.ps1", "input.cs", "pmc.wprp", "pmc_summary.py")
# A dead connection ends in a minute instead of hanging (2026-10-13: one scp of a finished bundle stalled 6 minutes;
# the same copy by hand took 0.9 s).
LINK = ["-o", "ConnectTimeout=15", "-o", "ServerAliveInterval=15", "-o", "ServerAliveCountMax=4"]
JOB_SECONDS = 2 * 3600        # a job's ssh ends after this even if the link stays alive (audit 2026-10-13)


def ssh_job(command: str) -> int:
    """Run a job's command on the laptop; 124 if it outlives JOB_SECONDS."""
    try:
        return subprocess.run(["ssh", *LINK, HOST, command], timeout=JOB_SECONDS).returncode
    except subprocess.TimeoutExpired:
        return 124


def copy(*args: str, attempts: int = 3) -> bool:
    """scp with the link's timeouts, tried up to `attempts` times."""
    return any(subprocess.run(["scp", "-q", *LINK, *args]).returncode == 0 for _ in range(attempts))


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def git(*args: str) -> bytes:
    return subprocess.run(["git", "-C", str(ROOT), *args], capture_output=True, check=True).stdout


def verdict(code: int, fetched: bool, bundle: Path, repeat: int) -> tuple[int, str]:
    """The run's exit status and its line: a missing bundle, then run-job's code, then a short count, then an
    execution that exited non-zero or wrote no rows (audit 2026-10-13: a crashed second execution counted as one)."""
    if not fetched:
        return code or 1, "bundle not copied back; the laptop's job folder is kept"
    execs = sorted(bundle.glob("exec-*.json"))
    n = len(execs)
    if code:
        return code, f"run-job exit {code}, {n} of {repeat} executions"
    if n != repeat:
        return 1, f"{n} of {repeat} executions"
    failed = [e.stem for e in execs if json.loads(e.read_text()).get("exit_code") != 0]
    if failed:
        return 1, f"{n} of {repeat} executions; non-zero exit: {', '.join(failed)}"
    return 0, f"{n} of {repeat} executions"


def run_job(name: str, repeat: int, binary: str | None = None, dll: str | None = None, session: str = "desktop",
            pin: str | None = None) -> int:
    if not (ROOT / "probes" / name).is_dir():
        print(f"no probes/{name}", file=sys.stderr)
        return 1
    (ROOT / "build").mkdir(exist_ok=True)
    with (ROOT / "build" / "windows_machine.lock").open("w") as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)              # one job at a time on the laptop's one workspace
        head = git("rev-parse", "HEAD").decode().strip()   # read under the lock: a queued job names what it ships
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
            (job / "tree.tar").write_bytes(git("archive", "--format=tar", head, "--", *paths))
            for script in EXECUTOR:                   # the executor itself comes from the committed tree
                (job / script).write_bytes(git("show", f"{head}:tools/windows_machine/{script}"))
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
            if not copy("-r", str(job), f"{HOST}:{JOBS}/"):
                raise RuntimeError(f"{run}: the job could not be copied to the laptop")
            runner = "run-in-desktop.ps1" if session == "desktop" else "run-job.ps1"   # desktop: as the runners
            code = ssh_job(f"{JOBS}/{run}/{runner} -Job {JOBS}/{run}")
            dst.parent.mkdir(parents=True, exist_ok=True)
            fetched = copy("-r", f"{HOST}:{JOBS}/{run}/out", str(dst))
            if fetched:
                (dst / "job.json").write_text((job / "job.json").read_text())
                subprocess.run(["ssh", *LINK, HOST, f"Remove-Item -Recurse -Force {JOBS}/{run}"])
    status, line = verdict(code, fetched, dst, repeat)
    print(f"{run}: {line}; bundle {dst.relative_to(ROOT)}")
    return status


def batch_items(families: list[str]) -> list[dict[str, str]]:
    """One item per distinct stored binary of each family, in family order: its probe, binary, DLL and the stored
    runs that used it."""
    sys.path.insert(0, str(ROOT / "scripts"))
    import family_run
    items: list[dict[str, str]] = []
    for name in families:
        fam = family_run.FAMILIES[name]
        seen: dict[str, dict[str, str]] = {}
        for rid, r in fam.runs.items():
            if r.binary in seen:
                seen[r.binary]["runs"] += f" {rid}"
                continue
            item = {"family": name, "probe": fam.probe, "binary": r.binary, "runs": rid}
            if r.dll:
                item["dll"] = r.dll
                item["dll_name"] = json.loads((ROOT / "probes" / fam.probe / "probe.json").read_text())["dll"]["name"]
            seen[r.binary] = item
        items.extend(seen.values())
    return items


def run_batch(items: list[dict[str, str]], repeat: int, pin: str | None = None, window: str = "hidden") -> int:
    (ROOT / "build").mkdir(exist_ok=True)
    with (ROOT / "build" / "windows_machine.lock").open("w") as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        head = git("rev-parse", "HEAD").decode().strip()   # read under the lock: a queued job names what it ships
        now = time.time()
        stamp = time.strftime("%Y%m%dT%H%M%S", time.gmtime(now)) + f"{int(now * 1000) % 1000:03d}Z"
        run = f"lap-{stamp}-{head[:8]}"
        for n, item in enumerate(items, 1):
            item["execution_id"] = f"{item['family']}-{stamp}-{n:03d}"
        spec: dict[str, object] = {"kind": "batch", "execution_id": run, "repeat": repeat, "commit": head,
                                   "session": "desktop", "console_window": window, "items": items}
        if pin:
            spec["cpu_class"] = pin
        with tempfile.TemporaryDirectory() as d:
            job = Path(d) / run
            (job / "bin").mkdir(parents=True)
            (job / "tree.tar").write_bytes(git("archive", "--format=tar", head, "--", "probes"))
            for script in EXECUTOR:
                (job / script).write_bytes(git("show", f"{head}:tools/windows_machine/{script}"))
            inputs = {"tree.tar": sha256(job / "tree.tar")}
            for item in items:
                for kind in ("exe", "dll"):
                    sha = item.get("binary" if kind == "exe" else "dll")
                    if sha and f"bin/{sha}.{kind}" not in inputs:
                        (job / "bin" / f"{sha}.{kind}").write_bytes((ROOT / "store" / "binaries" / f"{sha}.{kind}").read_bytes())
                        inputs[f"bin/{sha}.{kind}"] = sha
            spec["inputs"] = inputs
            (job / "job.json").write_text(json.dumps(spec, indent=1))
            if not copy("-r", str(job), f"{HOST}:{JOBS}/"):
                raise RuntimeError(f"{run}: the job could not be copied to the laptop")
            code = ssh_job(f"{JOBS}/{run}/run-in-desktop.ps1 -Job {JOBS}/{run}")
            bundle = ROOT / "build" / "batches" / run
            bundle.parent.mkdir(parents=True, exist_ok=True)
            fetched = copy("-r", f"{HOST}:{JOBS}/{run}/out", str(bundle))
            if fetched:
                (bundle / "job.json").write_text((job / "job.json").read_text())
                subprocess.run(["ssh", *LINK, HOST, f"Remove-Item -Recurse -Force {JOBS}/{run}"])
    if not fetched:
        print(f"{run}: bundle not copied back; the laptop's job folder is kept")
        return code or 1
    short = 0
    for n, item in enumerate(items, 1):
        dst = ROOT / "build" / "probe-runs" / f"{run}-{n:03d}" / f"traces-{item['execution_id']}"
        dst.parent.mkdir(parents=True, exist_ok=True)
        src = bundle / item["execution_id"]
        if src.is_dir():
            shutil.copytree(src, dst, dirs_exist_ok=True)
        dst.mkdir(parents=True, exist_ok=True)
        (dst / "batch.txt").write_text(run)          # its profile.json, run-job.log and job.json: the batch's
        status, line = verdict(0, True, dst, repeat)
        short += status != 0
        if status:
            print(f"{run}-{n:03d} {item['family']} ({item['runs'].split()[0]}): {line}")
    print(f"{run}: {len(items)} binaries, {len(items) - short} with {repeat} executions; run-job exit {code}; "
          f"bundle {bundle.relative_to(ROOT)}")
    return code or (1 if short else 0)


def run_game(game: str, exe: str, executions: int, seconds: int, mode: str, audio: str = "on",
             inputs: str | None = None, record: bool = False, pmc: bool = False, capture: str = "on") -> int:
    (ROOT / "build").mkdir(exist_ok=True)
    with (ROOT / "build" / "windows_machine.lock").open("w") as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        head = git("rev-parse", "HEAD").decode().strip()   # read under the lock: a queued job names what it ships
        now = time.time()
        stamp = time.strftime("%Y%m%dT%H%M%S", time.gmtime(now)) + f"{int(now * 1000) % 1000:03d}Z"
        run = f"lap-{stamp}-{head[:8]}"
        # each execution: the tree's mirror and check (about 1 minute), the game's seconds, and its stop
        limit = executions * (seconds + 180) // 60 + 10
        spec: dict[str, object] = {"kind": "game", "execution_id": run, "game": game, "exe": exe, "seconds": seconds,
                                   "mode": mode, "audio": audio, "input_script": inputs, "record": record,
                                   "pmc": pmc, "capture": capture,
                                   "executions": executions, "repeat": 0, "commit": head,
                                   "session": "desktop", "limit_minutes": limit}
        with tempfile.TemporaryDirectory() as d:
            job = Path(d) / run
            job.mkdir()
            (job / "tree.tar").write_bytes(git("archive", "--format=tar", head, "--", "tools/jit_capture",
                                               f"games/{game}"))
            for script in EXECUTOR:
                (job / script).write_bytes(git("show", f"{head}:tools/windows_machine/{script}"))
            spec["inputs"] = {"tree.tar": sha256(job / "tree.tar")}
            (job / "job.json").write_text(json.dumps(spec, indent=1))
            if not copy("-r", str(job), f"{HOST}:{JOBS}/"):
                raise RuntimeError(f"{run}: the job could not be copied to the laptop")
            try:
                code = subprocess.run(["ssh", *LINK, HOST, f"{JOBS}/{run}/run-in-desktop.ps1 -Job {JOBS}/{run}"],
                                      timeout=limit * 60 + 600).returncode
            except subprocess.TimeoutExpired:
                code = 124
            bundle = ROOT / "build" / "batches" / run
            bundle.parent.mkdir(parents=True, exist_ok=True)
            fetched = copy("-r", f"{HOST}:{JOBS}/{run}/out", str(bundle))
            if fetched:
                (bundle / "job.json").write_text((job / "job.json").read_text())
                subprocess.run(["ssh", *LINK, HOST, f"Remove-Item -Recurse -Force {JOBS}/{run}"])
    if not fetched:
        print(f"{run}: bundle not copied back; the laptop's job folder is kept")
        return code or 1
    done = 0
    for i in range(1, executions + 1):
        src = bundle / str(i)
        if not (src / "exec.json").is_file():
            print(f"{run}-{i:02d}: no exec.json")
            continue
        dst = ROOT / "build" / "game-runs" / f"{run}-{i:02d}"
        shutil.copytree(src, dst, dirs_exist_ok=True)
        (dst / "batch.txt").write_text(run)
        if (bundle / "profile.json").is_file():          # the machine the game asked (its processor count): per run
            shutil.copyfile(bundle / "profile.json", dst / "profile.json")
        done += 1
    print(f"{run}: {done} of {executions} executions back; run-job exit {code}; bundle {bundle.relative_to(ROOT)}")
    return code or (0 if done == executions else 1)


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser()
    sub = ap.add_subparsers(dest="kind", required=True)
    gm = sub.add_parser("game")
    gm.add_argument("--game", default="hail-to-the-rainbow")
    gm.add_argument("--exe", default="Hail to the Rainbow")
    gm.add_argument("--executions", type=int, default=10)
    gm.add_argument("--seconds", type=int, default=180)
    gm.add_argument("--mode", choices=("batch", "play"), default="batch")
    gm.add_argument("--input", help="an input script in games/<game>/inputs/<name>.jsonl, played with SendInput")
    gm.add_argument("--record", action="store_true", help="keep the raw keyboard and mouse as inputs.jsonl")
    gm.add_argument("--pmc", action="store_true",
                    help="instructions per thread and DXGI Presents, per second (pmc.wprp; Sprint 068 item I)")
    gm.add_argument("--capture", choices=("on", "off"), default="on",
                    help="off: the game's own exe without the capture's launcher and hooks")
    gm.add_argument("--audio", choices=("on", "off"), default="on",
                    help="off: the audio endpoints disabled for the job, as the runners have none")
    bt = sub.add_parser("batch")
    bt.add_argument("families", nargs="*")
    bt.add_argument("--repeat", type=int, default=2)
    bt.add_argument("--pin", choices=("performance", "efficiency"), help="run on one core type only (G1)")
    bt.add_argument("--window", choices=("hidden", "minimized", "normal"), default="hidden",
                    help="the job's console window in the desktop session")
    for kind in ("probe", "binary"):
        p = sub.add_parser(kind)
        p.add_argument("probe" if kind == "probe" else "family")
        if kind == "binary":
            p.add_argument("--run", help="the stored run whose binary runs (default: the family's first)")
        p.add_argument("--repeat", type=int, default=2)
        p.add_argument("--session", choices=("desktop", "ssh"), default="desktop")
        p.add_argument("--pin", choices=("performance", "efficiency"), help="run on one core type only (G1)")
    a = ap.parse_args(argv)
    if a.kind == "game":
        return run_game(a.game, a.exe, a.executions, a.seconds, a.mode, a.audio, a.input, a.record, a.pmc, a.capture)
    if a.kind == "batch":
        sys.path.insert(0, str(ROOT / "scripts"))
        import family_run
        return run_batch(batch_items(a.families or sorted(family_run.FAMILIES)), a.repeat, a.pin, a.window)
    if a.kind == "probe":
        return run_job(a.probe, a.repeat, session=a.session, pin=a.pin)
    sys.path.insert(0, str(ROOT / "scripts"))
    import family_run
    fam = family_run.FAMILIES[a.family]
    chosen = fam.runs[a.run] if a.run else next(iter(fam.runs.values()))
    return run_job(fam.probe, a.repeat, chosen.binary, chosen.dll, a.session, a.pin)


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
