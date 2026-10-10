"""probe_fp_control.py — AArch64 FPCR.AH in a Hypervisor.framework guest, judged against x86
(Sprint 039; review 2026-10-09 progress N4).

Builds probes/mac_hv_fp/hv_fp.c, signs it with the hypervisor entitlement, runs it, and judges
every test line run with FPCR.AH=1 (and AH|FIZ) against the x86 answer x86fp gives for the same
operands (MXCSR all masked, round to nearest; FIZ stands for DAZ). FPSR's cumulative bits map to
MXCSR's flags: IOC IE, DZC ZE, OFC OE, UFC UE, IXC PE, IDC DE. One CAPABILITY_PROBED per
capability (vocabulary 0.7, P17): present when every test of the group gives x86's value and
flags, absent when any differs, with the differing tests in the detail.

Usage: uv run python scripts/probe_fp_control.py [--runs runs]
"""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
from collections.abc import Iterator
from pathlib import Path
from typing import Any

import msgspec

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))
from w2m import events  # noqa: E402
from w2m import x86fp as fp  # noqa: E402
from w2m.record import one_step_run  # noqa: E402

SRC = ROOT / "probes" / "mac_hv_fp" / "hv_fp.c"
BIN = ROOT / "build" / "hv_fp"
ENTITLEMENTS = ROOT / "probes" / "mac_hv_capability" / "entitlements.plist"
FPCR_AH, FPCR_FIZ = 2, 1
FPSR_TO_MXCSR = {0: fp.IE, 1: fp.ZE, 2: fp.OE, 3: fp.UE, 4: fp.PE, 7: fp.DE}
GROUPS = {
    "guest_fp_minmax_x86": ("fmin_", "fmax_"),
    "guest_fp_nan_x86": ("fsub_inf_inf_default_nan", "fadd_qnan_snan_priority"),
    "guest_fp_tininess_x86": ("fmul_tiny",),
    "guest_fp_denormal_x86": ("fadd_denormal_input",),
    "guest_fp_to_int_x86": ("fcvtzs_",),
}


def sh(*cmd: str) -> str:
    return subprocess.run(cmd, check=True, capture_output=True, text=True).stdout.strip()


def x86_answer(test: str, s1: int, s2: int, daz: bool) -> tuple[int, int]:
    """(result, MXCSR flags) on x86 for the test's operation: SSE scalar single, first source s1."""
    env = fp.Env(fp.MXCSR_DEFAULT | (fp.DAZ if daz else 0))
    op = test.split("_")[0]
    if op in ("fmin", "fmax"):
        r = fp.minmax(op[1:], fp.F32, s1, s2, env)
    elif op in ("fmul", "fadd", "fsub"):
        r = fp.arith(op[1:], fp.F32, s1, s2, env)
    elif op == "fcvtzs":
        r = fp.to_int(fp.F32, s1, 64 if "_x_" in test else 32, True, env)
    else:
        raise ValueError(test)
    return r, env.flags


def judge(line: dict[str, Any]) -> str | None:
    """None when the guest's line equals x86; otherwise what differs."""
    daz = bool(line["fpcr"] & FPCR_FIZ)
    want, want_flags = x86_answer(line["test"], line["s1"], line["s2"], daz)
    got = line["x0"] if line["test"].startswith("fcvtzs") else line["s0"]
    if line["test"].startswith("fcvtzs") and "_x_" not in line["test"]:
        got &= 0xFFFF_FFFF
    flags = sum(m for bit, m in FPSR_TO_MXCSR.items() if line["fpsr"] >> bit & 1)
    out = []
    if got != want:
        out.append(f"value 0x{got:x}, x86 0x{want:x}")
    if flags != want_flags:
        out.append(f"flags 0x{flags:x}, x86 0x{want_flags:x}")
    return f"{line['test']} (FPCR {line['fpcr']}): " + "; ".join(out) if out else None


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--runs", type=Path, default=ROOT / "runs")
    a = ap.parse_args(argv)
    BIN.parent.mkdir(exist_ok=True)
    sh("clang", "-O1", "-Wall", "-Werror", "-o", str(BIN), str(SRC), "-framework", "Hypervisor")
    sh("codesign", "-s", "-", "-f", "--entitlements", str(ENTITLEMENTS), str(BIN))
    lines = [json.loads(x) for x in sh(str(BIN)).splitlines()]
    checks = {x["check"]: x for x in lines if "check" in x}
    tests = [x for x in lines if "test" in x and x["fpcr"] & FPCR_AH]
    host_id = f"mac-{sh('sysctl', '-n', 'hw.model')}-macos-{sh('sw_vers', '-buildVersion')}"
    afp = checks["id_aa64mmfr1"]["value"] >> 44 & 0xF

    def cap(name: str, ok: bool, detail: str) -> msgspec.Struct:
        return events.CAPABILITY_PROBED(capability=name, host_id=host_id,  # type: ignore[arg-type]
                                        outcome="present" if ok else "absent", detail=detail)

    def work() -> Iterator[msgspec.Struct]:
        rb = checks["fpcr_readback"]
        yield cap("guest_fp_alternate_handling", afp >= 1 and rb["read"] == rb["set"],
                  f"guest mrs ID_AA64MMFR1_EL1 = 0x{checks['id_aa64mmfr1']['value']:x} (AFP field {afp}); "
                  f"FPCR set {rb['set']} (AH|FIZ|NEP), guest mrs FPCR reads {rb['read']}")
        for name, prefixes in GROUPS.items():
            group = [t for t in tests if t["test"].startswith(prefixes)]
            bad = [d for d in map(judge, group) if d]
            yield cap(name, not bad, f"{len(group)} runs with FPCR.AH=1 (and AH|FIZ), each judged against "
                      f"x86fp: " + ("all equal x86 in value and flags" if not bad else "; ".join(bad)))

    root, kinds = one_step_run(a.runs / f"capability-fp-{__import__('time').time_ns()}", profile=host_id,
                               contract_version="0.0.0", purpose="capability", work=work,
                               baseline={"host_id": host_id, "probe": "probes/mac_hv_fp/hv_fp.c"})
    from substrate.api import read_record
    for e in read_record(root):
        if e["kind"] == "CAPABILITY_PROBED":
            p = e["payload"]
            print(f"{p['capability']:28} {p['outcome']:8} {p['detail']}")
    print(f"record {root}: {dict(kinds)}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
