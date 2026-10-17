# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""pe.py — PE32+ (x86-64) image reader (v0.2 section 4.1: the frontend starts from the PE).

Reads headers, sections, imports, base relocations, exports, and TLS callbacks from the bytes
of a Windows executable, following Microsoft's "PE Format" specification. Malformed input
raises PeError naming the file offset; nothing is guessed. The import table is the first
draft of an application's contract inventory (v0.2 section 7); the entry point, exports, and
TLS callbacks are statically known code targets (v0.2 section 5.1).
"""

from __future__ import annotations

import struct
from dataclasses import dataclass, field

IMAGE_FILE_MACHINE_AMD64 = 0x8664
IMAGE_FILE_MACHINE_I386 = 0x14C
PE32_PLUS = 0x20B
PE32 = 0x10B                     # a 32-bit image's optional header (PE Format, "Optional Header Standard Fields")
SCN_MEM_EXECUTE = 0x20000000
SCN_MEM_READ = 0x40000000
SCN_MEM_WRITE = 0x80000000
DIR_EXPORT, DIR_IMPORT, DIR_RESOURCE, DIR_EXCEPTION, DIR_BASERELOC, DIR_TLS, DIR_DELAY = 0, 1, 2, 3, 5, 9, 13
RT_MANIFEST = 24
REL_ABSOLUTE, REL_DIR64 = 0, 10


class PeError(ValueError):
    """The bytes are not a well-formed PE32+ image."""


@dataclass(frozen=True)
class Section:
    name: str
    virtual_address: int
    virtual_size: int
    raw_offset: int
    raw_size: int
    characteristics: int

    @property
    def flags(self) -> str:
        c = self.characteristics
        return ("r" if c & SCN_MEM_READ else "-") + ("w" if c & SCN_MEM_WRITE else "-") + (
            "x" if c & SCN_MEM_EXECUTE else "-")


@dataclass(frozen=True)
class PeImage:
    machine: int
    timestamp: int
    characteristics: int
    image_base: int
    entry_rva: int
    section_alignment: int
    size_of_image: int
    subsystem: int
    dll_characteristics: int
    sections: tuple[Section, ...]
    imports: dict[str, tuple[str, ...]] = field(default_factory=dict)
    relocations: dict[int, int] = field(default_factory=dict)  # type -> count
    relocation_sites: tuple[tuple[int, int], ...] = ()          # (RVA, type), type != ABSOLUTE
    delay_imports: dict[str, tuple[str, ...]] = field(default_factory=dict)  # bound on first call
    exports: tuple[tuple[str, int], ...] = ()                  # (name, rva)
    # an export whose RVA lies inside the export directory is a forwarder: a string naming another DLL's function
    # (kernel32 forwards 211 of its named exports, gdi32 294; Sprint 093); keyed by "#ordinal", and by name if named
    forwarders: dict[str, str] = field(default_factory=dict)
    export_ordinals: dict[int, int] = field(default_factory=dict)  # ordinal -> RVA, every export not forwarded
    tls_callbacks: tuple[int, ...] = ()                         # RVAs
    tls: tuple[int, int, int, int] | None = None                 # TLS directory: template start, end, index (VAs), zero fill
    iat: dict[int, str] = field(default_factory=dict)           # IAT slot RVA -> "dll!name" (incl. delay)
    function_starts: tuple[int, ...] = ()                       # .pdata RUNTIME_FUNCTION.BeginAddress
    function_ends: tuple[int, ...] = ()                         # .pdata EndAddress, in the same order
    stack_reserve: int = 0x100000                               # SizeOfStackReserve (a new thread's default; Sprint 051)
    stack_commit: int = 0x1000                                  # SizeOfStackCommit
    resources: tuple[tuple[int | str, int | str, int, int, int], ...] = ()   # (type, name, language, data RVA, size)

    def read_rva(self, data: bytes, rva: int, size: int) -> bytes:
        """Up to `size` bytes of the image at `rva`, from file `data`. Bytes past a section's raw
        data but inside its virtual size read as zero, as the loader maps them (PE Format,
        "Section Table": the remainder is zero-padded). The result stops at the section's end;
        it never invents bytes beyond it, so a decoder sees a truncated instruction as such."""
        for s in self.sections:
            extent = max(s.virtual_size, s.raw_size)
            if s.virtual_address <= rva < s.virtual_address + extent:
                delta = rva - s.virtual_address
                size = min(size, extent - delta)
                raw = data[s.raw_offset + delta: s.raw_offset + min(s.raw_size, delta + size)]
                return raw + b"\0" * (size - len(raw))
        raise PeError(f"RVA 0x{rva:x} is in no section")

    def resource(self, data: bytes, rtype: int | str, name: int | str | None = None) -> bytes | None:
        """The bytes of a resource of type `rtype` (and `name`; any name when None), its first language, or None."""
        for t, n, _, rva, size in self.resources:
            if t == rtype and (name is None or n == name):
                return self.read_rva(data, rva, size)
        return None

    def code_targets(self) -> list[tuple[str, int]]:
        """Statically known entries into guest code: (discovery kind, RVA)."""
        out = [("static", self.entry_rva)] if self.entry_rva else []
        out += [("export", rva) for _, rva in self.exports]
        out += [("callback", rva) for rva in self.tls_callbacks]
        out += [("pdata", rva) for rva in self.function_starts]
        return out


class _Reader:
    def __init__(self, data: bytes) -> None:
        self.data = data

    def u(self, fmt: str, off: int) -> tuple[int, ...]:
        size = struct.calcsize(fmt)
        if off < 0 or off + size > len(self.data):
            raise PeError(f"read of {size} bytes at offset 0x{off:x} beyond file size 0x{len(self.data):x}")
        return struct.unpack_from(fmt, self.data, off)

    def cstr(self, off: int, limit: int = 4096) -> str:
        if off < 0 or off >= len(self.data):
            raise PeError(f"string at offset 0x{off:x} beyond file size")
        end = self.data.find(b"\0", off, off + limit)
        if end < 0:
            raise PeError(f"unterminated string at offset 0x{off:x}")
        try:
            return self.data[off:end].decode("ascii", errors="strict")
        except UnicodeDecodeError:              # found by tests/test_fuzz.py (Sprint 027)
            raise PeError(f"non-ASCII string at offset 0x{off:x}") from None


def _rva_to_offset(sections: tuple[Section, ...], rva: int) -> int:
    for s in sections:
        span = max(s.virtual_size, s.raw_size)
        if s.virtual_address <= rva < s.virtual_address + span:
            delta = rva - s.virtual_address
            if delta >= s.raw_size:
                raise PeError(f"RVA 0x{rva:x} lies in uninitialized data of {s.name}")
            return s.raw_offset + delta
    raise PeError(f"RVA 0x{rva:x} is in no section")


def parse(data: bytes, x86: bool = False) -> PeImage:
    """Parse a PE32+ x86-64 image; raise PeError on anything malformed. With `x86`, a PE32 x86 (i386) image is read too
    (Sprint 085, the static census): its optional header holds BaseOfData and a 32-bit ImageBase, 32-bit stack and heap
    sizes, the data directories at +96, and 32-bit import, delay-import and TLS entries (PE Format). The loader takes
    PE32+ only."""
    r = _Reader(data)
    if data[:2] != b"MZ":
        raise PeError("no MZ signature at offset 0")
    (lfanew,) = r.u("<I", 0x3C)
    if data[lfanew:lfanew + 4] != b"PE\0\0":
        raise PeError(f"no PE signature at e_lfanew 0x{lfanew:x}")
    coff = lfanew + 4
    machine, nsec, timestamp, _, _, opt_size, chars = r.u("<HHIIIHH", coff)
    wide = machine == IMAGE_FILE_MACHINE_AMD64
    if not wide and not (x86 and machine == IMAGE_FILE_MACHINE_I386):
        raise PeError(f"machine 0x{machine:x} is not AMD64 (0x8664)")
    opt = coff + 20
    (magic,) = r.u("<H", opt)
    if magic != (PE32_PLUS if wide else PE32):
        raise PeError(f"optional header magic 0x{magic:x} is not {'PE32+ (0x20b)' if wide else 'PE32 (0x10b)'}")
    (entry,) = r.u("<I", opt + 16)
    (image_base,) = r.u("<Q", opt + 24) if wide else r.u("<I", opt + 28)
    sect_align, _ = r.u("<II", opt + 32)
    size_of_image, _ = r.u("<II", opt + 56)
    subsystem, dllchars = r.u("<HH", opt + 68)
    # PE Format, Optional Header Windows-Specific Fields: four stack and heap sizes, 8 bytes each in PE32+, 4 in PE32
    stack_reserve, stack_commit = r.u("<QQ", opt + 72) if wide else r.u("<II", opt + 72)
    dir_at = opt + (112 if wide else 96)
    (ndirs,) = r.u("<I", dir_at - 4)
    dirs: list[tuple[int, int]] = []
    for i in range(min(ndirs, 16)):
        va, sz = r.u("<II", dir_at + 8 * i)
        dirs.append((va, sz))
    thunk_fmt, thunk_size, ordinal_bit = ("<Q", 8, 63) if wide else ("<I", 4, 31)

    sections = tuple(
        Section(
            name=data[o:o + 8].rstrip(b"\0").decode("ascii", errors="replace"),
            virtual_size=r.u("<I", o + 8)[0], virtual_address=r.u("<I", o + 12)[0],
            raw_size=r.u("<I", o + 16)[0], raw_offset=r.u("<I", o + 20)[0],
            characteristics=r.u("<I", o + 36)[0])
        for o in (opt + opt_size + 40 * i for i in range(nsec)))

    def d(i: int) -> tuple[int, int]:
        return dirs[i] if i < len(dirs) else (0, 0)

    imports: dict[str, tuple[str, ...]] = {}
    iat: dict[int, str] = {}
    rva, size = d(DIR_IMPORT)
    if rva:
        off = _rva_to_offset(sections, rva)
        while True:
            oft, _, _, name_rva, ft = r.u("<IIIII", off)
            if (oft, name_rva, ft) == (0, 0, 0):
                break
            dll = r.cstr(_rva_to_offset(sections, name_rva))
            thunk = _rva_to_offset(sections, oft or ft)
            names: list[str] = []
            slot = ft
            while True:
                (t,) = r.u(thunk_fmt, thunk)
                if t == 0:
                    break
                if t >> ordinal_bit:
                    names.append(f"#{t & 0xFFFF}")
                else:
                    names.append(r.cstr(_rva_to_offset(sections, t & 0x7FFFFFFF) + 2))
                iat[slot] = f"{dll}!{names[-1]}"
                thunk += thunk_size
                slot += thunk_size
            imports[dll] = tuple(names)
            off += 20

    # Delay-load imports (PE Format, "Delay-Load Directory Table"; descriptor field order from
    # LLVM Object/COFF.h delay_import_directory_table_entry). The loader binds a slot on its first
    # call, so calls through it land on the named function exactly as an ordinary import does.
    delay: dict[str, tuple[str, ...]] = {}
    rva, size = d(DIR_DELAY)
    if rva:
        off = _rva_to_offset(sections, rva)
        while True:
            attrs, name_rva, _, diat, dint, _, _, _ = r.u("<IIIIIIII", off)
            if name_rva == 0:
                break
            if not attrs & 1:   # pre-VC7 descriptors hold VAs, not RVAs (delayimp.h dlattrRva)
                raise PeError(f"delay-load descriptor at offset 0x{off:x} uses VAs (attributes 0)")
            dll = r.cstr(_rva_to_offset(sections, name_rva))
            thunk, slot, names = _rva_to_offset(sections, dint), diat, []
            while True:
                (t,) = r.u(thunk_fmt, thunk)
                if t == 0:
                    break
                names.append(f"#{t & 0xFFFF}" if t >> ordinal_bit
                             else r.cstr(_rva_to_offset(sections, t & 0x7FFFFFFF) + 2))
                iat[slot] = f"{dll}!{names[-1]}"
                thunk += thunk_size
                slot += thunk_size
            delay[dll] = tuple(names)
            off += 32

    relocs: dict[int, int] = {}
    sites: list[tuple[int, int]] = []
    rva, size = d(DIR_BASERELOC)
    if rva:
        off, end = _rva_to_offset(sections, rva), _rva_to_offset(sections, rva) + size
        while off < end:
            page, block = r.u("<II", off)
            if block < 8:
                raise PeError(f"relocation block size {block} at offset 0x{off:x}")
            for i in range((block - 8) // 2):
                (e,) = r.u("<H", off + 8 + 2 * i)
                relocs[e >> 12] = relocs.get(e >> 12, 0) + 1
                if e >> 12:                       # type 0 (ABSOLUTE) is block padding
                    sites.append((page + (e & 0xFFF), e >> 12))
            off += block

    exports: list[tuple[str, int]] = []
    forwarders: dict[str, str] = {}
    export_ordinals: dict[int, int] = {}
    rva, size = d(DIR_EXPORT)
    if rva:
        e = _rva_to_offset(sections, rva)
        _, _, _, _, _, ordinal_base, nfuncs, nnames, funcs, names_rva, ords = r.u("<IIHHIIIIIII", e)
        named: dict[int, str] = {}
        for i in range(nnames):
            (nrva,) = r.u("<I", _rva_to_offset(sections, names_rva) + 4 * i)
            (ordinal,) = r.u("<H", _rva_to_offset(sections, ords) + 2 * i)
            (frva,) = r.u("<I", _rva_to_offset(sections, funcs) + 4 * ordinal)
            named[ordinal] = r.cstr(_rva_to_offset(sections, nrva))
            exports.append((named[ordinal], frva))
        for index in range(nfuncs):
            (frva,) = r.u("<I", _rva_to_offset(sections, funcs) + 4 * index)
            if frva == 0:
                continue                                  # an unused slot
            if rva <= frva < rva + size:             # under its ordinal always, and its name if it has one (d3d11
                target = r.cstr(_rva_to_offset(sections, frva))   # imports dxcore's named forwarders by ordinal)
                forwarders[f"#{ordinal_base + index}"] = target
                if index in named:
                    forwarders[named[index]] = target
            else:
                export_ordinals[ordinal_base + index] = frva

    callbacks: list[int] = []
    tls = None
    rva, size = d(DIR_TLS)
    if rva:
        start_va, end_va, index_va, cb_va, zero_fill = r.u("<QQQQI" if wide else "<IIIII", _rva_to_offset(sections, rva))
        tls = (start_va, end_va, index_va, zero_fill)
        if cb_va:
            off = _rva_to_offset(sections, cb_va - image_base)
            while True:
                (va,) = r.u(thunk_fmt, off)
                if va == 0:
                    break
                callbacks.append(va - image_base)
                off += thunk_size

    starts: list[int] = []
    ends: list[int] = []
    rva, size = d(DIR_EXCEPTION)
    if rva and wide:                                     # x86-64's RUNTIME_FUNCTION table; PE32 has none of its kind
        off = _rva_to_offset(sections, rva)
        for i in range(size // 12):
            begin, end, _ = r.u("<III", off + 12 * i)
            starts.append(begin)
            ends.append(end)

    resources: list[tuple[int | str, int | str, int, int, int]] = []
    rva, size = d(DIR_RESOURCE)
    if rva:
        root = _rva_to_offset(sections, rva)

        def entries(off: int) -> list[tuple[int | str, int]]:
            # PE Format, "The .rsrc Section": a directory table, its named entries, then its ID entries
            named, ids = r.u("<HH", off + 12)
            out: list[tuple[int | str, int]] = []
            for i in range(named + ids):
                key, target = r.u("<II", off + 16 + 8 * i)
                if key & 0x80000000:
                    (length,) = r.u("<H", root + (key & 0x7FFFFFFF))
                    start = root + (key & 0x7FFFFFFF) + 2
                    if start + 2 * length > len(data):
                        raise PeError(f"resource name at 0x{start:x} beyond the file")
                    out.append((data[start:start + 2 * length].decode("utf-16-le", errors="replace"), target))
                else:
                    out.append((key, target))
            return out

        for rtype, t in entries(root):
            if not t & 0x80000000:
                raise PeError("a resource type entry is not a directory")
            for name, n in entries(root + (t & 0x7FFFFFFF)):
                if not n & 0x80000000:
                    raise PeError("a resource name entry is not a directory")
                for lang, leaf in entries(root + (n & 0x7FFFFFFF)):
                    if leaf & 0x80000000 or not isinstance(lang, int):
                        raise PeError("a resource language entry is not a data entry")
                    data_rva, data_size = r.u("<II", root + leaf)
                    resources.append((rtype, name, lang, data_rva, data_size))

    if not 0 <= entry < size_of_image:
        raise PeError(f"entry RVA 0x{entry:x} outside image size 0x{size_of_image:x}")
    return PeImage(machine=machine, timestamp=timestamp, characteristics=chars,
                   image_base=image_base, entry_rva=entry, section_alignment=sect_align,
                   size_of_image=size_of_image, subsystem=subsystem, dll_characteristics=dllchars,
                   sections=sections, imports=imports, relocations=relocs,
                   relocation_sites=tuple(sites), delay_imports=delay,
                   exports=tuple(exports), forwarders=forwarders, export_ordinals=export_ordinals,
                   tls_callbacks=tuple(callbacks), tls=tls, iat=iat,
                   function_starts=tuple(starts), function_ends=tuple(ends), stack_reserve=stack_reserve, stack_commit=stack_commit,
                   resources=tuple(resources))
