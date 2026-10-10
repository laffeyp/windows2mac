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
import hashlib
import json
import sys
import tempfile
import time
from collections import Counter
from collections.abc import AsyncIterator, Callable
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


@dataclass(frozen=True)
class Plant:
    old: str              # text of the guest C file, present exactly once
    new: str
    reach: tuple[str, ...]  # the rows it changes, named before it runs


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


FAMILIES: dict[str, Family] = {
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
    "console": Family(
        name="console",
        contract_id="windows.console.gui_start",
        version="0.1.0",
        summary=("A GUI process started without handles (as Steam starts the game) gets NULL standard handles, "
                 "no console, and Windows's answers from GetStdHandle, SetStdHandle, GetFileType, GetConsoleMode, "
                 "GetConsoleCP, WriteConsoleW, FlushFileBuffers, GetStartupInfoW and GetCommandLineW."),
        probe="console", guest_file="console.c",
        runs={
              "38041313080": Run("windows-2025", "b5d610d864cb7a1a7a5c09fd5aec8e2d39291b5831c765f283911c97ec205e5d", "console-1791624304-0", "AMD EPYC 7763 64-Core Processor"),
              "38041315490": Run("windows-2022", "39a6cda432228ad4f72af4aa44f85ca5bfe508397108df9cf3676ba3d3fe8cff", "console-1791624304-1", "INTEL(R) XEON(R) PLATINUM 8573C"),
              "38041317837": Run("windows-2025", "9cafe66a6c848f58271ae1ce9c5152c8daa8c683d8935402830d778d64c7ed41", "console-1791624304-2", "INTEL(R) XEON(R) PLATINUM 8573C"),
              "38041320429": Run("windows-2022", "fd3d5c379120837747c1f35ebfd8540ab1f93308ab20f3c305fc3a760c09aabb", "console-1791624304-3", "AMD EPYC 7763 64-Core Processor"),
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


def run_in_vm(binary: str, sources: dict[str, str] | None = None, f: Family | None = None) -> tuple[bytes, str, float]:
    """A runner's probe binary in the managed VM with a fresh file-system root, started as `f` says;
    (its rows' bytes: standard output, or f.output_file; the stop; seconds)."""
    from w2m.vm_backend import VM
    t0 = time.monotonic()
    with tempfile.TemporaryDirectory(prefix="w2m-family-") as d:
        files = FileBridge(Path(d))
        line = None if f is None or not f.arguments else f'"C:\\w2m\\app\\probe.exe" {f.arguments}'
        vm = VM(load((ROOT / "store" / "binaries" / f"{binary}.exe").read_bytes()), files=files,
                runtime_sources=sources, image_name="probe.exe",      # the runner ran it as probe.exe
                gui=f is not None and f.gui, command_line=line)
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
    text = (sources or {}).get(f.guest_file, (GUEST / f.guest_file).read_text())
    sha = "sha256:" + hashlib.sha256(text.encode()).hexdigest()
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
    if a.run:
        s = item_run(f, a.runs)
        index = a.runs / "family-runs.json"
        done = json.loads(index.read_text()) if index.exists() else {}
        done[f.name] = s
        index.write_text(json.dumps(done, indent=1, sort_keys=True) + "\n")
        print(json.dumps(s, indent=1))
    if a.plants:
        binary = next(iter(f.runs.values())).binary
        for name, p in f.plants.items():
            out, stop, _ = run_in_vm(binary, planted_sources(f, name), f)
            rows = unequal_rows(f, out)
            print(f"{name}: unequal {rows} ({stop}); named reach {list(p.reach)}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
