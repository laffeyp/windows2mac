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
    FILE_STAT    (path, chars, out)     -> kind as FILE_LOOKUP; for a file or directory, 32 bytes at out: the size,
                                           then the birth, access and modification times as FILETIME (Sprint 047)
    FILE_FSTAT   (fd, out)              -> 0; 40 bytes at out: FILE_STAT's 32, then the change time (st_ctime) as FILETIME
                                           (Sprint 049: GetFileInformationByHandleEx asks by handle, so a name deleted
                                           or renamed since the open still answers)
    FILE_DUP     (fd)                   -> a second descriptor on the same open file (Sprint 049: a file mapping holds
                                           one, so its views outlive the file's handle)
    FILE_MKDIR   (path, chars)          -> 0, or -errno (Sprint 048)
    FILE_RMDIR   (path, chars)          -> 0, or -errno (Sprint 048)
    FILE_LIST    (path, chars, out, size)-> bytes the directory's names take, each UTF-16 with a NUL, then one more NUL,
                                           written at out when they fit in size; arg[5] the kind as FILE_LOOKUP (Sprint
                                           054: FindFirstFileExW; the guest matches, sorts and adds "." and "..")

Paths arrive as UTF-16LE as the guest normalized them: full paths ("C:\\w2m\\app\\a.txt", tools/guest_runtime/paths.c)
or, as before Sprint 048, names relative to the working directory. Drive C: is a host directory per run holding
Users\\runneradmin (the profile, with AppData\\Local\\Temp, LocalLow and Roaming), Users\\w2m\\AppData\\Local\\Temp and w2m\\app, where `root` (the working
directory: the probe's, or the game's depot) is mounted; a read-only mount refuses every write by name. The folders
specs/recorded_folders.json names (C:\\Windows and C:\\Windows\\Fonts, Sprint 065) answer as the runners' do: each
entry's attributes (in FILE_STAT's arg[5]; 0 elsewhere, where the guest derives them), size and times, and the
folder's listing where one is recorded, and a file's bytes from the folder's copy (its `contents`, read-only and
checked against the row's sha256); a name not recorded in a folder whose listing is not, and any change inside them,
stop the run. Components compare ASCII case-folded, as NTFS compares
ASCII names (its upcase table; non-ASCII folding is not needed by any recorded row and is not done). Another drive, a UNC name, or a ".." the guest
did not resolve is Unimplemented: the run stops rather than guess.
"""

from __future__ import annotations

import errno
import os
import stat
from pathlib import Path

from w2m.kernel32 import Unimplemented
from typing import Any, Protocol

FILE_LOOKUP, FILE_OPEN, FILE_CLOSE, FILE_READ, FILE_WRITE, FILE_SIZE, FILE_TRUNCATE, FILE_UNLINK = range(6, 14)
FILE_STAT, FILE_MKDIR, FILE_RMDIR, FILE_FSTAT, FILE_DUP = 24, 25, 26, 27, 28   # after the host requests 14-23
FILE_LIST = 32                                                              # after the host requests 29-31
FILETIME_UNIX_EPOCH = 116444736000000000   # 1970-01-01 in 100 ns units since 1601-01-01
ABSENT, IS_FILE, IS_DIRECTORY, PARENT_MISSING = 0, 1, 2, 3
# C:\Users\runneradmin, the runners' user (Sprint 061), and the AppData folders SHGetKnownFolderPath names, each an
# existing directory on Windows (contracts/windows.shell.known_folder_path); fresh each run, like the rest of drive C:;
# and C:\Users\w2m\AppData\Local\Temp, where USER_ENVIRONMENT's TMP and TEMP point (vm_backend.py) as the reference
# game run's did (windows-game.yml)
PROFILE_FOLDERS = ("Users/runneradmin/AppData/LocalLow", "Users/runneradmin/AppData/Local/Temp",
                   "Users/runneradmin/AppData/Roaming", "Users/w2m/AppData/Local/Temp")
M64 = (1 << 64) - 1
RECORDED_FOLDERS = Path(__file__).resolve().parents[2] / "specs" / "recorded_folders.json"


def _recorded_folders() -> dict[tuple[str, ...], dict[str, Any]]:
    """specs/recorded_folders.json's folders by their folded components from C:\\."""
    import json
    spec = json.loads(RECORDED_FOLDERS.read_text())
    return {tuple(part.lower() for part in name.split("\\")): row for name, row in spec["folders"].items()}


class BridgeRequestError(RuntimeError):
    """The guest runtime sent a request no Windows path or descriptor can mean (a NUL inside a
    path, an unknown descriptor): a runtime defect; the run stops with this message."""


