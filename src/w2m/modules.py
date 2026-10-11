# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""modules.py — which module a name means, for the guest runtime's module calls (Sprint 044).

The guest runtime (`tools/guest_runtime/module.c`) implements LoadLibraryExW, GetModuleHandleW,
GetModuleHandleExW, GetModuleFileNameW, GetProcAddress and FreeLibrary as Windows answers them
(contracts/windows.module.queries: error codes, truncation, what success leaves in the last error).
It asks the host only what w2m's process holds:

  the image             its handle is its base (PEB.ImageBaseAddress); its path C:\\w2m\\app\\<name>
  its DLLs              each DLL the loader mapped from the program's directory (Sprint 046): handle its
                        base, path C:\\w2m\\app\\<name>, GetProcAddress its exports
  kernel32, kernelbase  synthetic images in the process area, one page each with DOS and PE headers;
                        their export names are Windows's (specs/windows_exports.json, Server 2025)
  system DLLs           each other system DLL an image imports statically (SHELL32, USER32, ...; Sprint 048): a
                        synthetic image of one page, loaded from the start as on Windows (contracts/windows.module.
                        system: LoadLibraryW and GetModuleHandleW find it under any spelling, its path is
                        C:\\Windows\\System32\\<the import's spelling>, GetProcAddress gives the import's own thunk for an
                        imported name, a thunk per name for another export, NULL and 127 for a name it does not export)
  ntdll                 a synthetic image too (it is in every process's loader list, contracts/
                        windows.loader.static_import); its exports are not listed, so GetProcAddress on it stops
  api-ms-win-core-*     kernelbase, as Windows maps api-ms-win-core-synch-l1-2-0 (the modules rows)
  anything else         a stop by name (Unimplemented) when Windows would find the module: a DLL of
                        System32 or of the program's directory, or an API set w2m has not observed;
                        ERROR_MOD_NOT_FOUND only for a name found nowhere

