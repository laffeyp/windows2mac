# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""static_census.py — what every depot on disk declares, read without running anything (Sprint 085;
docs/recentering-2026-10-16-knowable-first.md, step 1; made exact in Sprint 088, the large review of 082-086).

Windows's side comes from Windows itself: the Windows laptop's System32 (x86-64) and SysWOW64 (x86) DLLs, copied whole
to ~/w2m-depots/windows-system, give every DLL's exports (`--windows` writes them to store/census/windows-exports.json.gz
with the build), and its registry's HKCR\\Interface and HKCR\\CLSID exports give every registered COM interface and
class by GUID (store/census/windows-com.json.gz).

For each depot under ~/w2m-depots it writes store/census/static/<depot>.json:

- `images`: every PE file (by its MZ and PE signatures): path in the depot, sha256, size, machine (0x8664 x86-64,
  0x14C x86), subsystem, whether it is a .NET assembly, whether another image of the depot imports it by name, and the
  reason when pe.parse refuses it;
- `imports`: each image's imports and delay imports from DLLs the depot does not ship, by DLL and name (or #ordinal);
- `dlls`: each DLL imported, P/Invoked or named, classed: `windows` (on the laptop, not a redistributable),
  `redistributable` (a Microsoft runtime a game ships or installs: REDISTRIBUTABLE), `shipped`, or `unknown`;
- `pinvoke`: each .NET assembly's [DllImport]s, (DLL, entry point), from its ImplMap and ModuleRef tables;
- `named`: in a native image that imports GetProcAddress, the ASCII strings equal to a Windows export, less those it
  imports (a candidate run-time lookup);
