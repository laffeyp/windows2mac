# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""laptop_calibration.py — calibration step 2 on the laptop (Sprint 067 item C; docs/windows-reference-machine-2026-10-12.md §6).

`run` sends every family's stored runner binaries to the laptop in one batch job (scripts/windows_machine.py) and
judges the result; `judge <batch>` judges a batch already back. A laptop execution is judged as w2m is: by
family_run.unequal_rows, the family's derivation and exclusions applied and its accepted contract's rows compared by
key. So a value the family already treats as machine-decided (a pointer, a handle, an excluded row) does not count,
and every row that differs is one the contract pins. The verdict per binary: `equal` (both executions conform),
`differ` (the rows each execution misses), the two laptop executions' agreement, and a missing body as `failed`.
build/calibration/<batch>.json keeps it all.

`store <batch>` puts the batch's runs on the record: store_probe_runs.py copies each into store/probe-runs/ and,
since the laptop's runs have no Actions record behind them, one session per run goes to store/runs/probe-run-<run>
(EXECUTION_STARTED and EXECUTION_COMPLETED for each execution, profile_kind windows_reference). A dirty tree is
refused.

Usage: uv run python scripts/laptop_calibration.py run [<family> ...]
       uv run python scripts/laptop_calibration.py judge <batch run id>
       uv run python scripts/laptop_calibration.py store <batch run id>
"""

from __future__ import annotations

import hashlib
import json
import sys
from pathlib import Path
from typing import Any

import msgspec

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))
sys.path.insert(0, str(ROOT / "src"))
import family_run  # noqa: E402
import store_probe_runs  # noqa: E402
import windows_machine  # noqa: E402
from w2m import VOCAB_VERSION, events  # noqa: E402
from w2m.contract import ExactRows  # noqa: E402
from w2m.record import one_step_run, uncommitted  # noqa: E402

EMPTY = "sha256:" + hashlib.sha256(b"").hexdigest()


def batch_runs(batch: str) -> list[tuple[Path, dict[str, Any]]]:
    """Each run the batch split into, with its item from the batch's job.json."""
    items = json.loads((ROOT / "build" / "batches" / batch / "job.json").read_text())["items"]
    out = []
    for n, item in enumerate(items, 1):
        found = sorted((ROOT / "build" / "probe-runs" / f"{batch}-{n:03d}").glob("traces-*"))
        out.append((found[0] if found else ROOT / "build" / "probe-runs" / f"{batch}-{n:03d}", item))
    return out


def judge_one(bundle: Path, item: dict[str, Any], repeat: int) -> dict[str, Any]:
    f = family_run.FAMILIES[item["family"]]
    res: dict[str, Any] = {"family": item["family"], "binary_of": item["runs"].split()[0], "run": bundle.parent.name}
    bodies = [bundle / f"body-{i}.jsonl" for i in range(1, repeat + 1)]
    if not all(b.is_file() for b in bodies):
        res["verdict"] = "failed"
        return res
    unequal = [family_run.unequal_rows(f, b.read_bytes()) for b in bodies]
    derived = [family_run.derived_output(f, b.read_bytes()) for b in bodies]
    res["verdict"] = "equal" if not any(unequal) else "differ"
    res["unequal"] = sorted(set().union(*map(set, unequal)))
    res["laptop_executions_agree"] = len(set(derived)) == 1
    if res["unequal"]:
        c = family_run.contract(f)
        assert isinstance(c.acceptance, ExactRows)
        want = {r["s"]: r for r in c.acceptance.rows}
        got = {r.get("s"): r for r in f.rows(family_run.parse(bodies[0].read_bytes()))}
        res["examples"] = [{"row": s, "runner": want.get(s), "laptop": got.get(s)} for s in res["unequal"][:8]]
    return res


