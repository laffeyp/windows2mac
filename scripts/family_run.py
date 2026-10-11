# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""family_run.py — a Windows function family judged on its own record (Sprint 044).

Sprint 042's pipeline (scripts/file_run.py) for any family of functions that share state or a
probe: the family's no-CRT probe ran on the windows-2025 and windows-2022 runners; `rows` derives
the contract's rows from a probe's output (identity, or a relation where values differ between
machines); the contract is those rows from every runner execution, which must agree; the guest
C file implements the family; the item run runs each runner binary in the managed VM, derives its
rows the same way, and decides once.

Usage: uv run python scripts/family_run.py <family> --contract | --run | --plants
"""

from __future__ import annotations

import argparse
import contextlib
import fnmatch
import hashlib
import importlib
import json
import sys
import tempfile
import time
from collections import Counter
from collections.abc import AsyncIterator, Callable, Iterator
from dataclasses import dataclass, field
from functools import partial
from pathlib import Path
from typing import Any

import msgspec

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))

from w2m import VOCAB_VERSION, events  # noqa: E402
from w2m.compare import compare_exact_rows  # noqa: E402
from w2m.contract import Contract, ExactRows, load_contract  # noqa: E402
from w2m.file_bridge import FileBridge  # noqa: E402
from w2m.loader import load  # noqa: E402
from w2m.record import producer, run_with_provenance  # noqa: E402

GUEST = ROOT / "tools" / "guest_runtime"
RETRIEVED = "retrieved 2026-10-10"


@dataclass(frozen=True)
class Run:
    image: str            # windows-2025, windows-2022
    binary: str           # the probe binary the runner built and ran (sha256 hex)
    execution: str        # the workflow's execution id prefix
    cpu: str
    dll: str | None = None    # the DLL the runner built beside the probe (sha256 hex; probe.json `dll`)


@dataclass(frozen=True)
class Plant:
    old: str              # text of the guest C file, present exactly once
    new: str
    reach: tuple[str, ...]  # the rows it changes, named before it runs (fnmatch patterns allowed)
    host: str | None = None  # a host module (w2m.vm_backend) to patch instead of the family's guest file


@dataclass(frozen=True)
class Family:
    name: str
    contract_id: str
    version: str
    summary: str
    probe: str            # probes/<probe>/
    guest_file: str       # tools/guest_runtime/<guest_file>
    runs: dict[str, Run]
    documented: tuple[dict[str, str], ...]
    open_questions: tuple[str, ...]
    derive: Callable[[list[dict[str, Any]]], list[dict[str, Any]]] = lambda printed: printed
    plants: dict[str, Plant] = field(default_factory=dict)
    excluded: dict[str, str] = field(default_factory=dict)    # row -> why the runners may differ on it
    expected_stop: str | None = None      # the stop an excluded row causes in w2m (a prefix of the stop text)
    gui: bool = False                     # the Steam-like start: no standard handles (console.c)
    arguments: str = ""                   # after the quoted module path on the command line
    output_file: str | None = None        # rows come from this file the probe writes, not standard output
    environment: tuple[str, ...] = ()     # the block the start passes (environment.c)
    dll_name: str | None = None           # the probe's own DLL, loaded from its directory (Sprint 046)
    host_files: tuple[str, ...] = ()      # host source that implements the family too (the loader's Python)

    def rows(self, printed: list[dict[str, Any]]) -> list[dict[str, Any]]:
        """The contract's rows from a probe's printed rows: derived, then the excluded rows dropped."""
        return [r for r in self.derive(printed) if r.get("s") not in self.excluded]

    @property
    def contract_path(self) -> Path:
        return ROOT / "contracts" / self.contract_id / f"{self.version}.json"


def documented(eid: str, page: str, updated: str, quote: str) -> dict[str, str]:
    return {"evidence_id": eid, "category": "documented", "applicability": "Windows 10 and later",
            "source_ref": f"https://learn.microsoft.com/en-us/windows/win32/{page} (page updated {updated}; {RETRIEVED})",
            "quote": quote}


def parse(output: bytes) -> list[dict[str, Any]]:
    out = []
    for line in output.splitlines():
        try:
            out.append(json.loads(line))
        except json.JSONDecodeError:
            continue
    return out


# ---- families -------------------------------------------------------------------------------

def process_query_rows(printed: list[dict[str, Any]]) -> list[dict[str, Any]]:
    """IsProcessorFeaturePresent's answer depends on the CPU; where a CPUID bit names the feature,
    the row is whether the answer equals that bit (on Windows, the runner's CPU; in w2m, the guest's
    CPUID profile). Every other row is as printed."""
    rows = []
    for r in printed:
        if "cpuid" in r:
            rows.append({"s": r["s"], "equals_cpuid": r["value"] == r["cpuid"]})
        else:
            rows.append(r)
    return rows


def console_rows(printed: list[dict[str, Any]]) -> list[dict[str, Any]]:
    """The title's length is the runner's path length (D:\\a\\w2m\\w2m\\probe.exe); the row keeps only that
    the title is the module's path."""
    return [{k: v for k, v in r.items() if not (r.get("s") == "startup_info.child.title" and k == "length")}
            for r in printed]


def code_page_rows(printed: list[dict[str, Any]]) -> list[dict[str, Any]]:
    """The best-fit rows share one name; each is keyed by its UTF-16 unit."""
    return [{**r, "s": f"bestfit.{r['u']:04x}"} if r.get("s") == "bestfit" else r for r in printed]


def child_rows(prefix: str) -> Callable[[list[dict[str, Any]]], list[dict[str, Any]]]:
    """The runner parent prints both children's rows, each name prefixed with its start; w2m runs one child,
    whose rows carry no prefix. Either way the family keeps its start's rows, unprefixed."""
    starts = ("set.", "empty.")

    def derive(printed: list[dict[str, Any]]) -> list[dict[str, Any]]:
        out = []
        for r in printed:
            name = r.get("s", "")
            if name.startswith(prefix):
                out.append({**r, "s": name[len(prefix):]})
            elif not name.startswith(starts):
                out.append(r)
        return out
    return derive


ENVIRONMENT_RUNS = {
              "38044699077": Run("windows-2025", "f4f14814713c4e7ebbb495466dbac26666a57852b33bf382c69359c876dd7191", "environment-1791627776-0", "AMD EPYC 9V74 80-Core Processor"),
              "38044700976": Run("windows-2022", "70dc13b7c0dbdbc4a4ddbe167a3f737e10c469d9e27e3055b8254f28cfb58bae", "environment-1791627776-1", "AMD EPYC 9V74 80-Core Processor"),
              "38044703336": Run("windows-2025", "e989ea6b47e0bc74b60cb091ca7c4b07ed066e29df8d5cd2d31cc6b647356431", "environment-1791627776-2", "AMD EPYC 9V45 96-Core Processor"),
              "38044705168": Run("windows-2022", "35692e390c4e8f8baae67fed1239a419c7a609ad0b0ddb13f1a4af2a5fb6ed35", "environment-1791627776-3", "AMD EPYC 9V45 96-Core Processor"),
              "38044763968": Run("windows-2025", "d513171aa16ebe3810bbcc4ffc887dfb4336894d13cd1e68f5523f7a1c9d9dbf", "environment-1791627843-0", "AMD EPYC 9V74 80-Core Processor"),
              "38044766150": Run("windows-2022", "9a2591f53214278bb3dde9694dc09f63a74fa4f155102c24ec056b229ab5b078", "environment-1791627843-1", "AMD EPYC 7763 64-Core Processor"),
              "38044768224": Run("windows-2025", "6902502708041214181579d816ae4098ce8dd360315f13a19a8680658d1dc8bc", "environment-1791627843-2", "AMD EPYC 7763 64-Core Processor"),
              "38044770045": Run("windows-2022", "e42c82c80d83bed0a8a41ca735c0c0d854348a5384340c484c993c4b88281878", "environment-1791627843-3", "AMD EPYC 9V45 96-Core Processor"),
              "38044810768": Run("windows-2025", "55c0b9c2fd822fac38e77d4c913d1206fc7c2a5f84bc217ff431a616bdf0616f", "environment-1791627891-0", "INTEL(R) XEON(R) PLATINUM 8573C"),
              "38044813152": Run("windows-2022", "421b2d96ab2e1155cad7c91dfc32b62357544c5d38280a34da7b5c2e103af110", "environment-1791627891-1", "AMD EPYC 7763 64-Core Processor"),
              "38044815499": Run("windows-2025", "c4f024890f252f5daa25820ab16dd3f416c23061af88d259b7da0c322d8ab0ce", "environment-1791627891-2", "AMD EPYC 9V45 96-Core Processor"),
              "38044817343": Run("windows-2022", "85d227c34bee19a6b6f6e22642937ba35e906c5aa3a5d499e0fa2a4c9c47bfb1", "environment-1791627891-3", "AMD EPYC 7763 64-Core Processor"),
}
ENVIRONMENT_SET = ("ZED=last", "alpha=1", "=C:=C:\\w2m", "SteamAppId=480", "Path=C:\\Windows\\system32")


def environment_family(start: str, summary: str, environment: tuple[str, ...], plants: dict[str, Plant]) -> Family:
    return Family(
        name=f"environment_{start}", contract_id=f"windows.environment.{start}_block", version="0.1.0",
        summary=summary, probe="environment", guest_file="environment.c", runs=ENVIRONMENT_RUNS,
        documented=(
            documented("ms-getenvironmentstringsw", "api/processenv/nf-processenv-getenvironmentstringsw",
                       "2024-02-22", "Treat this memory as read-only; do not modify it directly. To add or change "
                       "an environment variable, use the GetEnvironmentVariable and SetEnvironmentVariable functions."),),
        open_questions=(
            "The rows come from a child started with an explicit block (CREATE_UNICODE_ENVIRONMENT) and no handles; the "
            "block Steam passes the game is not observed, and w2m's frontier gives the launcher an empty block.",),
        derive=child_rows(f"{start}."), gui=True, arguments=f"child-{start}",
        output_file=f"w2m-environment-{start}.jsonl", environment=environment, plants=plants)


def loader_rows(printed: list[dict[str, Any]]) -> list[dict[str, Any]]:
    """apphelp.dll is in the loader lists of a binary's first execution on a runner and not of its second
    (16 runs, both images): the application-compatibility engine looks at a binary it has not seen. The
    rows drop it, with its count."""
    out = []
    for r in printed:
        if "names" in r and "apphelp.dll" in r["names"]:
            r = {**r, "names": [n for n in r["names"] if n != "apphelp.dll"], "count": r["count"] - 1,
                 "bases_equal_module_handles": r["bases_equal_module_handles"] - 1}
        out.append(r)
    return out


def virtual_memory_rows(printed: list[dict[str, Any]]) -> list[dict[str, Any]]:
    """A released region's size is the distance to the next allocation, which differs between executions on
    Windows; the row keeps that it covers at least the reservation (0x1FFFF000 bytes)."""
    out = []
    for r in printed:
        if r.get("s") == "query.released" and "region_size" in r:
            r = {k: v for k, v in r.items() if k != "region_size"} | {"region_covers_reservation": r["region_size"] >= 0x1FFFF000}
        out.append(r)
    return out


TIME_RUNS = {
              "38053043702": Run("windows-2025", "46c86fd1558cf48b36dc5d6ff12061091baad08937f94615692724417884c186", "time_and_error_mode-1791636241-0", "AMD EPYC 7763 64-Core Processor"),
              "38053047590": Run("windows-2022", "fc3b0c25fbaf64daf486791fb26572dd3998058832d9e3f0fb8a2c66fb272957", "time_and_error_mode-1791636241-1", "AMD EPYC 9V74 80-Core Processor"),
              "38053049693": Run("windows-2025", "46c86fd1558cf48b36dc5d6ff12061091baad08937f94615692724417884c186", "time_and_error_mode-1791636241-2", "AMD EPYC 9V74 80-Core Processor"),
              "38053053337": Run("windows-2022", "fc3b0c25fbaf64daf486791fb26572dd3998058832d9e3f0fb8a2c66fb272957", "time_and_error_mode-1791636241-3", "AMD EPYC 9V74 80-Core Processor"),
              "38053109812": Run("windows-2025", "46c86fd1558cf48b36dc5d6ff12061091baad08937f94615692724417884c186", "time_and_error_mode-1791636304-0", "AMD EPYC 7763 64-Core Processor"),
              "38053112133": Run("windows-2022", "fc3b0c25fbaf64daf486791fb26572dd3998058832d9e3f0fb8a2c66fb272957", "time_and_error_mode-1791636304-1", "AMD EPYC 9V45 96-Core Processor"),
              "38053114329": Run("windows-2025", "46c86fd1558cf48b36dc5d6ff12061091baad08937f94615692724417884c186", "time_and_error_mode-1791636304-2", "INTEL(R) XEON(R) PLATINUM 8573C"),
              "38053116643": Run("windows-2022", "fc3b0c25fbaf64daf486791fb26572dd3998058832d9e3f0fb8a2c66fb272957", "time_and_error_mode-1791636304-3", "AMD EPYC 7763 64-Core Processor"),

}


def time_rows(start: str) -> Callable[[list[dict[str, Any]]], list[dict[str, Any]]]:
    """The probe's parent prints its rows, then its child's one row; w2m runs each alone (it serves no
    CreateProcessW), so the parent's family drops the child's row and the child's keeps only it."""
    def derive(printed: list[dict[str, Any]]) -> list[dict[str, Any]]:
        return [r for r in printed if (r.get("s") == "error_mode.child_inherited") == (start == "child")]
    return derive


def time_family(start: str, summary: str) -> Family:
    child = start == "child"
    plants: dict[str, Plant] = {} if child else {
        # named first as system_to_file.february_31; February 31 stays invalid with 29 days, and every date after
        # February of a common year moves instead (recorded in Sprint 046's card)
        "february_29_always": Plant("    return days[m - 1] + (m == 2 && leap(y));",
                                    "    return days[m - 1] + (m == 2);", ("system_to_file.fixed", "file_to_system.fixed")),
        "time_zone_leaves_error": Plant("    set_error(0);\n    return w2m_logged(W2M_API_GetTimeZoneInformation",
                                        "    return w2m_logged(W2M_API_GetTimeZoneInformation", ("time_zone",))}
    return Family(
        name="time_and_error_mode" + ("_child" if child else ""),
        contract_id="windows.time.error_mode_child" if child else "windows.time.local_and_error_mode",
        version="0.1.0", summary=summary, probe="time_and_error_mode", guest_file="time_and_error_mode.c",
        runs=TIME_RUNS,
        documented=(
            documented("ms-seterrormode", "api/errhandlingapi/nf-errhandlingapi-seterrormode", "2024-02-22",
                       "Each process has an associated error mode ... A child process inherits the error mode of its "
                       "parent process."),),
        open_questions=(
            "The runners keep UTC and w2m does too; a player's machine has its own zone. The zone's names are not in "
            "the rows.",
            "Steam's error mode, which the game inherits, is not observed; w2m's GUI start uses 0, as the child of a "
            "parent that set 0."),
        derive=time_rows(start),
        expected_stop=None if child else "Unimplemented: import KERNEL32.dll!CreateProcessW",
        gui=child, arguments="child" if child else "", output_file="w2m-error-mode-child.jsonl" if child else None,
        plants=plants)


def topology_rows(printed: list[dict[str, Any]]) -> list[dict[str, Any]]:
    """Processor topology (Sprint 047): `smt` is the runner's hardware (every runner's cores run two threads;
    w2m's one processor has one core and no SMT) and `modules` and `size_other` in the RelationAll row are the OS's (Server 2025
    lists a processor module, 2022 does not; relationship 7 is excluded below), so both fields are dropped."""
    out = []
    for r in printed:
        r = dict(r)
        r.pop("smt", None)
        if r.get("s") == "ex.query.65535":
            r.pop("modules", None)
            r.pop("size_other", None)                 # the module entry's size, 48, on 2025 only
        out.append(r)
    return out


def paths_rows(printed: list[dict[str, Any]]) -> list[dict[str, Any]]:
    """Paths (Sprint 048): the file part of "." and ".." is the length of the working directory's own last name,
    the machine's (a runner's "windows2mac", w2m's "app"), so it is dropped; the drive after "\\\\?\\" is the
    machine's letter (D: on the runners, C: in w2m) and is written <d>."""
    out = []
    for r in printed:
        r = dict(r)
        if r.get("s") in ("full.dot", "full.dotdot_alone"):
            r.pop("part_from_end", None)
        for k in ("in", "out"):
            v = r.get(k)
            if isinstance(v, str) and v.startswith("\\\\?\\") and v[5:6] == ":":
                r[k] = v[:4] + "<d>" + v[5:]
        out.append(r)
    return out


