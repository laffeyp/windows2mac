"""Sprint 013 grading surface: the PE reader on real binaries built by the windows-2025 runner,
checked against llvm-objdump (an independent reader), plus malformed inputs."""

from __future__ import annotations

import re
import shutil
import subprocess
from pathlib import Path

import pytest

from w2m.pe import PeError, parse

BINARIES = sorted((Path(__file__).resolve().parent.parent / "store" / "binaries").glob("*.exe"))
OBJDUMP = shutil.which("llvm-objdump") or (
    subprocess.run(["xcrun", "--find", "llvm-objdump"], capture_output=True, text=True).stdout.strip()
    if shutil.which("xcrun") else "")


def objdump_imports(path: Path) -> dict[str, set[str]]:
    out = subprocess.run([OBJDUMP, "-p", str(path)], capture_output=True, text=True, check=True).stdout
    imports: dict[str, set[str]] = {}
    dll = None
    for line in out.splitlines():
        m = re.match(r"\s+DLL Name: (\S+)", line)
        if m:
            dll = m.group(1)
            imports[dll] = set()
            continue
        m = re.match(r"\s+\d+\s+(\S+)$", line)
        if dll and m:
            imports[dll].add(m.group(1))
        if line.strip().startswith("lookup "):
            dll = None
    return imports


def objdump_field(path: Path, name: str) -> int:
    out = subprocess.run([OBJDUMP, "-p", str(path)], capture_output=True, text=True, check=True).stdout
    m = re.search(rf"^{name}\s+([0-9a-fA-F]+)", out, re.M)
    assert m, name
    return int(m.group(1), 16)


def test_fixtures_present() -> None:
    assert len(BINARIES) >= 2


@pytest.mark.skipif(not OBJDUMP, reason="llvm-objdump not available")
@pytest.mark.parametrize("path", BINARIES, ids=lambda p: p.name[:12])
def test_matches_llvm_objdump(path: Path) -> None:
    img = parse(path.read_bytes())
    assert img.entry_rva == objdump_field(path, "AddressOfEntryPoint")
    assert img.image_base == objdump_field(path, "ImageBase")
    assert {d: set(n) for d, n in img.imports.items()} == objdump_imports(path)


@pytest.mark.parametrize("path", BINARIES, ids=lambda p: p.name[:12])
def test_structure(path: Path) -> None:
    img = parse(path.read_bytes())
    text = next(s for s in img.sections if s.name == ".text")
    assert text.flags == "r-x"
    assert text.virtual_address <= img.entry_rva < text.virtual_address + text.virtual_size
    assert set(img.relocations) <= {0, 10}         # only DIR64 and ABSOLUTE padding
    assert ("static", img.entry_rva) in img.code_targets()
    if sum(map(len, img.imports.values())) > 20:   # CRT-linked probes (hello, event_auto_reset)
        assert "WaitForSingleObject" in img.imports["KERNEL32.dll"]
    else:                                          # no-CRT probes (Sprint 029): kernel32 only; DIR64
        assert set(img.imports) == {"KERNEL32.dll"}  # entries appear with static pointer tables
                                                     # (create_file: 12), unused at the preferred base


@pytest.mark.parametrize(("name", "mutate", "needle"), [
    ("no MZ", lambda b: b"XX" + b[2:], "MZ"),
    ("bad PE sig", lambda b: b[:0x3C] + (0x200).to_bytes(4, "little") + b[0x40:], "PE signature"),
    ("truncated", lambda b: b[:0x200], "beyond file size"),
])
def test_malformed(name: str, mutate: object, needle: str) -> None:
    data = BINARIES[0].read_bytes()
    with pytest.raises(PeError, match=needle):
        parse(mutate(data))  # type: ignore[operator]


def test_wrong_machine_refused() -> None:
    data = bytearray(BINARIES[0].read_bytes())
    lfanew = int.from_bytes(data[0x3C:0x40], "little")
    data[lfanew + 4:lfanew + 6] = (0x14C).to_bytes(2, "little")   # i386
    with pytest.raises(PeError, match="not AMD64"):
        parse(bytes(data))


def test_read_rva_stops_at_section_end() -> None:
    """Sprint 023: no invented bytes past a section's virtual extent (review 2026-10-08, 4.4)."""
    data = BINARIES[0].read_bytes() if BINARIES else pytest.skip("no fixtures")
    img = parse(data)
    s = img.sections[0]
    end = s.virtual_address + max(s.virtual_size, s.raw_size)
    assert len(img.read_rva(data, end - 3, 15)) == 3
    assert len(img.read_rva(data, s.virtual_address, 15)) == 15


CORPUS = Path(__file__).resolve().parent.parent / "corpus" / "python-3.14.8"


@pytest.mark.skipif(not (CORPUS / "_wmi.pyd").exists(), reason="corpus not downloaded")
def test_delay_load_imports_resolve() -> None:
    """Sprint 026: directory 13 read; every delay IAT slot is named (review 2026-10-08, 4.9)."""
    img = parse((CORPUS / "_wmi.pyd").read_bytes())
    assert img.delay_imports == {"ole32.dll": ("CoInitializeEx", "CoCreateInstance",
                                               "CoSetProxyBlanket", "CoInitializeSecurity",
                                               "CoUninitialize")}
    assert sum(v.startswith("ole32.dll!") for v in img.iat.values()) == 5


@pytest.mark.parametrize("path", BINARIES, ids=lambda p: p.name[:12])
def test_relocation_sites_match_counts(path: Path) -> None:
    img = parse(path.read_bytes())
    assert len(img.relocation_sites) == sum(n for t, n in img.relocations.items() if t)
    assert all(t == 10 for _, t in img.relocation_sites)        # IMAGE_REL_BASED_DIR64 only
