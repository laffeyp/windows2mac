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
    from w2m.vm_backend import STATE_OFF, TRACE_OFF, TRACE_OWNER_OFF, VM
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
            stack = vm.ld.image.image_base + 0x800                         # a mapped quadword: the header page
            struct.pack_into("<Q", vm.mm, vm._process(STATE_OFF) + 8 * 4, stack)
            vm.mem.write(stack, (0x7000_0000_0000).to_bytes(8, "little"))  # a caller outside the traced images
            assert vm._traced_proc(shared) == shared
        finally:
            vm.close()
            files.close_all()


def test_workers_pair_by_agreement_and_a_fault_on_every_worker_survives() -> None:
    """Job workers run one routine and start in the scheduler's order; the capture keeps different threads' early
    calls than w2m does. Windows' -1 worker pairs with w2m's -1 worker; a priority wrong on every worker still shows."""
    windows = [call(1, "GetACP", (), 1252, tid=100), call(2, "SetThreadPriority", ((1 << 64) - 2, (-1) & 0xFFFFFFFF), 1, tid=200),
               call(3, "SetThreadPriority", ((1 << 64) - 2, (-2) & 0xFFFFFFFF), 1, tid=300)]
    swapped = [call(1, "GetACP", (), 1252, tid=4), call(2, "SetThreadPriority", ((1 << 64) - 2, (-2) & 0xFFFFFFFF), 1, tid=8),
               call(3, "SetThreadPriority", ((1 << 64) - 2, (-1) & 0xFFFFFFFF), 1, tid=12)]
    assert call_compare.compare(windows, swapped, SIGNATURES)[0] == []
    wrong = [call(1, "GetACP", (), 1252, tid=4), call(2, "SetThreadPriority", ((1 << 64) - 2, 0), 1, tid=8),
             call(3, "SetThreadPriority", ((1 << 64) - 2, 0), 1, tid=12)]
    assert [(d.function, d.field) for d in call_compare.compare(windows, wrong, SIGNATURES)[0]] == \
        [("SetThreadPriority", "a1"), ("SetThreadPriority", "a1")]


def test_a_wait_that_returns_wait_object_0_has_no_failure_to_judge() -> None:
    """Sprint 064: the failure rule took any 4-byte 0 for failure and compared WaitForSingleObjectEx's stale last error
    (Windows 0, w2m 187 left by the thread's start). Only a BOOL's FALSE, a NULL or a bad handle say a call failed."""
    windows = [call(1, "WaitForSingleObjectEx", (0x1C4, 0xFFFFFFFF, 0), 0, 0)]
    w2m = [call(1, "WaitForSingleObjectEx", (0x3000, 0xFFFFFFFF, 0), 0, 187)]
    assert call_compare.compare(windows, w2m, SIGNATURES)[0] == []
    failed_close = [call(1, "CloseHandle", (0x1C4,), 0, 6)]
    assert [d.field for d in call_compare.compare(failed_close, [call(1, "CloseHandle", (0x3000,), 0, 0)],
                                                  SIGNATURES)[0]] == ["error"]


def test_named_threads_pair_by_name_before_agreement() -> None:
    """Sprint 064: the game's workers name themselves (SetThreadDescription); the comparison pairs them by name, so a
    priority wrong on one named worker shows even where pairing by agreement would swap the two workers."""
    def worker(seq: int, tid: int, name: str, priority: int) -> list[call_compare.Call]:
        return [call(seq, "SetThreadDescription", ((1 << 64) - 2, 0x50000 + seq), 0, tid=tid),
                call(seq + 1, "SetThreadPriority", ((1 << 64) - 2, priority & 0xFFFFFFFF), 1, tid=tid)]
    windows = worker(10, 200, "A", -1) + worker(20, 300, "B", -2)
    w_strings = [(10, 1, "A"), (20, 1, "B")]
    right = worker(10, 8, "B", -2) + worker(20, 12, "A", -1)
    r_strings = [(10, 1, "B"), (20, 1, "A")]
    assert call_compare.compare(windows, right, SIGNATURES, frozenset(), (w_strings, r_strings))[0] == []
    wrong = worker(10, 8, "B", -1) + worker(20, 12, "A", -2)
    found = call_compare.compare(windows, wrong, SIGNATURES, frozenset(), (w_strings, r_strings))[0]
    assert [(d.function, d.field) for d in found] == [("SetThreadPriority", "a1"), ("SetThreadPriority", "a1")]
    assert call_compare.compare(windows, wrong, SIGNATURES)[0] == []      # by agreement alone, the swap hides it


def test_a_hinted_reservation_and_its_retry_drop_out_on_both_sides() -> None:
    # mono asks for the address past its last code block; whether it is free is the address space's (Sprint 065)
    windows = [call(1, "VirtualAlloc", (0x2448B3C0000, 0x10000, 0x3000, 0x40), 0x2448B3C0000, image="mono-2.0-bdwgc.dll"),
               call(2, "VirtualAlloc", (0, 0x56000, 0x3000, 4), 0x2448B3D0000, image="mono-2.0-bdwgc.dll")]
    w2m = [call(1, "VirtualAlloc", (0x1222E70000, 0x10000, 0x3000, 0x40), 0, 487, image="mono-2.0-bdwgc.dll"),
           call(2, "VirtualAlloc", (0, 0x10000, 0x3000, 0x40), 0x1222ED0000, image="mono-2.0-bdwgc.dll"),
           call(3, "VirtualAlloc", (0, 0x56000, 0x3000, 4), 0x1222F00000, image="mono-2.0-bdwgc.dll")]
    assert call_compare.compare(windows, w2m, SIGNATURES) == ([], 1)


