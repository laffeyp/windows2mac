# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""windows_depot_manifest.py — the whole-Windows copy judged against the laptop's own listing (Sprint 091;
docs/solution-2026-10-16.md, section 3).

The depot ~/w2m-depots/windows holds the laptop's C:\\Windows (streamed by its tar.exe), its SOFTWARE and SYSTEM
hives (reg save), `listing.tsv` (every file's path under C:\\Windows and its size, written on the laptop before the
copy), `unreadable.txt` (tar's errors on the laptop), and `not-copied.txt` (path TAB cause: listed files neither copied
nor named by tar, each cause measured on the laptop). This script:

- writes store/census/windows-manifest.tsv.gz: every copied file's path (backslashes, as on Windows), size and sha256,
  sorted, with the build on its first line;
- judges the copy: each listed file is in the depot, or tar named it unreadable, or not-copied.txt gives its cause;
  files whose size changed during the copy (live logs) and files new since the listing are counted apart;
- checks the hives' `regf` signature;
- compares the first copy (~/w2m-depots/windows-system: System32 and SysWOW64, Sprint 088) with this one by sha256.

Usage: uv run python scripts/windows_depot_manifest.py [--depot ~/w2m-depots/windows]
                                                       [--old ~/w2m-depots/windows-system]
"""

from __future__ import annotations

import argparse
import gzip
import hashlib
import os
import re
import sys
from concurrent.futures import ProcessPoolExecutor
from pathlib import Path

MANIFEST = Path(__file__).resolve().parent.parent / "store" / "census" / "windows-manifest.tsv.gz"
TAR_ERROR = re.compile(r"^tar(?:\.exe)?: Couldn't open (Windows[\\/].*): (.*)$")   # the laptop's bsdtar, 2026-10-16


def read_listing(text: str) -> dict[str, int]:
    """listing.tsv: `path<TAB>size` per line, the path relative to C:\\Windows."""
    out: dict[str, int] = {}
    for line in text.splitlines():
        if line:
            path, size = line.rsplit("\t", 1)
            out[path] = int(size)
    return out


def read_unreadable(text: str) -> dict[str, str]:
    """tar's error lines: the path (relative to C:\\Windows, backslashes) and the error."""
    out: dict[str, str] = {}
    for line in text.splitlines():
        m = TAR_ERROR.match(line.strip())
        if m:
            out[m.group(1).replace("/", "\\").split("\\", 1)[1]] = "Couldn't open: " + m.group(2)
    return out


def judge(listing: dict[str, int], copied: dict[str, int], unreadable: dict[str, str]) -> dict[str, list[str]]:
    """Every listed file copied at its size, or named unreadable; the rest sorted by kind."""
    out: dict[str, list[str]] = {"copied": [], "unreadable": [], "missing": [], "size_changed": [], "new": []}
    for path, size in listing.items():
        if path in copied:
            out["copied" if copied[path] == size else "size_changed"].append(path)
        else:
            out["unreadable" if path in unreadable else "missing"].append(path)
    out["new"] = [p for p in copied if p not in listing]
    return out


def _hash(path: str) -> tuple[str, int, str]:
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for block in iter(lambda: f.read(1 << 20), b""):
            h.update(block)
    return path, os.path.getsize(path), h.hexdigest()


def hash_tree(root: Path) -> dict[str, tuple[int, str]]:
    """Every regular file under root: relative path with backslashes -> (size, sha256)."""
    files = [str(p) for p in root.rglob("*") if p.is_file() and not p.is_symlink()]
    out: dict[str, tuple[int, str]] = {}
    with ProcessPoolExecutor() as pool:
        for path, size, digest in pool.map(_hash, files, chunksize=64):
            out[os.path.relpath(path, root).replace("/", "\\")] = (size, digest)
    return out


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--depot", type=Path, default=Path.home() / "w2m-depots" / "windows")
    ap.add_argument("--old", type=Path, default=Path.home() / "w2m-depots" / "windows-system")
    a = ap.parse_args()
    build = " ".join((a.depot / "build.txt").read_text().split("\n")[:4])
    listing = read_listing((a.depot / "listing.tsv").read_text(encoding="utf-8-sig"))
    unreadable = read_unreadable((a.depot / "unreadable.txt").read_text(errors="replace"))
    not_copied = a.depot / "not-copied.txt"
    if not_copied.exists():
        unreadable.update(dict(line.split("\t", 1) for line in not_copied.read_text().splitlines() if line))
    tree = hash_tree(a.depot / "Windows")
    verdict = judge(listing, {p: s for p, (s, _) in tree.items()}, unreadable)
    lines = [f"# {build}"] + [f"{p}\t{s}\t{d}" for p, (s, d) in sorted(tree.items())]
    MANIFEST.write_bytes(gzip.compress(("\n".join(lines) + "\n").encode(), mtime=0))
    print(f"listed {len(listing)}, in the depot {len(tree)} ({sum(s for s, _ in tree.values()):,} bytes); "
          + ", ".join(f"{k} {len(v)}" for k, v in verdict.items()))
    for kind in ("missing", "size_changed", "unreadable", "new"):
        for p in sorted(verdict[kind])[:15]:
            print(f"  {kind}: {p}" + (f"  ({unreadable[p]})" if kind == "unreadable" else ""))
    for hive in ("SOFTWARE", "SYSTEM"):
        print(f"hive {hive}: {'regf' if (a.depot / 'hives' / hive).read_bytes()[:4] == b'regf' else 'NOT A HIVE'}")
    old = {f"{top}\\{p}": v for top in ("System32", "SysWOW64") for p, v in hash_tree(a.old / top).items()}
    differ = [p for p, (_, d) in old.items() if tree.get(p, (0, ""))[1] != d]
    print(f"first copy: {len(old)} files, {len(old) - len(differ)} equal by sha256, {len(differ)} not")
    for p in sorted(differ)[:15]:
        print(f"  not equal: {p}" + ("" if p in tree else " (absent from the new copy)"))
    return 1 if verdict["missing"] or differ else 0


if __name__ == "__main__":
    sys.exit(main())
