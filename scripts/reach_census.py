# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""reach_census.py — E2: what each game's imports reach through Microsoft's DLLs, in system calls and information
classes (Sprint 093; docs/solution-2026-10-16.md 4.2; the walk is src/w2m/reach.py).

Inputs: the copied C:\\Windows (~/w2m-depots/windows, Sprint 091); the census's imports and P/Invokes per image
(store/census/static/<game>.json); System Informer's phnt headers (~/w2m-depots/phnt) for which argument of each system
call selects among its kinds: a parameter named *Class, or InformationLevel, IoControlCode, FsControlCode,
FunctionCode, Command, SectionType or xpfnProc (NtUserCall*'s routine).

Roots, four sets, reported apart: `imports` (every image's imports, by name and ordinal); `pinvoke` (the managed
code's P/Invokes, bound as the CLR binds them: the exact name, then with W, then A); `loader` (ntdll's
LdrInitializeThunk and the KiUser* dispatchers, and each reached DLL's entry point and TLS callbacks, which the loader
calls); `lookups` (the census's run-time lookup candidates: a string equal to an export, in an image that imports
GetProcAddress, bound in each DLL its strings or imports name). Each set is reported twice: by direct edges, and
`with_pointers`, where every code address a reached function takes, every import it reads as a value, every code pointer
in a reached image's data (vtables, callback tables) and every run-time lookup a reached image can make (its strings
equal to exports, by the census's rule) is reached too (Confine's address-taken over-approximation; COM and the drivers'
callback tables are called only this way). Reach is a graph search over direct edges; classes come from the summaries (cycles cut, counted).

Writes store/census/reach/<game>.json. With --binary <exe> --arch x64|x86 it walks one image's imports instead (the
E1 judge's probe), printing the stubs reached.

Usage: uv run python scripts/reach_census.py [game ...] [--binary file.exe --arch x64]
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import subprocess
import sys
from collections import deque
from pathlib import Path

from w2m import reach
from w2m.pe import parse

ROOT = Path(__file__).resolve().parent.parent
DEPOTS = Path.home() / "w2m-depots"
WINDOWS = DEPOTS / "windows" / "Windows"
PHNT = DEPOTS / "phnt"
CENSUS = ROOT / "store" / "census" / "static"
OUT = ROOT / "store" / "census" / "reach"
GAMES = ("hail-to-the-rainbow", "bioshock2-remastered")
DISPATCH_NAMES = {"InformationLevel", "IoControlCode", "FsControlCode", "FunctionCode", "Command", "SectionType",
                  "xpfnProc"}
LOADER = ("LdrInitializeThunk", "KiUserExceptionDispatcher", "KiUserApcDispatcher", "KiUserCallbackDispatcher")


def class_arguments() -> dict[str, int]:
    """System call name -> the index of its class or dispatch argument, from phnt's prototypes."""
    text = "\n".join(p.read_text(errors="replace") for p in sorted(PHNT.glob("*.h")))
    text = re.sub(r"/\*.*?\*/", "", re.sub(r"//[^\n]*", "", text), flags=re.S)
    out: dict[str, int] = {}
    for m in re.finditer(r"NTSYSCALLAPI\s+[\w\s\*]+?\bNTAPI\s+((?:Nt|Zw)\w+)\s*\(([^;]*?)\)\s*;", text):
        name = "Nt" + m[1][2:]
        for i, p in enumerate(x.strip() for x in m[2].split(",")):
            words = re.sub(r"_\w+_(?:\([^)]*\))?", " ", p).split()
            if words and (words[-1].endswith("Class") or words[-1] in DISPATCH_NAMES):
                out.setdefault(name, i)
                break
    return out


