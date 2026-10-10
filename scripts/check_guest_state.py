"""check_guest_state.py — judge specs/guest_state.json against probes/guest_state runs (Sprint 034).

Three comparisons, each field by field:
  layout   — every CONTEXT, XMM_SAVE_AREA32, EXCEPTION_RECORD, TEB and PEB offset the spec states
             from a header equals the runner's offsetof/sizeof, in every run;
  observed — every TEB/PEB field the spec takes from observation names a check that was equal in
             every run, at the offset the spec states;
  defaults — MXCSR, the x87 control word and IF at thread start equal the runner's fault CONTEXT.
Also: the layout lines are identical across runs, and every run's summary has 0 failures.

--record writes one causal differential run (src/w2m/differential.py, Sprint 043) over the units
runs, CONTEXT, XMM_SAVE_AREA32, EXCEPTION_RECORD, TEB, PEB and defaults: each unit's comparison
causes its acceptance or its report (routed contract_revision: Windows is the reference, the spec
states the wrong value), and an acceptance resolves what an earlier run left open on its unit.

Usage: uv run python scripts/check_guest_state.py store/probe-runs/<run id> [--record] [--runs runs]
"""

from __future__ import annotations

import argparse
import hashlib
import json
import sys
from pathlib import Path
from typing import Any

from w2m import differential
from w2m.differential import Verdict

ROOT = Path(__file__).resolve().parent.parent
SPEC = ROOT / "specs" / "guest_state.json"


def runs_of(d: Path) -> list[list[dict[str, Any]]]:
    return [[json.loads(line) for line in p.read_text().splitlines() if line.strip()]
            for p in sorted(d.glob("body-*.jsonl"))]


UNITS = ("runs", "CONTEXT", "XMM_SAVE_AREA32", "EXCEPTION_RECORD", "TEB", "PEB", "defaults")


def compare_units(spec: dict[str, Any], runs: list[list[dict[str, Any]]]) -> dict[str, tuple[int, list[str]]]:
    """Per unit (UNITS): fields compared, and one line per difference."""
    out: dict[str, tuple[int, list[str]]] = {u: (0, []) for u in UNITS}

    def diff(unit: str, line: str) -> None:
        out[unit][1].append(line)

    def count(unit: str) -> None:
        out[unit] = (out[unit][0] + 1, out[unit][1])
    layouts = [{r["name"]: r["value"] for r in run if r["kind"] == "layout"} for run in runs]
    if any(lay != layouts[0] for lay in layouts[1:]):
        diff("runs", "layout lines differ between runs")
    checks = [{r["name"]: r for r in run if r["kind"] == "check"} for run in runs]
    for i, run in enumerate(runs, 1):
        summary = [r for r in run if r["kind"] == "summary"]
        if not summary or summary[0]["failures"] != 0:
            diff("runs", f"run {i}: summary {summary[0] if summary else 'missing'}")
    w = spec["windows"]
    for struct in ("CONTEXT", "XMM_SAVE_AREA32", "EXCEPTION_RECORD"):
        for field, want in w[struct]["fields"].items():
            count(struct)
            got = layouts[0].get(f"{struct}.{field}")
            if got != want:
                diff(struct, f"{struct}.{field}: spec {want:#x}, runner {got if got is None else hex(got)}")
    for struct in ("TEB", "PEB"):
        for f in w[struct]["fields"]:
            count(struct)
            name = f["name"]
            if f["source"] == "winternl.h":
                got = layouts[0].get(f"{struct}.{name}")
                if got != f["offset"]:
                    diff(struct, f"{struct}.{name}: spec {f['offset']:#x}, runner {got if got is None else hex(got)}")
            if "check" in f:
                for i, c in enumerate(checks, 1):
                    r = c.get(f["check"])
                    if r is None or not r["equal"]:
                        diff(struct, f"{struct}.{name}: run {i} check {f['check']} {r}")
                    elif r["offset"] >= 0 and r["offset"] != f["offset"] and f["check"] != "teb.tls_slot":
                        diff(struct, f"{struct}.{name}: spec offset {f['offset']:#x}, check read {r['offset']:#x}")
                    elif f["check"] == "teb.tls_slot" and not 0x1480 <= r["offset"] < 0x1480 + 512:
                        diff(struct, f"{struct}.{name}: slot read at {r['offset']:#x}, outside TlsSlots")
            elif f["source"] == "observation":
                diff(struct, f"{struct}.{name}: source observation but no check named")
    d = w["defaults_at_thread_start"]
    for key, check, want in (("mxcsr", "fault.mxcsr", d["mxcsr"]),
                             ("x87_control_word", "fault.x87_control_word", d["x87_control_word"]),
                             ("rflags_if", "fault.eflags_if", d["rflags_if"])):
        count("defaults")
        for i, c in enumerate(checks, 1):
            r = c.get(check)
            if r is None or r["observed"] != want:
                diff("defaults", f"defaults.{key}: spec {want:#x}, run {i} {r and hex(r['observed'])}")
    return out


def compare(spec: dict[str, Any], runs: list[list[dict[str, Any]]]) -> tuple[int, list[str]]:
    """Number of fields compared, and one line per difference (all units)."""
    units = compare_units(spec, runs)
    return sum(n for n, _ in units.values()), [line for u in UNITS for line in units[u][1]]


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("run_dir", type=Path)
    ap.add_argument("--record", action="store_true")
    ap.add_argument("--runs", type=Path, default=Path("runs"))
    a = ap.parse_args(argv)
    spec = json.loads(SPEC.read_text())
    runs = runs_of(a.run_dir)
    profile = (a.run_dir / "profile.txt").read_text().strip()
    n, diffs = compare(spec, runs)
    print(f"spec {spec['version']}; runs {len(runs)} on {profile}; fields compared {n}; differences {len(diffs)}")
    for d in diffs:
        print(f"  {d}")
    if a.record:
        print(json.dumps(record(a, spec, runs, profile), indent=1))
    return 1 if diffs else 0


def record(a: argparse.Namespace, spec: dict[str, Any], runs: list[list[dict[str, Any]]],
           profile: str, spec_text: bytes | None = None) -> dict[str, Any]:
    """One causal differential run (Sprint 043): each unit's candidate is the spec as it states
    that unit; Windows on the runner is the reference, so a difference routes to contract_revision."""
    units = compare_units(spec, runs)
    version = spec["version"]

    def judge(unit: str) -> Verdict:
        n, lines = units[unit]
        rule = f"specs/guest_state.json {version}: {n} fields of {unit} against {a.run_dir}"
        if not lines:
            return Verdict("conforms", rule_ref=rule)
        return Verdict("discrepancy", tuple(lines), rule, "contract_revision",
                       "Windows on the runner is the reference; the spec states the wrong value")

    raw = spec_text if spec_text is not None else SPEC.read_bytes()
    return differential.run(differential.Differential(
        name="guest-state", profile=profile, purpose="conformance",
        contract_id=lambda unit: "windows.guest_state.layout", contract_version=version, relation="exact",
        implementation=f"specs/guest_state.json {version}",
        implementation_sha256="sha256:" + hashlib.sha256(raw).hexdigest(),
        generator_ref="hand (agent-authored)", reference_ref=a.run_dir.name, target_execution_id=a.run_dir.name,
        units=UNITS, compare=judge, baseline={"run_dir": str(a.run_dir)}), a.runs)


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