- `com`: the registered COM interfaces and classes whose GUIDs are in each image's bytes (4-byte aligned);
- `strings`: Steam interface versions ("SteamUser021") and DLL names in each image's bytes;
- `exports`: the names each image of the depot exports (the targets of its own GetProcAddress calls: mono's API);
- `served`: for every Windows or unknown function the census names, whether w2m serves it (vm_backend.service_for);
- `source`: the export and COM tables' Windows build and sha256, and this script's commit.

and prints a summary per depot. The output is sorted and has no timestamps.

Usage: uv run python scripts/static_census.py [--depots ~/w2m-depots] [--out store/census/static] [--judge]
       uv run python scripts/static_census.py --windows ~/w2m-depots/windows-system   (the Windows tables, once)
  --judge: Hail to the Rainbow's census against the functions the game called in its Windows run
  (store/game-runs/38006665419/jit_capture.bin.imports.tsv, .dynamic.tsv), which no part of the census reads.
"""

from __future__ import annotations

import argparse
import gzip
import hashlib
import json
import re
import struct
import subprocess
import sys
import uuid
from collections import Counter, defaultdict
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))

from w2m.clr_metadata import pinvokes  # noqa: E402
from w2m.pe import PeError, parse  # noqa: E402
from w2m.vm_backend import service_for  # noqa: E402

DEPOTS = Path.home() / "w2m-depots"
OUT = ROOT / "store" / "census" / "static"
EXPORTS = ROOT / "store" / "census" / "windows-exports.json.gz"
COM = ROOT / "store" / "census" / "windows-com.json.gz"
SUFFIXES = {".exe", ".dll", ".sys", ".ocx", ".drv", ".cpl", ".ax", ".acm", ".asi", ".flt", ".scr", ".efi"}
WINDOWS_RUN = ROOT / "store" / "game-runs" / "38006665419"
DIR_CLR = 14                                             # PE Format: the CLR runtime header's data directory
WINDOWS_DEPOT = "windows-system"                         # Windows's own DLLs: a source, not a game
# Microsoft's redistributable runtimes (Microsoft Learn: "Latest supported Visual C++ Redistributable downloads"; the
# DirectX End-User Runtime's d3dx, d3dcompiler, xinput1_x, XAudio2_x and X3DAudio): a game ships or installs them, so
# w2m can load them as PE files where Windows's own DLLs are services
REDISTRIBUTABLE = re.compile(r"^(msvcp\d+.*|msvcr\d+.*|vcruntime\d+.*|concrt\d+|vcomp\d+.*|vccorlib\d+|mfc\d+.*|"
                             r"d3dx\d+_\d+|d3dx(9|10|11)\w*|d3dcompiler_\d+|xinput1_[1-4]|xinput9_1_0|xaudio2_[0-7]|"
                             r"x3daudio1_\d+|xapofx1_\d+)\.dll$", re.I)
DLL_STRING = re.compile(rb"[A-Za-z0-9_.+-]{2,60}\.(?:dll|DLL|Dll)(?=\0)")
DLL_STRING16 = re.compile(rb"(?:[A-Za-z0-9_.+-]\0){2,60}\.\0(?:d\0l\0l\0|D\0L\0L\0)(?=\0\0)")
STEAM_INTERFACE = re.compile(rb"(?:Steam|STEAM)[A-Za-z]+\d{3}(?=\0)")
STRING = re.compile(rb"[A-Za-z_][A-Za-z0-9_]{3,63}(?=\0)")


# Windows's tables ----------------------------------------------------------------------------------------------
def build_windows_tables(system: Path) -> None:
    """From a copy of the laptop's System32 and SysWOW64 DLLs and its registry exports (interface.reg, clsid.reg, which
    `reg export` writes in UTF-16), the committed export and COM tables."""
    exports: dict[str, dict[str, list[str]]] = {}
    for arch, folder in (("x86-64", "System32"), ("x86", "SysWOW64")):
        table: dict[str, list[str]] = {}
        for p in sorted((system / folder).glob("*.dll")):
            try:
                img = parse(p.read_bytes(), x86=True)
            except PeError:
                continue
            if img.exports:
                table[p.name.lower()] = sorted({n for n, _ in img.exports})
        exports[arch] = table
    build = " ".join((system / "build.txt").read_text().split())
    EXPORTS.write_bytes(_gzip(json.dumps({"build": build, "exports": exports}, sort_keys=True)))
    com: dict[str, dict[str, str]] = {}
    for kind, name in (("interfaces", "interface.reg"), ("classes", "clsid.reg")):
        text = (system / name).read_bytes().decode("utf-16")
        # every GUID key, named by its default value where it has one (an unnamed interface is still Windows's)
        found = re.findall(r"^\[HKEY_CLASSES_ROOT\\(?:Interface|CLSID)\\(\{[0-9A-Fa-f-]{36}\})\]\r?\n(?:@=\"([^\"\r\n]*)\")?",
                           text, re.M)
        com[kind] = {g.upper(): n for g, n in sorted(found)}
    COM.write_bytes(_gzip(json.dumps({"build": build, **com}, sort_keys=True)))


def _gzip(text: str) -> bytes:
    import io
    buf = io.BytesIO()
    with gzip.GzipFile(fileobj=buf, mode="wb", mtime=0, filename="") as g:
        g.write(text.encode())
    return buf.getvalue()


def windows_tables() -> tuple[dict[str, Any], dict[str, Any], dict[str, str]]:
    """(the export table, the COM table, their sources: build and sha256)."""
    exports = json.loads(gzip.decompress(EXPORTS.read_bytes()))
    com = json.loads(gzip.decompress(COM.read_bytes()))
    return exports, com, {"windows_build": exports["build"],
                          "exports_sha256": hashlib.sha256(EXPORTS.read_bytes()).hexdigest(),
                          "com_sha256": hashlib.sha256(COM.read_bytes()).hexdigest()}


# images --------------------------------------------------------------------------------------------------------
def _clr(img: Any, data: bytes) -> bool:
    """A CLR runtime header in the data directories (a .NET assembly)."""
    (lfanew,) = struct.unpack_from("<I", data, 0x3C)
    at = lfanew + 24 + (112 if img.machine == 0x8664 else 96) + 8 * DIR_CLR
    return bool(at + 8 <= len(data) and struct.unpack_from("<I", data, at)[0] != 0)


def _machine(data: bytes) -> int | None:
    if data[:2] != b"MZ" or len(data) < 0x40:
        return None
    (lfanew,) = struct.unpack_from("<I", data, 0x3C)
    if data[lfanew:lfanew + 4] != b"PE\0\0":
        return None
    return int(struct.unpack_from("<H", data, lfanew + 4)[0])


def _guids(data: bytes, known: dict[bytes, str]) -> list[str]:
    """The known GUIDs (as their 16 bytes in memory order) at 4-byte aligned offsets of `data`."""
    found = set()
    for at in range(0, len(data) - 15, 4):
        name = known.get(data[at:at + 16])
        if name is not None:
            found.add(name)
    return sorted(found)


def census(depot: Path, windows: dict[str, Any], com: dict[str, Any], source: dict[str, str]) -> dict[str, Any]:
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
    arch_names = {arch: {n for names in table.values() for n in names} for arch, table in windows["exports"].items()}
    exporters: dict[str, set[str]] = defaultdict(set)
    for table in windows["exports"].values():
        for dll, names in table.items():
            for n in names:
                exporters[n].add(dll)
    known_guids = {uuid.UUID(g).bytes_le: f"interface {n or g}" for g, n in com["interfaces"].items()}
    known_guids.update({uuid.UUID(g).bytes_le: f"class {n or g}" for g, n in com["classes"].items()})
    imports: dict[str, dict[str, list[str]]] = {}
    named: dict[str, list[str]] = {}
    pinvoke: dict[str, list[tuple[str, str]]] = {}
    found_com: dict[str, list[str]] = {}
    strings: dict[str, dict[str, list[str]]] = {}
    functions: set[tuple[str, str]] = set()
    for rel, (img, data) in sorted(parsed.items()):
        own: dict[str, list[str]] = {}
        for dll, fns in [*img.imports.items(), *img.delay_imports.items()]:
            if dll.lower() in shipped:
                continue
            own.setdefault(dll.lower(), []).extend(fns)
        imports[rel] = {d: sorted(set(f)) for d, f in sorted(own.items())}
        functions |= {(d, f) for d, fs in own.items() for f in fs}
        if images[rel]["dotnet"]:
            try:
                pinvoke[rel] = sorted({(d.lower(), f) for d, f in pinvokes(data, img)})
            except PeError as e:
                images[rel]["pinvoke_refused"] = str(e)
            continue                                     # a .NET image's strings are C# names, not lookups (M1)
        arch = "x86-64" if img.machine == 0x8664 else "x86"
        declared = {f for fs in own.values() for f in fs}
        if any("GetProcAddress" in fs for fs in own.values()):
            names = sorted({s.decode() for s in STRING.findall(data)} & arch_names[arch] - declared)
            if names:
                named[rel] = names
        guids = _guids(data, known_guids)
        if guids:
            found_com[rel] = guids
        dlls = {m.decode("latin-1").lower() for m in DLL_STRING.findall(data)}
        dlls |= {m.decode("utf-16-le").lower() for m in DLL_STRING16.findall(data)}
        steam = sorted({m.decode() for m in STEAM_INTERFACE.findall(data)})
        entry_strings = {k: v for k, v in (("dlls", sorted(dlls)), ("steam_interfaces", steam)) if v}
        if entry_strings:
            strings[rel] = entry_strings
    for rel, pairs in pinvoke.items():
        functions |= {(d if d.endswith(".dll") else d + ".dll", f) for d, f in pairs}
    functions |= {(sorted(exporters.get(n, {"?"}))[0], n) for ns in named.values() for n in ns}
    on_windows = set(windows["exports"]["x86-64"]) | set(windows["exports"]["x86"])

    def klass(dll: str) -> str:
        name = dll.lower() if dll.lower().endswith(".dll") else dll.lower() + ".dll"
        if name in shipped:
            return "shipped"
        if REDISTRIBUTABLE.match(name):
            return "redistributable"
        if name in on_windows or name.startswith(("api-ms-win-", "ext-ms-win-")):
            return "windows"
        return "unknown"

    dll_class = {d: klass(d) for d in sorted({d for d, _ in functions})}
    served = {f"{d}!{f}": service_for(d, f) is not None for d, f in sorted(functions)
              if dll_class[d] in ("windows", "unknown")}
    shipped_exports = {rel: sorted({n for n, _ in img.exports}) for rel, (img, _) in sorted(parsed.items())
                       if img.exports}
    return {"depot": depot.name, "source": source, "images": images, "imports": imports, "dlls": dll_class,
            "pinvoke": {r: [list(p) for p in ps] for r, ps in pinvoke.items()}, "named": named, "com": found_com,
            "strings": strings, "exports": shipped_exports, "served": served}


def summary(c: dict[str, Any]) -> str:
    machines = Counter(f"0x{e['machine']:x}" for e in c["images"].values())
    dotnet = sum(1 for e in c["images"].values() if e.get("dotnet"))
    imported = {(d, f) for fs in c["imports"].values() for d, ns in fs.items() for f in ns}
    by_class: Counter[str] = Counter()
    served_by_class: Counter[str] = Counter()
    for d, f in imported:
        k = c["dlls"].get(d, "unknown")
        by_class[k] += 1
        served_by_class[k] += bool(c["served"].get(f"{d}!{f}"))
    pinv = sum(len(v) for v in c["pinvoke"].values())
    com = {g for gs in c["com"].values() for g in gs}
    return (f"{c['depot']}: {len(c['images'])} images {dict(machines)}, {dotnet} .NET; {len(imported)} functions "
            f"imported by class {dict(by_class)}, served {dict(served_by_class)}; {pinv} P/Invokes; "
            f"{len({n for v in c['named'].values() for n in v})} candidate run-time lookups (a string equal to a Windows "
            f"export, in an image that imports GetProcAddress: candidates, not calls); {len(com)} COM GUIDs")


def judge(c: dict[str, Any]) -> dict[str, Any]:
    """The functions Hail to the Rainbow called on Windows, each found in the census: an import of the module that
    called it; a name looked up at run time among the names some image imports, names, P/Invokes or exports. The
    residue is what no file on disk shows."""
    imported = {(Path(rel).name.lower(), d, f) for rel, fs in c["imports"].items() for d, ns in fs.items() for f in ns}
    named = ({n for ns in c["named"].values() for n in ns} | {f for _, _, f in imported}
             | {n for ns in c["exports"].values() for n in ns} | {f for ps in c["pinvoke"].values() for _, f in ps})
    missing_imports = []
    calls = 0
    for line in (WINDOWS_RUN / "jit_capture.bin.imports.tsv").read_text().splitlines():
        module, dll, fn, _ = line.split("\t")
        calls += 1
        if (module.lower(), dll.lower(), fn) not in imported:
            missing_imports.append(f"{module}!{dll}!{fn}")
    dynamic = [n for n in (WINDOWS_RUN / "jit_capture.bin.dynamic.tsv").read_text().split() if n]
    return {"imports": calls, "imports missing": missing_imports, "dynamic": len(dynamic),
            "dynamic missing": [n for n in dynamic if n not in named]}


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--depots", type=Path, default=DEPOTS)
    ap.add_argument("--out", type=Path, default=OUT)
    ap.add_argument("--judge", action="store_true")
    ap.add_argument("--windows", type=Path, help="build the Windows tables from this copy of System32 and SysWOW64")
    a = ap.parse_args(argv)
    if a.windows:
        build_windows_tables(a.windows)
        return 0
    windows, com, source = windows_tables()
    commit = subprocess.run(["git", "-C", str(ROOT), "log", "-1", "--format=%H", "--", "scripts/static_census.py"],
                            capture_output=True, text=True).stdout.strip()
    source = {**source, "script_commit": commit}
    print(f"Windows build {source['windows_build']}: " + ", ".join(
        f"{arch} {len(t)} DLLs {sum(len(n) for n in t.values())} names" for arch, t in windows["exports"].items())
          + f"; {len(com['interfaces'])} interfaces, {len(com['classes'])} classes")
    a.out.mkdir(parents=True, exist_ok=True)
    for depot in sorted(p for p in a.depots.iterdir() if p.is_dir() and p.name != WINDOWS_DEPOT):
        c = census(depot, windows, com, source)
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
