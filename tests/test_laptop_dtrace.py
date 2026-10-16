# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""The laptop's DTrace traces (Sprint 092): ordering by timestamp, and E1's summary of a process's start."""

import gzip
import importlib.util
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
_spec = importlib.util.spec_from_file_location("laptop_dtrace", ROOT / "scripts" / "laptop_dtrace.py")
assert _spec and _spec.loader
m = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(m)

# E1's own lines (store/laptop-traces/process-start-hello.tsv.gz), given out of order as DTrace's per-CPU buffers do
LINES = ("E\t292380930496\t10144\tNtCreateFile\tb8a30ff2c0\t40100080\tb8a30ff348\tb8a30ff2f0\t0\t80\t1d800000003\t"
         "47ae00000005\t60\t0\r\n"
         "N\t292380930496\t10144\tNtCreateFile\t0\t\\??\\C:\\w2m-stage\\e1\\trace.txt\r\n"
         "E\t292379113399\t10144\tNtConnectPort\t7ffa6e194ee0\t7ffa6e194ec0\tb8a30fe430\tb8a30fe318\tb8a30fe348\t"
         "b8a30fe2fc\tb8a30fe398\tb8a30fe2f8\t0\t0\r\n"
         "N\t292379113399\t10144\tNtConnectPort\t0\t\\Windows\\ApiPort\r\n"
         "R\t292379159356\t10144\tNtConnectPort\t0\r\n")


def test_lines_come_back_in_timestamp_order() -> None:
    rows = m.ordered(LINES)
    assert [int(r[1]) for r in rows] == sorted(int(r[1]) for r in rows)
    assert rows[0][3] == "NtConnectPort" and rows[-1][3] == "NtCreateFile"


def test_the_summary_stops_at_main_and_names_the_port() -> None:
    s = m.summary(m.ordered(LINES), "\\trace.txt")
    assert s["calls_before_main"] == 1 and s["calls"] == 2
    assert s["ports"] == [("NtConnectPort", "\\Windows\\ApiPort")]


def test_the_stored_start_reproduces_its_counts() -> None:
    text = gzip.decompress((ROOT / "store" / "laptop-traces" / "process-start-hello.tsv.gz").read_bytes()).decode()
    header, _, body = text.partition("\n")
    assert "build 26200.7840" in header
    s = m.summary(m.ordered(body), "\\trace.txt")
    assert (s["calls"], s["distinct"], s["calls_before_main"], s["distinct_before_main"]) == (483, 73, 342, 62)
