# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""The guest runtime refuses two patterns that accept input silently (Sprint 056; the review of Sprint 054, KIT_DIARY #47).

1. A `w2m_heap_take` result used without a test: the heap answers 0 when its arena is full, and file search used that
   0 as an address. Every result is tested on one of the two lines after the call.
2. UTF-16 narrowed to `char`: the registry's lower() made U+0173 an 's'. A function taking a u16 may not return a
   char, and each `(char)` cast is listed here with the guard that makes it sound.
"""

from __future__ import annotations

import re
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
RUNTIME = ROOT / "tools" / "guest_runtime"

TAKE = re.compile(r"([\w>.\-\[\]]+)\s*=\s*(?:\([^)]*\)\s*)?w2m_heap_take\(")
NARROWING_FUNCTION = re.compile(r"\bchar\s+\w+\s*\(\s*(?:const\s+)?(?:u16|unsigned short)\b")
# file -> (char) casts allowed, each sound for the reason given
CHAR_CASTS = {
    "console.c": 1,      # GetCommandLineA: a unit above 0x7F stops the run first
    "module.c": 2,       # GetModuleFileNameA: the same
    "resources.c": 2,    # bytes of a manifest, already char-sized
}


def unchecked_takes(text: str) -> list[str]:
    lines = text.splitlines()
    out = []
    for i, line in enumerate(lines):
        m = TAKE.search(line)
        if m is None:
            continue
        var = m.group(1).split()[-1].lstrip("*")
        after = "\n".join(lines[i + 1:i + 3])
        if f"!{var})" not in after and f"!{var} " not in after:
            out.append(f"{i + 1}: {line.strip()}")
    return out


def test_every_heap_take_is_tested() -> None:
    found = {p.name: unchecked_takes(p.read_text()) for p in sorted(RUNTIME.glob("*.c"))}
    assert {k: v for k, v in found.items() if v} == {}


def test_no_utf16_narrowing() -> None:
    for p in sorted(RUNTIME.glob("*.c")):
        text = p.read_text()
        assert not NARROWING_FUNCTION.search(text), f"{p.name}: a function from u16 to char"
        assert text.count("(char)") == CHAR_CASTS.get(p.name, 0), f"{p.name}: a (char) cast not listed in CHAR_CASTS"


def test_the_gate_catches_its_plants() -> None:
    """The gate is judged too: an unchecked take and a narrowing function are each found."""
    assert unchecked_takes("    u64 b = w2m_heap_take(8);\n    use(b);\n    more(b);\n") == ["1: u64 b = w2m_heap_take(8);"]
    assert unchecked_takes("    u64 b = w2m_heap_take(8);\n    if (!b) stop();\n") == []
    assert NARROWING_FUNCTION.search("static char lower(u16 c) { return (char)c; }")