FAMILIES: dict[str, Family] = {
    "loader": Family(
        name="loader",
        contract_id="windows.loader.static_import",
        version="0.1.0",
        summary=("A program that imports a DLL from its directory: the DLL's entry runs once with DLL_PROCESS_ATTACH "
                 "and a non-NULL lpReserved before the program's entry; the DLL sits at its preferred base; "
                 "GetModuleHandleW finds it by any case and without its extension; the PEB's loader lists hold the "
                 "program, ntdll, KERNEL32, KERNELBASE and the DLL, initialization order ntdll, KERNELBASE, KERNEL32, "
                 "the DLL."),
        probe="loader", guest_file="module.c",
        runs={
              "38047927781": Run("windows-2025", "2bc372b71143f8a74323f8ecac7f5a73ec622d6a329e9000647f5e779d6d8e04", "loader-1791631092-0", "AMD EPYC 9V45 96-Core Processor", dll="6a3dd26e2da2b4f25525c3f0940e7d03b6c3b1bf3ce325808d738ce2a0c62ff7"),
              "38047929442": Run("windows-2022", "93a5ae454dab7f3c50cdada62a5d07e2f56791937fba8d500b727304b98fed96", "loader-1791631092-1", "AMD EPYC 7763 64-Core Processor", dll="8848e2031be46c16d060b7bf7dfcc855f0fc0d6ca0d3d565cc278dfd3aa42765"),
              "38047930988": Run("windows-2025", "2bc372b71143f8a74323f8ecac7f5a73ec622d6a329e9000647f5e779d6d8e04", "loader-1791631092-2", "AMD EPYC 7763 64-Core Processor", dll="6a3dd26e2da2b4f25525c3f0940e7d03b6c3b1bf3ce325808d738ce2a0c62ff7"),
              "38047932539": Run("windows-2022", "93a5ae454dab7f3c50cdada62a5d07e2f56791937fba8d500b727304b98fed96", "loader-1791631092-3", "AMD EPYC 7763 64-Core Processor", dll="8848e2031be46c16d060b7bf7dfcc855f0fc0d6ca0d3d565cc278dfd3aa42765"),
              "38047981900": Run("windows-2025", "2bc372b71143f8a74323f8ecac7f5a73ec622d6a329e9000647f5e779d6d8e04", "loader-1791631150-0", "AMD EPYC 9V74 80-Core Processor", dll="6a3dd26e2da2b4f25525c3f0940e7d03b6c3b1bf3ce325808d738ce2a0c62ff7"),
              "38047983522": Run("windows-2022", "93a5ae454dab7f3c50cdada62a5d07e2f56791937fba8d500b727304b98fed96", "loader-1791631150-1", "AMD EPYC 7763 64-Core Processor", dll="8848e2031be46c16d060b7bf7dfcc855f0fc0d6ca0d3d565cc278dfd3aa42765"),
              "38047984894": Run("windows-2025", "2bc372b71143f8a74323f8ecac7f5a73ec622d6a329e9000647f5e779d6d8e04", "loader-1791631150-2", "AMD EPYC 7763 64-Core Processor", dll="6a3dd26e2da2b4f25525c3f0940e7d03b6c3b1bf3ce325808d738ce2a0c62ff7"),
              "38047987237": Run("windows-2022", "93a5ae454dab7f3c50cdada62a5d07e2f56791937fba8d500b727304b98fed96", "loader-1791631150-3", "AMD EPYC 7763 64-Core Processor", dll="8848e2031be46c16d060b7bf7dfcc855f0fc0d6ca0d3d565cc278dfd3aa42765"),
              "38048037669": Run("windows-2025", "2bc372b71143f8a74323f8ecac7f5a73ec622d6a329e9000647f5e779d6d8e04", "loader-1791631207-0", "AMD EPYC 7763 64-Core Processor", dll="6a3dd26e2da2b4f25525c3f0940e7d03b6c3b1bf3ce325808d738ce2a0c62ff7"),
              "38048039731": Run("windows-2022", "93a5ae454dab7f3c50cdada62a5d07e2f56791937fba8d500b727304b98fed96", "loader-1791631207-1", "AMD EPYC 7763 64-Core Processor", dll="8848e2031be46c16d060b7bf7dfcc855f0fc0d6ca0d3d565cc278dfd3aa42765"),
              "38048041658": Run("windows-2025", "2bc372b71143f8a74323f8ecac7f5a73ec622d6a329e9000647f5e779d6d8e04", "loader-1791631207-2", "AMD EPYC 7763 64-Core Processor", dll="6a3dd26e2da2b4f25525c3f0940e7d03b6c3b1bf3ce325808d738ce2a0c62ff7"),
              "38048043520": Run("windows-2022", "93a5ae454dab7f3c50cdada62a5d07e2f56791937fba8d500b727304b98fed96", "loader-1791631207-3", "AMD EPYC 7763 64-Core Processor", dll="8848e2031be46c16d060b7bf7dfcc855f0fc0d6ca0d3d565cc278dfd3aa42765"),
              "38048095121": Run("windows-2025", "2bc372b71143f8a74323f8ecac7f5a73ec622d6a329e9000647f5e779d6d8e04", "loader-1791631264-0", "AMD EPYC 7763 64-Core Processor", dll="6a3dd26e2da2b4f25525c3f0940e7d03b6c3b1bf3ce325808d738ce2a0c62ff7"),
              "38048097583": Run("windows-2022", "93a5ae454dab7f3c50cdada62a5d07e2f56791937fba8d500b727304b98fed96", "loader-1791631264-1", "AMD EPYC 7763 64-Core Processor", dll="8848e2031be46c16d060b7bf7dfcc855f0fc0d6ca0d3d565cc278dfd3aa42765"),
              "38048099528": Run("windows-2025", "2bc372b71143f8a74323f8ecac7f5a73ec622d6a329e9000647f5e779d6d8e04", "loader-1791631264-2", "AMD EPYC 7763 64-Core Processor", dll="6a3dd26e2da2b4f25525c3f0940e7d03b6c3b1bf3ce325808d738ce2a0c62ff7"),
              "38048101245": Run("windows-2022", "93a5ae454dab7f3c50cdada62a5d07e2f56791937fba8d500b727304b98fed96", "loader-1791631264-3", "AMD EPYC 9V74 80-Core Processor", dll="8848e2031be46c16d060b7bf7dfcc855f0fc0d6ca0d3d565cc278dfd3aa42765"),
        },
        documented=(
            documented("ms-dllmain", "dlls/dllmain", "2024-02-22",
                       "lpReserved: If fdwReason is DLL_PROCESS_ATTACH, lpvReserved is NULL for dynamic loads and "
                       "non-NULL for static loads."),),
        open_questions=(
            "apphelp.dll joins the lists on a binary's first execution on a runner; whether it does for the game "
            "started by Steam is not observed. The rows drop it.",
            "The probe's DLL loaded at its preferred base on all 16 runs though it is ASLR-enabled; UnityPlayer.dll's "
            "base on Windows is not observed, and w2m places it at its preferred base when free.",
            "ntdll's exports are not listed: GetProcAddress on ntdll stops the run."),
        derive=loader_rows, dll_name="loader_dll.dll",
        host_files=("src/w2m/loader.py", "src/w2m/modules.py", "src/w2m/vm_backend.py"),
        plants={"init_order_kernel32_first": Plant(
                    "        init = [nt, kb, k32, *(dll[d.name] for d in self.ld.dependency_order)]",
                    "        init = [nt, k32, kb, *(dll[d.name] for d in self.ld.dependency_order)]",
                    ("ldr.init_order",), host="w2m.vm_backend"),
                "reserved_null": Plant(
                    '        out += bytes.fromhex("49b8") + LOADER_STUB.to_bytes(8, "little")   # mov r8, lpReserved (non-NULL: static load)',
                    '        out += bytes.fromhex("49b8") + (0).to_bytes(8, "little")',
                    ("entry.calls_at_exe_entry",), host="w2m.loader")}),

    "environment_set": environment_family(
        "set", "A process started with an explicit environment block sees that block as it was passed, in its order, "
        "as a fresh process-heap copy of exactly its size on each GetEnvironmentStringsW call; "
        "FreeEnvironmentStringsW frees it and answers TRUE.", ENVIRONMENT_SET,
        {"shared_copy": Plant("    u64 copy = w2m_heap_take(units * 2);",
                              "    static u64 shared;\n    u64 copy = shared ? shared : (shared = w2m_heap_take(units * 2));",
                              ("second_call",)),
         "sorted_block": Plant("    for (u64 i = 0; i < units; i++) to[i] = host[i];",
                               "    for (u64 i = 0; i < units; i++) to[i] = host[i];\n"
                               "    { u16 t = to[0]; to[0] = to[9]; to[9] = t; }",
                               ("block",))}),
    "environment_empty": environment_family(
        "empty", "A process started with an empty environment block gets two NULs, a 4-byte process-heap copy, from "
        "each GetEnvironmentStringsW call.", (),
        {"one_nul": Plant("    if (!host[0]) units = 2;", "    if (!host[0]) units = 1;", ("heap",))}),

    "process_queries": Family(
        name="process_queries",
        contract_id="windows.process.queries",
        version="0.1.0",
        summary=("GetSystemTimeAsFileTime, GetCurrentProcessId, GetCurrentThreadId, QueryPerformanceCounter, "
                 "QueryPerformanceFrequency, IsProcessorFeaturePresent, InitializeSListHead, GetStartupInfoW and "
                 "IsDebuggerPresent answer as Windows does: fixed values exactly, machine-dependent values by the "
                 "relations every Windows machine observed holds."),
        probe="process_queries", guest_file="process.c",
        runs={
              "38041796227": Run("windows-2025", "39e501bce923469a19ac478de13b430949a8eb4719e2ca96a3785aba09468034", "process_queries-1791624794-0", "AMD EPYC 7763 64-Core Processor"),
              "38041797992": Run("windows-2022", "7d7b1d3ff3e5a081473d9b629c7434893e7aa42042a7e0029d9edf75985f4c61", "process_queries-1791624794-1", "AMD EPYC 9V45 96-Core Processor"),
              "38041800459": Run("windows-2025", "9f874e107a0b22eb469d2fb7a80aef286c8060202f6d19bb644d815bd3d8f348", "process_queries-1791624794-2", "AMD EPYC 7763 64-Core Processor"),
              "38041803105": Run("windows-2022", "211f7359ab2c0f1ec20e90467c863a345a9b66130dda3d37521ae19ea2133e91", "process_queries-1791624794-3", "AMD EPYC 9V45 96-Core Processor"),
              "38041805183": Run("windows-2025", "6693106bb1ace5cfc627e407d4832df41fd79689dfac1c86b97c3050d811e033", "process_queries-1791624794-4", "Intel(R) Xeon(R) Platinum 8370C CPU @ 2.80GHz"),
        },
        documented=(
            documented("ms-isprocessorfeaturepresent", "api/processthreadsapi/nf-processthreadsapi-isprocessorfeaturepresent",
                       "2025-12-18", "If the feature is supported, the return value is a nonzero value. If the feature is "
                       "not supported, the return value is zero. ... Support for PF_ARM_SVE_INSTRUCTIONS_AVAILABLE through "
                       "PF_MOVDIR64B_INSTRUCTION_AVAILABLE ... were added in the Windows SDK (26100) and are supported by "
                       "Windows 11, version 24H2 and Windows Server 2025 or later."),
            documented("ms-queryperformancefrequency", "api/profileapi/nf-profileapi-queryperformancefrequency", "2025-11-21",
                       "The frequency of the performance counter is fixed at system boot and is consistent across all "
                       "processors. ... On systems that run Windows XP or later, the function will always succeed when given "
                       "valid parameters and will thus never return zero.")),
        open_questions=(
            "The rows for IsProcessorFeaturePresent are relations where a CPUID bit names the feature: on Windows the "
            "runner CPU's bit, in w2m the guest CPUID profile's (specs/guest_cpuid.json). The PF-to-CPUID-bit table is "
            "the agent's reading of the Intel SDM CPUID tables; 25 of 26 rows agree with Windows on five machines.",
            "GetStartupInfoW's lpReserved is non-null under the runner's cmd; what it points to is not observed. A "
            "process started by Steam may differ.",
            "Excluded rows: see `sections.excluded`."),
        derive=process_query_rows,
        excluded={"ipfp.60": "PF_BMI2_INSTRUCTIONS_AVAILABLE: Server 2022 answers 0 on CPUs with BMI2, Server 2025 "
                             "follows CPUID; Microsoft Learn: supported from Windows 11 24H2 and Server 2025. w2m answers "
                             "as Server 2025.",
                  "ipfp.21": "PF_VIRT_FIRMWARE_ENABLED: 1 on the four AMD machines, 0 on the Intel 8370C; a property of "
                             "the host machine. w2m answers 0: the guest is offered no virtualization."},
        plants={"counter_frequency_wrong": Plant("#define QPC_FREQUENCY 10000000ull", "#define QPC_FREQUENCY 24000000ull",
                                                 ("time.qpf", "time.counter_tracks_time")),
                "process_id_from_thread": Plant("(u32)w2m_teb()->client_process", "(u32)w2m_teb()->client_thread",
                                                ("ids.process", "ids.thread"))}),
    "modules": Family(
        name="modules",
        contract_id="windows.module.queries",
        version="0.1.0",
        summary=("LoadLibraryExW, GetModuleHandleW, GetModuleHandleExW, GetModuleFileNameW, GetProcAddress and "
                 "FreeLibrary answer as Windows does: which handle a name yields relative to the others, the "
                 "error codes, what success leaves in the last error, and GetModuleFileNameW's truncation."),
        probe="modules", guest_file="module.c",
        runs={"38039204088": Run("windows-2025", "dc5dc17938bc5d6fe90630cea27d2ecd92dc37d1c3055cc2ff971d4d235d8803",
                                 "modules-1791622152-0", "Intel Xeon 6973P-C"),
              "38039205782": Run("windows-2022", "2023b96471cb2f483ce8f0f7b1fd644897c39820e6de2a53df8c6093a7315fc2",
                                 "modules-1791622154-1", "AMD EPYC 7763"),
              "38039207305": Run("windows-2025", "16fa45b585275647ae802b8e735675f6aae2a9781e06b3d57deea0700c7f8826",
                                 "modules-1791622156-2", "AMD EPYC 7763"),
              "38039209016": Run("windows-2022", "5c2a1271b0fd34fb03eef5dd2219421a63dedc8a3ce1572e189318b7e24a1ad9",
                                 "modules-1791622157-3", "AMD EPYC 7763"),
              "38039210828": Run("windows-2025", "845b240e2d35ba8c4cb102a65fc665b53e3369262747cca46b83db5b7e75d9b0",
                                 "modules-1791622159-4", "AMD EPYC 9V74"),
              "38039212442": Run("windows-2022", "d289f28d9c8c03b406e6a171cc3eb45795a076a1f6fb962e56de40df5704cc27",
                                 "modules-1791622160-5", "AMD EPYC 9V74")},
        documented=(
            documented("ms-getmodulefilenamew", "api/libloaderapi/nf-libloaderapi-getmodulefilenamew", "2025-10-07",
                       "If the buffer is too small to hold the module name, the string is truncated to nSize characters "
                       "including the terminating null character, the function returns nSize, and the function sets the "
                       "last error to ERROR_INSUFFICIENT_BUFFER."),
            documented("ms-getprocaddress", "api/libloaderapi/nf-libloaderapi-getprocaddress", "2025-07-01",
                       "If the function succeeds, the return value is the address of the exported function or variable. "
                       "If the function fails, the return value is NULL.")),
        open_questions=(
            "api-ms-win-core-synch-l1-2-0 loads as kernelbase on every run; w2m maps every api-ms-win-core-* name "
            "the same way and stops on other API-set families (src/w2m/modules.py). Each new API set the frontier "
            "meets is a row to add.",
            "Success leaves the last error untouched for LoadLibraryExW, GetModuleHandleW, GetModuleHandleExW, "
            "GetProcAddress and FreeLibrary; GetModuleFileNameW sets it to 0 (the 'err' rows), which the page does "
            "not say.",
            "Handles and addresses are compared only as relations (equal to another answer, the image base)."),
        plants={"file_name_leaves_error": Plant("        set_error(0);\n        return w2m_logged(W2M_API_GetModuleFileNameW",
                                                "        return w2m_logged(W2M_API_GetModuleFileNameW",
                                                ("file_name.null",)),
                "truncation_returns_size_minus_one": Plant("return w2m_logged(W2M_API_GetModuleFileNameW, module, size);",
                                                           "return w2m_logged(W2M_API_GetModuleFileNameW, module, size - 1);",
                                                           ("file_name.small_buffer",)),
                "proc_missing_wrong_code": Plant("if (!p) set_error(ERROR_PROC_NOT_FOUND);",
                                                 "if (!p) set_error(ERROR_MOD_NOT_FOUND);", ("proc.missing",))}),
    "critical_sections": Family(
        name="critical_sections",
        contract_id="windows.critical_section.single_thread",
        version="0.1.0",
        summary=("InitializeCriticalSectionEx, InitializeCriticalSectionAndSpinCount, InitializeCriticalSection, "
                 "EnterCriticalSection, TryEnterCriticalSection, LeaveCriticalSection and DeleteCriticalSection on "
                 "one thread leave the CRITICAL_SECTION's fields as Windows leaves them, with Windows's returns."),
        probe="critical_sections", guest_file="critical_section.c",
        runs={
              "38039748482": Run("windows-2025", "9289dc8bfe06771ffa5434e170798322cafb71f9eeb959bc12301f05b37bfd00", "critical_sections-1791622717-0", "AMD EPYC 7763 64-Core Processor"),
              "38039751056": Run("windows-2022", "88e8478288b4dc4e444a32fdcd6882e966a8fcb060d744e84f2331d18c1bca57", "critical_sections-1791622717-1", "AMD EPYC 7763 64-Core Processor"),
              "38039753569": Run("windows-2025", "0f7bb5f8e79380972f0ac6bf0f5927de765185a0fdc08abf01c1bb1d64fcd867", "critical_sections-1791622717-2", "AMD EPYC 9V45 96-Core Processor"),
              "38039755720": Run("windows-2022", "9669215f8cf17a60d82a487c2b7a11a938b26399505554a9dba7a112b951e5d3", "critical_sections-1791622717-3", "INTEL(R) XEON(R) PLATINUM 8573C"),
              "38039757756": Run("windows-2025", "25376298d8c925e49c8362f51f2ec9830e8cff395952ee380ab83bc5e8171704", "critical_sections-1791622717-4", "AMD EPYC 7763 64-Core Processor"),
              "38039759979": Run("windows-2022", "9de03e8427a1c25d4b18220e6312cb348059e168e5a439497885dbc6054d52bc", "critical_sections-1791622717-5", "AMD EPYC 7763 64-Core Processor"),
        },
        documented=(
            documented("ms-initializecriticalsectionex", "api/synchapi/nf-synchapi-initializecriticalsectionex",
                       "2024-02-22", "If the function succeeds, the return value is nonzero. ... A critical section "
                       "object cannot be moved or copied. The process must also not modify the object, but must treat "
                       "it as logically opaque."),),
        open_questions=(
            "The page calls the structure opaque; the rows record its fields anyway, because a program can read them "
            "and w2m must then see what Windows leaves (LockCount -1 free, -2 held; DebugInfo -1; SpinCount flags).",
            "One thread only: a section held by another thread is Sprint 047's (waits, LockSemaphore).",
            "InitializeCriticalSectionEx with a zero count and flags was not observed; w2m stops on it."),
        # (a first plant, LockCount not restored on the last Leave, was named for 3 rows and in fact stopped the
        # run: TryEnter then finds a held section with no owner; the named reach was wrong, recorded in Sprint
        # 044's card, and the plant replaced by one whose effect stays in its row)
        plants={"delete_keeps_debug_info": Plant("    for (int i = 0; i < 40; i++) b[i] = 0;",
                                                 "    for (int i = 8; i < 40; i++) b[i] = 0;", ("delete",)),
                "default_spin_without_flag": Plant("#define DEFAULT_SPIN 0x020007D0ull", "#define DEFAULT_SPIN 0x000007D0ull",
                                                   ("init_ex.0.0", "init"))}),
    "thread_storage": Family(
        name="thread_storage",
        contract_id="windows.thread_storage.single_thread",
        version="0.1.0",
        summary=("TlsAlloc, TlsGetValue, TlsSetValue, TlsFree, FlsAlloc, FlsGetValue, FlsSetValue and FlsFree on "
                 "one thread give Windows's indices, values, returns and last errors, with TLS values in the TEB."),
        probe="thread_storage", guest_file="thread_storage.c",
        runs={
              "38040526466": Run("windows-2025", "4426a716a6a15990383d970d2ee20c6b14b651128b6866fd5f70e4f7fda5a10c", "thread_storage-1791623504-0", "AMD EPYC 9V74 80-Core Processor"),
              "38040528813": Run("windows-2022", "2997768ba33f5c1acf33aef96e588e39216db887037b78693243ccfc290e9722", "thread_storage-1791623504-1", "AMD EPYC 7763 64-Core Processor"),
              "38040531024": Run("windows-2025", "b7c76bb07b75b708f02b637ad0c47b8cc11eda44540f95f3eb0e61f1f0b68a3c", "thread_storage-1791623504-2", "AMD EPYC 7763 64-Core Processor"),
              "38040533511": Run("windows-2022", "88100e479bb25c1a9c3f0efc985d6d7d74888b65b6a3b9aecd3866d757181c8a", "thread_storage-1791623504-3", "AMD EPYC 9V45 96-Core Processor"),
              "38040598310": Run("windows-2025", "2f99a59a6f85d897db00e24e2091f488dad95a554b6c50c3f2aa8d68774a3825", "thread_storage-1791623577-0", "AMD EPYC 9V74 80-Core Processor"),
              "38040600812": Run("windows-2022", "7a74e31d3b1bbcec1289858f22e4af691ffab6f29c81dc2c5cff2d6a21751f5f", "thread_storage-1791623577-1", "AMD EPYC 7763 64-Core Processor"),
              "38040602719": Run("windows-2025", "f6f0e51889a8351333cf5c471489640d23e300e36538e85842fcfa8b5b0fc93c", "thread_storage-1791623577-2", "AMD EPYC 9V74 80-Core Processor"),
              "38040604585": Run("windows-2022", "82c8ad6ef3f4dd191bcf0011c113e6e6cf2cb3e6919f5ca553f737c76c8be1b1", "thread_storage-1791623577-3", "INTEL(R) XEON(R) PLATINUM 8573C"),
        },
        documented=(),
        open_questions=(
            "A fresh process holds TLS indices 0 and 16 before the program's first TlsAlloc (row "
            "tls.alloc.many.indices on every run); which component holds them is not observed.",
            "FlsGetValue answers ERROR_INVALID_PARAMETER for an allocated index until the thread's first "
            "FlsSetValue (rows fls.get.fresh, fls.get.after_set).",
            "Excluded rows: see sections.excluded."),
        excluded={"fls.free.with_callback": "FlsFree runs the index's callback in the program's code; w2m cannot yet "
                  "call into guest code from the runtime and stops there (Sprint 047, with threads, whose exits run "
                  "FLS callbacks too). The probe writes its other rows first."},
        expected_stop="Unimplemented: FlsFree",
        plants={"tls_get_keeps_error": Plant("    set_error(0);\n    return w2m_logged(W2M_API_TlsGetValue",
                                             "    return w2m_logged(W2M_API_TlsGetValue",
                                             ("tls.get.fresh", "tls.get.after_set", "tls.get.reused")),
                "tls_index_16_free": Plant("    tls_used[W2M_TLS_TAKEN] = 1;", "",
                                           ("tls.alloc.many", "tls.alloc.many.indices"))}),
    "heap": Family(
        name="heap",
        contract_id="windows.heap.process",
        version="0.1.0",
        summary=("GetProcessHeap, HeapAlloc, HeapFree, HeapSize and HeapReAlloc on the process heap answer as "
                 "Windows does: the handle is PEB.ProcessHeap, blocks are 16-byte aligned with their exact size, "
                 "zeroing and reallocation keep their promises, and failures return NULL with ERROR_NOT_ENOUGH_MEMORY."),
        probe="heap", guest_file="heap.c",
        runs={
              "38041103334": Run("windows-2025", "2f52e248f92990807fc2222c28b5f50a4ca83372c70c0016cddd76cbdeda92cb", "heap-1791624089-0", "AMD EPYC 9V74 80-Core Processor"),
              "38041105720": Run("windows-2022", "134d6dcd5c311dc80a7fab92f1eff20cd1ff7932c8de8964c18909ad65d482de", "heap-1791624089-1", "AMD EPYC 7763 64-Core Processor"),
              "38041107830": Run("windows-2025", "2214b9352c627faedd3c2aed012096cb1adc924ccaf7d08fbdc91d37065224a1", "heap-1791624089-2", "AMD EPYC 7763 64-Core Processor"),
              "38041110146": Run("windows-2022", "08c6d99a46b15f2ad89a85bf766ae843280d1cd47ac30ec2e5a65920d65cd1ef", "heap-1791624089-3", "AMD EPYC 7763 64-Core Processor"),
              "38041154554": Run("windows-2025", "95fc2b0f7a3d423ffe8720f27c41018cd61532504d6009d1f34197bc7ca27677", "heap-1791624144-0", "AMD EPYC 7763 64-Core Processor"),
              "38041156624": Run("windows-2022", "fa8a0a92909c7a93daf637d488820eba033c9de650a2f201d6c15e3b21769da2", "heap-1791624144-1", "Intel(R) Xeon(R) Platinum 8370C CPU @ 2.80GHz"),
              "38041158857": Run("windows-2025", "ab263ddf08e75bacbcf705e3c3a7edecf949dec9dd752cfa9afa087fd39ef06a", "heap-1791624144-2", "AMD EPYC 7763 64-Core Processor"),
              "38041160693": Run("windows-2022", "fb32f84609c92f406c2e1196e530e09e5ff5887d16b259b2597785122274a031", "heap-1791624144-3", "INTEL(R) XEON(R) PLATINUM 8573C"),
        },
        documented=(
            documented("ms-heapalloc", "api/heapapi/nf-heapapi-heapalloc", "2024-02-22",
                       "If the function fails and you have not specified HEAP_GENERATE_EXCEPTIONS, the return value is "
                       "NULL. ... If the function fails, it does not call SetLastError. ... The alignment of memory "
                       "returned by HeapAlloc is MEMORY_ALLOCATION_ALIGNMENT in WinNT.h"),),
        open_questions=(
            "The page says a failing HeapAlloc does not call SetLastError; on every run a 2^46-byte request and an "
            "in-place-only grow leave ERROR_NOT_ENOUGH_MEMORY (8). The rows follow the observation.",
            "w2m's arena is 256 MB (src/w2m/vm_backend.py HEAP_SIZE); a request Windows would grant that it cannot "
            "hold stops the run rather than answering no memory."),
        # (first plants: unaligned blocks tripped heap.c's own pointer check and stopped the run; an ignored
        # HEAP_ZERO_MEMORY changed nothing on a fresh, zero arena: a blind spot the probe now covers with a
        # dirty, freed block. Both named reaches were wrong; recorded in Sprint 044's card.)
        plants={"size_is_capacity": Plant("header_of(W2M_API_HeapSize, p)->size_and_magic_check",
                                          "header_of(W2M_API_HeapSize, p)->capacity",
                                          # named first as 2 rows; it changed 4: a block shrunk or refused in
                                          # place keeps its capacity, so those sizes change too (recorded)
                                          ("size.matches_request", "alloc.zero_bytes", "realloc.shrink",
                                           "realloc.in_place_only_grow")),
                "zero_flag_ignored": Plant("    if (flags & HEAP_ZERO_MEMORY) zero((u8 *)(b + 1), b->capacity);", "",
                                           ("alloc.zero_memory_after_dirty_free",))}),
    "code_pages": Family(
        name="code_pages",
        contract_id="windows.code_pages.ansi_1252",
        version="0.2.0",
        summary=("GetACP, GetOEMCP, IsValidCodePage, GetCPInfo, MultiByteToWideChar, WideCharToMultiByte, "
                 "GetStringTypeW and LCMapStringW answer as Windows does with ANSI code page 1252: every byte's "
                 "character, type and case, best fit into 1252, and UTF-8 with Windows's rule for malformed input."),
        probe="code_pages", guest_file="code_pages.c",
        runs={
              "38044061149": Run("windows-2025", "d71917e00b97b4728718da9129095dabe8929c3c01e6e07f6e5d1bba8c28d7eb", "code_pages-1791627109-0", "AMD EPYC 9V74 80-Core Processor"),
              "38044062619": Run("windows-2022", "24d23cef6fda9c3543ea90bfc43c527c3b1cf309c2eeecda5058fc0d7db5e60e", "code_pages-1791627109-1", "AMD EPYC 7763 64-Core Processor"),
              "38044064173": Run("windows-2025", "2154008644b78e654ecfc54ee6d668ae5737eb4d94bdc1178c4c3fcf1df719f6", "code_pages-1791627109-2", "AMD EPYC 7763 64-Core Processor"),
              "38044065903": Run("windows-2022", "7a4202f7c2e7f4e8be8f2c48e89e8663f2dbff41b6de3a2c9ac5fd550b6f10a7", "code_pages-1791627109-3", "AMD EPYC 7763 64-Core Processor"),
              "38044118023": Run("windows-2025", "c377564a0a4fcdc2bdbe0da644438588c485bfcb1735a8727a0e7fb38263fe8c", "code_pages-1791627168-0", "AMD EPYC 7763 64-Core Processor"),
              "38044119652": Run("windows-2022", "461ddfc01643127ffc9d5a2c43f276d41624d0b480aeb6de9721dce3c9add5b1", "code_pages-1791627168-1", "AMD EPYC 7763 64-Core Processor"),
              "38044121378": Run("windows-2025", "e3b1209130abbd3551afd0de5afd8bfc50c1f035606cfafff9375e55230a1ec8", "code_pages-1791627168-2", "AMD EPYC 9V74 80-Core Processor"),
              "38044123237": Run("windows-2022", "1751988b3f82681ecb66379b3dfccf23172c3a89d0269ad9740c78d41af26155", "code_pages-1791627168-3", "AMD EPYC 7763 64-Core Processor"),
              "38044173696": Run("windows-2025", "07bbc435b060c6444a9aae4376590def7cc2f7de561ccee5be5a85fe03e59164", "code_pages-1791627226-0", "AMD EPYC 7763 64-Core Processor"),
              "38044175691": Run("windows-2022", "6d2b9e3251a94d6241bb2bf36450686c2b4fe649bae0abab0aea9c79d98178c5", "code_pages-1791627226-1", "AMD EPYC 7763 64-Core Processor"),
              "38044177324": Run("windows-2025", "14227c1ca1c7c637a41ab779cf64c1a94aae17c1ab7062c0a212f264b3e3d9ee", "code_pages-1791627226-2", "AMD EPYC 9V74 80-Core Processor"),
              "38044179699": Run("windows-2022", "b0b1701bbf805f61625eddd12d29382f9e59a311c04cab4440bbb46e31a78f44", "code_pages-1791627226-3", "AMD EPYC 9V45 96-Core Processor"),
              "38044229348": Run("windows-2025", "9395509bfe645a3c33c322e88efdd81285f919b9897020d7a134db5e8cbd45bf", "code_pages-1791627287-0", "Intel(R) Xeon(R) Platinum 8370C CPU @ 2.80GHz"),
              "38044230916": Run("windows-2022", "cea66797b99bdfd85df7a2a6ffd04eaf42d3e06c61f6758b2a60d9de1b30e211", "code_pages-1791627287-1", "AMD EPYC 7763 64-Core Processor"),
              "38044232724": Run("windows-2025", "5bc8d3151e5456a719d636b5a6076c0ff3daadbaab931b73e2d7a4f9c2dce311", "code_pages-1791627287-2", "AMD EPYC 7763 64-Core Processor"),
              "38044234598": Run("windows-2022", "cea66797b99bdfd85df7a2a6ffd04eaf42d3e06c61f6758b2a60d9de1b30e211", "code_pages-1791627287-3", "AMD EPYC 7763 64-Core Processor"),
        },
        documented=(
            documented("ms-multibytetowidechar", "api/stringapiset/nf-stringapiset-multibytetowidechar", "2024-02-22",
                       "MB_ERR_INVALID_CHARS: Fail if an invalid input character is encountered. ... "
                       "ERROR_NO_UNICODE_TRANSLATION. Invalid Unicode was found in a string."),),
        open_questions=(
            "Malformed UTF-8 follows the Unicode Standard's Table 3-7 except after E0, ED, F0 and F4: a second byte in "
            "80..BF outside the table's range is taken into the U+FFFD (3,072 swept inputs). Whether Windows documents "
            "this rule is not established.",
            "GetStringTypeW and LCMapStringW are observed for code page 1252's 256 characters only; w2m stops on any "
            "other unit, on LCMapStringW size queries and on locales other than the user default and invariant.",
            "The batched forms the launcher's C runtime uses (rows forms.*) were found by a scratch run of the launcher "
            "on a guest that logged refused forms (2026-10-10); a form the launcher reaches later may still be missing."),
        derive=code_page_rows,
        plants={"whatwg_maximal_subpart": Plant(
                    "        if (k == 1 && (s[*i] < lo || s[*i] > hi)) { (*i)++; return MALFORMED; }   /* taken: the rows */",
                    "        if (k == 1 && (s[*i] < lo || s[*i] > hi)) return MALFORMED;",
                    ("m2w.utf8.surrogate_encoded", "u8.e0.80*", "u8.e0.8f*", "u8.e0.90*", "u8.e0.9f*", "u8.ed.a0*",
                     "u8.ed.bf*", "u8.f0.80*", "u8.f0.8f*", "u8.f4.90*", "u8.f4.9f*", "u8.f4.a0*", "u8.f4.bf*")),
                "no_best_fit": Plant("            if (b < 0) b = best_fit(c);\n", "            (void)best_fit;\n", ("bestfit.*",))}),
    "exception_filter": Family(
        name="exception_filter",
        contract_id="windows.exception_filter.set",
        version="0.1.0",
        summary=("SetUnhandledExceptionFilter returns the filter it replaces: NULL in a fresh process, then each "
                 "earlier filter, NULL included; it leaves the last error."),
        probe="exception_filter", guest_file="exception_filter.c",
        runs={
              "38045461436": Run("windows-2025", "1f99c34664447d076e17c4cbb02a3225abcf4cec1c55eaa32fd56f06b0b5676f", "exception_filter-1791628557-0", "AMD EPYC 9V45 96-Core Processor"),
              "38045463327": Run("windows-2022", "7b7cb9155af9c2e1c981e0124d26fc64701b827fc583770daebee553d7af3714", "exception_filter-1791628557-1", "AMD EPYC 9V45 96-Core Processor"),
              "38045465137": Run("windows-2025", "b7bf8ad48559ced781c1ab5424f817337aeee0c14b76e9902ffb4aa44a844df7", "exception_filter-1791628557-2", "AMD EPYC 9V74 80-Core Processor"),
              "38045466832": Run("windows-2022", "206c9bad8f5ba0031d2ab50a9bbf8f95acc31ae11e399479476a9057fa2ad697", "exception_filter-1791628557-3", "AMD EPYC 9V74 80-Core Processor"),
              "38045524237": Run("windows-2025", "d0b53af3e88d3166ce9915839932692e02a93041cccd274e9d06d17c7cf121cf", "exception_filter-1791628622-0", "AMD EPYC 7763 64-Core Processor"),
              "38045526273": Run("windows-2022", "28c318a01ba9543409d7a8493722e2df045a3ae528132b00727376a4e71b2169", "exception_filter-1791628622-1", "AMD EPYC 7763 64-Core Processor"),
              "38045528177": Run("windows-2025", "5c868e4e261844ac774eed443abbdc66dce5a1f618b9b6de7e6c040b03c34fe2", "exception_filter-1791628622-2", "AMD EPYC 9V45 96-Core Processor"),
              "38045530145": Run("windows-2022", "d71257aef7dfcfdf8322e786141a3620cc445f876d428e94cab2e145e0d65b18", "exception_filter-1791628622-3", "AMD EPYC 7763 64-Core Processor"),
              "38045583044": Run("windows-2025", "941e12ceffef0b42e0d84b33ba0e473d7086a7b2efd3c86bdbb1b7508a0e416f", "exception_filter-1791628684-0", "AMD EPYC 7763 64-Core Processor"),
              "38045585339": Run("windows-2022", "e7ae9797cfdc4288c533b3d3dd2344822dcb2f084554bd7eb800330136ceadaf", "exception_filter-1791628684-1", "AMD EPYC 7763 64-Core Processor"),
              "38045587579": Run("windows-2025", "941e12ceffef0b42e0d84b33ba0e473d7086a7b2efd3c86bdbb1b7508a0e416f", "exception_filter-1791628684-2", "AMD EPYC 7763 64-Core Processor"),
              "38045589805": Run("windows-2022", "e7ae9797cfdc4288c533b3d3dd2344822dcb2f084554bd7eb800330136ceadaf", "exception_filter-1791628684-3", "AMD EPYC 7763 64-Core Processor"),
              "38045626475": Run("windows-2025", "941e12ceffef0b42e0d84b33ba0e473d7086a7b2efd3c86bdbb1b7508a0e416f", "exception_filter-1791628730-0", "Intel(R) Xeon(R) Platinum 8370C CPU @ 2.80GHz"),
              "38045628559": Run("windows-2022", "e7ae9797cfdc4288c533b3d3dd2344822dcb2f084554bd7eb800330136ceadaf", "exception_filter-1791628730-1", "AMD EPYC 7763 64-Core Processor"),
              "38045630511": Run("windows-2025", "941e12ceffef0b42e0d84b33ba0e473d7086a7b2efd3c86bdbb1b7508a0e416f", "exception_filter-1791628730-2", "AMD EPYC 7763 64-Core Processor"),
              "38045632682": Run("windows-2022", "e7ae9797cfdc4288c533b3d3dd2344822dcb2f084554bd7eb800330136ceadaf", "exception_filter-1791628730-3", "AMD EPYC 7763 64-Core Processor"),
        },
        documented=(
            documented("ms-setunhandledexceptionfilter", "api/errhandlingapi/nf-errhandlingapi-setunhandledexceptionfilter",
                       "2024-02-22", "The SetUnhandledExceptionFilter function returns the address of the previous "
                       "exception filter established with the function. A NULL return value means that there is no "
                       "current top-level exception handler."),),
        open_questions=(
            "The rows cover setting the filter; Windows calling it on an unhandled exception is exception dispatch "
            "(roadmap Sprint 047).",),
        plants={"returns_null": Plant("    return w2m_logged(W2M_API_SetUnhandledExceptionFilter, f, before);",
                                      "    (void)before;\n    return w2m_logged(W2M_API_SetUnhandledExceptionFilter, f, 0);",
                                      ("set.second", "set.null"))}),
    "pointer_encoding": Family(
        name="pointer_encoding",
        contract_id="windows.pointer.encoding",
        version="0.1.0",
        summary=("EncodePointer and DecodePointer as relations: every value round-trips, encodes the same twice and "
                 "to something else, NULL included; no single XOR mask; the last error is left."),
        probe="pointer_encoding", guest_file="pointer_encoding.c",
        runs={
              "38048676507": Run("windows-2025", "56daafe12cec36befc6e503ae5e61a778f3c0d9da921b78e4093cc147696fef1", "pointer_encoding-1791631853-0", "AMD EPYC 7763 64-Core Processor"),
              "38048678695": Run("windows-2022", "96768777a67d8dbe5a1d436fc3987b9e04cb0306e7212f5600b82901311a41e6", "pointer_encoding-1791631853-1", "AMD EPYC 9V74 80-Core Processor"),
              "38048680921": Run("windows-2025", "56daafe12cec36befc6e503ae5e61a778f3c0d9da921b78e4093cc147696fef1", "pointer_encoding-1791631853-2", "AMD EPYC 7763 64-Core Processor"),
              "38048682934": Run("windows-2022", "96768777a67d8dbe5a1d436fc3987b9e04cb0306e7212f5600b82901311a41e6", "pointer_encoding-1791631853-3", "AMD EPYC 7763 64-Core Processor"),
              "38048732025": Run("windows-2025", "56daafe12cec36befc6e503ae5e61a778f3c0d9da921b78e4093cc147696fef1", "pointer_encoding-1791631910-0", "AMD EPYC 7763 64-Core Processor"),
              "38048733916": Run("windows-2022", "96768777a67d8dbe5a1d436fc3987b9e04cb0306e7212f5600b82901311a41e6", "pointer_encoding-1791631910-1", "AMD EPYC 7763 64-Core Processor"),
              "38048735710": Run("windows-2025", "56daafe12cec36befc6e503ae5e61a778f3c0d9da921b78e4093cc147696fef1", "pointer_encoding-1791631910-2", "AMD EPYC 7763 64-Core Processor"),
              "38048737499": Run("windows-2022", "96768777a67d8dbe5a1d436fc3987b9e04cb0306e7212f5600b82901311a41e6", "pointer_encoding-1791631910-3", "AMD EPYC 7763 64-Core Processor"),
              "38048790083": Run("windows-2025", "56daafe12cec36befc6e503ae5e61a778f3c0d9da921b78e4093cc147696fef1", "pointer_encoding-1791631968-0", "AMD EPYC 7763 64-Core Processor"),
              "38048791630": Run("windows-2022", "96768777a67d8dbe5a1d436fc3987b9e04cb0306e7212f5600b82901311a41e6", "pointer_encoding-1791631968-1", "AMD EPYC 7763 64-Core Processor"),
              "38048793388": Run("windows-2025", "56daafe12cec36befc6e503ae5e61a778f3c0d9da921b78e4093cc147696fef1", "pointer_encoding-1791631968-2", "AMD EPYC 7763 64-Core Processor"),
              "38048794925": Run("windows-2022", "96768777a67d8dbe5a1d436fc3987b9e04cb0306e7212f5600b82901311a41e6", "pointer_encoding-1791631968-3", "AMD EPYC 7763 64-Core Processor"),
              "38048847109": Run("windows-2025", "56daafe12cec36befc6e503ae5e61a778f3c0d9da921b78e4093cc147696fef1", "pointer_encoding-1791632026-0", "AMD EPYC 7763 64-Core Processor"),
              "38048848661": Run("windows-2022", "96768777a67d8dbe5a1d436fc3987b9e04cb0306e7212f5600b82901311a41e6", "pointer_encoding-1791632026-1", "AMD EPYC 9V74 80-Core Processor"),
              "38048850441": Run("windows-2025", "56daafe12cec36befc6e503ae5e61a778f3c0d9da921b78e4093cc147696fef1", "pointer_encoding-1791632026-2", "AMD EPYC 7763 64-Core Processor"),
              "38048852058": Run("windows-2022", "96768777a67d8dbe5a1d436fc3987b9e04cb0306e7212f5600b82901311a41e6", "pointer_encoding-1791632026-3", "AMD EPYC 7763 64-Core Processor"),
        },
        documented=(
            documented("ms-encodepointer", "api/utilapiset/nf-utilapiset-encodepointer", "2024-02-22",
                       "Encoding globally available pointers helps protect them from being exploited. The "
                       "EncodePointer function obfuscates the pointer value with a secret so that it cannot be "
                       "predicted by an external agent. The secret used by EncodePointer is different for each process."),),
        open_questions=(
            "The rows fix relations, not Windows's algorithm; w2m keeps one secret for every process, where Windows's "
            "differs per process (its page).",),
        plants={"identity": Plant("    return w2m_logged(W2M_API_EncodePointer, p, ror(p ^ SECRET, SECRET & 63));",
                                  "    (void)ror;\n    return w2m_logged(W2M_API_EncodePointer, p, p);",
                                  ("value.*", "encoding.shape"))}),
    "slim_locks": Family(
        name="slim_locks",
        contract_id="windows.sync.slim_locks",
        version="0.1.0",
        summary=("SRW locks and condition variables on one thread: the words Windows leaves (0 free, 1 exclusive, "
                 "(n << 4) | 1 for n shared holders), the try forms' answers, a timed sleep's FALSE and ERROR_TIMEOUT, "
                 "and wakes with no waiter."),
        probe="slim_locks", guest_file="slim_locks.c",
        runs={
              "38048676516": Run("windows-2025", "d233c8440737eb0e6ddb8ddae518df8d65d8778f5afc0de37a8b2124074a8c10", "slim_locks-1791631853-0", "AMD EPYC 7763 64-Core Processor"),
              "38048679185": Run("windows-2022", "05310917f137b890e6e9c7cc147e380260ccca50af2ae9311d0b76be5e76ea47", "slim_locks-1791631853-1", "AMD EPYC 7763 64-Core Processor"),
              "38048681485": Run("windows-2025", "d233c8440737eb0e6ddb8ddae518df8d65d8778f5afc0de37a8b2124074a8c10", "slim_locks-1791631853-2", "Intel(R) Xeon(R) Platinum 8370C CPU @ 2.80GHz"),
              "38048683615": Run("windows-2022", "05310917f137b890e6e9c7cc147e380260ccca50af2ae9311d0b76be5e76ea47", "slim_locks-1791631853-3", "AMD EPYC 7763 64-Core Processor"),
        },
        documented=(
            documented("ms-sleepconditionvariablesrw", "api/synchapi/nf-synchapi-sleepconditionvariablesrw",
                       "2024-02-22", "If the timeout expires the function returns FALSE and GetLastError returns "
                       "ERROR_TIMEOUT."),),
        open_questions=(
            "One thread: an acquire of a held lock, an INFINITE sleep and a wake with a waiter stop the run (Sprint 047).",
            "The timed sleep returns at once in w2m: no thread can wake it, and w2m does not pass the time.",
            "The words are opaque on Microsoft Learn; the rows hold what a program can read."),
        plants={"shared_count_low_bits": Plant("    *(u64 *)lock = (w ? w + SHARED_ONE : SHARED_ONE | EXCLUSIVE);",
                                               "    *(u64 *)lock = (w ? w + 1 : SHARED_ONE | EXCLUSIVE);",
                                               ("srw.shared_twice", "srw.shared_released_once", "srw.shared_released_twice")),
                "sleep_leaves_error": Plant("    set_error(ERROR_TIMEOUT);\n    return w2m_logged(W2M_API_SleepConditionVariableSRW",
                                           "    (void)set_error;\n    return w2m_logged(W2M_API_SleepConditionVariableSRW",
                                           ("cv.sleep_timeout_zero", "cv.sleep_timeout_one_ms"))}),
    "thread_queries": Family(
        name="thread_queries",
        contract_id="windows.thread.queries",
        version="0.1.0",
        summary=("GetCurrentThread's pseudo-handle -2; GetThreadTimes on it (times as relations) and on NULL "
                 "(ERROR_INVALID_HANDLE); GetThreadPriority 0."),
        probe="thread_queries", guest_file="thread_queries.c",
        runs={
              "38048923955": Run("windows-2025", "397c3cb472085b1bc1b455fb2fb312a37bf9ea2cc4990fbf8165a988299aad7f", "thread_queries-1791632104-0", "Intel(R) Xeon(R) 6973P-C"),
              "38048925932": Run("windows-2022", "a4f1490cbd7485c120dd20a7125c7d90e99e8c8e8e292039e290c19e53b69af3", "thread_queries-1791632104-1", "AMD EPYC 9V45 96-Core Processor"),
              "38048928046": Run("windows-2025", "397c3cb472085b1bc1b455fb2fb312a37bf9ea2cc4990fbf8165a988299aad7f", "thread_queries-1791632104-2", "AMD EPYC 7763 64-Core Processor"),
              "38048930112": Run("windows-2022", "a4f1490cbd7485c120dd20a7125c7d90e99e8c8e8e292039e290c19e53b69af3", "thread_queries-1791632104-3", "AMD EPYC 7763 64-Core Processor"),
        },
        documented=(),
        open_questions=(
            "On Server 2022 a thread's creation time can read later than the system time taken after it (6 of 8 "
            "executions, first probe revision); the row is a window around now.",
            "w2m's user time is the time since creation, an upper bound on the thread's own time."),
        plants={"null_handle_wrong_error": Plant("        set_error(ERROR_INVALID_HANDLE);", "        set_error(87);",
                                                 ("times.null_handle",))}),
    "system_info": Family(
        name="system_info",
        contract_id="windows.system.information",
        version="0.1.0",
        summary=("GetNativeSystemInfo and GetSystemInfo (fixed fields as observed, processor level and revision from "
                 "CPUID), GetVersionExW without a manifest (6.2 build 9200), GlobalMemoryStatusEx, GetTickCount and "
                 "GetSystemDirectoryW, with their size and length errors."),
        probe="system_info", guest_file="system_info.c",
        runs={
              "38049865817": Run("windows-2025", "3a8a76e25b85a37d5966fefda2d3c607a6bdf5baf1fc885b46f8d5442e86ce97", "system_info-1791633068-0", "AMD EPYC 7763 64-Core Processor"),
              "38049867401": Run("windows-2022", "d253082973f83d2c0c6ed95f911884f9e253b2c7519029efa7a1edbd969538e4", "system_info-1791633068-1", "AMD EPYC 7763 64-Core Processor"),
              "38049869138": Run("windows-2025", "3a8a76e25b85a37d5966fefda2d3c607a6bdf5baf1fc885b46f8d5442e86ce97", "system_info-1791633068-2", "AMD EPYC 7763 64-Core Processor"),
              "38049870799": Run("windows-2022", "d253082973f83d2c0c6ed95f911884f9e253b2c7519029efa7a1edbd969538e4", "system_info-1791633068-3", "AMD EPYC 7763 64-Core Processor"),
              "38049919716": Run("windows-2025", "3a8a76e25b85a37d5966fefda2d3c607a6bdf5baf1fc885b46f8d5442e86ce97", "system_info-1791633124-0", "AMD EPYC 7763 64-Core Processor"),
              "38049921396": Run("windows-2022", "d253082973f83d2c0c6ed95f911884f9e253b2c7519029efa7a1edbd969538e4", "system_info-1791633124-1", "AMD EPYC 7763 64-Core Processor"),
              "38049922830": Run("windows-2025", "3a8a76e25b85a37d5966fefda2d3c607a6bdf5baf1fc885b46f8d5442e86ce97", "system_info-1791633124-2", "AMD EPYC 7763 64-Core Processor"),
              "38049924427": Run("windows-2022", "d253082973f83d2c0c6ed95f911884f9e253b2c7519029efa7a1edbd969538e4", "system_info-1791633124-3", "AMD EPYC 7763 64-Core Processor"),
              "38049993205": Run("windows-2025", "3a8a76e25b85a37d5966fefda2d3c607a6bdf5baf1fc885b46f8d5442e86ce97", "system_info-1791633198-0", "AMD EPYC 9V45 96-Core Processor"),
              "38049995593": Run("windows-2022", "d253082973f83d2c0c6ed95f911884f9e253b2c7519029efa7a1edbd969538e4", "system_info-1791633198-1", "AMD EPYC 7763 64-Core Processor"),
              "38049997476": Run("windows-2025", "3a8a76e25b85a37d5966fefda2d3c607a6bdf5baf1fc885b46f8d5442e86ce97", "system_info-1791633198-2", "INTEL(R) XEON(R) PLATINUM 8573C"),
              "38049999396": Run("windows-2022", "d253082973f83d2c0c6ed95f911884f9e253b2c7519029efa7a1edbd969538e4", "system_info-1791633198-3", "AMD EPYC 7763 64-Core Processor"),
        },
        documented=(
            documented("ms-getversionexw", "api/sysinfoapi/nf-sysinfoapi-getversionexw", "2024-02-22",
                       "applications not manifested for Windows 8.1 or Windows 10 will return the Windows 8 OS "
                       "version value (6.2)."),),
        open_questions=(
            "The runners are Windows Server: product type 3. A player's desktop answers 1 (VER_NT_WORKSTATION); w2m "
            "answers as the rows do until a desktop reference exists.",
            "Processor count, memory amounts and the tick count are machine values; the rows hold relations and w2m "
            "answers one processor, 16 GiB, and the guest counter's milliseconds."),
        plants={"two_processors_one_bit": Plant("    *(u32 *)(p + 32) = 1;", "    *(u32 *)(p + 32) = 2;",
                                                ("native_system_info", "system_info")),
                "version_build_off": Plant("    *(u32 *)(p + 12) = 9200;", "    *(u32 *)(p + 12) = 9600;", ("version_ex",))}),
    "virtual_memory": Family(
        name="virtual_memory",
        contract_id="windows.memory.virtual",
        version="0.1.0",
        summary=("VirtualAlloc, VirtualFree, VirtualProtect and VirtualQuery on one thread: 64 KB-aligned reservations "
                 "above 4 GB, zeroed commits, queries reporting the run of equal pages, the error codes (87, 50, 487), "
                 "and an image's pages (headers read-only, code execute-read, data copy-on-write until written)."),
        probe="virtual_memory", guest_file="virtual_memory.c",
        runs={
              "38050561886": Run("windows-2025", "3a51f28b7d6f9bef3dacbf83abce8a0020a43432201e9dbcfd3cdfc8662cc38a", "virtual_memory-1791633783-0", "AMD EPYC 7763 64-Core Processor"),
              "38050563770": Run("windows-2022", "1d665ea1088f700c8e027a5cb18d7f18d7c5b2352fa84c84695616063a153f08", "virtual_memory-1791633783-1", "AMD EPYC 7763 64-Core Processor"),
              "38050565681": Run("windows-2025", "3a51f28b7d6f9bef3dacbf83abce8a0020a43432201e9dbcfd3cdfc8662cc38a", "virtual_memory-1791633783-2", "AMD EPYC 7763 64-Core Processor"),
              "38050567245": Run("windows-2022", "1d665ea1088f700c8e027a5cb18d7f18d7c5b2352fa84c84695616063a153f08", "virtual_memory-1791633783-3", "AMD EPYC 9V74 80-Core Processor"),
              "38050618749": Run("windows-2025", "3a51f28b7d6f9bef3dacbf83abce8a0020a43432201e9dbcfd3cdfc8662cc38a", "virtual_memory-1791633842-0", "AMD EPYC 7763 64-Core Processor"),
              "38050620683": Run("windows-2022", "1d665ea1088f700c8e027a5cb18d7f18d7c5b2352fa84c84695616063a153f08", "virtual_memory-1791633842-1", "AMD EPYC 7763 64-Core Processor"),
              "38050622465": Run("windows-2025", "3a51f28b7d6f9bef3dacbf83abce8a0020a43432201e9dbcfd3cdfc8662cc38a", "virtual_memory-1791633842-2", "AMD EPYC 7763 64-Core Processor"),
              "38050624259": Run("windows-2022", "1d665ea1088f700c8e027a5cb18d7f18d7c5b2352fa84c84695616063a153f08", "virtual_memory-1791633842-3", "AMD EPYC 7763 64-Core Processor"),
              "38050676216": Run("windows-2025", "3a51f28b7d6f9bef3dacbf83abce8a0020a43432201e9dbcfd3cdfc8662cc38a", "virtual_memory-1791633900-0", "AMD EPYC 7763 64-Core Processor"),
              "38050678382": Run("windows-2022", "1d665ea1088f700c8e027a5cb18d7f18d7c5b2352fa84c84695616063a153f08", "virtual_memory-1791633900-1", "AMD EPYC 7763 64-Core Processor"),
              "38050680580": Run("windows-2025", "3a51f28b7d6f9bef3dacbf83abce8a0020a43432201e9dbcfd3cdfc8662cc38a", "virtual_memory-1791633900-2", "AMD EPYC 7763 64-Core Processor"),
              "38050682385": Run("windows-2022", "1d665ea1088f700c8e027a5cb18d7f18d7c5b2352fa84c84695616063a153f08", "virtual_memory-1791633900-3", "AMD EPYC 7763 64-Core Processor"),
        },
        documented=(
            documented("ms-virtualquery", "api/memoryapi/nf-memoryapi-virtualquery", "2024-02-22",
                       "The function determines the attributes of the first page in the region and then scans "
                       "subsequent pages until it scans the entire range of pages or until it encounters a page with "
                       "a nonmatching set of attributes."),),
        open_questions=(
            "Protections are recorded, not enforced: a store to a read-only page succeeds in w2m.",
            "A released region's size runs to the next allocation and differs between executions; the row keeps that "
            "it is at least the reservation.",
            "A writable image page counts as written when its bytes differ from load; a write of the same bytes is "
            "missed."),
        derive=virtual_memory_rows, host_files=("src/w2m/virtual_memory.py", "src/w2m/vm_backend.py"),
        plants={"bad_protect_parameter": Plant("            return 0, ERROR_NOT_SUPPORTED                    # row alloc.bad_protect (0x12345)",
                                                "            return 0, ERROR_INVALID_PARAMETER",
                                                ("alloc.bad_protect",), host="w2m.virtual_memory"),
                "old_unwritten_on_failure": Plant("    *(u32 *)old = (u32)second;                     /* written on failure too (row protect.reserved_page) */\n    if (!ok) set_error((u32)(second >> 32));",
                                                  "    if (ok) *(u32 *)old = (u32)second;\n    if (!ok) set_error((u32)(second >> 32));",
                                                  ("protect.reserved_page",))}),
    "sync_objects": Family(
        name="sync_objects",
        contract_id="windows.sync.objects",
        version="0.1.0",
        summary=("Unnamed semaphores, events and mutexes on one thread: creation (last error 0), zero-timeout waits, "
                 "release past the maximum (298), release when not owner (288), waits on several, CloseHandle twice "
                 "and a wait on a closed handle (6)."),
        probe="sync_objects", guest_file="sync_objects.c",
        runs={
              "38051398628": Run("windows-2025", "a564e428b9e44d62b809d084179a261b08927fad516d862a6f8339a29bf0bbf6", "sync_objects-1791634627-0", "AMD EPYC 7763 64-Core Processor"),
              "38051400336": Run("windows-2022", "530e281c3c7800ab121fa13cf1ed9f1d15e95bbef4bb7e806ff04bd961c4482c", "sync_objects-1791634627-1", "AMD EPYC 7763 64-Core Processor"),
              "38051401855": Run("windows-2025", "a564e428b9e44d62b809d084179a261b08927fad516d862a6f8339a29bf0bbf6", "sync_objects-1791634627-2", "INTEL(R) XEON(R) PLATINUM 8573C"),
              "38051403311": Run("windows-2022", "530e281c3c7800ab121fa13cf1ed9f1d15e95bbef4bb7e806ff04bd961c4482c", "sync_objects-1791634627-3", "AMD EPYC 9V45 96-Core Processor"),

        },
        documented=(
            documented("ms-releasesemaphore", "api/synchapi/nf-synchapi-releasesemaphore", "2024-02-22",
                       "If the specified amount would cause the semaphore's count to exceed the maximum count that "
                       "was specified when the semaphore was created, the count is not changed and the function "
                       "returns FALSE."),),
        open_questions=(
            "One thread: a finite wait on an unsignaled object returns WAIT_TIMEOUT at once in w2m (no thread can "
            "signal it); INFINITE waits, named objects and initially owned mutexes stop the run (Sprint 047).",),
        plants={"auto_event_stays_set": Plant("    else if (o->kind != EVENT_MANUAL) o->count--;",
                                              "    else if (o->kind == SEMAPHORE) o->count--;", ("event.auto.wait_consumed",)),
                "create_leaves_error": Plant("            set_error(0);                              /* rows *.create: err 0 */\n",
                                             "", ("semaphore.create", "event.manual.create", "event.auto.create_set",
                                                  "mutex.create"))}),
    "storage_get2": Family(
        name="storage_get2",
        contract_id="windows.thread_storage.get2",
        version="0.1.0",
        summary=("FlsGetValue2 and TlsGetValue2, which Server 2025 exports and 2022 does not: the values FlsGetValue and "
                 "TlsGetValue give, with the last error left; FlsGetValue on a freed index answers NULL with error 0."),
        probe="storage_get2", guest_file="thread_storage.c",
        runs={
              "38051903385": Run("windows-2025", "23a70c5de442cb5ebd8d6415efaf87b330d7e453ad9463651f0ff915a5f425a4", "storage_get2-1791635131-0", "AMD EPYC 7763 64-Core Processor"),
              "38051906733": Run("windows-2025", "23a70c5de442cb5ebd8d6415efaf87b330d7e453ad9463651f0ff915a5f425a4", "storage_get2-1791635131-2", "AMD EPYC 9V74 80-Core Processor"),
              "38051960386": Run("windows-2025", "23a70c5de442cb5ebd8d6415efaf87b330d7e453ad9463651f0ff915a5f425a4", "storage_get2-1791635187-0", "AMD EPYC 7763 64-Core Processor"),
              "38051963773": Run("windows-2025", "23a70c5de442cb5ebd8d6415efaf87b330d7e453ad9463651f0ff915a5f425a4", "storage_get2-1791635187-2", "AMD EPYC 7763 64-Core Processor"),
              "38052017832": Run("windows-2025", "23a70c5de442cb5ebd8d6415efaf87b330d7e453ad9463651f0ff915a5f425a4", "storage_get2-1791635244-0", "AMD EPYC 9V45 96-Core Processor"),
              "38052021003": Run("windows-2025", "23a70c5de442cb5ebd8d6415efaf87b330d7e453ad9463651f0ff915a5f425a4", "storage_get2-1791635244-2", "AMD EPYC 7763 64-Core Processor"),
              "38052073638": Run("windows-2025", "23a70c5de442cb5ebd8d6415efaf87b330d7e453ad9463651f0ff915a5f425a4", "storage_get2-1791635300-0", "AMD EPYC 9V74 80-Core Processor"),
              "38052076917": Run("windows-2025", "23a70c5de442cb5ebd8d6415efaf87b330d7e453ad9463651f0ff915a5f425a4", "storage_get2-1791635300-2", "AMD EPYC 9V74 80-Core Processor"),

        },
        documented=(
            documented("ms-tlsgetvalue2", "api/processthreadsapi/nf-processthreadsapi-tlsgetvalue2", "2024-09-04",
                       "This function is identical to TlsGetValue except that it doesn't set the thread's last error."),),
        open_questions=(
            "The runs are Server 2025's: Server 2022 exports neither function (its rows say so), and w2m's export list "
            "is Server 2025's (specs/windows_exports.json). FlsGetValue2 has no Microsoft Learn page.",),
        plants={"get2_sets_error": Plant("    return w2m_logged(W2M_API_FlsGetValue2, index, v);",
                                         "    set_error(0);\n    return w2m_logged(W2M_API_FlsGetValue2, index, v);",
                                         ("fls2.unset", "fls2.set", "fls2.freed"))}),
    "static_tls": Family(
        name="static_tls",
        contract_id="windows.loader.static_tls",
        version="0.1.0",
        summary=("A program without TLS importing a DLL with static TLS (built on the static C runtime, as UnityPlayer.dll "
                 "is): the DLL's TLS index is 1, TEB.ThreadLocalStoragePointer's slot holds the thread's copy of the "
                 "template with its zero fill, a write stays in the copy; the DLL's C runtime start-up runs and "
                 "AreFileApisANSI answers TRUE."),
        probe="static_tls", guest_file="code_pages.c",
        runs={
              "38052225205": Run("windows-2025", "b1d571baf3562ff9cf5c32951ab6654124865314a2e1db9ae68c3df84fe928d3", "static_tls-1791635442-0", "AMD EPYC 7763 64-Core Processor", dll="f4815320beb098eabba19eccfdb720cee5291af84ce1ccc5d784217383c329d5"),
              "38052227164": Run("windows-2022", "f9d9233c10ced3784aecdeba7de1de96b7931296d3548217383c4f6529069ada", "static_tls-1791635442-1", "AMD EPYC 9V74 80-Core Processor", dll="3f6b51cc422038c706dda36ef9d49002c8869402d7161f6d31e8f46aa0ba1ff2"),
              "38052228936": Run("windows-2025", "b1d571baf3562ff9cf5c32951ab6654124865314a2e1db9ae68c3df84fe928d3", "static_tls-1791635442-2", "AMD EPYC 7763 64-Core Processor", dll="f4815320beb098eabba19eccfdb720cee5291af84ce1ccc5d784217383c329d5"),
              "38052230688": Run("windows-2022", "f9d9233c10ced3784aecdeba7de1de96b7931296d3548217383c4f6529069ada", "static_tls-1791635442-3", "AMD EPYC 7763 64-Core Processor", dll="3f6b51cc422038c706dda36ef9d49002c8869402d7161f6d31e8f46aa0ba1ff2"),
              "38052283584": Run("windows-2025", "b1d571baf3562ff9cf5c32951ab6654124865314a2e1db9ae68c3df84fe928d3", "static_tls-1791635498-0", "AMD EPYC 9V74 80-Core Processor", dll="f4815320beb098eabba19eccfdb720cee5291af84ce1ccc5d784217383c329d5"),
              "38052285625": Run("windows-2022", "f9d9233c10ced3784aecdeba7de1de96b7931296d3548217383c4f6529069ada", "static_tls-1791635498-1", "AMD EPYC 7763 64-Core Processor", dll="3f6b51cc422038c706dda36ef9d49002c8869402d7161f6d31e8f46aa0ba1ff2"),
              "38052287519": Run("windows-2025", "b1d571baf3562ff9cf5c32951ab6654124865314a2e1db9ae68c3df84fe928d3", "static_tls-1791635498-2", "AMD EPYC 7763 64-Core Processor", dll="f4815320beb098eabba19eccfdb720cee5291af84ce1ccc5d784217383c329d5"),
              "38052289305": Run("windows-2022", "f9d9233c10ced3784aecdeba7de1de96b7931296d3548217383c4f6529069ada", "static_tls-1791635498-3", "AMD EPYC 9V74 80-Core Processor", dll="3f6b51cc422038c706dda36ef9d49002c8869402d7161f6d31e8f46aa0ba1ff2"),
              "38052338873": Run("windows-2025", "b1d571baf3562ff9cf5c32951ab6654124865314a2e1db9ae68c3df84fe928d3", "static_tls-1791635553-0", "AMD EPYC 9V45 96-Core Processor", dll="f4815320beb098eabba19eccfdb720cee5291af84ce1ccc5d784217383c329d5"),
              "38052340301": Run("windows-2022", "f9d9233c10ced3784aecdeba7de1de96b7931296d3548217383c4f6529069ada", "static_tls-1791635553-1", "AMD EPYC 7763 64-Core Processor", dll="3f6b51cc422038c706dda36ef9d49002c8869402d7161f6d31e8f46aa0ba1ff2"),
              "38052341746": Run("windows-2025", "b1d571baf3562ff9cf5c32951ab6654124865314a2e1db9ae68c3df84fe928d3", "static_tls-1791635553-2", "AMD EPYC 7763 64-Core Processor", dll="f4815320beb098eabba19eccfdb720cee5291af84ce1ccc5d784217383c329d5"),
              "38052343190": Run("windows-2022", "f9d9233c10ced3784aecdeba7de1de96b7931296d3548217383c4f6529069ada", "static_tls-1791635553-3", "AMD EPYC 7763 64-Core Processor", dll="3f6b51cc422038c706dda36ef9d49002c8869402d7161f6d31e8f46aa0ba1ff2"),
              "38052401476": Run("windows-2025", "b1d571baf3562ff9cf5c32951ab6654124865314a2e1db9ae68c3df84fe928d3", "static_tls-1791635614-0", "AMD EPYC 7763 64-Core Processor", dll="f4815320beb098eabba19eccfdb720cee5291af84ce1ccc5d784217383c329d5"),
              "38052403044": Run("windows-2022", "f9d9233c10ced3784aecdeba7de1de96b7931296d3548217383c4f6529069ada", "static_tls-1791635614-1", "AMD EPYC 7763 64-Core Processor", dll="3f6b51cc422038c706dda36ef9d49002c8869402d7161f6d31e8f46aa0ba1ff2"),
              "38052404525": Run("windows-2025", "b1d571baf3562ff9cf5c32951ab6654124865314a2e1db9ae68c3df84fe928d3", "static_tls-1791635614-2", "AMD EPYC 7763 64-Core Processor", dll="f4815320beb098eabba19eccfdb720cee5291af84ce1ccc5d784217383c329d5"),
              "38052406160": Run("windows-2022", "f9d9233c10ced3784aecdeba7de1de96b7931296d3548217383c4f6529069ada", "static_tls-1791635614-3", "AMD EPYC 7763 64-Core Processor", dll="3f6b51cc422038c706dda36ef9d49002c8869402d7161f6d31e8f46aa0ba1ff2"),
        },
        documented=(
            documented("ms-pe-tls", "../../windows/win32/debug/pe-format", "2024-02-22",
                       "The loader assigns the value of the TLS index to the place that was indicated by the Address "
                       "of Index field."),),
        open_questions=(
            "Index 0 belongs to the program even without a TLS directory (the DLL's index is 1); what its slot holds "
            "is not observed (w2m: NULL).",
            "Windows calls the DLL's entry with DLL_PROCESS_DETACH at ExitProcess; w2m does not yet, and no row reads it.",
            "The console start's standard input and error answer NULL and its standard output FILE_TYPE_DISK in w2m, "
            "which the DLL's C runtime asks at start-up; the rows do not read them."),
        dll_name="static_tls_dll.dll", host_files=("src/w2m/loader.py", "src/w2m/vm_backend.py"),
        # (a first plant, indices from 0 with no program slot, was named for tls.index and faulted the run: the DLL's
        # block moved to slot 0 while it read slot 1; recorded in Sprint 046's card, replaced by this one)
        plants={"template_not_copied": Plant("        block = bytes(region.data[start - image.base:end - image.base]) + bytes(zero_fill)",
                                            "        block = bytes(end - start) + bytes(zero_fill)",
                                            ("tls.values", "tls.write"), host="w2m.loader")}),
    "time_and_error_mode": time_family("parent", (
        "Local time on a UTC machine, the time zone (TIME_ZONE_ID_UNKNOWN, zero biases, error 0), exact SYSTEMTIME "
        "and FILETIME conversions and their invalid inputs (87), and the error mode a process inherits from the "
        "runner's cmd (0x8003) and sets.")),
    "time_and_error_mode_child": time_family("child", (
        "The error mode of a child whose parent set 0 and passed no flag: 0 (the Steam-like start).")),
    "console": Family(
        name="console",
        contract_id="windows.console.gui_start",
        version="0.2.0",
        summary=("A GUI process started without handles (as Steam starts the game) gets NULL standard handles, "
                 "no console, and Windows's answers from GetStdHandle, SetStdHandle, GetFileType, GetConsoleMode, "
                 "GetConsoleCP, WriteConsoleW, FlushFileBuffers, GetStartupInfoW, GetCommandLineW and GetCommandLineA."),
        probe="console", guest_file="console.c",
        runs={
              "38046095100": Run("windows-2025", "457d94e7c517df5a63f65a4ea3e0dae4bd815b96a87f356c0b94961e3f2c3af6", "console-1791629208-0", "AMD EPYC 7763 64-Core Processor"),
              "38046096985": Run("windows-2022", "db079f8142eb749edada1614469f9d12c97c2a3bf45ff3756923bac2490a2c45", "console-1791629208-1", "AMD EPYC 7763 64-Core Processor"),
              "38046099129": Run("windows-2025", "457d94e7c517df5a63f65a4ea3e0dae4bd815b96a87f356c0b94961e3f2c3af6", "console-1791629208-2", "INTEL(R) XEON(R) PLATINUM 8573C"),
              "38046100953": Run("windows-2022", "db079f8142eb749edada1614469f9d12c97c2a3bf45ff3756923bac2490a2c45", "console-1791629208-3", "AMD EPYC 7763 64-Core Processor"),
        },
        documented=(),
        open_questions=(
            "The rows come from a child the probe starts with no inherited handles and an empty STARTUPINFO; that "
            "Steam starts the game the same way is the BLACKBOARD's assumption of 2026-10-10 (revisit with Steam in "
            "the local VM).",
            "GetStartupInfoW leaves hStdInput, hStdOutput and hStdError unwritten when the start passed no handles: "
            "the probe's fill survives (row startup_info.child.values).",
            "The title's length is the runner's path and is dropped from the rows; the title being the module path is kept."),
        derive=console_rows, gui=True, arguments="child", output_file="w2m-console-child.jsonl",
        plants={"write_console_wrong_error": Plant("    set_error(ERROR_INVALID_HANDLE);\n    return w2m_logged(api, arg, 0);",
                                                   "    set_error(87);\n    return w2m_logged(api, arg, 0);",
                                                   # no_console serves GetConsoleMode too: the std rows' console_mode_err
                                                   ("std.input", "std.output", "std.error", "console_cp",
                                                    "write_console.no_console")),
                "file_type_null_no_error": Plant("    if (handle == 0 || handle == ~0ull) {\n        set_error(ERROR_INVALID_HANDLE);",
                                                 "    if (handle == 0 || handle == ~0ull) {",
                                                 ("std.input", "std.output", "std.error"))}),
    "command_line_args": Family(
        name="command_line_args",
        contract_id="windows.shell.command_line_to_argv",
        version="0.1.0",
        summary=("SHELL32's CommandLineToArgvW on 24 lines over both rule sets (the program name: quoted to the next "
                 "quote, else to the first blank with quotes literal; later arguments: the backslash-quote rules and a "
                 "doubled quote inside quotes), the empty line (the module's path), a NULL count pointer (NULL, error "
                 "87); one LocalAlloc block, the strings after the pointer array and its NULL; LocalFree returns NULL; "
                 "success sets the last error to 0."),
        probe="command_line_args", guest_file="command_line_args.c",
        runs={
              "38084317468": Run("windows-2025", "05aab19d035c00a13cfb0216748ff247e743da60b2b9319d9bc787004601a77f", "command_line_args-1791664503-0", "AMD EPYC 7763 64-Core Processor"),
              "38084319695": Run("windows-2022", "5e2d2107d657a04f095b2d27d26d204e35ef691662e8d233ede9d3205dc77787", "command_line_args-1791664503-1", "AMD EPYC 7763 64-Core Processor")},
        documented=(
            documented("ms-commandlinetoargvw", "api/shellapi/nf-shellapi-commandlinetoargvw", "2018-12-05",
                       "2n backslashes followed by a quotation mark produce n backslashes followed by begin/end quote. "
                       "... (2n) + 1 backslashes followed by a quotation mark again produce n backslashes followed by "
                       "a quotation mark literal (\"). ... if lpCmdLine starts with any amount of whitespace, "
                       "CommandLineToArgvW will consider the first argument to be an empty string. Excess whitespace "
                       "at the end of lpCmdLine is ignored."),),
        open_questions=(
            "The page does not say that the program name takes quotes literally (input.09), that a doubled quote "
            "inside quotes is one literal quote (input.06, input.15), or that success sets the last error to 0; the "
            "rows do.",
            "Only ASCII lines are probed; the game's command line is ASCII (w2m's GUI start)."),
        plants={
            "program_name_honours_quotes": Plant(
                "        for (; *p && !blank(*p); p++) { if (out) out[n] = *p; n++; }",
                "        for (int q = 0; *p && (q || !blank(*p)); p++) { if (*p == '\"') { q = !q; continue; } "
                "if (out) out[n] = *p; n++; }",
                ("input.09",)),
            "doubled_quote_toggles": Plant(
                "                if (quoted && p[1] == '\"') { if (out) out[n] = '\"'; n++; p += 2; continue; }\n", "",
                ("input.06", "input.15")),
            "success_leaves_error": Plant(
                "    w2m_set_last_error(0);                               /* every row: the last error is 0 after success */\n",
                "", ("input.*",))}),
    "generate_random": Family(
        name="generate_random",
        contract_id="windows.bcrypt.gen_random",
        version="0.1.0",
        summary=("bcrypt's BCryptGenRandom: a NULL handle with BCRYPT_USE_SYSTEM_PREFERRED_RNG succeeds (length 0 too; the "
                 "entropy-in-buffer flag beside it accepted); a NULL buffer, another flag bit or a handle with the system "
                 "flag is STATUS_INVALID_PARAMETER; a NULL handle without it STATUS_INVALID_HANDLE; bytes past the length "
                 "kept; the last error untouched; two draws differ and 64 KB hold every byte value."),
        probe="generate_random", guest_file="bcrypt.c", host_files=("src/w2m/vm_backend.py",),
        runs={
              "38085850021": Run("windows-2025", "4fb5e3ac666bfe211b8fdd30d94a94c9617ec850291c646c439b676aa6d64934", "generate_random-1791665930-0", "AMD EPYC 7763 64-Core Processor"),
              "38085852392": Run("windows-2022", "306d0d2ade5c08a2ed7dad9f327f09d978792cc1e545256790701bd3089a363f", "generate_random-1791665930-1", "AMD EPYC 7763 64-Core Processor")},
        documented=(
            documented("ms-bcryptgenrandom", "api/bcrypt/nf-bcrypt-bcryptgenrandom", "2018-12-05",
                       "BCRYPT_USE_SYSTEM_PREFERRED_RNG ... Use the system-preferred random number generator algorithm. "
                       "The hAlgorithm parameter must be NULL. ... STATUS_INVALID_HANDLE: The handle in the hAlgorithm "
                       "parameter is not valid. STATUS_INVALID_PARAMETER: One or more parameters are not valid."),),
        open_questions=(
            "An algorithm handle from BCryptOpenAlgorithmProvider is not probed; the game passes NULL with the system flag.",
            "The page does not say that a NULL buffer with length 0 fails or that the last error is untouched; the rows do."),
        plants={
            "null_buffer_with_zero_length_succeeds": Plant(
                "    if (status == STATUS_SUCCESS && !buffer) status = STATUS_INVALID_PARAMETER;",
                "    if (status == STATUS_SUCCESS && !buffer && length) status = STATUS_INVALID_PARAMETER;",
                ("system_rng.null_buffer_0",)),
            "handle_check_swapped": Plant(
                "    else if (!handle) status = STATUS_INVALID_HANDLE;",
                "    else if (!handle) status = STATUS_INVALID_PARAMETER;",
                ("null_handle.no_flags",))}),
    "processor_topology": Family(
        name="processor_topology",
        contract_id="windows.system.processor_topology",
        version="0.1.0",
        summary=("GetLogicalProcessorInformationEx over every relationship (sizes, errors, entry sizes 48, 56 and 80, masks "
                 "as the active processors), the classic call's 32-byte entries, GetActiveProcessorCount, "
                 "GetProcessAffinityMask, SetThreadAffinityMask, GetNumaHighestNodeNumber and GetCurrentProcess; the rows "
                 "agree on Intel Xeon 8573C and 6973P and AMD EPYC 7763 and differ only by OS, in relationship 7."),
        probe="processor_topology", guest_file="topology.c",
        runs={
              "38086761220": Run("windows-2025", "08fd4d30db6ca50556d9543ce274921c6a447adf1c3b4f0682477953b119dc75", "processor_topology-1791666778-0", "INTEL(R) XEON(R) PLATINUM 8573C"),
              "38086763271": Run("windows-2022", "08e3d2c9420d640bff4a643fe57eefbbde28da5e2933e5348c5a1b3a792c5f72", "processor_topology-1791666778-1", "AMD EPYC 7763 64-Core Processor"),
              "38086765021": Run("windows-2025", "08fd4d30db6ca50556d9543ce274921c6a447adf1c3b4f0682477953b119dc75", "processor_topology-1791666778-2", "AMD EPYC 7763 64-Core Processor"),
              "38086767249": Run("windows-2022", "08e3d2c9420d640bff4a643fe57eefbbde28da5e2933e5348c5a1b3a792c5f72", "processor_topology-1791666778-3", "Intel(R) Xeon(R) 6973P-C"),
        },
        documented=(
            documented("ms-getlogicalprocessorinformationex", "api/sysinfoapi/nf-sysinfoapi-getlogicalprocessorinformationex",
                       "2023-02-27",
                       "If the buffer is not large enough to contain all of the data, the function fails, GetLastError "
                       "returns ERROR_INSUFFICIENT_BUFFER, and ReturnedLength is set to the buffer length required to "
                       "contain all of the data."),),
        open_questions=(
            "The machine is w2m's choice where the rows give relations: one processor, one core without SMT, the "
            "i3-6100's caches of the guest CPUID profile. A game that sizes its thread pool from these sees one core.",
            "Relationship 7 (processor module) exists on Server 2025 and fails with error 31 on 2022; w2m answers as "
            "2025 does."),
        derive=topology_rows,
        excluded={"ex.size_query.7": "Server 2022 fails it with ERROR_GEN_FAILURE (31); 2025 answers",
                  "ex.short_buffer.7": "Server 2022 has no relationship 7",
                  "ex.query.7": "Server 2022 has no relationship 7"},
        plants={
            "short_buffer_leaves_length": Plant(
                "    if (!buffer || *(u32 *)length < need) {\n        *(u32 *)length = need;\n        set_error(ERROR_INSUFFICIENT_BUFFER);\n        return w2m_logged(W2M_API_GetLogicalProcessorInformationEx",
                "    if (!buffer || *(u32 *)length < need) {\n        set_error(ERROR_INSUFFICIENT_BUFFER);\n        return w2m_logged(W2M_API_GetLogicalProcessorInformationEx",
                ("ex.size_query.*", "ex.short_buffer.*", "ex.query.*")),
            "cache_entry_48": Plant("    return 56;\n}", "    return 48;\n}", ("ex.query.2", "ex.query.65535")),
            "zero_mask_accepted": Plant("    if (!mask || (mask & ~1ull)) {", "    if (mask & ~1ull) {", ("thread_affinity.zero",))}),
    "file_attributes": Family(
        name="file_attributes",
        contract_id="windows.file.attributes",
        version="0.1.0",
        summary=("GetFileAttributesExW and GetFileAttributesW over a 5-byte file (any case, trailing dot or space), the "
                 "working directory (0x10, size 4096), the empty name and missing paths (3), a missing file (2), "
                 "wildcards (123) and an invalid level (87); times as relations to the clock; success leaves the last "
                 "error and failure the caller's structure."),
        probe="file_attributes", guest_file="file_attributes.c",
        host_files=("src/w2m/file_bridge.py", "tools/guest_runtime/file.c"),
        runs={
              "38087355443": Run("windows-2025", "f7c82e7ade1ebdbea2207111490e87681ba1d74256df2a614cc5237d41e9a334", "file_attributes-1791667332-0", "AMD EPYC 9V74 80-Core Processor"),
              "38087357359": Run("windows-2022", "28cb58d08437a23643ec9661d294e90a72617def99961475c27e11c04516e248", "file_attributes-1791667332-1", "AMD EPYC 9V74 80-Core Processor"),
        },
        documented=(
            documented("ms-getfileattributesexw", "api/fileapi/nf-fileapi-getfileattributesexw", "2018-12-05",
                       "If the function succeeds, the return value is a nonzero value. If the function fails, the "
                       "return value is zero (0). To get extended error information, call GetLastError."),),
        open_questions=(
            "A file named with a trailing backslash: ERROR_DIRECTORY (267) on Server 2025, ERROR_INVALID_NAME (123) on "
            "2022; excluded, and w2m answers as 2025.",
            "Absolute paths (the game asks about C:\\w2m\\app\\Data\\...) need the guest's drive namespace; the "
            "probe names files relative to its working directory."),
        excluded={"ex.file.trailing_backslash": "Server 2025: ERROR_DIRECTORY (267); 2022: ERROR_INVALID_NAME (123)",
                  "plain.file.trailing_backslash": "Server 2025: ERROR_DIRECTORY (267); 2022: ERROR_INVALID_NAME (123)"},
        plants={
            "directory_size_zero": Plant("    if (kind == 2) st[0] = 4096;", "    if (kind == 2) st[0] = 0;",
                                         ("ex.dot", "ex.dot.backslash")),
            "empty_name_not_found": Plant("    if (!s[0]) return ERROR_PATH_NOT_FOUND;", "    if (!s[0]) return ERROR_FILE_NOT_FOUND;",
                                          ("ex.empty", "plain.empty")),
            "wildcard_looked_up": Plant("        if (s[n] == '*' || s[n] == '?') return ERROR_INVALID_NAME;", "        ;",
                                        ("ex.wildcard", "plain.wildcard"))}),
    "paths": Family(
        name="paths",
        contract_id="windows.file.paths",
        version="0.1.0",
        summary=("GetCurrentDirectoryW, GetFullPathNameW over 26 names (relative, rooted, drive-relative, other drives, "
                 "\\\\?\\, UNC, dots, slashes, blanks, wildcards), SetCurrentDirectoryW, CreateDirectoryW and "
                 "RemoveDirectoryW with their errors; paths printed against <cwd>, <parent> and <root>."),
        probe="paths", guest_file="paths.c",
        host_files=("src/w2m/file_bridge.py", "tools/guest_runtime/file.c"),
        runs={
              "38090281974": Run("windows-2025", "a720b00e8a38dcb0482217ade8c02b84281782fa011fafcd9f8fa613ef59bdd1", "paths-1791670095-0", "INTEL(R) XEON(R) PLATINUM 8573C"),
              "38090284442": Run("windows-2022", "22e3d64701297ce3aa758740cfd378a143b49475321683ed7bfbf2d4e29ba2a4", "paths-1791670095-1", "AMD EPYC 7763 64-Core Processor"),
        },
        documented=(
            documented("ms-getfullpathnamew", "api/fileapi/nf-fileapi-getfullpathnamew", "2018-12-05",
                       "GetFullPathName merges the name of the current drive and directory with a specified file name "
                       "to determine the full path and file name of a specified file. ... This function does not verify "
                       "that the resulting path and file name are valid, or that they see an existing file on the "
                       "associated volume."),),
        open_questions=(
            "\"x\\CON\": a device (\\\\.\\CON) on Server 2022, an ordinary name on 2025; excluded, and w2m answers as 2025.",
            "Each drive's own working directory (\"Q:x\" while on C:) is not modelled; the rows show Q:\\x, as for a "
            "drive never visited."),
        derive=paths_rows,
        excluded={"full.device_name": "Server 2022 maps x\\CON to the device \\\\.\\CON; 2025 does not"},
        plants={
            "inner_dots_kept": Plant(
                "        while (len && (in[start + len - 1] == '.' || in[start + len - 1] == ' ')) len--;",
                "        while (!in[i] && len && (in[start + len - 1] == '.' || in[start + len - 1] == ' ')) len--;",
                ("full.inner_trailing_dot",)),
            "dotdot_past_root": Plant(
                "            if (n > root) {                             /* drop the last \"component\\\" */",
                "            if (n > 0) {                                /* drop the last \"component\\\" */",
                ("full.rooted_dotdot",)),
            "working_directory_removed": Plant(
                "    if (same(scratch, working())) {", "    if (same(scratch, working()) && 0) {", ("dir.remove_cwd",))}),
    "file_security": Family(
        name="file_security",
        contract_id="windows.file.security",
        version="0.1.0",
        summary=("CreateFileW with SECURITY_ATTRIBUTES and no descriptor: nLength unchecked (0, 12, 1000 open), "
                 "bInheritHandle nonzero sets HANDLE_FLAG_INHERIT (GetHandleInformation), the open's result and error "
                 "as with no attributes."),
        probe="file_security", guest_file="file.c", host_files=("src/w2m/file_bridge.py", "tools/guest_runtime/paths.c"),
        runs={
              "38090863707": Run("windows-2025", "54f99a38848d08f519159a5e922708481f6859da345febe832e457e753c46364", "file_security-1791670645-0", "AMD EPYC 7763 64-Core Processor"),
              "38090865352": Run("windows-2022", "30a17799849fecb415ae48da8d6a5ca42c71478579f4b263c9b007c5f245d8e8", "file_security-1791670645-1", "AMD EPYC 7763 64-Core Processor"),
        },
        documented=(
            documented("ms-createfilew", "api/fileapi/nf-fileapi-createfilew", "2022-08-16",
                       "lpSecurityAttributes ... If this parameter is NULL, the handle returned by CreateFile cannot be "
                       "inherited by any child processes the application may create and the file or device associated "
                       "with the returned handle gets a default security descriptor."),),
        open_questions=(
            "A security descriptor is not probed; w2m stops on one by name.",
            "Inheritance matters only to a child process; w2m creates none yet (CreateProcessW stops by name)."),
        plants={
            "inherit_ignored": Plant("    u32 inherit = security ? *(u32 *)(security + 16) != 0 : 0;", "    u32 inherit = 0;",
                                     ("inherit.existing", "length_*.existing", "inherit_2.existing")),
            "length_checked": Plant(
                "    if (security && *(u64 *)(security + 8)) w2m_unimplemented(W2M_API_CreateFileW, security);",
                "    if (security && *(u64 *)(security + 8)) w2m_unimplemented(W2M_API_CreateFileW, security);\n"
                "    if (security && *(u32 *)security != 24) { set_error(87); return w2m_logged(W2M_API_CreateFileW, name, INVALID_HANDLE_VALUE); }",
                ("length_*",))}),
    "local_free": Family(
        name="local_free",
        contract_id="windows.memory.local_free",
        version="0.1.0",
        summary=("LocalFree as UnityPlayer calls it on argv + 1: NULL and a block's start answer NULL; a pointer 8 bytes "
                 "past a 16-byte boundary (argv + 1, a HeapAlloc block + 8) answers itself with ERROR_INVALID_HANDLE and "
                 "the block stays intact; the last error is left on success. A pointer 16 bytes in ends the process "
                 "(STATUS_HEAP_CORRUPTION), recorded by the probe's parent and excluded."),
        probe="local_free", guest_file="local_memory.c", arguments="safe",
        runs={
              "38092775612": Run("windows-2025", "ac9257dab2fa346fe224a45462b87a9d2ad1ad687f37f4b065a9fb336906f44c", "local_free-1791672496-0", "AMD EPYC 9V45 96-Core Processor"),
              "38092777429": Run("windows-2022", "ac655d2145982f7850a7e9c30d8afab979bcc651e15faf68f3bb8d9c99be395d", "local_free-1791672496-1", "AMD EPYC 7763 64-Core Processor"),
        },
        documented=(
            documented("ms-localfree", "api/winbase/nf-winbase-localfree", "2018-12-05",
                       "If the function succeeds, the return value is NULL. If the function fails, the return value is "
                       "equal to a handle to the local memory object. To get extended error information, call "
                       "GetLastError."),),
        open_questions=(
            "Windows ends the process for a 16-aligned pointer inside a block (exit.argv_plus_2, 0xC0000374); w2m stops "
            "the run there by name instead.",),
        excluded={"exit.safe": "the parent's row: w2m runs the probe's safe child, which prints no exit codes",
                  "exit.argv_plus_2": "the parent's row: the 16-byte case ends its child with STATUS_HEAP_CORRUPTION"},
        plants={
            "interior_error_left": Plant("        w2m_set_last_error(6);\n", "",
                                         ("argv_plus_1", "heap_block_plus_8")),
            "interior_answers_null": Plant("        return w2m_logged(W2M_API_LocalFree, block, block);",
                                           "        return w2m_logged(W2M_API_LocalFree, block, 0);",
                                           ("argv_plus_1", "heap_block_plus_8"))}),
    "file_access": Family(
        name="file_access",
        contract_id="windows.file.access",
        version="0.1.0",
        summary=("CreateFileW's access masks (13): reading with GENERIC_READ, FILE_GENERIC_READ or FILE_READ_DATA; writing "
                 "with GENERIC_WRITE, FILE_GENERIC_WRITE, FILE_WRITE_DATA or FILE_APPEND_DATA; both with FILE_ALL_ACCESS, "
                 "GENERIC_ALL or MAXIMUM_ALLOWED, which also carry DELETE into sharing; FILE_READ_ATTRIBUTES, SYNCHRONIZE "
                 "and 0 open with no data access and no part in sharing."),
        probe="file_access", guest_file="file.c", host_files=("src/w2m/file_bridge.py", "tools/guest_runtime/paths.c"),
        runs={
              "38093116676": Run("windows-2025", "0ee5b17f1b2961169e640617187689155381e46c2a8207c2966ffd6a76aa885e", "file_access-1791672842-0", "AMD EPYC 7763 64-Core Processor"),
              "38093118511": Run("windows-2022", "96bb232d78a05bf731dde8f6eba9671e7d06bcdf1ef95524aebf39923c3af8da", "file_access-1791672842-1", "AMD EPYC 7763 64-Core Processor"),
        },
        documented=(
            documented("ms-file-access-rights", "fileio/file-access-rights-constants", "2018-05-31",
                       "FILE_APPEND_DATA ... For a file object, the right to append data to the file."),),
        open_questions=(
            "ReadFile and WriteFile are checked against data access only; the attribute rights' own calls "
            "(GetFileInformationByHandle, SetFileAttributesW) are not served yet.",
            "An append-only handle (FILE_APPEND_DATA without FILE_WRITE_DATA) writes at the end of the file, as the "
            "access-rights page says; no row reads the file back, so that rule rests on the page alone."),
        plants={
            "append_gives_no_write": Plant("(desired & (GENERIC_WRITE | FILE_WRITE_DATA | FILE_APPEND_DATA) ? W : 0)",
                                           "(desired & (GENERIC_WRITE | FILE_WRITE_DATA) ? W : 0)",
                                           ("open.append_data", "share.none.append_data", "share.read.append_data")),
            "maximum_without_delete": Plant("    u32 all = desired & (GENERIC_ALL | MAXIMUM_ALLOWED) ? R | W | D : 0;",
                                            "    u32 all = desired & GENERIC_ALL ? R | W | D : desired & MAXIMUM_ALLOWED ? R | W : 0;",
                                            ("share.read_write.maximum_allowed",))}),
    "system_modules": Family(
        name="system_modules",
        contract_id="windows.module.system",
        version="0.1.0",
        summary=("A system DLL imported statically (SHELL32): LoadLibraryW and GetModuleHandleW find it under any "
                 "spelling (case, no extension, full path); GetModuleFileNameW gives C:\\Windows\\System32\\SHELL32.dll; "
                 "GetProcAddress gives the import's address for an imported name, an address for another export, NULL "
                 "and 127 for a name it does not export; FreeLibrary succeeds and the module stays."),
        probe="system_modules", guest_file="module.c", host_files=("src/w2m/modules.py", "src/w2m/vm_backend.py"),
        runs={
              "38094160385": Run("windows-2025", "202f2b6b77d3d16a726ca95179127ab97bb20c6046018bb51cfdcdf89827a815", "system_modules-1791673893-0", "AMD EPYC 7763 64-Core Processor"),
              "38094162181": Run("windows-2022", "fac0b4e6a2d2de1aaea8d131e2cbe342c0940cff819e154fbd11940add2ec7ff", "system_modules-1791673893-1", "AMD EPYC 7763 64-Core Processor"),
        },
        documented=(
            documented("ms-getmodulehandlew", "api/libloaderapi/nf-libloaderapi-getmodulehandlew", "2024-12-09",
                       "Retrieves a module handle for the specified module. The module must have been loaded by the "
                       "calling process."),),
        open_questions=(
            "The system DLLs are not in PEB.Ldr's lists in w2m; Windows lists them. Nothing on the frontier walks the "
            "lists for them yet.",
            "Export names are Server 2025's (specs/windows_exports.json, run 38094046782); 2022 differs in counts "
            "(user32 1,007 against 1,041).",),
        plants={
            "loadlibraryw_not_found": Plant("    u64 h = find(name, 1);\n    if (!h) set_error(ERROR_MOD_NOT_FOUND);\n    return w2m_logged(W2M_API_LoadLibraryW",
                                            "    u64 h = 0;\n    if (!h) set_error(ERROR_MOD_NOT_FOUND);\n    return w2m_logged(W2M_API_LoadLibraryW",
                                            ("load.lower", "load.upper", "load.no_extension", "load.full_path"))}),
    "known_folders": Family(
        name="known_folders",
        contract_id="windows.shell.known_folder_path",
        version="0.1.0",
        summary=("SHELL32's SHGetKnownFolderPath as UnityPlayer calls it (FOLDERID_LocalAppDataLow, flags 0, no token): "
                 "S_OK and a new block holding <profile>\\AppData\\LocalLow, an existing directory, no trailing "
                 "backslash; the profile is C:\\Users\\<user>; LocalAppData and RoamingAppData are AppData\\Local and "
                 "AppData\\Roaming; KF_FLAG_DONT_VERIFY and KF_FLAG_CREATE give the same path; ole32's CoTaskMemFree "
                 "frees it and leaves the last error, NULL included."),
        probe="known_folders", guest_file="known_folders.c", host_files=("src/w2m/file_bridge.py",),
        derive=lambda printed: [{k: v for k, v in r.items() if not ("hr" in r and k == "err")} for r in printed],
        runs={
              "38098265812": Run("windows-2025", "93fdcb0f8270b50ff9a2b03c38926f3ab49945806eca0c43fc4d53302cb2309a", "known_folders-1791678212-0", "AMD EPYC 9V45 96-Core Processor"),
              "38098267402": Run("windows-2022", "15d1d468c6c4e878e6eb1d5d5d20fde4dd250f793eb65915a75117026c5e1d0d", "known_folders-1791678212-1", "INTEL(R) XEON(R) PLATINUM 8573C"),
        },
        documented=(
            documented("ms-shgetknownfolderpath", "api/shlobj_core/nf-shlobj_core-shgetknownfolderpath", "2018-12-05",
                       "The calling process is responsible for freeing this resource once it is no longer needed by "
                       "calling CoTaskMemFree, whether SHGetKnownFolderPath succeeds or not. The returned path does "
                       "not include a trailing backslash. ... Not all KNOWNFOLDERID values are present on all "
                       "systems."),
            documented("ms-knownfolderid", "shell/knownfolderid", "2020-07-27",
                       "FOLDERID_LocalAppDataLow ... GUID {A520A1A4-1780-4FF6-BD18-167343C5AF16} ... Default Path "
                       "%USERPROFILE%\\AppData\\LocalLow")),
        open_questions=(
            "The last error after success is not part of the contract (the derive drops `err` from the "
            "SHGetKnownFolderPath rows): Windows leaves 1008 (ERROR_NO_TOKEN) for the profile, Local, Roaming and "
            "LocalLow with KF_FLAG_DONT_VERIFY, 0 for LocalLow, 183 after KF_FLAG_CREATE, on all four executions; "
            "the rows show no rule, and w2m leaves the last error as it was.",
            "w2m's profile is C:\\Users\\w2m on a drive made fresh each run: nothing the game writes there "
            "outlives the run.",
            "Excluded rows: see sections.excluded."),
        excluded={"unregistered": "an unregistered GUID: 0x80070002, *ppszPath NULL, last error 0. Which GUIDs a "
                  "system registers varies, so w2m serves its four folders and stops at any other GUID; the probe "
                  "asks last",
                  "unregistered.create": "as unregistered, with KF_FLAG_CREATE",
                  "proc": "GetProcAddress(shell32, \"SHGetKnownFolderPath\") is not the import's address on Windows, "
                  "though SHELL32 exports the name itself, not as a forwarder (exports run 38094046782); the cause is "
                  "not established. w2m answers the import's address, as Windows does for CommandLineToArgvW "
                  "(contracts/windows.module.system); nothing on the frontier compares the two"},
        expected_stop="Unimplemented: SHGetKnownFolderPath",
        plants={
            "trailing_backslash": Plant("'L', 'o', 'w', 0};", "'L', 'o', 'w', '\\\\', 0};",
                                        ("local_low", "local_low.dont_verify", "local_low.create")),
            "free_sets_error": Plant("    if (p) w2m_heap_release(W2M_API_CoTaskMemFree, p);",
                                     "    if (p) w2m_heap_release(W2M_API_CoTaskMemFree, p);\n    w2m_set_last_error(0);",
                                     ("free.null", "free.path"))}),
    "temp_path": Family(
        name="temp_path",
        contract_id="windows.environment.temp_path",
        version="0.1.0",
        summary=("GetTempPathW over an environment the probe sets: the first of TMP, TEMP and USERPROFILE made full "
                 "(relative, '/', '..') with one trailing backslash, a missing directory included, else C:\\Windows\\; "
                 "a 303-character TMP is skipped; an empty TMP answers 1 and writes nothing; a short buffer gets "
                 "size - 1 units and a NUL, the answer the size needed. GetEnvironmentVariableW: case-insensitive "
                 "names, 203 for a missing one, a NUL at [0] of a short buffer, 0 for an empty value. "
                 "SetEnvironmentVariableW: 87 for an empty name or '=' inside one, TRUE for deleting a missing name, "
                 "a replaced name keeps its spelling, new entries in upper-cased name order. The last error is left "
                 "on every success."),
        probe="temp_path", guest_file="environment.c",
        derive=lambda printed: [{k: v for k, v in r.items() if not (r.get("s") == "temp.relative" and k == "ret")}
                                for r in printed],
        runs={
              "38098847298": Run("windows-2025", "ff7714fa701c3870ceac7080865aaac604719f71d7318a1ca00b371236dfdba1", "temp_path-1791678823-0", "AMD EPYC 9V45 96-Core Processor"),
              "38098848806": Run("windows-2022", "4c25a456e35e5b7cf5461baa1e2a1dba6c79451c8e5b6fbc3e932ef36d3f3e20", "temp_path-1791678823-1", "AMD EPYC 7763 64-Core Processor"),
        },
        documented=(
            documented("ms-gettemppathw", "api/fileapi/nf-fileapi-gettemppathw", "2018-12-05",
                       "The GetTempPath function checks for the existence of environment variables in the following "
                       "order and uses the first path found: The path specified by the TMP environment variable. The "
                       "path specified by the TEMP environment variable. The path specified by the USERPROFILE "
                       "environment variable. The Windows directory. Note that the function does not verify that the "
                       "path exists ... The maximum possible return value is MAX_PATH+1 (261)."),
            documented("ms-getenvironmentvariablew", "api/processenv/nf-processenv-getenvironmentvariablew",
                       "2022-08-05", "If lpBuffer is not large enough to hold the data, the return value is the buffer "
                       "size, in characters, required to hold the string and its terminating null character and the "
                       "contents of lpBuffer are undefined. ... If the specified environment variable was not found in "
                       "the environment block, GetLastError returns ERROR_ENVVAR_NOT_FOUND."),
            documented("ms-setenvironmentvariablew", "api/processenv/nf-processenv-setenvironmentvariablew",
                       "2022-08-05", "If this parameter is NULL, the variable is deleted from the current process's "
                       "environment.")),
        open_questions=(
            "temp.relative's return is the length of a path under the runner's working directory; the derive drops "
            "it and keeps the path (<cwd>\\rel\\).",
            "Where a long variable stops being skipped is not probed: w2m skips a value or path past MAX_PATH (260), "
            "from the page's maximum return of 261; the rows hold 303 (skipped) and 24 (used).",
            "An empty TMP answers 1 and writes nothing (temp.tmp_empty); the page says nothing of it, and w2m does "
            "the same for an empty TEMP or USERPROFILE without a row.",
            "Names compare by ASCII upper case in w2m; non-ASCII names are not probed.",
            "The first probe run faulted (0xC0000005) inside its own row writer: a runner's block held more than "
            "the 64 names one pass deleted, and the rest overflowed the output buffer (runs 38098431870, "
            "38098511094, 38098565802, 38098627044); the probe now deletes in passes."),
        plants={
            "temp_before_tmp": Plant("ORDER[] = {TMP, TEMP, PROFILE};", "ORDER[] = {TEMP, TMP, PROFILE};",
                                     # not temp.size_zero: it prints a length, and C:\t\ is as long as C:\m\
                                     ("temp.tmp", "temp.size_exact", "temp.size_short", "temp.tmp_backslash",
                                      "temp.relative", "temp.forward", "temp.dots", "temp.missing_dir",
                                      "temp.tmp_empty")),
            "replace_respells": Plant("        keep = e;\n", "",
                                      ("order.before", "order.after")),
            "new_entries_last": Plant("        if (c > 0) break;\n", "",
                                      ("order.before", "order.after"))}),
    "integrity_level": Family(
        name="integrity_level",
        contract_id="windows.token.integrity_level",
        version="0.1.0",
        summary=("UnityPlayer's low-integrity test: OpenProcessToken(current, TOKEN_QUERY) answers a handle, a "
                 "multiple of 4; GetTokenInformation(TokenIntegrityLevel) answers 122 and 28 bytes with no buffer or "
                 "one byte short, then a TOKEN_MANDATORY_LABEL (attributes 0x60, the SID at offset 16, revision 1, "
                 "one sub-authority, authority 16) whose RID is not low; GetSidSubAuthority answers the field's "
                 "address and last error 0; CloseHandle closes the token. LocalAlloc (LMEM_FIXED, LPTR) gives a "
                 "16-aligned process-heap block of the size asked, LPTR zeroed; LocalFree answers NULL."),
        probe="integrity_level", guest_file="token.c",
        host_files=("tools/guest_runtime/local_memory.c", "tools/guest_runtime/sync_objects.c"),
        derive=lambda printed: [({k: v for k, v in r.items() if k != "rid"} | {"low": r["rid"] == 0x1000})
                                if "rid" in r else r for r in printed],
        runs={
              "38099150504": Run("windows-2025", "6d84a44be6a006613661b693b1677cb0b096a17ceca4c5773975f329822f7456", "integrity_level-1791679146-0", "AMD EPYC 7763 64-Core Processor"),
              "38099152338": Run("windows-2022", "06e41e1e27336675a83949d6e2956998ed18b60a11742e5c006577a58091aac5", "integrity_level-1791679146-1", "AMD EPYC 9V45 96-Core Processor"),
        },
        documented=(
            documented("ms-mandatory-integrity-control", "secauthz/mandatory-integrity-control", "2025-07-08",
                       "Windows defines four integrity levels: low, medium, high, and system. Standard users receive "
                       "medium, elevated users receive high."),),
        open_questions=(
            "A runner's process is elevated and its RID is 0x3000 (high) on all four executions; w2m answers 0x2000 "
            "(medium), a standard user's level. The derive keeps whether the RID is low (0x1000), UnityPlayer's "
            "question at 0x180525de0.",),
        plants={
            "rid_low": Plant("#define MEDIUM_RID 0x2000u", "#define MEDIUM_RID 0x1000u", ("info",)),
            "sub_authority_keeps_error": Plant("    set_error(0);\n    return w2m_logged(W2M_API_GetSidSubAuthority",
                                               "    return w2m_logged(W2M_API_GetSidSubAuthority", ("sub_authority",)),
            "short_buffer_accepted": Plant("if ((u32)length < LABEL_SIZE || !info)", "if (!info)",
                                           ("short_buffer",))}),
}


