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
import functools
import hashlib
import importlib
import json
import re
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

from w2m import VOCAB_VERSION, events, guest_runtime  # noqa: E402
from w2m.compare import compare_exact_rows  # noqa: E402
from w2m.contract import Contract, ExactRows, load_contract  # noqa: E402
from w2m.file_bridge import FileBridge  # noqa: E402
from w2m.kernel import Machine, spawner  # noqa: E402
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
    file: str | None = None  # another guest runtime file (time_and_error_mode.c) to patch instead (Sprint 049)


def described(doc: str, fn: Callable[[list[dict[str, Any]]], list[dict[str, Any]]],
              name: str) -> Callable[[list[dict[str, Any]]], list[dict[str, Any]]]:
    """A derivation with the name and description its contract records (Sprint 056)."""
    fn.__doc__ = doc
    fn.__name__ = name
    return fn


def raw_rows(printed: list[dict[str, Any]]) -> list[dict[str, Any]]:
    """The rows as the probe printed them."""
    return printed


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
    derive: Callable[[list[dict[str, Any]]], list[dict[str, Any]]] = raw_rows
    plants: dict[str, Plant] = field(default_factory=dict)
    excluded: dict[str, str] = field(default_factory=dict)    # row -> why the runners may differ on it
    expected_stop: str | None = None      # the stop an excluded row causes in w2m (a prefix of the stop text)
    gui: bool = False                     # the Steam-like start: no standard handles (console.c)
    arguments: str = ""                   # after the quoted module path on the command line
    output_file: str | None = None        # rows come from this file the probe writes, not standard output
    environment: tuple[str, ...] = ()     # the block the start passes (environment.c)
    dll_name: str | None = None           # the probe's own DLL, loaded from its directory (Sprint 046)
    dll_on_disk: bool = False             # the DLL is also a file beside the probe, which the probe reads (Sprint 063)
    host_files: tuple[str, ...] = ()      # Python no runtime request reaches that implements it (the loader's)

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
        """Keeps the rows of one start, unprefixed (child_rows)."""
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


def version_rows(printed: list[dict[str, Any]]) -> list[dict[str, Any]]:
    """DLLVERSIONINFO2's ullVersion ends in shell32's file revision, which differs by image (33438 on Server 2025, 5499
    on 2022); the row keeps that it is set."""
    return [{k: v for k, v in r.items() if k != "version_low"} | {"version_low_set": r["version_low"] != 0}
            if "version_low" in r else r for r in printed]


def display_rows(printed: list[dict[str, Any]]) -> list[dict[str, Any]]:
    """The runners' taskbars differ (the work area ends at 720 on Server 2025, 728 on 2022), and with it
    SM_CYFULLSCREEN (17: the work area's height less the caption, 23) and SM_CYMAXIMIZED (62: the work area's height
    and 16); the rows keep those as relations to the work area, and the work area's bottom as lying above the
    screen's. Index 93 has no SM_ name and differs from run to run (5852, 6552, 7696): its row keeps no value. The
    adapter's registry key ends in a GUID that differs by machine: the row keeps the key's form."""
    import re
    info = next((r for r in printed if r["s"] == "info.wide" and "work_bottom" in r), None)
    work = info["work_bottom"] - info["work_top"] if info else 0
    out = []
    for r in printed:
        r = dict(r)
        s = r["s"]
        if s == "metric.17":
            r["value"] = r["value"] - (work - 23)
        elif s == "metric.62":
            r["value"] = r["value"] - (work + 16)
        elif s == "metric.93":
            r.pop("value", None)
        if "work_bottom" in r:
            r["work_bottom"] = r["work_bottom"] < r.get("monitor_bottom", 768)
        if s == "devices.adapter_0" and "key" in r:
            r["key"] = re.sub(r"\{[0-9A-F-]{36}\}", "{GUID}", r["key"])
        out.append(r)
    return out


def processes_rows(printed: list[dict[str, Any]]) -> list[dict[str, Any]]:
    """The child's command line ends in the parent's process id, which differs by run: the row keeps that it is the
    word "child" and a number."""
    import re
    return [r | {"tail": "child <id>" if re.fullmatch(r"child \d+", r["tail"]) else r["tail"]}
            if r.get("s") == "child.command_line" else r for r in printed]


def parameters_rows(printed: list[dict[str, Any]]) -> list[dict[str, Any]]:
    """ProcessParameters' rows as relations where a run decides the value: the header keeps MaximumLength == Length
    (both grow with the strings inside the block, and Server 2025's are 8 bytes longer than 2022's); the block keeps
    no region size (the heap's: 0x9000, 0xf000, 0x10000 seen). The parent is the runner's start, which w2m does not
    copy: its window values and title are the runner's STARTUPINFO's and its environment's version the runner's
    (3 or 4), so it keeps only that they equal GetStartupInfoW's, and its standard handles only that they are
    GetStdHandle's (w2m's console start gives no input or error handle, Sprint 046)."""
    out = []
    for r in printed:
        s = r.get("s", "")
        r = dict(r)
        if s.endswith(".header"):
            r["length_is_maximum"] = r.pop("maximum_length") == r.pop("length")
        if s.endswith(".block"):
            r.pop("region_size", None)
        if s == "parent.window":
            r = {k: v for k, v in r.items() if k in ("s", "flags_are_startup", "show_is_startup")}
        if s == "parent.strings":
            r.pop("title_is_module", None)
        if s == "parent.environment":
            r.pop("version", None)
        if s == "parent.standard_handles":
            r = {k: v for k, v in r.items() if not k.endswith("_null")}
        out.append(r)
    return out


def without_message_0x60(printed: list[dict[str, Any]]) -> Iterator[dict[str, Any]]:
    """The probe's drains (rows queue.<drain>.<NN> and their "after" rows) without message 0x60, renumbered. An
    empty queue's last error after the window was activated is 0 or left as the system's own messages fall (the
    second executions of runs 38210803867 and 38210806334 differ there), so it is kept only for the first drain."""
    dropped: set[str] = set()
    kept: dict[str, int] = {}
    for r in printed:
        s = r.get("s", "")
        parts = s.split(".")
        if parts[0] != "queue" or len(parts) != 3:
            yield r
            continue
        if r.get("after"):
            if s in dropped:
                continue
            yield r | {"s": f"queue.{parts[1]}.{kept[s]:02d}"}
            continue
        if r.get("msg") == 0x60:
            dropped.add(s)
            continue
        n = sum(1 for k in kept if k.startswith(f"queue.{parts[1]}."))
        kept[s] = n
        if parts[1] != "first" and not r.get("noremove"):
            r = {k: v for k, v in r.items() if k != "noremove_err"}
        yield r | {"s": f"queue.{parts[1]}.{n:02d}"}


def windows_rows(printed: list[dict[str, Any]]) -> list[dict[str, Any]]:
    """Where a window lands depends on the desktop: CW_USEDEFAULT cascades in steps of 26 from a start set by the
    windows the session made before (52, 78 and 130 seen for a first window), and the default height is 3/4 of the
    work area's less 21 (519 on Server 2025's 720, 525 on 2022's 728: the taskbars differ). The rows keep each
    window's geometry from its own origin, positions as relations (x = y, a multiple of 26, the second window 26 past
    the first), the batch window's heights as their excess over that default, and the work area's bottom as lying
    above the screen's. Sprint 063: the game's window and another thread's ("game", "other", rows msg.game.*,
    msg.other.*) are placed as the batch window is; a drained message's row after DispatchMessageA shares its key and
    is keyed "<key>.after". With five windows the cascade did not always advance (the second executions of runs
    38210803867 and 38210806334 put the play window on the batch window's 26), so a window's place past the first is
    no longer kept, only that it lies on the cascade. Message 0x60, which Windows posts to a system window of the
    thread on its own schedule after activation (open question), is taken out of each drain's rows, and the drain's
    later rows are numbered without it."""
    printed = list(without_message_0x60(printed))
    work = next((r for r in printed if r["s"] == "screen.work_area"), None)
    default_height = (work["rect_bottom"] - work["rect_top"]) * 3 // 4 - 21 if work else 0
    origin: dict[str, tuple[int, int]] = {}
    for r in printed:
        if r["s"] in ("msg.batch.1", "msg.play.1", "msg.game.1", "msg.other.1") and "x" in r:
            origin[r["s"].split(".")[1]] = (r["x"], r["y"])
    out = []
    for r in printed:
        if r.get("after"):
            r = {k: v for k, v in r.items() if k != "after"} | {"s": r["s"] + ".after"}
        s = r["s"]
        if s == "screen.work_area":
            out.append({"s": s, "left": r["rect_left"], "top": r["rect_top"], "right": r["rect_right"],
                        "bottom_above_768": r["rect_bottom"] < 768})
            continue
        which = next((w for w in ("batch", "play", "game", "other") if s.startswith((f"msg.{w}.", f"{w}."))), None)
        if which is None or which not in origin:
            out.append(r)
            continue
        x, y = origin[which]
        tall = which != "play"                      # CW_USEDEFAULT height: from the work area
        r = dict(r)
        if "x" in r:
            r |= {"x": None, "y": None}
            r = {k: v for k, v in r.items() if v is not None} | {
                "x_equals_y": x == y, "x_in_cascade": x > 0 and x % 26 == 0}
        for prefix in ("in", "out", "rect"):
            if f"{prefix}_left" not in r:
                continue
            left, top, right, bottom = (r.pop(f"{prefix}_{k}") for k in ("left", "top", "right", "bottom"))
            if s.endswith(".client_rect"):
                left0, top0 = 0, 0
            else:
                left0, top0 = x, y
            height = bottom - top0
            r |= {f"{prefix}_left": left - left0, f"{prefix}_top": top - top0, f"{prefix}_right": right - left0,
                  f"{prefix}_bottom": height - default_height if tall else height}
        out.append(r)
    return out


TIME_RUNS = {   # Sprint 061: the probe records the zone's names (row time_zone.names)
              "38206225386": Run("windows-2025", "214581358977175f87eac5e82de8663bdcc29d83433e1b23493583324645703b", "time_and_error_mode-1791781331-0", "AMD EPYC 7763 64-Core Processor"),
              "38206227300": Run("windows-2022", "111a16ac4849950896efb41851003c6ee840734fb8e82fce280a6df0348bba1e", "time_and_error_mode-1791781331-1", "AMD EPYC 7763 64-Core Processor"),
}


def time_rows(start: str) -> Callable[[list[dict[str, Any]]], list[dict[str, Any]]]:
    """The probe's parent prints its rows, then its child's one row. Since Sprint 055 w2m starts the child as
    Windows does, and the parent's family judges every row; the child's family, started alone with "child", keeps
    only its own."""
    def derive(printed: list[dict[str, Any]]) -> list[dict[str, Any]]:
        """The child's family keeps only error_mode.child_inherited; the parent's keeps every row (time_rows)."""
        if start == "child":
            return [r for r in printed if r.get("s") == "error_mode.child_inherited"]
        return printed
    return derive


def time_family(start: str, summary: str) -> Family:
    child = start == "child"
    plants: dict[str, Plant] = {} if child else {
        # named first as system_to_file.february_31; February 31 stays invalid with 29 days, and every date after
        # February of a common year moves instead (recorded in Sprint 046's card)
        "february_29_always": Plant("    return days[m - 1] + (m == 2 && leap(y));",
                                    "    return days[m - 1] + (m == 2);", ("system_to_file.fixed", "file_to_system.fixed")),
        "time_zone_unnamed": Plant("        ((u16 *)(p + 4))[i] = (u16)UTC_NAME[i];\n", "", ("time_zone.names",)),
        "time_zone_leaves_error": Plant("    set_error(0);\n    return w2m_logged(W2M_API_GetTimeZoneInformation",
                                        "    return w2m_logged(W2M_API_GetTimeZoneInformation", ("time_zone",)),
        "error_mode_not_inherited": Plant("    list[n][2] = w2m_error_mode();", "    list[n][2] = 0x8003;",
                                          ("error_mode.child_inherited",), file="processes.c")}
    return Family(
        name="time_and_error_mode" + ("_child" if child else ""),
        contract_id="windows.time.error_mode_child" if child else "windows.time.local_and_error_mode",
        version="0.2.0" if child else "0.3.0", summary=summary, probe="time_and_error_mode", guest_file="time_and_error_mode.c",
        runs=TIME_RUNS,
        documented=(
            documented("ms-seterrormode", "api/errhandlingapi/nf-errhandlingapi-seterrormode", "2024-02-22",
                       "Each process has an associated error mode ... A child process inherits the error mode of its "
                       "parent process."),),
        open_questions=(
            "The runners keep UTC and w2m does too; a player's machine has its own zone (its names are UTC's: "
            "\"Coordinated Universal Time\" twice, row time_zone.names, Sprint 061).",
            "Steam's error mode, which the game inherits, is not observed; w2m's GUI start uses 0, as the child of a "
            "parent that set 0."),
        derive=time_rows(start),
        expected_stop=None if child else "Unimplemented: import KERNEL32.dll!CreateProcessW",
        gui=child, arguments="child" if child else "", output_file="w2m-error-mode-child.jsonl" if child else None,
        plants=plants)


def topology_rows(printed: list[dict[str, Any]]) -> list[dict[str, Any]]:
    """Processor topology (Sprint 047): `modules` and `size_other` in the RelationAll row are the OS's (Server 2025
    lists a processor module, 2022 does not; relationship 7 is excluded below), so both fields are dropped. Sprint
    061: w2m reports the runners' machine (four processors on two SMT cores, row machine), so `smt` is kept; the
    caches' sizes are the CPU's (Xeon 8573C 48 KB, 2 MB, 260 MB; EPYC 7763 32 KB, 512 KB, 32 MB) and are dropped."""
    out = []
    for r in printed:
        r = dict(r)
        if r.get("s") == "machine":
            for k in ("l1d_size", "l2_size", "l3_size"):
                r.pop(k, None)
        if r.get("s") == "ex.query.65535":
            r.pop("modules", None)
            r.pop("size_other", None)                 # the module entry's size, 48, on 2025 only
        out.append(r)
    return out


