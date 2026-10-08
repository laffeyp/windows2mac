"""litmus_mp.py — EnTSO message-passing litmus on two vCPUs (Sprint 009).

Builds probes/mac_hv_litmus/mp_litmus.c, ad-hoc signs it with the hypervisor entitlement, runs
it, and records CAPABILITY_PROBED(guest_tsoen) and CAPABILITY_PROBED(guest_tsoen_context_switch)
with the counts on a substrate run record.

Usage: uv run python scripts/litmus_mp.py [--rounds 100] [--iterations 1000000] [--runs runs]
Exit 0 when EnTSO-on shows zero forbidden outcomes and ACTLR never drifts.
"""

from __future__ import annotations

import argparse
import asyncio
import json
import math
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
SRC = ROOT / "probes" / "mac_hv_litmus" / "mp_litmus.c"
ENT = ROOT / "probes" / "mac_hv_capability" / "entitlements.plist"
BIN = ROOT / "build" / "mp_litmus"


def sh(*cmd: str) -> str:
    return subprocess.run(cmd, check=True, capture_output=True, text=True).stdout.strip()


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--rounds", type=int, default=100)
    ap.add_argument("--iterations", type=int, default=1_000_000)
    ap.add_argument("--runs", type=Path, default=ROOT / "runs")
    a = ap.parse_args(argv)
    BIN.parent.mkdir(exist_ok=True)
    sh("clang", "-O1", "-Wall", "-Werror", "-o", str(BIN), str(SRC), "-framework", "Hypervisor")
    sh("codesign", "-s", "-", "-f", "--entitlements", str(ENT), str(BIN))
    t0 = time.monotonic()
    proc = subprocess.run([str(BIN), str(a.rounds), str(a.iterations)], capture_output=True,
                          text=True)
    elapsed = time.monotonic() - t0
    rows = [json.loads(x) for x in proc.stdout.splitlines()]
    off, on, tail = rows[0], rows[1], rows[2]
    if proc.returncode != 0 or tail["fail"]:
        print(f"probe failed: exit {proc.returncode} {tail['fail']} {proc.stderr}", file=sys.stderr)
        return 1
    host_id = f"mac-{sh('sysctl', '-n', 'hw.model')}-macos-{sh('sw_vers', '-buildVersion')}"
    # rule of three: zero events in n trials bounds the rate below 3/n at 95% confidence
    bound = 3 / on["iterations"]
    demonstrated = off["forbidden_outcomes"] > 0 and on["forbidden_outcomes"] == 0
    tso = events.validate(events.CAPABILITY_PROBED(
        capability="guest_tsoen", host_id=host_id,
        # present needs power: the same run's EnTSO-off rounds must show a forbidden outcome
        # (KIT_DIARY H4). off = 0 and on = 0 means the test could not tell (vocabulary 0.4, P12).
        outcome=("present" if demonstrated else "absent" if on["forbidden_outcomes"] > 0
                 else "not_established"),
        detail=(f"mp litmus, 2 vcpus, guest mmu on (normal wb ish), {a.rounds} rounds alternating: "
                f"entso off {off['forbidden_outcomes']}/{off['iterations']} forbidden outcomes "
                f"({off['forbidden_outcomes'] / off['iterations']:.3e}); entso on "
                f"{on['forbidden_outcomes']}/{on['iterations']} (95% upper bound {bound:.1e}); "
                + ("effect demonstrated" if demonstrated else "effect not demonstrated"))))
    exits = off["vcpu_exits"] + on["vcpu_exits"]
    drift = off["actlr_mismatch"] + on["actlr_mismatch"]
    ctx = events.validate(events.CAPABILITY_PROBED(
        capability="guest_tsoen_context_switch", host_id=host_id,
        outcome="present" if drift == 0 else "absent",
        detail=(f"actlr_el1 read back after each of {exits} vcpu exits on 2 host threads: "
                f"{drift} mismatches; other exits {tail['other_exits']}")))

    async def harness(_inp: Any) -> AsyncIterator[msgspec.Struct]:
        yield events.validate(events.SESSION_INIT(
            vocab_version=VOCAB_VERSION, profile=host_id, contract_version="0.0.0",
            harness_version="0.0.1", purpose="capability"))
        yield tso
        yield ctx
        yield events.validate(events.SESSION_COMPLETED(
            outcome="completed", discrepancy_count=0, accepted_count=0,
            elapsed_seconds=round(elapsed, 3)))

    def topology(b: TopologyBuilder) -> None:
        b.baseline(host_id=host_id, probe="probes/mac_hv_litmus/mp_litmus.c",
                   rounds=a.rounds, iterations=a.iterations)
        b.producer_kind("destination_evaluator", schemas=list(events.HARNESS_EVENTS),
                        schema_version=1, factory=lambda: harness)
        b.initial("destination_evaluator", input=None)
        b.termination(threshold_count("substrate.ProducerCompleted", 1))

    result = asyncio.run(Runtime(a.runs / f"litmus-{int(time.time())}").run(topology))
    for e in read_record(result.record_root):
        if e["kind"] == "CAPABILITY_PROBED":
            print(f"{e['payload']['capability']:28} {e['payload']['outcome']:8} {e['payload']['detail']}")
    print(f"record {result.record_root}; probe {elapsed:.1f} s; "
          f"off rate {off['forbidden_outcomes'] / off['iterations']:.3e}, "
          f"log10 ratio vs bound {math.log10(max(off['forbidden_outcomes'], 1) / off['iterations'] / bound):.1f}")
    return 0 if demonstrated and drift == 0 else 1


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
