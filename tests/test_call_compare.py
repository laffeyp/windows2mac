# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""The comparison of w2m's calls with the game's on Windows (Sprint 060): each value by the relation its type allows,
and the call stream as a guest trace the ingester accepts."""

from __future__ import annotations

import hashlib
import json
import sys
from pathlib import Path

import pytest

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


def test_assigned_ids_map_one_to_one_and_thread_ids_through_the_thread_map() -> None:
    windows = [call(1, "TlsAlloc", (), 0x20, tid=6024), call(2, "TlsSetValue", (0x20, 0x50000), 1, tid=6024),
               call(3, "GetCurrentThreadId", (), 6024, tid=6024)]
    w2m = [call(1, "TlsAlloc", (), 0x14), call(2, "TlsSetValue", (0x14, 0x60000), 1),
           call(3, "GetCurrentThreadId", (), 4)]
    assert call_compare.compare(windows, w2m, SIGNATURES)[0] == []
    crossed = [call(1, "TlsAlloc", (), 0x14), call(2, "TlsSetValue", (0x15, 0x60000), 1),
               call(3, "GetCurrentThreadId", (), 8)]
    fields = [(d.function, d.field) for d in call_compare.compare(windows, crossed, SIGNATURES)[0]]
    assert fields == [("TlsSetValue", "a0"), ("GetCurrentThreadId", "ret")]


def test_threads_pair_by_first_appearance_and_excluded_threads_drop_out() -> None:
    windows = [call(1, "GetACP", (), 1252, tid=6636), call(2, "GetACP", (), 1252, tid=6024),
               call(3, "Sleep", (5,), 0, tid=7504), call(4, "Sleep", (0,), 0, tid=6024)]
    w2m = [call(1, "GetACP", (), 1252, tid=4), call(2, "Sleep", (0,), 0, tid=4), call(3, "Sleep", (5,), 0, tid=8)]
    assert call_compare.compare(windows, w2m, SIGNATURES, frozenset({6636}))[0] == []
    assert call_compare.compare(windows, w2m, SIGNATURES)[0] != []      # the capture's thread shifts the map


def test_an_integer_carrying_an_address_agrees_on_null() -> None:
    windows = [call(1, "DefWindowProcW", (0x501D0, 0x24, 0, 0x807D90ED60), 0),
               call(2, "DefWindowProcW", (0x501D0, 0x100, 0x41, 0x1E0001), 0)]
    w2m = [call(1, "DefWindowProcW", (0x20010, 0x24, 0, 0xFFFEFB08), 0),
           call(2, "DefWindowProcW", (0x20010, 0x100, 0x41, 0x1E0002), 0)]
    found = call_compare.compare(windows, w2m, SIGNATURES)[0]
    assert [(d.index, d.field) for d in found] == [(1, "a3")]           # WM_KEYDOWN's lParam is a value


@pytest.mark.hvf
def test_getprocaddress_while_the_trace_is_closed_answers_an_untraced_thunk() -> None:
    import struct
    import tempfile

    from w2m.file_bridge import FileBridge
    from w2m.loader import THUNK_BASE, load
    from w2m.vm_backend import TRACE_OFF, TRACE_OWNER_OFF, VM
    sys.path.insert(0, str(ROOT / "scripts"))
    import family_run
    f = family_run.FAMILIES["threads"]
    run = next(iter(f.runs.values()))
    assert f.dll_name is not None and run.dll is not None
    exe = (ROOT / "store" / "binaries" / f"{run.binary}.exe").read_bytes()
    dll = (ROOT / "store" / "binaries" / f"{run.dll}.dll").read_bytes()
    with tempfile.TemporaryDirectory() as d:
        files = FileBridge(Path(d))
        vm = VM(load(exe, lambda n: dll if n.lower() == "threads_dll.dll" else None, "probe.exe"), files=files,
                image_name="probe.exe", call_trace=("probe.exe",), call_trace_from="threads_dll.dll!threads_dll_count")
        try:
            shared = vm.modules.proc(vm.modules.kernel32, "GetACP")
            quiet = vm._traced_proc(shared)
            assert quiet != shared and vm.ld.thunks[quiet] == vm.ld.thunks[shared]
            assert vm.mm[vm._process(TRACE_OFF) + TRACE_OWNER_OFF + (quiet - THUNK_BASE) // 16] == 0xFF
            assert vm._traced_proc(shared) == quiet                       # one per name
            struct.pack_into("<Q", vm.mm, vm._process(TRACE_OFF), 1)       # the trace open
            assert vm._traced_proc(shared) == shared
        finally:
            vm.close()
            files.close_all()