def slist_rows(printed: list[dict[str, Any]]) -> list[dict[str, Any]]:
    """Interlocked lists (Sprint 063): a push advances the header's sequence by 2 on Server 2025 and by 1 on 2022
    (row push.1); pop and flush leave it. The sequence is kept as the count of pushes it stands for, the step
    being push.1's."""
    step = next(r["sequence"] for r in printed if r.get("s") == "push.1") or 1
    out = []
    for r in printed:
        r = dict(r)
        for k in ("sequence", "sequence_after"):
            if k in r:
                r[k.replace("sequence", "pushes")] = r.pop(k) // step
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
        host_files=("src/w2m/loader.py",),
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
    "interlocked_lists": Family(
        name="interlocked_lists",
        contract_id="windows.interlocked.lists",
        version="0.1.0",
        summary=("InterlockedPushEntrySList, InterlockedPopEntrySList and InterlockedFlushSList on headers "
                 "InitializeSListHead made: the header's first entry, depth and sequence after each call, the values "
                 "returned, the entries' links, the last error left, and two threads pushing onto one list at once."),
        probe="slist", guest_file="slist.c",
        runs={
              "38210349431": Run("windows-2025", "3ab0c9d0351bc7663c61f28495315b4f3678935002714dac15a90a51774423a3", "slist-1791784738-0", "AMD EPYC 7763 64-Core Processor"),
              "38210351696": Run("windows-2022", "82d608691d4201d62b3fe2f93504429d969a156a34c8759a4e0836edc194de58", "slist-1791784738-1", "AMD EPYC 9V74 80-Core Processor"),
        },
        documented=(
            documented("ms-interlockedpushentryslist", "api/interlockedapi/nf-interlockedapi-interlockedpushentryslist",
                       "2024-02-22", "The return value is the previous first item in the list. If the list was "
                       "previously empty, the return value is NULL. ... All list items must be aligned on a "
                       "MEMORY_ALLOCATION_ALIGNMENT boundary; otherwise, this function will behave unpredictably."),),
        open_questions=(
            "The sequence advances by 2 a push on Server 2025 and by 1 on 2022; the rows keep the count of pushes, and "
            "w2m advances it as Server 2025 does, the game's reference system (game run 38206263976).",
            "A header or entry off a 16-byte boundary has no row: Microsoft Learn says the functions behave "
            "unpredictably, and w2m stops the run."),
        derive=slist_rows,
        plants={"push_keeps_depth": Plant("(((h[0] & DEPTH_MASK) + 1) & DEPTH_MASK)", "(h[0] & DEPTH_MASK)",
                                          ("push.1", "push.2", "push.3", "pop.1", "push.after_flush",
                                           "threads.push")),
                "flush_keeps_first": Plant("    h[1] = 0;\n    h[0] &= ~DEPTH_MASK;", "    h[0] &= ~DEPTH_MASK;",
                                           ("flush.full", "pop.after_flush", "push.after_flush")),
                "pop_advances_sequence": Plant("        h[0] = (h[0] & ~DEPTH_MASK) | (((h[0] & DEPTH_MASK) - 1) & DEPTH_MASK);",
                                               "        h[0] = ((h[0] & ~DEPTH_MASK) + SEQUENCE_STEP) | (((h[0] & DEPTH_MASK) - 1) & DEPTH_MASK);",
                                               ("pop.1", "flush.full", "pop.after_flush", "push.after_flush",
                                                "threads.push"))}),
    "module_search": Family(
        name="module_search",
        contract_id="windows.module.search",
        version="0.1.0",
        summary=("EnumProcessModules on the current process lists the program, ntdll, KERNEL32 and KERNELBASE first, "
                 "a DLL LoadLibrary maps last, and reports the whole list's length whatever the buffer holds, leaving "
                 "the last error; SetDllDirectoryW adds a directory LoadLibrary searches for a bare name and NULL "
                 "removes it, each answering 1 with the last error left (Sprint 063: mono's EnumProcessModules, "
                 "UnityPlayer's SetDllDirectoryW around steam_api64.dll)."),
        probe="module_search", guest_file="module.c", dll_name="module_search_dll.dll", dll_on_disk=True,
        runs={
              "38210455471": Run("windows-2025", "df6ca688fd04c4599788e7c9ce2ff311246e6d96bb2273edaa87282ece84c023", "module_search-1791784818-0", "AMD EPYC 7763 64-Core Processor", dll="735a3c71d68c4329e287bd1730b1bae07a5f448c80312daefd34f624f40d79cb"),
              "38210458062": Run("windows-2022", "1924070efea39f066a9859354c9c77138e32db51f26e5426e51b669140ea016d", "module_search-1791784818-1", "AMD EPYC 7763 64-Core Processor", dll="14dc071ae4f8c7321ae556b887ea440281eb7070c3ae9b8998aba1ce195ee192"),
        },
        documented=(
            documented("ms-setdlldirectoryw", "api/winbase/nf-winbase-setdlldirectoryw", "2025-07-01",
                       "After calling SetDllDirectory, the standard DLL search path is: 1. The directory from which "
                       "the application loaded. 2. The directory specified by the lpPathName parameter."),
            documented("ms-setdlldirectoryw-null", "api/winbase/nf-winbase-setdlldirectoryw", "2025-07-01",
                       "If this parameter is NULL, the function restores the default search order."),
            documented("ms-enumprocessmodules", "api/psapi/nf-psapi-enumprocessmodules", "2024-02-22",
                       "If PSAPI_VERSION is 1, this function is defined as EnumProcessModules in Psapi.h and exported "
                       "in Psapi.lib and Psapi.dll as a wrapper that calls K32EnumProcessModules.")),
        open_questions=(
            "The list's length differs by machine (each loads other system DLLs), so the rows hold its first four "
            "entries in load order and where a DLL the probe loads appears; w2m lists the program, ntdll, KERNEL32, "
            "KERNELBASE and the DLLs it maps, not the other system DLLs a Windows process holds.",
            "SetDllDirectoryW(\"\") (the current directory out of the search) has no row and stops the run."),
        plants={
            "directory_not_searched": Plant("    if (h || !dll_directory_too || !dll_directory[0]) return h;",
                                            "    return h;",
                                            ("search.load", "enum.after_load", "search.loaded_after_reset")),
            "reset_kept": Plant("        dll_directory[0] = 0;\n        return w2m_logged(W2M_API_SetDllDirectoryW, 0, 1);",
                                "        return w2m_logged(W2M_API_SetDllDirectoryW, 0, 1);", ("search.after_reset",)),
            "needed_is_written": Plant("    *(u32 *)needed = (u32)(8 * count);",
                                       "    *(u32 *)needed = (u32)(8 * (count < size / 8 ? count : size / 8));",
                                       ("enum.short", "enum.size_only"))}),
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
                "sleep_leaves_error": Plant("    if (!woken) set_error(ERROR_TIMEOUT);",
                                           "    if (!woken) (void)set_error;",
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
        plants={"count_off_mask": Plant("    *(u32 *)(p + 32) = W2M_PROCESSORS;", "    *(u32 *)(p + 32) = W2M_PROCESSORS + 1;",
                                        ("native_system_info", "system_info")),   # Sprint 061: was two_processors_one_bit
                "version_build_off": Plant("windows_10 ? 26100 : 9200;", "windows_10 ? 26100 : 9600;", ("version_ex",))}),
    "virtual_memory": Family(
        name="virtual_memory",
        contract_id="windows.memory.virtual",
        version="0.2.0",
        summary=("VirtualAlloc, VirtualFree, VirtualProtect and VirtualQuery on one thread: 64 KB-aligned reservations "
                 "above 4 GB, zeroed commits, queries reporting the run of equal pages, the error codes (87, 50, 487), "
                 "MEM_COMMIT alone at NULL reserving and committing (UnityPlayer's form, Sprint 054), "
                 "and an image's pages (headers read-only, code execute-read, data copy-on-write until written)."),
        probe="virtual_memory", guest_file="virtual_memory.c",
        runs={
              "38172734444": Run("windows-2025", "af77fd8015671e678be7df19b74160d4cc93ce8691c64127412da3572cf73955", "virtual_memory-1791750828-0", "AMD EPYC 7763 64-Core Processor"),
              "38172736319": Run("windows-2022", "8385bc74e8db79d6ef5c62d4ebd0dc161e7985f85f4d763dc73dd42bed2a137d", "virtual_memory-1791750828-1", "INTEL(R) XEON(R) PLATINUM 8573C"),
        },
        documented=(
            documented("ms-virtualquery", "api/memoryapi/nf-memoryapi-virtualquery", "2024-02-22",
                       "The function determines the attributes of the first page in the region and then scans "
                       "subsequent pages until it scans the entire range of pages or until it encounters a page with "
                       "a nonmatching set of attributes."),
            documented("ms-virtualalloc", "api/memoryapi/nf-memoryapi-virtualalloc", "2024-02-05",
                       "Attempting to commit a specific address range by specifying MEM_COMMIT without MEM_RESERVE and "
                       "a non-NULL lpAddress fails unless the entire range has already been reserved."),),
        open_questions=(
            "Protections are recorded, not enforced: a store to a read-only page succeeds in w2m.",
            "A released region's size runs to the next allocation and differs between executions; the row keeps that "
            "it is at least the reservation.",
            "A writable image page counts as written when its bytes differ from load; a write of the same bytes is "
            "missed."),
        derive=virtual_memory_rows,
        plants={"commit_null_refused": Plant("        if kind & MEM_RESERVE or not address:     # MEM_COMMIT alone at NULL reserves too (row alloc.commit_null)",
                                             "        if kind & MEM_RESERVE:",
                                             ("alloc.commit_null", "query.commit_null", "protect.commit_null_middle",
                                              "query.commit_null_middle", "query.commit_null_last", "free.commit_null",
                                              "image.section._data", "image.rdata_after"), host="w2m.virtual_memory"),
                "bad_protect_parameter": Plant("            return 0, ERROR_NOT_SUPPORTED                    # row alloc.bad_protect (0x12345)",
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
        plants={"auto_event_stays_set": Plant("    } else if (o->kind == SEMAPHORE || o->kind == EVENT_AUTO) {",
                                              "    } else if (o->kind == SEMAPHORE) {", ("event.auto.wait_consumed",)),
                "create_leaves_error": Plant("    set_error(0);                                  /* rows *.create: err 0 */\n",
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
        dll_name="static_tls_dll.dll", host_files=("src/w2m/loader.py",),
        # (a first plant, indices from 0 with no program slot, was named for tls.index and faulted the run: the DLL's
        # block moved to slot 0 while it read slot 1; recorded in Sprint 046's card, replaced by this one)
        plants={"template_not_copied": Plant("        block = bytes(region.data[start - image.base:end - image.base]) + bytes(zero_fill)",
                                            "        block = bytes(end - start) + bytes(zero_fill)",
                                            ("tls.values", "tls.write"), host="w2m.loader")}),
    "late_tls": Family(
        name="late_tls",
        contract_id="windows.loader.late_tls",
        version="0.1.0",
        summary=("A program without TLS loads a DLL with static TLS (static C runtime) by LoadLibrary while a second "
                 "thread waits: the DLL's TLS index is 1; TEB.ThreadLocalStoragePointer is set before the load and "
                 "keeps its value; the loading thread (after DllMain), the thread started before the load and one "
                 "started after it each read their own copy of the template with its zero fill, in the block their "
                 "slot holds."),
        probe="late_tls", guest_file="module.c",
        runs={
              "38229484497": Run("windows-2025", "960385e12284c9f19802c7c086fe4c6da26a9a5d3a5a18d16701e18c9306ded6", "late_tls-3-0", "INTEL(R) XEON(R) PLATINUM 8573C", dll="3afacd2a2ecb363b36139ac04034ff5c35482e6906dd29f0b6ba049d2a9cc42e"),
              "38229488362": Run("windows-2022", "8b7a25e1116431096c9107a08cd288591e4b1ac3032d3cbf41cb7807c7f94a5b", "late_tls-3-1", "AMD EPYC 9V74 80-Core Processor", dll="f32fa39b489c9525ee73ea52d9ae0916eb6368ef027f156c665eb54e7e4a0695"),
              "38229493876": Run("windows-2025", "960385e12284c9f19802c7c086fe4c6da26a9a5d3a5a18d16701e18c9306ded6", "late_tls-3-2", "AMD EPYC 7763 64-Core Processor", dll="3afacd2a2ecb363b36139ac04034ff5c35482e6906dd29f0b6ba049d2a9cc42e"),
              "38229497641": Run("windows-2022", "8b7a25e1116431096c9107a08cd288591e4b1ac3032d3cbf41cb7807c7f94a5b", "late_tls-3-3", "AMD EPYC 7763 64-Core Processor", dll="f32fa39b489c9525ee73ea52d9ae0916eb6368ef027f156c665eb54e7e4a0695"),
        },
        documented=(
            documented("ms-pe-tls", "../../windows/win32/debug/pe-format", "2024-02-22",
                       "The loader assigns the value of the TLS index to the place that was indicated by the Address "
                       "of Index field."),),
        open_questions=(
            "A late DLL's TLS callbacks: no row has one (steam_api64.dll has none); w2m stops on one.",
            "How many modules a thread's array holds before Windows moves it: w2m gives room for loader.TLS_SLOTS."),
        dll_name="late_tls_dll.dll", dll_on_disk=True, host_files=("src/w2m/vm_backend.py", "src/w2m/loader.py"),
        plants={"block_not_copied": Plant("            self.mem.write(va, block)",
                                          "            self.mem.write(va, bytes(len(block)))",
                                          ("late.main", "late.before_load"), host="w2m.vm_backend")}),
    "time_and_error_mode": time_family("parent", (
        "Local time on a UTC machine, the time zone (TIME_ZONE_ID_UNKNOWN, zero biases, error 0), exact SYSTEMTIME "
        "and FILETIME conversions and their invalid inputs (87), and the error mode a process inherits from the "
        "runner's cmd (0x8003) and sets.")),
    "time_and_error_mode_child": time_family("child", (
        "The error mode of a child whose parent set 0 and passed no flag: 0 (the Steam-like start).")),
    "console": Family(
        name="console",
        contract_id="windows.console.gui_start",
        version="0.3.0",
        summary=("A GUI process started without handles (as Steam starts the game) gets NULL standard handles, "
                 "no console, and Windows's answers from GetStdHandle, SetStdHandle, GetFileType, GetConsoleMode, "
                 "GetConsoleCP, WriteConsoleW, FlushFileBuffers, GetStartupInfoW, GetCommandLineW and GetCommandLineA."),
        probe="console", guest_file="console.c",
        runs={   # Sprint 063: the child also asks GetConsoleOutputCP (row console_output_cp)
              "38210455282": Run("windows-2025", "e182a66ebffa41eb86b7b12f9ef4256945ac1110c0d22b8289d7cc1ee1ebe139", "console-1791784818-0", "AMD EPYC 7763 64-Core Processor"),
              "38210458372": Run("windows-2022", "51d95fba138a0f0e8d6e891f911ea46ec32526334d8dc276cb97c115eb989e4a", "console-1791784818-1", "Intel(R) Xeon(R) Platinum 8370C CPU @ 2.80GHz"),
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
        plants={
            "output_cp_answers": Plant("    return no_console(W2M_API_GetConsoleOutputCP, 0);",
                                       "    return w2m_logged(W2M_API_GetConsoleOutputCP, 0, 437);", ("console_output_cp",)),
            "write_console_wrong_error": Plant("    set_error(ERROR_INVALID_HANDLE);\n    return w2m_logged(api, arg, 0);",
                                                   "    set_error(87);\n    return w2m_logged(api, arg, 0);",
                                                   # no_console serves GetConsoleMode too: the std rows' console_mode_err
                                                   ("std.input", "std.output", "std.error", "console_cp",
                                                    "console_output_cp", "write_console.no_console")),
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
        probe="generate_random", guest_file="bcrypt.c",
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
        version="0.2.0",
        summary=("GetLogicalProcessorInformationEx over every relationship (sizes, errors, entry sizes 48, 56 and 80, masks "
                 "as the active processors), the classic call's 32-byte entries, GetActiveProcessorCount, "
                 "GetProcessAffinityMask, SetThreadAffinityMask, GetNumaHighestNodeNumber and GetCurrentProcess; the rows "
                 "agree on Intel Xeon 8573C and 6973P and AMD EPYC 7763 and differ only by OS, in relationship 7."),
        probe="processor_topology", guest_file="topology.c",
        runs={   # Sprint 061: the probe records the machine's numbers (row machine)
              "38206551545": Run("windows-2025", "2f63103bfa140d4a20dd320a8992fb92ec5860a31a39c0edf2c17f489a96b5bc", "processor_topology-1791781612-0", "INTEL(R) XEON(R) PLATINUM 8573C"),
              "38206553365": Run("windows-2022", "858b8ebb308ec8906279d302a3ececb2d4634a15185decfa7a31d00bf10c892a", "processor_topology-1791781612-1", "AMD EPYC 7763 64-Core Processor"),
        },
        documented=(
            documented("ms-getlogicalprocessorinformationex", "api/sysinfoapi/nf-sysinfoapi-getlogicalprocessorinformationex",
                       "2023-02-27",
                       "If the buffer is not large enough to contain all of the data, the function fails, GetLastError "
                       "returns ERROR_INSUFFICIENT_BUFFER, and ReturnedLength is set to the buffer length required to "
                       "contain all of the data."),),
        open_questions=(
            "The machine is the runners' (row machine: four processors on two SMT cores, two L1d and L2, one L3; "
            "Decision 2026-10-11, user); the caches' sizes, which differ by CPU, are the guest CPUID profile's i3-6100.",
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
            "zero_mask_accepted": Plant("    if (!mask || (mask & ~ALL)) {", "    if (mask & ~ALL) {", ("thread_affinity.zero",)),
            "one_core": Plant("static const u64 CORE_MASKS[] = {0x3, 0xC};", "static const u64 CORE_MASKS[] = {0xF};",
                              ("machine",))}),       # the Ex rows hold the core count as relations (cores_le_n)
    "file_attributes": Family(
        name="file_attributes",
        contract_id="windows.file.attributes",
        version="0.1.0",
        summary=("GetFileAttributesExW and GetFileAttributesW over a 5-byte file (any case, trailing dot or space), the "
                 "working directory (0x10, size 4096), the empty name and missing paths (3), a missing file (2), "
                 "wildcards (123) and an invalid level (87); times as relations to the clock; success leaves the last "
                 "error and failure the caller's structure."),
        probe="file_attributes", guest_file="file_attributes.c",
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
        probe="file_security", guest_file="file.c",
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
        probe="file_access", guest_file="file.c",
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
        probe="system_modules", guest_file="module.c",
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
            "loadlibraryw_not_found": Plant("    u64 h = load(name, rsp, 1);\n    if (!h) set_error(ERROR_MOD_NOT_FOUND);\n    return w2m_logged(W2M_API_LoadLibraryW",
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
        probe="known_folders", guest_file="known_folders.c",
        derive=described("Drops `err` from every row that holds `hr`.", lambda printed: [{k: v for k, v in r.items() if not ("hr" in r and k == "err")} for r in printed], "known_folders_rows"),
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
        derive=described("Drops `ret` from temp.relative.", lambda printed: [{k: v for k, v in r.items() if not (r.get("s") == "temp.relative" and k == "ret")}
                                for r in printed], "temp_path_rows"),
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
        derive=described("Replaces `rid` with `low`: whether it is 0x1000 (SECURITY_MANDATORY_LOW_RID).", lambda printed: [({k: v for k, v in r.items() if k != "rid"} | {"low": r["rid"] == 0x1000})
                                if "rid" in r else r for r in printed], "integrity_level_rows"),
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
    "critical_section_flags": Family(
        name="critical_section_flags",
        contract_id="windows.critical_section.flags",
        version="0.1.0",
        summary=("InitializeCriticalSectionEx's flags, as mono's start-up passes CRITICAL_SECTION_NO_DEBUG_INFO with spin 0: "
                 "spin 0 gives 2000 with the dynamic-spin flag, NO_DEBUG_INFO adds its bit (0x030007D0), DYNAMIC_SPIN "
                 "with spin 0 changes nothing; a nonzero spin keeps the flags; STATIC_INIT alone answers TRUE and leaves "
                 "every byte; the last error is left."),
        probe="critical_section_flags", guest_file="critical_section.c",
        runs={
              "38113796820": Run("windows-2025", "78f7203b0f64375e7a55f06cbadde9d67f2a91c32ab0674d0775934545c69a3f", "critical_section_flags-1791695270-0", "INTEL(R) XEON(R) PLATINUM 8573C"),
              "38113797868": Run("windows-2022", "eab7ad8bdc971a686c4cd9ba5dddb919be7207ae1e2a18d00ad415498eb23c38", "critical_section_flags-1791695270-1", "AMD EPYC 9V74 80-Core Processor"),
        },
        documented=(
            documented("ms-initializecriticalsectionex", "api/synchapi/nf-synchapi-initializecriticalsectionex",
                       "2018-12-05", "CRITICAL_SECTION_NO_DEBUG_INFO: The critical section is created without debug "
                       "information."),),
        open_questions=(
            "STATIC_INIT and DYNAMIC_SPIN are not documented for this call; the rows record what Windows does with "
            "them alone and with NO_DEBUG_INFO; other combinations stop the run.",),
        plants={
            "no_debug_dropped_at_spin0": Plant("DEFAULT_SPIN | (flags & NO_DEBUG_INFO)", "DEFAULT_SPIN",
                                               ("spin0.no_debug_info", "spin0.no_debug_dynamic")),
            "static_init_initializes": Plant(
                "    if (flags == STATIC_INIT) return w2m_logged(W2M_API_InitializeCriticalSectionEx, cs, 1);\n",
                "    if (flags == STATIC_INIT) flags = 0;\n", ("spin0.static_init",))}),
    "module_names_ansi": Family(
        name="module_names_ansi",
        contract_id="windows.module.names_ansi",
        version="0.1.0",
        summary=("GetModuleFileNameA answers GetModuleFileNameW's text narrowed and its length; a buffer too small gets "
                 "nSize - 1 bytes and a NUL, the answer nSize and error 122; success sets error 0. GetModuleHandleA finds "
                 "kernel32 under any case and without its extension, NULL is the image, a missing name is NULL and 126; "
                 "success leaves the last error."),
        probe="module_names_ansi", guest_file="module.c",
        derive=described("Drops `w_len` from file.image, file.exact, file.short and file.one, and `ret` from the first three.", lambda printed: [{k: v for k, v in r.items()
                                 if not (r.get("s") in ("file.image", "file.exact", "file.short", "file.one")
                                         and (k == "w_len" or (k == "ret" and r["s"] != "file.one")))}
                                for r in printed], "module_names_ansi_rows"),
        runs={
              "38113666467": Run("windows-2025", "101395b58178b3fc6fbda6647f30e73123f1728782a9b92c59d5925ecd777a4a", "module_names_ansi-1791695127-0", "AMD EPYC 7763 64-Core Processor"),
              "38113667870": Run("windows-2022", "c131bd9f752c01b7966c229dc45cc0c7986bdb904a118c55ce10168e9493695c", "module_names_ansi-1791695127-1", "Intel(R) Xeon(R) Platinum 8370C CPU @ 2.80GHz"),
        },
        documented=(
            documented("ms-getmodulefilenamea", "api/libloaderapi/nf-libloaderapi-getmodulefilenamea", "2018-12-05",
                       "If the buffer is too small to hold the module name, the string is truncated to nSize characters "
                       "including the terminating null character, the function returns nSize, and the function sets the "
                       "last error to ERROR_INSUFFICIENT_BUFFER."),),
        open_questions=(
            "The image's path length is the runner's (38); the derive drops it (w_len, and ret where ret is that "
            "length) from file.image, file.exact, file.short and file.one, and keeps ret_is_w_len.",
            "Only ASCII paths and names are probed; w2m stops at another byte.",),
        plants={
            "short_returns_size_minus_one": Plant("return w2m_logged(W2M_API_GetModuleFileNameA, module, size);",
                                                  "return w2m_logged(W2M_API_GetModuleFileNameA, module, size - 1);",
                                                  ("file.short", "file.one")),
            "missing_handle_no_error": Plant("    if (!h) set_error(ERROR_MOD_NOT_FOUND);\n    return w2m_logged(W2M_API_GetModuleHandleA",
                                             "    return w2m_logged(W2M_API_GetModuleHandleA", ("handle.missing",))}),
    "function_tables": Family(
        name="function_tables",
        contract_id="windows.exception.function_tables",
        version="0.1.0",
        summary=("ntdll's growable function tables as mono's JIT registers its code: RtlAddGrowableFunctionTable answers "
                 "0 and a table, also for zero entries; RtlLookupFunctionEntry answers the entry whose [RangeBase + "
                 "BeginAddress, RangeBase + EndAddress) holds the address among the first EntryCount, with RangeBase as "
                 "the image base, else NULL with the image base untouched; RtlGrowFunctionTable raises the count; "
                 "RtlDeleteGrowableFunctionTable removes the table; no call touches the last error."),
        probe="function_tables", guest_file="function_tables.c",
        runs={
              "38114067866": Run("windows-2025", "d228185b9db008604eab043294136e6eb76bded19fa5e4ce724156145356fc5f", "function_tables-1791695574-0", "INTEL(R) XEON(R) PLATINUM 8573C"),
              "38114069629": Run("windows-2022", "0b7bc93d8c5d8fc730ccb613a581c8dbdf86565d666563f32d1b54e9ac67ae70", "function_tables-1791695574-1", "INTEL(R) XEON(R) PLATINUM 8573C"),
        },
        documented=(
            documented("ms-rtladdgrowablefunctiontable", "api/winnt/nf-winnt-rtladdgrowablefunctiontable", "2018-12-05",
                       "The entries in this array must remain sorted in ascending order of the BeginAddress members. ... "
                       "This function returns zero on success. ... This table is used for dispatching exceptions "
                       "through runtime-generated code and for collecting stack backtraces."),),
        open_questions=(
            "RtlLookupFunctionEntry over an image's own .pdata is not served: an address in a loaded image stops the "
            "run. Exception dispatch will need it.",),
        plants={
            "end_inclusive": Plant("            else if (pc >= tb->base + f[mid].end) lo = mid + 1;",
                                   "            else if (pc > tb->base + f[mid].end) lo = mid + 1;", ("find.first_end",)),
            "grow_ignored": Plant("    tb->count = (u32)count;\n", "", ("find.second", "find.third")),
            "miss_writes_image": Plant("        if (entry) *(u64 *)image = base;", "        *(u64 *)image = base;",
                                       ("find.first_end", "find.second_before_grow", "find.gap",
                                        "find.fourth_beyond_count", "find.empty"))}),
    "vectored_handlers": Family(
        name="vectored_handlers",
        contract_id="windows.exception.vectored_handlers",
        version="0.2.0",
        summary=("AddVectoredExceptionHandler answers a new non-NULL handle each call, the same handler again included; "
                 "RemoveVectoredExceptionHandler answers 1 for a registered handle and 0 for it again or for any other "
                 "value; neither touches the last error. Dispatch (0.2.0, Sprint 053): handlers added first run before "
                 "those added last, CONTINUE_EXECUTION ends the search and RaiseException returns, the handler sees "
                 "the code and flags 0x80."),
        probe="vectored_handlers", guest_file="vectored_handlers.c",
        runs={
              "38114204685": Run("windows-2025", "ee140359e6683f4072a35535c52483aa788b381c6f44f9cdf883e45e48a03682", "vectored_handlers-1791695726-0", "AMD EPYC 7763 64-Core Processor"),
              "38114206477": Run("windows-2022", "62adbf5d647a07f02182ceecc86b535ee508163913152ea86b9ffe9e82d7bc18", "vectored_handlers-1791695726-1", "Intel(R) Xeon(R) Platinum 8370C CPU @ 2.80GHz"),
        },
        documented=(
            documented("ms-addvectoredexceptionhandler", "api/errhandlingapi/nf-errhandlingapi-addvectoredexceptionhandler",
                       "2018-12-05", "If the First parameter is nonzero, the handler is the first handler to be "
                       "called until a subsequent call to AddVectoredExceptionHandler is used to specify a different "
                       "handler as the first handler."),),
        open_questions=(),
        plants={
            "remove_answers_one": Plant("    return w2m_logged(W2M_API_RemoveVectoredExceptionHandler, handle, 0);",
                                        "    return w2m_logged(W2M_API_RemoveVectoredExceptionHandler, handle, 1);",
                                        ("remove.again", "remove.unknown")),
            "first_at_back": Plant("            if ((u32)first) {", "            if ((u32)first && !count) {",
                                   ("dispatch.continue",))}),
    "thread_stack": Family(
        name="thread_stack",
        contract_id="windows.memory.thread_stack",
        version="0.1.0",
        summary=("The main thread's stack, as mono's collector queries it: one MEM_PRIVATE reservation of "
                 "SizeOfStackReserve from TEB.DeallocationStack, allocation protection read-write; reserved below "
                 "StackLimit - 3 pages, those 3 pages committed with PAGE_GUARD, committed read-write from StackLimit "
                 "to StackBase; VirtualQuery's BaseAddress is the queried page; GetCurrentThreadStackLimits answers "
                 "DeallocationStack and StackBase; the last error is left."),
        probe="thread_stack", guest_file="thread_queries.c",
        derive=described("Drops `base_address` from query.sp and query.base_minus_one.", lambda printed: [{k: v for k, v in r.items()
                                 if not (r.get("s") in ("query.sp", "query.base_minus_one") and k == "base_address")}
                                for r in printed], "thread_stack_rows"),
        runs={
              "38114444717": Run("windows-2025", "09f0d8199eb9748f2ebd1855df859eeed84a7028982ffc6fe7091e7ccfb44091", "thread_stack-1791696004-0", "AMD EPYC 7763 64-Core Processor"),
              "38114446614": Run("windows-2022", "d39ae9f9aa57c21fdc3a440f5e0acc4df8b073df4def00ef03b21a5ad047e7dd", "thread_stack-1791696004-1", "INTEL(R) XEON(R) PLATINUM 8573C"),
        },
        documented=(
            documented("ms-thread-stack-size", "procthread/thread-stack-size", "2025-07-14",
                       "The system commits additional pages from the reserved stack memory as they are needed, until "
                       "either the stack reaches the reserved size minus one page (which is used as a guard page to "
                       "prevent stack overflow) or the system is so low on memory that the operation fails."),),
        open_questions=(
            "Where the stack pointer's page lies above StackLimit depends on how far the runner's stack had grown "
            "(2 or 3 pages in runs 38114383995, 38114385651, 38114444717, 38114446614); the derive drops base_address "
            "from query.sp and query.base_minus_one, and query.limit and query.below_limit keep the layout.",
            "w2m commits all but 4 pages of the 1 MB at the start and never moves StackLimit; Windows commits on "
            "demand through the guard pages. A program that reads StackLimit to size its stack sees more room in "
            "w2m.",
            "The page speaks of one guard page; every run shows a 3-page PAGE_GUARD region below StackLimit "
            "(query.below_limit, query.below_guard). The rows are followed."),
        plants={
            # the expression, not the constant: host_patched rebinds changed objects by identity, and small ints are shared
            "guard_one_page": Plant("        guard = limit - STACK_GUARD_PAGES * PAGE", "        guard = limit - 1 * PAGE",
                                    ("query.below_guard", "query.dealloc"), host="w2m.virtual_memory"),
            "limits_swapped": Plant("    *(u64 *)low = *(const u64 *)((u8 *)w2m_teb() + 0x1478);",
                                    "    *(u64 *)low = *(const u64 *)((u8 *)w2m_teb() + 0x10);", ("limits",))}),
    "thread_handles": Family(
        name="thread_handles",
        contract_id="windows.handle.thread_handles",
        version="0.1.0",
        summary=("DuplicateHandle as mono calls it: the pseudo-handles -1 and -2 become real handles, new ones each "
                 "time, naming the current process and thread; a thread's handle does not signal while it runs; a "
                 "duplicate of an event names the same event, and with DUPLICATE_CLOSE_SOURCE takes the source's value; "
                 "a source that is no handle: FALSE, 6, the target NULL; success leaves the last error. OpenThread on "
                 "the current thread's id gives a real handle, on an id no thread has NULL and 87. CreateSemaphoreW "
                 "sets error 0 on success; an initial count above the maximum is NULL and 87."),
        probe="thread_handles", guest_file="sync_objects.c",
        runs={
              "38114820680": Run("windows-2025", "42e42696025a5ee2c71994db9a47d4be2fe667d8b769e65a617b68359cf65703", "thread_handles-1791696424-0", "AMD EPYC 9V45 96-Core Processor"),
              "38114822014": Run("windows-2022", "6b75ead1c4597d8c4b1ec8426c3e775e4b8fbd9e859d861ef6a249f8a51c56a6", "thread_handles-1791696424-1", "AMD EPYC 7763 64-Core Processor"),
        },
        documented=(
            documented("ms-duplicatehandle", "api/handleapi/nf-handleapi-duplicatehandle", "2018-12-05",
                       "DUPLICATE_CLOSE_SOURCE: Closes the source handle. This occurs regardless of any error status "
                       "returned."),),
        open_questions=(
            "One thread runs in w2m: OpenThread answers only the current thread's id, and waiting without a timeout on "
            "a running thread stops the run.",
            "Windows reuses a closed handle's value; w2m gives a CLOSE_SOURCE copy the source's slot and otherwise the "
            "lowest free slot. Which value a later handle gets is not in the rows."),
        plants={
            "close_source_new_slot": Plant("        if (options & DUPLICATE_CLOSE_SOURCE) h = source;          /* the copy keeps the slot (row dup.close_source) */\n        else h",
                                           "        h", ("dup.close_source",)),
            "thread_signals": Plant("    if (o->kind == THREAD) return w2m_thread_ended((u32)o->count);",
                                    "    if (o->kind == THREAD) return 1;", ("wait.own_thread",)),
            "semaphore_leaves_error": Plant("    u64 h = handle_for(api, new_object(api, kind, count, max));\n    set_error(0);",
                                            "    u64 h = handle_for(api, new_object(api, kind, count, max));",
                                            ("semaphore.zero", "semaphore.one"))}),
    "heap_query": Family(
        name="heap_query",
        contract_id="windows.memory.heap_query",
        version="0.1.0",
        summary=("The process heap as mono's collector queries it: a block's region belongs to one MEM_PRIVATE "
                 "reservation whose base is the heap's handle (64 KB aligned), allocation protection read-write; "
                 "committed read-write where blocks lie, reserved after; BaseAddress is the queried page; the last "
                 "error is left."),
        probe="heap_query", guest_file="heap.c",
        derive=described("Drops `allocation_base_below_block` from query.small and query.after_small_region.", lambda printed: [{k: v for k, v in r.items()
                                 if not (r.get("s") in ("query.small", "query.after_small_region")
                                         and k == "allocation_base_below_block")}
                                for r in printed], "heap_query_rows"),
        runs={
              "38115120174": Run("windows-2025", "5b2e6d18b8335848996effd91796cf03bc4e055ddc6e740f0d696a4c2f05b871", "heap_query-1791696755-0", "AMD EPYC 9V74 80-Core Processor"),
              "38115121838": Run("windows-2022", "90374fedfe0602d20ca0cab4e4cab1bb5ef0da0dea4e6ff7ebb0087b957cdde9", "heap_query-1791696755-1", "AMD EPYC 7763 64-Core Processor"),
        },
        documented=(),
        open_questions=(
            "Whether a 64-byte block lies within 64 KB of the heap's base depends on where Windows put it (it differs "
            "between a run's two executions); the derive drops that relation for query.small and "
            "query.after_small_region.",
            "Excluded rows: see sections.excluded."),
        excluded={"query.big": "Windows gives a 1 MB block an allocation of its own (allocation_base_is_heap 0); "
                               "w2m's heap.c serves every size from its one reservation. The threshold is not probed; "
                               "the allocator that replaces heap.c (review F5) takes it up"},
        plants={
            "all_committed": Plant("        committed = _up(max(used, base + PAGE), PAGE)",
                                   "        committed = base + size", ("query.after_small_region",), host="w2m.virtual_memory"),
            "base_is_page": Plant("            return Info(page, base, PAGE_READWRITE, committed - page, MEM_COMMIT, PAGE_READWRITE, MEM_PRIVATE)",
                                  "            return Info(page, page, PAGE_READWRITE, committed - page, MEM_COMMIT, PAGE_READWRITE, MEM_PRIVATE)",
                                  ("query.small",), host="w2m.virtual_memory")}),
    "files_more": Family(
        name="files_more",
        contract_id="windows.file.files_more",
        version="0.1.0",
        summary=("The game's remaining file calls (specs/work_list.json). GetFileInformationByHandleEx: FileBasicInfo "
                 "(0x28 bytes, the pad zeroed) holds GetFileAttributesExW's times, a change time not before the write "
                 "time, and FILE_ATTRIBUTE_ARCHIVE; FileStandardInfo (0x18) one link, not deleting, not a directory, "
                 "the size, and an allocation of 8-byte units up to 720 bytes and 4096-byte clusters from 740 on, kept "
                 "in clusters after the file shrinks; a smaller buffer is ERROR_BAD_LENGTH with nothing written. "
                 "GetFileSize: the low 32 bits, the high ones through the pointer. SetHandleInformation: the mask's "
                 "INHERIT and PROTECT_FROM_CLOSE bits, other bits ignored. GetWindowsDirectoryW: C:\\Windows, 10; 11 "
                 "for a short buffer, left unwritten. GetTempFileNameW: <dir as given>\\<prefix's first 3><hex, upper "
                 "case, no leading zeros>.tmp, ERROR_DIRECTORY for a missing directory; uUnique 0 creates the empty "
                 "file and sets the last error to 0. A handle opened with no access answers every query; a closed one "
                 "is ERROR_INVALID_HANDLE; every other success leaves the last error."),
        probe="files_more", guest_file="file.c",
        derive=described("Names each standard.sweep row by its size, standard.sweep.<size>.", lambda printed: [{**r, "s": f"standard.sweep.{r['size']}"} if r.get("s") == "standard.sweep" else r
                                for r in printed], "files_more_rows"),                       # one row per swept size
        runs={
              "38119625685": Run("windows-2025", "4571f790108785c15174f5fe7d2a97727996b6bf8c6be138cf6a23b2db4201e8", "files_more-1791701711-0", "AMD EPYC 7763 64-Core Processor"),
              "38119627458": Run("windows-2022", "17d996052924c712ba56fb559c8b0f851988187a4ee4da56c368ac66ee9b02f0", "files_more-1791701711-1", "INTEL(R) XEON(R) PLATINUM 8573C"),
        },
        documented=(
            documented("ms-gettempfilenamew", "api/fileapi/nf-fileapi-gettempfilenamew", "2025-09-24",
                       "If uUnique is zero, the function attempts to form a unique file name using the current system "
                       "time. If the file already exists, the number is increased by one ... Only the lower 16 bits of "
                       "the uUnique parameter are used."),
            documented("ms-getfileinformationbyhandleex", "api/winbase/nf-winbase-getfileinformationbyhandleex",
                       "2018-12-05", "FileBasicInfo (0) FILE_BASIC_INFO; FileStandardInfo (1) FILE_STANDARD_INFO ... "
                       "If the function fails, the return value is zero.")),
        open_questions=(
            "The GetTempFileNameW page gives the form <path>\\<pre><uuuu>.TMP; both builds write \".tmp\" in lower case "
            "and the number without leading zeros (rows temp_name.long_prefix: abc1.tmp). w2m follows the rows.",
            "Where NTFS stops keeping data in the file record lies between 721 and 739 bytes and depends on what else "
            "the record holds (the name's length among it); w2m keeps 720 and below there, the largest size a row "
            "holds resident.",
            "uUnique 0 takes its number from GetTickCount's milliseconds in w2m; the page says \"the current system "
            "time\" and the rows print the number only as matching the name. A uUnique whose low 16 bits are 0, a "
            "directory past MAX_PATH-14 (the page's ERROR_BUFFER_OVERFLOW), other information classes, and handles "
            "of other kinds (pipes, events, the console) are not in the rows and stop the run.",
            "CloseHandle on a handle with PROTECT_FROM_CLOSE is not probed (Windows raises under a debugger); w2m "
            "stops."),
        plants={
            "unknown_bits_kept": Plant("    u32 m = (u32)mask & (HANDLE_FLAG_INHERIT | HANDLE_FLAG_PROTECT_FROM_CLOSE);",
                                       "    u32 m = (u32)mask;", ("set.unknown_bit",)),
            "short_buffers_taken": Plant("    if (size < (info_class == FILE_BASIC_INFO_CLASS ? 0x28u : 0x18u)) {",
                                         "    if (size < (info_class == FILE_BASIC_INFO_CLASS ? 0x24u : 0x16u)) {",
                                         ("basic.short", "basic.exact_less_pad", "standard.short",
                                          "standard.exact_less_pad")),
            "record_holds_740": Plant("#define RESIDENT_MAX 720u", "#define RESIDENT_MAX 740u", ("standard.sweep.740",)),
            "temp_error_left": Plant("        set_error(0);\n        return w2m_logged(W2M_API_GetTempFileNameW",
                                     "        return w2m_logged(W2M_API_GetTempFileNameW",
                                     ("temp_name.zero", "temp_name.cwd_full", "temp_name.second"))}),
    "locale": Family(
        name="locale",
        contract_id="windows.locale.locale",
        version="0.1.0",
        summary=("The locale and local time the game asks for (specs/work_list.json), en-US in UTC as on the runners. "
                 "SystemTimeToTzSpecificLocalTime(NULL): the universal time with wDayOfWeek computed; a date or time "
                 "out of range (month 0 or 13, day 0, February 29 of 2025, hour 24, 1000 ms, years 1600 and 30828) is "
                 "ERROR_INVALID_PARAMETER with nothing written. GetThreadLocale: 0x409. GetLocaleInfoA: \"en\" and "
                 "\"US\", 3, for 0x409, LOCALE_USER_DEFAULT and LOCALE_SYSTEM_DEFAULT; a short buffer gets what fits "
                 "without a NUL and ERROR_INSUFFICIENT_BUFFER; size 0 gives 3; a negative size is "
                 "ERROR_INVALID_PARAMETER. GetUserDefaultLocaleName: \"en-US\", 6; a short buffer gets size - 1 units "
                 "and a NUL, and it and size 0 answer 0 with error 266. Success leaves the last error."),
        probe="locale", guest_file="locale.c",
        runs={
              "38120346754": Run("windows-2025", "acbe1c6e43d39d53f83d838bf64f81750c95b6b45302c98238e58e4b1c15785d", "locale-1791702485-0", "AMD EPYC 7763 64-Core Processor"),
              "38120348019": Run("windows-2022", "cc344b3608f76dfe67851701a49772a4f967fb08ff02defe066c43367758c03d", "locale-1791702485-1", "AMD EPYC 7763 64-Core Processor"),
        },
        documented=(
            documented("ms-getlocaleinfoa", "api/winnls/nf-winnls-getlocaleinfoa", "2018-12-05",
                       "If the function succeeds and the value of cchData is 0, the return value is the required size, "
                       "in characters including a null character, for the locale data buffer. ... "
                       "ERROR_INSUFFICIENT_BUFFER. A supplied buffer size was not large enough"),
            documented("ms-getuserdefaultlocalename", "api/winnls/nf-winnls-getuserdefaultlocalename", "2018-12-05",
                       "Returns the size of the buffer containing the locale name, including the terminating null "
                       "character, if successful. ... ERROR_INSUFFICIENT_BUFFER. A supplied buffer size was not large "
                       "enough, or it was incorrectly set to NULL.")),
        open_questions=(
            "GetUserDefaultLocaleName's page names ERROR_INSUFFICIENT_BUFFER (122) for a short buffer; both builds set "
            "266 (ERROR_CANNOT_COPY) and write size - 1 units and a NUL. w2m follows the rows.",
            "GetLocaleInfoA's short buffer receives the chars that fit with no NUL before the failure (row "
            "info.language_short); the page does not say.",
            "The runners' locale (en-US) and zone (UTC) are w2m's profile, not a property of Windows; other LCIDs, "
            "LCTYPEs and an explicit time zone are not in the rows and stop the run. The eight locale imports the "
            "captures never call (CompareStringEx, EnumSystemLocalesEx, GetDateFormatEx, GetLocaleInfoEx, "
            "GetTimeFormatEx, IsValidLocaleName, LCIDToLocaleName, LocaleNameToLCID) stop the run by name."),
        plants={
            "day_of_week_kept": Plant("    to_systemtime(v, (u16 *)out);\n    return w2m_logged(W2M_API_SystemTimeToTzSpecificLocalTime",
                                      "    for (int i = 0; i < 8; i++) ((u16 *)out)[i] = ((const u16 *)in)[i];\n"
                                      "    return w2m_logged(W2M_API_SystemTimeToTzSpecificLocalTime",
                                      ("local.day_of_week_ignored", "local.last_day"),
                                      file="time_and_error_mode.c"),
            "short_ansi_unwritten": Plant("    for (u32 i = 0; i < need && i < (u32)cch; i++) out[i] = value[i];",
                                          "    for (u32 i = 0; i < need && (u32)cch >= need; i++) out[i] = value[i];",
                                          ("info.language_short",)),
            "locale_name_error_122": Plant("        set_error(ERROR_CANNOT_COPY);", "        set_error(ERROR_INSUFFICIENT_BUFFER);",
                                           ("user_name.short", "user_name.size_zero"))}),
    "file_mapping": Family(
        name="file_mapping",
        contract_id="windows.file.file_mapping",
        version="0.1.0",
        summary=("CreateFileMappingW and MapViewOfFile in mono's read-only form (specs/work_list.json: five assemblies "
                 "per run, never unmapped). A PAGE_READONLY mapping of a file opened for reading succeeds and sets the "
                 "last error to 0; an empty file is ERROR_FILE_INVALID, a size past the file ERROR_NOT_ENOUGH_MEMORY, no "
                 "read access or PAGE_READWRITE on a read handle ERROR_ACCESS_DENIED, a closed handle "
                 "ERROR_INVALID_HANDLE. A FILE_MAP_READ or FILE_MAP_COPY view starts on a 64 KB boundary, holds the "
                 "file's bytes and zeros to its last page's end, outlives the file's and the mapping's handles, and "
                 "VirtualQuery reports it MEM_MAPPED, committed PAGE_READONLY, its pages rounded; the rest of the 64 KB "
                 "granule is free. Mapping more bytes than the file, or FILE_MAP_WRITE, is ERROR_ACCESS_DENIED."),
        probe="file_mapping", guest_file="file.c",
        derive=described("Drops `region_size` from query.after_view and query.granule_end.", lambda printed: [{k: v for k, v in r.items()
                                 if not (r.get("s") in ("query.after_view", "query.granule_end") and k == "region_size")}
                                for r in printed], "file_mapping_rows"),
        runs={
              "38120624307": Run("windows-2025", "0407e57fcef44dd02237f541b902804fe843d8e6cef4f76163811c7331893c87", "file_mapping-1791702788-0", "AMD EPYC 7763 64-Core Processor"),
              "38120626017": Run("windows-2022", "fa3540a10d0b61f3c0ce0e96dfca6caca497edb8208b9ebf9040a0cddd382e51", "file_mapping-1791702788-1", "AMD EPYC 7763 64-Core Processor"),
        },
        documented=(),
        open_questions=(
            "The free region after a view runs to whatever Windows placed next (110,592 bytes in one execution, "
            "138,958,864,691,200 in another); the derive drops its RegionSize and keeps its state, protection and type.",
            "A view in w2m is a copy made when MapViewOfFile runs: a write through another handle afterwards is not "
            "seen in it, as it would be on Windows. mono writes no file it maps in the captures; writable mappings, "
            "named mappings, offsets and UnmapViewOfFile are not in the rows and stop the run."),
        plants={
            "view_private": Plant("        r = Reservation(base, pages, protect, type=MEM_MAPPED)",
                                  "        r = Reservation(base, pages, protect, type=MEM_PRIVATE)",
                                  ("query.view", "query.view_middle", "query.whole_file", "query.small"),
                                  host="w2m.virtual_memory"),
            "mapping_error_left": Plant("    set_error(0);\n    return w2m_logged(W2M_API_CreateFileMappingW, file, h);",
                                        "    return w2m_logged(W2M_API_CreateFileMappingW, file, h);",
                                        ("map.game_form", "map.again", "map.small")),
            "empty_file_mapped": Plant("    if (!error && size == 0 && file_size == 0) error = ERROR_FILE_INVALID;\n", "",
                                       ("map.empty_file",)),
            "copy_refused": Plant("    if (access == FILE_MAP_WRITE || bytes > size) {",
                                  "    if (access != FILE_MAP_READ || bytes > size) {", ("view.copy_access",))}),
    "format_message": Family(
        name="format_message",
        contract_id="windows.messages.format_message",
        version="0.1.0",
        summary=("FormatMessageW from the system message table in mono's form (FROM_SYSTEM | IGNORE_INSERTS with "
                 "ALLOCATE_BUFFER, LANG_USER_DEFAULT; ids 2 and 3 in the captures, 44 each): the text with its CR LF, "
                 "inserts such as %1 kept, the length in units, a NUL after it; 0x8007xxxx names id xxxx; LANG_NEUTRAL "
                 "and en-US answer alike, de-DE is ERROR_RESOURCE_LANG_NOT_FOUND, an id the table lacks "
                 "ERROR_MR_MID_NOT_FOUND, each with the allocated pointer overwritten; a caller's buffer shorter than "
                 "text and NUL is ERROR_INSUFFICIENT_BUFFER; FORMAT_MESSAGE_MAX_WIDTH_MASK makes the closing CR LF one "
                 "space. Success leaves the last error. The table itself (rows \"dump\", ids 0-16383) is "
                 "specs/system_messages.json, from build 26100."),
        probe="format_message", guest_file="messages.c",
        derive=described("Drops the dump rows.", lambda printed: [r for r in printed if r.get("s") != "dump"], "format_message_rows"),
        runs={
              "38120951889": Run("windows-2025", "4224a823b406642df21fc8778e5fbd0182890b8063d6a1c28e0861c0dcabb926", "format_message-1791703140-0", "AMD EPYC 7763 64-Core Processor"),
              "38120953179": Run("windows-2022", "a2ecf6240cd728247b4cd842f563485d3124b0e20573b702a12da4bca9d5a31e", "format_message-1791703140-1", "AMD EPYC 7763 64-Core Processor"),
        },
        documented=(
            documented("ms-formatmessagew", "api/winbase/nf-winbase-formatmessagew", "2022-08-04",
                       "If the function succeeds, the return value is the number of TCHARs stored in the output buffer, "
                       "excluding the terminating null character. ... FORMAT_MESSAGE_MAX_WIDTH_MASK 0x000000FF The "
                       "function ignores regular line breaks in the message definition text."),),
        open_questions=(
            "The system message tables differ between builds: Server 2025 (26100) answers 4,493 ids from 0 to 16383, "
            "Server 2022 (20348) 4,427, and 109 texts are worded otherwise (id 334: \"The specified operation is not "
            "supported on a resident file.\" against \"An operation is not supported on a resident file.\"). w2m "
            "carries build 26100's; the contract rows are the forms, on which the builds agree.",
            "A failed call with ALLOCATE_BUFFER overwrites the caller's pointer (rows form.language_de_de, "
            "form.unknown_id print only that it changed); w2m writes NULL.",
            "A short caller's buffer: the rows show its first unit written ('T'); w2m writes as many units as fit.",
            "MAX_WIDTH_MASK on a message with a line break inside, other widths, inserts without IGNORE_INSERTS, "
            "FROM_STRING and FROM_HMODULE, and other languages are not in the rows and stop the run."),
        plants={
            "width_keeps_cr": Plant("    if (space) out[len - 1] = ' ';", "    if (space) out[len - 1] = '\\r';",
                                    ("form.max_width",)),
            "short_buffer_untouched": Plant(
                "            for (u32 i = 0; i < size && i < len; i++) out[i] = (unsigned char)text[i];\n", "",
                ("form.buffer_short",)),
            "pointer_left_on_failure": Plant("        if (flags & ALLOCATE_BUFFER) *(u64 *)buffer = 0;      /* the rows: the pointer is not left as it was */\n",
                                             "", ("form.language_de_de", "form.unknown_id"))}),
    "threads": Family(
        name="threads",
        contract_id="windows.thread.threads",
        version="0.2.0",
        summary=("Guest threads in the game's forms (specs/work_list.json), sequenced by events so every run prints "
                 "the same rows. CreateThread: a handle (a multiple of 4), a new id, the last error left; the new "
                 "thread runs each module's TLS callback then DllMain with DLL_THREAD_ATTACH, then its routine with "
                 "the last error 187, empty TLS slots and its own static TLS (the template's 41), and at its end the "
                 "same with DLL_THREAD_DETACH; created suspended, none of it until resumed. GetExitCodeThread: 259 "
                 "while it runs, then its return or ExitThread's code. SuspendThread and ResumeThread answer the "
                 "previous count; a suspended thread runs no instruction; on an ended thread SuspendThread fails with "
                 "5 and ResumeThread answers 0. GetThreadContext: TRUE, ContextFlags as asked, RIP in the image, RSP "
                 "in the thread's stack, CS 0x33, SS 0x2B. Priorities -2 and 0 read back, 100 is error 87; the "
                 "description calls answer 0x10000000; SleepEx answers 0; AppPolicyGetThreadInitializationType gives "
                 "policy 0."),
        probe="threads", guest_file="threads.c", dll_name="threads_dll.dll",
        runs={   # Sprint 061: FLS read from a worker and inside its DLL's thread attach (rows worker.start, worker.attach_fls)
              "38207005406": Run("windows-2025", "b08ba1fef04feddc4e5dad53663c5873a0322e162316b54247fa0565c9f05c4b", "threads-1791782002-0", "AMD EPYC 7763 64-Core Processor", dll="37a2ee1b08bfcdc6962a9b6138bac5d758eb3fe2eb3ddbdda866f80e981fef47"),
              "38207007264": Run("windows-2022", "17761236925f2c5da4380aa83b7e598b5e670e1248b75c2f04c763f20ee3260c", "threads-1791782002-1", "AMD EPYC 7763 64-Core Processor", dll="b679fd7f4c488095f5f03ae3516795b499b100556c15af1f4d8e0efdc4340063"),
        },
        documented=(),
        open_questions=(
            "The order of the TLS callbacks and DllMain calls between modules is not in the rows (the probe has one "
            "DLL); w2m calls each DLL's in load order, the program's TLS callbacks last.",
            "SwitchToThread's answer depends on what else is ready on the CPU; the row keeps only that it is TRUE or "
            "FALSE.",
            "A thread's stack in w2m is all committed above StackLimit with no guard pages, where Windows commits on "
            "demand behind a guard page; threads.c and VirtualQuery show the difference only to code that probes "
            "another thread's stack pages."),
        plants={
            "fls_shared": Plant(
                "    struct fls_block **slot = (struct fls_block **)((u8 *)w2m_teb() + TEB_FLS_DATA);",
                "    static struct fls_block *one;\n    struct fls_block **slot = &one;",
                ("worker.start", "worker.attach_fls", "main.after_worker"), file="thread_storage.c"),
            "start_last_error_kept": Plant('    code += b"\\x65\\xc7\\x04\\x25\\x68\\x00\\x00\\x00" + struct.pack("<I", START_LAST_ERROR)',
                                           '    code += b"\\x65\\xc7\\x04\\x25\\x68\\x00\\x00\\x00" + struct.pack("<I", 0)',
                                           ("worker.start",), host="w2m.scheduler"),
            "detach_as_attach": Plant("    code = b\"\".join(_call(fn, base, DLL_THREAD_DETACH) for base, fn in callbacks)",
                                      "    code = b\"\".join(_call(fn, base, DLL_THREAD_ATTACH) for base, fn in callbacks)",
                                      ("events.worker_ended", "events.spinner", "events.suspended_ran", "events.exiter"),
                                      host="w2m.scheduler"),
            "suspend_answers_new_count": Plant("            t.suspend += 1\n            return t.suspend - 1",
                                               "            t.suspend += 1\n            return t.suspend",
                                               ("suspend",), host="w2m.vm_backend"),
            "context_cs_32_bit": Plant('        struct.pack_into("<H", buf, 56, CS)', '        struct.pack_into("<H", buf, 56, CS - 0x10)',
                                       ("context",), host="w2m.scheduler"),
            "exited_suspend_error": Plant("        set_error(ERROR_ACCESS_DENIED);\n        return w2m_logged(W2M_API_SuspendThread",
                                          "        set_error(6);\n        return w2m_logged(W2M_API_SuspendThread",
                                          ("suspend.exited",))}),
    "dxgi": Family(
        name="dxgi",
        contract_id="windows.dxgi.dxgi",
        version="0.1.0",
        summary=("CreateDXGIFactory and the walk UnityPlayer makes with the factory (RVA 0x520700; game run 38206263976 "
                 "seq 3298-3301): EnumAdapters to its end, each adapter's EnumOutputs to its end, the outputs' and "
                 "adapters' descriptions, Release. On the runners, which have no GPU: two Microsoft Basic Render Driver "
                 "adapters (0x1414, 0x8C), no dedicated memory, 8,587,180,032 bytes shared; the first with one output, "
                 "\\\\.\\DISPLAY1 at 0,0-1024,768 on the primary monitor; DXGI_ERROR_NOT_FOUND past each end; "
                 "E_NOINTERFACE for another interface, E_POINTER for a NULL out pointer; every object new with one "
                 "reference."),
        probe="dxgi", guest_file="dxgi.c",
        runs={
              "38210591842": Run("windows-2025", "15e0aa0873454d1bbacd5b9c343b730970d6df7b776afeeccdf273caf9168dc9", "dxgi-1791784916-0", "INTEL(R) XEON(R) PLATINUM 8573C"),
              "38210594155": Run("windows-2022", "8fa545b798033b50b3254efc6919816129e168d774fe7041270fc653cf4d3445", "dxgi-1791784916-1", "AMD EPYC 7763 64-Core Processor"),
        },
        documented=(
            documented("ms-createdxgifactory", "api/dxgi/nf-dxgi-createdxgifactory", "2024-02-22",
                       "Creates a DXGI 1.0 factory that you can use to generate other DXGI objects."),
            documented("ms-enumadapters", "api/dxgi/nf-dxgi-idxgifactory-enumadapters", "2024-02-22",
                       "Returns S_OK if successful; otherwise, returns DXGI_ERROR_NOT_FOUND if the index is greater than "
                       "or equal to the number of adapters in the local system, or DXGI_ERROR_INVALID_CALL if ppAdapter "
                       "parameter is NULL.")),
        open_questions=(
            "The runners have no GPU, and w2m answers as they do: the Basic Render Driver twice, one output. A Mac "
            "adapter backed by Metal is milestone B's (plan, phase 6), with rows from a machine that has a GPU.",
            "The adapters' LUIDs are the machine's; the rows hold only that each is nonzero.",
            "IDXGIFactory1 and IDXGIFactory2, the adapters' and outputs' QueryInterface and every method the game does "
            "not call stop the run by name: no row shows them."),
        plants={
            "second_adapter_has_output": Plant("    if (((struct object *)self)->index != 0 || (u32)index >= 1) {",
                                               "    if ((u32)index >= 1) {",
                                               ("adapter.1.outputs.end.0",)),   # its Release still answers 0
            "null_out_invalid_arg": Plant("    if (!out) return w2m_logged(W2M_API_CreateDXGIFactory, iid, E_POINTER);",
                                          "    if (!out) return w2m_logged(W2M_API_CreateDXGIFactory, iid, 0x80070057u);",
                                          ("create.null_out",)),
            "output_on_another_monitor": Plant("    *(u64 *)(p + 0x58) = MONITOR;", "    *(u64 *)(p + 0x58) = MONITOR + 0x10000;",
                                               ("adapter.0.output.0", "game_walk")),
            "release_keeps_count": Plant("    s64 left = --o->refs;", "    s64 left = o->refs;",
                                         ("refcount", "adapter.0.release", "adapter.1.release", "release.last"))}),
    "com": Family(
        name="com",
        contract_id="windows.com.com",
        version="0.1.0",
        summary=("COM as the game uses it (specs/work_list.json): apartments per thread (S_OK, S_FALSE, "
                 "RPC_E_CHANGED_MODE; the process's multithreaded apartment serves a thread without one; "
                 "CO_E_NOTINITIALIZED with none); CoCreateInstance of the audio device enumerator, one object COM keeps "
                 "a reference to, REGDB_E_CLASSNOTREG for an unknown class, E_NOINTERFACE for an interface it lacks, "
                 "E_INVALIDARG for a NULL out pointer; no audio endpoint on the runners (empty collections, "
                 "GetDefaultAudioEndpoint E_NOTFOUND); notification clients registered without AddRef; "
                 "PropVariantClear, CoCreateGuid (version 4, variant 10), BSTRs and VariantClear."),
        probe="com", guest_file="com.c",
        runs={
              "38126872187": Run("windows-2025", "da7ca465e5f2b4e226c8f49a2d20f159c20e88b37d89e449dacf621fab691f09", "com-1791709273-0", "AMD EPYC 7763 64-Core Processor"),
              "38126873913": Run("windows-2022", "a6e5f68b6d7b952199acac0418054346609592ff7cd382c32d6608d6d9e8c229", "com-1791709273-1", "AMD EPYC 9V45 96-Core Processor"),
        },
        documented=(),
        open_questions=(
            "The runners have no audio endpoint, and w2m answers as they do: the game's audio code sees no device. "
            "An endpoint backed by the Mac's output is later work (sound), with rows from a machine that has one.",
            "UnityPlayer's first CoInitialize answers S_FALSE in the capture (the main thread already in an "
            "apartment by then, from a call the capture does not show); w2m answers S_OK.",
            "GetDevice, an endpoint collection's Item, aggregation, other class contexts and other VARIANT types "
            "are not in the rows and stop the run."),
        plants={
            "enumerator_count_reset": Plant("    if (!enumerator.vtable) {", "    if (1) {", ("refcount",)),
            "default_endpoint_found": Plant("    return w2m_logged(W2M_API_W2mEnumeratorGetDefaultAudioEndpoint, flow, E_NOTFOUND);",
                                            "    return w2m_logged(W2M_API_W2mEnumeratorGetDefaultAudioEndpoint, flow, 0x80004005u);",
                                            ("default.render_console", "default.render_multimedia", "default.capture_console")),
            "guid_version_1": Plant("    g[7] = (u8)((g[7] & 0x0F) | 0x40);", "    g[7] = (u8)((g[7] & 0x0F) | 0x10);", ("guid",)),
            "implicit_mta_refused": Plant("    if (!w2m_thread_scratch()->com_inits && !process_mta)",
                                          "    if (!w2m_thread_scratch()->com_inits)", ("create.implicit_mta",))}),
    "time": Family(
        name="time",
        contract_id="windows.time.time",
        version="0.1.0",
        summary=("The work list's time family as relations to the clocks w2m serves: GetSystemTimePreciseAsFileTime "
                 "(KERNELBASE, through GetProcAddress) within a second of GetSystemTimeAsFileTime, timeGetTime within a "
                 "second of GetTickCount, GetTickCount64's low half with GetTickCount; none going back over 1,000 reads, "
                 "each advancing 40 ms to 5 s across Sleep(50); timeBeginPeriod and timeEndPeriod answering 0, and 97 "
                 "(TIMERR_NOCANDO) for a period of 0."),
        probe="time", guest_file="time.c",
        runs={
              "38173127764": Run("windows-2025", "b96af5861876df13a97b0e822e3c03134b5ebedf40f6b027b8ec792e510239c6", "time-1791751169-0", "AMD EPYC 7763 64-Core Processor"),
              "38173129974": Run("windows-2022", "3cf5e2998c1778dc7f8e5d54fb9b55d9c0c59cc127649782e6a4987f95206c81", "time-1791751169-1", "AMD EPYC 7763 64-Core Processor"),
        },
        documented=(),
        open_questions=("A period ended that was not begun, and periods over 16 ms, are not in the rows and stop the run.",),
        plants={
            "precise_from_counter_only": Plant("    u64 now = w2m_filetime_now();\n    *(u32 *)out = (u32)now;\n    *(u32 *)(out + 4) = (u32)(now >> 32);\n    return w2m_logged(W2M_API_GetSystemTimePreciseAsFileTime",
                                               "    u64 now = w2m_counter_now();\n    *(u32 *)out = (u32)now;\n    *(u32 *)(out + 4) = (u32)(now >> 32);\n    return w2m_logged(W2M_API_GetSystemTimePreciseAsFileTime",
                                               ("precise.against_system_time",)),
            "time_in_ticks": Plant("(u32)(w2m_counter_now() / 10000));\n}\n\nu64 winmm_timeBeginPeriod",
                                   "(u32)(w2m_counter_now() / 1000));\n}\n\nu64 winmm_timeBeginPeriod",
                                   ("time_get_time.against_tick_count",)),
            "period_zero_accepted": Plant("    if ((u32)period == 0) return w2m_logged(W2M_API_timeBeginPeriod, period, TIMERR_NOCANDO);",
                                          "    if ((u32)period == 0) return w2m_logged(W2M_API_timeBeginPeriod, period, TIMERR_NOERROR);",
                                          ("period.begin_0",)),
            "tick_64_high_set": Plant("    return w2m_logged(W2M_API_GetTickCount64, 0, w2m_counter_now() / 10000);",
                                      "    return w2m_logged(W2M_API_GetTickCount64, 0, w2m_counter_now() / 10000 + (1ull << 32));",
                                      ("tick_count_64.against_tick_count",))}),
    "registry": Family(
        name="registry",
        contract_id="windows.registry.registry",
        version="0.3.0",
        summary=("RegOpenKeyExW and RegCloseKey as the game first calls them: a key below HKEY_CURRENT_USER\\Software "
                 "that does not exist is ERROR_FILE_NOT_FOUND with NULL written, names without case, Software opens, a "
                 "NULL or empty subkey gives the root's predefined handle, a handle that is no key ERROR_INVALID_HANDLE; "
                 "a key closes once; the last error is left. Sprint 063, the game's writes (Unity's PlayerPrefs, game run "
                 "38206263976 seq 2734-3490): RegCreateKeyW of its key, RegSetValueExA of REG_BINARY, REG_DWORD and "
                 "REG_SZ, RegQueryValueExA in the game's two forms and at its edges (REG_NONE written for a missing "
                 "value, ERROR_MORE_DATA, the size alone, ERROR_INVALID_PARAMETER for data without a size), names "
                 "without case in ASCII and Latin-1, ERROR_ACCESS_DENIED through a handle opened for reading, "
                 "RegDeleteValueA, and RegDeleteKeyW refusing a key with a subkey."),
        probe="registry", guest_file="registry.c",
        runs={   # Sprint 063: the game's writes and their edges
              "38210383833": Run("windows-2025", "5cffb57fa38d5a97fb95f46607823da0d42a4ec3232e9cff9b797f1a282828c6", "registry-1791784765-0", "AMD EPYC 7763 64-Core Processor"),
              "38210386046": Run("windows-2022", "c560b178aa6e68c0b37ddf59f16feb181fe47b6cade16234896b04f7a4b3a225", "registry-1791784765-1", "INTEL(R) XEON(R) PLATINUM 8573C"),
        },
        documented=(),
        open_questions=("w2m's registry starts each run with HKEY_CURRENT_USER\\Software and nothing below it, as a fresh "
                        "machine has; what a run writes stays for the rest of the run and nothing persists between runs "
                        "(the reference is the game's first run on a fresh runner). Other roots, value types and sizes "
                        "outside the rows stop the run.",),
        plants={
            "missing_untouched": Plant("    if (!exists(path)) {\n        *(u64 *)result = 0;", "    if (!exists(path)) {",
                                       ("open.missing", "open.missing_two_levels", "open.missing_case", "open.missing_below_software_handle", "open.u0173_oftware", "open.software_u0100_x", "open.long_s_oftware", "open.software_dotless_i",
                                        # Sprint 063: the game's key opened before it is made and after it is deleted
                                        "write.open_before", "write.open_after_delete")),
            "case_kept": Plant("return c >= 'A' && c <= 'Z' ? (u16)(c + 32) : c;", "return c;",
                               ("open.software", "open.software_case", "open.missing_below_software_handle",
                                "close.software", "close.software_case", "close.software_again")),
            "close_twice_succeeds": Plant("    keys[(h - KEY_BASE) / 4].open = 0;", "", ("close.software_again",)),
            "narrowed_again": Plant("return c >= 'A' && c <= 'Z' ? (u16)(c + 32) : c;",
                                    "return c >= 'A' && c <= 'Z' ? (u16)(c + 32) : (u16)(u8)c;",
                                    ("open.u0173_oftware", "open.software_u0100_x")),
            "root_new_handle": Plant("        *(u64 *)result = root;\n", "        *(u64 *)result = KEY_BASE + 0x100;\n",
                                     ("open.root_null_subkey", "open.root_empty_subkey", "close.root_null_subkey",
                                      "close.root_empty_subkey")),
            # Sprint 063: the value functions
            "latin1_unfolded": Plant("if ((c >= 'A' && c <= 'Z') || (c >= 0xC0 && c <= 0xDE && c != 0xD7)) c = (u16)(c + 32);",
                                     "if (c >= 'A' && c <= 'Z') c = (u16)(c + 32);", ("query.latin1_case",)),
            "missing_type_untouched": Plant("        if (type) *(u32 *)type = REG_NONE;\n", "",
                                            ("query.missing", "query.after_delete")),
            "short_buffer_copies": Plant("        if (*size < n) {", "        if (0) {", ("query.binary_20_short",)),
            "read_handle_writes": Plant(
                "    if (!(keys[slot].access & right)) { *status = ERROR_ACCESS_DENIED; return -1; }\n", "",
                ("set.through_read_handle", "delete.through_read_handle")),
            "sz_utf16_copy_skipped": Plant(
                "        for (u32 i = 0; i < n; i++) data[2 * i] = values[v].data[i], data[2 * i + 1] = 0;\n", "",
                ("query.sz_with_nul", "query.sz_without_nul")),
            "delete_value_kept": Plant(
                "    values[v].used = 0;\n    return w2m_logged(W2M_API_RegDeleteValueA, h, ERROR_SUCCESS);",
                "    return w2m_logged(W2M_API_RegDeleteValueA, h, ERROR_SUCCESS);",
                ("query.after_delete", "delete.again"))}),
    "file_search": Family(
        name="file_search",
        contract_id="windows.file.file_search",
        version="0.2.0",
        summary=("FindFirstFileExW in UnityPlayer's form (FindExInfoBasic, name match, no filter or flags), FindNextFileW "
                 "and FindClose: a folder's entries \".\" and \"..\" first, then by name without case; attributes 0x10 with "
                 "size 0 for a folder, 0x20 and the size for a file; times as GetFileAttributesExW's; no short name; * and ? "
                 "without case; a plain name found as stored; ERROR_NO_MORE_FILES at the end; ERROR_PATH_NOT_FOUND for a "
                 "missing folder, ERROR_FILE_NOT_FOUND for no match; FindClose of INVALID_HANDLE_VALUE ERROR_INVALID_HANDLE."),
        probe="file_search", guest_file="file_search.c",
        runs={
              "38182330015": Run("windows-2025", "664269b73531514ae7e205d5f4f0f2ba62b5c2d6f6351779f6ed39fa015e86f2", "file_search-1791759336-0", "INTEL(R) XEON(R) PLATINUM 8573C"),
              "38182331619": Run("windows-2022", "bd4cb3db57c693ffa810b711cd6b873ce8394565cb19f17688708783d78abc44", "file_search-1791759336-1", "AMD EPYC 7763 64-Core Processor"),
        },
        documented=(),
        open_questions=("FindExInfoStandard (short names), other search operations, flags, filters, the DOS wildcards and "
                        "\"*.*\" are not in the rows and stop the run.",),
        plants={
            "name_cut_short": Plant("#define NAME 255 ", "#define NAME 100 ", ("long_name*", "missing_pattern", "missing_file", "close.invalid")),
            "order_with_case": Plant("    return upper(*a) < upper(*b);", "    return *a < *b;", ("all.next1", "all.next2", "all.next3")),
            "folder_size_4096": Plant("    if (kind != 2) { *(u32 *)(d + 28)", "    if (1) { *(u32 *)(d + 28)",
                                      ("all", "all.next0", "all.next3", "folder_itself")),
            "case_matters": Plant("upper(*p) == upper(*s)", "*p == *s",
                                  ("exact_other_case", "exact_other_case.next0", "exact_other_case.close")),
            "end_error_left": Plant("        set_error(ERROR_NO_MORE_FILES);\n", "",
                                    ("all.next4", "pattern_txt.next0", "pattern_question.next0", "exact.next0", "exact_other_case.next0", "folder_itself.next0", "long_name.next0", "long_name_exact.next0")),
            "missing_folder_as_file": Plant("    if (kind != 2) {\n        set_error(ERROR_PATH_NOT_FOUND);",
                                            "    if (kind != 2) {\n        set_error(ERROR_FILE_NOT_FOUND);",
                                            ("missing_folder",))}),
    "version": Family(
        name="version",
        contract_id="windows.version.version",
        version="0.1.0",
        summary=("The version check under the game's manifest: VerSetConditionMask's three bits per type and bit 63; "
                 "RtlVerifyVersionInfo and VerifyVersionInfoW comparing major, minor and service pack as one version, "
                 "build, platform and product type alone, a condition of 0 skipped (STATUS_REVISION_MISMATCH, "
                 "ERROR_OLD_WIN_VERSION; a type mask of 0 STATUS_INVALID_PARAMETER, ERROR_BAD_ARGUMENTS); GetVersionExW "
                 "10.0, platform 2, product 3, suite 0x190; SHELL32's DllGetVersion 10.0.10586."),
        probe="version", guest_file="version.c",
        runs={
              "38173733146": Run("windows-2025", "3a49efbae6b80ad79bc6226b6d41c39241097745e032f0848118c10d034fef4f", "version-1791751693-0", "AMD EPYC 9V45 96-Core Processor"),
              "38173735116": Run("windows-2022", "a51dc8c571e44c403686444ec1206ef923c91fc0163d23c96f06079e465c11f7", "version-1791751693-1", "AMD EPYC 7763 64-Core Processor"),
        },
        documented=(),
        open_questions=("The build is w2m's profile (Server 2025, 26100); Server 2022 answers 20348, and the rows keep "
                        "the build as relations to the two.",
                        "VerifyVersionInfoW for a program whose manifest does not name Windows 10, and the suite "
                        "conditions, are not in the rows and stop the run."),
        derive=version_rows,
        plants={
            "mask_no_high_bit": Plant("    return w2m_logged(W2M_API_VerSetConditionMask, t, mask | 1ull << 63);",
                                      "    return w2m_logged(W2M_API_VerSetConditionMask, t, mask);",
                                      ("mask.game_*", "mask.type_*", "mask.two_types_at_once", "mask.replace")),
            "fields_apart": Plant("        if (!decided) ok = holds(0, 0, c, api);", "        if (!decided) ok = holds(0, 0, c, api);\n        for (int i = 0; i < 4; i++) if (types & order[i] && !holds(have[i], want[i], c, api)) ok = 0;",
                                  ("verify.game_6_1", "verify.game_6_2")),
            "condition_zero_checked": Plant("    if (c) {\n        int decided",
                                            "    if (!c && types & 0x33) return STATUS_REVISION_MISMATCH;\n    if (c) {\n        int decided",
                                            ("verify.type_without_condition",)),
            "unmanifested_version": Plant("    int windows_10 = (w2m_manifest() & W2M_MANIFEST_WINDOWS_10) != 0;",
                                          "    int windows_10 = 0;", ("get_version.ex", "get_version.plain"),
                                          file="system_info.c"),
            "shell32_system_build": Plant("#define SHELL32_BUILD 10586u", "#define SHELL32_BUILD 26100u",
                                          ("dll_get_version.info", "dll_get_version.info2"))}),
    "display": Family(
        name="display",
        contract_id="windows.display.display",
        version="0.1.0",
        summary=("The display as the game queries it, under its manifest (PerMonitorV2): GetSystemMetrics 0-99, the "
                 "Hyper-V adapter and its one monitor (EnumDisplayDevicesA), EnumDisplayMonitors' one callback, "
                 "GetMonitorInfoA and W, MonitorFromPoint, Rect and Window, shcore's GetDpiForMonitor (96 x 96), "
                 "EnumDisplaySettingsW's current settings and 13 modes; one 1024 x 768 monitor, the work area above the "
                 "taskbar."),
        probe="display", guest_file="display.c",
        runs={
              "38174316653": Run("windows-2025", "82d7d53cf6046d5dc6ce1250fa32c2ad6374cd5fed028e22d419f5d5998d9b1c", "display-1791752210-0", "AMD EPYC 9V74 80-Core Processor"),
              "38174318347": Run("windows-2022", "d1b4bbaa68f38a42a29c9bb3df5c1debcb150795f6bbf16a9bbe957d93839a9b", "display-1791752210-1", "INTEL(R) XEON(R) PLATINUM 8573C"),
        },
        documented=(),
        open_questions=("w2m's display is the runners' (one 1024 x 768 monitor at 96 DPI, Server 2025's taskbar) until "
                        "the game's window is a Mac window (phase 6).",
                        "GetSystemMetrics(93), which has no SM_ name, answers a different value on each run; w2m "
                        "answers 0."),
        derive=display_rows,
        plants={
            "metrics_unaware": Plant("    1024, 768, 17, 17, 23, 1, 1, 3, 3,", "    1280, 768, 17, 17, 23, 1, 1, 3, 3,", ("metric.0",)),
            "monitor_string": Plant("        copy(d + 36, \"Generic PnP Monitor\", 128);", "        copy(d + 36, \"Generic Monitor\", 128);",
                                    ("devices.adapter_0_monitor_0",)),
            "callback_rect_work": Plant("    f->rect[3] = SCREEN_H;", "    f->rect[3] = WORK_H;", ("enum_monitors.callback",)),
            "dpi_scaled": Plant("    *(u32 *)x = 96; *(u32 *)y = 96;", "    *(u32 *)x = 144; *(u32 *)y = 144;",
                                ("dpi.effective", "dpi.angular")),
            "null_window_null": Plant("    if (window) w2m_unimplemented(W2M_API_MonitorFromWindow, window);           /* not a window: no row */\n    return outside(",
                                      "    if (window) w2m_unimplemented(W2M_API_MonitorFromWindow, window);           /* not a window: no row */\n    return flags == 2 ? w2m_logged(W2M_API_MonitorFromWindow, 0, 0) : outside(",
                                      ("from_window.null_nearest",)),
            "one_mode_short": Plant("    else if (m >= 0 && m < 13) {", "    else if (m >= 0 && m < 12) {",
                                    ("settings.mode_12", "settings.mode_count"))}),
    "windows": Family(
        name="windows",
        contract_id="windows.windows.windows",
        version="0.3.0",
        summary=("Window classes, the game's two windows (the batch run's: a RegisterClassW class by atom, style 0, "
                 "CW_USEDEFAULT; the play run's: 0xCA0000, 320 x 240) and what their procedure sees: WM_GETMINMAXINFO, "
                 "WM_NCCREATE, WM_NCCALCSIZE, WM_CREATE in order, DefWindowProcW's answers, the cascade and default "
                 "size, the frame (8 at the sides and bottom, 31 at the top), window longs and extra bytes, the "
                 "rectangles; LoadIconA by id and IDI_APPLICATION, LoadCursorA(IDC_ARROW). Sprint 063: the game's "
                 "later calls on its window (batch run 38206263976): SetWindowLongPtrA, the queue (WM_DWMNCRENDERINGCHANGED "
                 "posted to each new window; PeekMessageA, GetMessageExtraInfo, TranslateMessage, DispatchMessageA; "
                 "PostMessageW, WM_QUIT), RegisterDeviceNotificationW, GetFocus and SetFocus with the activation's "
                 "messages, LoadImageW of a cursor file, a missing file and a missing icon."),
        probe="windows", guest_file="windows.c",
        # the probe writes its cursor file under GetTempPathW, which reads TMP (set on the runners; Sprint 063)
        environment=("TEMP=C:\\Users\\w2m\\AppData\\Local\\Temp", "TMP=C:\\Users\\w2m\\AppData\\Local\\Temp"),
        runs={   # Sprint 063: the game's later calls and the boundaries (probe windows, rows from create.game on)
              "38210803867": Run("windows-2025", "2fe586f116a1fb28b546e1009d605675be018d6001503aca11660d6985bba172", "windows-1791785079-0", "AMD EPYC 9V74 80-Core Processor"),
              "38210806334": Run("windows-2022", "6e75412ca9f139bdf500185bbe5fc8c996dbe1e339fb173a25cba5f3fbc591e7", "windows-1791785079-1", "AMD EPYC 9V74 80-Core Processor"),
        },
        documented=(),
        open_questions=("Where CW_USEDEFAULT places a window depends on the windows the desktop made before (52, 78, 130 "
                        "seen for a probe's first); w2m's cascade starts at 52.",
                        "The default height is fitted: 3/4 of the work area's height less 21 on both runners (519, 525).",
                        "Windows are records; nothing is drawn, and ShowWindow is later work.",
                        "Message 0x60, undocumented (wiert.me, 2020-12-22, lists it among loop messages without "
                        "documentation), is posted to a system window of the thread some time after its window is "
                        "activated: the probe met it after a posted WM_USER message on every run (wParam 0x80000001 on "
                        "Server 2025, 1 on 2022), and the game dispatched one at seq 3491-3493 to a procedure it never "
                        "saw. Its arrival varies (one execution met two in the WM_QUIT drain); w2m posts none, and the "
                        "derivation takes it out of each drain (windows_rows).",
                        "On Windows DefWindowProcW's WM_ACTIVATE sends WM_IME_SETCONTEXT, WM_IME_NOTIFY and the first "
                        "WM_SETFOCUS from inside; w2m sends them from SetFocus in the same order. A procedure that keeps "
                        "WM_ACTIVATE from DefWindowProcW would get them in w2m and not on Windows.",
                        "LoadImageW of a cursor file left the last error 0 in the probe; the game's call left 6 (its "
                        "file, written past the capture's 32-call count, is not on the record)."),
        derive=windows_rows,
        plants={
            "minmax_not_sent": Plant("    w2m_call_guest(&f->call, w->procedure, h, WM_GETMINMAXINFO, 0, (u64)f->minmax);",
                                     "    f->step = 1;\n    w2m_call_guest(&f->call, w->procedure, h, WM_NCCREATE, 0, (u64)f->create);",
                                     ("msg.batch.*", "create.batch", "batch.client_rect", "batch.window_rect", "msg.play.*", "create.play", "play.window_rect", "msg.offscreen.*", "create.offscreen",
                                      # Sprint 063: the game's and the other thread's windows; the game window's later
                                      # messages are numbered after its creation's, so one fewer moves each of them
                                      # (msg.game.4, 5, 10 and 11 land on rows of the same message)
                                      "msg.game.0", "msg.game.1", "msg.game.2", "msg.game.3", "create.game", "msg.game.6", "msg.game.7", "msg.game.8", "msg.game.9",
                                      "msg.game.12", "msg.game.13", "msg.game.14", "msg.game.15", "msg.game.16", "msg.game.17", "msg.game.18", "msg.game.19", "msg.game.20",
                                      "msg.game.21", "msg.game.22", "msg.other.*", "create.other")),
            "frame_thin": Plant("#define FRAME_TOP 31 ", "#define FRAME_TOP 26 ",
                                ("msg.batch.2", "batch.client_rect", "msg.play.2", "play.client_rect", "msg.offscreen.2", "msg.game.2", "msg.other.2")),
            "height_from_screen": Plant("        cy = (work[3] - work[1]) * 3 / 4 - 21;", "        cy = 768 * 3 / 4 - 21;",
                                        ("msg.batch.2", "batch.client_rect", "batch.window_rect", "msg.game.2", "msg.other.2")),
            "no_edge_in_create": Plant("    *(u32 *)(cs + 72) = ((u32)style & WS_CAPTION) == WS_CAPTION ? WS_EX_WINDOWEDGE : 0;",
                                       "    *(u32 *)(cs + 72) = 0;", ("msg.play.1", "msg.play.3", "msg.offscreen.1", "msg.offscreen.3")),
            "extra_unbounded": Plant("        if (index + 8 > classes[w->cls].extra) return 0;", "        if (index + 8 > EXTRA) return 0;",
                                     ("batch.long_ptr_8", "play.long_ptr_8")),
            "duplicate_class_allowed": Plant("        if (classes[k].instance != instance) w2m_unimplemented(api, instance);        /* another module's: no row */\n        set_error(ERROR_CLASS_ALREADY_EXISTS);\n        return w2m_logged(api, name, 0);",
                                             "        if (classes[k].instance != instance) w2m_unimplemented(api, instance);        /* another module's: no row */\n        return w2m_logged(api, name, classes[k].atom);",
                                             ("class.register_ex_again", "class.name_64_again", "class.name_254_again", "class.name_255_again")),
            "class_limit_256": Plant("#define CLASS_NAME 255 ", "#define CLASS_NAME 256 ",
                                     ("class.name_256", "class.name_256_again")),
            "monitor_ignores_window": Plant("        if (r[0] < SCREEN_W && r[2] > 0 && r[1] < SCREEN_H && r[3] > 0)\n            return w2m_logged(W2M_API_MonitorFromWindow",
                                            "        if (1)\n            return w2m_logged(W2M_API_MonitorFromWindow",
                                            ("offscreen.monitor",), file="display.c"),
            "icon_error_left": Plant("    set_error(0);\n    return w2m_logged(W2M_API_LoadIconA", "    return w2m_logged(W2M_API_LoadIconA",
                                     ("icon.own_103",)),
            # Sprint 063
            "no_render_message": Plant("        post(w->owner, f->window, WM_DWMNCRENDERINGCHANGED, 1, 0);", "",
                                       ("queue.first.*", "msg.game.[4-9]", "msg.game.1[0-9]", "msg.game.2[0-2]")),
            "quit_code_dropped": Plant("    t->quit_code = (u64)(s64)(s32)code;", "    t->quit_code = 0;", ("queue.quit.00",)),
            "focus_any_thread": Plant("    if (w->owner != tid) {                                                  /* row focus.other_thread */",
                                      "    if (0) {", ("focus.other_thread",)),
            "no_activate_app_to_others": Plant("        if (windows[i].used && other != h && windows[i].owner == tid) SEND(other, WM_ACTIVATEAPP, 1, 1);",
                                               "        if (0 && other) SEND(other, WM_ACTIVATEAPP, 1, 1);", ("msg.game.1[0-9]", "msg.game.2[0-2]", "focus.set")),
            "activate_sends_nothing": Plant("    int sends = ((u32)msg == WM_ACTIVATE && (u32)wparam == 1) ||", "    int sends = 0 ||",
                                            ("msg.game.1[5-9]", "msg.game.2[0-2]", "focus.set")),
            "focus_returns_null": Plant("        w2m_return_to(f->rsp, w2m_logged(W2M_API_SetFocus, f->window, f->window));",
                                        "        w2m_return_to(f->rsp, w2m_logged(W2M_API_SetFocus, f->window, 0));", ("focus.set",)),
            "cursor_same_handle": Plant("CURSOR_BASE + 0x10 * ++cursors);", "CURSOR_BASE + 0x10 * (cursors = 1));", ("image.cursor_file_again",)),
            "missing_file_error_3": Plant("            set_error(error);                                                  /* row image.missing_file */",
                                          "            set_error(3);", ("image.missing_file",)),
            "notification_error_left": Plant("    set_error(0);\n    return w2m_logged(W2M_API_RegisterDeviceNotificationW", "    return w2m_logged(W2M_API_RegisterDeviceNotificationW", ("device_notification",))}),
    "winrt": Family(
        name="winrt",
        contract_id="windows.winrt.winrt",
        version="0.1.0",
        summary=("RoInitialize as UnityPlayer reaches it: the winrt, com and winrt-string API sets load as combase.dll, "
                 "whose RoInitialize is the COM apartment (S_OK, S_FALSE, RPC_E_CHANGED_MODE, E_INVALIDARG for another "
                 "type) and RoUninitialize balances it."),
        probe="winrt", guest_file="com.c",
        runs={
              "38178400798": Run("windows-2025", "3795d70ec166716f65d011203027e0e8a6896a71e1a8687e34a50998559ec6f5", "winrt-1791755781-0", "AMD EPYC 9V74 80-Core Processor"),
              "38178402576": Run("windows-2022", "11bb048fbe6f9acbc7a0fd308aab24598dbb3d59210024a9a9c35de7f201456d", "winrt-1791755781-1", "AMD EPYC 7763 64-Core Processor"),
        },
        documented=(),
        open_questions=("The rest of the winrt family (RoActivateInstance, HSTRINGs) is not in these rows.",),
        plants={
            "type_unchecked": Plant("    if ((u32)type > 1) return w2m_logged(W2M_API_RoInitialize, type, E_INVALIDARG);", "",
                                    ("init.bad_type",)),
            "single_as_multi": Plant("    return init(W2M_API_RoInitialize, (u32)type ? MTA : STA);",
                                     "    return init(W2M_API_RoInitialize, MTA);", ("init.multi_after_single", "init.multi"))}),
    "processes": Family(
        name="processes",
        contract_id="windows.process.processes",
        version="0.2.0",
        summary=("A second process as UnityPlayer starts its crash handler: named events shared by name "
                 "(ERROR_ALREADY_EXISTS, one object), CreatePipe and SetHandleInformation, CreateProcessW with inherited "
                 "handles and the pipe as the child's standard input, framed writes and reads, ERROR_BROKEN_PIPE when the "
                 "writer is gone, OpenProcess by id, waits on a process (WAIT_TIMEOUT, STILL_ACTIVE, then the exit code), "
                 "WaitForMultipleObjectsEx; SetConsoleCtrlHandler. Since 0.2.0 (Sprint 058, the review of Sprint 055): a "
                 "child gets the parent's environment and directory as they are when it starts; a writer's process "
                 "ending breaks its pipe; a pipe of size 0 holds 4096 bytes, and a writer waits for room."),
        probe="processes", guest_file="processes.c",
        runs={
              "38189162864": Run("windows-2025", "6a37ef1f1ea85d0e2fbc5bb09c1b1daae97c78729d65796f3ffea7d2de96fce9", "processes-1791765864-0", "Intel(R) Xeon(R) Platinum 8370C CPU @ 2.80GHz"),
              "38189164581": Run("windows-2022", "232d481b305e92488da79d45bd0bb1b13409d74b268fb7ee0c536bb5e665a0d3", "processes-1791765864-1", "AMD EPYC 7763 64-Core Processor"),
        },
        documented=(
            documented("ms-createprocessw", "api/processthreadsapi/nf-processthreadsapi-createprocessw", "2024-02-22",
                       "If this parameter is NULL, the new process uses the environment of the calling process. ... "
                       "If this parameter is NULL, the new process will have the same current drive and directory as "
                       "the calling process."),
            documented("ms-createpipe", "api/namedpipeapi/nf-namedpipeapi-createpipe", "2021-10-13",
                       "nSize: The size of the buffer for the pipe, in bytes. The size is only a suggestion; the "
                       "system uses the value to calculate an appropriate buffering mechanism. If this parameter is "
                       "zero, the system uses the default buffer size."),),
        open_questions=("Each process is a VM of its own; the objects they share are the host's (src/w2m/kernel.py).",
                        "An application name, an environment, a directory, creation flags other than "
                        "NORMAL_PRIORITY_CLASS, a child with no standard handles, and a write no process reads are not "
                        "in the rows and stop the run."),
        derive=processes_rows,
        plants={
            "broken_pipe_as_end_of_file": Plant("        set_error(ERROR_BROKEN_PIPE);\n        return 0;", "        return 1;",
                                                ("child.record_3", "writer.pipe")),
            "exists_unreported": Plant("    set_error(existed ? ERROR_ALREADY_EXISTS : 0);", "    set_error(0);", ("event.create_again", "child.event_0", "child.event_1", "child.event_2"),
                                       file="sync_objects.c"),
            "exit_code_unread": Plant("    u64 code = (u32)pid == current_pid() ? STILL_ACTIVE\n                                          : w2m_host(",
                                      "    u64 code = (u32)pid != 0 ? STILL_ACTIVE\n                                          : w2m_host(",
                                      ("process.exited", "writer.pipe")),
            "remove_missing_accepted": Plant("    set_error(ERROR_INVALID_PARAMETER);\n    return w2m_logged(W2M_API_SetConsoleCtrlHandler, routine, 0);",
                                             "    return w2m_logged(W2M_API_SetConsoleCtrlHandler, routine, 1);", ("ctrl.remove_again",)),
            "create_error_left": Plant("    set_error(0);                                                     /* row process.create */", "",
                                       ("process.create",)),
            # Sprint 058: one plant per defect the review of Sprint 055 found and the new rows judge
            "environment_at_start": Plant("vm.environment_now(), directories[0]", "vm.environment, directories[0]",
                                          ("writer.environment",), host="w2m.kernel"),
            "directory_not_inherited": Plant('directory = self.directory if self.directory is not None else ',
                                             'directory = None or ', ("writer.directory",), host="w2m.vm_backend"),
            "pipe_unbounded": Plant("                n = min(a[3], PIPE_CAPACITY - len(pipe.data))",
                                    "                n = a[3]", ("pipe.capacity", "pipe.large_write"), host="w2m.kernel"),
            # a short write reported whole: the filler counts 65536 bytes into a pipe holding 4096, the drain waits for
            # the rest, and the run deadlocks before the probe prints its rows (all at the end)
            "write_reports_part": Plant("    if (done < (count & 0xFFFFFFFFull)) {", "    if (0) {", ("*",)),
            "ended_process_keeps_handles": Plant("                    self._release(current)\n", "",
                                                 ("*",), host="w2m.kernel")}),
    "process_parameters": Family(
        name="process_parameters",
        contract_id="windows.process.parameters",
        version="0.1.0",
        summary=("PEB.ProcessParameters (RTL_USER_PROCESS_PARAMETERS) for a GUI program started by the runner and "
                 "started as Steam starts the game: Flags 0x4001; MaximumLength equal to Length; no console handle; "
                 "the standard handles GetStdHandle answers, kept current by SetStdHandle; the current directory "
                 "with a final backslash in a 0x208-byte buffer outside the block, moved by SetCurrentDirectoryW; no "
                 "DLL path; the image path, command line (GetCommandLineW's own buffer), title and desktop inside "
                 "the block, the last two GetStartupInfoW's own buffers; the environment outside it, its size the "
                 "bytes in use, moved by SetEnvironmentVariableW; the window fields GetStartupInfoW's; the block in "
                 "the process heap."),
        probe="process_parameters", guest_file="console.c",
        runs={
              "38194049597": Run("windows-2025", "37cd0420619adb06f4e2500a8003be37f0c97f197e3e04ed86eb8802001949ed", "process_parameters-1791770394-0", "AMD EPYC 7763 64-Core Processor"),
              "38194051809": Run("windows-2022", "7a54f8eddbaa4ce5b16a615833b0d70075656cf4396ae4bd15126758d16637a4", "process_parameters-1791770394-1", "AMD EPYC 9V45 96-Core Processor"),
        },
        documented=(
            documented("ms-rtl-user-process-parameters", "api/winternl/ns-winternl-rtl_user_process_parameters",
                       "2021-07-27", "ImagePathName: The path of the image file for the process. CommandLine: The "
                       "command-line string passed to the process."),),
        open_questions=(
            "Offsets past CommandLine are phnt's (Process Hacker's reverse-engineered ntrtl.h); Microsoft documents "
            "the structure as reserved fields around ImagePathName and CommandLine.",
            "The rows hold a GUI program's block; a console program's ConsoleHandle (a handle to its console) is not in "
            "them, and w2m leaves it NULL: no service w2m serves reads it.",
            "A child given standard handles (STARTF_USESTDHANDLES) has them in its block; its WindowFlags are not in "
            "the rows and stay 0."),
        derive=parameters_rows,
        plants={
            "flags_normalized_only": Plant("PARAMETERS_FLAGS = 0x4001", "PARAMETERS_FLAGS = 0x0001",
                                           ("parent.header", "child.header"), host="w2m.vm_backend"),
            "set_std_handle_elsewhere": Plant("    std_handles()[i] = handle;", "    (void)handle;",
                                              ("parent.after.set_std_handle",)),
            "title_copied": Plant("    *(u64 *)(si + 24) = *(const u64 *)(p + 0xB8);          /* lpTitle: WindowTitle.Buffer */",
                                  "    *(u64 *)(si + 24) = *(const u64 *)(p + 0xB8) + 2;",
                                  ("parent.strings", "child.strings"), file="process.c"),
            "environment_in_place": Plant("    if (from == to) to += W2M_ENVIRONMENT_SIZE / 4;", "    to = from;",
                                          ("parent.after.set_environment",), file="environment.c"),
            "environment_size_stale": Plant("    *(u64 *)(p + 0x3F0) = 2 * (n == 1 ? 2 : n);", "",
                                            ("parent.after.set_environment",), file="environment.c"),
            "directory_same_buffer": Plant("    if (now == next) next += 0x208 / 2;", "    next = now;",
                                           ("parent.after.current_directory",), file="paths.c")}),
    "network": Family(
        name="network",
        contract_id="windows.network.network",
        version="0.1.0",
        summary=("WSAStartup and WSACleanup in the game's forms (mono 2.0, UnityPlayer 2.2): the version granted at "
                 "most 2.2, wHighVersion 2.2, \"WinSock 2.0\" and \"Running\", socket limits 0 from 2.0 and 32767 and "
                 "65467 for 1.1; 0.0 is WSAVERNOTSUPPORTED, a NULL WSADATA WSAEFAULT; each success owes one "
                 "WSACleanup, which answers 0 and the last error 0, and SOCKET_ERROR with WSANOTINITIALISED after."),
        probe="network", guest_file="network.c",
        runs={
              "38127406287": Run("windows-2025", "5b9775dd1e1878ccb61334b584054ca1f631a3e9b6ebdb6b8ccc0e8fa6a65d02", "network-1791709809-0", "AMD EPYC 9V45 96-Core Processor"),
              "38127407661": Run("windows-2022", "b5a4a5e85883bc29f7f7765b0630090d2bc50484645e45bfdf70c6352f0c0a9b", "network-1791709809-1", "AMD EPYC 7763 64-Core Processor"),
        },
        documented=(),
        open_questions=("Sockets themselves (socket, connect, send) are not called in the captures and are not served.",),
        plants={
            "version_cap_9_9": Plant("major > 2 || (major == 2 && minor > 2) ? 0x0202u", "major > 9 || (major == 9 && minor > 9) ? 0x0202u",
                                     ("startup.3_0",)),
            "cleanup_leaves_error": Plant("    owed--;\n    set_error(0);", "    owed--;",
                                          ("cleanup.1", "cleanup.2", "cleanup.3", "cleanup.4"))}),
    "exceptions": Family(
        name="exceptions",
        contract_id="windows.exception.exceptions",
        version="0.1.0",
        summary=("Structured exceptions as the game's modules use them, with the probe's own __C_specific_handler as "
                 "their static C runtimes carry: RaiseException's record (flags 0x80, the arguments, no chained "
                 "record, the last error left) reaches vectored handlers first, then each frame's language handler "
                 "through .pdata; EXECUTE_HANDLER unwinds with RtlUnwindEx, running each __finally, to the __except "
                 "with the code in RAX; CONTINUE_EXECUTION returns from RaiseException; continuing a noncontinuable "
                 "exception raises STATUS_NONCONTINUABLE_EXCEPTION (flags 0x81, no chained record); unwinding crosses "
                 "frame-pointer, large-allocation and saved-XMM frames; access violations carry the access and the "
                 "address; RtlCaptureContext, RtlLookupFunctionEntry, RtlVirtualUnwind and RtlPcToFileHeader answer "
                 "for the probe's own code; an exception no frame takes reaches the top-level filter."),
        probe="exceptions", guest_file="exceptions.c",
        runs={
              "38165931122": Run("windows-2025", "a71f491c4e8b9c1a563597497a004b74add2cf9caca0c7a9d3b36b041f34553a", "exceptions-1791745002-0", "AMD EPYC 7763 64-Core Processor"),
              "38165933249": Run("windows-2022", "55369a03c431c6c2448a9ddbe99ebcfe452227a2fcb79c184015697bc57cb6b3", "exceptions-1791745002-1", "AMD EPYC 7763 64-Core Processor"),
        },
        documented=(
            documented("ms-rtlunwindex", "api/winnt/nf-winnt-rtlunwindex", "2024-02-02",
                       "A value to be placed in the integer function return register before continuing execution."),),
        open_questions=(
            "C++ exceptions (STATUS_UNWIND_CONSOLIDATE), collided unwinds and an exception raised inside a handler are "
            "not in the rows: the game's Windows run raises one exception, mono's thread name, and none from "
            "UnityPlayer.",),
        plants={
            "no_software_origin": Plant("((u32)flags & EXCEPTION_NONCONTINUABLE) | EXCEPTION_SOFTWARE_ORIGINATE;",
                                        "((u32)flags & EXCEPTION_NONCONTINUABLE);",
                                        ("except.thread_naming", "unhandled.top_filter")),
            "vectored_skipped": Plant("    u64 h = w2m_vectored_handler(f->next_vectored++);",
                                      "    u64 h = 0 * w2m_vectored_handler(f->next_vectored++);",
                                      ("except.vectored_first",)),
            "finally_skipped": Plant("        if (handler) {\n            if (establisher == f->target_frame)",
                                     "        if (handler && establisher == f->target_frame) {\n            if (establisher == f->target_frame)",
                                     ("except.search_order", "except.deep")),
            "no_return_value": Plant("    f->ctx.gpr[RAX] = f->return_value;\n", "",
                                     ("except.thread_naming",)),
            "noncontinuable_continues": Plant("    if (f->rec.flags & EXCEPTION_NONCONTINUABLE) raise_noncontinuable(f);",
                                              "    if (0 & f->rec.flags & EXCEPTION_NONCONTINUABLE) raise_noncontinuable(f);",
                                              ("except.noncontinuable",)),
            "frame_register_ignored": Plant("                case 3: c->gpr[RSP] = c->gpr[frame_reg] - 16 * frame_off; *frame = c->gpr[RSP]; break;",
                                            "                case 3: break;",
                                            # the frame-pointer frame resumes with the wrong registers: the rows
                                            # after it fail too
                                            ("except.deep", "fault.write_null", "fault.read_low", "rtl.queries",
                                             "unhandled.top_filter")),
            "no_top_filter": Plant("    if (filter && f->step != STEP_TOP_FILTER) {",
                                   "    if (!filter && f->step != STEP_TOP_FILTER) {",
                                   ("unhandled.top_filter",))}),
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
        "acceptance": {"relation": "exact_rows", "key": "s", "rows": first,
                       **({"derivation": f"scripts/family_run.py {derivation_name(f)}: "
                                         + " ".join((f.derive.__doc__ or "").split())} if f.derive is not raw_rows else {})},
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


def derivation_name(f: Family) -> str:
    if not (f.derive.__doc__ or "").strip():
        raise ValueError(f"{f.name}: its derive function says nothing of what it keeps (Sprint 056)")
    return f.derive.__qualname__.replace(".<locals>.", ".") if "<locals>" in f.derive.__qualname__ else f.derive.__name__


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
        # the program's own file at its path, as on the runner (Sprint 055: a probe that starts itself reads it)
        (Path(d) / "probe.exe").write_bytes((ROOT / "store" / "binaries" / f"{binary}.exe").read_bytes())
        line = None if f is None or not f.arguments else f'"C:\\w2m\\app\\probe.exe" {f.arguments}'
        if dll is None and f is not None and f.dll_name is not None:
            dll = next(r.dll for r in f.runs.values() if r.binary == binary)
        dll_bytes = None if dll is None else (ROOT / "store" / "binaries" / f"{dll}.dll").read_bytes()
        if f is not None and f.dll_on_disk and f.dll_name is not None and dll_bytes is not None:
            (Path(d) / f.dll_name).write_bytes(dll_bytes)    # as the runner's out/ has it beside probe.exe

        def dlls(name: str) -> bytes | None:
            return dll_bytes if f is not None and f.dll_name is not None and name.lower() == f.dll_name.lower() else None

        environment = () if f is None else f.environment
        vm = VM(load((ROOT / "store" / "binaries" / f"{binary}.exe").read_bytes(), dlls, "probe.exe"), files=files,
                runtime_sources=sources, image_name="probe.exe",      # the runner ran it as probe.exe
                gui=f is not None and f.gui, command_line=line, environment=environment)
        # every probe runs on a kernel.Machine: a probe that starts a process gets it (Sprint 055)
        machine = Machine(spawner(files, dlls, sources), max_steps=1 << 40, timeout_ms=60_000)
        try:
            r = machine.run(vm)                                  # the clock probes spin until time advances
            foreign = foreign_processes(machine, sources)
            if foreign:
                r.stop = foreign
        finally:
            for other in machine.processes.values():
                if other.vm is not vm:
                    other.vm.close()
                    other.vm.files.close_all()
            vm.close()
            files.close_all()
        out = r.stdout
        if f is not None and f.output_file is not None:
            written = Path(d) / f.output_file
            out = written.read_bytes() if written.exists() else b""
    return out, r.stop, time.monotonic() - t0


FOREIGN = "a process ran a runtime not built from the candidate's sources"


def foreign_processes(machine: Machine, sources: dict[str, str] | None) -> str | None:
    """Each process the run started (the main one, and every child the Machine built) holds the runtime image built
    from `sources`, or the run says which did not (Sprint 057; KIT_DIARY #46: a plant reached the parent and not the
    child, and its run reported nothing)."""
    from w2m.vm_backend import build_runtime
    want = build_runtime(sources=sources).sha256
    other = [f"process {p.pid}: {p.vm.runtime.sha256[7:19]}"
             for p in machine.processes.values() if p.vm.runtime.sha256 != want]
    return f"{FOREIGN} ({want[7:19]}): {'; '.join(other)}" if other else None


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


@functools.cache
def runtime_objects() -> dict[str, guest_runtime.ObjectFile]:
    """The runtime's object files as the VM's image links them: each file's external symbols and headers."""
    from w2m.vm_backend import MONITOR_VA, RUNTIME_IMAGE, runtime_defines
    return guest_runtime.objects(MONITOR_VA + RUNTIME_IMAGE, runtime_defines())


def host_named_symbols() -> tuple[str, ...]:
    """Runtime symbols the host reaches by name rather than through an import: the image's entry (guest_runtime's
    ld -e) and each symbols["..."] and runtime_symbol("...") in vm_backend.py."""
    text = (ROOT / "src" / "w2m" / "vm_backend.py").read_text()
    return ("w2m_image_entry", *sorted(set(re.findall(HOST_NAMED, text))))


HOST_NAMED = re.compile(r'(?:symbols\["|runtime_symbol\(")(\w+)"')


def family_binaries(f: Family) -> list[bytes]:
    """The probe binaries (and DLLs) the runners ran for the family."""
    names = sorted({h for r in f.runs.values() for h in (f"{r.binary}.exe", f"{r.dll}.dll" if r.dll else None) if h})
    return [(ROOT / "store" / "binaries" / n).read_bytes() for n in names]


def runtime_files(f: Family) -> tuple[str, ...]:
    """The runtime files a family's runs can reach (Sprint 057): from its guest file, the files defining the services
    its probe binaries import and the symbols the host names, every file that defines a symbol a reached file uses."""
    from w2m.pe import parse as parse_pe
    from w2m.vm_backend import service_for
    objects = runtime_objects()
    definer = {symbol: name for name, o in objects.items() for symbol in o.defined}
    by_name = {symbol: name for name, o in objects.items() for symbol in o.local} | definer
    reached = {f.guest_file} | {by_name[s] for s in host_named_symbols()}
    for data in family_binaries(f):
        image = parse_pe(data)
        for dll, names in [*image.imports.items(), *image.delay_imports.items()]:
            for fn in names:
                service = service_for(dll, fn)
                if service is not None:
                    reached.add(definer[service[0]])
    todo = list(reached)
    while todo:
        for symbol in objects[todo.pop()].used:
            name = definer.get(symbol)
            if name is None:
                raise ValueError(f"{symbol}: no runtime file defines it")
            if name not in reached:
                reached.add(name)
                todo.append(name)
    return tuple(sorted(reached))


REQUEST = re.compile(r"\bW2M_(?:HOST|FILE)_[A-Z_]+\b")


def candidate_files(f: Family) -> tuple[list[str], list[str], list[str]]:
    """(runtime files, headers, host files) of the family's candidate: the runtime files it reaches, the headers they
    include (api_ids.h aside: it numbers every file's services, and the build refuses a collision), and the Python
    answering their requests (vm_backend.REQUEST_FILES) with the family's own host_files."""
    files = runtime_files(f)
    objects = runtime_objects()
    headers = sorted({h for n in files for h in objects[n].headers} - {"api_ids.h"})
    return list(files), headers, sorted(set(requested_host_files(files)) | set(f.host_files))


def requested_host_files(files: tuple[str, ...]) -> tuple[str, ...]:
    """The Python answering the requests runtime files `files` make (vm_backend.REQUEST_FILES)."""
    from w2m.vm_backend import REQUEST_FILES
    host: set[str] = set()
    for n in files:
        for request in set(REQUEST.findall((GUEST / n).read_text())):
            if request not in REQUEST_FILES:
                raise ValueError(f"{n}: {request} answers from no Python vm_backend.REQUEST_FILES names")
            host.update(REQUEST_FILES[request])
    return tuple(sorted(host))


def candidate_sha256(f: Family, sources: dict[str, str] | None = None) -> str:
    """The candidate is every file a run of the family reaches (candidate_files), each preceded by its path, the
    runtime's files as `sources` has them: an edit to any makes a new candidate (Sprint 057; before, a list kept by
    hand missed heap.c)."""
    files, headers, host = candidate_files(f)
    generated = guest_runtime.generated_headers()
    text = ""
    for n in files:
        text += f"\0tools/guest_runtime/{n}\0" + (sources or {}).get(n, (GUEST / n).read_text())
    for h in headers:
        text += f"\0{h}\0" + (generated[h] if h in generated else (GUEST / h).read_text())
    for path in host:
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
        # this module too: it imports Machine and spawner (Sprint 058: kernel plants changed nothing until it did)
        if other is module or other is None or not (name.startswith("w2m") or other is sys.modules.get(__name__)):
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
    target = p.file or f.guest_file
    text = (GUEST / target).read_text()
    assert text.count(p.old) == 1, f"plant {name}: its target text is not in {target} exactly once"
    return {target: text.replace(p.old, p.new)}


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
            from w2m.guest_runtime import BuildError
            try:
                out, stop, _ = run_in_vm(r.binary, sources, f)
            except BuildError as e:                       # a candidate that does not build is rejected, on the record
                out, stop = b"", f"the guest runtime did not build: {str(e)[:400]}"
            cmp = compare_exact_rows(c, derived_output(f, out), f"cmp-{cand}-{run}", f"{r.execution}-1",
                                     f"managed_vm-{cand}-{run}")
            expected = f.expected_stop is not None and stop.startswith(f.expected_stop)
            if stop != "ExitProcess" and not expected and (cmp.verdict == "conforms" or not out):
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


def plant_run(f: Family, name: str) -> tuple[bool, str]:
    """Plant `name` and run the family's first binary: (whether the unequal rows are those it names, and every process
    ran the planted runtime; what it found)."""
    from w2m.guest_runtime import BuildError
    p = f.plants[name]
    binary = next(iter(f.runs.values())).binary
    try:
        if p.host is not None:
            with host_patched(p.host, p.old, p.new):
                out, stop, _ = run_in_vm(binary, None, f)
        else:
            out, stop, _ = run_in_vm(binary, planted_sources(f, name), f)
    except BuildError as e:                       # a plant that does not build tests nothing
        return False, f"{name}: the planted runtime did not build: {str(e)[:300]}"
    rows, named = unequal_rows(f, out), named_reach(f, p)
    ok = rows == named and not stop.startswith(FOREIGN)
    verdict = ("NOT judged: " + stop if stop.startswith(FOREIGN) else "as named" if rows == named
               else f"NOT as named: named {named}")
    shown = rows if len(rows) <= 20 else [*rows[:20], f"... {len(rows) - 20} more"]
    return ok, f"{name}: {len(rows)} unequal rows {shown} ({stop}); {len(named)} named; {verdict}"


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
        import fcntl
        with open(a.runs / "family-runs.lock", "w") as lock:   # families run in parallel share the index (Sprint 065:
            fcntl.flock(lock, fcntl.LOCK_EX)                   # one read it half-written and failed)
            done = json.loads(index.read_text()) if index.exists() else {}
            done[f.name] = s
            index.write_text(json.dumps(done, indent=1, sort_keys=True) + "\n")
        print(json.dumps(s, indent=1))
    if a.plants:
        for name in f.plants:
            ok, line = plant_run(f, name)
            failed |= not ok
            print(line)
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
