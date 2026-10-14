# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""guest_runtime.py — build the w2m guest runtime image and read it for loading (Sprint 035).

The runtime is freestanding C (`tools/guest_runtime/`) compiled to AArch64 with Xcode's clang and
linked by Xcode's `ld -preload` at a fixed virtual address: an MH_PRELOAD Mach-O, the linker's
output type for code that runs without an operating system. Its globals are reached PC-relative
(ADRP), so the image runs where it was linked once its segments are copied there. There is no
ELF linker on this host (checked 2026-10-09), and none is needed.

`build(base_va, defines, last_error_source)` returns the segments to copy and the symbol
addresses; images are cached under build/guest_runtime/ by the sha256 of every input.
"""

from __future__ import annotations

import hashlib
import re
import struct
import subprocess
from dataclasses import dataclass
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SOURCE = ROOT / "tools" / "guest_runtime"
CACHE = ROOT / "build" / "guest_runtime"
CFLAGS = ("--target=arm64-apple-macos14", "-O2", "-std=c11", "-Wall", "-Werror", "-ffreestanding",
          "-fno-builtin", "-fno-stack-protector", "-fno-exceptions", "-fno-unwind-tables",
          "-fno-asynchronous-unwind-tables", "-mgeneral-regs-only")

LC_SEGMENT_64, LC_SYMTAB = 0x19, 0x2
MH_MAGIC_64, MH_PRELOAD = 0xFEEDFACF, 0x5


class BuildError(RuntimeError):
    """The runtime (or a candidate compiled into it) failed to compile or link."""


@dataclass(frozen=True)
class Image:
    segments: tuple[tuple[int, bytes], ...]      # (virtual address, bytes including zero fill)
    symbols: dict[str, int]                      # C name (no leading underscore) -> address
    sha256: str

    @property
    def end(self) -> int:
        return max(va + len(data) for va, data in self.segments)


# A service is a definition `u64 <dll>_<Name>(...) {` in a runtime file (Sprint 049): `k32` serves KERNEL32's
# imports, any other prefix the DLL of that name; W2M_VOID before it marks a function Windows declares void, whose
# bridge leaves RAX as it was. The service tables (vm_backend.SERVICES, DLL_SERVICES) and the W2M_API_* ids are
# derived from these definitions, so a family's C file is the one place its services are declared.
# a service's definition: u64 <dll prefix>_<import name>(...) {; WS2_32's prefix holds an underscore itself (Sprint 052)
SERVICE = re.compile(r"^(W2M_VOID\s+)?u64\s+(ws2_32|[a-z0-9]+)_(\w+)\([^;{]*\)\s*\{", re.M)
INTERNAL_APIS = ("Cpuid", "Exception")          # ring entries that are not services (runtime.c)


def _texts() -> dict[str, str]:
    return {p.name: p.read_text() for p in sorted(SOURCE.glob("*.c"))}


def service_table(texts: dict[str, str] | None = None) -> dict[str, dict[str, tuple[str, bool]]]:
    """DLL ("KERNEL32", "SHELL32.DLL", ...) -> import name -> (C symbol, sets RAX)."""
    out: dict[str, dict[str, tuple[str, bool]]] = {}
    for file, text in sorted((texts or _texts()).items()):
        for void, prefix, name in SERVICE.findall(text):
            if prefix == "w2m":
                continue
            dll = "KERNEL32" if prefix == "k32" else f"{prefix.upper()}.DLL"
            table = out.setdefault(dll, {})
            if name in table:
                raise BuildError(f"{file}: {prefix}_{name} is defined twice")
            table[name] = (f"{prefix}_{name}", not void)
    return out


def api_names(texts: dict[str, str] | None = None) -> list[str]:
    names = [n for table in service_table(texts).values() for n in table] + list(INTERNAL_APIS)
    if len(set(names)) != len(names):
        raise BuildError("two services share an import name: their W2M_API_ ids would collide")
    return sorted(names)


def api_ids() -> dict[int, str]:
    """W2M_API_<name> numbers: the ring's api field, 1 up in name order (api_ids.h, written at build)."""
    return {i + 1: name for i, name in enumerate(api_names())}


