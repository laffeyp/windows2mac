# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""E2's walk (Sprint 093): the API set map, forwarders, stubs, and values carried to a system call's class argument."""

import struct
from pathlib import Path

import pytest

from w2m import apiset

WINDOWS = Path.home() / "w2m-depots" / "windows" / "Windows"


def schema(entries: list[tuple[str, int, str, list[tuple[str, str]]]]) -> bytes:
    """A version-6 schema as Geoff Chappell documents it: header 0x1C, entries 0x18, values 0x14, UTF-16LE strings."""
    strings = bytearray()
    entry_rows, value_rows = [], []
    base_entries = 0x1C
    base_values = base_entries + 0x18 * len(entries)
    nvalues = sum(1 + len(o) for _, _, _, o in entries)
    base_strings = base_values + 0x14 * nvalues

    def put(s: str) -> tuple[int, int]:
        b = s.encode("utf-16-le")
        strings.extend(b)
        return base_strings + len(strings) - len(b), len(b)

    vi = 0
    for name, hashed_chars, default, overrides in entries:
        no, nl = put(name)
        rows = [("", default), *overrides]
        entry_rows.append(struct.pack("<6I", 0, no, nl, 2 * hashed_chars, base_values + 0x14 * vi, len(rows)))
        for importer, host in rows:
            io, il = put(importer) if importer else (0, 0)
            ho, hl = put(host)
            value_rows.append(struct.pack("<5I", 0, io, il, ho, hl))
            vi += 1
    header = struct.pack("<7I", 6, 0, 0, len(entries), base_entries, 0, 0)
    return header + b"".join(entry_rows) + b"".join(value_rows) + bytes(strings)


def test_an_api_set_maps_by_name_up_to_its_last_hyphen() -> None:
    name = "api-ms-win-core-io-l1-1-1"
    m = apiset.read_schema(schema([(name, len(name.rsplit("-", 1)[0]), "kernel32.dll",
                                    [("kernel32.dll", "kernelbase.dll")])]))
    assert m.host("API-MS-WIN-CORE-IO-L1-1-0.dll") == "kernel32.dll"       # another version, upper case: same set
    assert m.host("api-ms-win-core-io-l1-1-1.dll", "kernel32.dll") == "kernelbase.dll"
    assert m.host("kernel32.dll") is None and m.host("api-ms-win-core-file-l1-1-0.dll") is None


@pytest.mark.depot
@pytest.mark.skipif(not WINDOWS.is_dir(), reason="the copy of C:\\Windows is absent")
def test_the_copy_resolves_forwarders_api_sets_and_classes() -> None:
    from w2m import reach
    win = reach.Windows(WINDOWS)
    # kernel32 forwards AcquireSRWLockExclusive to ntdll (measured: 211 of its named exports forward)
    node = win.resolve("x64", "game.exe", "kernel32.dll!AcquireSRWLockExclusive")
    assert not isinstance(node, str) and node[1] == "ntdll.dll"
    # gdi32's D3DKMT calls forward through an ext-ms API set, by ordinal, to dxcore, which forwards on to win32u's stub
    node = win.resolve("x64", "game.exe", "gdi32.dll!D3DKMTAbandonSwapChain")
    assert not isinstance(node, str) and node[1] == "win32u.dll"
    stub = win.summary(node).stub
    assert stub is not None and stub.table == "win32k" and stub.name == "NtGdiDdDDIAbandonSwapChain"
    # ntdll's NtClose is a stub; a 32-bit one too
    numbers = set()
    for arch in ("x64", "x86"):
        node = win.resolve(arch, "", "ntdll.dll!NtClose")
        assert not isinstance(node, str)
        stub = win.summary(node).stub
        assert stub is not None and stub.name == "NtClose"
        numbers.add(stub.number)
    assert numbers == {0xF}                  # the 32-bit stub loads 0x3000f; the number is the low word
    # kernelbase!Sleep is `xor edx, edx; jmp SleepEx`: a tail jump with the constant 0 and the caller's first argument
    node = win.resolve("x64", "", "kernelbase.dll!Sleep")
    assert not isinstance(node, str)
    site = win.summary(node).sites[0]
    assert site.args[:2] == (("param", 0), ("const", 0))