def test_a_value_two_windows_runs_disagree_on_is_not_compared() -> None:
    windows = [call(1, "TryEnterCriticalSection", (0x242F01BFB80,), 1)]
    w2m = [call(1, "TryEnterCriticalSection", (0x11804AA120,), 0)]
    assert len(call_compare.compare(windows, w2m, SIGNATURES)[0]) == 1
    undecided = call_compare.undetermined(json.loads(call_compare.CONTRACT.read_text()))
    assert ("TryEnterCriticalSection", "ret") in undecided
    assert call_compare.compare(windows, w2m, SIGNATURES, undecided=undecided)[0] == []


def test_a_pools_workers_pair_per_function_by_agreement() -> None:
    """Sprint 065: Windows' Job.Worker 1 made a HeapSize walk that w2m's Job.Worker 0 made, answer for answer; pairing
    by name compared the walk with Job.Worker 1's later one. Each function pairs a pool's workers by agreement, so each
    worker's own start-up calls stay with the worker of its name and the walk goes to the worker that made it."""
    def worker(seq: int, tid: int, name: str) -> list[call_compare.Call]:
        return [call(seq, "SetThreadDescription", ((1 << 64) - 2, 0x50000 + seq), 0, tid=tid),
                call(seq + 1, "SetThreadPriority", ((1 << 64) - 2, (-1) & 0xFFFFFFFF), 1, tid=tid)]
    def walk(seq: int, tid: int, sizes: list[int]) -> list[call_compare.Call]:
        return [call(seq + i, "HeapSize", (0x30000, 0, 0x40000 + i * 16), s, tid=tid) for i, s in enumerate(sizes)]
    names = ([(10, 1, "Job.Worker 0"), (20, 1, "Job.Worker 1")], [(10, 1, "Job.Worker 0"), (20, 1, "Job.Worker 1")])
    windows = worker(10, 200, "Job.Worker 0") + worker(20, 300, "Job.Worker 1") + walk(30, 300, [0x28, 0x422])
    w2m = worker(10, 8, "Job.Worker 0") + worker(20, 12, "Job.Worker 1") + walk(30, 8, [0x28, 0x422]) + \
        walk(40, 12, [0x34, 0x94e])
    assert call_compare.compare(windows, w2m, SIGNATURES, frozenset(), names)[0] == []
    wrong = worker(10, 8, "Job.Worker 0") + worker(20, 12, "Job.Worker 1") + walk(30, 8, [0x28, 0x423]) + \
        walk(40, 12, [0x34, 0x94e])
    found = call_compare.compare(windows, wrong, SIGNATURES, frozenset(), names)[0]
    assert [(d.function, d.field, d.w2m) for d in found] == [("HeapSize", "ret", 0x423)]
    assert call_compare.pools({1: "Job.Worker 0", 2: "Job.Worker 1", 3: "Loading.AsyncRead", 4: "Worker 7"}) == \
        {1: "Job.Worker", 2: "Job.Worker"}


def test_a_focus_messages_window_maps_to_the_same_window() -> None:
    """Sprint 065: WM_KILLFOCUS's and WM_SETFOCUS's wParam is a window (0x50150 on Windows, 0x20010 on w2m: the game's
    own); it maps one to one with the message functions' windows, so another window there is a divergence."""
    def msgs(window: int, focus: int) -> list[call_compare.Call]:
        return [call(1, "DefWindowProcW", (window, 0x08, window, 0), 0), call(2, "DefWindowProcW", (window, 0x07, focus, 0), 0)]
    assert call_compare.compare(msgs(0x50150, 0x50150), msgs(0x20010, 0x20010), SIGNATURES)[0] == []
    found = call_compare.compare(msgs(0x50150, 0x50150), msgs(0x20010, 0x30020), SIGNATURES)[0]
    assert [(d.function, d.index, d.field) for d in found] == [("DefWindowProcW", 1, "a2")]


def test_a_call_windows_makes_and_w2m_does_not_is_missing() -> None:
    # review of 2026-10-12, B1: the comparison walked w2m's calls only, and a Windows call with no partner went unseen
    windows = [call(1, "GetACP", (), 1252), call(2, "GetACP", (), 1252), call(3, "GetOEMCP", (), 437)]
    w2m = [call(1, "GetACP", (), 1252)]
    found, compared = call_compare.compare(windows, w2m, SIGNATURES)
    assert compared == 1
    assert [d.text() for d in found] == ["UnityPlayer.dll!GetACP call 1 missing: Windows 0x4e4, w2m none",
                                         "UnityPlayer.dll!GetOEMCP call 0 missing: Windows 0x1b5, w2m none"]
