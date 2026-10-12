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

from w2m.file_bridge import (FILE_LIST, FILE_LOOKUP, FILE_OPEN, FILE_READ, FILE_STAT, FILE_WRITE, IS_DIRECTORY, IS_FILE,  # noqa: E402
                             RECORDED_FOLDERS, FileBridge)
from w2m.kernel32 import Unimplemented  # noqa: E402


class Memory:
    def __init__(self) -> None:
        self.b = bytearray(1 << 21)

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


def test_unrecorded_names_and_creation_stop_the_run(bridge: FileBridge) -> None:
    mem = Memory()
    with pytest.raises(Unimplemented, match="recorded from the runners"):
        ask(bridge, mem, FILE_OPEN, "C:\\Windows\\Fonts\\new.ttf", 1)
    with pytest.raises(Unimplemented, match="listing is not recorded"):
        ask(bridge, mem, FILE_LOOKUP, "C:\\Windows\\System32\\kernel32.dll")
    with pytest.raises(Unimplemented, match="listing"):
        ask(bridge, mem, FILE_LIST, "C:\\Windows", 0x10000, 0x80000)


FONTS = json.loads(RECORDED_FOLDERS.read_text())["folders"]["Windows\\Fonts"]
COPY = Path(FONTS["contents"]["root"]).expanduser()


@pytest.mark.skipif(not COPY.is_dir(), reason="the runner's fonts are copied to ~/w2m-depots, not into CI")
def test_a_font_reads_as_the_runners_copy_and_refuses_writes(bridge: FileBridge) -> None:
    import hashlib
    mem = Memory()
    fd, _ = ask(bridge, mem, FILE_OPEN, "C:\\Windows\\Fonts\\CANDARA.TTF", 0)
    row = FONTS["files"]["Candara.ttf"]
    data = b""
    while len(data) < row["size"]:
        n, _ = bridge.request(mem, FILE_READ, [fd, 0x20000, 0x80000, len(data), 0, 0])
        data += mem.read(0x20000, n)
    assert hashlib.sha256(data).hexdigest() == row["sha256"]
    with pytest.raises(Unimplemented, match="write"):
        bridge.request(mem, FILE_WRITE, [fd, 0x20000, 4, 0, 0, 0])