def search(win: reach.Windows, roots: list[reach.NodeKey], counts: reach.Reach,
           taken: bool = False) -> set[reach.NodeKey]:
    """Every function reachable from `roots` by direct edges; DLL entry points and TLS callbacks join when their
    image is first reached. With `taken`, every code address a reached function takes (lea) and every code pointer in
    a reached image's data (vtables, callback tables) joins too: calls through pointers, over-approximated."""
    seen: set[reach.NodeKey] = set()
    images: set[tuple[str, str]] = set()
    queue = deque(roots)
    params: dict[reach.NodeKey, frozenset[int]] = {}
    while queue:
        key = queue.popleft()
        if key in seen:
            continue
        seen.add(key)
        if (key[0], key[1]) not in images:
            images.add((key[0], key[1]))
            image = win.image(key[0], key[1])
            if image is not None:
                queue.extend((key[0], key[1], r) for r in (image.pe.entry_rva, *image.pe.tls_callbacks)
                             if r and image.executable(r))
                if taken:
                    queue.extend((key[0], key[1], r) for r in sorted(win.data_code_pointers(image)))
                    for ref in win.lookups(image):
                        e = win.resolve(key[0], key[1], ref)
                        if not isinstance(e, str):
                            queue.append(e)
        if taken:
            queue.extend((key[0], key[1], r) for r in sorted(win.summary(key).taken))
            for ref in sorted(win.summary(key).imports_taken):
                e = win.resolve(key[0], key[1], ref)
                if not isinstance(e, str):
                    queue.append(e)
        counts.indirect += win.summary(key).indirect
        for e in reach.edges(win, key, params):
            if isinstance(e, str):
                counts.unresolved[e] = counts.unresolved.get(e, 0) + 1
            else:
                queue.append(e)
    counts.functions = len(seen)
    return seen


def stub_row(s: reach.Stub) -> str:
    return f"{s.table}:{s.number:#x}:{s.name}"


def walk(win: reach.Windows, groups: dict[str, list[tuple[str, reach.NodeKey]]],
         cls: dict[str, int]) -> dict[str, object]:
    out: dict[str, object] = {}
    memo: dict[reach.NodeKey, frozenset[tuple[reach.Stub, reach.Value]]] = {}
    for group, roots in groups.items():
        counts = reach.Reach()
        seen = search(win, [k for _, k in roots], counts)
        wide = search(win, [k for _, k in roots], reach.Reach(), taken=True)
        wide_stubs = sorted({s for s in (win.summary(k).stub for k in wide) if s is not None},
                            key=lambda s: (s.table, s.number))
        found = (win.summary(k).stub for k in seen)
        stubs = sorted({s for s in found if s is not None}, key=lambda s: (s.table, s.number))
        classes: dict[str, set[str]] = {}
        for label, key in roots:
            for stub, v in reach.effects(win, key, cls, memo, set(), counts):
                if stub.name not in cls:
                    continue
                value = ("unknown" if v is None else f"{int(v[1]):#x}" if v[0] == "const"
                         else f"caller argument {v[1]}" if v[0] == "param" else "unknown")
                classes.setdefault(f"{stub.name} {value}", set()).add(label)
        out[group] = {
            "roots": len(roots), "functions": counts.functions, "unresolved_indirect_sites": counts.indirect,
            "cycles_cut": counts.cycles_cut, "unresolved_imports": dict(sorted(counts.unresolved.items())),
            "nt": [stub_row(s) for s in stubs if s.table == "nt"],
            "win32k": [stub_row(s) for s in stubs if s.table == "win32k"],
            "classes": {k: len(v) for k, v in sorted(classes.items())},
            "with_pointers": {"functions": len(wide), "nt": [stub_row(s) for s in wide_stubs if s.table == "nt"],
                              "win32k": [stub_row(s) for s in wide_stubs if s.table == "win32k"]},
        }
    return out


def game_roots(win: reach.Windows, game: str) -> tuple[dict[str, list[tuple[str, reach.NodeKey]]], dict[str, int]]:
    census = json.loads((CENSUS / f"{game}.json").read_text())
    missing: dict[str, int] = {}
    groups: dict[str, list[tuple[str, reach.NodeKey]]] = {"imports": [], "pinvoke": [], "loader": []}
    arches: set[str] = set()
    for image, dlls in census["imports"].items():
        arch = "x86" if census["images"][image]["machine"] == 0x14C else "x64"
        arches.add(arch)
        for dll, names in dlls.items():
            if win.files[arch].get(dll.lower()) is None and not dll.lower().startswith(("api-", "ext-")):
                continue                                  # the game's own DLL or a redistributable
            for name in names:
                key = win.resolve(arch, Path(image).name.lower(), f"{dll}!{name}")
                if isinstance(key, str):
                    missing[key] = missing.get(key, 0) + 1
                else:
                    groups["imports"].append((f"{dll.lower()}!{name}", key))
    for image, calls in census["pinvoke"].items():
        arch = "x64" if "x64" in arches else "x86"
        for dll, name in calls:
            for spelled in (name, name + "W", name + "A"):
                key = win.resolve(arch, "", f"{dll}!{spelled}")
                if not isinstance(key, str):
                    groups["pinvoke"].append((f"{dll.lower()}!{spelled}", key))
                    break
            else:
                missing[f"P/Invoke {dll}!{name} binds nowhere"] = missing.get(f"P/Invoke {dll}!{name}", 0) + 1
    groups["lookups"] = []
    for image, names in census["named"].items():
        arch = "x86" if census["images"][image]["machine"] == 0x14C else "x64"
        candidates = sorted(set(census["strings"].get(image, {}).get("dlls", [])) | set(census["imports"][image]))
        for name in names:
            for dll in candidates:
                if win.files[arch].get(dll) is None and not dll.startswith(("api-", "ext-")):
                    continue
                key = win.resolve(arch, Path(image).name.lower(), f"{dll}!{name}")
                if not isinstance(key, str):
                    groups["lookups"].append((f"{dll}!{name}", key))
    for arch in sorted(arches):
        for name in LOADER:
            key = win.resolve(arch, "", f"ntdll.dll!{name}")
            if not isinstance(key, str):
                groups["loader"].append((f"ntdll.dll!{name}", key))
    return groups, missing


