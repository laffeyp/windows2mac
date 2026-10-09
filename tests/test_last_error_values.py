"""Sprint 032 grading surface: windows.thread.last_error.set_get@0.2.0 on probes/last_error_values.
The probe's table carries true argument hashes; the `exact` relation conforms only when each
GetLastError returns the argument of the latest SetLastError; the values differ between runs on
Windows and on the Mac; lockstep holds when a kernel32 call returns to an address already
compiled."""

from __future__ import annotations

import hashlib
import json
import re
import sys
from pathlib import Path

import pytest

from w2m.compare import compare
from w2m.contract import ContractError, load_contract
from w2m.kernel32 import HandlerFailed
from w2m.loader import load
from w2m.runtime import Runtime

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))
import gen_last_error_values  # noqa: E402

CONTRACTS = ROOT / "contracts" / "windows.thread.last_error.set_get"
PROBE_SHA256 = "d210df33adce2f0807fefba2e572cc48bddf9649b8b2474658c92fb46f7b1210"
OPENED = (b'{"tag":"GUEST_TRACE_OPENED","execution_id":"t","vocab_version":"0.6","profile_id":"t",'
          b'"trace_format_version":"0.1"}')


def _probe() -> bytes:
    return (ROOT / "store" / "binaries" / f"{PROBE_SHA256}.exe").read_bytes()


def _judge(body: bytes, version: str = "0.2.0") -> tuple[str, str | None]:
    tid = json.loads(body.splitlines()[0])["thread_id"]
    out = compare(load_contract(CONTRACTS / f"{version}.json"), OPENED + b"\n" + body,
                  {"main": tid}, "c", "r", "t")
    return out.verdict, out.first_divergence


def _set_hashes(body: bytes) -> list[str]:
    return re.findall(rb'SetLastError","args_sha256":"(sha256:[0-9a-f]{64})', body)


def test_table_hashes_are_the_canonical_rendering() -> None:
    assert gen_last_error_values.main(["--check"]) == 0          # values.h is the generator's output
    text = (ROOT / "probes" / "last_error_values" / "values.h").read_text()
    values = [int(v, 16) for v in re.findall(r"^\s+0x([0-9a-f]{8})u,$", text, re.M)]
    hashes = re.findall(r'"sha256:([0-9a-f]{64})"', text)
    assert len(values) == len(set(values)) == len(hashes) == 256
    for v, h in zip(values, hashes, strict=True):
        assert hashlib.sha256(f"SetLastError({v})".encode()).hexdigest() == h
    assert {0, 1, 0x7FFF_FFFF, 0x8000_0000, 0xFFFF_FFFF} <= set(values)


def test_windows_reference_conforms_with_values_varying() -> None:
    triples = []
    for m in sorted((ROOT / "store" / "executions").glob("*.json")):
        meta = json.loads(m.read_text())
        if meta.get("probe_sha256") != f"sha256:{PROBE_SHA256}" or "profile_kind" in meta:
            continue
        raw = (ROOT / "store" / "traces" / f"{m.stem}.jsonl").read_bytes()
        out = compare(load_contract(CONTRACTS / "0.2.0.json"), raw, meta["roles"], "c", "r", "t")
        assert out.verdict == "conforms" and out.relation == "exact"
        triples.append(tuple(_set_hashes(raw)))
    assert len(triples) == 3 and len(set(triples)) == 3
    assert all(a != b for t in triples for a, b in zip(t, t[1:]))   # consecutive values differ


def test_model_runtime_conforms_and_clock_chooses_values() -> None:
    runs = [Runtime(load(_probe()), clock=lambda c=c: c).run() for c in (1_000, 1_000, 987_654_321)]
    assert all((r.stop, r.exit_code) == ("ExitProcess", 0) for r in runs)
    assert runs[0].calls[:3] == ["GetCurrentThreadId", "QueryPerformanceCounter", "SetLastError"]
    assert all(_judge(r.stdout)[0] == "conforms" for r in runs)
    assert _set_hashes(runs[0].stdout) == _set_hashes(runs[1].stdout)
    assert _set_hashes(runs[0].stdout) != _set_hashes(runs[2].stdout)


def test_constant_and_lost_values_are_discrepancies() -> None:
    body = Runtime(load(_probe()), clock=lambda: 42).run().stdout
    constant = re.sub(rb'(GetLastError","args_sha256":"sha256:[0-9a-f]{64}","return_value":)\d+',
                      rb"\g<1>1234", body)
    verdict, why = _judge(constant)
    assert verdict == "discrepancy" and "returned 1234" in (why or "")
    second_lost = body.splitlines()
    first_get = second_lost[2]
    second_lost[4] = re.sub(rb'"return_value":\d+', re.search(rb'"return_value":\d+', first_get)
                            .group(0), second_lost[4])                 # second read repeats the first
    verdict, why = _judge(b"\n".join(second_lost) + b"\n")
    assert verdict == "discrepancy" and "read 2 of 3" in (why or "")


def test_missing_or_unordered_reads_are_inconclusive() -> None:
    lines = Runtime(load(_probe()), clock=lambda: 42).run().stdout.splitlines()
    assert _judge(b"\n".join(lines[:5] + lines[7:]) + b"\n")[0] == "inconclusive"    # 2 reads
    get_first = [lines[0], lines[2].replace(b'"thread_seq":2', b'"thread_seq":1'),
                 lines[1].replace(b'"thread_seq":1', b'"thread_seq":2'), *lines[3:]]
    verdict, why = _judge(b"\n".join(get_first) + b"\n")
    assert verdict == "inconclusive" and "precedes any" in (why or "")


def test_contract_0_1_0_still_judges_the_skeleton() -> None:
    c = load_contract(CONTRACTS / "0.1.0.json")
    assert type(c.acceptance).__struct_config__.tag == "allowed_outcome_set"


def test_exact_relation_rejects_same_read_and_write_api(tmp_path: Path) -> None:
    d = tmp_path / "windows.thread.last_error.set_get"
    d.mkdir()
    c = json.loads((CONTRACTS / "0.2.0.json").read_text())
    c["acceptance"]["read_api"] = c["acceptance"]["write_api"]
    (d / "0.2.0.json").write_text(json.dumps(c))
    with pytest.raises(ContractError, match="must differ"):
        load_contract(d / "0.2.0.json")


def test_failing_handler_stops_the_run() -> None:
    rt = Runtime(load(_probe()))

    def broken() -> int:
        return 1 // 0

    rt.kernel.handlers["GetLastError"] = broken
    r = rt.run()
    assert r.exit_code is None and r.stop.startswith("HandlerFailed: GetLastError raised ZeroDivisionError")
    assert issubclass(HandlerFailed, RuntimeError)


@pytest.mark.hvf
def test_lockstep_holds_across_repeated_kernel32_returns() -> None:
    """Before the fix, lockstep reported 23 differences from step 4,404: the third SetLastError
    returned to a block compiled on the first, and the VM ran it before the check."""
    from w2m.vm_backend import lockstep
    result = lockstep(_probe())
    assert result.differences == []
    assert result.compared == result.model.instructions_executed + len(result.model.calls)
    assert result.vm.stdout == result.model.stdout
