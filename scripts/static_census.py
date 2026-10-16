# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""static_census.py — what every depot on disk declares, read without running anything (Sprint 085;
docs/recentering-2026-10-16-knowable-first.md, step 1).

For each depot under ~/w2m-depots it writes store/census/static/<depot>.json:

- `images`: every PE file (by its MZ and PE signatures): path in the depot, sha256, size, machine (0x8664 x86-64,
  0x14C x86), subsystem, whether it is a .NET assembly (a CLR runtime header), whether another image of the depot
  imports it by name (the depot ships it), and the reason when pe.parse refuses it;
- `imports`: each image's imports and delay imports from DLLs the depot does not ship, by DLL and name (or #ordinal);
- `named`: the Windows functions each image's bytes name as an ASCII string (a `GetProcAddress` candidate): strings
  equal to an export of a Windows DLL, from mingw-w64's import libraries (x86-64 and i686), less those it imports;
- `exports`: the names each image of the depot exports (the targets of its own `GetProcAddress` calls: mono's API);
- `served`: for every Windows function in `imports` or `named`, whether w2m serves it (vm_backend.service_for).

and prints a summary per depot. The output is sorted and has no timestamps, so a census of the same depots is the
same bytes.

Usage: uv run python scripts/static_census.py [--depots ~/w2m-depots] [--out store/census/static] [--judge]
  --judge: Hail to the Rainbow's census against the functions the game called in its Windows run
  (store/game-runs/38006665419/jit_capture.bin.imports.tsv, .dynamic.tsv), which no part of the census reads.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import struct
import sys
from collections import Counter, defaultdict
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))

from w2m.pe import PeError, parse  # noqa: E402
from w2m.vm_backend import service_for  # noqa: E402

DEPOTS = Path.home() / "w2m-depots"
OUT = ROOT / "store" / "census" / "static"
MINGW = Path("/opt/homebrew/opt/mingw-w64")
LIBS = {"x86-64": MINGW / "toolchain-x86_64" / "x86_64-w64-mingw32" / "lib",
        "x86": MINGW / "toolchain-i686" / "i686-w64-mingw32" / "lib"}
SUFFIXES = {".exe", ".dll", ".sys", ".ocx", ".drv", ".cpl", ".ax", ".acm", ".asi", ".flt", ".scr", ".efi"}
WINDOWS_RUN = ROOT / "store" / "game-runs" / "38006665419"
DIR_CLR = 14                                             # PE Format: the CLR runtime header's data directory
DLL_NAME = re.compile(rb"([A-Za-z0-9_.+-]+\.(?:dll|DLL|Dll|drv|DRV|exe|EXE|sys|SYS|ocx|OCX|cpl|CPL))\0")


def _archive_symbols(data: bytes) -> list[str]:
    """The symbol names of a GNU ar archive's index (its first member, named "/"): a big-endian count, the offsets,
    then the names, each ending in a NUL."""
    if not data.startswith(b"!<arch>\n") or data[8:9] != b"/":
        return []
    size = int(data[8 + 48:8 + 58].decode().strip())
    body = data[68:68 + size]
    (n,) = struct.unpack_from(">I", body, 0)
    return [s.decode("latin-1") for s in body[4 + 4 * n:].split(b"\0")[:n]]


def windows_exports() -> dict[str, dict[str, set[str]]]:
    """For x86-64 and x86, each Windows DLL (lower case) mingw-w64 has an import library for, and its exported names:
    the library's `__imp_` symbols (i686 names carry a leading underscore and a stdcall @n, both taken off)."""
    out: dict[str, dict[str, set[str]]] = {}
    for arch, lib in LIBS.items():
        table: dict[str, set[str]] = defaultdict(set)
        for path in sorted(lib.glob("lib*.a")):
            data = path.read_bytes()
            names = {s[6:] for s in _archive_symbols(data) if s.startswith("__imp_")}
            if arch == "x86":
                names = {n[1:] if n.startswith("_") else n for n in names}
                names = {n.split("@")[0] if re.fullmatch(r"\w+@\d+", n) else n for n in names}
            dlls = Counter(m.decode("latin-1").lower() for m in DLL_NAME.findall(data))
            if names and dlls:
                table[dlls.most_common(1)[0][0]] |= names
        out[arch] = dict(table)
    return out


def _clr(img: Any, data: bytes) -> bool:
    """A CLR runtime header in the data directories (a .NET assembly)."""
    (lfanew,) = struct.unpack_from("<I", data, 0x3C)
    opt = lfanew + 24
    at = opt + (112 if img.machine == 0x8664 else 96) + 8 * DIR_CLR
    return bool(at + 8 <= len(data) and struct.unpack_from("<I", data, at)[0] != 0)


def _machine(data: bytes) -> int | None:
    if data[:2] != b"MZ" or len(data) < 0x40:
        return None
    (lfanew,) = struct.unpack_from("<I", data, 0x3C)
    if data[lfanew:lfanew + 4] != b"PE\0\0":
        return None
    return int(struct.unpack_from("<H", data, lfanew + 4)[0])


STRING = re.compile(rb"[A-Za-z_][A-Za-z0-9_]{3,63}(?=\0)")