def judge_e3(game: str = "hail-to-the-rainbow") -> list[str]:
    """E3 as judge: every system call and win32k call the game made in E3's window (store/laptop-traces) that no
    root set of the walk reaches."""
    result = json.loads((OUT / f"{game}.json").read_text())
    reached = {row.split(":", 2)[2] for group in ("imports", "pinvoke", "lookups", "loader") for kind in ("nt", "win32k")
               for row in result[group]["with_pointers"][kind]}
    traced = ROOT / "store" / "laptop-traces" / "frame-calls-hail-to-the-rainbow.tsv"
    misses = []
    for line in traced.read_text().splitlines()[2:]:
        kind, call, _, _ = line.split("\t")
        name = "Nt" + call[2:] if call.startswith("Zw") else call
        if kind != "win32u export, not a system call" and name not in reached:
            misses.append(name)
    return misses


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("games", nargs="*", default=list(GAMES))
    ap.add_argument("--binary", type=Path)
    ap.add_argument("--arch", choices=["x64", "x86"], default="x64")
    ap.add_argument("--judge", action="store_true", help="check E3's traced calls against the walk")
    a = ap.parse_args(argv)
    win = reach.Windows(WINDOWS)
    cls = class_arguments()
    if a.binary:
        pe = parse(a.binary.read_bytes(), x86=a.arch == "x86")
        roots = []
        for dll, names in pe.imports.items():
            for name in names:
                key = win.resolve(a.arch, a.binary.name.lower(), f"{dll}!{name}")
                if not isinstance(key, str):
                    roots.append((f"{dll.lower()}!{name}", key))
        loader = [(n, k) for n in LOADER if not isinstance(k := win.resolve(a.arch, "", f"ntdll.dll!{n}"), str)]
        print(json.dumps(walk(win, {"imports": roots, "loader": loader}, cls), indent=1))
        return 0
    stamp = {"windows": "26200.7840 (store/census/windows-manifest.tsv.gz)",
             "phnt": subprocess.run(["git", "-C", str(PHNT), "rev-parse", "HEAD"], capture_output=True,
                                    text=True).stdout.strip(),
             "script_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
             "reach_sha256": hashlib.sha256((ROOT / "src" / "w2m" / "reach.py").read_bytes()).hexdigest()}
    OUT.mkdir(parents=True, exist_ok=True)
    if a.judge:
        misses = judge_e3()
        print(f"E3 judge: {len(misses)} traced calls not reached: {misses}")
        return 1 if misses else 0
    for game in a.games:
        groups, missing = game_roots(win, game)
        result = {"game": game, **stamp, "roots_unbound": dict(sorted(missing.items())), **walk(win, groups, cls)}
        (OUT / f"{game}.json").write_text(json.dumps(result, indent=1, sort_keys=True) + "\n")
        for group in groups:
            g = result[group]
            assert isinstance(g, dict)
            print(f"{game} {group}: {g['roots']} roots, {g['functions']} functions, {len(g['nt'])} nt and "
                  f"{len(g['win32k'])} win32k calls, {len(g['classes'])} (call, class) rows, "
                  f"{g['unresolved_indirect_sites']} indirect sites, {sum(g['unresolved_imports'].values())} "
                  f"unresolved import edges")
    return 0


if __name__ == "__main__":
    sys.exit(main())