def birth_ns(path: Path | int, st: os.stat_result) -> int:
    """A file's birth time in exact nanoseconds: libc stat's st_birthtimespec (macOS arm64 struct stat: the access,
    modification, change and birth timespecs at 32, 48, 64 and 80). Python 3.12 gives it only as float seconds, about
    120 ns off at this date, coarser than FILETIME's 100 ns (a run read an access time before the creation time). The
    access and modification times at those offsets must equal os.stat's, or the layout is not the one assumed. An int
    is a host descriptor (fstat)."""
    import ctypes
    import struct as st_struct
    buf = ctypes.create_string_buffer(256)
    libc = ctypes.CDLL(None)
    if (libc.fstat(path, buf) if isinstance(path, int) else libc.stat(os.fsencode(path), buf)) != 0:
        raise Unimplemented(f"stat of {path} failed")
    a, m, b = (st_struct.unpack_from("<qq", buf.raw, off) for off in (32, 48, 80))
    if a[0] * 10**9 + a[1] != st.st_atime_ns or m[0] * 10**9 + m[1] != st.st_mtime_ns:
        raise Unimplemented("struct stat's layout is not macOS arm64's")
    return int(b[0]) * 10**9 + int(b[1])


class GuestMemory(Protocol):
    def read(self, va: int, n: int) -> bytes: ...
    def write(self, va: int, data: bytes) -> None: ...