def judge(batch: str) -> int:
    repeat = json.loads((ROOT / "build" / "batches" / batch / "job.json").read_text())["repeat"]
    results = [judge_one(b, item, repeat) for b, item in batch_runs(batch)]
    out = ROOT / "build" / "calibration" / f"{batch}.json"
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(json.dumps(results, indent=1, default=str))
    count = {v: sum(r["verdict"] == v for r in results) for v in ("equal", "differ", "failed")}
    families = sorted({r["family"] for r in results})
    print(f"{batch}: {len(results)} binaries over {len(families)} families: {count['equal']} equal, "
          f"{count['differ']} differ, {count['failed']} failed; {out.relative_to(ROOT)}")
    for r in results:
        if r["verdict"] != "equal":
            agree = "" if r.get("laptop_executions_agree", True) else "; the laptop's two executions differ"
            print(f"  {r['verdict']:6} {r['family']} ({r['binary_of']}): {', '.join(r.get('unequal', []))}{agree}")
    return 0


def record(run: str, family: str) -> None:
    """One session for a stored laptop run: each execution's start and end, on the windows_reference profile."""
    src = ROOT / "store" / "probe-runs" / run
    profile = (src / "profile.txt").read_text().split()[0]
    binary = (src / "probe.sha256").read_text().strip()
    executions = sorted(src.glob("exec-*.json"))

    def work() -> list[msgspec.Struct]:
        evs: list[msgspec.Struct] = []
        for e in executions:
            i = e.stem.split("-")[1]
            xid = f"probe-run-{run}-{i}"
            ended = json.loads(e.read_text())
            body = src / f"body-{i}.jsonl"
            if not body.is_file():                   # no rows is not an empty trace (audit 2026-10-13)
                raise SystemExit(f"{run}: {e.name} has no {body.name}; not recorded")
            data = body.read_bytes()
            evs += [
                events.validate(events.EXECUTION_STARTED(
                    execution_id=xid, subject_kind="probe", subject_sha256="sha256:" + binary,
                    profile_kind="windows_reference", profile_id=profile, initial_state_sha256=EMPTY,
                    injected_inputs_sha256=EMPTY,
                    unobserved="initial state and injected inputs: the executor records neither; the machine is in "
                               "profile.json beside the rows")),
                events.validate(events.EXECUTION_COMPLETED(
                    execution_id=xid, exit_status="exited", exit_code=ended["exit_code"],
                    elapsed_seconds=ended["elapsed_seconds"], trace_sha256="sha256:" + hashlib.sha256(data).hexdigest(),
                    guest_line_count=len([ln for ln in data.splitlines() if ln]),
                    unobserved=f"the trace is store/probe-runs/{run}/{body.name}, the probe's printed rows"))]
        return evs

    one_step_run(ROOT / "store" / "runs" / f"probe-run-{run}", profile="windows_reference",
                 contract_version=VOCAB_VERSION, purpose="measurement", work=work,
                 baseline={"probe_run": f"store/probe-runs/{run}", "family": family})


def store(batch: str) -> int:
    changed = uncommitted()
    if changed:
        print("refused: store/runs/ records name the commit that ran; uncommitted: " + ", ".join(changed[:10]))
        return 2
    every = batch_runs(batch)
    runs = [(b.parent.name, item["family"]) for b, item in every if (b / "body-1.jsonl").is_file()]
    skipped = [b.parent.name for b, _ in every if not (b / "body-1.jsonl").is_file()]
    if skipped:
        print(f"{batch}: {len(skipped)} runs have no rows and are not stored: {', '.join(skipped)}")
    code = store_probe_runs.main([r for r, _ in runs])
    if code:
        return code
    for run, family in runs:
        record(run, family)
    print(f"{batch}: {len(runs)} runs stored and recorded")
    return 1 if skipped else 0


def main(argv: list[str]) -> int:
    if not argv or argv[0] not in ("run", "judge", "store"):
        print(__doc__)
        return 2
    if argv[0] == "run":
        families = argv[1:] or sorted(family_run.FAMILIES)
        before = set((ROOT / "build" / "batches").glob("lap-*")) if (ROOT / "build" / "batches").is_dir() else set()
        code = windows_machine.run_batch(windows_machine.batch_items(families), 2)
        new = sorted(set((ROOT / "build" / "batches").glob("lap-*")) - before)
        if not new:
            return code or 1
        judge(new[-1].name)
        return code
    return judge(argv[1]) if argv[0] == "judge" else store(argv[1])


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
