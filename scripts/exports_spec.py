# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""exports_spec.py — specs/windows_exports.json from an exports probe run (Sprint 048).

Sprint 044 wrote the spec by hand from run 38039106988 (kernel32, kernelbase). This builds it from a run's rows
(probes/exports): {"m": module, "n": name, "f": forwarded} per export in directory order, and {"system32": name} per
DLL in the system directory. Every module the run dumped goes in; a module the run could not load stops the build.

Usage: uv run python scripts/exports_spec.py <store/probe-runs/run> [--check]
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parent.parent
SPEC = ROOT / "specs" / "windows_exports.json"


def build(run: Path, previous: dict[str, Any]) -> dict[str, Any]:
    rows = [json.loads(line) for line in (run / "body-1.jsonl").read_text().splitlines() if line.strip()]
    missing = [r["m"] for r in rows if r.get("missing")]
    if missing:
        raise SystemExit(f"{run.name}: modules not loaded: {missing}")
    modules: dict[str, list[str]] = {}
    forwarded: dict[str, list[str]] = {}
    for r in rows:
        if "m" in r:
            modules.setdefault(r["m"], []).append(r["n"])
            if r["f"]:
                forwarded.setdefault(r["m"], []).append(r["n"])
    profile = (run / "profile.txt").read_text().strip()
    return {"version": "0.2.0",
            "source": (f"probes/exports on run {run.name} ({profile}); the export directory of each module in memory, "
                       f"in order: kernel32 and kernelbase (Sprint 044, run 38039106988 then) and every other DLL "
                       f"UnityPlayer.dll imports (Sprint 048), built by scripts/exports_spec.py"),
            "forwarded": forwarded, "modules": modules,
            "system32": sorted(r["system32"] for r in rows if "system32" in r) or previous.get("system32", [])}


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("run", type=Path)
    ap.add_argument("--check", action="store_true")
    a = ap.parse_args(argv)
    previous = json.loads(SPEC.read_text())
    spec = build(a.run, previous)
    for m in ("kernel32", "kernelbase"):
        same = spec["modules"][m] == previous["modules"][m]
        print(f"{m}: {len(spec['modules'][m])} names; equal to the spec's: {same}")
    if a.check:
        return 0
    SPEC.write_text(json.dumps(spec, indent=1) + "\n")
    print(f"wrote {SPEC.relative_to(ROOT)}: {', '.join(f'{m} {len(n)}' for m, n in spec['modules'].items())}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
