# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""signatures.py — the parameter and return types of every Windows function the game calls, from Microsoft's
win32metadata (Sprint 060).

The game's calls judge w2m's (scripts/call_compare.py), and a value can be judged only by what its type allows: a
flag must be equal, a pointer or a handle differs between any two runs. Microsoft publishes the Windows API's
signatures as ECMA-335 metadata (`Windows.Win32.winmd`, NuGet package Microsoft.Windows.SDK.Win32Metadata); this
script reads it with dnfile and writes, for each function the capture names, its DLL, whether it sets the last error
(ImplMap's SupportsLastError flag, ECMA-335 II.23.1.8; the metadata's mark of SetLastError=true), and each parameter's and the return's kind:

  value     an integer, enumeration or flag set, or a typedef of one (BOOL, HRESULT, NTSTATUS, LPARAM)
  pointer   a pointer, a string typedef (PWSTR, PSTR), a callback, or a struct larger than a register
  handle    a typedef whose name starts with H (HANDLE, HKEY, HWND, HMODULE), HRESULT excepted
  float     a float or double: in an XMM register, which the Windows trace does not record
  void      the return of a function that returns nothing

The kinds are names for scripts/call_compare.py's relations; the metadata's own type name is kept beside each.
Functions the capture names that the metadata lacks are listed in `missing`.

