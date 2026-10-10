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


# Sprint 039: format 2 (XMM, MXCSR, AF, DF, faults). The case file is built from its seed (the
# runner builds the same bytes: cases.sha256); the repo keeps every 10th result of each run.
RUN_039_AMD = ROOT / "store" / "probe-runs" / "38010775429"


def _classes_039(run: Path, labels: set[str] | None = None) -> dict[str, int]:
    built = sd.build_cases()[0][::10]
    out: dict[str, int] = {}
    for c, r in zip(built, sd.read_results(run / "result-1.every10.bin"), strict=True):
        if labels is not None and c.label not in labels:
            continue
        for k, _ in sd.judge(c, r, c.label):
            out[k] = out.get(k, 0) + 1
    return out


def test_039_runner_ran_the_case_file_this_generator_builds(tmp_path: Path) -> None:
    import hashlib
    sd.write_cases(sd.build_cases()[0], tmp_path / "cases.bin")
    assert hashlib.sha256((tmp_path / "cases.bin").read_bytes()).hexdigest() == \
        (RUN_039_AMD / "cases.sha256").read_text().strip()


def test_039_amd_cpu_and_model_agree_except_0f0d_register_forms() -> None:
    found = _classes_039(RUN_039_AMD)
    assert list(found) == ["NOP cpu fault"]


def test_039_planted_minps_first_source_on_nan_is_caught(monkeypatch: pytest.MonkeyPatch) -> None:
    """x86 MIN returns the second source when either is NaN; plant the first."""
    from w2m import x86fp
    original = x86fp.minmax

    def first_on_nan(op: str, f: x86fp.Fmt, a: int, b: int, env: x86fp.Env) -> int:
        r = original(op, f, a, b, env)
        return a if x86fp.is_nan(f, b) and not x86fp.is_nan(f, a) else r
    monkeypatch.setattr(x86fp, "minmax", first_on_nan)
    found = _classes_039(RUN_039_AMD, {"MINPS", "MINSS", "MINPD", "MINSD", "MAXPS", "MAXSS", "MAXPD", "MAXSD"})
    assert any(k.split()[1].startswith("xmm") for k in found), found


def test_039_planted_saturating_cvttss2si_is_caught(monkeypatch: pytest.MonkeyPatch) -> None:
    """x86 returns the integer indefinite 1 << (w-1) on overflow; AArch64 FCVTZS saturates."""
    from w2m import x86fp
    original = x86fp.to_int

    def saturating(f: x86fp.Fmt, b: int, bits: int, truncate: bool, env: x86fp.Env) -> int:
        r = original(f, b, bits, truncate, env)
        if r == 1 << (bits - 1) and not x86fp.is_nan(f, b) and not b >> (f.bits - 1) & 1:
            return (1 << (bits - 1)) - 1
        return r
    monkeypatch.setattr(x86fp, "to_int", saturating)
    found = _classes_039(RUN_039_AMD, {"CVTTSS2SI", "CVTTSD2SI", "CVTSS2SI", "CVTSD2SI"})
    assert any(" r" in f" {k.split()[1]}" for k in found), found


def test_039_planted_af_from_the_wrong_bit_is_caught(monkeypatch: pytest.MonkeyPatch) -> None:
    from w2m.runtime import Runtime
    original = Runtime._set_flags

    def af_bit3(self: Runtime, kind: str, a: int, b: int, r: int, bits: int) -> None:
        original(self, kind, a, b, r, bits)
        if self.th.flags.get("A") is not None:
            self.th.flags["A"] = bool((a ^ b ^ r) & 0x8)
    monkeypatch.setattr(Runtime, "_set_flags", af_bit3)
    found = _classes_039(RUN_039_AMD, {"ADD", "SUB", "CMP", "INC", "DEC"})
    assert any(k.endswith(" AF") for k in found), found


def test_039_planted_partial_store_before_a_fault_is_caught(monkeypatch: pytest.MonkeyPatch) -> None:
    """A store straddling the end of the data page must write nothing; plant one that writes the
    mapped bytes first, then faults."""
    from w2m.loader import GuestFault, Memory
    original = Memory.write

    def partial(self: Memory, addr: int, data: bytes) -> None:
        try:
            original(self, addr, data)
        except GuestFault:
            for i, byte in enumerate(data):
                try:
                    original(self, addr + i, bytes([byte]))
                except GuestFault:
                    break
            raise
    monkeypatch.setattr(Memory, "write", partial)
    found = _classes_039(RUN_039_AMD, {"MOV", "MOVAPS", "MOVUPS", "MOVSS", "MOVSD_XMM", "MOVQ", "MOVD", "PUSH"})
    assert any(k.endswith(" edge") for k in found), found