# ---- the pipeline ---------------------------------------------------------------------------

def runner_rows(f: Family) -> dict[tuple[str, int], list[dict[str, Any]]]:
    """Every runner execution's rows (each run made two)."""
    return {(run, i): f.rows(parse((ROOT / "store" / "probe-runs" / run / f"body-{i}.jsonl").read_bytes()))
            for run in f.runs for i in (1, 2)}


def build_contract(f: Family) -> dict[str, Any]:
    by_run = runner_rows(f)
    first = next(iter(by_run.values()))
    unequal = {k: [a["s"] for a, b in zip(first, v) if a != b] for k, v in by_run.items() if v != first}
    if unequal:
        raise SystemExit(f"runner rows differ: {unequal}")
    return {
        "contract_id": f.contract_id, "version": f.version, "status": "accepted", "summary": f.summary,
        "applicability": "; ".join(sorted({f"{r.image} ({r.cpu})" for r in f.runs.values()})),
        "roles": ["main"],
        "scenario": {"initial": {"process": "started by the runner's cmd with standard output redirected"},
                     "execution": {"main": f"probes/{f.probe}/{f.probe}.c, top to bottom"},
                     "observation": {"phase": "after each call"}},
        "acceptance": {"relation": "exact_rows", "key": "s", "rows": first},
        "evidence": [
            {"evidence_id": "runner-rows", "category": "observed",
             "source_ref": "; ".join(f"run {k}: {r.image}, {r.cpu}, probe sha256:{r.binary}" for k, r in f.runs.items()),
             "applicability": "the builds and CPUs observed",
             "execution_ids": [f"{r.execution}-{i}" for r in f.runs.values() for i in (1, 2)]},
            *f.documented],
        "open_questions": list(f.open_questions),
        "sections": {"probe": f"probes/{f.probe} (Sprint 044)", "implementation": f"tools/guest_runtime/{f.guest_file}",
                     "rows": "scripts/family_run.py " + f.name,
                     "excluded": "; ".join(f"{k}: {v}" for k, v in sorted(f.excluded.items())) or "none"}}


