# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""file_bridge.py — the host side of the guest runtime's file system (Sprint 042).

The guest runtime (`tools/guest_runtime/file.c`) implements the Windows file calls: dispositions,
sharing, error codes, name rules, file positions. It reaches the Mac only through the primitives
here, on the transport page (hvc #8), all under one root directory per run that stands in for the
guest's working directory:

    FILE_LOOKUP  (path, chars)          -> kind: 0 absent, 1 file, 2 directory, 3 a directory on the way is missing;
                                           arg[5] the file id (inode) for 1 and 2
    FILE_OPEN    (path, chars, create)  -> host descriptor, or -errno; create=1 is O_CREAT|O_EXCL; arg[5] the file id
    FILE_CLOSE   (fd)                   -> 0
    FILE_READ    (fd, buffer, n, offset)-> bytes read (pread)
    FILE_WRITE   (fd, buffer, n, offset)-> bytes written (pwrite)
    FILE_SIZE    (fd)                   -> size
    FILE_TRUNCATE(fd, size)             -> 0
    FILE_UNLINK  (path, chars)          -> 0, or -errno

Paths arrive as UTF-16LE as the guest normalized them. Components compare ASCII case-folded, as
NTFS compares ASCII names (its upcase table; non-ASCII folding is not needed by any recorded row
and is not done). An absolute path, a drive letter or a component that leaves the root is
Unimplemented: the run stops rather than guess.
"""

from __future__ import annotations

import errno
import os
import stat
from pathlib import Path

from w2m.kernel32 import Unimplemented
from typing import Protocol

FILE_LOOKUP, FILE_OPEN, FILE_CLOSE, FILE_READ, FILE_WRITE, FILE_SIZE, FILE_TRUNCATE, FILE_UNLINK = range(6, 14)
ABSENT, IS_FILE, IS_DIRECTORY, PARENT_MISSING = 0, 1, 2, 3
M64 = (1 << 64) - 1


class BridgeRequestError(RuntimeError):
    """The guest runtime sent a request no Windows path or descriptor can mean (a NUL inside a
    path, an unknown descriptor): a runtime defect; the run stops with this message."""


class GuestMemory(Protocol):
    def read(self, va: int, n: int) -> bytes: ...
    def write(self, va: int, data: bytes) -> None: ...


class FileBridge:
    """POSIX primitives under `root` for one guest process."""

    def __init__(self, root: Path) -> None:
        self.root = root.resolve()
        self.fds: dict[int, int] = {}            # guest-visible descriptor -> host fd
        self.next_fd = 3

    def fold(self, name: str) -> str:
        """The form two names are compared in: ASCII upper case folded to lower."""
        return name.translate({c: c + 32 for c in range(ord("A"), ord("Z") + 1)})

    def _path(self, mem: GuestMemory, va: int, chars: int) -> list[str]:
        name = mem.read(va, 2 * chars).decode("utf-16-le", errors="replace")
        if "\0" in name:
            raise BridgeRequestError(f"path of {chars} characters has a NUL at character {name.index(chr(0))}: {name!r}")
        if name.startswith(("\\", "/")) or (len(name) > 1 and name[1] == ":"):
            raise Unimplemented(f"file path outside the working directory: {name!r}")
        parts = [p for p in name.replace("/", "\\").split("\\") if p not in ("", ".")]
        if ".." in parts:
            raise Unimplemented(f"file path with '..': {name!r}")
        return parts

    def _resolve(self, parts: list[str]) -> tuple[int, Path]:
        """(kind, host path): each existing component is matched ASCII case-folded."""
        here = self.root
        for i, part in enumerate(parts):
            if not here.is_dir():
                return PARENT_MISSING, here / part
            folded = self.fold(part)
            match = next((e for e in os.listdir(here) if self.fold(e) == folded), None)
            if match is None:
                return (ABSENT if i == len(parts) - 1 else PARENT_MISSING), here / part
            here = here / match
        return (IS_DIRECTORY if here.is_dir() else IS_FILE), here

    def request(self, mem: GuestMemory, op: int, arg: list[int]) -> tuple[int, int]:
        """Serve one primitive; returns (result, arg[5])."""
        if op == FILE_LOOKUP:
            kind, path = self._resolve(self._path(mem, arg[0], arg[1]))
            return kind, (os.stat(path).st_ino if kind in (IS_FILE, IS_DIRECTORY) else 0)
        if op == FILE_OPEN:
            kind, path = self._resolve(self._path(mem, arg[0], arg[1]))
            flags = os.O_RDWR | (os.O_CREAT | os.O_EXCL if arg[2] else 0)
            try:
                host = os.open(path, flags, 0o644)
            except OSError as e:
                return -(e.errno or errno.EIO) & M64, 0
            fd = self.next_fd
            self.next_fd += 1
            self.fds[fd] = host
            return fd, os.fstat(host).st_ino
        if op == FILE_UNLINK:
            kind, path = self._resolve(self._path(mem, arg[0], arg[1]))
            try:
                os.unlink(path)
            except OSError as e:
                return -(e.errno or errno.EIO) & M64, 0
            return 0, 0
        if arg[0] not in self.fds:
            raise BridgeRequestError(f"file primitive {op} on descriptor {arg[0]}, which is not open")
        host = self.fds[arg[0]]
        if op == FILE_CLOSE:
            os.close(self.fds.pop(arg[0]))
            return 0, 0
        if op == FILE_READ:
            data = os.pread(host, arg[2], arg[3])
            mem.write(arg[1], data)
            return len(data), 0
        if op == FILE_WRITE:
            return os.pwrite(host, mem.read(arg[1], arg[2]), arg[3]), 0
        if op == FILE_SIZE:
            st = os.fstat(host)
            assert stat.S_ISREG(st.st_mode)
            return st.st_size, 0
        if op == FILE_TRUNCATE:
            os.ftruncate(host, arg[1])
            return 0, 0
        raise ValueError(f"unknown file primitive {op}")

    def close_all(self) -> None:
        for host in self.fds.values():
            os.close(host)
        self.fds.clear()
