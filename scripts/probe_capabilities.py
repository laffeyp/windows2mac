"""probe_capabilities.py — destination capability probes on this Mac (Sprint 008).

Builds probes/mac_hv_capability/hv_capability.c, ad-hoc signs it with the
com.apple.security.hypervisor entitlement, runs the 4k and 16k modes, and records one
CAPABILITY_PROBED event per capability on a substrate run record (signals/0.1.json).

Usage: uv run python scripts/probe_capabilities.py [--runs runs]
Exit 0 when every expected capability outcome is observed.
"""

from __future__ import annotations

import argparse
import asyncio
import json
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
SRC = ROOT / "probes" / "mac_hv_capability"
BIN = ROOT / "build" / "hv_capability"


def sh(*cmd: str) -> str:
    return subprocess.run(cmd, check=True, capture_output=True, text=True).stdout.strip()


def build() -> None:
    BIN.parent.mkdir(exist_ok=True)
    sh("clang", "-O1", "-Wall", "-Werror", "-o", str(BIN), str(SRC / "hv_capability.c"),
       "-framework", "Hypervisor")
    sh("codesign", "-s", "-", "-f", "--entitlements", str(SRC / "entitlements.plist"), str(BIN))


def run_mode(mode: str) -> dict[str, dict[str, Any]]:
    out = subprocess.run([str(BIN), mode], capture_output=True, text=True, check=True).stdout
    return {c["check"]: c for c in map(json.loads, out.splitlines())}


def capabilities(host_id: str) -> list[events.CAPABILITY_PROBED]:
    k4, k16 = run_mode("4k"), run_mode("16k")
    page = int(sh("sysctl", "-n", "hw.pagesize"))

    def cap(name: events.EnumCapability, ok: bool | None, detail: str) -> events.CAPABILITY_PROBED:
        """ok None: the probe cannot decide (vocabulary 0.4, P12)."""
        return events.validate(events.CAPABILITY_PROBED(
            capability=name, host_id=host_id,
            outcome="not_established" if ok is None else "present" if ok else "absent",
            detail=detail))

    ran = k4["guest_runs_from_4k_page"]
    control = k16["map_4k_page_at_0x10001000"]
    return [
        cap("ipa_granule_4k",
            k4["set_granule_4k"]["ok"] and k4["map_4k_page_at_0x10001000"]["ok"] and ran["ok"]
            and not control["ok"],
            f"hv_vm_config_set_ipa_granule(4KB) ok; 4 KB page at IPA 0x10001000 mapped; guest "
            f"ran from it ({ran['detail']}); control with default 16 KB granule refused the "
            f"same map ({control['detail']}, HV_BAD_ARGUMENT)"),
        cap("guest_tsoen", k4["entso_set_readback"]["ok"],
            f"hv_vcpu_set_sys_reg(HV_SYS_REG_ACTLR_EL1, 1<<1) then get: "
            f"{k4['entso_set_readback']['detail']}"),
        cap("guest_tsoen_context_switch", k4["entso_after_run"]["ok"],
            f"after one hv_vcpu_run exit, ACTLR_EL1 reads: {k4['entso_after_run']['detail']}; "
            f"one vCPU, one exit; effect on ordering not tested here"),
        cap("host_page_16k", page == 16384, f"sysctl hw.pagesize = {page}"),
        cap("native_process_tso", None,
            "no public interface found in macOS 26.5 SDK headers outside Hypervisor.framework "
            "(searched EnTSO, TSOEN, total store order); a header search cannot establish "
            "absence, and v0.2 8.1 keeps the question open"),
    ]


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--runs", type=Path, default=ROOT / "runs")
    a = ap.parse_args(argv)
    build()
    host_id = f"mac-{sh('sysctl', '-n', 'hw.model')}-macos-{sh('sw_vers', '-buildVersion')}"
    caps = capabilities(host_id)
    t0 = time.monotonic()

    async def harness(_inp: Any) -> AsyncIterator[msgspec.Struct]:
        yield events.validate(events.SESSION_INIT(
            vocab_version=VOCAB_VERSION, profile=host_id, contract_version="0.0.0",
            harness_version="0.0.1", purpose="capability"))
        for c in caps:
            yield c
        yield events.validate(events.SESSION_COMPLETED(
            outcome="completed", discrepancy_count=0, accepted_count=0,
            elapsed_seconds=round(time.monotonic() - t0, 3)))

    def topology(b: TopologyBuilder) -> None:
        b.baseline(host_id=host_id, probe="probes/mac_hv_capability/hv_capability.c")
        b.producer_kind("destination_evaluator", schemas=list(events.HARNESS_EVENTS),
                        schema_version=1, factory=lambda: harness)
        b.initial("destination_evaluator", input=None)
        b.termination(threshold_count("substrate.ProducerCompleted", 1))

    result = asyncio.run(Runtime(a.runs / f"capability-{time.time_ns()}").run(topology))
    expected = {"ipa_granule_4k": "present", "guest_tsoen": "present",
                "guest_tsoen_context_switch": "present", "host_page_16k": "present",
                "native_process_tso": "absent"}
    ok = True
    for e in read_record(result.record_root):
        if e["kind"] == "CAPABILITY_PROBED":
            p = e["payload"]
            ok &= expected[p["capability"]] == p["outcome"]
            print(f"{p['capability']:28} {p['outcome']:8} {p['detail']}")
    print(f"record {result.record_root} ({host_id})")
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