def contract(f: Family) -> Contract:
    return load_contract(f.contract_path)


def run_in_vm(binary: str, sources: dict[str, str] | None = None, f: Family | None = None,
              dll: str | None = None) -> tuple[bytes, str, float]:
    """A runner's probe binary in the managed VM with a fresh file-system root, started as `f` says;
    (its rows' bytes: standard output, or f.output_file; the stop; seconds)."""
    from w2m.vm_backend import VM
    t0 = time.monotonic()
    with tempfile.TemporaryDirectory(prefix="w2m-family-") as d:
        files = FileBridge(Path(d))
        line = None if f is None or not f.arguments else f'"C:\\w2m\\app\\probe.exe" {f.arguments}'
        if dll is None and f is not None and f.dll_name is not None:
            dll = next(r.dll for r in f.runs.values() if r.binary == binary)
        dll_bytes = None if dll is None else (ROOT / "store" / "binaries" / f"{dll}.dll").read_bytes()

        def dlls(name: str) -> bytes | None:
            return dll_bytes if f is not None and f.dll_name is not None and name.lower() == f.dll_name.lower() else None

        vm = VM(load((ROOT / "store" / "binaries" / f"{binary}.exe").read_bytes(), dlls, "probe.exe"), files=files,
                runtime_sources=sources, image_name="probe.exe",      # the runner ran it as probe.exe
                gui=f is not None and f.gui, command_line=line, environment=() if f is None else f.environment)
        try:
            r = vm.run(max_steps=1 << 40, timeout_ms=60_000)    # the clock probes spin until time advances
        finally:
            vm.close()
            files.close_all()
        out = r.stdout
        if f is not None and f.output_file is not None:
            written = Path(d) / f.output_file
            out = written.read_bytes() if written.exists() else b""
    return out, r.stop, time.monotonic() - t0


