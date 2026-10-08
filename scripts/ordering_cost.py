"""ordering_cost.py — execution time of x86-TSO ordering lowerings on one synthetic kernel, native process and managed VM (Sprint 011).

Builds and signs probes/ordering_cost/ordering_cost.c, checks the native TSO lowering with an MP
litmus (ldapr/stlr must show zero forbidden outcomes), then times 6 variants x 2 working sets,
5 interleaved repeats each, and records the medians as MEASUREMENT_RECORDED on a substrate record.

Usage: uv run python scripts/ordering_cost.py [--repeats 5] [--runs runs]
"""

from __future__ import annotations

import argparse
import asyncio
import json
import statistics
import subprocess
import sys
import time
from collections.abc import AsyncIterator
from pathlib import Path
from typing import Any

import msgspec
from substrate.api import Runtime, TopologyBuilder, read_record, threshold_count

from w2m import VOCAB_VERSION, events

ROOT = Path(__file__).resolve().parent.parent
SRC = ROOT / "probes" / "ordering_cost" / "ordering_cost.c"
ENT = ROOT / "probes" / "mac_hv_capability" / "entitlements.plist"
BIN = ROOT / "build" / "ordering_cost"
VARIANTS = [("native", "plain"), ("native", "ldapr_stlr"), ("native", "ldar_stlr"),
            ("native", "dmb_fences"), ("vm", "entso_off"), ("vm", "entso_on"),
            ("vm", "entso_on_16k")]
SETS = {"l1": (1024, 50_000), "dram": (16 * 1024 * 1024, 2)}  # (elements per array, reps)
APP = "synthetic.ordering_kernel"


def sh(*cmd: str) -> str:
    return subprocess.run(cmd, check=True, capture_output=True, text=True).stdout.strip()


def probe(*args: str) -> dict[str, Any]:
    out: dict[str, Any] = json.loads(sh(str(BIN), *args))
    return out


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--repeats", type=int, default=5)
    ap.add_argument("--mp-iterations", type=int, default=10_000_000)
    ap.add_argument("--runs", type=Path, default=ROOT / "runs")
    a = ap.parse_args(argv)
    BIN.parent.mkdir(exist_ok=True)
    sh("clang", "-O1", "-Wall", "-Werror", "-o", str(BIN), str(SRC), "-framework", "Hypervisor")
    sh("codesign", "-s", "-", "-f", "--entitlements", str(ENT), str(BIN))
    host_id = f"mac-{sh('sysctl', '-n', 'hw.model')}-macos-{sh('sw_vers', '-buildVersion')}"

    # The litmus has power only if plain loads/stores fail at least once in the same attempt.
    mp: dict[str, Any] = {}
    for attempt in range(1, 11):
        mp = probe("mp", str(a.mp_iterations)) | {"attempt": attempt}
        print(f"native mp litmus attempt {attempt}: plain {mp['plain_forbidden']}/"
              f"{mp['iterations']}, ldapr_stlr {mp['ldapr_stlr_forbidden']}/{mp['iterations']}")
        if mp["ldapr_stlr_forbidden"] != 0:
            print("ldapr/stlr lowering admitted a forbidden MP outcome; no timings recorded",
                  file=sys.stderr)
            return 1
        if mp["plain_forbidden"] > 0:
            break
    else:
        print("plain never failed in 10 attempts; the lowering check has no power; "
              "no timings recorded", file=sys.stderr)
        return 1

    samples: dict[tuple[str, str, str], list[float]] = {}
    t0 = time.monotonic()
    for _ in range(a.repeats):
        for set_name, (n, reps) in SETS.items():
            for mode, variant in VARIANTS:
                r = probe(mode, variant, str(n), str(reps))
                samples.setdefault((set_name, mode, variant), []).append(r["ns_per_access"])
    elapsed = time.monotonic() - t0

    measurements = []
    for (set_name, mode, variant), xs in samples.items():
        measurements.append(events.validate(events.MEASUREMENT_RECORDED(
            measurement_id=f"ordering-{set_name}-{variant}", metric="ns_per_access",
            value=round(statistics.median(xs), 4), unit="ns", phase="sustained",
            backend="native_process" if mode == "native" else "managed_vm",
            application_profile_id=APP, artifact_state="warm", destination_profile_id=host_id,
            variant=f"{variant}/{set_name}")))

    async def harness(_inp: Any) -> AsyncIterator[msgspec.Struct]:
        yield events.validate(events.SESSION_INIT(
            vocab_version=VOCAB_VERSION, profile=host_id, contract_version="0.0.0",
            harness_version="0.0.1", purpose="measurement"))
        for m in measurements:
            yield m
        yield events.validate(events.SESSION_COMPLETED(
            outcome="completed", discrepancy_count=0, accepted_count=0,
            elapsed_seconds=round(elapsed, 3)))

    def topology(b: TopologyBuilder) -> None:
        b.baseline(host_id=host_id, probe="probes/ordering_cost/ordering_cost.c",
                   repeats=a.repeats, sets={k: list(v) for k, v in SETS.items()},
                   mp_check=mp)
        b.producer_kind("destination_evaluator", schemas=list(events.HARNESS_EVENTS),
                        schema_version=1, factory=lambda: harness)
        b.initial("destination_evaluator", input=None)
        b.termination(threshold_count("substrate.ProducerCompleted", 1))

    result = asyncio.run(Runtime(a.runs / f"ordering-{int(time.time())}").run(topology))
    rec = {e["payload"]["variant"]: e["payload"]["value"]
           for e in read_record(result.record_root) if e["kind"] == "MEASUREMENT_RECORDED"}
    for set_name in SETS:
        base = rec[f"plain/{set_name}"]
        print(f"-- {set_name}")
        for mode, variant in VARIANTS:
            v = rec[f"{variant}/{set_name}"]
            spread = samples[(set_name, mode, variant)]
            print(f"  {mode:6} {variant:11} {v:7.4f} ns/access  x{v / base:4.2f}  "
                  f"(min {min(spread):.4f} max {max(spread):.4f})")
    print(f"record {result.record_root}; {elapsed:.1f} s of timing")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
