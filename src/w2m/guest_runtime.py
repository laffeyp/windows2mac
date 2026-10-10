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


def api_ids() -> dict[int, str]:
    """W2M_API_<name> numbers from runtime.h: the ring's api field."""
    text = (SOURCE / "runtime.h").read_text()
    return {int(n): name for name, n in re.findall(r"#define W2M_API_(\w+) (\d+)", text)}


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


def build(base_va: int, defines: dict[str, int], last_error_source: str | None = None,
          file_source: str | None = None, sources: dict[str, str] | None = None) -> Image:
    """Compile every .c file of tools/guest_runtime/ and link them at `base_va`. `sources` replaces
    files by name (a planted fault, a model's draft: Sprints 042, 044); `last_error_source` (a
    candidate's last-error pair, which has no includes) and `file_source` are its older spellings."""
    texts = {p.name: p.read_text() for p in sorted(SOURCE.glob("*.c"))}
    if last_error_source is not None:
        texts["last_error.c"] = '#include "runtime.h"\n' + last_error_source
    if file_source is not None:
        texts["file.c"] = file_source
    for name, text in (sources or {}).items():
        if name not in texts:
            raise BuildError(f"{name}: no such guest runtime file")
        texts[name] = text
    header = (SOURCE / "runtime.h").read_text()
    from w2m.cpuid import c_header
    cpuid_table = c_header()
    dflags = [f"-D{k}=0x{v:x}ULL" for k, v in sorted(defines.items())]
    key = hashlib.sha256("\0".join([*(f"{k}\0{v}" for k, v in sorted(texts.items())), header, cpuid_table,
                                    " ".join(CFLAGS), " ".join(dflags), f"{base_va:x}"]).encode()).hexdigest()
    out = CACHE / f"{key}.bin"
    if not out.exists():
        CACHE.mkdir(parents=True, exist_ok=True)
        work = CACHE / key
        work.mkdir(exist_ok=True)
        (work / "cpuid_table.h").write_text(cpuid_table)
        objs = []
        for name, text in sorted(texts.items()):
            src = work / name
            src.write_text(text)
            obj = work / (src.stem + ".o")
            r = subprocess.run(["clang", *CFLAGS, *dflags, f"-I{SOURCE}", f"-I{work}", "-c", str(src), "-o", str(obj)],
                               capture_output=True, text=True)
            if r.returncode:
                raise BuildError(f"{name}: {r.stderr.strip()}")
            objs.append(str(obj))
        r = subprocess.run(["ld", "-arch", "arm64", "-preload", "-e", "_w2m_image_entry",
                            "-segaddr", "__TEXT", f"0x{base_va:x}", "-o", str(out), *objs],
                           capture_output=True, text=True)
        if r.returncode:
            raise BuildError(f"ld: {r.stderr.strip()}")
    data = out.read_bytes()
    segments, symbols = _parse(data)
    return Image(tuple(segments), symbols, "sha256:" + hashlib.sha256(data).hexdigest())