def derived_output(f: Family, out: bytes) -> bytes:
    return b"".join(json.dumps(r, separators=(",", ":")).encode() + b"\n" for r in f.rows(parse(out)))


def unequal_rows(f: Family, out: bytes) -> list[str]:
    c = contract(f)
    assert isinstance(c.acceptance, ExactRows)
    got = {r.get("s"): r for r in f.rows(parse(out))}
    return [r["s"] for r in c.acceptance.rows if got.get(r["s"]) != r]


def named_reach(f: Family, p: Plant) -> list[str]:
    """The contract rows a plant's reach names, its patterns expanded."""
    c = contract(f)
    assert isinstance(c.acceptance, ExactRows)
    return [r["s"] for r in c.acceptance.rows if any(fnmatch.fnmatchcase(r["s"], pat) for pat in p.reach)]


def candidate_sha256(f: Family, sources: dict[str, str] | None = None) -> str:
    """The candidate is the family's guest file, and its host files when it names any (in order, each
    preceded by its path): an edit to either makes a new candidate."""
    text = (sources or {}).get(f.guest_file, (GUEST / f.guest_file).read_text())
    for path in f.host_files:
        text += f"\0{path}\0" + (ROOT / path).read_text()
    return "sha256:" + hashlib.sha256(text.encode()).hexdigest()


