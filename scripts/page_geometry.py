"""page_geometry.py — 4 KB guest page protection on this Mac: correctness and execution time of each enforcement variant (Sprint 012).

Builds and signs probes/page_geometry/page_geometry.c; runs the native and VM correctness checks
for the v0.2 section 8 example (page A read/write, page A+4096 inaccessible); then times
native plain / check_each / check_page_cross and VM pages_4k / blocks_2m on L1- and DRAM-sized
working sets, 5 interleaved repeats each, and records CAPABILITY_PROBED and MEASUREMENT_RECORDED
on a substrate record.

Usage: uv run python scripts/page_geometry.py [--repeats 5] [--runs runs]
"""

from __future__ import annotations

import argparse
import json
import statistics
import subprocess
import sys
import time
from collections.abc import AsyncIterator
from pathlib import Path
from typing import Any

import msgspec
from substrate.api import TopologyBuilder, read_record, threshold_count

from w2m import VOCAB_VERSION, events
from w2m.record import producer, run_with_provenance

ROOT = Path(__file__).resolve().parent.parent
SRC = ROOT / "probes" / "page_geometry" / "page_geometry.c"
ENT = ROOT / "probes" / "mac_hv_capability" / "entitlements.plist"
BIN = ROOT / "build" / "page_geometry"
VARIANTS = [("native", "plain"), ("native", "check_each"), ("native", "check_page_cross"),
            ("vm", "pages_4k"), ("vm", "blocks_2m")]
SETS = {"l1": (1024, 50_000), "dram": (16 * 1024 * 1024, 2)}
APP = "synthetic.page_check_kernel"


def sh(*cmd: str) -> str:
    return subprocess.run(cmd, check=True, capture_output=True, text=True).stdout.strip()


def probe(*args: str) -> dict[str, Any]:
    out: dict[str, Any] = json.loads(sh(str(BIN), *args))
    return out


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--repeats", type=int, default=5)
    ap.add_argument("--runs", type=Path, default=ROOT / "runs")
    a = ap.parse_args(argv)
    BIN.parent.mkdir(exist_ok=True)
    sh("clang", "-O1", "-Wall", "-Werror", "-o", str(BIN), str(SRC), "-framework", "Hypervisor")
    sh("codesign", "-s", "-", "-f", "--entitlements", str(ENT), str(BIN))
    host_id = f"mac-{sh('sysctl', '-n', 'hw.model')}-macos-{sh('sw_vers', '-buildVersion')}"

    nc, vc = probe("native_correctness"), probe("vm_correctness")
    mprotect_refused = nc["mprotect_4k_rc"] == -1 and nc["mprotect_4k_errno"] == 22
    check_ok = (nc["check_each_page0_fault"] == 0 and nc["check_each_page1_fault_offset"] == 4096
                and nc["check_page_cross_page1_fault_offset"] == 4096)
    guest_ok = (vc["hvc_imm"] == 1 and vc["page0_value"] == "0x11223344"
                and vc["guest_esr_ec"] == "0x25" and vc["guest_far"] == "0x20001000"
                and vc["guest_elr_offset"] == 4)

    def cap(name: events.EnumCapability, present: bool, detail: str) -> events.CAPABILITY_PROBED:
        return events.validate(events.CAPABILITY_PROBED(
            capability=name, host_id=host_id, outcome="present" if present else "absent",
            detail=detail))

    caps = [
        cap("native_mprotect_4k", not mprotect_refused,
            f"mprotect(A+4096, 4096, PROT_NONE) on a {nc['host_page']}-byte host page: rc "
            f"{nc['mprotect_4k_rc']} errno {nc['mprotect_4k_errno']} (EINVAL)"),
        cap("native_page_check_4k", check_ok,
            f"software shadow check (1 byte per 4 KB page): page A no fault; read of A+4096 "
            f"faulted at offset {nc['check_each_page1_fault_offset']} (check_each) and "
            f"{nc['check_page_cross_page1_fault_offset']} (check_page_cross)"),
        cap("guest_page_protection_4k", guest_ok,
            f"guest 4 KB stage-1 pages, page A+4096 invalid: read of A returned "
            f"{vc['page0_value']}; read of A+4096 raised guest data abort ec {vc['guest_esr_ec']} "
            f"far {vc['guest_far']} at code offset {vc['guest_elr_offset']}, delivered through "
            f"the guest vector table (hvc #{vc['hvc_imm']})"),
    ]
    if not (mprotect_refused and check_ok and guest_ok):
        print(f"correctness precondition failed: {nc} {vc}", file=sys.stderr)
        return 1

    samples: dict[tuple[str, str, str], list[float]] = {}
    t0 = time.monotonic()
    for _ in range(a.repeats):
        for set_name, (n, reps) in SETS.items():
            for mode, variant in VARIANTS:
                r = probe(mode, variant, str(n), str(reps))
                samples.setdefault((set_name, mode, variant), []).append(r["ns_per_access"])
    elapsed = time.monotonic() - t0
    measurements = [events.validate(events.MEASUREMENT_RECORDED(
        measurement_id=f"pagegeom-{s}-{v}", metric="ns_per_access",
        value=round(statistics.median(xs), 4), unit="ns", phase="sustained",
        backend="native_process" if m == "native" else "managed_vm",
        application_profile_id=APP, artifact_state="warm", destination_profile_id=host_id,
        variant=f"{v}/{s}")) for (s, m, v), xs in samples.items()]

    async def harness(_inp: Any) -> AsyncIterator[msgspec.Struct]:
        yield events.validate(events.SESSION_INIT(
            vocab_version=VOCAB_VERSION, profile=host_id, contract_version="0.0.0",
            harness_version="0.0.1", purpose="capability"))
        for c in caps:
            yield c
        for mm in measurements:
            yield mm
        yield events.validate(events.SESSION_COMPLETED(
            outcome="completed", discrepancy_count=0, accepted_count=0,
            elapsed_seconds=round(elapsed, 3)))

    def topology(b: TopologyBuilder) -> None:
        b.baseline(host_id=host_id, probe="probes/page_geometry/page_geometry.c",
                   repeats=a.repeats, sets={k: list(v) for k, v in SETS.items()})
        b.producer_kind("destination_evaluator", schemas=list(events.HARNESS_EVENTS),
                        schema_version=1, factory=lambda: producer(harness))
        b.initial("destination_evaluator", input=None)
        b.termination(threshold_count("substrate.ProducerCompleted", 1))

    result = run_with_provenance(a.runs / f"pagegeom-{time.time_ns()}", topology)
    rec = read_record(result.record_root)
    vals: dict[str, float] = {}
    for e in rec:
        p = e["payload"]
        if e["kind"] == "CAPABILITY_PROBED":
            print(f"{p['capability']:26} {p['outcome']:8} {p['detail']}")
        elif e["kind"] == "MEASUREMENT_RECORDED":
            vals[p["variant"]] = p["value"]
    for s in SETS:
        base = vals[f"plain/{s}"]
        print(f"-- {s}")
        for m, v in VARIANTS:
            xs = samples[(s, m, v)]
            print(f"  {m:6} {v:16} {vals[f'{v}/{s}']:7.4f} ns/access  "
                  f"x{vals[f'{v}/{s}'] / base:4.2f}  (min {min(xs):.4f} max {max(xs):.4f})")
    print(f"record {result.record_root}; {elapsed:.1f} s of timing")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
