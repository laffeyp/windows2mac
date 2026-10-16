# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""clr_metadata.py — the native calls a .NET assembly declares (Sprint 088; the static census).

A managed method marked [DllImport] is a row of the ImplMap table, which names the method's import name and a ModuleRef
row, which names the DLL (ECMA-335 6th edition, Partition II: 24.3 the metadata root and streams, 24.2.6 the #~ stream,
22.22 ImplMap, 22.31 ModuleRef). Reading ImplMap means skipping every table before it (0x00-0x1B), whose row sizes
depend on the heaps' index widths and on coded indexes sized by the tables they can point at (II.24.2.6); the schema
below is II.22's columns for those tables. Nothing here runs managed code.
"""

from __future__ import annotations

import struct
from typing import Any

from w2m.pe import PeError, PeImage

DIR_CLR = 14

# coded indexes (II.24.2.6): the tables each can point at, in tag order
CODED = {
    "TypeDefOrRef": (0x02, 0x01, 0x1B),
    "HasConstant": (0x04, 0x08, 0x17),
    "HasCustomAttribute": (0x06, 0x04, 0x01, 0x02, 0x08, 0x09, 0x0A, 0x00, 0x0E, 0x17, 0x14, 0x11, 0x1A, 0x1B, 0x20,
                           0x23, 0x26, 0x27, 0x28, 0x2A, 0x2C, 0x2B),
    "HasFieldMarshal": (0x04, 0x08),
    "HasDeclSecurity": (0x02, 0x06, 0x20),
    "MemberRefParent": (0x02, 0x01, 0x1A, 0x06, 0x1B),
    "HasSemantics": (0x14, 0x17),
    "MethodDefOrRef": (0x06, 0x0A),
    "MemberForwarded": (0x04, 0x06),
    "ResolutionScope": (0x00, 0x1A, 0x23, 0x01),
    "CustomAttributeType": (0xFF, 0xFF, 0x06, 0x0A, 0xFF),   # tags 0, 1 and 4 unused
}
# the columns of tables 0x00-0x1C (II.22): an int is a fixed width in bytes; "s", "g", "b" the string, GUID and blob
# heap indexes; ("t", n) a simple index into table n; a str a coded index
SCHEMA: dict[int, tuple[Any, ...]] = {
    0x00: (2, "s", "g", "g", "g"),                                   # Module
    0x01: ("ResolutionScope", "s", "s"),                             # TypeRef
    0x02: (4, "s", "s", "TypeDefOrRef", ("t", 0x04), ("t", 0x06)),   # TypeDef
    0x03: (("t", 0x04),),                                            # FieldPtr
    0x04: (2, "s", "b"),                                             # Field
    0x05: (("t", 0x06),),                                            # MethodPtr
    0x06: (4, 2, 2, "s", "b", ("t", 0x08)),                          # MethodDef
    0x07: (("t", 0x08),),                                            # ParamPtr
    0x08: (2, 2, "s"),                                               # Param
    0x09: (("t", 0x02), "TypeDefOrRef"),                             # InterfaceImpl
    0x0A: ("MemberRefParent", "s", "b"),                             # MemberRef
    0x0B: (2, "HasConstant", "b"),                                   # Constant (type, padding)
    0x0C: ("HasCustomAttribute", "CustomAttributeType", "b"),        # CustomAttribute
    0x0D: ("HasFieldMarshal", "b"),                                  # FieldMarshal
    0x0E: (2, "HasDeclSecurity", "b"),                               # DeclSecurity
    0x0F: (2, 4, ("t", 0x02)),                                       # ClassLayout
    0x10: (4, ("t", 0x04)),                                          # FieldLayout
    0x11: ("b",),                                                    # StandAloneSig
    0x12: (("t", 0x02), ("t", 0x14)),                                # EventMap
    0x13: (("t", 0x14),),                                            # EventPtr
    0x14: (2, "s", "TypeDefOrRef"),                                  # Event
    0x15: (("t", 0x02), ("t", 0x17)),                                # PropertyMap
    0x16: (("t", 0x17),),                                            # PropertyPtr
    0x17: (2, "s", "b"),                                             # Property
    0x18: (2, ("t", 0x06), "HasSemantics"),                          # MethodSemantics
    0x19: (("t", 0x02), "MethodDefOrRef", "MethodDefOrRef"),         # MethodImpl
    0x1A: ("s",),                                                    # ModuleRef
    0x1B: ("b",),                                                    # TypeSpec
    0x1C: (2, "MemberForwarded", "s", ("t", 0x1A)),                  # ImplMap
}


def _streams(data: bytes, img: PeImage) -> dict[str, bytes]:
    """The metadata root's streams (II.24.2.1, II.24.2.2), by name."""
    (lfanew,) = struct.unpack_from("<I", data, 0x3C)
    at = lfanew + 24 + (112 if img.machine == 0x8664 else 96) + 8 * DIR_CLR
    clr_rva, clr_size = struct.unpack_from("<II", data, at)
    if not clr_rva:
        raise PeError("no CLR runtime header")
    md_rva, md_size = struct.unpack_from("<II", img.read_rva(data, clr_rva, 72), 8)
    md = img.read_rva(data, md_rva, md_size)
    if md[:4] != b"BSJB":
        raise PeError("no metadata root signature BSJB")
    (vlen,) = struct.unpack_from("<I", md, 12)
    p = 16 + vlen
    _, nstreams = struct.unpack_from("<HH", md, p)
    p += 4
    out: dict[str, bytes] = {}
    for _ in range(nstreams):
        off, size = struct.unpack_from("<II", md, p)
        end = md.index(b"\0", p + 8)
        name = md[p + 8:end].decode("ascii")
        p = (end + 4) & ~3
        out[name] = md[off:off + size]
    return out