def api_header(texts: dict[str, str] | None = None) -> str:
    return "".join(f"#define W2M_API_{n} {i + 1}\n" for i, n in enumerate(api_names(texts)))


def _parse(data: bytes) -> tuple[list[tuple[int, bytes]], dict[str, int]]:
    magic, _, _, filetype, ncmds = struct.unpack_from("<IiiII", data, 0)
    if magic != MH_MAGIC_64 or filetype != MH_PRELOAD:
        raise BuildError(f"not an MH_PRELOAD Mach-O: magic {magic:#x}, filetype {filetype}")
    segments: list[tuple[int, bytes]] = []
    symbols: dict[str, int] = {}
    p = 32
    for _ in range(ncmds):
        cmd, size = struct.unpack_from("<II", data, p)
        if cmd == LC_SEGMENT_64:
            vmaddr, vmsize, fileoff, filesize = struct.unpack_from("<QQQQ", data, p + 24)
            if vmsize:
                body = data[fileoff:fileoff + filesize]
                segments.append((vmaddr, body + bytes(vmsize - len(body))))
        elif cmd == LC_SYMTAB:
            symoff, nsyms, stroff, _ = struct.unpack_from("<IIII", data, p + 8)
            for i in range(nsyms):
                strx, ntype, _, _, value = struct.unpack_from("<IBBHQ", data, symoff + 16 * i)
                if ntype & 0x0E == 0x0E:                         # N_SECT: defined in a section
                    end = data.index(b"\0", stroff + strx)
                    name = data[stroff + strx:end].decode()
                    symbols[name[1:] if name.startswith("_") else name] = value
        p += size
    return segments, symbols


@dataclass(frozen=True)
class ObjectFile:
    """One runtime file's object: the external symbols it defines and those it uses, the headers it includes, and its
    file-local symbols (the host reaches some by name in the linked image: w2m_enumerator_vtable)."""
    defined: frozenset[str]
    used: frozenset[str]
    headers: frozenset[str]
    local: frozenset[str] = frozenset()


def _texts_for(last_error_source: str | None, file_source: str | None,
               sources: dict[str, str] | None) -> dict[str, str]:
    texts = _texts()
    if last_error_source is not None:
        texts["last_error.c"] = '#include "runtime.h"\n' + last_error_source
    if file_source is not None:
        texts["file.c"] = file_source
    for name, text in (sources or {}).items():
        if name not in texts:
            raise BuildError(f"{name}: no such guest runtime file")
        texts[name] = text
    return texts


def generated_headers() -> dict[str, str]:
    """The headers the build writes from Python's tables (api_ids.h aside: it is written from the sources)."""
    from w2m.approximate import c_header as approximation_header
    from w2m.code_pages import c_header as code_page_header
    from w2m.cpuid import c_header
    from w2m.system_messages import c_header as message_header
    return {"cpuid_table.h": c_header(), "code_page_tables.h": code_page_header(),
            "system_messages.h": message_header(), "approximation_tables.h": approximation_header()}


def _compile(base_va: int, defines: dict[str, int], texts: dict[str, str]) -> tuple[Path, Path]:
    """Every file compiled to an object (and its header list, clang -MD) under build/guest_runtime/<key>/, where
    either is missing; (that folder, the image's path there is to be linked to)."""
    header = (SOURCE / "runtime.h").read_text()
    generated = generated_headers()
    dflags = [f"-D{k}=0x{v:x}ULL" for k, v in sorted(defines.items())]
    key = hashlib.sha256("\0".join([*(f"{k}\0{v}" for k, v in sorted(texts.items())), header,
                                    generated["cpuid_table.h"], generated["code_page_tables.h"],
                                    generated["system_messages.h"], generated["approximation_tables.h"],
                                    " ".join(CFLAGS), " ".join(dflags),
                                    f"{base_va:x}"]).encode()).hexdigest()
    out = CACHE / f"{key}.bin"
    work = CACHE / key
    names = sorted(texts)
    if all((work / f"{Path(n).stem}.o").exists() and (work / f"{Path(n).stem}.d").exists() for n in names):
        return work, out
    work.mkdir(parents=True, exist_ok=True)
    for name, text in generated.items():
        (work / name).write_text(text)
    (work / "api_ids.h").write_text(api_header(texts))
    for name in names:
        src = work / name
        src.write_text(texts[name])
        obj = work / (src.stem + ".o")
        r = subprocess.run(["clang", *CFLAGS, *dflags, f"-I{SOURCE}", f"-I{work}", "-MD", "-MF", str(work / (src.stem + ".d")),
                            "-c", str(src), "-o", str(obj)], capture_output=True, text=True)
        if r.returncode:
            raise BuildError(f"{name}: {r.stderr.strip()}")
    return work, out


