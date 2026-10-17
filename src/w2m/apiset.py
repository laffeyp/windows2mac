# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""apiset.py — Windows's API set map, read from apisetschema.dll's .apiset section (Sprint 093).

An import from `api-ms-win-core-synch-l1-2-0.dll` names no file; the loader maps the name to a host DLL through this
schema. Layout (schema version 6, Windows 10 and later; Geoff Chappell, "API Set Schema",
geoffchappell.com/studies/windows/win32/apisetschema): a 0x1C-byte header (version, size, flags, count, entries
offset, hash offset, hash multiplier); 0x18-byte namespace entries (flags, name offset, name length, hashed length,
values offset, values count); 0x14-byte value entries (flags, name offset, name length, value offset, value length).
Strings are UTF-16LE at offsets from the schema's start. A name matches up to, not including, its last hyphen, case
ignored. A value with a name applies only when that module imports the set; the value without one is the default.
"""

from __future__ import annotations

import struct
from dataclasses import dataclass

from w2m.pe import PeImage


@dataclass(frozen=True)
class ApiSetMap:
    # set name up to its last hyphen, lower case -> (default host, {importing module: host})
    sets: dict[str, tuple[str, dict[str, str]]]

    def host(self, name: str, importer: str = "") -> str | None:
        """The DLL an import of `name` binds to, for the module `importer`; None if `name` is not an API set or maps
        to nothing (an API set with no host on this build)."""
        stem = name.lower().removesuffix(".dll")
        if not stem.startswith(("api-", "ext-")) or "-" not in stem:
            return None
        found = self.sets.get(stem.rsplit("-", 1)[0])
        if found is None:
            return None
        default, overrides = found
        return overrides.get(importer.lower(), default) or None


def read(image: PeImage, data: bytes) -> ApiSetMap:
    """The map in apisetschema.dll's .apiset section."""
    section = next(s for s in image.sections if s.name == ".apiset")
    return read_schema(data[section.raw_offset:section.raw_offset + section.raw_size])


def read_schema(schema: bytes) -> ApiSetMap:
    version, _, _, count, entries, _, _ = struct.unpack_from("<7I", schema, 0)
    if version != 6:
        raise ValueError(f"API set schema version {version}, not 6")

    def text(offset: int, length: int) -> str:
        return schema[offset:offset + length].decode("utf-16-le").lower()

    sets: dict[str, tuple[str, dict[str, str]]] = {}
    for i in range(count):
        _, name_off, _, hashed, values_off, values = struct.unpack_from("<6I", schema, entries + 0x18 * i)
        default, overrides = "", {}
        for j in range(values):
            _, vname_off, vname_len, value_off, value_len = struct.unpack_from("<5I", schema, values_off + 0x14 * j)
            host = text(value_off, value_len)
            if vname_len:
                overrides[text(vname_off, vname_len)] = host
            else:
                default = host
        sets[text(name_off, hashed)] = (default, overrides)
    return ApiSetMap(sets)
