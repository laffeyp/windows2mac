"""Sprint 037: the semantics oracle. The committed case file is what the generator builds; the
runner's CPU results agree with the model on every case (Intel Xeon Platinum 8573C, run
38001673879) and on every case except the 0F 0D register forms, which AMD faults on (AMD EPYC 7763,
run 38001569019); a planted lifter-semantics fault is caught by the CPU alone, with no lockstep or
state differential involved. Full result files are assets of the private release semantics-037
(sha256 in store/probe-runs/<run>/results.sha256); the repo keeps every 10th case per vendor."""

from __future__ import annotations

import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))
import semantics_differential as sd  # noqa: E402

AMD = ROOT / "store" / "probe-runs" / "38001569019" / "result-1.every10.bin"
INTEL = ROOT / "store" / "probe-runs" / "38001673879" / "result-1.every10.bin"


CASES_037 = ROOT / "store" / "probe-runs" / "semantics-037-cases.bin"


def _classes(results: Path = AMD, limit: int | None = None) -> dict[str, int]:
    """Sprint 037's format-1 cases against its stored results; AF is compared since Sprint 039."""
    out: dict[str, int] = {}
    cases = sd.read_cases_format1(CASES_037)[::10]
    for c, r in zip(cases, sd.read_results_format1(results), strict=True):
        label = sd.decode(c.code).iclass if c.code[0] not in range(0x70, 0x80) else "Jcc"
        if limit is not None and label not in ("SUB", "CMP"):
            continue
        for k, _ in sd.judge_format1(c, r, label):
            out[k] = out.get(k, 0) + 1
    return out


def test_case_file_round_trips(tmp_path: Path) -> None:
    built, _ = sd.build_cases()
    sd.write_cases(built[::50], tmp_path / "cases.bin")
    stored = sd.read_cases(tmp_path / "cases.bin")
    assert [(c.code, c.regs, c.rflags, c.mxcsr, c.xmm, c.data, c.stack, c.edge, c.kind) for c in built[::50]] == \
        [(c.code, c.regs, c.rflags, c.mxcsr, c.xmm, c.data, c.stack, c.edge, c.kind) for c in stored]


def test_intel_cpu_and_model_agree_on_every_case() -> None:
    assert _classes(INTEL) == {}


def test_amd_cpu_faults_on_0f0d_register_forms_only() -> None:
    found = _classes(AMD)
    assert list(found) == ["NOP cpu fault"] and found["NOP cpu fault"] > 0


def test_planted_signed_carry_on_sub_is_caught_by_the_cpu(monkeypatch: pytest.MonkeyPatch) -> None:
    """Sprint 030 planted this fault in the VM compiler, and lockstep alone missed it. Here it is
    planted in the model and only the CPU's results are the reference."""
    from w2m.runtime import Runtime
    original = Runtime._set_flags

    def signed_carry(self: Runtime, kind: str, a: int, b: int, r: int, bits: int) -> None:
        original(self, kind, a, b, r, bits)
        if kind == "sub":
            m = 1 << (bits - 1)
            self.th.flags["C"] = ((a & ((1 << bits) - 1)) ^ m) < ((b & ((1 << bits) - 1)) ^ m)

    monkeypatch.setattr(Runtime, "_set_flags", signed_carry)
    found = _classes(INTEL, limit=1)
    assert found.get("SUB CF", 0) > 0 and found.get("CMP CF", 0) > 0