class FileBridge:
    """POSIX primitives under `root` for one guest process."""

    def __init__(self, root: Path, read_only: bool = False, drive: Path | None = None) -> None:
        import tempfile
        self.recorded = _recorded_folders()      # folded components -> the runners' entry and listing
        self.root = root.resolve()               # C:\\w2m\\app, the working directory
        self.read_only = read_only               # the game's depot: nothing under it may change
        if drive is None:
            self._drive: tempfile.TemporaryDirectory[str] | None = tempfile.TemporaryDirectory(prefix="w2m-drive-c-")
            self.drive = Path(self._drive.name).resolve()   # C:\\, holding w2m (app is the mount)
            (self.drive / "w2m").mkdir()
            for folder in PROFILE_FOLDERS:       # the user's profile (known_folders.c, Sprint 048)
                (self.drive / folder).mkdir(parents=True)
            for parts in self.recorded:          # each recorded folder exists; its entries are the recording's
                self.drive.joinpath(*(p.capitalize() for p in parts)).mkdir(parents=True, exist_ok=True)
        else:                                    # another process of the same machine: the same drive (Sprint 055)
            self._drive = None
            self.drive = drive
        self.fds: dict[int, int] = {}            # guest-visible descriptor -> host fd
        self.read_only_fds: set[int] = set()     # descriptors of files under the mount (refused writes when read-only)
        self.recorded_fds: dict[int, dict[str, Any]] = {}   # descriptors of recorded files -> their rows (no writes)
        self.checked: set[Path] = set()          # recorded copies whose sha256 has been checked this run
        self.next_fd = 3

    def _under_mount(self, path: Path) -> bool:
        return path == self.root or self.root in path.parents

    def _writable(self, path: Path) -> None:
        if self.read_only and self._under_mount(path):
            raise Unimplemented(f"a change under the read-only C:\\w2m\\app: {path}")
        if self._recorded_parent(path) is not None:
            raise Unimplemented(f"a change in a folder recorded from the runners: {path}")

    def _components(self, path: Path) -> tuple[str, ...] | None:
        """A host path's folded components from C:\\, or None outside the drive (the mount is not recorded)."""
        if self._under_mount(path) or not (path == self.drive or self.drive in path.parents):
            return None
        return tuple(self.fold(p) for p in path.relative_to(self.drive).parts)

    def _recorded_parent(self, path: Path) -> dict[str, Any] | None:
        """The recorded folder `path` lies in, or None."""
        parts = self._components(path)
        return self.recorded.get(parts[:-1]) if parts else None

    def _open_recorded(self, path: Path, row: dict[str, Any], create: bool) -> tuple[int, int]:
        """A recorded file's bytes: the folder's copy (its `contents`), read-only, its sha256 the row's."""
        parts = self._components(path)
        assert parts is not None
        folder = self.recorded[parts[:-1]]
        if create or "contents" not in folder or "sha256" not in row:
            raise Unimplemented(f"the contents of a file recorded from the runners by name only: {path}")
        root = Path(os.path.expanduser(folder["contents"]["root"]))
        name = next(n for n, r in folder["files"].items() if r is row)
        copy = root / name
        if copy not in self.checked:
            import hashlib
            if not copy.is_file() or hashlib.sha256(copy.read_bytes()).hexdigest() != row["sha256"]:
                raise Unimplemented(f"the copy of {name} at {copy} is missing or not the runner's (sha256 {row['sha256']})")
            self.checked.add(copy)
        host = os.open(copy, os.O_RDONLY)
        fd = self.next_fd
        self.next_fd += 1
        self.fds[fd] = host
        self.recorded_fds[fd] = row
        return fd, int.from_bytes(name.lower().encode()[:7], "little") | 1 << 60

    def _recorded_row(self, kind: int, path: Path) -> tuple[int, dict[str, int] | None]:
        """(kind, the recording's row) for a path in or of a recorded folder; (kind, None) elsewhere."""
        parts = self._components(path)
        if parts is None:
            return kind, None
        for i in range(len(parts)):                       # below a folder recorded without its listing, only the
            above = self.recorded.get(parts[:i])          # folders recorded in it are known
            if above is not None and above["files"] is None and parts[:i + 1] not in self.recorded:
                raise Unimplemented(f"a name in a folder whose listing is not recorded: {path}")
        folder = self.recorded.get(parts)
        if folder is not None:
            return IS_DIRECTORY, folder["entry"]
        parent = self.recorded.get(parts[:-1])
        if parent is None or kind == PARENT_MISSING:
            return kind, None
        files = parent["files"]
        row = next((r for n, r in files.items() if self.fold(n) == parts[-1]), None)
        return (IS_FILE, row) if row is not None else (ABSENT, None)

    def fold(self, name: str) -> str:
        """The form two names are compared in: ASCII upper case folded to lower."""
        return name.translate({c: c + 32 for c in range(ord("A"), ord("Z") + 1)})

    def _path(self, mem: GuestMemory, va: int, chars: int) -> list[str]:
        """The components from C:\\ (a full path), or from C:\\w2m\\app (a relative name)."""
        name = mem.read(va, 2 * chars).decode("utf-16-le", errors="replace")
        if "\0" in name:
            raise BridgeRequestError(f"path of {chars} characters has a NUL at character {name.index(chr(0))}: {name!r}")
        return self._parts(name)

    def host_file(self, name: str) -> Path | None:
        """The host path of the file a guest name means (full, or relative to C:\\w2m\\app), or None when no file
        is there: LoadLibrary's search of the program's directory (Sprint 049)."""
        kind, path = self._resolve(self._parts(name))
        return path if kind == IS_FILE else None

    def _parts(self, name: str) -> list[str]:
        if name.startswith("\\\\?\\"):
            name = name[4:]
        if len(name) > 1 and name[1] == ":":
            if name[0] not in "cC" or (len(name) > 2 and name[2] not in "\\/"):
                raise Unimplemented(f"file path on a drive other than C: or drive-relative: {name!r}")
            prefix: list[str] = []
            name = name[3:]
        elif name.startswith(("\\", "/")):
            raise Unimplemented(f"file path rooted without a drive, or UNC: {name!r}")
        else:
            prefix = ["w2m", "app"]
        parts = [p for p in name.replace("/", "\\").split("\\") if p not in ("", ".")]
        if ".." in parts:
            raise Unimplemented(f"file path with '..': {name!r}")
        return prefix + parts

    def _resolve(self, parts: list[str]) -> tuple[int, Path]:
        """(kind, host path): each existing component is matched ASCII case-folded; C:\\w2m\\app is `root`."""
        here = self.drive
        for i, part in enumerate(parts):
            if here == self.drive / "w2m" and self.fold(part) == "app":
                here = self.root
                continue
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
            kind, row = self._recorded_row(kind, path)
            if row is not None and kind == IS_FILE:        # no host file: an id from the name, stable within the run
                return kind, int.from_bytes(path.name.lower().encode()[:7], "little") | 1 << 60
            return kind, (os.stat(path).st_ino if kind in (IS_FILE, IS_DIRECTORY) else 0)
        if op == FILE_OPEN:
            kind, path = self._resolve(self._path(mem, arg[0], arg[1]))
            kind, row = self._recorded_row(kind, path)
            if row is not None:
                return self._open_recorded(path, row, bool(arg[2]))
            if arg[2]:
                self._writable(path)
            flags = (os.O_RDONLY if self._under_mount(path) and self.read_only else os.O_RDWR) | (
                os.O_CREAT | os.O_EXCL if arg[2] else 0)
            try:
                host = os.open(path, flags, 0o644)
            except OSError as e:
                return -(e.errno or errno.EIO) & M64, 0
            fd = self.next_fd
            self.next_fd += 1
            self.fds[fd] = host
            if self._under_mount(path):
                self.read_only_fds.add(fd)
            return fd, os.fstat(host).st_ino
        if op == FILE_STAT:
            kind, path = self._resolve(self._path(mem, arg[0], arg[1]))
            kind, row = self._recorded_row(kind, path)
            if row is not None:
                mem.write(arg[2], b"".join(row[k].to_bytes(8, "little") for k in ("size", "creation", "access", "write")))
                return kind, row["attributes"]
            if kind in (IS_FILE, IS_DIRECTORY):
                st = os.stat(path)
                times = [t // 100 + FILETIME_UNIX_EPOCH for t in (birth_ns(path, st), st.st_atime_ns, st.st_mtime_ns)]
                mem.write(arg[2], b"".join(v.to_bytes(8, "little") for v in [st.st_size, *times]))
            return kind, 0
        if op == FILE_LIST:
            kind, path = self._resolve(self._path(mem, arg[0], arg[1]))
            if kind != IS_DIRECTORY:
                return 0, kind
            folder = self.recorded.get(self._components(path) or ())
            if folder is not None and folder["files"] is None:
                raise Unimplemented(f"the listing of a folder recorded without one: {path}")
            entries = set(folder["files"]) if folder is not None else set(os.listdir(path))
            if path == self.drive / "w2m":               # the mount _resolve finds there is listed too (review of 054)
                entries = {e for e in entries if self.fold(e) != "app"} | {"app"}
            names = b"".join(e.encode("utf-16-le") + b"\0\0" for e in sorted(entries)) + b"\0\0"
            if len(names) <= arg[3]:
                mem.write(arg[2], names)
            return len(names), kind
        if op in (FILE_MKDIR, FILE_RMDIR):
            kind, path = self._resolve(self._path(mem, arg[0], arg[1]))
            if kind == PARENT_MISSING:          # _resolve's path is then the first missing component, not the name
                return -errno.ENOENT & M64, 0
            if op == FILE_MKDIR and kind in (IS_FILE, IS_DIRECTORY):
                return -errno.EEXIST & M64, 0
            self._writable(path)
            try:
                (os.mkdir if op == FILE_MKDIR else os.rmdir)(path)
            except OSError as e:
                return -(e.errno or errno.EIO) & M64, 0
            return 0, 0
        if op == FILE_UNLINK:
            kind, path = self._resolve(self._path(mem, arg[0], arg[1]))
            self._writable(path)
            try:
                os.unlink(path)
            except OSError as e:
                return -(e.errno or errno.EIO) & M64, 0
            return 0, 0
        if arg[0] not in self.fds:
            raise BridgeRequestError(f"file primitive {op} on descriptor {arg[0]}, which is not open")
        host = self.fds[arg[0]]
        if op == FILE_CLOSE:
            self.recorded_fds.pop(arg[0], None)
            os.close(self.fds.pop(arg[0]))
            return 0, 0
        if op == FILE_READ:
            data = os.pread(host, arg[2], arg[3])
            mem.write(arg[1], data)
            return len(data), 0
        if op == FILE_WRITE:
            if arg[0] in self.recorded_fds:
                raise Unimplemented("a write to a file recorded from the runners")
            if self.read_only and arg[0] in self.read_only_fds:
                raise Unimplemented("a write to a file under the read-only C:\\w2m\\app")
            return os.pwrite(host, mem.read(arg[1], arg[2]), arg[3]), 0
        if op == FILE_DUP:
            fd = self.next_fd
            self.next_fd += 1
            self.fds[fd] = os.dup(host)
            if arg[0] in self.recorded_fds:
                self.recorded_fds[fd] = self.recorded_fds[arg[0]]
            if arg[0] in self.read_only_fds:
                self.read_only_fds.add(fd)
            return fd, 0
        if op == FILE_FSTAT and arg[0] in self.recorded_fds:     # the runner's times, not the copy's
            row = self.recorded_fds[arg[0]]
            mem.write(arg[1], b"".join(row[k].to_bytes(8, "little") for k in ("size", "creation", "access", "write", "write")))
            return 0, 0
        if op == FILE_FSTAT:
            st = os.fstat(host)
            times = [t // 100 + FILETIME_UNIX_EPOCH
                     for t in (birth_ns(host, st), st.st_atime_ns, st.st_mtime_ns, st.st_ctime_ns)]
            mem.write(arg[1], b"".join(v.to_bytes(8, "little") for v in [st.st_size, *times]))
            return 0, 0
        if op == FILE_SIZE:
            st = os.fstat(host)
            assert stat.S_ISREG(st.st_mode)
            return st.st_size, 0
        if op == FILE_TRUNCATE:
            if arg[0] in self.recorded_fds:
                raise Unimplemented("a truncation of a file recorded from the runners")
            if self.read_only and arg[0] in self.read_only_fds:
                raise Unimplemented("a truncation of a file under the read-only C:\\w2m\\app")
            os.ftruncate(host, arg[1])
            return 0, 0
        raise ValueError(f"unknown file primitive {op}")

    def close_all(self) -> None:
        for host in self.fds.values():
            os.close(host)
        self.fds.clear()
        self.read_only_fds.clear()
        if self._drive is not None:              # the drive's owner removes it; another process's bridge shares it
            self._drive.cleanup()