Usage: uv run --with dnfile==0.18.0 python scripts/signatures.py <game-run-dir> [--out specs/signatures.json]
"""

from __future__ import annotations

import argparse
import hashlib
import json
import sys
import urllib.request
import zipfile
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parent.parent
VERSION = "71.0.30-preview"
PACKAGE = (f"https://api.nuget.org/v3-flatcontainer/microsoft.windows.sdk.win32metadata/{VERSION}/"
           f"microsoft.windows.sdk.win32metadata.{VERSION}.nupkg")
PACKAGE_SHA256 = "b1f4ae72797e76c7512f08ba24ee3fd2deff01c26a7d812083d56229f7d62a3b"
WINMD_SHA256 = "8cce5eb353c9395784fba3204675ee73d5ba808573d0f7730e86148e51e9e616"
HELD = ROOT / "build" / "win32metadata"

# ECMA-335 II.23.1.16 element types
PRIMITIVE = {0x02: "bool", 0x03: "char", 0x04: "i1", 0x05: "u1", 0x06: "i2", 0x07: "u2", 0x08: "i4", 0x09: "u4",
             0x0A: "i8", 0x0B: "u8", 0x18: "isize", 0x19: "usize"}
FLOAT = {0x0C: "f32", 0x0D: "f64"}
SIZES = {"bool": 1, "i1": 1, "u1": 1, "char": 2, "i2": 2, "u2": 2, "i4": 4, "u4": 4, "f32": 4, "i8": 8, "u8": 8,
         "f64": 8, "isize": 8, "usize": 8}
VOID, PTR, BYREF, VALUETYPE, CLASS, FNPTR, SZARRAY, ARRAY, CMOD_REQD, CMOD_OPT = (
    0x01, 0x0F, 0x10, 0x11, 0x12, 0x1B, 0x1D, 0x14, 0x1F, 0x20)


def acquire() -> Path:
    """The winmd, from the NuGet package at its recorded digest."""
    HELD.mkdir(parents=True, exist_ok=True)
    package = HELD / f"win32metadata.{VERSION}.nupkg"
    if not package.is_file():
        with urllib.request.urlopen(PACKAGE) as r:
            package.write_bytes(r.read())
    if hashlib.sha256(package.read_bytes()).hexdigest() != PACKAGE_SHA256:
        raise SystemExit(f"{package}: sha256 is not {PACKAGE_SHA256}")
    winmd = HELD / "Windows.Win32.winmd"
    if not winmd.is_file():
        with zipfile.ZipFile(package) as z:
            winmd.write_bytes(z.read("Windows.Win32.winmd"))
    if hashlib.sha256(winmd.read_bytes()).hexdigest() != WINMD_SHA256:
        raise SystemExit(f"{winmd}: sha256 is not {WINMD_SHA256}")
    return winmd


def called(run: Path) -> set[str]:
    """Every function the capture saw the game call: imports with a count, names resolved at run time, and the
    traced calls of the game and its crash handler."""
    names = {f for _, _, f, n in (ln.split("\t") for ln in (run / "jit_capture.bin.imports.tsv").read_text().splitlines()
                                  if ln) if int(n) > 0}
    names |= {ln.strip() for ln in (run / "jit_capture.bin.dynamic.tsv").read_text().splitlines() if ln.strip()}
    for trace in run.glob("jit_capture.bin*.trace.tsv"):
        names |= {ln.split("\t")[4] for ln in trace.read_text().splitlines() if ln}
    return {n for n in names if not n.startswith("#")}


class Metadata:
    def __init__(self, winmd: Path) -> None:
        import dnfile  # type: ignore[import-not-found]  # dnfile 0.18.0 ships no stubs; run with uv --with
        self.pe = dnfile.dnPE(str(winmd))
        t = self.pe.net.mdtables
        self.t = t
        self.by_name = {(str(d.TypeNamespace), str(d.TypeName)): d for d in t.TypeDef}
        self.typedef_attribute: set[str] = set()
        for ca in t.CustomAttribute:
            ctor = ca.Type.row
            parent = getattr(ctor, "Class", None)
            owner = parent.row if parent is not None else None
            if owner is not None and str(getattr(owner, "TypeName", "")) == "NativeTypedefAttribute":
                p = ca.Parent.row
                if hasattr(p, "TypeName"):
                    self.typedef_attribute.add(f"{p.TypeNamespace}.{p.TypeName}")

    def resolve(self, coded: int) -> Any:
        tag, row = coded & 3, coded >> 2
        if tag == 0:
            return self.t.TypeDef[row - 1]
        if tag == 1:
            ref = self.t.TypeRef[row - 1]
            return self.by_name.get((str(ref.TypeNamespace), str(ref.TypeName)), ref)
        return None

    def kind_of(self, blob: bytes, i: int) -> tuple[str, str, int]:
        """(kind, type name, next offset) of the type at blob[i]."""
        e = blob[i]
        if e in (CMOD_REQD, CMOD_OPT):
            _, i = uncompress(blob, i + 1)
            return self.kind_of(blob, i)
        if e == VOID:
            return "void", "void", i + 1
        if e in PRIMITIVE:
            return "value", PRIMITIVE[e], i + 1
        if e in FLOAT:
            return "float", FLOAT[e], i + 1
        if e in (PTR, BYREF, SZARRAY):              # the target's name only: a struct may point to itself
            name, j = self.name_of(blob, i + 1)
            return "pointer", name + "*", j
        if e == FNPTR:
            return "pointer", "fnptr", skip_method_sig(blob, i + 1)
        if e in (VALUETYPE, CLASS):
            coded, j = uncompress(blob, i + 1)
            d = self.resolve(coded)
            return (*self.classify(d), j)
        raise ValueError(f"element type 0x{e:x} at {i} in {blob.hex()}")

    def name_of(self, blob: bytes, i: int) -> tuple[str, int]:
        e = blob[i]
        if e in (CMOD_REQD, CMOD_OPT):
            return self.name_of(blob, uncompress(blob, i + 1)[1])
        if e in (PTR, BYREF, SZARRAY):
            name, j = self.name_of(blob, i + 1)
            return name + "*", j
        if e in (VALUETYPE, CLASS):
            coded, j = uncompress(blob, i + 1)
            d = self.resolve(coded)
            return (str(d.TypeName) if d is not None else "unresolved"), j
        if e == FNPTR:
            return "fnptr", skip_method_sig(blob, i + 1)
        return {VOID: "void", **PRIMITIVE, **FLOAT}.get(e, f"0x{e:x}"), i + 1

    def classify(self, d: Any) -> tuple[str, str]:
        if d is None or not hasattr(d, "Extends"):
            return "pointer", "unresolved"
        name = str(d.TypeName)
        base = d.Extends.row
        base_name = str(getattr(base, "TypeName", "")) if base is not None else ""
        if base_name == "Enum":
            return "value", name
        if base_name == "MulticastDelegate":
            return "pointer", name
        fields = list(d.FieldList)
        if f"{d.TypeNamespace}.{name}" in self.typedef_attribute and len(fields) == 1:
            blob = signature(fields[0].row.Signature)
            inner, inner_name, _ = self.kind_of(blob, 1)              # FieldSig: 0x06, then the type
            if name.startswith("H") and name != "HRESULT":
                return "handle", name
            return inner, name
        if len(fields) == 1:                                          # a wrapper (MEMORY_MAPPED_VIEW_ADDRESS)
            inner, _, _ = self.kind_of(signature(fields[0].row.Signature), 1)
            if inner != "value":
                return inner, name
        size = self.size(d)               # Windows x64 passes a struct of 1, 2, 4 or 8 bytes in a register
        return ("value" if size in (1, 2, 4, 8) else "pointer"), name

    def value_bytes(self, blob: bytes, i: int) -> int | None:
        """The bytes a value of the type at blob[i] occupies: the register bits a comparison reads (the caller leaves
        anything in the rest)."""
        e = blob[i]
        if e in (CMOD_REQD, CMOD_OPT):
            return self.value_bytes(blob, uncompress(blob, i + 1)[1])
        if e in PRIMITIVE:
            return SIZES[PRIMITIVE[e]]
        if e in (VALUETYPE, CLASS):
            d = self.resolve(uncompress(blob, i + 1)[0])
            if d is None or not hasattr(d, "FieldList"):
                return None
            base = d.Extends.row
            fields = [f.row for f in d.FieldList if not int(getattr(f.row.Flags, "value", 0) or 0) & 0x10]
            if str(getattr(base, "TypeName", "")) == "Enum" or len(fields) == 1:
                return self.value_bytes(signature(fields[0].Signature), 1)    # value__, or a typedef's Value
            return self.size(d)
        return None

    def size(self, d: Any) -> int | None:
        """A struct's size, its fields laid out at their natural alignment, or None for a field that is not a
        primitive, a pointer or such a struct (an explicit layout or packing is not read)."""
        sizes = SIZES
        total, align = 0, 1
        for f in d.FieldList:
            if int(getattr(f.row.Flags, "value", 0) or 0) & 0x10:      # a static field takes no room
                continue
            blob = signature(f.row.Signature)
            e = blob[1]                                   # FieldSig: 0x06, then the type
            if e in PRIMITIVE or e in FLOAT:
                n = a = sizes[(PRIMITIVE.get(e) or FLOAT[e])]
            elif e in (PTR, FNPTR):
                n = a = 8
            elif e == VALUETYPE:
                inner = self.resolve(uncompress(blob, 2)[0])
                if inner is None or not hasattr(inner, "FieldList"):
                    return None
                base = inner.Extends.row
                if str(getattr(base, "TypeName", "")) == "Enum":
                    n = a = 4
                else:
                    inner_size = self.size(inner)
                    if inner_size is None:
                        return None
                    n = inner_size
                    a = min(8, n) if n in (1, 2, 4, 8) else 8
            else:
                return None
            total = (total + a - 1) // a * a + n
            align = max(align, a)
        return (total + align - 1) // align * align if total else None

    def functions(self, wanted: set[str]) -> dict[str, Any]:
        out: dict[str, Any] = {}
        for im in self.t.ImplMap:
            name = str(im.ImportName)
            if name not in wanted:
                continue
            md = im.MemberForwarded.row
            blob = signature(md.Signature)
            count, i = uncompress(blob, 1)            # after the calling convention
            def described(i: int) -> tuple[dict[str, Any], int]:
                kind, type_name, j = self.kind_of(blob, i)
                d: dict[str, Any] = {"type": type_name, "kind": kind}
                if kind == "value":
                    d["bytes"] = self.value_bytes(blob, i)
                return d, j
            ret_entry, i = described(i)
            params = []
            names = [str(p.row.Name) for p in md.ParamList if p.row.Sequence > 0]
            for k in range(count):
                d, i = described(i)
                params.append({"name": names[k] if k < len(names) else f"p{k}", **d})
            entry = {"dll": str(im.ImportScope.row.Name), "sets_last_error": bool(im.MappingFlags.pmSupportsLastError),
                     "return": ret_entry, "params": params}
            if name in out and out[name] != entry:
                out[name].setdefault("also", []).append(entry)
            else:
                out[name] = entry
        return out


def signature(column: Any) -> bytes:
    """A Signature column's blob without the heap's compressed length prefix (ECMA-335 II.24.2.4)."""
    raw = bytes(column.raw_data if hasattr(column, "raw_data") else column)
    n, i = uncompress(raw, 0)
    return raw[i:i + n]


def uncompress(blob: bytes, i: int) -> tuple[int, int]:
    """ECMA-335 II.23.2 compressed unsigned integer."""
    b = blob[i]
    if b & 0x80 == 0:
        return b, i + 1
    if b & 0xC0 == 0x80:
        return (b & 0x3F) << 8 | blob[i + 1], i + 2
    return (b & 0x1F) << 24 | blob[i + 1] << 16 | blob[i + 2] << 8 | blob[i + 3], i + 4


def skip_method_sig(blob: bytes, i: int) -> int:
    count, i = uncompress(blob, i + 1)
    for _ in range(count + 1):
        i = skip_type(blob, i)
    return i


def skip_type(blob: bytes, i: int) -> int:
    e = blob[i]
    if e in (CMOD_REQD, CMOD_OPT):
        return skip_type(blob, uncompress(blob, i + 1)[1])
    if e in (PTR, BYREF, SZARRAY):
        return skip_type(blob, i + 1)
    if e in (VALUETYPE, CLASS):
        return uncompress(blob, i + 1)[1]
    if e == FNPTR:
        return skip_method_sig(blob, i + 1)
    return i + 1


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("run", type=Path)
    ap.add_argument("--out", type=Path, default=ROOT / "specs" / "signatures.json")
    a = ap.parse_args(argv)
    wanted = called(a.run)
    found = Metadata(acquire()).functions(wanted)
    missing = sorted(wanted - set(found))
    doc: dict[str, Any] = {"version": "0.1.0", "source": {"package": f"Microsoft.Windows.SDK.Win32Metadata {VERSION}",
                                         "package_sha256": PACKAGE_SHA256, "winmd_sha256": WINMD_SHA256,
                                         "reader": "dnfile 0.18.0"},
           "capture": str(a.run.relative_to(ROOT)) if a.run.is_absolute() else str(a.run),
           "missing": missing, "functions": dict(sorted(found.items()))}
    a.out.write_text(json.dumps(doc, indent=1) + "\n")
    print(f"{len(found)} of {len(wanted)} functions; missing {len(missing)}: {missing[:40]}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
