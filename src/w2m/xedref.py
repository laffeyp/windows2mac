"""xedref.py — Intel XED as the primary reference decoder (decision 2026-10-08,
docs/review-2026-10-08-decoder-theory.md).

Batch interface to `build/xedshim` (tools/xedshim/xedshim.c linked against third_party/xed,
v2026.08.23, 64-bit long mode). One subprocess call decodes any number of byte strings.
"""

from __future__ import annotations

import subprocess
from collections.abc import Sequence
from dataclasses import dataclass
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SHIM = ROOT / "build" / "xedshim"
SHIM_SRC = ROOT / "tools" / "xedshim" / "xedshim.c"
XED_KIT_GLOB = "third_party/xed/kits/xed-install-base-*"


@dataclass(frozen=True)
class XedResult:
    ok: bool
    length: int = 0
    iclass: str = ""
    iform: str = ""
    text: str = ""
    error: str = ""
    operands: tuple[str, ...] = ()


def build_shim() -> Path:
    """Compile the shim against the installed XED kit if it is missing or older than its source."""
    kits = sorted(ROOT.glob(XED_KIT_GLOB))
    if not kits:
        raise FileNotFoundError("XED kit not built: run `python3 mfile.py --ar=$(xcrun --find ar) "
                                "install` in third_party/xed")
    if SHIM.exists() and SHIM.stat().st_mtime >= SHIM_SRC.stat().st_mtime:
        return SHIM
    SHIM.parent.mkdir(exist_ok=True)
    kit = kits[-1]
    subprocess.run(["clang", "-O2", "-Wall", "-Werror", f"-I{kit / 'include'}", "-o", str(SHIM),
                    str(SHIM_SRC), str(kit / "lib" / "libxed.a")], check=True)
    return SHIM


def decode_many(items: Sequence[bytes], chip: str = "ALL") -> list[XedResult]:
    """Decode each byte string (first 15 bytes used) with XED for `chip` (an XED chip name such as
    ALL, SKYLAKE, or AMD_ZEN4 — decoding depends on CPU features); one result per input, in order."""
    shim = build_shim()
    out = subprocess.run([str(shim), chip], input="".join(b[:15].hex() + "\n" for b in items),
                         capture_output=True, text=True, check=True).stdout.splitlines()
    if len(out) != len(items):
        raise RuntimeError(f"xedshim returned {len(out)} lines for {len(items)} inputs")
    results = []
    for line in out:
        f = line.split("\t")
        if f[0] == "ok":
            ops = tuple(f[5].split(";")) if len(f) > 5 and f[5] else ()
            results.append(XedResult(True, int(f[1]), f[2], f[3], f[4] if len(f) > 4 else "",
                                     operands=ops))
        else:
            results.append(XedResult(False, error=f[1] if len(f) > 1 else "unknown"))
    return results