def census(depot: Path, exports: dict[str, dict[str, set[str]]]) -> dict[str, Any]:
    paths = sorted(p for p in depot.rglob("*") if p.is_file() and p.suffix.lower() in SUFFIXES)
    images: dict[str, dict[str, Any]] = {}
    parsed: dict[str, tuple[Any, bytes]] = {}
    for p in paths:
        data = p.read_bytes()
        machine = _machine(data)
        if machine is None:
            continue
        rel = str(p.relative_to(depot))
        entry: dict[str, Any] = {"sha256": hashlib.sha256(data).hexdigest(), "size": len(data), "machine": machine}
        try:
            img = parse(data, x86=True)
            entry.update(subsystem=img.subsystem, dotnet=_clr(img, data))
            parsed[rel] = (img, data)
        except PeError as e:
            entry["refused"] = str(e)
        images[rel] = entry
    shipped = {Path(r).name.lower() for r in images}
    imported_by_name = {dll.lower() for img, _ in parsed.values() for dll in (*img.imports, *img.delay_imports)}
    for rel, entry in images.items():
        entry["shipped"] = Path(rel).name.lower() in imported_by_name
    all_exports = {arch: {n for names in table.values() for n in names} for arch, table in exports.items()}
    imports: dict[str, dict[str, list[str]]] = {}
    named: dict[str, list[str]] = {}
    shipped_exports = {rel: sorted({n for n, _ in img.exports}) for rel, (img, _) in sorted(parsed.items())
                       if img.exports}            # every image on disk: one loaded by LoadLibrary is not "shipped"
    windows: set[tuple[str, str]] = set()
    for rel, (img, data) in sorted(parsed.items()):
        own: dict[str, list[str]] = {}
        for dll, fns in [*img.imports.items(), *img.delay_imports.items()]:
            if dll.lower() in shipped:
                continue
            own.setdefault(dll.lower(), []).extend(fns)
        imports[rel] = {d: sorted(set(f)) for d, f in sorted(own.items())}
        windows |= {(d, f) for d, fs in own.items() for f in fs}
        arch = "x86-64" if img.machine == 0x8664 else "x86"
        declared = {f for fs in own.values() for f in fs}
        names = sorted({s.decode() for s in STRING.findall(data)} & all_exports[arch] - declared)
        if names:
            named[rel] = names
    by_name = {n: d for arch in exports.values() for d, ns in arch.items() for n in ns}
    windows |= {(by_name.get(n, "?"), n) for ns in named.values() for n in ns}
    served = {f"{d}!{f}": service_for(d, f) is not None for d, f in sorted(windows)}
    return {"depot": depot.name, "images": images, "imports": imports, "named": named, "exports": shipped_exports,
            "served": served}


def summary(c: dict[str, Any]) -> str:
    machines = Counter(f"0x{e['machine']:x}" for e in c["images"].values())
    refused = sum(1 for e in c["images"].values() if "refused" in e)
    dotnet = sum(1 for e in c["images"].values() if e.get("dotnet"))
    imported = {f"{d}!{f}" for fs in c["imports"].values() for d, ns in fs.items() for f in ns}
    named = {n for ns in c["named"].values() for n in ns}
    served_imported = sum(1 for k in imported if c["served"].get(k))
    by_dll = Counter(k.split("!")[0] for k in imported)
    return (f"{c['depot']}: {len(c['images'])} images {dict(machines)}, {refused} refused, {dotnet} .NET; "
            f"{len(imported)} Windows functions imported, {served_imported} served; {len(named)} more named in "
            f"data; by DLL {by_dll.most_common(8)}")


def judge(c: dict[str, Any]) -> dict[str, Any]:
    """The functions Hail to the Rainbow called on Windows, each found in the census: an import of the module that
    called it; a name looked up at run time among the Windows functions some image names or imports, or among the
    exports of an image the depot ships. The residue is what no file on disk shows."""
    imported = {(Path(rel).name.lower(), d, f) for rel, fs in c["imports"].items() for d, ns in fs.items() for f in ns}
    named = ({n for ns in c["named"].values() for n in ns} | {f for _, _, f in imported}
             | {n for ns in c["exports"].values() for n in ns})
    missing_imports, missing_dynamic, calls = [], [], 0
    for line in (WINDOWS_RUN / "jit_capture.bin.imports.tsv").read_text().splitlines():
        module, dll, fn, _ = line.split("\t")
        calls += 1
        if (module.lower(), dll.lower(), fn) not in imported:
            missing_imports.append(f"{module}!{dll}!{fn}")
    dynamic = [n for n in (WINDOWS_RUN / "jit_capture.bin.dynamic.tsv").read_text().split() if n]
    missing_dynamic = [n for n in dynamic if n not in named]
    return {"imports": calls, "imports missing": missing_imports, "dynamic": len(dynamic),
            "dynamic missing": missing_dynamic}


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--depots", type=Path, default=DEPOTS)
    ap.add_argument("--out", type=Path, default=OUT)
    ap.add_argument("--judge", action="store_true")
    a = ap.parse_args(argv)
    exports = windows_exports()
    print("Windows exports from mingw-w64: " + ", ".join(
        f"{arch} {len(t)} DLLs {sum(len(n) for n in t.values())} names" for arch, t in exports.items()))
    a.out.mkdir(parents=True, exist_ok=True)
    for depot in sorted(p for p in a.depots.iterdir() if p.is_dir()):
        c = census(depot, exports)
        if not c["images"]:
            continue
        (a.out / f"{depot.name}.json").write_text(json.dumps(c, indent=1, sort_keys=True) + "\n")
        print(summary(c))
        if a.judge and depot.name == "hail-to-the-rainbow":
            j = judge(c)
            print(f"judge: {j['imports']} imports called on Windows, {len(j['imports missing'])} not in the census "
                  f"{j['imports missing'][:10]}; {j['dynamic']} looked up by name, {len(j['dynamic missing'])} not "
                  f"named in any image {j['dynamic missing'][:20]}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