def pinvokes(data: bytes, img: PeImage) -> list[tuple[str, str]]:
    """(DLL as the ModuleRef names it, import name) for every ImplMap row, in table order."""
    s = _streams(data, img)
    tables = s.get("#~") or s.get("#-")
    if tables is None:
        raise PeError("no #~ table stream")
    strings = s.get("#Strings", b"")
    heap = tables[6]
    sw, gw, bw = (4 if heap & 1 else 2), (4 if heap & 2 else 2), (4 if heap & 4 else 2)
    (valid,) = struct.unpack_from("<Q", tables, 8)
    rows: dict[int, int] = {}
    p = 24
    for t in range(64):
        if valid >> t & 1:
            (rows[t],) = struct.unpack_from("<I", tables, p)
            p += 4
    if heap & 0x40:                                      # an extra 4 bytes in some #- streams (ECMA-335 notes)
        p += 4

    def coded(name: str) -> int:
        targets = CODED[name]
        bits = (len(targets) - 1).bit_length()
        return 2 if max(rows.get(t, 0) for t in targets if t != 0xFF) < 1 << (16 - bits) else 4

    def width(col: Any) -> int:
        if isinstance(col, int):
            return col
        if col == "s":
            return sw
        if col == "g":
            return gw
        if col == "b":
            return bw
        if isinstance(col, tuple):
            return 2 if rows.get(col[1], 0) < 1 << 16 else 4
        return coded(col)

    starts: dict[int, int] = {}
    for t in range(0x1D):
        if t not in rows:
            continue
        starts[t] = p
        p += rows[t] * sum(width(c) for c in SCHEMA[t])

    def read(t: int, i: int) -> list[int]:
        at = starts[t] + i * sum(width(c) for c in SCHEMA[t])
        out = []
        for c in SCHEMA[t]:
            w = width(c)
            out.append(int.from_bytes(tables[at:at + w], "little"))
            at += w
        return out

    def string(i: int) -> str:
        return strings[i:strings.index(b"\0", i)].decode("utf-8", errors="replace")

    modules = [string(read(0x1A, i)[0]) for i in range(rows.get(0x1A, 0))]
    out = []
    for i in range(rows.get(0x1C, 0)):
        _, _, name, scope = read(0x1C, i)
        out.append((modules[scope - 1] if 0 < scope <= len(modules) else "?", string(name)))
    return out