@contextlib.contextmanager
def host_patched(module_name: str, old: str, new: str) -> Iterator[None]:
    """Run with a host module's source changed once (a planted fault in the loader's Python); restored after."""
    import w2m.vm_backend  # noqa: F401  (the importers exist before the patch, so they can be repointed)
    module = importlib.import_module(module_name)
    source = Path(module.__file__ or "").read_text()
    assert source.count(old) == 1, f"plant: its target text is not in {module_name} exactly once"
    saved = dict(module.__dict__)
    exec(compile(source.replace(old, new), module.__file__ or module_name, "exec"), module.__dict__)  # noqa: S102
    # a module that imported a name from this one (from w2m.virtual_memory import VirtualMemory) holds the old
    # object: point it at the patched one, and back after (Sprint 046: a patch leaked into the next plant)
    moved = {id(v): module.__dict__[k] for k, v in saved.items() if not k.startswith("__") and module.__dict__.get(k) is not v}
    rebound: list[tuple[Any, str, Any]] = []
    for name, other in list(sys.modules.items()):
        if other is module or not name.startswith("w2m") or other is None:
            continue
        for k, v in list(vars(other).items()):
            if id(v) in moved:
                rebound.append((other, k, v))
                setattr(other, k, moved[id(v)])
    try:
        yield
    finally:
        module.__dict__.clear()
        module.__dict__.update(saved)
        for other, k, v in rebound:
            setattr(other, k, v)