GetProcAddress returns a thunk, as imports do: the program's own import thunk when it imports that
function from kernel32 (Windows returns the import's address), otherwise a thunk per module and
name, so kernel32's and kernelbase's answers differ as they do on Windows. Calling a thunk whose
function w2m does not serve stops the run naming it.
"""

from __future__ import annotations

import json
from collections.abc import Callable
from dataclasses import dataclass, field
from pathlib import Path

from w2m.kernel32 import Unimplemented

ROOT = Path(__file__).resolve().parents[2]
EXPORTS = ROOT / "specs" / "windows_exports.json"
APP_DIRECTORY = "C:\\w2m\\app\\"


def _canonical(name: str) -> str:
    """Lowercase, the last path component, '.dll' appended when there is no extension (LoadLibrary)."""
    base = name.replace("/", "\\").rsplit("\\", 1)[-1].lower()
    return base if "." in base else base + ".dll"


@dataclass
class Modules:
    image_base: int
    image_size: int
    image_name: str
    kernel32: int                    # synthetic image VAs (vm_backend)
    kernelbase: int
    image_exports: dict[str, int]    # name -> VA
    imports: dict[str, int]          # "kernel32.dll!name" (lowercased dll) -> import thunk
    make_thunk: Callable[[str], int]  # "KERNEL32.dll!Name" -> a new thunk with its bridge
    ntdll: int = 0
    dlls: dict[str, tuple[int, int, dict[str, int]]] = field(default_factory=dict)   # canonical name -> base, size, exports
    system32: frozenset[str] = frozenset()
    in_app_directory: Callable[[str], bool] = lambda name: False
    _thunks: dict[str, int] = field(default_factory=dict)

    dll_names: dict[str, str] = field(default_factory=dict)   # canonical name -> the name as loaded
    system: dict[str, tuple[int, str]] = field(default_factory=dict)   # canonical name -> handle, the import's spelling
    load_dll: Callable[[str], int] | None = None    # LoadLibrary of a DLL file after the start: its base, or 0 (Sprint 049)
    dll_paths: dict[str, str] = field(default_factory=dict)   # canonical name -> full path, for DLLs not beside the program

    def __post_init__(self) -> None:
        spec = json.loads(EXPORTS.read_text())
        self.exports = {m: frozenset(names) for m, names in spec["modules"].items()}

    @property
    def image_path(self) -> str:
        return APP_DIRECTORY + self.image_name

    def module(self, name: str, loading: bool) -> int:
        """The handle `name` means (0: found nowhere). `loading`: LoadLibrary, which would load a
        DLL not yet in the process; GetModuleHandle only reports loaded ones."""
        c = _canonical(name)
        if c in ("kernel32.dll",):
            return self.kernel32
        if c in ("kernelbase.dll",):
            return self.kernelbase
        if c == self.image_name.lower():
            return self.image_base
        if c.startswith("api-ms-win-core-"):
            return self.kernelbase
        if c.startswith(("api-ms-", "ext-ms-")):
            raise Unimplemented(f"API set {name}: its host module on Windows is not observed")
        if c == "ntdll.dll":
            if not self.ntdll:
                raise Unimplemented("ntdll.dll: loaded in every Windows process, not modeled")
            return self.ntdll
        if c in self.dlls:
            return self.dlls[c][0]
        if c in self.system:
            return self.system[c][0]
        if not loading:
            return 0                                    # not loaded in a process of only these modules
        if self.load_dll is not None:
            h = self.load_dll(name)                     # a file the name finds: mapped, bound, its DllMain pending
            if h:
                return h
        if c in self.system32 or self.in_app_directory(c):
            raise Unimplemented(f"LoadLibrary {name}: a Windows DLL w2m does not model")
        return 0

    def file_name(self, handle: int) -> str | None:
        if handle == self.image_base:
            return self.image_path
        if handle == self.kernel32:
            return "C:\\Windows\\System32\\KERNEL32.DLL"
        if handle == self.kernelbase:
            return "C:\\Windows\\System32\\KERNELBASE.dll"
        for name, (base, _, _) in self.dlls.items():
            if handle == base:
                return self.dll_paths.get(name, APP_DIRECTORY + self.dll_names[name])
        for h, spelling in self.system.values():
            if handle == h:
                return "C:\\Windows\\System32\\" + spelling
        return None

    def containing(self, address: int) -> int:
        if self.image_base <= address < self.image_base + self.image_size:
            return self.image_base
        for h in (self.kernel32, self.kernelbase, self.ntdll, *(h for h, _ in self.system.values())):
            if h and h <= address < h + 0x1000:
                return h
        for base, size, _ in self.dlls.values():
            if base <= address < base + size:
                return base
        return 0

    def proc(self, handle: int, name: str) -> int:
        """GetProcAddress: a thunk, or 0 when the module does not export `name`."""
        if handle == self.image_base:
            return self.image_exports.get(name, 0)
        for base, _, exports in self.dlls.values():
            if handle == base:
                return exports.get(name, 0)
        for c, (h, spelling) in self.system.items():
            if handle == h:
                stem = c[:-4]                                # "shell32"
                if name not in self.exports.get(stem, frozenset()):
                    return 0
                if f"{c}!{name.lower()}" in self.imports:   # Windows answers the import's address (proc.imported)
                    return self.imports[f"{c}!{name.lower()}"]
                key = f"{spelling}!{name}"
                if key not in self._thunks:
                    self._thunks[key] = self.make_thunk(key)
                return self._thunks[key]
        module = {self.kernel32: "kernel32", self.kernelbase: "kernelbase"}.get(handle)
        if handle and handle == self.ntdll:
            module = "ntdll"                            # its exports: specs/windows_exports.json 0.3.0 (Sprint 049)
        if module is None:
            raise Unimplemented(f"GetProcAddress on module 0x{handle:x}, which w2m did not hand out")
        if name not in self.exports[module]:
            return 0
        if module == "kernel32" and f"kernel32.dll!{name.lower()}" in self.imports:
            return self.imports[f"kernel32.dll!{name.lower()}"]
        key = f"{'ntdll' if module == 'ntdll' else module.upper()}.dll!{name}"
        if key not in self._thunks:
            self._thunks[key] = self.make_thunk(key)
        return self._thunks[key]
