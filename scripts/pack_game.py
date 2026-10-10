# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""pack_game.py — pack an installed Steam depot for the Windows runner (Sprint 033).

Writes games/<name>/files.json (every installed file: path, size, sha256), then a zstd-compressed
tar of the same files split into parts under 2 GiB (GitHub's limit per release asset), and
games/<name>/release.json (each part's name, size, sha256). SteamCMD's own bookkeeping folder,
`steamapps/`, is not part of the depot and is left out.

Usage: uv run python scripts/pack_game.py <install_dir> <name> <app_id> <build_id> <out_dir>
"""

from __future__ import annotations

import hashlib
import json
import subprocess
import sys
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parent.parent
PART_BYTES = 1900 * 1024 * 1024
SKIP = "steamapps"


def sha256(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as f:
        while chunk := f.read(1 << 22):
            h.update(chunk)
    return h.hexdigest()


def main(argv: list[str]) -> int:
    install, name, app_id, build_id, out = Path(argv[0]), argv[1], argv[2], argv[3], Path(argv[4])
    files = sorted(p for p in install.rglob("*") if p.is_file()
                   and p.relative_to(install).parts[0] != SKIP)
    listing: list[dict[str, Any]] = [{"path": p.relative_to(install).as_posix(), "size": p.stat().st_size,
                "sha256": sha256(p)} for p in files]
    meta = ROOT / "games" / name
    meta.mkdir(parents=True, exist_ok=True)
    (meta / "files.json").write_text(json.dumps(
        {"app_id": int(app_id), "build_id": int(build_id), "files": len(listing),
         "bytes": sum(f["size"] for f in listing), "listing": listing}, indent=1) + "\n")
    out.mkdir(parents=True, exist_ok=True)
    stem = f"{name}-{build_id}.tar.zst.part-"
    names = "\n".join(f["path"] for f in listing) + "\n"
    tar = subprocess.Popen(["tar", "-cf", "-", "-C", str(install), "-T", "-"],
                           stdin=subprocess.PIPE, stdout=subprocess.PIPE)
    zstd = subprocess.Popen(["zstd", "-T0", "-6", "-q", "-c"], stdin=tar.stdout,
                            stdout=subprocess.PIPE)
    split = subprocess.Popen(["split", "-b", str(PART_BYTES), "-a", "2", "-", str(out / stem)],
                             stdin=zstd.stdout)
    tar.stdout.close()                                            # type: ignore[union-attr]
    zstd.stdout.close()                                           # type: ignore[union-attr]
    tar.communicate(names.encode())
    if (zstd.wait(), split.wait(), tar.returncode) != (0, 0, 0):
        print("packing failed", file=sys.stderr)
        return 1
    parts: list[dict[str, Any]] = [{"name": p.name, "size": p.stat().st_size, "sha256": sha256(p)}
             for p in sorted(out.glob(stem + "*"))]
    (meta / "release.json").write_text(json.dumps(
        {"tag": f"game-{app_id}-{build_id}", "parts": parts}, indent=1) + "\n")
    print(f"{len(listing)} files, {sum(f['size'] for f in listing)} bytes; "
          f"{len(parts)} parts, {sum(p['size'] for p in parts)} bytes")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
