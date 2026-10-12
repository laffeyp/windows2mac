# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""A family's candidate is derived (Sprint 057): the runtime files its runs reach, read from the runtime's object
files, their headers, and the Python answering their requests; and a run fails when a process it started ran a runtime
not built from the candidate's sources (KIT_DIARY #46)."""

from __future__ import annotations

import re
import sys
from pathlib import Path
from typing import Any

import pytest

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))

import family_run  # noqa: E402

GUEST = ROOT / "tools" / "guest_runtime"


def test_an_edit_to_heap_c_makes_file_search_stale() -> None:
    f = family_run.FAMILIES["file_search"]
    edited = {"heap.c": (GUEST / "heap.c").read_text() + "\n/* an edit */\n"}
    assert "heap.c" in family_run.runtime_files(f)
    assert family_run.candidate_sha256(f, edited) != family_run.candidate_sha256(f)


def test_an_edit_to_a_file_no_run_reaches_leaves_the_family_fresh() -> None:
    f = family_run.FAMILIES["registry"]
    assert "windows.c" not in family_run.runtime_files(f)
    edited = {"windows.c": (GUEST / "windows.c").read_text() + "\n/* an edit */\n"}
    assert family_run.candidate_sha256(f, edited) == family_run.candidate_sha256(f)


def test_the_runtime_reads_a_files_symbols_from_its_object() -> None:
    o = family_run.runtime_objects()
    assert {"k32_FindFirstFileExW", "k32_FindNextFileW", "k32_FindClose"} <= o["file_search.c"].defined
    assert {"w2m_heap_take", "w2m_heap_release"} <= o["file_search.c"].used
    assert "w2m_heap_take" in o["heap.c"].defined
    assert "w2m_threads_blocked" in o["threads.c"].defined           # a common symbol defines it
    assert "runtime.h" in o["file_search.c"].headers


def test_every_request_the_runtime_makes_names_the_python_answering_it() -> None:
    from w2m.vm_backend import REQUEST_FILES
    made = {r for p in GUEST.glob("*.c") for r in family_run.REQUEST.findall(p.read_text())}
    assert made <= set(REQUEST_FILES), sorted(made - set(REQUEST_FILES))
    for files in REQUEST_FILES.values():
        assert all((ROOT / path).is_file() for path in files), files


def test_host_files_hold_only_python_no_request_reaches() -> None:
    for name, f in family_run.FAMILIES.items():
        derived = set(family_run.requested_host_files(family_run.runtime_files(f)))
        for path in f.host_files:
            assert path.endswith(".py"), f"{name}: {path} (a runtime file or header is derived)"
            assert path not in derived, f"{name}: {path} is derived from its requests"


def test_every_family_reaches_its_own_guest_file_and_the_services_its_probe_imports() -> None:
    from w2m.pe import parse as parse_pe
    from w2m.vm_backend import service_for
    objects = family_run.runtime_objects()
    definer = {s: n for n, o in objects.items() for s in o.defined}
    for name, f in family_run.FAMILIES.items():
        files = set(family_run.runtime_files(f))
        assert f.guest_file in files, name
        for data in family_run.family_binaries(f):
            image = parse_pe(data)
            for dll, fns in image.imports.items():
                for fn in fns:
                    service = service_for(dll, fn)
                    if service is not None:
                        assert definer[service[0]] in files, (name, dll, fn)


def test_the_host_names_symbols_the_runtime_defines() -> None:
    objects = family_run.runtime_objects()
    known = {s for o in objects.values() for s in (*o.defined, *o.local)}
    text = (ROOT / "src" / "w2m" / "vm_backend.py").read_text()
    named = set(re.findall(r'(?:symbols\[|runtime_symbol\()"(\w+)"', text))
    assert {"w2m_cpuid", "w2m_exception", "w2m_threads_blocked"} <= named
    assert named <= set(family_run.host_named_symbols()) and set(family_run.host_named_symbols()) <= known


@pytest.mark.hvf
def test_a_plant_whose_child_ran_another_runtime_fails(monkeypatch: Any) -> None:
    """Sprint 055's defect put back: the spawner builds the child from the unplanted runtime. The plant run fails and
    names the child; with the spawner as it is, the same plant is caught as named."""
    f = family_run.FAMILIES["time_and_error_mode"]
    ok, line = family_run.plant_run(f, "error_mode_not_inherited")
    assert ok, line
    from w2m.kernel import spawner as real

    def unplanted(files: Any, dlls: Any = None, runtime_sources: Any = None) -> Any:
        return real(files, dlls, None)

    monkeypatch.setattr(family_run, "spawner", unplanted)
    ok, line = family_run.plant_run(f, "error_mode_not_inherited")
    assert not ok
    assert family_run.FOREIGN in line and "process 4100" in line, line