def planted_sources(f: Family, name: str) -> dict[str, str]:
    p = f.plants[name]
    text = (GUEST / f.guest_file).read_text()
    assert text.count(p.old) == 1, f"plant {name}: its target text is not in {f.guest_file} exactly once"
    return {f.guest_file: text.replace(p.old, p.new)}


def item_run(f: Family, runs: Path, sources: dict[str, str] | None = None,
             generator_ref: str = "hand (agent-authored)") -> dict[str, Any]:
    """Judge the family's guest C (or `sources`) on its own substrate record."""
    from substrate.api import Once, PerEvent, Subscription, TopologyBuilder, read_record, threshold_count

    c = contract(f)
    sha = candidate_sha256(f, sources)
    cand = f"{f.name}-{sha[7:19]}"
    run_id = f"family-{f.name}-{time.time_ns()}"
    t0 = time.monotonic()
    state: dict[str, Any] = {"verdicts": {}, "divergence": {}}

    async def harness(_inp: Any) -> AsyncIterator[msgspec.Struct]:
        yield events.validate(events.SESSION_INIT(
            vocab_version=VOCAB_VERSION, profile="; ".join(f"{r.image} run {k}" for k, r in f.runs.items()),
            contract_version=f.version, harness_version="0.0.1", purpose="conformance"))

    async def generator(_inp: Any) -> AsyncIterator[msgspec.Struct]:
        yield events.validate(events.CANDIDATE_GENERATED(
            candidate_id=cand, contract_id=f.contract_id, contract_version=f.version,
            generator_ref=f"{generator_ref}: tools/guest_runtime/{f.guest_file}", attempt=1, candidate_sha256=sha))

    def judge(run: str) -> Callable[[Any], AsyncIterator[msgspec.Struct]]:
        async def go(_inp: dict[str, Any]) -> AsyncIterator[msgspec.Struct]:
            r = f.runs[run]
            out, stop, _ = run_in_vm(r.binary, sources, f)
            cmp = compare_exact_rows(c, derived_output(f, out), f"cmp-{cand}-{run}", f"{r.execution}-1",
                                     f"managed_vm-{cand}-{run}")
            expected = f.expected_stop is not None and stop.startswith(f.expected_stop)
            if stop != "ExitProcess" and not expected and cmp.verdict == "conforms":
                cmp = msgspec.structs.replace(cmp, verdict="discrepancy", first_divergence=f"run stopped: {stop}")
            state["verdicts"][run] = cmp.verdict
            state["divergence"][run] = cmp.first_divergence or (stop if stop != "ExitProcess" else None)
            yield cmp
        return go

    async def decide(_inp: Any) -> AsyncIterator[msgspec.Struct]:
        if all(v == "conforms" for v in state["verdicts"].values()):
            yield events.validate(events.CANDIDATE_ACCEPTED(
                candidate_id=cand, contract_id=f.contract_id, contract_version=f.version,
                comparison_ids=tuple(f"cmp-{cand}-{r}" for r in f.runs)))
            return
        yield events.validate(events.CANDIDATE_REJECTED(
            candidate_id=cand, contract_id=f.contract_id, contract_version=f.version,
            reason=f"{sum(v != 'conforms' for v in state['verdicts'].values())} of {len(f.runs)} comparisons differ",
            discrepancy_id=f"{run_id}-d1"))
        first = next(r for r, v in state["verdicts"].items() if v != "conforms")
        yield events.validate(events.DISCREPANCY_REPORTED(
            discrepancy_id=f"{run_id}-d1", contract_id=f.contract_id, contract_version=f.version, mismatch_class="exact",
            minimized_program_sha256="sha256:" + f.runs[first].binary, initial_state_sha256="sha256:" + "0" * 64,
            first_divergence=f"{f.runs[first].image} binary: {state['divergence'][first]}"[:500],
            reference_profile_id=f"{f.runs[first].image} run {first}", destination_profile_id="w2m-managed_vm",
            comparison_id=f"cmp-{cand}-{first}"))
        yield events.validate(events.DISCREPANCY_ROUTED(
            discrepancy_id=f"{run_id}-d1", repair_path="implementation_repair", reference_conforms=True,
            grounds=f"Windows's rows are the reference; tools/guest_runtime/{f.guest_file} differs"))

    async def close(_inp: Any) -> AsyncIterator[msgspec.Struct]:
        bad = sum(1 for v in state["verdicts"].values() if v != "conforms")
        yield events.validate(events.SESSION_COMPLETED(
            outcome="completed_with_discrepancies" if bad else "completed", discrepancy_count=1 if bad else 0,
            accepted_count=0 if bad else 1, elapsed_seconds=round(time.monotonic() - t0, 3)))

    def topology(b: TopologyBuilder) -> None:
        b.baseline(contract_id=f.contract_id, contract_version=f.version, candidate_sha256=sha,
                   reference_runs=list(f.runs), family=f.name)
        b.producer_kind("harness", schemas=[events.SESSION_INIT], schema_version=1, factory=lambda: producer(harness))
        b.producer_kind("implementation_source", schemas=[events.CANDIDATE_GENERATED], schema_version=1,
                        factory=lambda: producer(generator))
        names = {run: f"vm_judge_{run}" for run in f.runs}
        for run, name in names.items():
            b.producer_kind(name, schemas=[events.COMPARISON_COMPLETED], schema_version=1,
                            factory=partial(lambda x: producer(judge(x)), run))
        b.producer_kind("acceptance", schemas=[events.CANDIDATE_ACCEPTED, events.CANDIDATE_REJECTED,
                        events.DISCREPANCY_REPORTED, events.DISCREPANCY_ROUTED], schema_version=1,
                        factory=lambda: producer(decide))
        b.producer_kind("harness_close", schemas=[events.SESSION_COMPLETED], schema_version=1,
                        factory=lambda: producer(close))
        b.initial("harness", input=None)

        def on(kinds: set[str]) -> Subscription:
            return Subscription(kinds=frozenset(kinds))

        b.trigger("implementation-on-session-init", subscription=on({"SESSION_INIT"}), predicate=lambda c: True,
                  starts="implementation_source", input_builder=lambda c: {}, policy=PerEvent())
        for name in names.values():
            b.trigger(f"{name}-on-candidate", subscription=on({"CANDIDATE_GENERATED"}), predicate=lambda c: True,
                      starts=name, policy=PerEvent(), input_builder=lambda c: {})
        b.trigger("acceptance-on-comparisons", subscription=on({"COMPARISON_COMPLETED"}),
                  predicate=lambda c: len(state["verdicts"]) == len(f.runs), starts="acceptance", policy=Once(),
                  input_builder=lambda c: {})
        b.trigger("close-on-decision", subscription=on({"substrate.ProducerCompleted"}),
                  predicate=lambda c: c.event.payload["producer"]["kind"] == "acceptance",
                  starts="harness_close", policy=PerEvent(), input_builder=lambda c: None)
        b.termination(threshold_count("SESSION_COMPLETED", 1))

    result = run_with_provenance(runs / run_id, topology)
    kinds = Counter(e["kind"] for e in read_record(result.record_root) if not e["kind"].startswith("substrate."))
    record = Path(result.record_root)
    return {"family": f.name, "record": str(record.relative_to(ROOT) if record.is_relative_to(ROOT) else record),
            "candidate_sha256": sha,
            "verdicts": dict(state["verdicts"]), "divergence": dict(state["divergence"]), "events": dict(kinds)}


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("family", choices=sorted(FAMILIES))
    ap.add_argument("--contract", action="store_true")
    ap.add_argument("--run", action="store_true")
    ap.add_argument("--plants", action="store_true")
    ap.add_argument("--runs", type=Path, default=ROOT / "store" / "runs")
    a = ap.parse_args(argv)
    f = FAMILIES[a.family]
    if a.contract:
        f.contract_path.parent.mkdir(parents=True, exist_ok=True)
        f.contract_path.write_text(json.dumps(build_contract(f), indent=1) + "\n")
        print(f"{f.contract_path.relative_to(ROOT)}: {len(contract(f).acceptance.rows)} rows")  # type: ignore[union-attr]
    failed = False
    if a.run:
        s = item_run(f, a.runs)
        # accepted means the record says so (KIT_DIARY #34): every run conforms and CANDIDATE_ACCEPTED was written
        failed |= not (s["verdicts"] and all(v == "conforms" for v in s["verdicts"].values())
                       and s["events"].get("CANDIDATE_ACCEPTED") == 1)
        index = a.runs / "family-runs.json"
        done = json.loads(index.read_text()) if index.exists() else {}
        done[f.name] = s
        index.write_text(json.dumps(done, indent=1, sort_keys=True) + "\n")
        print(json.dumps(s, indent=1))
    if a.plants:
        binary = next(iter(f.runs.values())).binary
        for name, p in f.plants.items():
            if p.host is not None:
                with host_patched(p.host, p.old, p.new):
                    out, stop, _ = run_in_vm(binary, None, f)
            else:
                out, stop, _ = run_in_vm(binary, planted_sources(f, name), f)
            rows, named = unequal_rows(f, out), named_reach(f, p)
            verdict = "as named" if rows == named else f"NOT as named: named {named}"
            failed |= rows != named
            shown = rows if len(rows) <= 20 else [*rows[:20], f"... {len(rows) - 20} more"]
            print(f"{name}: {len(rows)} unequal rows {shown} ({stop}); {len(named)} named; {verdict}")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
