# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""The comparison of w2m's calls with the game's on Windows (Sprint 060): each value by the relation its type allows,
and the call stream as a guest trace the ingester accepts."""

from __future__ import annotations

import hashlib
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))

import call_compare  # noqa: E402
from w2m import events  # noqa: E402
from w2m.trace import ingest  # noqa: E402

SIGNATURES = json.loads(call_compare.SIGNATURES.read_text())["functions"]
NONE8 = (0,) * 8


def call(seq: int, fn: str, args: tuple[int, ...], ret: int, error: int = 0, tid: int = 4,
         image: str = "UnityPlayer.dll") -> call_compare.Call:
    return (seq, tid, image, "KERNEL32.dll", fn, (*args, *NONE8)[:8], ret, error)


def test_a_32_bit_argument_is_compared_in_its_own_bytes() -> None:
    windows = [call(1, "WaitForSingleObject", (0x1C4, 0xFFFFFFFF), 0)]
    w2m = [call(7, "WaitForSingleObject", (0x3000, 0xDEAD0000FFFFFFFF), 0)]   # a different handle; upper half left
    assert call_compare.compare(windows, w2m, SIGNATURES) == ([], 1)


def test_a_wrong_return_is_a_divergence_at_its_call() -> None:
    windows = [call(1, "GetACP", (), 1252), call(2, "GetACP", (), 1252)]
    w2m = [call(1, "GetACP", (), 1252), call(2, "GetACP", (), 65001)]
    found, compared = call_compare.compare(windows, w2m, SIGNATURES)
    assert compared == 2
    assert [d.text() for d in found] == ["UnityPlayer.dll!GetACP call 1 ret: Windows 0x4e4, w2m 0xfde9"]


def test_a_last_error_is_compared_where_the_call_failed() -> None:
    # CreateFileW sets the last error (SupportsLastError); a failed open is INVALID_HANDLE_VALUE
    failed = (1 << 64) - 1
    windows = [call(1, "CreateFileW", (0x7FF0000, 0x80000000, 1, 0, 3, 0x80, 0), failed, 2)]
    same = [call(1, "CreateFileW", (0x50000, 0x80000000, 1, 0, 3, 0x80, 0), failed, 2)]
    dropped = [call(1, "CreateFileW", (0x50000, 0x80000000, 1, 0, 3, 0x80, 0), failed, 0)]
    assert call_compare.compare(windows, same, SIGNATURES)[0] == []
    assert [d.field for d in call_compare.compare(windows, dropped, SIGNATURES)[0]] == ["error"]
    succeeded = [call(1, "CreateFileW", (0x7FF0000, 0x80000000, 1, 0, 3, 0x80, 0), 0x1C4, 183)]
    ours = [call(1, "CreateFileW", (0x50000, 0x80000000, 1, 0, 3, 0x80, 0), 0x3000, 0)]
    assert call_compare.compare(succeeded, ours, SIGNATURES)[0] == []         # success: the last error is older


def test_pointers_and_handles_agree_on_null_and_ids_on_value() -> None:
    windows = [call(1, "CreateFileW", (0x7FF0000, 0x80000000, 1, 0, 3, 0x80, 0), 0x1C4)]
    null_name = [call(1, "CreateFileW", (0, 0x80000000, 1, 0, 3, 0x80, 0), 0x3000)]
    assert [d.field for d in call_compare.compare(windows, null_name, SIGNATURES)[0]] == ["a0"]
    pseudo = [call(1, "WaitForSingleObject", ((1 << 64) - 1, 0), 0)]
    other = [call(1, "WaitForSingleObject", ((1 << 64) - 2, 0), 0)]
    assert [d.field for d in call_compare.compare(pseudo, other, SIGNATURES)[0]] == ["a0"]


def test_a_function_windows_never_calls_is_a_divergence() -> None:
    found, _ = call_compare.compare([], [call(1, "GetACP", (), 1252)], SIGNATURES)
    assert [(d.function, d.field) for d in found] == [("GetACP", "called")]


def test_the_windows_traces_text_reads_back_as_written() -> None:
    windows = call_compare.read_tsv(ROOT / "store" / "game-runs" / "38176899195" / "jit_capture.bin.trace.tsv")
    assert len(windows) == 3496
    assert call_compare.read_tsv_text(call_compare.write_tsv(windows)) == windows


def test_the_call_stream_is_a_guest_trace_the_ingester_accepts(tmp_path: Path) -> None:
    calls = [call(0, "GetACP", (), 1252), call(1, "CreateThread", (0, 0, 0x140001000, 0, 0, 0), 0x3010, tid=4),
             call(2, "GetACP", (), 1252, tid=8)]
    text = call_compare.guest_trace("frontier-x", calls, {4: (0, 0x140001000, False), 8: (4, 0x140002000, False)},
                                    0, SIGNATURES)
    sha = "sha256:" + hashlib.sha256(text).hexdigest()
    (tmp_path / "traces").mkdir()
    (tmp_path / "traces" / f"{sha[7:]}.jsonl").write_bytes(text)
    result = ingest(tmp_path, sha, "frontier-x")
    assert isinstance(result, events.TRACE_INGESTED), result
    assert (result.line_count, result.thread_count) == (7, 2)


def test_the_last_error_column_is_decimal_as_call_trace_h_prints_it() -> None:
    """call_trace.h line 286 prints the last error with %lu: the first comparison read 203 (ERROR_ENVVAR_NOT_FOUND)
    as 0x203 and reported 32 GetEnvironmentVariableW divergences that were not there."""
    windows = call_compare.read_tsv(ROOT / "store" / "game-runs" / "38176899195" / "jit_capture.bin.trace.tsv")
    errors = {c[7] for c in windows if c[4] == "GetEnvironmentVariableW"}
    assert 203 in errors and 0x203 not in errors
