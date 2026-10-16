# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""The whole-Windows copy's judge (Sprint 091): listing, tar's errors, and the verdict."""

import importlib.util
from pathlib import Path

_spec = importlib.util.spec_from_file_location(
    "windows_depot_manifest", Path(__file__).resolve().parent.parent / "scripts" / "windows_depot_manifest.py")
assert _spec and _spec.loader
m = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(m)


def test_listing_and_errors() -> None:
    assert m.read_listing("System32\\ntdll.dll\t2490368\nexplorer.exe\t5000\n") == {
        "System32\\ntdll.dll": 2490368, "explorer.exe": 5000}
    errors = ("tar: Couldn't open Windows/System32/config/BBI: Permission denied\n"   # the laptop's own line
              "tar: Error exit delayed from previous errors.\n")
    assert m.read_unreadable(errors) == {"System32\\config\\BBI": "Couldn't open: Permission denied"}


def test_judge_sorts_every_file() -> None:
    listing = {"a.dll": 10, "b.dll": 20, "c.dll": 30, "d.dll": 40}
    copied = {"a.dll": 10, "b.dll": 21, "e.log": 5}
    v = m.judge(listing, copied, {"c.dll": "Permission denied"})
    assert v == {"copied": ["a.dll"], "size_changed": ["b.dll"], "unreadable": ["c.dll"], "missing": ["d.dll"],
                 "new": ["e.log"]}
