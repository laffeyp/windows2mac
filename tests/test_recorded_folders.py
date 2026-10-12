# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""The file bridge's folders recorded from the runners (specs/recorded_folders.json; Sprint 065)."""

from __future__ import annotations

import json
import struct
import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))

from w2m.file_bridge import (FILE_LIST, FILE_LOOKUP, FILE_OPEN, FILE_STAT, IS_DIRECTORY, IS_FILE,  # noqa: E402
                             RECORDED_FOLDERS, FileBridge)
from w2m.kernel32 import Unimplemented  # noqa: E402


class Memory:
    def __init__(self) -> None:
        self.b = bytearray(1 << 20)

    def read(self, va: int, n: int) -> bytes:
        return bytes(self.b[va:va + n])

    def write(self, va: int, data: bytes) -> None:
        self.b[va:va + len(data)] = data


@pytest.fixture
def bridge(tmp_path: Path):  # type: ignore[no-untyped-def]
    fb = FileBridge(tmp_path, read_only=True)
    yield fb
    fb.close_all()


def ask(fb: FileBridge, mem: Memory, op: int, name: str, *rest: int) -> tuple[int, int]:
    mem.write(0x1000, name.encode("utf-16-le"))
    return fb.request(mem, op, [0x1000, len(name), *rest, 0, 0, 0][:6])


def test_the_font_folder_answers_as_the_runner_listed_it(bridge: FileBridge) -> None:
    mem, spec = Memory(), json.loads(RECORDED_FOLDERS.read_text())
    fonts = spec["folders"]["Windows\\Fonts"]
    assert len(fonts["files"]) == 346                       # probe run 38222954208: 348 entries with . and ..
    assert ask(bridge, mem, FILE_STAT, "C:\\Windows\\Fonts", 0x8000) == (IS_DIRECTORY, 0x15)
    assert ask(bridge, mem, FILE_STAT, "C:\\Windows", 0x8000) == (IS_DIRECTORY, 0x10)
    assert ask(bridge, mem, FILE_STAT, "c:\\windows\\fonts\\STATICCACHE.DAT", 0x8000) == (IS_FILE, 0x27)
    row = fonts["files"]["StaticCache.dat"]
    assert struct.unpack_from("<4Q", mem.b, 0x8000) == (row["size"], row["creation"], row["access"], row["write"])
    assert ask(bridge, mem, FILE_LOOKUP, "C:\\Windows\\Fonts\\missing.ttf")[0] == 0
    n, kind = ask(bridge, mem, FILE_LIST, "C:\\Windows\\Fonts", 0x10000, 0x80000)
    names = mem.read(0x10000, n).decode("utf-16-le").split("\0")
    assert kind == IS_DIRECTORY and set(names) - {""} == set(fonts["files"])


def test_contents_and_unrecorded_names_stop_the_run(bridge: FileBridge) -> None:
    mem = Memory()
    with pytest.raises(Unimplemented, match="by name only"):
        ask(bridge, mem, FILE_OPEN, "C:\\Windows\\Fonts\\arial.ttf", 0)
    with pytest.raises(Unimplemented, match="listing is not recorded"):
        ask(bridge, mem, FILE_LOOKUP, "C:\\Windows\\System32\\kernel32.dll")
    with pytest.raises(Unimplemented, match="listing"):
        ask(bridge, mem, FILE_LIST, "C:\\Windows", 0x10000, 0x80000)