def _object_symbols(data: bytes) -> tuple[set[str], set[str], set[str]]:
    """An MH_OBJECT's symbols: (external and defined in a section, external and undefined, file-local and defined),
    without the leading underscore."""
    magic, _, _, _, ncmds = struct.unpack_from("<IiiII", data, 0)
    if magic != MH_MAGIC_64:
        raise BuildError(f"not a 64-bit Mach-O object: magic {magic:#x}")
    defined: set[str] = set()
    used: set[str] = set()
    local: set[str] = set()
    p = 32
    for _ in range(ncmds):
        cmd, size = struct.unpack_from("<II", data, p)
        if cmd == LC_SYMTAB:
            symoff, nsyms, stroff, _ = struct.unpack_from("<IIII", data, p + 8)
            for i in range(nsyms):
                strx, ntype, _, _, value = struct.unpack_from("<IBBHQ", data, symoff + 16 * i)
                if ntype & 0xE0:                                 # a debugging entry (N_STAB)
                    continue
                end = data.index(b"\0", stroff + strx)
                name = data[stroff + strx:end].decode()
                name = name[1:] if name.startswith("_") else name
                if ntype & 0x0E == 0x0E:                         # N_SECT
                    (defined if ntype & 0x01 else local).add(name)
                elif ntype & 0x0E == 0x00 and ntype & 0x01:      # N_UNDF, N_EXT: a common symbol (a tentative
                    (defined if value else used).add(name)       # definition, its size in n_value) defines it
        p += size
    return defined, used, local


def objects(base_va: int, defines: dict[str, int], sources: dict[str, str] | None = None) -> dict[str, ObjectFile]:
    """Each runtime file's object, as build() links it (Sprint 057: a family's runtime files are read from these)."""
    texts = _texts_for(None, None, sources)
    work, _ = _compile(base_va, defines, texts)
    out = {}
    for name in sorted(texts):
        stem = Path(name).stem
        defined, used, local = _object_symbols((work / f"{stem}.o").read_bytes())
        rule = (work / f"{stem}.d").read_text().replace("\\\n", " ")
        listed = rule.split(":", 1)[1].split()
        headers = frozenset(Path(h).name for h in listed if h.endswith(".h"))
        out[name] = ObjectFile(frozenset(defined), frozenset(used), headers, frozenset(local))
    return out


def build(base_va: int, defines: dict[str, int], last_error_source: str | None = None,
          file_source: str | None = None, sources: dict[str, str] | None = None) -> Image:
    """Compile every .c file of tools/guest_runtime/ and link them at `base_va`. `sources` replaces
    files by name (a planted fault, a model's draft: Sprints 042, 044); `last_error_source` (a
    candidate's last-error pair, which has no includes) and `file_source` are its older spellings."""
    texts = _texts_for(last_error_source, file_source, sources)
    work, out = _compile(base_va, defines, texts)
    if not out.exists():
        objs = [str(work / f"{Path(n).stem}.o") for n in sorted(texts)]
        r = subprocess.run(["ld", "-arch", "arm64", "-preload", "-e", "_w2m_image_entry",
                            "-segaddr", "__TEXT", f"0x{base_va:x}", "-o", str(out), *objs],
                           capture_output=True, text=True)
        if r.returncode:
            raise BuildError(f"ld: {r.stderr.strip()}")
    data = out.read_bytes()
    segments, symbols = _parse(data)
    return Image(tuple(segments), symbols, "sha256:" + hashlib.sha256(data).hexdigest())
