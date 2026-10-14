# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""vm.py — the managed-VM back end: lifted x86-64 operations compiled to AArch64 and run inside a
Hypervisor.framework VM (Sprint 030; profile_kind `managed_vm`).

Process split. `tools/vm_monitor/vm_monitor.c` maps one shared memory file into the guest and runs the vCPU
on command; everything with policy lives here: page tables, code generation, dispatch, kernel32.

Guest address space (stage 1, 4 KB granule, 48-bit VA, EL1, MMU on). Every x86 region from
`loader.load` (the image at its preferred base, the stack) appears at its own x86 virtual
address, so an x86 load or store becomes one AArch64 load or store at the same address. The
monitor (counters, vectors, dispatcher, dispatch table, code cache, guest runtime image and its
stack) sits at MONITOR_VA. Loader thunks are not mapped: the dispatch table sends each one to an
AArch64 bridge stub that calls the guest runtime's C service for that import (Sprint 035,
tools/guest_runtime/). The thread's TEB, the PEB, its state block, the host transport page and the
service ring sit at PROCESS_VA, at the offsets of specs/guest_state.json.

Register map. x86 GPR i is x_i (i = 0..15). CF ZF SF OF PF are x19..x23 as 0/1; x24 holds one
bit per flag (C=1 Z=2 S=4 O=8 P=16) set while that flag is undefined; x28 points at the monitor
data page; x16, x17 are scratch (x17 carries the x86 target into the dispatcher); temporaries of
one lifted instruction use x18, x25-x27, x29, x30.

Exits (HVC immediate): 1 dispatch miss (x17 = x86 target), 2 EL1 exception taken through the
vector table, 4 step limit, 8 a host request from the guest runtime (the transport page holds
it), 9 an import with no guest service (x17 = its thunk), 0x40|bit an undefined flag read. Each
mirrors a stop of runtime.py.
"""

from __future__ import annotations

import json
import mmap
import os
import random
import struct
import subprocess
import sys
import tempfile
import time
from bisect import bisect_right
from collections import Counter
from collections.abc import Callable, Generator
from dataclasses import dataclass
from functools import partial
from pathlib import Path
from typing import Any, Literal

from w2m import a64_encoder as a64
from w2m import guest_runtime
from w2m.kernel import KIND_ERROR_MODE, KIND_STANDARD
from w2m.file_bridge import (FILE_DUP, FILE_FSTAT, FILE_LIST, FILE_LOOKUP, FILE_MKDIR, FILE_RMDIR, FILE_STAT, FILE_UNLINK, BridgeRequestError,
                             FileBridge)
from w2m.modules import APP_DIRECTORY as APP_PATH, EXPORTS as EXPORTS_SPEC, Modules
from w2m.virtual_memory import MEM_COMMIT, MEM_RESERVE, PAGE_EXECUTE_READWRITE, PAGE_GUARD, STACK_GUARD_PAGES, VirtualMemory
from w2m.kernel32 import M64, HandlerFailed, Thread, Unimplemented, monotonic_count
from w2m.lift import RAX, RCX, RDI, RDX, RSI, Lifted, LiftError, Op, lift
from w2m.loader import (DLL_INIT_FAILED, THUNK_BASE, TLS_SLOTS, GuestFault, Image, LoadError, Loaded, Memory, Region,
                        map_image)
from w2m.pe import parse as parse_pe
from w2m.x86dec import DecodeError
from w2m.runtime import Result
from w2m import guest_stubs
from w2m.scheduler import (PER_THREAD_MONITOR, SCRATCH_OFF, TIME_SLICE, WAIT_KEY_OFF, GuestThread, attach_stub,
                            detach_stub, write_context)

ROOT = Path(__file__).resolve().parents[2]
MONITOR_SOURCE = ROOT / "tools" / "vm_monitor" / "vm_monitor.c"
MONITOR_ENTITLEMENTS = ROOT / "tools" / "vm_monitor" / "entitlements.plist"
MONITOR_BINARY = ROOT / "build" / "vm_monitor"

IPA_BASE = 0x1000_0000
MONITOR_VA = 0x7F00_0000_0000
PAGE = 0x1000
BLOCK = 0x20_0000                   # a level-2 block (2 MB) in the 4 KB granule (Arm ARM D8.3)
GIGA_BLOCK = 0x4000_0000            # a level-1 block (1 GB) in the same granule
HEAP_VA = 0x0000_0002_0000_0000     # the process heap's arena (heap.c), above the image and the stack
HEAP_SIZE = 0x1000_0000             # 256 MB, sparse in the VM's memory file
# monitor layout (offsets into the shared file and from MONITOR_VA)
MONITOR_DATA = 0x00000              # [0] instructions executed, [8] limit, [16] dispatch table VA
AF_OPERANDS = 0x20                  # MONITOR_DATA + this: the last ADD/SUB/INC/DEC's a, b, r (lazy AF)
# PF is lazy too (specs/guest_state.json 0.3.0, recentering step 5): every instruction that defines PF stores its result
# at AF_OPERANDS + 16, and PF is set when that value's low byte has an even number of ones. A value restored from flags
# stores 0 there for PF set and 1 for PF clear, with AF<<4 and 0 as the operands.
PF_RESULT = AF_OPERANDS + 16
DF_BYTE = 0x38                      # MONITOR_DATA + this: DF (0 or 1)
RECON_COUNT = 0x40                  # MONITOR_DATA + this: entries in the reconstruction table
MXCSR_OFF = 0x50                    # MONITOR_DATA + this: MXCSR's control bits and the flags not yet in FPSR
STATE_GS_BASE = 0xb0               # the state block's gs_base (specs/guest_state.json state_block; runtime.h)
START_MODE = 0x68                   # MONITOR_DATA + this: 0 console start (redirected stdout), 1 GUI start (console.c)
FOREGROUND_MODE = 0x80              # MONITOR_DATA + this: 0 the process takes the foreground, 1 another holds it (windows.c)
PROGRESS_SECONDS = 30               # a run prints a progress line this often (wall clock) on stderr
CLOCK_MODE = 0x60                   # MONITOR_DATA + this: 0 the guest counter (CNTVCT_EL0), 1 the host's clock,
                                    # 2 the instructions run at CLOCK_RATE a second plus CLOCK_WARP (process.c)
CLOCK_RATE, CLOCK_WARP, CLOCK_EPOCH = 0x90, 0x98, 0xA0   # MONITOR_DATA + these (runtime.h; Sprint 065 item 10)
YMM_HI_OFF = 0x400                  # MONITOR_DATA + this: YMM0-15 bits 255:128, 16 bytes each (rules/_vex.py; Sprint 049)
FCW_OFF = 0x70                      # MONITOR_DATA + this: the x87 control word (FNSTCW, run by the model; Sprint 049)
X87_SW_OFF, X87_ST_OFF = 158, 768    # the state block's x87_status_word (x87_tag_abridged after it) and st0-st7
                                     # (specs/guest_state.json; Sprint 065: the model's x87 steps keep them)
SPILL = 0x78                        # MONITOR_DATA + this: one register a lowering borrows (CMPXCHG16B's status)
CODE_WRITE_SPILL = 0x88             # MONITOR_DATA + this: x17 while the exception entry asks about a write to code
PAUSE_COUNT = 0xA8                  # MONITOR_DATA + this: PAUSE instructions run (Sprint 068 item J, the spin census)
SAVE_AREA = 0x100                   # MONITOR_DATA + this: x0-x30, then ESR, FAR, ELR at +0x100 (exception entry),
                                    # a software exception's code and RIP at +0x118, +0x120, and Q0-Q15 at +0x200
SAVE_VECTORS = 0x200
NATIVE_OPERANDS = 0x300             # MONITOR_DATA + this: two 16-byte operands for a native helper call (PCMPISTRI)
VECTORS = 0x01000               # vector table, 16 slots of 0x80
DISPATCHER = 0x01800
EXCEPTION_ENTRY = 0x01C00           # EL1 synchronous-exception path into the guest runtime
EXCEPTION_ENTRY_END = 0x02000
TRANSLATION_TABLES = 0x12000            # stage-1 translation tables
TRANSLATION_TABLES_SIZE = 0x40000
# Sprint 047: UnityPlayer filled the 4 MB code cache's 65,536-entry reconstruction table at 1,457,522 instructions.
# The areas are sparse in the memory file; the cache stays below the runtime so every branch between them is in
# B's and BL's +-128 MB.
CODE_CACHE = 0x60000
CODE_CACHE_SIZE = 0x400_0000          # 64 MB
RUNTIME_IMAGE = CODE_CACHE + CODE_CACHE_SIZE   # guest runtime (tools/guest_runtime), linked at MONITOR_VA + this
RUNTIME_IMAGE_SIZE = 0x100000             # 1 MB: the system message table takes 430 KB (Sprint 049)
RUNTIME_STACK = RUNTIME_IMAGE + RUNTIME_IMAGE_SIZE   # the runtime's AArch64 stack (SP set by each bridge)
RUNTIME_STACK_SIZE = 0x10000
RECON_TABLE = RUNTIME_STACK + RUNTIME_STACK_SIZE     # reconstruction table: (host code VA, x86 address) pairs, sorted
RECON_ENTRIES = 0x40_0000             # 4 Mi entries, 64 MB
# The dispatch table: (x86 address, code VA) pairs indexed by the address's low DISPATCH_BITS bits. Sprint 064: with
# 4,096 entries the game's 62,000 blocks evicted each other, and 8,474,510 of a billion-instruction frontier run's
# 8,747,587 exits were misses back to the host; 2^20 entries (16 MB, sparse) hold them.
DISPATCH_BITS = 20
DISPATCH_ENTRIES = 1 << DISPATCH_BITS
DISPATCH_TABLE = RECON_TABLE + 16 * RECON_ENTRIES


def dispatch_slot(target: int) -> int:
    """The dispatcher's index for an x86 address: its low DISPATCH_BITS bits folded with the next two DISPATCH_BITS.
    Sprint 064: the game's modules sit at 1 MB-aligned bases (0x140000000, 0x180000000, 0x1180000000, the thunks at
    0x7FFE00000000), so code at one offset in two modules shared a slot when only the low bits indexed it."""
    return (target ^ target >> DISPATCH_BITS ^ target >> 2 * DISPATCH_BITS) & (DISPATCH_ENTRIES - 1)
# The tables that split a block descriptor where translated code makes 4 KB pages read-only (Sprint 065 item 9e):
# frontier 643a6283510c-1791841443768764000 filled the 64 of TRANSLATION_TABLES at 50.5 billion instructions
SPLIT_TABLES = DISPATCH_TABLE + 16 * DISPATCH_ENTRIES
SPLIT_TABLES_SIZE = 0x40_0000             # 1,024 tables, sparse
# Each thread's Q0-Q15, FPCR and FPSR while another runs (Sprint 068 item B): the monitor's `vswap` writes and reads
# them, so a switch makes no `vget` or `vset` (21 µs each over the pipe; 205,614 switches in a 400-million-instruction
# frontier). In the padding below MONITOR_SIZE's 2 MB rounding.
VECTOR_SLOTS = SPLIT_TABLES + SPLIT_TABLES_SIZE
VECTOR_SLOT = 0x120                       # 16 registers of 16 bytes, FPCR and FPSR of 8 (vm_monitor.c vswap)
VECTOR_SLOT_COUNT = 1024
# The channel (Sprint 068 item E): one page where the host and the monitor exchange a run's registers and its exit,
# each side waking the other on a sequence word (vm_monitor.c `channel`), in place of the text pipe's `set` and `run`
CHANNEL = VECTOR_SLOTS + VECTOR_SLOT * VECTOR_SLOT_COUNT
CH_CMD, CH_EXIT, CH_OP, CH_MS, CH_SAVE, CH_LOAD, CH_PC, CH_CPSR, CH_X = 0, 4, 8, 16, 24, 32, 40, 48, 56
CH_KIND, CH_SYN, CH_EPC, CH_ECPSR, CH_ESR, CH_FAR, CH_ELR, CH_EX, CH_STATUS, CH_VEC = (
    320, 328, 336, 344, 352, 360, 368, 376, 640, 1024)
OP_RUN, OP_VGET, OP_VSET, OP_LEAVE, OP_SWAP = 1, 2, 3, 4, 5
BLOCK_CENSUS = False         # recentering step 3: each block counts its own executions (arm64 words per x86 instruction)
FLAGS_FROM_NZCV = True       # recentering step 5: ADD SUB INC DEC set CF ZF SF OF through NZCV; False: the bitwise way
TRANSPORT = "channel"        # "pipe": every exit's registers as text over the monitor's pipe, as before Sprint 068
MONITOR_SIZE = (SPLIT_TABLES + SPLIT_TABLES_SIZE + BLOCK - 1) & ~(BLOCK - 1)   # 2 MB multiple: mapped in blocks
assert CHANNEL + PAGE <= MONITOR_SIZE
# process area: Windows-visible state at specs/guest_state.json offsets (Sprint 034)
PROCESS_VA = 0x7FFE_0000_0000
TEB_OFF, PEB_OFF, STATE_OFF, TRANSPORT_OFF, RING_OFF = 0x0000, 0x2000, 0x3000, 0x4000, 0x5000
RING_SIZE = 0x40000                 # 10,922 records; Sprint 042 probe makes 1,275 calls (shared-memory rings: Sprint 044)
RING_APIS = 512                     # per-API call counters after the ring's count and capacity (runtime.h)
RING_HEADER = 16 + 8 * RING_APIS
EXCEPTION_OFF = RING_OFF + RING_SIZE             # the last unhandled exception: record and CONTEXT (runtime.h)
EXCEPTION_MAGIC = 0x7832774E4F495445
KERNEL32_IMAGE_OFF = EXCEPTION_OFF + 0x1000     # synthetic module images, one page each (modules.py)
KERNELBASE_IMAGE_OFF = KERNEL32_IMAGE_OFF + 0x1000
TLS_EXPANSION_OFF = KERNELBASE_IMAGE_OFF + 0x1000   # TEB.TlsExpansionSlots' array: 1024 values (thread_storage.c)
COMMAND_LINE_OFF = TLS_EXPANSION_OFF + 0x2000    # the command line, UTF-16 with a NUL (console.c)
ENVIRONMENT_OFF = COMMAND_LINE_OFF + 0x1000      # the environment block, UTF-16 entries and a NUL (environment.c)
ENVIRONMENT_SIZE = 0x8000          # two halves: a change writes the other (Sprint 059: Windows moves the block)
NTDLL_IMAGE_OFF = ENVIRONMENT_OFF + ENVIRONMENT_SIZE   # synthetic ntdll image, one page (modules.py; Sprint 046)
LOADER_DATA_OFF = NTDLL_IMAGE_OFF + 0x1000     # PEB.Ldr: PEB_LDR_DATA, the LDR_DATA_TABLE_ENTRYs, their names
LOADER_DATA_SIZE = 0x4000
SYSTEM_IMAGES_OFF = LOADER_DATA_OFF + LOADER_DATA_SIZE   # one page per other system DLL imported (modules.py; Sprint 048)
SYSTEM_IMAGES = 32
LDR_ENTRY_SIZE = 0x120
INHERIT_OFF = SYSTEM_IMAGES_OFF + SYSTEM_IMAGES * 0x1000   # a child's inherited handles (sync_objects.c; Sprint 055)
FAULT_LOG_OFF = INHERIT_OFF + 0x1000   # faults dispatched: a count, then the first 64 as (code, RIP, info 0, info 1)
CURRENT_DIRECTORY_OFF = FAULT_LOG_OFF + 0x1000   # ProcessParameters.CurrentDirectory's two buffers (paths.c)
TRACE_OFF = CURRENT_DIRECTORY_OFF + 0x1000   # the game's calls as w2m answers them (call_trace.c; Sprint 060)
TRACE_SIZE = 0x800000              # 8 MB: per-thread counters (3 MB), then about 43,000 records
TRACE_RECORDS_OFF, TRACE_RECORD, TRACE_THUNKS, TRACE_OWNER_OFF = 0x34D000, 120, 16384, 0x80
TRACE_DYNAMIC = 2                                # call_trace.c W2M_TRACE_DYNAMIC: the GetProcAddress label
TRACE_WIDE_OFF, TRACE_STRINGS_OFF, TRACE_STRING = 0x4300, 0x308300, 136
# call_trace.h's STRING_ARGS this trace keeps: bit i, argument i is a UTF-16 string (Sprint 064: thread names)
TRACE_WIDE_ARGS = {"SetThreadDescription": 1 << 1}
TRACE_CAPACITY = (TRACE_SIZE - TRACE_RECORDS_OFF) // TRACE_RECORD
TRACED_IMAGES = ("UnityPlayer.dll", "mono-2.0-bdwgc.dll")     # the images tools/jit_capture/call_trace.h hooks
# call_trace.h's UNTRACED: functions that read or unwind their caller's context or never return keep a plain jump
UNTRACED = frozenset({"RaiseException", "RtlUnwindEx", "RtlUnwind", "RtlCaptureContext", "RtlVirtualUnwind",
                      "RtlLookupFunctionEntry", "RtlRestoreContext", "RtlCaptureStackBackTrace",
                      "CaptureStackBackTrace", "ExitProcess", "ExitThread", "FreeLibraryAndExitThread",
                      "TerminateProcess", "GetProcAddress", "SetUnhandledExceptionFilter", "UnhandledExceptionFilter"})
PROCESS_SIZE = TRACE_OFF + TRACE_SIZE
# PEB.ProcessParameters (RTL_USER_PROCESS_PARAMETERS, Sprint 059), as contracts/windows.process.parameters records
# Windows filling it: in the process heap's arena, its strings inside it, the current directory and the environment
# outside. Offsets: phnt's ntrtl.h (ImagePathName 0x60 and CommandLine 0x70 as winternl.h has them).
PARAMETERS_VA = HEAP_VA + 0x1000    # heap.c's first block follows its Length
PARAMETERS_FIXED = 0x448            # the structure before its strings (Windows's is longer by build; the rows keep
                                    # MaximumLength == Length only)
PARAMETERS_FLAGS = 0x4001           # every start the rows hold
CURRENT_DIRECTORY_MAX = 0x208       # CurrentDirectory.DosPath.MaximumLength: MAX_PATH units and a NUL, in bytes
CURRENT_DIRECTORY_HANDLE = 0x10     # CurrentDirectory.Handle: non-null (rows *.current_directory); below every
                                    # handle range w2m gives out (stdout 0x74, files 0x1000, objects 0x2000)
DESKTOP = "winsta0\\default"
PROCESSORS = 4                      # the runners' processors (runtime.h W2M_PROCESSORS; Sprint 061)
START_GROUP = 1000                  # ProcessGroupId of a process w2m starts: neither 0 nor its own id (rows *.header);
                                    # a child keeps its parent's
STDOUT_HANDLE = 0x74
# The services the guest runtime defines (Sprint 049: derived from its C files, guest_runtime.service_table):
# kernel32 import name -> (C symbol, sets RAX), and for each other system DLL (upper case) the same
_SERVICE_TABLE = guest_runtime.service_table()
SERVICES = _SERVICE_TABLE.pop("KERNEL32")
DLL_SERVICES = _SERVICE_TABLE
HOST_WRITE, HOST_EXIT, HOST_CLOCK, HOST_UNIMPLEMENTED, HOST_RUNTIME_FAULT = 1, 2, 3, 4, 5
HOST_FILETIME = 14                  # the wall clock as FILETIME (6-13: the file primitives)
HOST_MODULE, HOST_PROC, HOST_MODULE_PATH, HOST_MODULE_AT = 15, 16, 17, 18   # src/w2m/modules.py
HOST_VIRTUAL_ALLOC, HOST_VIRTUAL_FREE, HOST_VIRTUAL_PROTECT, HOST_VIRTUAL_QUERY = 19, 20, 21, 22   # virtual_memory.py
INIT_STUB_SLOT = 0x80                # where module.c keeps LoadLibrary's return address in a DllMain stub (Sprint 049)
HOST_RANDOM = 23                    # a0 bytes of the host's CSPRNG (os.urandom) at guest address a1 (bcrypt.c)
HOST_MAP_VIEW = 29                  # a view of a0 bytes of file descriptor a1 with protection a2 (file.c; Sprint 049;
                                    # 24-28 are file primitives)
# Exports a program imports by ordinal (Sprint 052): OLEAUT32's, as Wine's oleaut32.spec numbers them
ORDINAL_NAMES = {"OLEAUT32.DLL": {"#2": "SysAllocString", "#6": "SysFreeString", "#7": "SysStringLen",
                                  "#8": "VariantInit", "#9": "VariantClear"},
                 "WS2_32.DLL": {"#111": "WSAGetLastError", "#115": "WSAStartup", "#116": "WSACleanup"}}   # ws2_32.spec
# Each COM class's vtable in the runtime image: (its DLL, its methods' name prefix, the methods in vtable order).
# A slot whose method has no service stops the run by its name when called (hvc #9).
_DXGI_OBJECT = ("QueryInterface", "AddRef", "Release", "SetPrivateData", "SetPrivateDataInterface", "GetPrivateData",
                "GetParent")
COM_VTABLES = {"w2m_enumerator_vtable": ("MMDEVAPI.dll", "W2mEnumerator", (
                   "QueryInterface", "AddRef", "Release", "EnumAudioEndpoints", "GetDefaultAudioEndpoint", "GetDevice",
                   "RegisterEndpointNotificationCallback", "UnregisterEndpointNotificationCallback")),
               "w2m_collection_vtable": ("MMDEVAPI.dll", "W2mCollection",
                                         ("QueryInterface", "AddRef", "Release", "GetCount", "Item")),
               # dxgi.h (Sprint 063): IDXGIFactory, IDXGIAdapter, IDXGIOutput
               "w2m_dxgi_factory_vtable": ("dxgi.dll", "W2mFactory", _DXGI_OBJECT + (
                   "EnumAdapters", "MakeWindowAssociation", "GetWindowAssociation", "CreateSwapChain",
                   "CreateSoftwareAdapter")),
               "w2m_dxgi_adapter_vtable": ("dxgi.dll", "W2mAdapter", _DXGI_OBJECT + (
                   "EnumOutputs", "GetDesc", "CheckInterfaceSupport")),
               "w2m_dxgi_output_vtable": ("dxgi.dll", "W2mOutput", _DXGI_OBJECT + (
                   "GetDesc", "GetDisplayModeList", "FindClosestMatchingMode", "WaitForVBlank", "TakeOwnership",
                   "ReleaseOwnership", "GetGammaControlCapabilities", "SetGammaControl", "GetGammaControl",
                   "SetDisplaySurface", "GetDisplaySurfaceData", "GetFrameStatistics"))}
HOST_THREAD = 30                    # threads.c's requests: a0 the W2M_THREAD_* sub-operation (scheduler.py; Sprint 051)
HOST_KERNEL = 33                    # kernel.py's shared objects (Sprint 055): a0 the sub-operation
HOST_EXCEPTION = 31                 # exceptions.c's requests (Sprint 053): a0 1, the x86 stubs' page; 2, XMM0-15 to a1
EXCEPTION_STUBS, EXCEPTION_CAPTURE_XMM = 1, 2
# The Python that answers each request the runtime's C names (Sprint 057): a family's candidate holds the files its
# runtime files' requests reach (scripts/family_run.py); _host_request receives every one of them
_HERE, _FILES, _VIRTUAL = "src/w2m/vm_backend.py", "src/w2m/file_bridge.py", "src/w2m/virtual_memory.py"
REQUEST_FILES: dict[str, tuple[str, ...]] = {
    **{f"W2M_HOST_{n}": (_HERE,) for n in ("WRITE", "EXIT", "CLOCK", "UNIMPLEMENTED", "RUNTIME_FAULT", "FILETIME",
                                            "RANDOM")},
    **{f"W2M_HOST_{n}": (_HERE, "src/w2m/modules.py") for n in ("MODULE", "PROC", "MODULE_PATH", "MODULE_AT")},
    **{f"W2M_HOST_VIRTUAL_{n}": (_HERE, _VIRTUAL) for n in ("ALLOC", "FREE", "PROTECT", "QUERY")},
    "W2M_HOST_MAP_VIEW": (_HERE, _VIRTUAL, _FILES),
    "W2M_HOST_THREAD": (_HERE, "src/w2m/scheduler.py"),
    "W2M_HOST_KERNEL": (_HERE, "src/w2m/kernel.py"),
    "W2M_HOST_EXCEPTION": (_HERE, "src/w2m/guest_stubs.py"),
    **{f"W2M_FILE_{n}": (_HERE, _FILES) for n in ("LOOKUP", "OPEN", "CLOSE", "READ", "WRITE", "SIZE", "TRUNCATE",
                                                   "UNLINK", "STAT", "MKDIR", "RMDIR", "FSTAT", "DUP", "LIST")},
}
THREAD_CREATE, THREAD_EXIT_STUB, THREAD_END, THREAD_SUSPEND, THREAD_RESUME, THREAD_CONTEXT, THREAD_EXIT_CODE, \
    THREAD_SIGNAL, THREAD_STACK_GROW = range(1, 10)
VIRTUAL_VA = 0x0000_0010_0000_0000  # the VirtualAlloc arena (virtual_memory.py), sparse in the VM's memory file
VIRTUAL_SIZE = 0x80_0000_0000       # 512 GB of address space (Sprint 047: UnityPlayer reserves 512 MB at a time), mapped
                                    # in 1 GB blocks; the file is sparse, so only pages the guest touches take host memory
FILETIME_UNIX_EPOCH = 116444736000000000   # 1970-01-01 in 100 ns units since 1601-01-01
# The block w2m starts a game with (run_frontier.py; Sprint 048): the profile and its temporary directory, which
# GetTempPathW reads (environment.c), in the block's sorted order. The reference game run's (Sprint 061): the
# runners' user, and TMP and TEMP where windows-game.yml sets them; each directory exists on drive C:
# (file_bridge.py PROFILE_FOLDERS).
USER_ENVIRONMENT = ("TEMP=C:\\Users\\w2m\\AppData\\Local\\Temp", "TMP=C:\\Users\\w2m\\AppData\\Local\\Temp",
                    "USERPROFILE=C:\\Users\\runneradmin")
TLS_TAKEN = 16                      # the TLS index besides 0 a fresh Windows process holds: set from the rows
PROCESS_ID = 4096                   # TEB.ClientId.UniqueProcess: Windows ids are nonzero multiples of 4 (Sprint 044 rows)
Discovery = Literal["static", "runtime_transfer"]           # vocabulary 0.5 enum `discovery`, the values used

SYS_REG_ID = {"SCTLR_EL1": 0xC080, "CPACR_EL1": 0xC082, "TTBR0_EL1": 0xC100, "TCR_EL1": 0xC102,
              "MAIR_EL1": 0xC510, "VBAR_EL1": 0xC600}
CPACR_FPEN = 3 << 20                         # FP/SIMD at EL1 and EL0 not trapped (Arm ARM D19 CPACR_EL1.FPEN)
PAGE_ATTRIBUTES = (3 << 8) | (1 << 10)                 # AttrIndx 0 (MAIR byte 0 = 0xFF, Normal WB), ISH, AF
SCTLR = 0x30D01805                          # M, C, I; alignment checks off (Sprint 012 value)
TCR = 16 | (1 << 8) | (1 << 10) | (3 << 12) | (1 << 23) | (2 << 32)   # T0SZ 16, WB, ISH, EPD1, IPS 40 (vm_monitor.c)
CPSR_EL1H = 0x3C5

TEMP_REGS = (18, 25, 26, 27, 29, 30)
REG_CF, REG_ZF, REG_SF, REG_OF, REG_PF = 19, 20, 21, 22, 23
REG_UNDEFINED_FLAGS = 24
# x23 (PF's until PF went lazy) holds the step budget left (recentering step 5, review item M): each block entry is
# SUBS x23, x23, #n and B.MI to its stop, where it loaded, added, stored and compared the count in memory. The host
# keeps the count and the limit in the monitor data page and sets x23 = limit - count before each run, and the count
# from x23 after each exit; a bridge writes the count back before a service runs (the runtime's clock reads it).
# The runtime's C keeps x23 as a callee-saved register of its own, so the meaning holds only outside the runtime image:
# the host reads and sets x23 only when the vCPU stopped outside it, and a bridge or the exception entry writes the
# count before its call into C and sets x23 from memory after it (the C restores the value it found, which a host
# request inside the call may have made stale: 2026-10-14, a thread's slice never fired).
REG_BUDGET = 23
FLAG_BIT = {"C": 0, "Z": 1, "S": 2, "O": 3, "P": 4, "A": 5}
U_CF, U_ZF, U_SF, U_OF, U_PF, U_AF = (1 << FLAG_BIT[f] for f in "CZSOPA")   # bits of REG_UNDEFINED_FLAGS
FLAG_REG = {"C": REG_CF, "Z": REG_ZF, "S": REG_SF, "O": REG_OF, "P": REG_PF}
MAX_BLOCK_INSNS = 64
BLOCKED_KEYS = 32                  # threads.c W2M_BLOCKED_KEYS: the waited-on keys published to the guest
ASYNC_STOP_RULE = "forward"     # "backward" exists only so a test can show the forward rule is what saves the count


IDLE: list[int] = []                       # _switch: no thread of this VM can run (a kernel.Machine runs another); compared by identity


class StopRun(Exception):
    """Raised by an on_dispatch callback to end a run cleanly."""


class CompileError(RuntimeError):
    """A lifted instruction the back end cannot compile (for example, too many live temporaries)."""


def build_monitor() -> Path:
    """Build and ad-hoc sign build/vm_monitor with the hypervisor entitlement when its source is newer."""
    if not MONITOR_BINARY.exists() or MONITOR_BINARY.stat().st_mtime < MONITOR_SOURCE.stat().st_mtime:
        MONITOR_BINARY.parent.mkdir(exist_ok=True)
        subprocess.run(["clang", "-O1", "-Wall", "-Werror", "-o", str(MONITOR_BINARY), str(MONITOR_SOURCE),
                        "-framework", "Hypervisor"], check=True, capture_output=True)
        subprocess.run(["codesign", "-s", "-", "-f", "--entitlements", str(MONITOR_ENTITLEMENTS), str(MONITOR_BINARY)],
                       check=True, capture_output=True)
    return MONITOR_BINARY


# ---------------------------------------------------------------------------------------------
# assembler with labels

class Assembler:
    def __init__(self, base_va: int) -> None:
        self.base = base_va
        self.words: list[int] = []
        self.fix: list[tuple[int, Callable[[int], int], object]] = []
        self.labels: dict[int, int] = {}
        self._n = 0

    def here(self) -> int:
        return self.base + 4 * len(self.words)

    def emit(self, *ws: int | list[int]) -> None:
        for w in ws:
            self.words.extend(w if isinstance(w, list) else [w])

    def label(self) -> int:
        self._n += 1
        return -self._n

    def bind(self, lab: int) -> None:
        self.labels[lab] = self.here()

    def branch(self, make: Callable[[int], int], target: int) -> None:
        """`target` is a VA (>= 0) or a label from label()."""
        self.fix.append((len(self.words), make, target))
        self.words.append(0)

    def finish(self) -> list[int]:
        for i, make, target in self.fix:
            assert isinstance(target, int)
            va = self.labels[target] if target < 0 else target
            self.words[i] = make(va - (self.base + 4 * i))
        return self.words


# ---------------------------------------------------------------------------------------------
# compiler: one lifted instruction -> AArch64

def _mask(a: Assembler, r: int, bits: int) -> None:
    if bits == 32:
        a.emit(a64.mov32(r, r))
    elif bits < 64:
        a.emit(a64.ubfx(r, r, 0, bits))


def _mark_defined(a: Assembler, bits: int) -> None:
    a.emit(a64.movz(16, bits), a64.bic(REG_UNDEFINED_FLAGS, REG_UNDEFINED_FLAGS, 16))


def _mark_undefined(a: Assembler, bits: int) -> None:
    a.emit(a64.movz(16, bits), a64.orr(REG_UNDEFINED_FLAGS, REG_UNDEFINED_FLAGS, 16))


def _nzcv(a: Assembler, add: bool, ra: int, rb: int, bits: int) -> None:
    """NZCV as the add or subtract of `bits`-wide ra and rb sets it: Z and N as x86's ZF and SF, V as OF, C as ADD's CF
    and as SUB's borrow inverted (recentering step 5: one instruction where OF, CF, ZF and SF took ten). An 8- or
    16-bit operation runs in the top of a 32-bit one so the carry and the sign land where x86 has them. Uses x16 for
    the narrow forms. Encodings checked against Apple clang's assembler."""
    base = 0x2B000000 if add else 0x6B000000                 # ADDS / SUBS, 32-bit, Wd = WZR
    if bits == 64:
        a.emit((a64.adds if add else a64.subs)(a64.ZR, ra, rb))
    elif bits == 32:
        a.emit(base | rb << 16 | ra << 5 | 31)
    else:
        s = 32 - bits
        a.emit(0x53000000 | ((32 - s) % 32) << 16 | (31 - s) << 10 | ra << 5 | 16,      # LSL W16, Wa, #s
               base | rb << 16 | s << 10 | 16 << 5 | 31)                               # with Wb, LSL #s


def _adds_subs(add: bool, rd: int, rn: int, rm: int, bits: int) -> int:
    """ADDS or SUBS Rd, Rn, Rm in 32 or 64 bits (the 32-bit form clears Rd's top half)."""
    w = (a64.adds if add else a64.subs)(rd, rn, rm)
    return w if bits == 64 else w & ~(1 << 31)


def count_from_budget() -> list[int]:
    """Monitor data word 0 (the count) = word 8 (the limit) - x23, before a call into the runtime's C (uses x17)."""
    return [a64.ldr(8, 17, 28, 8), a64.sub(17, 17, REG_BUDGET), a64.str_(8, 17, 28, 0)]


def budget_from_count() -> list[int]:
    """x23 = the limit - the count, after a call into the runtime's C (uses x16 and x17)."""
    return [a64.ldp(16, 17, 28, 0), a64.sub(REG_BUDGET, 17, 16)]


def _set_pf(a: Assembler, r: int) -> None:
    """PF, lazily: the result's parity is read from PF_RESULT when PF is; one store where the parity took seven."""
    a.emit(a64.str_(8, r, 28, PF_RESULT))


def parity_even(v: int) -> bool:
    """PF of a result: its low byte holds an even number of ones (SDM Vol 1 3.4.3.1)."""
    return bin(v & 0xFF).count("1") % 2 == 0


def _pf_into_x16(a: Assembler) -> None:
    """x16 = PF from PF_RESULT: 1 when the low byte holds an even number of ones."""
    a.emit(a64.ldr(8, 16, 28, PF_RESULT), a64.ubfx(16, 16, 0, 8), a64.eor(16, 16, 16, a64.LSR, 4),
           a64.eor(16, 16, 16, a64.LSR, 2), a64.eor(16, 16, 16, a64.LSR, 1), a64.ubfx(16, 16, 0, 1),
           a64.cmpi(16, 0), a64.cset(16, "eq"))


X24_CLEAR = 0xAA1F03F8                    # MOV X24, XZR: every flag defined
X24_KEEP_CF = 0x92400318                  # AND X24, X24, #1: every flag defined but CF, which keeps its state


def _set_zf_sf_pf(a: Assembler, r: int, bits: int) -> None:
    a.emit(a64.cmpi(r, 0), a64.cset(REG_ZF, "eq"), a64.ubfx(REG_SF, r, bits - 1, 1))
    _set_pf(a, r)


def _set_flags(a: Assembler, kind: str, ra: int, rb: int, rr: int, bits: int, nzcv_ready: bool = False,
               live: int = 0x3F, nzcv_out: bool = False) -> None:
    """runtime.Runtime._set_flags, compiled. AF is lazy (specs/guest_state.json 0.2.0): ADD, SUB,
    INC and DEC store a, b and r at MONITOR_DATA + AF_OPERANDS, and AF is bit 4 of a ^ b ^ r when
    the state is read; the instructions that leave AF undefined set its bit in x24.
    `live` holds the flags (U_* bits) a later instruction reads (flag liveness, recentering step 5): ADD, SUB, INC,
    DEC and the logic ops compute only those, and NZCV as well when `nzcv_out` (the next instruction's Jcc branches
    on it, NZCV_CONDITIONS)."""
    if kind == "mul":                                          # SDM MUL: SF ZF AF PF undefined
        a.emit(a64.cmpi(rr, 0), a64.cset(REG_CF, "ne"), a64.mov(REG_OF, REG_CF))
        _mark_defined(a, U_CF | U_OF)
        _mark_undefined(a, U_ZF | U_SF | U_PF | U_AF)
        return
    if kind in ("shr", "shl", "sar", "rol", "ror"):
        _shift_flags(a, kind, ra, rb, rr, bits)
        return
    if kind == "bt":                                           # SDM BT: CF the bit; OF SF AF PF undefined; ZF kept
        a.emit(a64.ubfx(REG_CF, rr, 0, 1))
        _mark_defined(a, U_CF)
        _mark_undefined(a, U_OF | U_SF | U_AF | U_PF)
        return
    if kind == "popcnt":                                       # SDM POPCNT: ZF the source was 0; CF OF SF AF PF 0
        a.emit(a64.ubfx(16, ra, 0, bits) if bits < 64 else a64.mov(16, ra), a64.cmpi(16, 0), a64.cset(REG_ZF, "eq"),
               a64.mov(REG_CF, a64.ZR), a64.mov(REG_OF, a64.ZR), a64.mov(REG_SF, a64.ZR), a64.movz(16, 1),
               a64.stp(a64.ZR, a64.ZR, 28, AF_OPERANDS), a64.str_(8, 16, 28, PF_RESULT),   # PF_RESULT 1: PF clear
               a64.movz(REG_UNDEFINED_FLAGS, 0))
        return
    if kind == "bsf":                                          # SDM BSF/BSR: ZF the source was 0; the rest undefined
        a.emit(a64.ubfx(16, ra, 0, bits) if bits < 64 else a64.mov(16, ra), a64.cmpi(16, 0), a64.cset(REG_ZF, "eq"))
        _mark_defined(a, U_ZF)
        _mark_undefined(a, U_CF | U_OF | U_SF | U_AF | U_PF)
        return
    if kind == "adc":                                          # SDM ADC: carry and overflow from a, b, r
        a.emit(a64.eor(16, ra, rb), a64.eor(17, ra, rr), a64.bic(17, 17, 16),
               a64.ubfx(REG_OF, 17, bits - 1, 1))
        a.emit(a64.and_(16, ra, rb), a64.orr(17, ra, rb), a64.bic(17, 17, rr), a64.orr(16, 16, 17),
               a64.ubfx(REG_CF, 16, bits - 1, 1))
        _mark_defined(a, U_CF | U_ZF | U_SF | U_OF | U_PF)
        a.emit(a64.stp(ra, rb, 28, AF_OPERANDS), a64.str_(8, rr, 28, AF_OPERANDS + 16))
        _mark_defined(a, U_AF)
        _set_zf_sf_pf(a, rr, bits)
        return
    if kind == "sbb":                                          # SDM SBB: borrow and overflow from a, b, r
        a.emit(a64.eor(16, ra, rb), a64.eor(17, ra, rr), a64.and_(17, 17, 16),
               a64.ubfx(REG_OF, 17, bits - 1, 1))
        a.emit(a64.bic(16, rb, ra), a64.eor(17, ra, rb), a64.bic(17, rr, 17), a64.orr(16, 16, 17),
               a64.ubfx(REG_CF, 16, bits - 1, 1))
        _mark_defined(a, U_CF | U_ZF | U_SF | U_OF | U_PF)
        a.emit(a64.stp(ra, rb, 28, AF_OPERANDS), a64.str_(8, rr, 28, AF_OPERANDS + 16))
        _mark_defined(a, U_AF)
        _set_zf_sf_pf(a, rr, bits)
        return
    if kind == "imul":                                         # SDM IMUL: CF=OF on truncation; SF ZF AF PF undefined
        if bits < 64:
            a.emit(a64.sbfx(16, ra, 0, bits), a64.sbfx(17, rb, 0, bits), a64.mul(16, 16, 17),
                   a64.sbfx(17, 16, 0, bits))
        else:
            a.emit(a64.smulh(16, ra, rb), a64.sbfx(17, rr, 63, 1))
        a.emit(a64.cmp(16, 17), a64.cset(REG_CF, "ne"), a64.mov(REG_OF, REG_CF))
        _mark_defined(a, U_CF | U_OF)
        _mark_undefined(a, U_ZF | U_SF | U_PF | U_AF)
        return
    if kind == "andn":                                         # SDM ANDN: SF ZF by the result, CF OF 0, AF PF undefined
        a.emit(a64.mov(REG_CF, a64.ZR), a64.mov(REG_OF, a64.ZR))
        _set_zf_sf_pf(a, rr, bits)
        _mark_defined(a, U_CF | U_ZF | U_SF | U_OF)
        _mark_undefined(a, U_AF | U_PF)
        return
    if FLAGS_FROM_NZCV and kind in ("add", "inc", "sub", "dec"):
        # recentering step 5: NZCV from one ADDS or SUBS gives CF ZF SF OF; INC and DEC keep CF (SDM INC, DEC)
        if not nzcv_ready and (live & (U_CF | U_ZF | U_SF | U_OF) or nzcv_out):   # ADDS or SUBS may have set it
            _nzcv(a, kind in ("add", "inc"), ra, rb, bits)
        for flag, reg, cond in ((U_ZF, REG_ZF, "eq"), (U_SF, REG_SF, "mi"), (U_OF, REG_OF, "vs")):
            if live & flag:
                a.emit(a64.cset(reg, cond))
        if kind in ("add", "sub") and live & U_CF:
            a.emit(a64.cset(REG_CF, "cs" if kind == "add" else "lo"))
        if live:                                               # a dead flag marked defined is never read
            a.emit(X24_CLEAR if kind in ("add", "sub") else X24_KEEP_CF)
        if live & U_AF:
            a.emit(a64.stp(ra, rb, 28, AF_OPERANDS))           # AF's operands; AF reads the result too
        if live & (U_AF | U_PF):
            a.emit(a64.str_(8, rr, 28, PF_RESULT))             # PF's result
        return
    if kind == "logic" and FLAGS_FROM_NZCV:                    # SDM AND/OR/XOR/TEST: CF OF 0, AF undefined
        if live & U_CF:
            a.emit(a64.mov(REG_CF, a64.ZR))
        if live & U_OF:
            a.emit(a64.mov(REG_OF, a64.ZR))
        if live:
            a.emit(a64.movz(REG_UNDEFINED_FLAGS, U_AF))
        if live & U_ZF or nzcv_out:
            a.emit(a64.cmpi(rr, 0))
        if live & U_ZF:
            a.emit(a64.cset(REG_ZF, "eq"))
        if live & U_SF:
            a.emit(a64.ubfx(REG_SF, rr, bits - 1, 1))
        if live & U_PF:
            _set_pf(a, rr)
        return
    if kind == "logic":
        a.emit(a64.mov(REG_CF, a64.ZR), a64.mov(REG_OF, a64.ZR))
        _mark_defined(a, U_CF | U_ZF | U_SF | U_OF | U_PF)
        _mark_undefined(a, U_AF)                               # SDM AND/OR/XOR/TEST: AF undefined
    elif kind in ("add", "inc"):
        a.emit(a64.eor(16, ra, rb), a64.eor(17, ra, rr), a64.bic(17, 17, 16),
               a64.ubfx(REG_OF, 17, bits - 1, 1))
        if kind == "add":
            if bits < 64:
                a.emit(a64.add(16, ra, rb), a64.ubfx(REG_CF, 16, bits, 1))
            else:
                a.emit(a64.adds(a64.ZR, ra, rb), a64.cset(REG_CF, "cs"))
        _mark_defined(a, (U_CF if kind == "add" else 0) | U_ZF | U_SF | U_OF | U_PF)
    elif kind in ("sub", "dec"):
        a.emit(a64.eor(16, ra, rb), a64.eor(17, ra, rr), a64.and_(17, 17, 16),
               a64.ubfx(REG_OF, 17, bits - 1, 1))
        if kind == "sub":
            a.emit(a64.cmp(ra, rb), a64.cset(REG_CF, "lo"))
        _mark_defined(a, (U_CF if kind == "sub" else 0) | U_ZF | U_SF | U_OF | U_PF)
    else:
        raise CompileError(f"flags kind {kind}")
    if kind in ("add", "inc", "sub", "dec"):
        a.emit(a64.stp(ra, rb, 28, AF_OPERANDS), a64.str_(8, rr, 28, AF_OPERANDS + 16))
        _mark_defined(a, U_AF)
    _set_zf_sf_pf(a, rr, bits)


def _shift_flags(a: Assembler, kind: str, ra: int, rb: int, rr: int, bits: int) -> None:
    """runtime.Runtime._set_flags for SHR SHL SAR ROL ROR, compiled (SDM SAL/SAR/SHL/SHR and
    RCL/RCR/ROL/ROR): a zero masked count changes nothing; CF is the last bit out (undefined for SHL
    and SHR when the count >= the size); OF only for a 1-bit count; shifts set ZF SF PF and leave AF
    undefined; rotates touch only CF and OF."""
    done, of_undef, after_of = a.label(), a.label(), a.label()
    a.emit(a64.ubfx(16, rb, 0, 6 if bits == 64 else 5))
    a.branch(lambda off: a64.cbz(16, off), done)
    if kind in ("rol", "ror"):
        if kind == "rol":
            a.emit(a64.ubfx(REG_CF, rr, 0, 1))
        else:
            a.emit(a64.ubfx(REG_CF, rr, bits - 1, 1))
        _mark_defined(a, U_CF)                                 # (uses x16: re-read the count below)
        a.emit(a64.ubfx(16, rb, 0, 6 if bits == 64 else 5), a64.cmpi(16, 1))
        a.branch(lambda off: a64.bcond("ne", off), of_undef)
        a.emit(a64.ubfx(16, rr, bits - 1, 1))
        a.emit(a64.eor(REG_OF, 16, REG_CF) if kind == "rol" else a64.ubfx(17, rr, bits - 2, 1))
        if kind == "ror":
            a.emit(a64.eor(REG_OF, 16, 17))
        _mark_defined(a, U_OF)
        a.branch(a64.b, done)
        a.bind(of_undef)
        _mark_undefined(a, U_OF)
        a.bind(done)
        return
    cf_undef = a.label()
    if kind == "sar":                                          # CF from the sign-extended value, any count
        a.emit(a64.sbfx(17, ra, 0, bits), a64.subi(16, 16, 1), a64.asrv(17, 17, 16), a64.addi(16, 16, 1),
               a64.ubfx(REG_CF, 17, 0, 1))
        _mark_defined(a, U_CF)
    else:
        a.emit(a64.cmpi(16, bits))
        a.branch(lambda off: a64.bcond("hs", off), cf_undef)   # count >= size: CF undefined (SHL, SHR)
        if kind == "shr":
            a.emit(a64.subi(17, 16, 1), a64.lsrv(17, ra, 17))
        else:                                                  # SHL: bit (size - count) of the operand
            a.emit(a64.sub(17, a64.ZR, 16), a64.addi(17, 17, bits), a64.lsrv(17, ra, 17))
        a.emit(a64.ubfx(REG_CF, 17, 0, 1))
        _mark_defined(a, U_CF)
    a.emit(a64.ubfx(16, rb, 0, 6 if bits == 64 else 5), a64.cmpi(16, 1))     # _mark_defined used x16
    a.branch(lambda off: a64.bcond("ne", off), of_undef)
    if kind == "shr":
        a.emit(a64.ubfx(REG_OF, ra, bits - 1, 1))
    elif kind == "shl":
        a.emit(a64.ubfx(17, rr, bits - 1, 1), a64.eor(REG_OF, 17, REG_CF))
    else:                                                      # SAR: OF cleared for a 1-bit shift
        a.emit(a64.mov(REG_OF, a64.ZR))
    _mark_defined(a, U_OF)
    a.branch(a64.b, after_of)
    a.bind(cf_undef)
    _mark_undefined(a, U_CF)
    a.bind(of_undef)
    _mark_undefined(a, U_OF)
    a.bind(after_of)
    _mark_defined(a, U_ZF | U_SF | U_PF)
    _mark_undefined(a, U_AF)
    _set_zf_sf_pf(a, rr, bits)
    a.bind(done)


def _shift_flags_const(a: Assembler, kind: str, ra: int, count: int, rr: int, bits: int) -> None:
    """_shift_flags for a count known when compiling (recentering step 5): the same rules (SDM SAL/SAR/SHL/SHR,
    RCL/RCR/ROL/ROR), decided here instead of tested at run time, and the undefined-flag mask set in one instruction
    where every flag's state is known."""
    m = count & (63 if bits == 64 else 31)
    if m == 0:                                                 # a zero masked count changes nothing
        return
    if kind in ("rol", "ror"):                                 # rotates touch only CF and OF
        a.emit(a64.ubfx(REG_CF, rr, 0, 1) if kind == "rol" else a64.ubfx(REG_CF, rr, bits - 1, 1))
        if m == 1:
            a.emit(a64.ubfx(16, rr, bits - 1, 1))
            a.emit(a64.eor(REG_OF, 16, REG_CF) if kind == "rol" else a64.ubfx(17, rr, bits - 2, 1))
            if kind == "ror":
                a.emit(a64.eor(REG_OF, 16, 17))
            _mark_defined(a, U_CF | U_OF)
        else:
            _mark_defined(a, U_CF)
            _mark_undefined(a, U_OF)
        return
    undefined = U_AF | (0 if m == 1 else U_OF)
    if kind == "sar":                                          # CF from the sign-extended value, any count
        if bits < 64:
            a.emit(a64.sbfx(17, ra, 0, bits), a64.ubfx(REG_CF, 17, min(m - 1, 63), 1))
        else:
            a.emit(a64.ubfx(REG_CF, ra, m - 1, 1))
    elif m >= bits:                                            # SHL, SHR: count >= size, CF undefined
        undefined |= U_CF
    elif kind == "shr":
        a.emit(a64.ubfx(REG_CF, ra, m - 1, 1))
    else:                                                      # SHL: bit (size - count) of the operand
        a.emit(a64.ubfx(REG_CF, ra, bits - m, 1))
    if m == 1:
        if kind == "shr":
            a.emit(a64.ubfx(REG_OF, ra, bits - 1, 1))
        elif kind == "shl":
            a.emit(a64.ubfx(16, rr, bits - 1, 1), a64.eor(REG_OF, 16, REG_CF))
        else:                                                  # SAR: OF cleared for a 1-bit shift
            a.emit(a64.mov(REG_OF, a64.ZR))
    a.emit(a64.movz(REG_UNDEFINED_FLAGS, undefined))
    _set_zf_sf_pf(a, rr, bits)


# A conditional branch right after the instruction that set the flags through NZCV (recentering step 5) branches on
# NZCV itself: the setter defined every flag the condition reads, so neither the undefined check nor the read of the flag
# registers is needed. After SUBS (SUB, CMP) every x86 condition is one Arm condition (CF is the borrow, C clear); after
# ADDS, CF is C, and the carry-or-zero pair has no Arm form; INC and DEC keep CF, so no condition that reads it; after
# the logic ops' CMP r, #0 only Z holds x86's meaning.
NZCV_CONDITIONS = {
    "sub": {"z": "eq", "nz": "ne", "b": "lo", "nb": "hs", "be": "ls", "nbe": "hi", "l": "lt", "nl": "ge", "le": "le",
            "nle": "gt", "s": "mi", "ns": "pl", "o": "vs", "no": "vc"},
    "add": {"z": "eq", "nz": "ne", "b": "cs", "nb": "cc", "l": "lt", "nl": "ge", "le": "le", "nle": "gt", "s": "mi",
            "ns": "pl", "o": "vs", "no": "vc"},
    "inc": {"z": "eq", "nz": "ne", "l": "lt", "nl": "ge", "le": "le", "nle": "gt", "s": "mi", "ns": "pl", "o": "vs",
            "no": "vc"},
    "logic": {"z": "eq", "nz": "ne"},
}
NZCV_CONDITIONS["dec"] = NZCV_CONDITIONS["inc"]
INVERSE = {name: next(n for n, c in a64.COND.items() if c == code ^ 1 and n not in ("cs", "cc"))
           for name, code in a64.COND.items() if name != "al"}


def nzcv_setter(ins: Lifted) -> str | None:
    """The flags kind whose NZCV a conditional branch right after `ins` may branch on, or None."""
    if not FLAGS_FROM_NZCV or not ins.ops or ins.ops[-1][0] != "flags":
        return None
    kind = ins.ops[-1][1]
    return kind if kind in NZCV_CONDITIONS else None


# Flag liveness (recentering step 5, Sprint 068 item L): a flag no later instruction reads is not computed. A block's
# instructions are walked backward from the flags live at its end, which come from its direct successors, each scanned
# forward until every flag is written or read (_flags_live_out). The masks are REG_UNDEFINED_FLAGS's bits. A flag
# written as undefined counts as written: the undefined mask is what a reader then finds.
# An instruction that may stop with an exception (a model step, #UD, DIV's #DE) reads every flag, since the CONTEXT
# shows them. A load or store does not: where a fault interrupts a stretch whose flags are dead, the CONTEXT's EFlags
# hold the flags an earlier instruction left, which only a handler that reads EFlags sees; a handler that resumes there
# runs into the instruction that writes them.
FLAG_LIVENESS = True
ALL_FLAGS = U_CF | U_ZF | U_SF | U_OF | U_PF | U_AF
COND_FLAGS = {"o": U_OF, "b": U_CF, "z": U_ZF, "s": U_SF, "p": U_PF, "be": U_CF | U_ZF, "l": U_SF | U_OF,
              "le": U_ZF | U_SF | U_OF}
FLAGS_WRITTEN = {kind: ALL_FLAGS for kind in ("add", "sub", "adc", "sbb", "logic", "andn", "mul", "imul", "bsf", "popcnt")}
FLAGS_WRITTEN.update(inc=ALL_FLAGS & ~U_CF, dec=ALL_FLAGS & ~U_CF, bt=ALL_FLAGS & ~U_ZF)   # SDM INC, DEC, BT
FLAGS_SSE = {"COMISS", "UCOMISS", "COMISD", "UCOMISD", "PCMPISTRI", "PCMPISTRI64"}       # each writes all six
FLAGS_NONE = {"imm", "addr", "ld", "st", "sel", "cst", "cstz", "mxcsr", "mmx", "fence", "rep", "cpuid", "df", "pause",
              "vex", "vexs", "fma", "vzeroupper"}
FLAGS_ALL_READ = {"x86", "ud", "fcw", "xgetbv", "x87"}        # the model's steps read the whole state
SHIFT_KINDS = ("shr", "shl", "sar", "rol", "ror")
LIVENESS_SCAN = 8                                              # instructions a successor is scanned for, at most


def _op_flags(op: Op, constant: dict[int, int]) -> tuple[int, int]:
    """(flags read, flags written) by one lifted op. A shift or rotate by a count unknown when compiling writes none
    (a zero count changes nothing); an op this table does not know reads every flag."""
    k = op[0]
    if k == "flags":
        kind, count, bits = op[1], op[3], op[5]
        if kind in SHIFT_KINDS:
            if count not in constant or constant[count] & (63 if bits == 64 else 31) == 0:
                return 0, 0
            return 0, U_CF | U_OF if kind in ("rol", "ror") else ALL_FLAGS
        return 0, FLAGS_WRITTEN.get(kind, 0)
    if k == "alu":
        return (U_CF if op[2] in ("adc", "sbb") else 0), 0
    if k == "br":
        return (0 if op[1] == "always" else COND_FLAGS[op[1].removeprefix("n")]), 0
    if k in ("set", "cmov"):
        return COND_FLAGS[op[1].removeprefix("n")], 0
    if k == "cx16":
        return 0, U_ZF
    if k == "div":                                             # #DE before any write; every flag undefined after
        return ALL_FLAGS, ALL_FLAGS
    if k == "sse":
        return 0, ALL_FLAGS if op[1] in FLAGS_SSE else 0
    if k in FLAGS_NONE:
        return 0, 0
    return ALL_FLAGS, 0                                        # FLAGS_ALL_READ, and anything not listed


def _op_flag_masks(ins: Lifted, nzcv_kind: str | None) -> list[tuple[int, int]]:
    """_op_flags of each op of `ins`. A Jcc that branches on the previous instruction's NZCV (`nzcv_kind`,
    NZCV_CONDITIONS) reads no flag register."""
    constant: dict[int, int] = {}
    out = []
    for op in ins.ops:
        if op[0] == "imm":
            constant[op[1]] = op[2]
        r, w = _op_flags(op, constant)
        if op[0] == "br" and op[1] in NZCV_CONDITIONS.get(nzcv_kind or "", {}):
            r = 0
        out.append((r, w))
    return out


def insn_flags(ins: Lifted, nzcv_kind: str | None = None) -> tuple[int, int]:
    """(flags `ins` reads before writing them, flags it writes)."""
    read = written = 0
    for r, w in _op_flag_masks(ins, nzcv_kind):
        read |= r & ~written
        written |= w
    return read, written


def nzcv_branch(setter: Lifted, nxt: Lifted) -> bool:
    """`nxt` is a Jcc that branches on `setter`'s NZCV (compile_insn's `nzcv_from`)."""
    kind = nzcv_setter(setter)
    return kind is not None and any(op[0] == "br" and op[1] in NZCV_CONDITIONS[kind] for op in nxt.ops)


def _exit_if_undefined(a: Assembler, f: str) -> None:
    """Exit with HVC 0x40|bit when flag f is undefined (runtime.Runtime._flag)."""
    a.emit(a64.tbz(REG_UNDEFINED_FLAGS, FLAG_BIT[f], 8), a64.hvc(0x40 | FLAG_BIT[f]))


def _eval_condition(a: Assembler, cc: str) -> None:
    """x16 = condition `cc`, reading flags in runtime.Runtime._cond's order."""
    neg = cc.startswith("n")
    key = cc[1:] if neg else cc
    single = {"o": "O", "b": "C", "z": "Z", "s": "S", "p": "P"}
    if key == "p":
        _exit_if_undefined(a, "P")
        _pf_into_x16(a)
    elif key in single:
        _exit_if_undefined(a, single[key])
        a.emit(a64.mov(16, FLAG_REG[single[key]]))
    elif key in ("be", "le"):
        end = a.label()
        first = "C" if key == "be" else "Z"
        _exit_if_undefined(a, first)
        a.emit(a64.mov(16, FLAG_REG[first]))
        a.branch(lambda off: a64.cbnz(16, off), end)
        if key == "be":
            _exit_if_undefined(a, "Z")
            a.emit(a64.mov(16, REG_ZF))
        else:
            _exit_if_undefined(a, "S")
            _exit_if_undefined(a, "O")
            a.emit(a64.eor(16, REG_SF, REG_OF))
        a.bind(end)
    elif key == "l":
        _exit_if_undefined(a, "S")
        _exit_if_undefined(a, "O")
        a.emit(a64.eor(16, REG_SF, REG_OF))
    else:
        raise CompileError(f"condition {cc}")
    if neg:
        a.emit(a64.cmpi(16, 0), a64.cset(16, "eq"))


def _temps_read(op: Op) -> list[int]:
    k = op[0]
    if k == "sse":
        return [loc[1] for loc in (op[2], op[3]) if loc[0] == "m"]
    if k == "vex":                                      # ("vex", base, d, s1, s2, L, align, imm) (Sprint 049)
        return [loc[1] for loc in (op[2], op[4]) if loc[0] == "m"]
    if k == "vexs":                                     # ("vexs", ic, d, s1, s2, L, imm): cross-lane VEX
        return [loc[1] for loc in (op[2], op[4]) if loc[0] == "m"]
    if k == "fma":                                      # ("fma", ic, d, s2, s3, L)
        return [op[4][1]] if op[4][0] == "m" else []
    if k == "ld":
        return [op[2][1]] if op[2][0] == "m" else []
    if k == "st":
        return ([op[1][1]] if op[1][0] == "m" else []) + [op[2]]
    if k == "alu":
        return [op[3], op[4]]
    if k == "flags":
        return [op[2], op[3], op[4]]
    if k == "br":
        return [op[2]]
    if k == "div":
        return [op[2]]
    if k == "cx16":
        return [op[1]]
    if k == "set":
        return [op[2][1]] if op[2][0] == "m" else []
    if k == "mxcsr":                                    # ("mxcsr", "st"|"ld", m32): lowered in A64 (Sprint 065)
        return [op[2][1]]
    if k == "mmx":                                      # ("mmx", ic, dst, src, imm): lowered in A64 (Sprint 065)
        return [o[1] for o in (op[2], op[3]) if o[0] == "m"]
    if k == "sel":
        return [op[2], op[3], op[4]]
    if k in ("cst", "cstz"):
        return [op[1], op[3]] + ([op[2][1]] if op[2][0] == "m" else [])
    if k == "cmov":
        return [op[3]] + ([op[2][1]] if op[2][0] == "m" else [])
    if k == "x86":                                      # ("x86", ic, dst, srcs, bits): the model's (Sprint 050)
        return list(op[3]) + ([op[2][1]] if op[2] is not None and op[2][0] == "m" else [])
    return []


def _temps_written(op: Op) -> list[int]:
    return [op[1]] if op[0] in ("imm", "addr", "ld", "alu", "sel") else []


def _canonical_dll(name: str) -> str:
    """A DLL's import name as the module table keys it: lower case, '.dll' added when it has no extension."""
    c = name.lower()
    return c if "." in c else c + ".dll"


def runtime_defines() -> dict[str, int]:
    """The guest-memory addresses the guest runtime is compiled against (tools/guest_runtime)."""
    return {
        "W2M_TEB_VA": PROCESS_VA + TEB_OFF, "W2M_PEB_VA": PROCESS_VA + PEB_OFF,
        "W2M_TRANSPORT_VA": PROCESS_VA + TRANSPORT_OFF, "W2M_RING_VA": PROCESS_VA + RING_OFF,
        "W2M_STATE_VA": PROCESS_VA + STATE_OFF, "W2M_EXCEPTION_VA": PROCESS_VA + EXCEPTION_OFF,
        "W2M_TLS_EXPANSION_VA": PROCESS_VA + TLS_EXPANSION_OFF, "W2M_TLS_TAKEN": TLS_TAKEN,
        "W2M_HEAP_VA": HEAP_VA, "W2M_HEAP_SIZE": HEAP_SIZE,
        "W2M_COMMAND_LINE_VA": PROCESS_VA + COMMAND_LINE_OFF,
        "W2M_INHERIT_VA": PROCESS_VA + INHERIT_OFF,
        "W2M_CURRENT_DIRECTORY_VA": PROCESS_VA + CURRENT_DIRECTORY_OFF,
        "W2M_FAULT_LOG_VA": PROCESS_VA + FAULT_LOG_OFF, "W2M_TRACE_VA": PROCESS_VA + TRACE_OFF,
        "W2M_ENVIRONMENT_VA": PROCESS_VA + ENVIRONMENT_OFF, "W2M_ENVIRONMENT_SIZE": ENVIRONMENT_SIZE,
        "W2M_INIT_STUB_SLOT": INIT_STUB_SLOT,
        "W2M_MONITOR_DATA_VA": MONITOR_VA + MONITOR_DATA, "W2M_SAVE_VA": MONITOR_VA + MONITOR_DATA + SAVE_AREA,
        "W2M_RECON_VA": MONITOR_VA + RECON_TABLE, "W2M_RECON_COUNT_VA": MONITOR_VA + MONITOR_DATA + RECON_COUNT,
        "W2M_CODE_START": MONITOR_VA + CODE_CACHE, "W2M_CODE_END": MONITOR_VA + CODE_CACHE + CODE_CACHE_SIZE}


# ---- SSE (Sprint 041) ---------------------------------------------------------------------------
# XMM i is V i. V30 and V31 are scratch. MXCSR is at MONITOR_DATA + MXCSR_OFF, and FPCR follows it
# (fpcr_for): AH=1 for x86's NaN, MIN/MAX, tininess and denormal-input behavior (Sprint 039 probe),
# FIZ for DAZ, FZ for FTZ, RMode for RC. FPSR's cumulative flags are MXCSR's exception flags
# (FPSR_TO_MXCSR); `VM.mxcsr()` folds them in. An instruction that can raise an SIMD exception
# leaves the fast path when MXCSR unmasks any (hvc #10): the host executes it with x86fp.

FP_CLASSES = {f"{op}{t}" for op in ("ADD", "SUB", "MUL", "DIV", "SQRT", "MIN", "MAX") for t in ("SS", "SD", "PS", "PD")} | {
    "CVTTSS2SI", "CVTTSD2SI", "CVTSS2SI", "CVTSD2SI", "CVTSI2SS", "CVTSI2SD", "CVTSS2SD", "CVTSD2SS",
    "UCOMISS", "UCOMISD", "COMISS", "COMISD"}
FPSR_TO_MXCSR = ((0, 0), (1, 2), (2, 3), (3, 4), (4, 5), (7, 1))   # (FPSR bit, MXCSR bit): IOC IE, DZC ZE, OFC OE, UFC UE, IXC PE, IDC DE
HVC_SLOW_SSE = 10
HVC_SLOW_STEP = 11           # one instruction run by the model (a misaligned LOCK operand)
HVC_SCHEDULE = 12            # a service handed the vCPU to the scheduler (state.schedule; Sprint 051)
HVC_CODE_WRITE = 13          # a write faulted on a page behind translated code: x17 the address (Sprint 065)
AP_READ_ONLY = 1 << 7        # a descriptor's AP[2]: no writes at EL1 (Arm ARM D8.4.1)
# A level-3 permission fault on a write from the current EL: ESR's EC 0x25, WnR, DFSC 0x0F (Arm ARM D17.2.40)
CODE_WRITE_ESR_MASK = (0x3F << 26) | (1 << 6) | 0x3F
CODE_WRITE_ESR = (0x25 << 26) | (1 << 6) | 0x0F
STATE_REDIRECT, STATE_SCHEDULE = 0xE8, 0xF0       # runtime.h w2m_state.redirect_rip, .schedule
SCHEDULE_YIELD, SCHEDULE_RETRY, SCHEDULE_END, SCHEDULE_ANSWER = 1, 2, 4, 8
STATE_SIZE = 896
CVT_INDEFINITE = True        # False exists only so a test can show the CPU catches the missing fix-up


def fpcr_for(mxcsr: int) -> int:
    """FPCR for an MXCSR: AH always; FIZ = DAZ (bit 6); FZ = FTZ (bit 15); RMode from RC (bits 13-14:
    00 nearest, 01 down, 10 up, 11 toward zero; FPCR RMode 00 RN, 01 RP, 10 RM, 11 RZ)."""
    rc = mxcsr >> 13 & 3
    rmode = {0: 0, 1: 2, 2: 1, 3: 3}[rc]
    return 1 << 1 | (mxcsr >> 6 & 1) | (mxcsr >> 15 & 1) << 24 | rmode << 22


def mxcsr_flags_from_fpsr(fpsr: int) -> int:
    return sum(1 << m for f, m in FPSR_TO_MXCSR if fpsr >> f & 1)


def _lower_mxcsr(a: Assembler, kind: str, loc: Op, reg_of: Callable[[int], int]) -> None:
    """STMXCSR and LDMXCSR in A64 (Sprint 065), as `_slow_sse` leaves the state: frontier-643a6283510c-1791802134771149000
    ran UnityPlayer.dll's STMXCSR / LDMXCSR pair at 0x1810bdae9 415,536 times through the model. x16 and x17 only:
    TEMP_REGS holds x18, and the operand's address may be in it.
    STMXCSR stores the saved MXCSR with FPSR's cumulative flags folded in (FPSR_TO_MXCSR). LDMXCSR with a reserved
    bit (31:16) set leaves to the model, which raises #GP before any write; otherwise it saves the value, clears
    FPSR (its flags are now in the saved value) and sets FPCR as fpcr_for does."""
    addr = reg_of(loc[1])
    if kind == "st":
        a.emit(a64.mrs(17, a64.FPSR), a64.lsri(16, 17, 7),         # x16 bit 0: IDC
               a64.bfi(17, 17, 1, 5),                               # bits 5:1: IOC DZC OFC UFC IXC
               a64.bfi(17, 16, 1, 1),                               # bit 1: IDC -> DE; bit 0 stays IOC -> IE
               a64.ubfx(17, 17, 0, 6),
               a64.ldr(4, 16, 28, MXCSR_OFF), a64.orr(16, 16, 17), a64.str_(4, 16, addr))
        return
    ok = a.label()
    a.emit(a64.ldr(4, 16, addr), a64.lsri(17, 16, 16))
    a.branch(lambda off: a64.cbz(17, off), ok)
    a.emit(a64.hvc(HVC_SLOW_STEP))
    a.bind(ok)
    a.emit(a64.str_(4, 16, 28, MXCSR_OFF), a64.msr(a64.FPSR, a64.ZR),
           a64.ubfx(17, 16, 6, 1),                                  # FIZ = DAZ
           a64.lsri(16, 16, 13), a64.bfi(17, 16, 23, 1),            # RMode bit 23 = RC bit 0 (MXCSR 13)
           a64.lsri(16, 16, 1), a64.bfi(17, 16, 22, 1),             # RMode bit 22 = RC bit 1 (MXCSR 14)
           a64.lsri(16, 16, 1), a64.bfi(17, 16, 24, 1),             # FZ = FTZ (MXCSR 15)
           a64.movz(16, 1), a64.bfi(17, 16, 1, 1),                  # AH
           a64.msr(a64.FPCR, 17))


def build_runtime(last_error_source: str | None = None, file_source: str | None = None,
                  sources: dict[str, str] | None = None) -> guest_runtime.Image:
    """The guest runtime image, refused if it lacks a service SERVICES names (a candidate file.c or
    last_error.c that left one out builds and links, since a preload image resolves nothing)."""
    image = guest_runtime.build(MONITOR_VA + RUNTIME_IMAGE, runtime_defines(), last_error_source, file_source, sources)
    missing = sorted(sym for sym, _ in SERVICES.values() if sym not in image.symbols)
    if missing:
        raise guest_runtime.BuildError(f"the guest runtime does not define {', '.join(missing)}")
    return image


FUSE_LOAD_STORE = True       # recentering step 5: a load whose only use is the store after it is one instruction
FOLD_ADDRESSES = True        # recentering step 5: an address only loads and stores use goes into their own form
AddressForm = tuple[Any, ...]  # ("imm", base, disp) | ("unscaled", base, disp) | ("reg", base, index, scaled)
# recentering step 5, operand forms: a temporary that loads a 64-bit guest register reads the register itself when no op
# of the instruction writes that register before the temporary's last use (copy propagation); a constant only ADD or
# SUB reads, 0-4095, is their immediate; a constant only a branch reads goes straight into x17. Instructions with an op
# outside OPERAND_SAFE (model steps, REP, DIV, the SSE and VEX lowerings, CPUID) keep every temporary in a register.
OPERAND_FORMS = True
OPERAND_SAFE = {"imm", "addr", "ld", "st", "alu", "flags", "br", "set", "cmov", "sel", "cst", "cstz", "fence", "pause"}
ADD_SUB_IMM = {("add", 64): 0x91000000, ("sub", 64): 0xD1000000, ("add", 32): 0x11000000, ("sub", 32): 0x51000000}


LOW32_ALU = ("add", "sub", "and", "or", "xor")
LOW32_FLAGS = ("add", "sub", "inc", "dec", "logic")
FORWARD_AFTER = {"alu", "flags", "imm", "br", "sel", "st", "set", "cmov", "cst", "cstz"}


def _reads_low32(o: Op, t: int) -> bool:
    """`o` uses only the low 32 bits of temporary `t`: a 32-bit ADD, SUB, AND, OR or XOR (masked or in its W form), the
    flags of one (NZCV from the W form; AF and PF from the low bits), or a 32-bit store of it."""
    if o[0] == "alu":
        return bool(o[5] == 32 and o[2] in LOW32_ALU)
    if o[0] == "flags":
        return bool(o[5] == 32 and o[1] in LOW32_FLAGS)
    if o[0] == "st" and o[2] == t:
        loc = o[1]
        return bool(loc[0] == "m" and loc[2] == 4 and loc[1] != t or loc[0] == "r" and loc[2] == 32 and not loc[3])
    return False


def _operand_forms(ops: tuple[Op, ...], last: dict[int, int],
                   dead: set[int]) -> tuple[dict[int, int], dict[int, int], dict[int, int]]:
    """(temporaries read as the guest register they load, constants kept out of registers, results computed straight
    into the guest register they are stored to), per OPERAND_FORMS. The ops at `dead` (flags ops no later instruction
    reads) emit nothing and read nothing.
    A 32-bit register is read in place when every use takes only its low 32 bits (_reads_low32). A result is forwarded
    to register R when its one store is to R at its own width (zero-extended for 32 bits), its other readers are flags
    ops, and every op after it is one that cannot fault (FORWARD_AFTER, stores only to registers), reads no temporary
    that is R itself, and writes R only by that store."""
    alias: dict[int, int] = {}
    quiet: dict[int, int] = {}
    forward: dict[int, int] = {}
    readers: dict[int, list[Op]] = {}
    for i, op in enumerate(ops):
        if i in dead:
            continue
        for t in _temps_read(op):
            readers.setdefault(t, []).append(op)

    def writes(o: Op) -> Op | None:
        return o[1] if o[0] == "st" else o[2] if o[0] in ("set", "cmov", "cst", "cstz") else None
    for i, op in enumerate(ops):
        if op[0] == "ld" and op[2][0] == "r" and op[2][2] in (32, 64) and not op[2][3] and op[1] in last:
            idx = op[2][1]
            if op[2][2] == 32 and not all(_reads_low32(o, op[1]) for o in readers.get(op[1], [])):
                continue
            if not any((w := writes(o)) is not None and w[0] == "r" and w[1] == idx for o in ops[i + 1:last[op[1]]]):
                alias[op[1]] = idx
        elif op[0] == "imm" and op[1] in readers:
            rs, t, v = readers[op[1]], op[1], op[2]
            if 0 <= v < 4096 and all(o[0] == "alu" and o[2] in ("add", "sub") and o[4] == t and o[3] != t for o in rs):
                quiet[t] = v
            elif len(rs) == 1 and rs[0][0] == "br" and rs[0][2] == t:
                quiet[t] = v
    for i, op in enumerate(ops):
        if op[0] == "alu" or op[0] == "ld" and op[2][0] == "m" and op[1] not in alias:
            t = op[1]
            width = op[5] if op[0] == "alu" else 8 * op[2][2]
            stores = [o for o in readers.get(t, []) if o[0] != "flags"]
            if len(stores) != 1 or stores[0][0] != "st" or stores[0][2] != t:
                continue
            loc = stores[0][1]
            if loc[0] != "r" or loc[3] or loc[2] != width or width not in (32, 64):
                continue
            idx = loc[1]
            later = [o for j, o in enumerate(ops) if j > i and j not in dead]
            if all(o[0] in FORWARD_AFTER and ((w := writes(o)) is None or w[0] == "r" and (w[1] != idx or o is stores[0]))
                   and not any(alias.get(r) == idx for r in _temps_read(o)) for o in later):
                forward[t] = idx
    return alias, quiet, forward


def _folded_addresses(ops: tuple[Op, ...]) -> dict[int, AddressForm]:
    """The `addr` temporaries whose every use in this instruction is a plain load or store, each with the Arm form that
    reaches it (recentering step 5): ("imm", base, disp) for LDR/STR's own scaled offset, ("unscaled", base, disp) for
    LDUR/STUR's -256..255, ("reg", base, index, scaled) for the register-offset form. 64-bit addresses with no segment
    only; anything else keeps the computed address."""
    users: dict[int, list[tuple[str, int]]] = {}
    other: set[int] = set()
    for op in ops:
        if op[0] == "ld" and op[2][0] == "m":
            users.setdefault(op[2][1], []).append(("ld", op[2][2]))
            continue
        if op[0] == "st" and op[1][0] == "m":
            users.setdefault(op[1][1], []).append(("st", op[1][2]))
            other.update(t for t in _temps_read(op) if t != op[1][1])
            continue
        other.update(_temps_read(op))
    folded: dict[int, AddressForm] = {}
    for op in ops:
        if op[0] != "addr":
            continue
        _, d, base, index, scale, disp, abits, seg = op
        if d in other or d not in users or seg is not None or abits != 64 or not isinstance(base, int):
            continue
        sizes = {size for _, size in users[d]}
        if index is None:
            if all(disp % s == 0 and 0 <= disp // s < 4096 for s in sizes):
                folded[d] = ("imm", base, disp)
            elif -256 <= disp <= 255:
                folded[d] = ("unscaled", base, disp)
        elif isinstance(index, int) and disp == 0 and (scale == 1 or sizes == {scale}):
            folded[d] = ("reg", base, index, scale != 1)
    return folded


def _mem(load: bool, size: int, rt: int, form: AddressForm) -> int:
    """The load or store of `size` bytes between rt and a folded address (_folded_addresses)."""
    if form[0] == "imm":
        return (a64.ldr if load else a64.str_)(size, rt, form[1], form[2])
    if form[0] == "unscaled":
        return (a64.ldur if load else a64.stur)(size, rt, form[1], form[2])
    return (a64.ldr_reg if load else a64.str_reg)(size, rt, form[1], form[2], form[3])


def _fused_move(a: Assembler, op: Op, ops: tuple[Op, ...], i: int, last: dict[int, int],
                reg_of: Callable[[int], int], folded: dict[int, AddressForm] | None = None) -> bool:
    """A load (op, at i) whose temporary only the next op stores, as one instruction straight between the guest's
    register and memory or register (MOV and MOVZX: 34.7% of the first 10 billion instructions ran 3.49 arm64 words
    each). A register to a register of 32 or 64 bits is one MOV; memory to a register of 32 or 64 bits is one LDR into
    it, which zero-extends as x86's 32-bit destinations do; a register to memory of its own width is one STR. A load
    that faults has written nothing, as before. Returns False when the pair is none of these."""
    if i + 1 >= len(ops):
        return False
    nxt, t, src = ops[i + 1], op[1], op[2]
    if nxt[0] != "st" or nxt[2] != t or last.get(t) != i + 1:
        return False
    dst = nxt[1]
    if src[0] == "r" and dst[0] == "r":
        _, s, sbits, shi = src
        _, d, dbits, dhi = dst
        if shi or dhi or sbits != dbits or sbits not in (32, 64):
            return False
        a.emit(a64.mov(d, s) if sbits == 64 else a64.mov32(d, s))
        return True
    if src[0] == "m" and dst[0] == "r":
        _, d, dbits, dhi = dst
        if dhi or dbits not in (32, 64) or src[2] * 8 > dbits:
            return False
        a.emit(_mem(True, src[2], d, folded[src[1]]) if folded and src[1] in folded else a64.ldr(src[2], d, reg_of(src[1])))
        return True
    if src[0] == "r" and dst[0] == "m":
        _, s, sbits, shi = src
        if shi or sbits != dst[2] * 8:
            return False
        a.emit(_mem(False, dst[2], s, folded[dst[1]]) if folded and dst[1] in folded else a64.str_(dst[2], s, reg_of(dst[1])))
        return True
    return False


def compile_insn(a: Assembler, ins: Lifted, exit_va: int, symbols: dict[str, int] | None = None,
                 nzcv_from: str | None = None, live_after: int = ALL_FLAGS, nzcv_out: bool = False) -> bool:
    """Append `ins` to `a`. A taken branch puts the x86 target in x17 and jumps to `exit_va`. `symbols` are the
    compiling VM's runtime image's (CPUID calls its w2m_cpuid; Sprint 058: one table for every VM in the host gave a
    parent the child's addresses); without them, as in an assembler check, the address is 0.
    `live_after`: the flags a later instruction reads (flag liveness); `nzcv_out`: the next instruction's Jcc
    branches on this one's NZCV.
    Returns True when control never falls through (an unconditional branch)."""
    flags_live: dict[int, int] = {}                      # each flags op: the flags read after it
    live = live_after
    if any(op[0] == "flags" for op in ins.ops):           # (most instructions set none)
        masks = _op_flag_masks(ins, nzcv_from)
        for i in range(len(ins.ops) - 1, -1, -1):
            if ins.ops[i][0] == "flags":
                flags_live[i] = live
            live = live & ~masks[i][1] | masks[i][0]
    last_flags = max(flags_live, default=-1)
    dead = {i for i, v in flags_live.items() if v == 0 and not (nzcv_out and i == last_flags)}   # emit nothing
    last: dict[int, int] = {}
    for i, op in enumerate(ins.ops):
        if i not in dead:
            for t in _temps_read(op):
                last[t] = i
    free = list(TEMP_REGS)
    reg: dict[int, int] = {}

    def reg_of(t: int) -> int:
        return reg[t]

    def store(loc: Op, rt: int) -> None:
        if loc[0] == "r":
            _, idx, bits, hi = loc
            if hi:
                a.emit(a64.bfi(idx, rt, 8, 8))
            elif bits == 64:
                a.emit(a64.mov(idx, rt))
            elif bits == 32:
                a.emit(a64.mov32(idx, rt))
            else:
                a.emit(a64.bfi(idx, rt, 0, bits))
        elif loc[1] in folded:
            a.emit(_mem(False, loc[2], rt, folded[loc[1]]))
        else:
            a.emit(a64.str_(loc[2], rt, reg_of(loc[1])))

    ends = False
    nzcv_ready = False                                   # an ALU op fused with the flags op that follows
    fused_store = False                                  # a load fused with the store that follows (_fused_move)
    constant: dict[int, int] = {}                        # temporaries an `imm` of this instruction set
    locked = ins.iclass.endswith("_LOCK")
    folded = _folded_addresses(ins.ops) if FOLD_ADDRESSES and not locked else {}
    lock_retry: list[int] = []
    alias, quiet, forward = ({}, {}, {}) if (not OPERAND_FORMS or locked
                                             or any(op[0] not in OPERAND_SAFE for op in ins.ops)) else \
        _operand_forms(ins.ops, last, dead)
    pinned = {**alias, **forward}                         # temporaries that are guest registers, never freed
    for i, op in enumerate(ins.ops):
        for t in _temps_written(op):
            if t in pinned:
                reg[t] = pinned[t]
                continue
            if t in quiet:
                continue
            if not free:
                raise CompileError(f"more than {len(TEMP_REGS)} live temporaries at 0x{ins.address:x}")
            reg[t] = free.pop(0)
        for t in _temps_written(op):
            if op[0] != "imm":
                constant.pop(t, None)
        k = op[0]
        if k == "imm":
            if op[1] not in quiet:
                a.emit(a64.mov_imm(reg_of(op[1]), op[2]))
            constant[op[1]] = op[2]
        elif k == "addr" and op[1] in folded:
            pass                                         # the loads and stores reach it in their own form
        elif k == "addr":
            _, d, base, index, scale, disp, abits, seg = op
            rd = reg_of(d)
            if isinstance(base, int) and FOLD_ADDRESSES and -4096 < disp < 4096:
                # recentering step 5: base (+ index) and a small displacement as ADD/SUB immediates, not a MOV of it
                if index is not None:
                    a.emit(a64.add(rd, base, index, a64.LSL, scale.bit_length() - 1))
                else:
                    a.emit(a64.mov(rd, base))
                if disp > 0:
                    a.emit(0x91000000 | disp << 10 | rd << 5 | rd)          # ADD Xd, Xd, #disp
                elif disp < 0:
                    a.emit(0xD1000000 | -disp << 10 | rd << 5 | rd)         # SUB Xd, Xd, #-disp
            else:
                a.emit(a64.mov_imm(rd, disp & M64))
                if base is not None:
                    a.emit(a64.add(rd, rd, base))
                if index is not None:
                    a.emit(a64.add(rd, rd, index, a64.LSL, scale.bit_length() - 1))
            _mask(a, rd, abits)
            if seg == "GS":                    # + the thread's GS base: state block gs_base (the TEB)
                a.emit(a64.mov_imm(16, PROCESS_VA + STATE_OFF + STATE_GS_BASE), a64.ldr(8, 16, 16, 0), a64.add(rd, rd, 16))
        elif k == "ld" and FUSE_LOAD_STORE and not locked and _fused_move(a, op, ins.ops, i, last, reg_of, folded):
            fused_store = True                           # the store that follows is done (_fused_move)
        elif k == "ld" and op[1] in alias:
            pass                                         # read as the guest register itself (OPERAND_FORMS)
        elif k == "ld":
            rt, loc = reg_of(op[1]), op[2]
            if loc[0] == "r":
                _, idx, bits, hi = loc
                if hi:
                    a.emit(a64.ubfx(rt, idx, 8, 8))
                elif bits == 64:
                    a.emit(a64.mov(rt, idx))
                elif bits == 32:
                    a.emit(a64.mov32(rt, idx))
                else:
                    a.emit(a64.ubfx(rt, idx, 0, bits))
            elif locked:
                # LOCK: a load-exclusive / store-exclusive loop. The commit rule leaves no guest effect
                # between the load and the store, so a failed store-exclusive repeats only temporaries.
                # Exclusives need natural alignment; a misaligned operand runs in the model (hvc #11).
                size, addr_reg = loc[2], reg_of(loc[1])
                if size > 1:
                    aligned = a.label()
                    a.emit(a64.ubfx(16, addr_reg, 0, size.bit_length() - 1))
                    a.branch(lambda off: a64.cbz(16, off), aligned)
                    a.emit(a64.hvc(HVC_SLOW_STEP))
                    a.bind(aligned)
                retry = a.label()
                a.bind(retry)
                lock_retry.append(retry)
                a.emit(a64.ldaxr(size, rt, addr_reg))
            elif loc[1] in folded:
                a.emit(_mem(True, loc[2], rt, folded[loc[1]]))
            else:
                a.emit(a64.ldr(loc[2], rt, reg_of(loc[1])))
        elif k == "st" and locked and op[1][0] == "m":
            a.emit(a64.stlxr(op[1][2], 16, reg_of(op[2]), reg_of(op[1][1])))
            a.branch(lambda off: a64.cbnz(16, off), lock_retry[-1])
        elif k == "st" and fused_store:
            fused_store = False
        elif k == "st" and op[2] in forward and op[1][0] == "r" and op[1][1] == forward[op[2]]:
            pass                                         # computed in place (OPERAND_FORMS)
        elif k == "st":
            store(op[1], reg_of(op[2]))
        elif k == "alu":
            _, d, o, x, y, bits = op
            rd, rx, ry = reg_of(d), reg_of(x), reg.get(y, -1)
            # the flags op this ALU op feeds: the next op past any store of the result (a store leaves NZCV alone)
            at = next((j for j in range(i + 1, len(ins.ops)) if ins.ops[j][0] != "st"), None)
            nxt = ins.ops[at] if at is not None and at not in dead else None
            if (FLAGS_FROM_NZCV and o in ("add", "sub") and bits in (32, 64) and nxt is not None and nxt[0] == "flags"
                    and nxt[1] in (("add", "inc") if o == "add" else ("sub", "dec")) and tuple(nxt[2:6]) == (x, y, d, bits)):
                # recentering step 5: the result and NZCV from one ADDS or SUBS; the 32-bit form zero-extends as
                # x86 does, so no mask follows, and the flags op that comes next skips its own
                a.emit(_adds_subs(o == "add", rd, rx, ry, bits))
                nzcv_ready = True
                continue
            if y in quiet:                                     # ADD or SUB with its immediate (OPERAND_FORMS)
                a.emit(ADD_SUB_IMM[(o, 32 if bits == 32 else 64)] | quiet[y] << 10 | rx << 5 | rd)
                bits = 64 if bits == 32 else bits                # the 32-bit form zero-extends: no mask
            elif o in ("add", "sub", "and", "or", "xor"):
                f = {"add": a64.add, "sub": a64.sub, "and": a64.and_, "or": a64.orr, "xor": a64.eor}[o]
                if bits == 32 and OPERAND_FORMS:                 # the W form zero-extends as x86 does: no mask
                    a.emit(f(rd, rx, ry) & ~(1 << 31))
                    bits = 64
                else:
                    a.emit(f(rd, rx, ry))
            elif o == "shr":
                a.emit(a64.ubfx(16, ry, 0, 6 if bits == 64 else 5))
                if bits < 64:
                    a.emit(a64.ubfx(17, rx, 0, bits) if bits != 32 else a64.mov32(17, rx))
                    a.emit(a64.lsrv(rd, 17, 16))
                else:
                    a.emit(a64.lsrv(rd, rx, 16))
            elif o == "mul_lo":
                a.emit(a64.mul(rd, rx, ry))
            elif o == "mul_hi":
                a.emit(a64.umulh(rd, rx, ry))
            elif o == "sext32":
                a.emit(a64.sxtw(rd, rx))
            elif o in ("sext8", "sext16"):
                a.emit(a64.sbfx(rd, rx, 0, 8 if o == "sext8" else 16))
            elif o == "bswap":
                a.emit(a64.rev(rd, rx, bits))
            elif o == "popcnt":                                 # V31 is scratch (the SSE lowerings' too)
                a.emit(a64.ubfx(16, rx, 0, bits) if bits < 64 else a64.mov(16, rx), a64.fmov_to_v(8, 31, 16),
                       a64.cnt_8b(31, 31), a64.addv_8b(31, 31), a64.fmov_from_v(4, rd, 31))
            elif o == "adc":                                    # a + b + CF; CF must be defined
                _exit_if_undefined(a, "C")
                a.emit(a64.add(rd, rx, ry), a64.add(rd, rd, REG_CF))
            elif o in ("bsf", "bsr"):                           # the result is stored only for a nonzero source
                a.emit(a64.ubfx(16, rx, 0, bits) if bits < 64 else a64.mov(16, rx))
                if o == "bsf":
                    a.emit(a64.rbit(16, 16), a64.clz(rd, 16))
                else:
                    a.emit(a64.clz(16, 16), a64.movz(17, 63), a64.sub(rd, 17, 16))
            elif o == "sbb":                                    # a - b - CF; CF must be defined
                _exit_if_undefined(a, "C")
                a.emit(a64.sub(rd, rx, ry), a64.sub(rd, rd, REG_CF))
            elif o in ("shl", "sar"):
                a.emit(a64.ubfx(16, ry, 0, 6 if bits == 64 else 5))
                if o == "shl":
                    a.emit(a64.lslv(rd, rx, 16))
                else:
                    a.emit(a64.sbfx(17, rx, 0, bits), a64.asrv(rd, 17, 16))
            elif o in ("rol", "ror"):
                a.emit(a64.ubfx(16, ry, 0, 6 if bits == 64 else 5), a64.ubfx(16, 16, 0, bits.bit_length() - 1))
                if bits in (32, 64):
                    if o == "rol":
                        a.emit(a64.sub(16, a64.ZR, 16))       # rotate left by k = right by -k
                    a.emit(a64.rorv(rd, rx, 16, bits))
                else:                                          # 8 and 16 bits: two shifts and an OR
                    a.emit(a64.ubfx(17, rx, 0, bits))
                    a.emit(a64.lslv(rd, 17, 16) if o == "rol" else a64.lsrv(rd, 17, 16))
                    a.emit(a64.sub(16, a64.ZR, 16), a64.addi(16, 16, bits))
                    a.emit(a64.lsrv(17, 17, 16) if o == "rol" else a64.lslv(17, 17, 16))
                    a.emit(a64.orr(rd, rd, 17))
            elif o in ("smul_lo", "smul_hi"):
                if bits == 64:
                    a.emit(a64.mul(rd, rx, ry) if o == "smul_lo" else a64.smulh(rd, rx, ry))
                else:
                    a.emit(a64.sbfx(16, rx, 0, bits), a64.sbfx(17, ry, 0, bits), a64.mul(rd, 16, 17))
                    if o == "smul_hi":
                        a.emit(a64.ubfx(rd, rd, bits, bits))
            else:
                raise CompileError(f"alu {o}")
            _mask(a, rd, bits)
        elif k == "flags":
            _, kind, x, y, r, bits = op
            live, nzcv = flags_live[i], nzcv_out and i == last_flags
            if live == 0 and not nzcv:
                pass                                     # no later instruction reads them (flag liveness)
            elif kind in ("shr", "shl", "sar", "rol", "ror") and y in constant and FLAGS_FROM_NZCV:
                _shift_flags_const(a, kind, reg_of(x), constant[y], reg_of(r), bits)
            else:
                _set_flags(a, kind, reg_of(x), reg_of(y), reg_of(r), bits, nzcv_ready, live, nzcv)
            nzcv_ready = False
        elif k == "mxcsr":
            _lower_mxcsr(a, op[1], op[2], reg_of)
        elif k == "mmx":                                         # Sprint 065: src/w2m/rules/_mmx.py
            from w2m.rules import _mmx
            _mmx.lower(a, op, reg_of)
        elif k in ("ud", "fcw", "xgetbv", "x87"):              # run by the model (hvc #11): MXCSR's fault rule
            a.emit(a64.hvc(HVC_SLOW_STEP))                       # and FPCR, the profile's #UD, said once
        elif k == "fence":
            a.emit({"st": a64.dmb_ishst, "ld": a64.dmb_ishld, "full": a64.dmb_ish}[op[1]]())
        elif k == "rep":
            # REP STOS / MOVS: the guest's RCX, RSI, RDI live in x1, x6, x7 and change once per element,
            # so a fault on an element leaves them at that iteration and the reconstruction table
            # gives the instruction's address (SDM Vol 1 7.3.9: the instruction is interruptible).
            _, kind, size = op
            loop, end = a.label(), a.label()
            a.emit(a64.ldr(1, 16, 28, DF_BYTE), a64.movz(17, size))
            a.branch(lambda off: a64.cbz(16, off), loop)
            a.emit(a64.sub(17, a64.ZR, 17))                    # DF: step down
            a.bind(loop)
            a.branch(lambda off: a64.cbz(RCX, off), end)
            if kind == "movs":
                a.emit(a64.ldr(size, 16, RSI, 0), a64.str_(size, 16, RDI, 0), a64.add(RSI, RSI, 17))
            else:
                a.emit(a64.str_(size, RAX, RDI, 0))
            a.emit(a64.add(RDI, RDI, 17), a64.subi(RCX, RCX, 1))
            a.branch(a64.b, loop)
            a.bind(end)
        elif k == "div":
            # DIV: the quotient overflows exactly when the dividend's high half >= the divisor, which
            # includes a zero divisor; those, a 64-bit dividend with RDX != 0 (no 128/64 divide here)
            # and the 8- and 16-bit forms run in the model (hvc #11), which raises #DE as Windows does.
            # Every check comes before any write (commit rule). Flags: all undefined.
            _, bits, src, *signed = op
            rs, fast, done = reg_of(src), a.label(), a.label()
            if signed and signed[0]:
                # IDIV (Sprint 046): fast only for a 64-bit dividend that is RAX sign-extended and a divisor
                # neither 0 nor -1 (INT64_MIN / -1 overflows), and (Sprint 064) the 32-bit form whose quotient
                # fits; the rest, every 8- and 16-bit form included, runs in the model
                if bits == 64:
                    a.emit(a64.sbfx(16, RAX, 63, 1), a64.cmp(16, RDX))
                    a.branch(lambda off: a64.bcond("ne", off), done)
                    a.branch(lambda off: a64.cbz(rs, off), done)
                    a.emit(a64.addi(16, rs, 1))
                    a.branch(lambda off: a64.cbz(16, off), done)
                    a.emit(a64.sdiv(16, RAX, rs), a64.msub(RDX, 16, rs, RAX), a64.mov(RAX, 16))
                    a.branch(a64.b, fast)
                elif bits == 32:
                    # Sprint 064 (the frontier ran ~100,000 model steps at four 32-bit IDIVs): EDX:EAX as a signed
                    # 64-bit dividend over the sign-extended divisor; a zero divisor or a quotient past 32 bits (the
                    # #DE cases, INT64_MIN / -1 among them) still runs in the model. Temporaries only until the end.
                    a.emit(a64.mov32(16, RDX), a64.eor(16, a64.ZR, 16, a64.LSL, 32), a64.bfi(16, RAX, 0, 32),
                           a64.sbfx(17, rs, 0, 32))
                    a.branch(lambda off: a64.cbz(17, off), done)
                    a.emit(a64.sdiv(18, 16, 17), a64.msub(16, 18, 17, 16), a64.sbfx(17, 18, 0, 32), a64.cmp(17, 18))
                    a.branch(lambda off: a64.bcond("ne", off), done)
                    a.emit(a64.mov32(RAX, 18), a64.mov32(RDX, 16))
                    a.branch(a64.b, fast)
            elif bits == 64:
                a.branch(lambda off: a64.cbnz(RDX, off), done)
                a.branch(lambda off: a64.cbz(rs, off), done)
                a.emit(a64.udiv(16, RAX, rs), a64.msub(RDX, 16, rs, RAX), a64.mov(RAX, 16))
                a.branch(a64.b, fast)
            elif bits == 32:
                a.emit(a64.mov32(16, RDX), a64.mov32(17, rs), a64.cmp(16, 17))
                a.branch(lambda off: a64.bcond("hs", off), done)
                a.emit(a64.eor(16, a64.ZR, 16, a64.LSL, 32), a64.bfi(16, RAX, 0, 32), a64.udiv(RAX, 16, 17),
                       a64.msub(RDX, RAX, 17, 16))
                a.branch(a64.b, fast)
            a.bind(done)
            a.emit(a64.hvc(HVC_SLOW_STEP))
            a.bind(fast)
            _mark_undefined(a, U_CF | U_ZF | U_SF | U_OF | U_PF | U_AF)
        elif k == "cx16":
            # CMPXCHG16B (Sprint 046): #GP unless 16-byte aligned, raised as SSE's alignment check raises it (a load
            # from 0 before anything is written); then an exclusive pair loop. Equal: RCX:RBX stored, ZF 1; else the
            # old value written back (x86 always writes), RDX:RAX loaded, ZF 0. The status register is borrowed and
            # kept in SPILL.
            rn = reg_of(op[1])
            status = 30 if rn != 30 else 29
            aligned, retry, differ, done = a.label(), a.label(), a.label(), a.label()
            a.emit(a64.ubfx(16, rn, 0, 4))
            a.branch(lambda off: a64.cbz(16, off), aligned)
            a.emit(a64.movz(16, 0), a64.ldr(8, 16, 16, 0))
            a.bind(aligned)
            a.emit(a64.str_(8, status, 28, SPILL))
            a.bind(retry)
            a.emit(a64.ldaxp(16, 17, rn), a64.cmp(16, RAX))
            a.branch(lambda off: a64.bcond("ne", off), differ)
            a.emit(a64.cmp(17, RDX))
            a.branch(lambda off: a64.bcond("ne", off), differ)
            a.emit(a64.stlxp(status, 3, 1, rn))                  # RBX low, RCX high
            a.branch(partial(a64.cbnz, status), retry)
            a.emit(a64.movz(REG_ZF, 1))
            a.branch(a64.b, done)
            a.bind(differ)
            a.emit(a64.stlxp(status, 16, 17, rn))
            a.branch(partial(a64.cbnz, status), retry)
            a.emit(a64.mov(RAX, 16), a64.mov(RDX, 17), a64.movz(REG_ZF, 0))
            a.bind(done)
            a.emit(a64.ldr(8, status, 28, SPILL))
            _mark_defined(a, U_ZF)
        elif k == "set":
            _eval_condition(a, op[1])
            store(op[2], 16)
        elif k == "sel":                                         # ("sel", t, c, x, y)
            rd, end = reg_of(op[1]), a.label()
            a.emit(a64.mov(rd, reg_of(op[4])))
            a.branch(partial(a64.cbz, reg_of(op[2])), end)
            a.emit(a64.mov(rd, reg_of(op[3])))
            a.bind(end)
        elif k in ("cst", "cstz"):                               # ("cst"|"cstz", c, loc, t)
            end = a.label()
            a.branch(partial(a64.cbz if k == "cst" else a64.cbnz, reg_of(op[1])), end)
            store(op[2], reg_of(op[3]))
            a.bind(end)
        elif k == "cmov":                                        # ("cmov", cc, loc, t): SDM CMOVcc
            other, end = a.label(), a.label()
            _eval_condition(a, op[1])
            a.branch(lambda off: a64.cbz(16, off), other)
            store(op[2], reg_of(op[3]))
            a.branch(a64.b, end)
            a.bind(other)
            if op[2][0] == "r" and op[2][2] == 32:              # a 32-bit destination is zero-extended anyway
                a.emit(a64.mov32(op[2][1], op[2][1]))
            a.bind(end)
        elif k == "x86":                                         # the model runs the instruction (Sprint 050;
            a.emit(a64.hvc(HVC_SLOW_STEP))                      # integer_model.py) and dispatches from its RIP
        elif k in ("vex", "vexs", "fma", "vzeroupper"):          # VEX (src/w2m/rules/_vex.py; Sprint 049)
            from w2m.rules import _vex
            _vex.lower(a, op, reg_of, store)
        elif k == "sse" and op[1] in ("PCMPISTRI", "PCMPISTRI64") and symbols and "w2m_pcmpistri" in symbols:
            # natively, as CPUID's call is made (2026-10-13, the speed work): the model ran each one at about 1.3 ms
            # (hvc #10; 2,129 s of the frontier's host time, UnityPlayer's strstr). The operands go to the monitor
            # data page at NATIVE_OPERANDS, the GPRs to the state block, and tools/guest_runtime/string_compare.c
            # answers ECX and CF ZF SF OF (AF and PF 0) in x0
            from w2m.rules._sse import _sse_src
            _, _, dst, src, _, imm = op
            v = _sse_src(a, src, 16, reg_of)
            state = PROCESS_VA + STATE_OFF
            a.emit(a64.str_v(16, int(dst[1]), 28, NATIVE_OPERANDS), a64.str_v(16, v, 28, NATIVE_OPERANDS + 16),
                   a64.mov_imm(16, state))
            for i in range(0, 16, 2):
                a.emit(a64.stp(i, i + 1, 16, 8 * i))
            a.emit(a64.addi(0, 28, NATIVE_OPERANDS), a64.addi(1, 28, NATIVE_OPERANDS + 16), a64.mov_imm(2, imm & 0xFF),
                   a64.mov_imm(16, MONITOR_VA + RUNTIME_STACK + RUNTIME_STACK_SIZE), a64.addi(31, 16, 0),
                   a64.mov_imm(16, symbols["w2m_pcmpistri"]), a64.blr(16), a64.mov(17, 0), a64.mov_imm(16, state))
            for i in range(0, 16, 2):
                a.emit(a64.ldp(i, i + 1, 16, 8 * i))
            a.emit(a64.ubfx(1, 17, 0, 5),                                       # ECX, RCX's upper half cleared
                   a64.ubfx(REG_CF, 17, 8, 1), a64.ubfx(REG_ZF, 17, 9, 1), a64.ubfx(REG_SF, 17, 10, 1),
                   a64.ubfx(REG_OF, 17, 11, 1), a64.movz(16, 1),          # PF and AF clear (PF_RESULT 1)
                   a64.stp(a64.ZR, a64.ZR, 28, AF_OPERANDS), a64.str_(8, 16, 28, PF_RESULT))
            _mark_defined(a, U_CF | U_ZF | U_SF | U_OF | U_PF | U_AF)
        elif k == "sse" and op[1] in ("RCPPS", "RSQRTPS", "RCPSS", "RSQRTSS") and symbols and "w2m_approximate" in symbols:
            # the reference CPU's tables in the runtime's C (tools/guest_runtime/approximate.c; Sprint 068 item C), as
            # PCMPISTRI's call: the destination and the source to NATIVE_OPERANDS, the GPRs to the state block (the
            # C uses no vector register: -mgeneral-regs-only), and the destination back with its new lanes
            from w2m.rules._sse import _check_alignment, _sse_src
            _, ic, dst, src, align, *_ = op
            n = 4 if ic.endswith("PS") else 1
            if src[0] == "m" and align:
                _check_alignment(a, reg_of(src[1]))
            v = _sse_src(a, src, 16 if n == 4 else 4, reg_of)
            state = PROCESS_VA + STATE_OFF
            a.emit(a64.str_v(16, int(dst[1]), 28, NATIVE_OPERANDS), a64.str_v(16, v, 28, NATIVE_OPERANDS + 16),
                   a64.mov_imm(16, state))
            for i in range(0, 16, 2):
                a.emit(a64.stp(i, i + 1, 16, 8 * i))
            a.emit(a64.addi(0, 28, NATIVE_OPERANDS), a64.addi(1, 28, NATIVE_OPERANDS + 16), a64.movz(2, int("RSQRT" in ic)),
                   a64.movz(3, n), a64.mov_imm(16, MONITOR_VA + RUNTIME_STACK + RUNTIME_STACK_SIZE), a64.addi(31, 16, 0),
                   a64.mov_imm(16, symbols["w2m_approximate"]), a64.blr(16), a64.mov_imm(16, state))
            for i in range(0, 16, 2):
                a.emit(a64.ldp(i, i + 1, 16, 8 * i))
            a.emit(a64.ldr_v(16, int(dst[1]), 28, NATIVE_OPERANDS))
        elif k == "sse":                                         # the class's own lowering (src/w2m/rules)
            from w2m.rules import registry
            lower = registry()[op[1]].lower
            if lower is None:
                raise CompileError(f"sse {op[1]}: its rule has no lowering")
            lower(a, op, reg_of, store)
        elif k == "cpuid":                                      # the guest runtime answers from the profile
            state = PROCESS_VA + STATE_OFF
            a.emit(a64.mov_imm(16, state))
            for i in range(0, 16, 2):
                a.emit(a64.stp(i, i + 1, 16, 8 * i))
            a.emit(a64.mov_imm(16, MONITOR_VA + RUNTIME_STACK + RUNTIME_STACK_SIZE), a64.addi(31, 16, 0),
                   a64.mov_imm(16, symbols["w2m_cpuid"] if symbols else 0), a64.blr(16), a64.mov_imm(16, state))
            for i in range(0, 16, 2):
                a.emit(a64.ldp(i, i + 1, 16, 8 * i))
        elif k == "df":                                         # CLD, STD
            a.emit(a64.movz(16, op[1]), a64.str_(1, 16, 28, DF_BYTE))
        elif k == "pause":                                      # PAUSE counted: the spin census (Sprint 068 item J)
            a.emit(a64.ldr(8, 16, 28, PAUSE_COUNT), a64.addi(16, 16, 1), a64.str_(8, 16, 28, PAUSE_COUNT))
        elif k == "br":
            target = a64.mov_imm(17, quiet[op[2]]) if op[2] in quiet else [a64.mov(17, reg_of(op[2]))]
            if op[1] == "always":
                a.emit(*target)
                a.branch(a64.b, exit_va)
                ends = True
            else:
                skip = a.label()
                arm = NZCV_CONDITIONS.get(nzcv_from or "", {}).get(op[1])
                if arm is not None:                              # the previous instruction's NZCV (NZCV_CONDITIONS)
                    inverse = INVERSE[arm]
                    a.branch(lambda off: a64.bcond(inverse, off), skip)
                else:
                    _eval_condition(a, op[1])
                    a.branch(lambda off: a64.cbz(16, off), skip)
                a.emit(*target)
                a.branch(a64.b, exit_va)
                a.bind(skip)
        else:
            raise CompileError(f"operation {k}")
        for t in [*_temps_read(op), *_temps_written(op)]:
            if t in reg and last.get(t, -1) <= i:
                r = reg.pop(t)
                if t not in pinned:
                    free.append(r)
    return ends


# ---------------------------------------------------------------------------------------------
# host driver

class Channel:
    """The host's side of the monitor's channel (vm_monitor.c `channel`; Sprint 068 item E). A call writes its fields,
    publishes them (a barrier, then the command word), wakes the monitor, and waits for the exit word: a spin of up
    to SPIN_NS, then os_sync_wait_on_address on the shared page (macOS 14.4)."""
    SPIN_NS = 50_000
    KINDS = {1: "hvc", 2: "exc", 3: "cancel", 4: "fail"}

    def __init__(self, mm: mmap.mmap) -> None:
        import ctypes
        self.mm = mm
        self._ref = ctypes.c_char.from_buffer(mm, CHANNEL)          # dropped at close, before the map closes
        # the two sequence words, each written and read as one 32-bit access: struct writes an integer a byte at a time
        self._cmd_word = ctypes.c_uint32.from_buffer(mm, CHANNEL + CH_CMD)
        self._exit_word = ctypes.c_uint32.from_buffer(mm, CHANNEL + CH_EXIT)
        base = ctypes.addressof(self._ref)
        self.cmd_addr, self.exit_addr = ctypes.c_void_p(base + CH_CMD), ctypes.c_void_p(base + CH_EXIT)
        lib = ctypes.CDLL("/usr/lib/libSystem.B.dylib")
        self.wait = lib.os_sync_wait_on_address
        self.wait.argtypes = [ctypes.c_void_p, ctypes.c_uint64, ctypes.c_size_t, ctypes.c_uint32]
        self.wake = lib.os_sync_wake_by_address_any
        self.wake.argtypes = [ctypes.c_void_p, ctypes.c_size_t, ctypes.c_uint32]
        self.barrier = lib.OSMemoryBarrier
        self.seq = struct.unpack_from("<I", mm, CHANNEL + CH_CMD)[0]

    def call(self, op: int) -> int:
        """Run `op` with the fields already written; its status."""
        struct.pack_into("<Q", self.mm, CHANNEL + CH_OP, op)
        self.barrier()
        self.seq = (self.seq + 1) & 0xFFFFFFFF
        self._cmd_word.value = self.seq
        self.wake(self.cmd_addr, 4, 1)
        until = time.perf_counter_ns() + self.SPIN_NS
        while (done := self._exit_word.value) != self.seq:
            if time.perf_counter_ns() > until:
                self.wait(self.exit_addr, done, 4, 1)
        self.barrier()
        return int(struct.unpack_from("<Q", self.mm, CHANNEL + CH_STATUS)[0])

    def run(self, pc: int, cpsr: int, x: list[int], ms: int, swap: tuple[int, int] | None) -> tuple["VcpuExit", int]:
        struct.pack_into("<QQQQ", self.mm, CHANNEL + CH_MS, ms, *(swap or (0, 0)), pc)
        struct.pack_into("<Q31Q", self.mm, CHANNEL + CH_CPSR, cpsr, *x)
        if self.call(OP_RUN):
            raise RuntimeError(f"vm_monitor: vswap {swap} failed")
        kind, syn, epc, cpsr_out, esr, far, elr = struct.unpack_from("<7Q", self.mm, CHANNEL + CH_KIND)
        regs = list(struct.unpack_from("<31Q", self.mm, CHANNEL + CH_EX))
        return VcpuExit(self.KINDS.get(kind, "fail"), syn, epc, esr, far, elr, regs), cpsr_out

    def swap(self, swap: tuple[int, int]) -> None:
        struct.pack_into("<QQ", self.mm, CHANNEL + CH_SAVE, *swap)
        if self.call(OP_SWAP):
            raise RuntimeError(f"vm_monitor: vswap {swap} failed")

    def vget(self) -> tuple[list[int], int, int]:
        if self.call(OP_VGET):
            raise RuntimeError("vm_monitor: vget failed")
        xmm = [int.from_bytes(self.mm[CHANNEL + CH_VEC + 16 * i:CHANNEL + CH_VEC + 16 * i + 16], "little") for i in range(16)]
        fpcr, fpsr = struct.unpack_from("<QQ", self.mm, CHANNEL + CH_VEC + 256)
        return xmm, int(fpcr), int(fpsr)

    def vset(self, xmm: list[int], fpcr: int, fpsr: int) -> None:
        for i, q in enumerate(xmm):
            self.mm[CHANNEL + CH_VEC + 16 * i:CHANNEL + CH_VEC + 16 * i + 16] = q.to_bytes(16, "little")
        struct.pack_into("<QQ", self.mm, CHANNEL + CH_VEC + 256, fpcr, fpsr)
        if self.call(OP_VSET):
            raise RuntimeError("vm_monitor: vset failed")

    def leave(self) -> None:
        self.call(OP_LEAVE)
        del self._ref, self._cmd_word, self._exit_word


@dataclass
class VcpuExit:
    kind: str
    syndrome: int
    pc: int
    esr: int
    far: int
    elr: int
    x: list[int]


def _write_translation_tables(mm: mmap.mmap, maps: list[tuple[int, int, int]]) -> tuple[int, int]:
    """Write 4-level stage-1 tables for (va, ipa, size) maps into the TRANSLATION_TABLES area; return the
    root table's IPA and the offset of the next free table. Descriptor layout: Arm ARM D8.3 (VMSAv8-64 translation
    table format)."""
    nxt = [TRANSLATION_TABLES + PAGE]

    def table_at(off: int, idx: int) -> int:
        desc = struct.unpack_from("<Q", mm, off + 8 * idx)[0]
        if desc:
            return int((desc & 0x0000_FFFF_FFFF_F000) - IPA_BASE)
        new = nxt[0]
        nxt[0] += PAGE
        if nxt[0] > TRANSLATION_TABLES + TRANSLATION_TABLES_SIZE:
            raise CompileError("stage-1 table area full")
        struct.pack_into("<Q", mm, off + 8 * idx, (IPA_BASE + new) | 0x3)
        return new

    for va, ipa, size in maps:
        if va % GIGA_BLOCK == 0 and ipa % GIGA_BLOCK == 0 and size % GIGA_BLOCK == 0:   # 1 GB blocks at level 1
            for p in range(0, size, GIGA_BLOCK):
                v = va + p
                off = table_at(TRANSLATION_TABLES, (v >> 39) & 0x1FF)
                struct.pack_into("<Q", mm, off + 8 * ((v >> 30) & 0x1FF), (ipa + p) | PAGE_ATTRIBUTES | 0x1)
            continue
        if va % BLOCK == 0 and ipa % BLOCK == 0 and size % BLOCK == 0:     # 2 MB blocks at level 2
            for p in range(0, size, BLOCK):
                v = va + p
                off = TRANSLATION_TABLES
                for level in (0, 1):
                    off = table_at(off, (v >> (39 - 9 * level)) & 0x1FF)
                struct.pack_into("<Q", mm, off + 8 * ((v >> 21) & 0x1FF), (ipa + p) | PAGE_ATTRIBUTES | 0x1)
            continue
        for p in range(0, size, PAGE):
            v = va + p
            off = TRANSLATION_TABLES
            for level in (0, 1, 2):
                off = table_at(off, (v >> (39 - 9 * level)) & 0x1FF)
            struct.pack_into("<Q", mm, off + 8 * ((v >> 12) & 0x1FF), (ipa + p) | PAGE_ATTRIBUTES | 0x3)
    return IPA_BASE + TRANSLATION_TABLES, nxt[0]


def _round_up(n: int, to: int = PAGE) -> int:
    return (n + to - 1) & ~(to - 1)


class _GuestView:
    """Guest memory as the file bridge sees it: the image's regions, and the monitor area where the
    runtime keeps its own buffers (a normalized path)."""

    def __init__(self, vm: VM) -> None:
        self.vm = vm

    def _monitor(self, va: int, n: int) -> int | None:
        off = va - MONITOR_VA
        return off if 0 <= off and off + n <= MONITOR_SIZE else None

    def read(self, va: int, n: int) -> bytes:
        off = self._monitor(va, n)
        return bytes(self.vm.mm[off:off + n]) if off is not None else self.vm.mem.read(va, n)

    def write(self, va: int, data: bytes) -> None:
        off = self._monitor(va, len(data))
        if off is not None:
            self.vm.mm[off:off + len(data)] = data
        else:
            self.vm.mem.write(va, data)


def service_for(dll: str, fn: str) -> tuple[str, bool] | None:
    """The runtime service an import binds to (its C symbol, whether it sets RAX), or None: hvc #9 at the call."""
    fn = ORDINAL_NAMES.get(dll.upper(), {}).get(fn, fn)
    service = (SERVICES.get(fn) if dll.upper() in ("KERNEL32.DLL", "KERNELBASE.DLL")
               else DLL_SERVICES.get(dll.upper(), {}).get(fn))
    if service is None and dll.upper() == "KERNEL.APPCORE.DLL":
        # kernel.appcore's AppPolicy* functions are threads.c's k32_ services; a C symbol cannot carry the dot in the
        # DLL's name (guest_runtime.service_table), so its names are answered from kernel32's table (Sprint 063)
        service = SERVICES.get(fn)
    if service is None and dll.upper() in ("KERNEL32.DLL", "KERNELBASE.DLL"):
        # kernel32 exports RtlUnwindEx, RtlCaptureContext and the other Rtl* exception functions as its own
        # (not forwarders: specs/windows_exports.json) and they do ntdll's work: one service serves both names
        service = DLL_SERVICES.get("NTDLL.DLL", {}).get(fn)
    return service


def synthetic_optional_header(mm: Any, page: int) -> None:
    """A synthetic module's PE32+ optional header (PE Format: Magic 0x20B at +0x18, SizeOfImage at +0x50 of the
    PE signature): the one page w2m keeps for it, so a program that sizes a module from its header (the
    running_thread_context probe: is RIP in ntdll) finds the page, as w2m's own module lookup does.
    SizeOfOptionalHeader (+0x14) says 0xF0, the PE32+ size."""
    struct.pack_into("<H", mm, page + 0x80 + 0x14, 0xF0)
    struct.pack_into("<H", mm, page + 0x80 + 0x18, 0x20B)
    struct.pack_into("<I", mm, page + 0x80 + 0x18 + 0x38, 0x1000)


class VM:
    """One guest thread of `loaded` in a fresh Hypervisor.framework VM.

    lockstep=True compiles one x86 instruction per block and sends every block exit to the host,
    so `on_dispatch` sees the guest state after each instruction (and after each kernel32 call).
    """

    def __init__(self, loaded: Loaded, tid: int = 4, lockstep: bool = False,
                 clock: Callable[[], int] | None = None, last_error_source: str | None = None,
                 files: FileBridge | None = None, file_source: str | None = None,
                 runtime_sources: dict[str, str] | None = None, image_name: str = "app.exe",
                 gui: bool = False, command_line: str | None = None,
                 environment: tuple[str, ...] = (), process_id: int = 0,
                 inherited: list[tuple[int, int, int]] | None = None, directory: str | None = None,
                 process_group: int = START_GROUP, call_trace: tuple[str, ...] = (),
                 call_trace_from: str | None = None, foreground_held: bool = False,
                 instruction_clock: tuple[int, int] | None = None) -> None:
        self.ld = loaded
        self.call_trace = call_trace       # the images whose calls call_trace.c records, at most 2 (Sprint 060)
        if len(call_trace) > 2:
            raise ValueError(f"call_trace.c traces at most two images, not {call_trace}")
        self.files = files                 # the file system's root on the Mac (Sprint 042); None: no file calls
        self.lockstep = lockstep
        self.clock = clock if clock is not None else monotonic_count   # a given clock is served by the host
        self.clock_from_host = clock is not None
        # (instructions a second, the wall clock's start as a FILETIME): the guest's clock counts the instructions it
        # runs, and jumps to the next deadline when no thread can run (Sprint 065 item 10); the host's random bytes
        # come from a fixed seed, so two runs of one tree make the same calls
        self.instruction_clock = instruction_clock
        self._random = random.Random(0x5EED) if instruction_clock else None
        self.gui = gui                     # the Steam-like start: no standard handles (console.c)
        self.foreground_held = foreground_held   # another process keeps the foreground (windows.c SetFocus; Sprint 065)
        self.command_line = command_line if command_line is not None else f'"C:\\w2m\\app\\{image_name}"'
        self.environment = environment     # the block's entries, in order ("NAME=value"; "=C:=..." too)
        self.process_id = process_id or PROCESS_ID   # TEB.ClientId.UniqueProcess; a child's from kernel.Machine
        self.inherited = inherited         # a child's handles from its parent (kernel.py PROCESS_CREATE); None: none
        self.directory = directory         # a child's current directory, its parent's (Sprint 058); None: the program's
        self.process_group = process_group   # ProcessParameters.ProcessGroupId (Sprint 059)
        self.image_name = image_name
        self.th = Thread(tid)
        self.exit_code: int | None = None
        self.out = bytearray()
        self.unimplemented: tuple[str, int] | None = None     # (import, return address) at a stop
        self.host_requests: dict[int, int] = {}       # transport requests by op (HOST_*, W2M_FILE_*): host exits
        self.exits: Counter[int] = Counter()            # hvc exits by number (Sprint 064's census)
        self.exit_sites: Counter[tuple[int, int]] = Counter()   # (hvc number, x86 address at the exit)
        # the time census (2026-10-13, the speed work): ns inside each `run` (the vCPU and the text pipe's round
        # trip), and ns the host spends on each exit kind from the run's return to the next `set`
        self.run_ns = 0
        self.exit_ns: Counter[int] = Counter()
        self.monitor_commands: Counter[str] = Counter()   # commands sent to the monitor, by name (Sprint 068)
        # a thread switch's vector swap, sent with the next `set` (vm_monitor.c vswap): the outgoing thread's slot
        # (0: none yet) and the incoming one's
        self._vswap: tuple[int, int] | None = None
        self._vslots_used = 0
        self._channel: Channel | None = None            # the monitor's channel while a run uses it (TRANSPORT)
        self._slice_start = (0, 0)                     # the counters when the running thread got the vCPU
        self._final_census: dict[int, tuple[int, int]] | None = None   # thread_census at close
        self._pending_init = 0                         # the DllMain stub of a DLL LoadLibrary just mapped (module.c)
        regions = [r for r in loaded.memory.regions if r.name != "thunks"]
        size = MONITOR_SIZE + PROCESS_SIZE
        placed: list[tuple[Region, int]] = []
        for r in regions:
            placed.append((r, size))
            size += _round_up(len(r.data))
        heap_off = _round_up(size, BLOCK)                 # the process heap: 2 MB aligned, mapped in blocks
        size = heap_off + HEAP_SIZE
        virtual_off = _round_up(IPA_BASE + size, GIGA_BLOCK) - IPA_BASE   # the VirtualAlloc arena: its IPA 1 GB aligned for 1 GB blocks; sparse
        size = virtual_off + VIRTUAL_SIZE
        self.size = _round_up(size, 0x4000)
        fd, self.path = tempfile.mkstemp(prefix="w2m-vm-")
        os.ftruncate(fd, self.size)
        self.mm = mmap.mmap(fd, self.size)
        os.close(fd)
        view = self._view = memoryview(self.mm)
        self.mem = Memory()
        for r, off in placed:
            n = len(r.data)
            self.mm[off:off + n] = bytes(r.data)
            self.mem.regions.append(Region(r.base, view[off:off + n], r.name))
        self.mem.regions.append(Region(HEAP_VA, view[heap_off:heap_off + HEAP_SIZE], "heap"))   # sparse: no copy
        self.mem.regions.append(Region(VIRTUAL_VA, view[virtual_off:virtual_off + VIRTUAL_SIZE], "virtual"))
        self.runtime = build_runtime(last_error_source, file_source, runtime_sources)
        if self.runtime.end > MONITOR_VA + RUNTIME_IMAGE + RUNTIME_IMAGE_SIZE:
            raise CompileError(f"guest runtime image ends at 0x{self.runtime.end:x}, past its area")
        for va, data in self.runtime.segments:
            off = va - MONITOR_VA
            self.mm[off:off + len(data)] = data
        self._init_process(tid)
        maps = [(MONITOR_VA, IPA_BASE, MONITOR_SIZE), (PROCESS_VA, IPA_BASE + MONITOR_SIZE, PROCESS_SIZE)]
        maps += [(r.base, IPA_BASE + off, _round_up(len(r.data))) for r, off in placed]
        maps += [(HEAP_VA, IPA_BASE + heap_off, HEAP_SIZE), (VIRTUAL_VA, IPA_BASE + virtual_off, VIRTUAL_SIZE)]
        self.ttbr, _ = _write_translation_tables(self.mm, maps)
        self._next_table = SPLIT_TABLES
        # Translated code is invalidated when its guest bytes change (Sprint 065 item 9e): the pages behind blocks are
        # read-only, a write to one faults (HVC_CODE_WRITE), and the blocks over it are dropped
        self.code_pages: dict[int, set[int]] = {}       # page VA -> the starts of the blocks over it
        # lifted instructions on pages behind translated code, and their flags (insn_flags by the NZCV kind before),
        # dropped with the page's blocks: a block's successor scan lifts what the successor's block lifts again
        self.lift_cache: dict[int, Lifted] = {}
        self.flags_cache: dict[tuple[int, str | None], tuple[int, int]] = {}
        self.lift_pages: dict[int, set[int]] = {}       # page VA -> the cached addresses whose bytes it holds
        self.code_writes = 0                            # writes that dropped blocks
        self.blocks_dropped = 0
        self.mem.on_write = self._host_wrote            # the host's own writes into guest memory fault on nothing
        self.code_cursor = CODE_CACHE
        self.blocks: dict[int, int] = {}                 # x86 address -> code VA
        self.addrs: set[int] = set()                     # x86 instructions compiled into blocks
        self.pc_map: list[tuple[int, int]] = []           # (code VA, x86 address), sorted
        self.translations: list[tuple[int, int, Discovery]] = []   # (guest start, guest end, discovery)
        self.slow_sse = 0                                # SSE instructions run by x86fp (MXCSR unmasked)
        self.slow_sse_classes: Counter[tuple[str, int]] = Counter()   # (iclass, MXCSR's six mask bits) per model step
        self._write_monitor()
        self.bridges = {thunk: self._bridge(name, thunk) for thunk, name in loaded.thunks.items()}
        self.modules = self._modules(image_name)
        self._trace_bound: set[str] = set()
        self._trace_quiet: dict[str, int] = {}           # untraced thunks GetProcAddress gave while the trace was closed
        self._trace_dynamic: dict[str, int] = {}         # the GetProcAddress hook's traced thunks, by name
        self._trace_ranges()
        # the trace opens when the guest first enters `call_trace_from` ("DLL!export"), as tools/jit_capture hooks the
        # images just before it calls UnityPlayer.dll!UnityMain (jit_capture.c wWinMain); 0: from the start
        self._trace_from = 0
        if call_trace_from is not None:
            dll, export = call_trace_from.split("!", 1)
            self._trace_from = self.modules.dlls[dll.lower()][2][export]
            struct.pack_into("<Q", self.mm, self._process(TRACE_OFF), 0)
        self._com_vtables()
        self.threads: dict[int, GuestThread] = {}       # guest threads by id (scheduler.py; Sprint 051)
        self.block_census: list[tuple[int, list[tuple[str, int]], int]] = []   # BLOCK_CENSUS's blocks
        self._stub_page = 0                              # guest_stubs.py's page, once written (Sprint 053)
        self.machine: Any = None                         # the kernel.Machine running this VM among others (Sprint 055)
        self._idle: tuple[GuestThread, int, bool] | None = None   # where _switch found no thread to run
        self._max_steps = 0
        self.current_tid = tid
        self.virtual = VirtualMemory(VIRTUAL_VA, VIRTUAL_SIZE, self._zero,
                                     [(i.base, i.pe) for i in loaded.images] or
                                     ([(loaded.image.image_base, loaded.image)] if loaded.image is not None else []),
                                     read=lambda va, n: bytes(self.mem.read(va, n)))
        self.virtual.stack = self._stack
        self.virtual.heap = (HEAP_VA, HEAP_SIZE)
        self.virtual.mapped = [(va, size) for va, _, size in maps] + [(THUNK_BASE, 1 << 32)]
        self.virtual.snapshot()
        self._flush_point = (self.code_cursor, struct.unpack_from("<Q", self.mm, MONITOR_DATA + RECON_COUNT)[0])

    def flush_translations(self) -> None:
        """Drop every translated block (bridges stay): the code cache, the dispatch table, the
        reconstruction table and the host's maps return to their state after construction. For a
        driver that puts different code at one address in turn (the semantics oracle)."""
        self.code_cursor, recon = self._flush_point
        struct.pack_into("<Q", self.mm, MONITOR_DATA + RECON_COUNT, recon)
        self.blocks.clear()
        self.lift_cache.clear()
        self.flags_cache.clear()
        self.lift_pages.clear()
        self.pc_map = [m for m in self.pc_map if m[0] < MONITOR_VA + self.code_cursor]
        empty = struct.pack("<QQ", M64, self.miss_va)
        self.mm[DISPATCH_TABLE:DISPATCH_TABLE + 16 * DISPATCH_ENTRIES] = empty * DISPATCH_ENTRIES
        self._write_exception_entry()                    # it returns to the guest through the dispatcher

    def _process(self, off: int) -> int:
        """File offset of a process-area offset."""
        return MONITOR_SIZE + off

    def _init_process(self, tid: int) -> None:
        """TEB and PEB fields at specs/guest_state.json offsets; the service ring's header; the clock mode."""
        struct.pack_into("<Q", self.mm, MONITOR_DATA + CLOCK_MODE,
                         1 if self.clock_from_host else 2 if self.instruction_clock else 0)
        if self.instruction_clock:
            struct.pack_into("<QQQ", self.mm, MONITOR_DATA + CLOCK_RATE, self.instruction_clock[0], 0,
                             self.instruction_clock[1])
        struct.pack_into("<Q", self.mm, MONITOR_DATA + START_MODE, 1 if self.gui else 0)
        struct.pack_into("<Q", self.mm, MONITOR_DATA + FOREGROUND_MODE, 1 if self.foreground_held else 0)
        line = self.command_line.encode("utf-16-le") + b"\0\0"
        if len(line) > 0x1000:
            raise CompileError(f"command line of {len(line)} bytes: the page holds 4096")
        self.mm[self._process(COMMAND_LINE_OFF):self._process(COMMAND_LINE_OFF) + len(line)] = line
        if any(not e or "\0" in e for e in self.environment):
            raise CompileError("an environment entry is empty or holds a NUL")
        block = "".join(e + "\0" for e in self.environment).encode("utf-16-le") + b"\0\0"
        if not self.environment:
            block = bytes(4)                                # an empty block is two NULs (row empty.heap)
        if len(block) > ENVIRONMENT_SIZE // 2:
            raise CompileError(f"environment of {len(block)} bytes: the area holds {ENVIRONMENT_SIZE // 2}")
        self.mm[self._process(ENVIRONMENT_OFF):self._process(ENVIRONMENT_OFF) + len(block)] = block
        self._environment_bytes = len(block)
        teb, peb = PROCESS_VA + TEB_OFF, PROCESS_VA + PEB_OFF
        stack = next(r for r in self.mem.regions if r.name == "stack")
        t = self._process(TEB_OFF)
        # the stack as Windows lays it out (contracts/windows.memory.thread_stack): DeallocationStack at the
        # reservation's bottom, one reserved page, the 3-page guard, then StackLimit; w2m commits the rest
        limit = stack.base + (1 + STACK_GUARD_PAGES) * 0x1000
        if self.inherited is not None:     # objects, then the standard handles (KIND_STANDARD), then the mark
            entries = [e for e in self.inherited if e[1] < KIND_STANDARD]
            std = {w: h for w, k, h in self.inherited if k == KIND_STANDARD}
            mode = [h for _, k, h in self.inherited if k == KIND_ERROR_MODE]
            if 32 + 24 * len(entries) > 0xFE8:           # the marks at 0xFE8 follow the entries (review of Sprint 054)
                raise Unimplemented(f"{len(entries)} inherited handles: the page holds {(0xFE8 - 32) // 24}")
            page = self._process(INHERIT_OFF)
            struct.pack_into("<QQQQ", self.mm, page, len(entries), std.get(0, 0), std.get(1, 0), std.get(2, 0))
            for i, e in enumerate(entries):
                struct.pack_into("<QQQ", self.mm, page + 32 + 24 * i, *e)
            # [509] standard handles given, [510] the error mode + 1 (0: none), [511] a child's page
            struct.pack_into("<QQQ", self.mm, page + 0xFE8, 1 if std else 0, mode[0] + 1 if mode else 0, 1)
        self._init_parameters()
        for field_off, value in ((0x08, self.ld.stack_top), (0x10, limit), (0x30, teb), (0x40, self.process_id),
                                 (0x48, tid), (0x58, self.ld.tls_array), (0x60, peb), (0x1478, stack.base)):
            struct.pack_into("<Q", self.mm, t + field_off, value)
        self._stack = (stack.base, limit, self.ld.stack_top)
        if self.ld.image is not None:
            struct.pack_into("<Q", self.mm, self._process(PEB_OFF) + 0x10, self.ld.image.image_base)
        struct.pack_into("<Q", self.mm, self._process(PEB_OFF) + 0x30, HEAP_VA)     # PEB.ProcessHeap (heap.c)
        struct.pack_into("<I", self.mm, self._process(PEB_OFF) + 0xB8, PROCESSORS)  # PEB.NumberOfProcessors (Chappell)
        st = self._process(STATE_OFF)                   # thread-start defaults (spec, observed on Windows)
        struct.pack_into("<I", self.mm, st + 152, 0x1F80)
        struct.pack_into("<H", self.mm, st + 156, 0x27F)
        struct.pack_into("<QQQ", self.mm, st + 176, teb, teb, 0)
        struct.pack_into("<QQ", self.mm, self._process(RING_OFF), 0, (RING_SIZE - RING_HEADER) // 24)
        struct.pack_into("<QQQQ", self.mm, self._process(TRACE_OFF), int(bool(self.call_trace)), 0, 0, TRACE_CAPACITY)

    def _init_parameters(self) -> None:
        """PEB.ProcessParameters as the rows of contracts/windows.process.parameters show Windows filling it."""
        std = [0, 0 if self.gui else STDOUT_HANDLE, 0]          # console.c's starts: the GUI start has none
        given = {w: h for w, k, h in (self.inherited or []) if k == KIND_STANDARD}
        if given:
            std = [given.get(i, 0) for i in range(3)]
        directory = self.directory if self.directory is not None else APP_PATH.rstrip("\\")
        dos_path = directory + ("" if directory.endswith("\\") else "\\")
        if 2 * len(dos_path) + 2 > CURRENT_DIRECTORY_MAX:
            raise Unimplemented(f"a current directory of {len(dos_path)} characters: the buffer holds "
                                f"{CURRENT_DIRECTORY_MAX // 2 - 1}")
        cwd = self._process(CURRENT_DIRECTORY_OFF)
        self.mm[cwd:cwd + 2 * len(dos_path) + 2] = dos_path.encode("utf-16-le") + b"\0\0"
        image = APP_PATH + self.image_name
        strings = [image, self.command_line, image, DESKTOP]   # ImagePathName, CommandLine, WindowTitle, DesktopInfo
        at, laid = PARAMETERS_VA + PARAMETERS_FIXED, []
        data = bytearray(PARAMETERS_FIXED)
        for text in strings:
            raw = text.encode("utf-16-le") + b"\0\0"
            laid.append((len(raw) - 2, len(raw), at))
            data += raw
            at += len(raw)
            data += bytes(-len(data) % 8)
            at = PARAMETERS_VA + len(data)
        length = len(data)

        def unicode_string(off: int, n: int, maximum: int, buffer: int) -> None:
            struct.pack_into("<HHIQ", data, off, n, maximum, 0, buffer)

        struct.pack_into("<IIII", data, 0, length, length, PARAMETERS_FLAGS, 0)
        struct.pack_into("<QQQQ", data, 0x18 - 8, 0, 0, std[0], std[1])        # ConsoleHandle 0x10, ConsoleFlags 0x18
        struct.pack_into("<Q", data, 0x30, std[2])
        unicode_string(0x38, 2 * len(dos_path), CURRENT_DIRECTORY_MAX, PROCESS_VA + CURRENT_DIRECTORY_OFF)
        struct.pack_into("<Q", data, 0x48, CURRENT_DIRECTORY_HANDLE)
        for off, (n, maximum, buffer) in zip((0x60, 0x70, 0xB0, 0xC0), laid):
            unicode_string(off, n, maximum, buffer)
        struct.pack_into("<Q", data, 0x80, PROCESS_VA + ENVIRONMENT_OFF)
        struct.pack_into("<QQ", data, 0x3F0, self._environment_bytes, 3)        # EnvironmentSize, EnvironmentVersion
        struct.pack_into("<I", data, 0x408, self.process_group)
        if PARAMETERS_VA + length > HEAP_VA + 0x10000:
            raise Unimplemented(f"process parameters of {length} bytes: the heap's first pages hold {0xF000}")
        self.mem.write(PARAMETERS_VA, bytes(data))
        struct.pack_into("<Q", self.mm, self._process(PEB_OFF) + 0x20, PARAMETERS_VA)   # PEB.ProcessParameters

    def _modules(self, image_name: str) -> Modules:
        """The module table (src/w2m/modules.py) and the synthetic kernel32 and kernelbase images."""
        k32, kb = PROCESS_VA + KERNEL32_IMAGE_OFF, PROCESS_VA + KERNELBASE_IMAGE_OFF
        ntdll = PROCESS_VA + NTDLL_IMAGE_OFF
        for off in (KERNEL32_IMAGE_OFF, KERNELBASE_IMAGE_OFF, NTDLL_IMAGE_OFF):
            page = self._process(off)
            self.mm[page:page + 2] = b"MZ"
            struct.pack_into("<I", self.mm, page + 0x3C, 0x80)                 # e_lfanew
            self.mm[page + 0x80:page + 0x84] = b"PE\0\0"
            struct.pack_into("<HH", self.mm, page + 0x84, 0x8664, 0)             # x64, no sections
            synthetic_optional_header(self.mm, page)
        img = self.ld.image
        base = img.image_base if img is not None else 0
        imports = {f"{n.split('!', 1)[0].lower()}!{n.split('!', 1)[1].lower()}": t for t, n in self.ld.thunks.items()}

        def make_thunk(key: str) -> int:
            addr = THUNK_BASE + 16 * len(self.ld.thunks)
            self.ld.thunks[addr] = key
            self.bridges[addr] = self._bridge(key, addr)
            return addr

        def in_app_directory(name: str) -> bool:
            return self.files is not None and any(e.lower() == name for e in os.listdir(self.files.root))

        spec = json.loads(EXPORTS_SPEC.read_text())
        own = {d.name.lower() for d in self.ld.images[1:]}
        spellings: dict[str, str] = {}
        for n in self.ld.thunks.values():
            dll = n.split("!", 1)[0]
            c = dll.lower() if "." in dll else dll.lower() + ".dll"
            if c not in own and c not in ("kernel32.dll", "kernelbase.dll", "ntdll.dll") and c[:-4] in spec["modules"]:
                spellings.setdefault(c, dll)
        if len(spellings) > SYSTEM_IMAGES:
            raise CompileError(f"{len(spellings)} system DLLs imported; the process area holds {SYSTEM_IMAGES}")
        system: dict[str, tuple[int, str]] = {}

        def system_image(c: str, spelling: str) -> int:
            """A synthetic system image, one page with DOS and PE headers, in the next free slot."""
            if len(system) >= SYSTEM_IMAGES:
                raise Unimplemented(f"{c}: the process area holds {SYSTEM_IMAGES} system DLLs")
            off = SYSTEM_IMAGES_OFF + 0x1000 * len(system)
            page = self._process(off)
            self.mm[page:page + 2] = b"MZ"
            struct.pack_into("<I", self.mm, page + 0x3C, 0x80)
            self.mm[page + 0x80:page + 0x84] = b"PE\0\0"
            struct.pack_into("<HH", self.mm, page + 0x84, 0x8664, 0)
            synthetic_optional_header(self.mm, page)
            system[c] = (PROCESS_VA + off, spelling)
            return PROCESS_VA + off

        for c, spelling in sorted(spellings.items()):
            system_image(c, spelling)
        dlls = {d.name.lower(): (d.base, d.pe.size_of_image, {n: d.base + rva for n, rva in d.pe.exports})
                for d in self.ld.images[1:]}
        modules = Modules(image_base=base, image_size=img.size_of_image if img is not None else 0,
                          image_name=image_name, kernel32=k32, kernelbase=kb,
                          image_exports={n: base + rva for n, rva in (img.exports if img is not None else ())},
                          imports=imports, make_thunk=make_thunk, system32=frozenset(spec.get("system32", ())),
                          in_app_directory=in_app_directory, ntdll=ntdll, dlls=dlls,
                          dll_names={d.name.lower(): d.name for d in self.ld.images[1:]}, system=system,
                          load_dll=self._load_dll, load_system=system_image)
        if self.ld.images:                       # a program from loader.load (tests build bare ones)
            self._write_loader_data(modules)
        return modules

    def _write_loader_data(self, m: Modules) -> None:
        """PEB.Ldr as the loader probe's rows have it (contracts/windows.loader.static_import): load and memory
        order the program, ntdll.dll, KERNEL32.DLL, KERNELBASE.dll, then the program's DLLs as loaded;
        initialization order ntdll.dll, KERNELBASE.dll, KERNEL32.DLL, then the DLLs dependencies first.
        Offsets: PEB.Ldr 0x18; PEB_LDR_DATA Length 0, Initialized 4, the three list heads at 0x10, 0x20,
        0x30; LDR_DATA_TABLE_ENTRY links at 0x00, 0x10, 0x20, DllBase 0x30, EntryPoint 0x38, SizeOfImage
        0x40, FullDllName 0x48, BaseDllName 0x58 (UNICODE_STRING: Length, MaximumLength, Buffer at +8)."""
        area, va0 = self._process(LOADER_DATA_OFF), PROCESS_VA + LOADER_DATA_OFF
        system = "C:\\Windows\\System32\\"
        program = (self.ld.images[0].base, self.ld.image_entry, self.ld.image.size_of_image, m.image_path, m.image_name)
        k32 = (m.kernel32, 0, 0x1000, system + "KERNEL32.DLL", "KERNEL32.DLL")
        kb = (m.kernelbase, 0, 0x1000, system + "KERNELBASE.dll", "KERNELBASE.dll")
        nt = (m.ntdll, 0, 0x1000, system + "ntdll.dll", "ntdll.dll")
        dll = {d.name: (d.base, d.entry or 0, d.pe.size_of_image, m.dll_paths.get(d.name.lower(), APP_PATH + d.name),
                        d.name) for d in self.ld.images[1:]}
        entries = [program, nt, k32, kb, *dll.values()]
        init = [nt, kb, k32, *(dll[d.name] for d in self.ld.dependency_order)]
        if 0x58 + LDR_ENTRY_SIZE * len(entries) > LOADER_DATA_SIZE // 2:
            raise CompileError(f"{len(entries)} loader entries: the area holds fewer")
        at = {e[0]: va0 + 0x58 + LDR_ENTRY_SIZE * i for i, e in enumerate(entries)}
        # the names from the area's middle: the list is written again when LoadLibrary maps a DLL (Sprint 063), and
        # an entry or a name the program already holds keeps its address, the order being load order
        strings = LOADER_DATA_SIZE // 2
        for i, (base, entry, size, full, leaf) in enumerate(entries):
            e = area + 0x58 + LDR_ENTRY_SIZE * i
            struct.pack_into("<QQQ", self.mm, e + 0x30, base, entry, size)
            for field_off, text in ((0x48, full), (0x58, leaf)):
                raw = text.encode("utf-16-le")
                if strings + len(raw) + 2 > LOADER_DATA_SIZE:
                    raise CompileError("loader names: the area is full")
                self.mm[area + strings:area + strings + len(raw)] = raw
                struct.pack_into("<HHIQ", self.mm, e + field_off, len(raw), len(raw) + 2, 0, va0 + strings)
                strings += (len(raw) + 2 + 7) & ~7

        def link(head_off: int, link_off: int, order: list[tuple[int, int, int, str, str]]) -> None:
            nodes = [va0 + head_off] + [at[o[0]] + link_off for o in order]
            for i, node in enumerate(nodes):
                off = area + (node - va0)
                struct.pack_into("<QQ", self.mm, off, nodes[(i + 1) % len(nodes)], nodes[i - 1])

        struct.pack_into("<II", self.mm, area, 0x58, 1)
        link(0x10, 0x00, entries)
        link(0x20, 0x10, entries)
        link(0x30, 0x20, init)
        struct.pack_into("<Q", self.mm, self._process(PEB_OFF) + 0x18, va0)

    # ---- threads (Sprint 051; scheduler.py) ------------------------------------------------------------
    def _thread_callbacks(self) -> list[tuple[int, int]]:
        """(module base, function) a thread's start and end call, in order: each DLL whose entry runs (load order,
        then the DLLs loaded after the start), its TLS callbacks and then its DllMain; the program's TLS callbacks
        last. The rows hold one DLL (its TLS callback before its DllMain); the order between modules is the
        loader's init order, not yet observed (contract open question)."""
        program = self.ld.images[0] if self.ld.images else None
        dlls = list(self.ld.init_order) + [i for i in self.ld.images[1:] if i not in self.ld.init_order]
        out: list[tuple[int, int]] = []
        for image in dlls:
            out += [(image.base, image.base + rva) for rva in image.pe.tls_callbacks]
            if image.pe.entry_rva:
                out.append((image.base, image.base + image.pe.entry_rva))
        if program is not None:
            out += [(program.base, program.base + rva) for rva in program.pe.tls_callbacks]
        return out

    def _thread_static_tls(self) -> int:
        """A new thread's ThreadLocalStoragePointer: the pointer array and each module's block, copied from the
        templates as loader._static_tls lays them out for the first thread, an array even when no module has static
        TLS (probe run 38229311268)."""
        images = self.ld.images
        if not images:
            return 0
        blocks: list[bytes | None] = []
        for k, image in enumerate([images[0], *[i for i in images[1:] if i.pe.tls is not None]]):
            if image.pe.tls is None:
                blocks.append(None)                       # the program's slot, when it has no TLS
                continue
            start, end, _, zero_fill = (v - image.pe.image_base + image.base if j < 3 else v
                                        for j, v in enumerate(image.pe.tls))
            blocks.append(bytes(self.mem.read(start, end - start)) + bytes(zero_fill))
        array = 8 * TLS_SLOTS                             # room for DLLs loaded later (_late_static_tls)
        size = array + sum((len(b) + 15) & ~15 for b in blocks if b is not None)
        base, err = self.virtual.alloc(0, size, MEM_COMMIT | MEM_RESERVE, 0x04)
        if err:
            raise Unimplemented(f"a thread's static TLS: VirtualAlloc error {err}")
        at = array
        for index, block in enumerate(blocks):
            if block is None:
                continue
            self.mem.write(base + 8 * index, (base + at).to_bytes(8, "little"))
            self.mem.write(base + at, block)
            at += (len(block) + 15) & ~15
        return base

    def _code_page(self, code: bytes) -> int:
        """x86 code the host writes for a thread (its attach or detach stub), on fresh pages of the arena: an address
        never translated before, so no stale translation can run it."""
        va, err = self.virtual.alloc(0, len(code), MEM_COMMIT | MEM_RESERVE, PAGE_EXECUTE_READWRITE)
        if err:
            raise Unimplemented(f"a thread stub: VirtualAlloc error {err}")
        self.mem.write(va, code)
        return va

    def _guest_stubs(self) -> int:
        """The x86 stubs services call guest code and resume contexts through (guest_stubs.py), on a
        page of the arena written at the first use, so a process that needs none keeps its address space."""
        if not self._stub_page:
            self._stub_page = self._code_page(guest_stubs.page(self._import_thunk("KERNEL32.dll!W2mGuestReturn")))
        return self._stub_page

    def _com_vtables(self) -> None:
        """Each COM class's vtable (com.c, dxgi.c, in the runtime image) filled with its methods' thunks: the services
        MMDEVAPI!W2mEnumerator<Method> and W2mCollection<Method> (Sprint 052), dxgi!W2mFactory<Method>,
        W2mAdapter<Method> and W2mOutput<Method> (Sprint 063)."""
        for symbol, (dll, prefix, methods) in COM_VTABLES.items():
            at = self.runtime.symbols[symbol] - MONITOR_VA
            for i, method in enumerate(methods):
                struct.pack_into("<Q", self.mm, at + 8 * i, self._import_thunk(f"{dll}!{prefix}{method}"))

    def _import_thunk(self, name: str) -> int:
        key = name.lower()
        m = self.modules
        va = m.imports.get(key) or m.make_thunk(name)
        m.imports[key] = va
        return va

    def _create_thread(self, start: int, parameter: int, flags: int, stack_size: int) -> int:
        """CreateThread's host half: a thread id, a TEB, a stack, static TLS, the attach stub and the first state."""
        tid = max(self.threads) + 4
        program = self.ld.images[0].pe if self.ld.images else None
        # STACK_SIZE_PARAM_IS_A_RESERVATION: the size asked for, smaller than the image's or not (row reserve.teb:
        # 64 pages for 0x40000); otherwise the image's SizeOfStackReserve
        reserve = max(stack_size if flags & 0x10000 and stack_size else
                      (program.stack_reserve if program is not None else 0x100000), 0x10000)
        reserve = (reserve + 0xFFFF) & ~0xFFFF
        dealloc, err = self.virtual.alloc(0, reserve, MEM_RESERVE, 0x04)
        if err:
            raise Unimplemented(f"a thread's stack of 0x{reserve:x} bytes: VirtualAlloc error {err}")
        # a created thread's stack as Windows lays it out (contracts/windows.memory.created_thread_stack, Sprint 066):
        # the initial commit at the top (the requested size, else the image's SizeOfStackCommit), three PAGE_GUARD
        # pages below StackLimit, the rest reserved; the guard and reserved pages are unmapped, and a fault in the
        # guard commits on demand (_stack_grow), one below it is an access violation
        top = dealloc + reserve
        commit = stack_size if stack_size and not flags & 0x10000 else (program.stack_commit if program else 0x1000)
        commit = min(max((commit + 0xFFF) & ~0xFFF, 0x1000), reserve - (2 + STACK_GUARD_PAGES) * 0x1000)
        limit = top - commit
        self.virtual.alloc(limit, commit, MEM_COMMIT, 0x04)
        r = self.virtual.reservations[dealloc]
        self.virtual._commit(r, limit - STACK_GUARD_PAGES * 0x1000, STACK_GUARD_PAGES * 0x1000, 0x04 | PAGE_GUARD)
        self._map_pages(dealloc, limit, False)
        teb, err = self.virtual.alloc(0, 0x4000, MEM_COMMIT | MEM_RESERVE, 0x04)     # TEB, then TlsExpansionSlots
        if err:
            raise Unimplemented(f"a thread's TEB: VirtualAlloc error {err}")
        peb = PROCESS_VA + PEB_OFF
        for off, value in ((0x08, top), (0x10, limit), (0x30, teb), (0x40, self.process_id), (0x48, tid),
                           (0x58, self._thread_static_tls()), (0x60, peb), (0x1478, dealloc), (0x1780, teb + 0x2000)):
            self.mem.write(teb + off, value.to_bytes(8, "little"))
        stub = self._code_page(attach_stub(self._thread_callbacks(), start, parameter,
                                           self._import_thunk("KERNEL32.dll!ExitThread")))
        x = [0] * 31
        x[4] = top - 0x48                                              # RSP = 8 mod 16, as at a function's entry
        x[17] = stub
        x[REG_UNDEFINED_FLAGS] = 0x3F
        x[28] = MONITOR_VA + MONITOR_DATA
        monitor = []
        for a, b in PER_THREAD_MONITOR:
            data = bytearray(b - a)
            if a == MXCSR_OFF:
                data[0:4] = (0x1F80).to_bytes(4, "little")
            if a == FCW_OFF:
                data[0:2] = (0x27F).to_bytes(2, "little")
            monitor.append(bytes(data))
        state = bytearray(STATE_SIZE)
        struct.pack_into("<I", state, 152, 0x1F80)
        struct.pack_into("<H", state, 156, 0x27F)
        struct.pack_into("<QQ", state, 176, teb, teb)
        t = GuestThread(tid, teb, (dealloc, limit, top), x=x, fpcr=fpcr_for(0x1F80), monitor=monitor,
                        state=bytes(state), suspend=1 if flags & 0x4 else 0, parent=self.current_tid, start=start,
                        created_suspended=bool(flags & 0x4))
        self.threads[tid] = t
        self._write_vectors(t, [0] * 16, t.fpcr, 0)       # as a Windows thread starts: XMM clear, MXCSR 0x1F80
        self.virtual.stacks.append(t.stack)
        self._set_budget()                                # two threads: time slices from now on
        return tid

    def _map_pages(self, start: int, end: int, valid: bool) -> None:
        """Map or unmap the pages [start, end) in the stage-1 tables (the descriptor's valid bit). The guest runtime
        flushes the TLB after the request that changed them."""
        for page in range(start, end, PAGE):
            at = self._page_descriptor(page)
            if at:
                desc = struct.unpack_from("<Q", self.mm, at)[0]
                struct.pack_into("<Q", self.mm, at, desc | 1 if valid else desc & ~1)

    def _stack_grow(self, address: int, probe: bool) -> int:
        """Commit the current thread's stack down to `address`'s page, as Windows does on a touch of its guard: the
        touched guard page and those above it committed read-write, StackLimit moved to it, three guard pages below
        it (contracts/windows.memory.created_thread_stack rows default.grow, default.grown.*). Only a page in the
        guard grows the stack; `probe` (the runtime placing a frame below StackLimit) grows it page by page as
        successive touches would. 0 when the address is not reached this way: an access violation (row skip)."""
        cur = self.threads[self.current_tid]
        dealloc, _, top = cur.stack
        r = self.virtual.reservations.get(dealloc)
        if r is None:              # the main thread's stack (the process area's TEB), committed whole: not grown here
            return 0
        limit = int.from_bytes(self._read_guest(cur.teb + 0x10, 8), "little")
        page = address & ~(PAGE - 1)
        if not dealloc <= page < limit:
            return 0
        if not probe and page < limit - STACK_GUARD_PAGES * PAGE:
            return 0
        while limit > page:
            new = limit - PAGE if probe else page
            if new - STACK_GUARD_PAGES * PAGE < dealloc + PAGE:      # the guard would reach the reservation's last page
                return 0
            self.virtual._commit(r, new, limit - new, 0x04)
            self._map_pages(new, limit, True)
            self.virtual._commit(r, new - STACK_GUARD_PAGES * PAGE, STACK_GUARD_PAGES * PAGE, 0x04 | PAGE_GUARD)
            limit = new
        self.mem.write(cur.teb + 0x10, limit.to_bytes(8, "little"))
        cur.stack = (dealloc, limit, top)
        return 1

    def _thread_request(self, sub: int, a0: int, a1: int, a2: int) -> int:
        """threads.c's W2M_THREAD_* requests."""
        if sub == THREAD_CREATE:
            return self._create_thread(a0, a1, a2 & 0xFFFFFFFF, a2 >> 32)
        if sub == THREAD_STACK_GROW:
            return self._stack_grow(a0, bool(a1))
        cur = self.threads[self.current_tid]
        if sub == THREAD_EXIT_STUB:
            return self._code_page(detach_stub(self._thread_callbacks(), self._import_thunk("KERNEL32.dll!W2mThreadEnd")))
        if sub == THREAD_END:
            cur.exit_code = a0 & 0xFFFFFFFF
            return 0
        if sub == THREAD_SIGNAL:                         # a0: the key signaled (threads.c w2m_signal; 0 any)
            for t in self.threads.values():
                if t.status == "blocked" and (not a0 or not (key := self._wait_key(t)) or key == a0):
                    t.status = "ready"
            self._publish_blocked()
            return 0
        found = self.threads.get(a0 & 0xFFFFFFFF)
        if found is None:
            raise Unimplemented(f"a thread request ({sub}) for thread id {a0 & 0xFFFFFFFF}, which w2m did not create")
        t = found
        if sub == THREAD_EXIT_CODE:
            return t.exit_code
        if sub == THREAD_SUSPEND:
            if t.status == "ended":
                return M64
            t.suspend += 1
            return t.suspend - 1
        if sub == THREAD_RESUME:
            if t.status == "ended" or t.suspend == 0:
                return 0
            t.suspend -= 1
            return t.suspend + 1
        if sub == THREAD_CONTEXT:
            buf = bytearray(self.mem.read(a1, 1232))
            flags = struct.unpack_from("<I", buf, 48)[0]
            mon = dict(zip((a for a, _ in PER_THREAD_MONITOR), t.monitor))
            af_a, af_b, af_r = struct.unpack_from("<QQQ", mon[0x20], 0)
            undefined = t.x[REG_UNDEFINED_FLAGS]
            eflags = 0x202 | (mon[0x20][DF_BYTE - 0x20] & 1) << 10
            for f, bit in (("C", 0), ("Z", 6), ("S", 7), ("O", 11)):
                if not undefined >> FLAG_BIT[f] & 1 and t.x[FLAG_REG[f]]:
                    eflags |= 1 << bit
            if not undefined >> FLAG_BIT["P"] & 1 and parity_even(af_r):          # PF lazy (PF_RESULT)
                eflags |= 1 << 2
            if not undefined >> FLAG_BIT["A"] & 1 and (af_a ^ af_b ^ af_r) & 0x10:
                eflags |= 1 << 4
            # a thread suspended in a wait answers RIP in ntdll, where Windows' NtWait* stub returns from its syscall
            # (contracts/windows.thread.running_context, context.waiting); w2m parks it at the bridge thunk, in no
            # module. The address is inside ntdll's synthetic page, which holds no code: nothing resumes there, as w2m
            # has no SetThreadContext and a thread resumes from its own saved state.
            rip = self.modules.ntdll + 0x800 if t.status == "blocked" and self.modules.ntdll else t.x[17]
            write_context(buf, flags, t.x[:16], rip, eflags, struct.unpack_from("<I", mon[MXCSR_OFF], 0)[0],
                          struct.unpack_from("<H", mon[FCW_OFF], 0)[0], self.thread_vectors(t)[0])
            self.mem.write(a1, bytes(buf))
            return 1
        raise Unimplemented(f"thread request {sub}")

    def _set_budget(self) -> None:
        """The step budget the block entries check: the run's limit, or with more than one thread a time slice."""
        executed = struct.unpack_from("<Q", self.mm, MONITOR_DATA)[0]
        shared = len(self.threads) > 1 or (self.machine is not None and len(self.machine.vms) > 1)
        limit = min(self._max_steps, executed + TIME_SLICE) if shared else self._max_steps
        struct.pack_into("<Q", self.mm, MONITOR_DATA + 8, limit)

    def _wait_key(self, t: GuestThread) -> int:
        """What blocked thread t waits on (w2m_block's key in its TEB scratch): 0 any signal, ~0 none."""
        return int(struct.unpack("<Q", self._read_guest(t.teb + SCRATCH_OFF + WAIT_KEY_OFF, 8))[0])

    def _read_guest(self, va: int, n: int) -> bytes:
        """Guest memory, the process area (the first thread's TEB, the PEB) included."""
        off = va - PROCESS_VA
        if 0 <= off and off + n <= PROCESS_SIZE:
            return bytes(self.mm[self._process(off):self._process(off) + n])
        return bytes(self.mem.read(va, n))

    def _publish_blocked(self) -> None:
        """The number of blocked threads and the keys they wait on, where threads.c's w2m_signal reads them: a key
        no blocked thread waits on, with none waiting on any key, costs no exit."""
        blocked = [t for t in self.threads.values() if t.status == "blocked"]
        keys = sorted({k for k in map(self._wait_key, blocked) if k})
        anywait = sum(1 for t in blocked if not self._wait_key(t)) + (len(keys) > BLOCKED_KEYS)
        struct.pack_into("<Q", self.mm, self.runtime.symbols["w2m_threads_blocked"] - MONITOR_VA, len(blocked))
        struct.pack_into("<Q", self.mm, self.runtime.symbols["w2m_blocked_any"] - MONITOR_VA, anywait)
        table = (keys + [0] * BLOCKED_KEYS)[:BLOCKED_KEYS]
        struct.pack_into(f"<{BLOCKED_KEYS}Q", self.mm, self.runtime.symbols["w2m_blocked_keys"] - MONITOR_VA, *table)

    def _vslot(self, t: GuestThread) -> int:
        """The thread's vector slot (VECTOR_SLOTS), given out on first use."""
        if not t.vslot:
            if self._vslots_used == VECTOR_SLOT_COUNT:
                raise Unimplemented(f"more than {VECTOR_SLOT_COUNT} threads in one process")
            t.vslot = VECTOR_SLOTS + VECTOR_SLOT * self._vslots_used
            self._vslots_used += 1
        return t.vslot

    def _write_vectors(self, t: GuestThread, xmm: list[int], fpcr: int, fpsr: int) -> None:
        at = self._vslot(t)
        for i, q in enumerate(xmm):
            self.mm[at + 16 * i:at + 16 * i + 16] = q.to_bytes(16, "little")
        struct.pack_into("<QQ", self.mm, at + 256, fpcr, fpsr)

    def thread_vectors(self, t: GuestThread) -> tuple[list[int], int, int]:
        """A thread's XMM0-15, FPCR and FPSR: the vCPU's for the running thread, else its slot (vm_monitor.c vswap
        wrote it at the switch's `set`)."""
        if t.tid == self.current_tid:
            return self.vectors()
        at = self._vslot(t)
        xmm = [int.from_bytes(self.mm[at + 16 * i:at + 16 * i + 16], "little") for i in range(16)]
        fpcr, fpsr = struct.unpack_from("<QQ", self.mm, at + 256)
        return xmm, int(fpcr), int(fpsr)

    def _counts(self) -> tuple[int, int]:
        executed, = struct.unpack_from("<Q", self.mm, MONITOR_DATA)
        pauses, = struct.unpack_from("<Q", self.mm, MONITOR_DATA + PAUSE_COUNT)
        return int(executed), int(pauses)

    def thread_census(self) -> dict[int, tuple[int, int]]:
        """Each thread's instructions and PAUSEs so far (the spin census, Sprint 068 item J): the totals at its switches,
        and the running thread's since it last got the vCPU."""
        if self._final_census is not None:              # closed: the census as it stood at close
            return dict(self._final_census)
        executed, pauses = self._counts()
        out = {tid: (t.instructions, t.pauses) for tid, t in self.threads.items()}
        if self.current_tid in out:
            n, p = out[self.current_tid]
            out[self.current_tid] = (n + executed - self._slice_start[0], p + pauses - self._slice_start[1])
        return out

    def _save_thread(self, t: GuestThread, x: list[int]) -> None:
        executed, pauses = self._counts()
        t.instructions += executed - self._slice_start[0]
        t.pauses += pauses - self._slice_start[1]
        self._slice_start = (executed, pauses)
        t.x = list(x)
        self._vswap = (self._vslot(t), 0)                # the monitor saves its vectors at the next `set`
        t.monitor = [bytes(self.mm[MONITOR_DATA + a:MONITOR_DATA + b]) for a, b in PER_THREAD_MONITOR]
        st = self._process(STATE_OFF)
        t.state = bytes(self.mm[st:st + STATE_SIZE])

    def _load_thread(self, t: GuestThread) -> list[int]:
        for (a, b), data in zip(PER_THREAD_MONITOR, t.monitor):
            self.mm[MONITOR_DATA + a:MONITOR_DATA + b] = data
        st = self._process(STATE_OFF)
        self.mm[st:st + STATE_SIZE] = t.state
        self._slice_start = self._counts()
        save = self._vswap[0] if self._vswap is not None else 0
        # the same thread back (another VM ran meanwhile, on its own vCPU): its vectors never left this one
        self._vswap = None if save == self._vslot(t) else (save, self._vslot(t))
        return list(t.x)

    def _schedule(self, x: list[int], schedule: int) -> list[int] | None:
        """Hand the vCPU from the current thread (its state in x, at an instruction boundary) to the next that can
        run: round robin over ready, unsuspended threads; when none is ready, a thread blocked with a finite deadline
        runs to check it (after a millisecond's sleep, so the clock moves); with neither, every thread waits
        forever and the run stops. `schedule` is state.schedule's bits (0: the time slice ran out)."""
        cur = self.threads[self.current_tid]
        if schedule & SCHEDULE_END:
            cur.status = "ended"
            for t in self.threads.values():              # its handle signals: waiters check again
                if t.status == "blocked":
                    t.status = "ready"
        elif schedule & SCHEDULE_RETRY:
            cur.status = "blocked"
            cur.deadline = struct.unpack_from("<Q", self._read_guest(cur.teb + SCRATCH_OFF, 8))[0]
        if all(t.status == "ended" for t in self.threads.values()):
            return None                                       # the last thread ended: the process ends with it
        return self._switch(x, cur, schedule)

    def _switch(self, x: list[int], cur: GuestThread, schedule: int) -> list[int] | None:
        """The next thread's state after cur's (x): IDLE, in a machine of several VMs, when none of this VM's
        threads can run (the driver runs another VM and resumes this one when a thread is ready again)."""
        order = sorted(self.threads)
        i = order.index(cur.tid)
        rotation = order[i + 1:] + order[:i + 1]
        nxt = next((self.threads[k] for k in rotation
                    if self.threads[k].status == "ready" and self.threads[k].suspend == 0), None)
        if nxt is None:
            timed = [self.threads[k] for k in rotation if self.threads[k].status == "blocked"
                     and self.threads[k].suspend == 0 and self.threads[k].deadline != M64]
            if self.machine is not None:
                self._idle = (cur, schedule, bool(timed))
                return IDLE
            if not timed:
                raise Unimplemented("every thread waits with no timeout and nothing can wake one (a deadlock)")
            if self.warp_to_deadline():
                nxt = next(t for t in (self.threads[k] for k in rotation) if t.status == "ready")
            else:
                time.sleep(0.001)
                nxt = timed[0]
                nxt.status = "ready"
        if schedule & SCHEDULE_ANSWER:
            x[0] = int(nxt is not cur)                       # SwitchToThread: nonzero when another thread runs
        self._publish_blocked()
        if nxt is cur:
            return x
        self._save_thread(cur, x)
        self.current_tid = nxt.tid
        return self._load_thread(nxt)

    def read_bytes(self, va: int, n: int) -> bytes:
        return _GuestView(self).read(va, n) if n else b""

    def write_bytes(self, va: int, data: bytes) -> None:
        _GuestView(self).write(va, data)

    def read_text(self, va: int, chars: int) -> str:
        return self.read_bytes(va, 2 * chars).decode("utf-16-le", errors="replace")

    def environment_now(self) -> tuple[str, ...]:
        """The environment block's entries as the guest holds them now (ProcessParameters.Environment, which
        environment.c moves on each change): what a child started with no environment of its own gets (kernel.py
        PROCESS_CREATE; Sprint 058)."""
        at = struct.unpack("<Q", self.mem.read(PARAMETERS_VA + 0x80, 8))[0]
        off = at - PROCESS_VA                            # environment.c keeps it in its area of the process area
        if not ENVIRONMENT_OFF <= off <= ENVIRONMENT_OFF + ENVIRONMENT_SIZE // 2:
            raise Unimplemented(f"ProcessParameters.Environment at 0x{at:x}, outside its area")
        block = bytes(self.mm[self._process(off):self._process(off) + ENVIRONMENT_SIZE // 2])
        units = block.decode("utf-16-le")
        end = units.find("\0\0")
        if end < 0:
            raise Unimplemented("the environment block has no end within its area")
        return tuple(e for e in units[:end].split("\0") if e)

    def warp_to_deadline(self) -> bool:
        """With the instruction clock and no thread ready: move the clock to the earliest deadline of a blocked thread
        and make that thread ready, as QEMU's -icount sleep=off does; False when no thread waits with a deadline."""
        timed = [t for t in self.threads.values() if t.status == "blocked" and t.suspend == 0 and t.deadline != M64]
        if not timed or not self.instruction_clock:
            return False
        first = min(timed, key=lambda t: (t.deadline, t.tid))
        rate = self.instruction_clock[0]
        executed, warp = struct.unpack_from("<Q", self.mm, MONITOR_DATA)[0], self._clock_warp()
        now = executed // rate * 10_000_000 + executed % rate * 10_000_000 // rate + warp
        if first.deadline > now:
            struct.pack_into("<Q", self.mm, MONITOR_DATA + CLOCK_WARP, warp + first.deadline - now)
        first.status = "ready"
        return True

    def _clock_warp(self) -> int:
        return int(struct.unpack_from("<Q", self.mm, MONITOR_DATA + CLOCK_WARP)[0])

    def _resume_idle(self, x: list[int]) -> list[int] | None:
        """After the driver resumed an idle VM: a thread made ready meanwhile runs, or a blocked thread with a
        deadline runs to check it."""
        assert self._idle is not None
        cur, schedule, _ = self._idle
        self._idle = None
        if not any(t.status == "ready" and t.suspend == 0 for t in self.threads.values()) \
                and not self.warp_to_deadline():
            timed = [t for t in self.threads.values() if t.status == "blocked" and t.suspend == 0 and t.deadline != M64]
            if timed:
                timed[0].status = "ready"
        return self._switch(x, cur, schedule)

    def ready(self) -> bool:
        """Whether a thread of this VM can run now (kernel.Machine): a process not yet started can."""
        return not self.threads or any(t.status == "ready" and t.suspend == 0 for t in self.threads.values())

    def waits_with_deadline(self) -> bool:
        return any(t.status == "blocked" and t.suspend == 0 and t.deadline != M64 for t in self.threads.values())

    def wake_all(self) -> None:
        """A shared object was signaled (kernel.py): every blocked thread checks its wait again."""
        for t in self.threads.values():
            if t.status == "blocked":
                t.status = "ready"
        self._publish_blocked()

    def _bridge(self, import_name: str, thunk: int) -> int:
        """AArch64 stub for one import: save the guest GPRs in the state block, pass RCX, RDX, R8,
        R9 and RSP in x0-x4, call the C service on the runtime stack, put its result in RAX unless
        the service is void, restore the GPRs, and do RET's pop (Microsoft x64 calling convention;
        AAPCS64 keeps x19-x28, so the flag registers survive). An import with no service exits
        with hvc #9. Sprint 051: a service may set state.redirect_rip (the next x86 address instead of the
        return; RSP keeps the return address) and state.schedule: RETRY leaves the call unreturned with RIP at
        `thunk`, so the thread re-enters it; any schedule bit exits with hvc #12 to the scheduler."""
        dll, fn = import_name.split("!", 1)
        a = Assembler(MONITOR_VA + self.code_cursor)
        service = service_for(dll, fn)
        state = PROCESS_VA + STATE_OFF
        if service is None:
            a.emit(a64.hvc(9))
        else:
            symbol, sets_rax = service
            a.emit(a64.mov_imm(16, state))
            for i in range(0, 16, 2):
                a.emit(a64.stp(i, i + 1, 16, 8 * i))
            a.emit(a64.mov(0, 1), a64.mov(1, 2), a64.mov(2, 8), a64.mov(3, 9))   # x4 = RSP already
            a.emit(a64.mov_imm(16, MONITOR_VA + RUNTIME_STACK + RUNTIME_STACK_SIZE), a64.addi(31, 16, 0))
            # call_trace.c records the call around the service; w2m's own COM methods (MMDEVAPI!W2m*, reached through
            # a vtable, not an import) have no hook in the capture
            if self.call_trace and fn not in UNTRACED and not fn.startswith("W2m"):
                a.emit(a64.mov_imm(5, self.runtime.symbols[symbol]), a64.mov_imm(6, (thunk - THUNK_BASE) // 16))
                if fn in TRACE_WIDE_ARGS and (thunk - THUNK_BASE) // 16 < TRACE_THUNKS:
                    self.mm[self._process(TRACE_OFF) + TRACE_WIDE_OFF + (thunk - THUNK_BASE) // 16] = TRACE_WIDE_ARGS[fn]
                symbol = "w2m_traced"
            a.emit(*count_from_budget())
            a.emit(a64.mov_imm(16, self.runtime.symbols[symbol]), a64.blr(16))
            a.emit(*budget_from_count())
            a.emit(a64.mov_imm(16, state))
            if sets_rax:
                a.emit(a64.str_(8, 0, 16, 0))
            for i in range(0, 16, 2):
                a.emit(a64.ldp(i, i + 1, 16, 8 * i))
            not_retry, pop, after, to_host = a.label(), a.label(), a.label(), a.label()
            a.emit(a64.ldr(4, 17, 16, STATE_SCHEDULE))
            a.branch(lambda off: a64.tbz(17, 1, off), not_retry)
            a.emit(a64.mov_imm(17, thunk))                            # RETRY: RIP back at the call, RSP as it is
            a.branch(a64.b, to_host)
            a.bind(not_retry)
            a.emit(a64.ldr(8, 17, 16, STATE_REDIRECT))
            a.branch(lambda off: a64.cbz(17, off), pop)
            a.emit(a64.str_(8, a64.ZR, 16, STATE_REDIRECT))
            a.branch(a64.b, after)
            a.bind(pop)
            a.emit(a64.ldr(8, 17, 4, 0), a64.addi(4, 4, 8))
            a.bind(after)
            a.emit(a64.ldr(4, 16, 16, STATE_SCHEDULE))
            a.branch(lambda off: a64.cbnz(16, off), to_host)
            a.branch(a64.b, self.miss_va if self.lockstep else self.dispatch_va)
            a.bind(to_host)
            a.emit(a64.hvc(HVC_SCHEDULE))
        words = a.finish()
        va = MONITOR_VA + self.code_cursor
        self._write_words(self.code_cursor, words)
        self.code_cursor += 4 * len(words)
        self._recon_append([(va, 0)])                   # a fault here is the runtime's, not the guest's
        return va

    def _host_request(self, x: list[int]) -> bool:
        """Service the transport page (hvc #8). False when the guest exited. A request the host cannot serve (an
        API set with no observed host module, a file call with no root) stops the run as an unserved service does,
        with the guest's return address (Sprint 060: the frontier reports it as SERVICE_UNIMPLEMENTED)."""
        try:
            return self._serve_host_request(x)
        except Unimplemented as e:
            if self.unimplemented is None:
                rsp = struct.unpack_from("<Q", self.mm, self._process(STATE_OFF) + 8 * 4)[0]
                self.unimplemented = (str(e), int.from_bytes(self.mem.read(rsp, 8), "little"))
            raise

    def _serve_host_request(self, x: list[int]) -> bool:
        t = self._process(TRANSPORT_OFF)
        op, a0, a1 = struct.unpack_from("<QQQ", self.mm, t)
        self.host_requests[op] = self.host_requests.get(op, 0) + 1
        if FILE_LOOKUP <= op <= FILE_UNLINK or op in (FILE_STAT, FILE_MKDIR, FILE_RMDIR, FILE_FSTAT, FILE_DUP, FILE_LIST):
            if self.files is None:
                raise Unimplemented("a file call in a run with no file-system root")
            result, second = self.files.request(_GuestView(self), op, list(struct.unpack_from("<6Q", self.mm, t + 8)))
            struct.pack_into("<QQ", self.mm, t + 48, second, result)
            return True
        if op == HOST_KERNEL:
            if self.machine is None:
                raise Unimplemented("an object other processes share (a named event, a pipe, a process): this run "
                                    "has no kernel.Machine")
            result, second = self.machine.request(self, list(struct.unpack_from("<4Q", self.mm, t + 8)))
            struct.pack_into("<QQ", self.mm, t + 48, second, result)
            return True
        if op == HOST_WRITE:
            self.out += self.mem.read(a0, a1)
            result = a1
        elif op == HOST_EXIT:
            self.exit_code = a0 & 0xFFFFFFFF
            return False
        elif op == HOST_CLOCK:
            result = self.clock() & M64
        elif op == HOST_RANDOM:
            self.mem.write(a1, self._random.randbytes(a0) if self._random else os.urandom(a0))
            result = a0
        elif op == HOST_FILETIME:
            result = time.time_ns() // 100 + FILETIME_UNIX_EPOCH
        elif op == HOST_THREAD:
            a2, a3 = struct.unpack_from("<QQ", self.mm, t + 24)
            result = self._thread_request(a0, a1, a2, a3) & M64
        elif op == HOST_EXCEPTION:
            if a0 == EXCEPTION_STUBS:
                result = self._guest_stubs()
            elif a0 == EXCEPTION_CAPTURE_XMM:
                xmm, _, _ = self.vectors()
                self.mem.write(a1, b"".join(q.to_bytes(16, "little") for q in xmm))
                result = 0
            else:
                raise CompileError(f"guest runtime: unknown exception request {a0}")
        elif op == HOST_MAP_VIEW:                       # the view's pages, then the file's bytes copied in
            a2 = struct.unpack_from("<Q", self.mm, t + 24)[0]
            if self.files is None or a1 not in self.files.fds:
                raise Unimplemented(f"a file view of descriptor {a1}, which is not open")
            base = self.virtual.map_view(a0, a2)
            self.mem.write(base, os.pread(self.files.fds[a1], a0, 0))
            struct.pack_into("<QQ", self.mm, t + 48, 0, base)
            return True
        elif op in (HOST_VIRTUAL_ALLOC, HOST_VIRTUAL_FREE, HOST_VIRTUAL_PROTECT, HOST_VIRTUAL_QUERY):
            a2, a3 = struct.unpack_from("<QQ", self.mm, t + 24)
            result, second = self._virtual_request(op, a0, a1, a2, a3)
            struct.pack_into("<QQ", self.mm, t + 48, second, result)
            return True
        elif op in (HOST_MODULE, HOST_PROC, HOST_MODULE_PATH, HOST_MODULE_AT):
            a2 = struct.unpack_from("<Q", self.mm, t + 24)[0]
            result = self._module_request(op, a0, a1, a2)
            struct.pack_into("<Q", self.mm, t + 48, self._pending_init)      # a DLL just loaded: its DllMain stub
            self._pending_init = 0
        elif op == HOST_RUNTIME_FAULT:
            raise CompileError(f"guest runtime fault: ELR 0x{a0:x} ESR 0x{a1:x} "
                               f"(EC 0x{a1 >> 26 & 0x3F:x}), not in translated code or not a data abort")
        elif op == HOST_UNIMPLEMENTED:
            name = guest_runtime.api_ids().get(a0, f"api {a0}")
            # the bridge saved the guest's GPRs in the state block; RSP (slot 4) holds the call's return address
            rsp = struct.unpack_from("<Q", self.mm, self._process(STATE_OFF) + 8 * 4)[0]
            self.unimplemented = (f"{name} argument form 0x{a1:x}", int.from_bytes(self.mem.read(rsp, 8), "little"))
            raise Unimplemented(f"{name} argument form 0x{a1:x}")
        else:
            raise CompileError(f"guest runtime: unknown host request {op}")
        struct.pack_into("<Q", self.mm, t + 56, result)
        return True

    def _module_request(self, op: int, a0: int, a1: int, a2: int) -> int:
        view = _GuestView(self)
        m = self.modules
        if op == HOST_MODULE:
            return m.module(view.read(a0, 2 * a1).decode("utf-16-le"), loading=bool(a2))
        if op == HOST_PROC:
            return self._traced_proc(m.proc(a0, view.read(a1, a2).decode("latin-1")))
        if op == HOST_MODULE_AT:
            return m.containing(a0)
        path = m.file_name(a0)                                  # HOST_MODULE_PATH: buffer a1, a2 characters
        if path is None:
            return M64
        raw = path.encode("utf-16-le")[:2 * a2]
        if a2:
            view.write(a1, raw)
        return len(path)

    def _load_dll(self, name: str) -> int:
        """LoadLibrary of a DLL file after the start (Sprint 049): the file the name finds (a full path, or a name
        in the program's directory) is mapped into the VirtualAlloc arena at its preferred base or relocated
        there, its imports bound (the program's DLLs by export, every other DLL through a thunk), and it joins
        the module table under its full path. Its DllMain(base, DLL_PROCESS_ATTACH, NULL) is left to a stub the
        service returns through (`_init_stub`), so the guest runs it as the caller's next code. 0: no file."""
        if self.files is None:
            return 0
        host = self.files.host_file(name)
        if host is None:
            return 0
        raw = host.read_bytes()
        pe = parse_pe(raw)
        if pe.tls_callbacks:                            # no row shows a late DLL's TLS callbacks
            raise Unimplemented(f"LoadLibrary {name}: TLS callbacks in a DLL loaded after the start")
        base = self.virtual.place_image(pe)
        staging = Memory()
        map_image(staging, raw, pe, base, host.name)
        self.mem.write(base, bytes(staging.regions[0].data))
        m = self.modules
        for slot, target in sorted(pe.iat.items()):
            dll, fn = target.split("!", 1)
            leaf = _canonical_dll(dll)
            if leaf in m.dlls:
                va = m.dlls[leaf][2].get(fn)
                if va is None:
                    raise LoadError(f"{host.name} imports {target}, which {leaf} does not export")
            else:
                key = f"{dll.lower()}!{fn.lower()}"
                va = m.imports.get(key) or m.make_thunk(target)
                m.imports[key] = va
            self.mem.write(base + slot, va.to_bytes(8, "little"))
        if pe.tls is not None:
            self._late_static_tls(pe, base)
        self.virtual.snapshot_image(base, pe)
        c = host.name.lower()
        m.dlls[c] = (base, pe.size_of_image, {n: base + rva for n, rva in pe.exports})
        m.dll_names[c] = host.name
        root = self.files.root
        m.dll_paths[c] = APP_PATH + "\\".join(host.relative_to(root).parts) if root in host.parents else str(host)
        self.ld.images.append(Image(host.name, pe, base))
        self._write_loader_data(m)                      # PEB.Ldr lists it, as EnumProcessModules reads (Sprint 063)
        self._trace_ranges()
        if pe.entry_rva:
            self._pending_init = self._init_stub(base, base + pe.entry_rva)
        return base

    def _late_static_tls(self, pe: Any, base: int) -> None:
        """Static TLS for a DLL loaded after the start, as probe run 38229055834 records Windows doing it: the DLL takes
        the next index (written at its AddressOfIndex), and every thread of the process, those started before the load
        and the loading one, gets its own block copied from the template with the zero fill, in the slot of a pointer
        array that does not move (each has room for loader.TLS_SLOTS modules). DllMain then runs on the loading thread
        with its block in place; threads started later get the block from _thread_static_tls."""
        index = 1 + sum(1 for i in self.ld.images[1:] if i.pe.tls is not None)
        if index >= TLS_SLOTS:
            raise Unimplemented(f"static TLS index {index}: each thread's array holds {TLS_SLOTS}")
        start, end, index_va, zero_fill = (v - pe.image_base + base if k < 3 else v for k, v in enumerate(pe.tls))
        self.mem.write(index_va, index.to_bytes(4, "little"))
        block = bytes(self.mem.read(start, end - start)) + bytes(zero_fill)
        for t in self.threads.values():
            if t.status == "ended":
                continue
            array = int.from_bytes(self._read_guest(t.teb + 0x58, 8), "little")
            if array == 0:                              # no module had static TLS: the thread's first array
                array, err = self.virtual.alloc(0, 8 * TLS_SLOTS, MEM_COMMIT | MEM_RESERVE, 0x04)
                if err:
                    raise Unimplemented(f"a thread's TLS array: VirtualAlloc error {err}")
                self._write_guest(t.teb + 0x58, array.to_bytes(8, "little"))
            va, err = self.virtual.alloc(0, max(len(block), 16), MEM_COMMIT | MEM_RESERVE, 0x04)
            if err:
                raise Unimplemented(f"a thread's TLS block: VirtualAlloc error {err}")
            self.mem.write(va, block)
            self.mem.write(array + 8 * index, va.to_bytes(8, "little"))

    def _write_guest(self, va: int, data: bytes) -> None:
        """Guest memory, the process area (the first thread's TEB, the PEB) included."""
        off = va - PROCESS_VA
        if 0 <= off and off + len(data) <= PROCESS_SIZE:
            self.mm[self._process(off):self._process(off) + len(data)] = data
        else:
            self.mem.write(va, data)

    def _init_stub(self, base: int, entry: int) -> int:
        """x86 that LoadLibrary returns through (module.c writes the caller's return address at +0x80): DllMain(base,
        DLL_PROCESS_ATTACH, NULL) with the shadow space (RSP is 16-aligned after the service's ret), a FALSE return
        calls a thunk that stops the run by name, else RAX = base and a jump to the caller. A traced image's DllMain
        runs with the call trace closed (Sprint 060): on Windows tools/jit_capture loads mono-2.0-bdwgc.dll, and its
        DllMain runs, before it hooks the images, and UnityPlayer.dll's later load of it runs no DllMain."""
        trace = PROCESS_VA + TRACE_OFF
        pause = (any(base == self.modules.dlls.get(n.lower(), (None,))[0] for n in self.call_trace)
                 and struct.unpack_from("<Q", self.mm, self._process(TRACE_OFF))[0] == 1)
        page, error = self.virtual.alloc(0, 0x1000, MEM_COMMIT | MEM_RESERVE, PAGE_EXECUTE_READWRITE)
        if error:
            raise Unimplemented(f"LoadLibrary: no page for the DllMain stub (error {error})")
        failed = self.modules.make_thunk(DLL_INIT_FAILED)
        code = bytearray()
        if pause:
            code += bytes.fromhex("48b9") + trace.to_bytes(8, "little")        # mov rcx, the trace header
            code += bytes.fromhex("48c70100000000")                            # mov qword [rcx], 0: closed
        code += bytes.fromhex("4883ec20")                                      # sub rsp, 0x20
        code += bytes.fromhex("48b9") + base.to_bytes(8, "little")             # mov rcx, instance
        code += bytes.fromhex("ba01000000")                                    # mov edx, DLL_PROCESS_ATTACH
        code += bytes.fromhex("4531c0")                                        # xor r8d, r8d: a dynamic load
        code += bytes.fromhex("48b8") + entry.to_bytes(8, "little")            # mov rax, DllMain
        code += bytes.fromhex("ffd0")                                          # call rax
        code += bytes.fromhex("4883c420")                                      # add rsp, 0x20
        if pause:
            code += bytes.fromhex("48b9") + trace.to_bytes(8, "little")        # mov rcx, the trace header
            code += bytes.fromhex("48c70101000000")                            # mov qword [rcx], 1: open again
        code += bytes.fromhex("85c0")                                          # test eax, eax
        code += bytes.fromhex("750c")                                          # jnz past the failure call
        code += bytes.fromhex("48b8") + failed.to_bytes(8, "little") + bytes.fromhex("ffd0")
        code += bytes.fromhex("48b8") + base.to_bytes(8, "little")             # mov rax, the module's handle
        disp = INIT_STUB_SLOT - (len(code) + 6)
        code += bytes.fromhex("ff25") + disp.to_bytes(4, "little", signed=True)   # jmp [the caller's return]
        assert len(code) <= INIT_STUB_SLOT
        self.mem.write(page, bytes(code))
        return page

    def _zero(self, va: int, n: int) -> None:
        self.mem.write(va, bytes(n))

    def _virtual_request(self, op: int, a0: int, a1: int, a2: int, a3: int) -> tuple[int, int]:
        """(result, second) for the guest's virtual_memory.c: second is the error code on failure, or
        VirtualProtect's old protection on success."""
        v = self.virtual
        if op == HOST_VIRTUAL_ALLOC:
            address, err = v.alloc(a0, a1, a2, a3)
            return address, err
        if op == HOST_VIRTUAL_FREE:
            ok, err = v.free(a0, a1, a2)
            return ok, err
        if op == HOST_VIRTUAL_PROTECT:                    # second: the old protection, the error above it
            ok, old, err = v.protect(a0, a1, a2)
            return ok, old | err << 32
        info = v.query(a0)                                # HOST_VIRTUAL_QUERY: a1 is the guest buffer
        if info is None:
            return 0, 87
        raw = struct.pack("<QQIHHQIIIi", info.base, info.allocation_base, info.allocation_protect, 0, 0,
                          info.region_size, info.state, info.protect, info.type, 0)
        self.mem.write(a1, raw)
        return len(raw), 0

    def calls(self) -> list[str]:
        """The service calls the guest runtime logged, in order: the first `capacity` of them."""
        count, capacity = struct.unpack_from("<QQ", self.mm, self._process(RING_OFF))
        names = guest_runtime.api_ids()
        base = self._process(RING_OFF) + RING_HEADER
        return [names[struct.unpack_from("<Q", self.mm, base + 24 * i)[0]] for i in range(min(count, capacity))]

    def faults(self) -> tuple[int, list[tuple[int, int, int, int]]]:
        """Faults the guest dispatched (runtime.c): how many, and the first 64 as (code, RIP, info 0, info 1)."""
        off = self._process(FAULT_LOG_OFF)
        count = struct.unpack_from("<Q", self.mm, off)[0]
        return count, [struct.unpack_from("<4Q", self.mm, off + 8 + 32 * i) for i in range(min(count, 64))]

    def _traced_proc(self, va: int) -> int:
        """GetProcAddress's answer as the capture's process sees it (Sprint 060): a name resolved while the trace is
        closed (a DLL's start before UnityMain, a traced image's DllMain) is the real function on Windows, resolved
        before tools/jit_capture hooked GetProcAddress, and its calls are never traced; here it is a thunk of its
        own marked untraced (owner 0xFF), the same service as the shared one. With the trace open, a traced image's
        GetProcAddress is call_trace.h's log_get_proc_address, whose answer for a Windows DLL's name is a traced
        thunk that records every call through it, whoever makes it (Sprint 066: mono's p/invoke wrappers, JIT code,
        call GetConsoleCP so, game run 38216081440 seq 2146); here that is a thunk of its own with the
        GetProcAddress label (owner 1 + W2M_TRACE_DYNAMIC). Another caller's GetProcAddress is the real one, and
        its answer the shared, untraced thunk."""
        if not self.call_trace or va not in self.ld.thunks:
            return va
        key = self.ld.thunks[va]
        if struct.unpack_from("<Q", self.mm, self._process(TRACE_OFF))[0]:
            # the bridge saved the guest's GPRs in the state block; RSP (slot 4) holds the call's return address
            rsp = struct.unpack_from("<Q", self.mm, self._process(STATE_OFF) + 8 * 4)[0]
            caller = int.from_bytes(self.mem.read(rsp, 8), "little")
            t = self._process(TRACE_OFF)
            if not any(lo <= caller < hi for lo, hi in (struct.unpack_from("<QQ", self.mm, t + 40 + 16 * i)
                                                        for i in range(2))):
                return va
            hooked = self._trace_dynamic.get(key)
            if hooked is None:
                hooked = self._trace_dynamic[key] = self.modules.make_thunk(key)
                index = (hooked - THUNK_BASE) // 16
                if index < TRACE_THUNKS:
                    self.mm[t + TRACE_OWNER_OFF + index] = 1 + TRACE_DYNAMIC
            return hooked
        quiet = self._trace_quiet.get(key)
        if quiet is None:
            quiet = self._trace_quiet[key] = self.modules.make_thunk(key)
            index = (quiet - THUNK_BASE) // 16
            if index < TRACE_THUNKS:
                self.mm[self._process(TRACE_OFF) + TRACE_OWNER_OFF + index] = 0xFF
        return quiet

    def _trace_ranges(self) -> None:
        """The traced images' address ranges in the trace header, as each is mapped, and their import slots on thunks
        of their own (call_trace.c): tools/jit_capture points each hooked image's slots at its own thunks, and
        GetProcAddress at another, so the capture labels a call by the slot it went through."""
        for i, name in enumerate(self.call_trace):
            c = name.lower()
            if c in self.modules.dlls:
                base, size, _ = self.modules.dlls[c]
            elif c == self.modules.image_name.lower() and self.ld.image is not None:
                base, size = self.ld.image.image_base, self.ld.image.size_of_image
            else:
                continue
            struct.pack_into("<QQ", self.mm, self._process(TRACE_OFF) + 40 + 16 * i, base, base + size)
            if c in self._trace_bound:
                continue
            self._trace_bound.add(c)
            image = next(im for im in self.ld.images if im.base == base)
            for slot in sorted(image.pe.iat):
                held = int.from_bytes(self.mem.read(base + slot, 8), "little")
                if held not in self.ld.thunks:
                    continue                             # bound to a game DLL's export: no Windows hook there
                own = self.modules.make_thunk(self.ld.thunks[held])
                self.mem.write(base + slot, own.to_bytes(8, "little"))
                index = (own - THUNK_BASE) // 16
                if index < TRACE_THUNKS:
                    self.mm[self._process(TRACE_OFF) + TRACE_OWNER_OFF + index] = i + 1

    def instructions_executed(self) -> int:
        """The guest instructions this VM has run (the monitor data's count)."""
        return int(struct.unpack_from("<Q", self.mm, MONITOR_DATA)[0])

    def traced_calls(self) -> tuple[list[tuple[int, int, str, str, str, tuple[int, ...], int, int]], int]:
        """The calls call_trace.c recorded, in the Windows trace's shape (tools/jit_capture/call_trace.h): (seq, tid,
        image, dll, function, a0..a7, ret, last error), and the count of calls the full area left out."""
        t = self._process(TRACE_OFF)
        _, _, count, _, dropped = struct.unpack_from("<5Q", self.mm, t)     # on, seq, count, capacity, dropped
        out = []
        for k in range(count):
            ready, seq, tid, image, thunk, *rest = struct.unpack_from("<15Q", self.mm,
                                                                       t + TRACE_RECORDS_OFF + TRACE_RECORD * k)
            if not ready:
                continue
            dll, fn = self.ld.thunks[THUNK_BASE + 16 * thunk].split("!", 1)
            label = self.call_trace[image] if image < len(self.call_trace) else "GetProcAddress"
            out.append((seq, tid, label, dll, fn, tuple(rest[:8]), rest[8], rest[9]))
        return out, dropped

    def traced_strings(self) -> list[tuple[int, int, str]]:
        """The string arguments call_trace.c kept, as the capture's <capture>.strings.tsv has them: (seq, argument,
        text with ASCII as is and every other unit as \\uXXXX)."""
        t = self._process(TRACE_OFF)
        n = struct.unpack_from("<Q", self.mm, t + TRACE_STRINGS_OFF)[0]
        out = []
        for k in range(n):
            at = t + TRACE_STRINGS_OFF + 16 + TRACE_STRING * k
            seq, arg = struct.unpack_from("<QQ", self.mm, at)
            units = struct.unpack_from("<60H", self.mm, at + 16)
            text = "".join(chr(u) if 0x20 <= u < 0x7F else f"\\u{u:04x}" for u in units[:units.index(0)])
            out.append((seq, arg, text))
        return out

    def calls_total(self) -> dict[str, int]:
        """Every service call by name, counted in the guest (exact past the ring's capacity)."""
        names = guest_runtime.api_ids()
        per = struct.unpack_from(f"<{RING_APIS}Q", self.mm, self._process(RING_OFF) + 16)
        return {names.get(i, f"api {i}"): n for i, n in enumerate(per) if n}

    def calls_dropped(self) -> int:
        count, capacity = struct.unpack_from("<QQ", self.mm, self._process(RING_OFF))
        return max(0, int(count) - int(capacity))

    def exception(self) -> dict[str, int] | None:
        """The last unhandled exception the guest runtime recorded: the EXCEPTION_RECORD and the
        CONTEXT fields it built (specs/guest_state.json offsets), or None."""
        e = self._process(EXCEPTION_OFF)
        if struct.unpack_from("<Q", self.mm, e)[0] != EXCEPTION_MAGIC:
            return None
        r, c = e + 16, e + 0x100
        code, flags, _rec, address, nparams = struct.unpack_from("<IIQQI", self.mm, r)
        info0, info1 = struct.unpack_from("<QQ", self.mm, r + 32)
        out = {"code": code, "flags": flags, "address": address, "parameters": nparams,
               "info0": info0, "info1": info1}
        out["context_flags"], out["mxcsr"] = struct.unpack_from("<II", self.mm, c + 48)
        segs = struct.unpack_from("<6H", self.mm, c + 56)
        out.update(zip(("seg_cs", "seg_ds", "seg_es", "seg_fs", "seg_gs", "seg_ss"), segs, strict=True))
        out["eflags"] = struct.unpack_from("<I", self.mm, c + 68)[0]
        for i, v in enumerate(struct.unpack_from("<16Q", self.mm, c + 120)):
            out[f"r{i}"] = v
        out["rip"] = struct.unpack_from("<Q", self.mm, c + 248)[0]
        out["fltsave_control_word"] = struct.unpack_from("<H", self.mm, c + 256)[0]
        out["fltsave_mxcsr"] = struct.unpack_from("<I", self.mm, c + 256 + 24)[0]
        for i in range(16):
            out[f"xmm{i}"] = int.from_bytes(self.mm[c + 256 + 160 + 16 * i:c + 256 + 176 + 16 * i], "little")
        return out

    def clear_exception(self) -> None:
        """Forget the last unhandled exception, and the service calls logged before it (a driver
        that goes on past an exception starts a new process's worth of calls)."""
        struct.pack_into("<Q", self.mm, self._process(EXCEPTION_OFF), 0)
        struct.pack_into("<Q", self.mm, self._process(RING_OFF), 0)

    def _flush_swap(self) -> None:
        """A switch's vector swap before any other vector command (the pipe sent vswap ahead of the next command)."""
        if self._vswap is not None and self._channel is not None:
            swap, self._vswap = self._vswap, None
            self._channel.swap(swap)
            self.monitor_commands["vswap"] += 1

    def vectors(self) -> tuple[list[int], int, int]:
        """XMM0-15 (as 128-bit integers), FPCR and FPSR of the stopped vCPU (during `run`)."""
        if self._channel is not None:
            self._flush_swap()
            self.monitor_commands["vget"] += 1
            return self._channel.vget()
        r = self._cmd("vget")
        return [int(q, 16) for q in r[1:17]], int(r[17], 16), int(r[18], 16)

    def set_vectors(self, xmm: list[int], fpcr: int, fpsr: int) -> None:
        if self._channel is not None:
            self._flush_swap()
            self.monitor_commands["vset"] += 1
            self._channel.vset(xmm, fpcr, fpsr)
            return
        self._cmd("vset " + " ".join(f"{q:032x}" for q in xmm) + f" {fpcr:x} {fpsr:x}")

    def ymm_hi(self) -> list[int]:
        """YMM0-15 bits 255:128 (the monitor data page's slots; Sprint 049)."""
        return [int.from_bytes(self.mm[MONITOR_DATA + YMM_HI_OFF + 16 * i:MONITOR_DATA + YMM_HI_OFF + 16 * i + 16],
                               "little") for i in range(16)]

    def set_ymm_hi(self, values: list[int]) -> None:
        for i, v in enumerate(values):
            self.mm[MONITOR_DATA + YMM_HI_OFF + 16 * i:MONITOR_DATA + YMM_HI_OFF + 16 * i + 16] = v.to_bytes(16, "little")

    def x87(self) -> tuple[int, int, int, list[int]]:
        """FCW, FSW, the abridged tags and ST0-ST7 in stack order (the monitor data page's control word and the state
        block's x87 fields; Sprint 065)."""
        st = self._process(STATE_OFF)
        cw = struct.unpack_from("<H", self.mm, MONITOR_DATA + FCW_OFF)[0]
        sw, tags = struct.unpack_from("<HB", self.mm, st + X87_SW_OFF)
        regs = [int.from_bytes(self.mm[st + X87_ST_OFF + 16 * i:st + X87_ST_OFF + 16 * i + 10], "little") for i in range(8)]
        return int(cw), int(sw), int(tags), regs

    def set_x87(self, cw: int, sw: int, tags: int, regs: list[int]) -> None:
        st = self._process(STATE_OFF)
        struct.pack_into("<H", self.mm, MONITOR_DATA + FCW_OFF, cw)
        struct.pack_into("<HB", self.mm, st + X87_SW_OFF, sw, tags)
        for i, v in enumerate(regs):
            self.mm[st + X87_ST_OFF + 16 * i:st + X87_ST_OFF + 16 * i + 10] = v.to_bytes(10, "little")

    def mxcsr(self) -> int:
        """MXCSR: the stored control bits and flags, with FPSR's cumulative flags folded in."""
        stored = struct.unpack_from("<I", self.mm, MONITOR_DATA + MXCSR_OFF)[0]
        return int(stored) | mxcsr_flags_from_fpsr(self.vectors()[2])

    def set_mxcsr(self, mxcsr: int, xmm: list[int] | None = None) -> None:
        """Store MXCSR, set FPCR from it and clear FPSR (its flags now live in the stored MXCSR)."""
        struct.pack_into("<I", self.mm, MONITOR_DATA + MXCSR_OFF, mxcsr & 0xFFFF)
        self.set_vectors(xmm if xmm is not None else self.vectors()[0], fpcr_for(mxcsr), 0)

    def af(self) -> bool:
        """AF from the lazy operands: bit 4 of a ^ b ^ r."""
        a, b, r = struct.unpack_from("<QQQ", self.mm, MONITOR_DATA + AF_OPERANDS)
        return bool((a ^ b ^ r) & 0x10)

    def df(self) -> bool:
        return bool(self.mm[MONITOR_DATA + DF_BYTE])

    def pf(self) -> bool:
        """PF from PF_RESULT."""
        return parity_even(struct.unpack_from("<Q", self.mm, MONITOR_DATA + PF_RESULT)[0])

    def set_af_df(self, af: bool, df: bool, pf: bool = True) -> None:
        """Lazy operands that read back as `af` and `pf`, and DF."""
        struct.pack_into("<QQQ", self.mm, MONITOR_DATA + AF_OPERANDS, 0x10 if af else 0, 0, 0 if pf else 1)
        self.mm[MONITOR_DATA + DF_BYTE] = int(df)

    def teb_u32(self, offset: int) -> int:
        return int(struct.unpack_from("<I", self.mm, self._process(TEB_OFF) + offset)[0])

    # monitor code ----------------------------------------------------------------------------
    def _write_monitor(self) -> None:
        struct.pack_into("<QQQ", self.mm, MONITOR_DATA, 0, 0, MONITOR_VA + DISPATCH_TABLE)
        vec = Assembler(MONITOR_VA + VECTORS)
        for slot in range(16):
            if slot == 4:                               # VBAR + 0x200: synchronous, current EL, SP_ELx
                vec.branch(a64.b, MONITOR_VA + EXCEPTION_ENTRY)
                vec.emit(*[a64.b(0)] * 31)
            else:
                vec.emit(a64.hvc(2), *[a64.b(0)] * 31)  # each 0x80 slot: hvc #2, then spin
        self._write_words(VECTORS, vec.finish())
        d = Assembler(MONITOR_VA + DISPATCHER)
        top, miss = d.label(), d.label()
        d.bind(top)
        d.emit(a64.ldr(8, 16, 28, 16), a64.eor(18, 17, 17, a64.LSR, DISPATCH_BITS),
               a64.eor(18, 18, 17, a64.LSR, 2 * DISPATCH_BITS), a64.ubfx(18, 18, 0, DISPATCH_BITS),
               a64.add(16, 16, 18, a64.LSL, 4),
               a64.ldp(18, 16, 16), a64.cmp(18, 17))
        d.branch(lambda off: a64.bcond("ne", off), miss)
        d.emit(a64.br(16))
        d.bind(miss)
        self.miss_va = d.here()
        # the host may have compiled a block and made its pages read-only (_protect_code): drop stale TLB entries too
        d.emit(a64.hvc(1), a64.IC_IALLU, a64.TLBI_VMALLE1, a64.DSB_ISH, a64.ISB)
        d.branch(a64.b, top)
        self.dispatch_va = MONITOR_VA + DISPATCHER
        # where a thread goes on after a switch or a model step, x17 its next x86 address: the dispatcher's table,
        # or in lockstep the miss stub, which exits at every instruction
        self.resume_va = self.miss_va if self.lockstep else self.dispatch_va
        self._write_words(DISPATCHER, d.finish())
        empty = struct.pack("<QQ", M64, self.miss_va)
        self.mm[DISPATCH_TABLE:DISPATCH_TABLE + 16 * DISPATCH_ENTRIES] = empty * DISPATCH_ENTRIES
        self._write_exception_entry()                    # it returns to the guest through the dispatcher

    def _slow_sse(self, x: list[int], rip: int) -> int:
        """Execute the SSE instruction at `rip` with x86fp (MXCSR unmasks an exception): the guest's
        registers, flags, XMM and MXCSR go into the model, one step runs on the VM's memory, and the
        result comes back. A fault the model raises is raised in the guest through the guest
        runtime's exception path. Returns the PC to resume at."""
        from w2m import x86fp as fp
        from w2m.loader import GuestFault, Loaded
        from w2m.runtime import Runtime

        self.slow_sse += 1
        try:                                           # the census: which instruction, under which masks
            from w2m.x86dec import decode
            self.slow_sse_classes[(decode(bytes(self.mem.read(rip, 15))).iclass,
                                   int(struct.unpack_from("<I", self.mm, MONITOR_DATA + MXCSR_OFF)[0]) >> 7 & 0x3F)] += 1
        except Exception:                              # noqa: BLE001 - the census never stops a run
            self.slow_sse_classes[("?", 0)] += 1
        xmm, fpcr, fpsr = self.vectors()
        mx = int(struct.unpack_from("<I", self.mm, MONITOR_DATA + MXCSR_OFF)[0]) | mxcsr_flags_from_fpsr(fpsr)
        rt = Runtime(Loaded(None, self.mem, rip, {}, 0))       # type: ignore[arg-type]
        th = rt.th
        th.regs, th.rip, th.xmm, th.mxcsr, th.df = list(x[:16]), rip, list(xmm), mx, self.df()
        th.fcw = struct.unpack_from("<H", self.mm, MONITOR_DATA + FCW_OFF)[0]
        th.ymm_hi = [int.from_bytes(self.mm[MONITOR_DATA + YMM_HI_OFF + 16 * i:MONITOR_DATA + YMM_HI_OFF + 16 * i + 16],
                                    "little") for i in range(16)]
        th.flags = x86_flags(x, self)
        st = self._process(STATE_OFF)
        th.x87_sw, th.x87_tags = struct.unpack_from("<HB", self.mm, st + X87_SW_OFF)
        top = th.x87_sw >> 11 & 7                              # the state block keeps ST(i) as FXSAVE does
        for i in range(8):
            raw = self.mm[st + X87_ST_OFF + 16 * i:st + X87_ST_OFF + 16 * i + 10]
            th.x87_regs[(top + i) & 7] = int.from_bytes(raw, "little")
        try:
            rt.step()
        except (fp.SimdFault, GuestFault) as e:
            code = fp.xm_status(th.mxcsr) if isinstance(e, fp.SimdFault) else e.code
            struct.pack_into("<I", self.mm, MONITOR_DATA + MXCSR_OFF, th.mxcsr)
            self._store_x87(st, th)                    # a faulting MMX store has set TOP 0 (set 065b)
            self.set_vectors(xmm, fpcr, 0)
            return self._raise_in_guest(x, xmm, code, rip)
        x[:16] = th.regs
        for f in "CZSOP":
            if th.flags[f] is not None:
                x[FLAG_REG[f]] = int(bool(th.flags[f]))
        x[REG_UNDEFINED_FLAGS] = sum(1 << FLAG_BIT[f] for f in "CZSOPA" if th.flags[f] is None)
        self.set_af_df(bool(th.flags["A"]), th.df, bool(th.flags["P"]))
        struct.pack_into("<I", self.mm, MONITOR_DATA + MXCSR_OFF, th.mxcsr)
        struct.pack_into("<H", self.mm, MONITOR_DATA + FCW_OFF, th.fcw)
        self._store_x87(st, th)
        for i, v in enumerate(th.ymm_hi):
            self.mm[MONITOR_DATA + YMM_HI_OFF + 16 * i:MONITOR_DATA + YMM_HI_OFF + 16 * i + 16] = v.to_bytes(16, "little")
        self.set_vectors(th.xmm, fpcr_for(th.mxcsr), 0)          # LDMXCSR may have changed the controls
        x[17] = th.rip
        # on through the dispatcher's table: the next instruction's block is usually there already. Returning to the
        # miss stub cost one hvc #1 per model step (frontier 643a6283510c-1791841443768764000: 207,514 misses at
        # 0x18140b91a against 207,512 model steps at the PCMPISTRI before it)
        return self.resume_va

    def _store_x87(self, st: int, th: Any) -> None:
        """The thread's x87 status, tags and registers back into the state block, ST(i) in FXSAVE's order."""
        struct.pack_into("<HB", self.mm, st + X87_SW_OFF, th.x87_sw, th.x87_tags)
        top = th.x87_sw >> 11 & 7
        for i in range(8):
            self.mm[st + X87_ST_OFF + 16 * i:st + X87_ST_OFF + 16 * i + 10] = th.x87_regs[(top + i) & 7].to_bytes(10, "little")

    def _raise_in_guest(self, x: list[int], xmm: list[int], code: int, rip: int) -> int:
        """Have the guest runtime raise exception `code` at `rip` with the state in `x` and `xmm`:
        fill the save area as the EL1 vector would, and enter the runtime's exception call."""
        save = MONITOR_DATA + SAVE_AREA
        for i in range(31):
            struct.pack_into("<Q", self.mm, save + 8 * i, x[i])
        struct.pack_into("<QQ", self.mm, save + 0x118, code, rip)
        for i, q in enumerate(xmm):
            self.mm[save + SAVE_VECTORS + 16 * i:save + SAVE_VECTORS + 16 * i + 16] = q.to_bytes(16, "little")
        return self.exception_call_va

    def _write_exception_entry(self) -> None:
        """Save x0-x30, ESR, FAR and ELR in the monitor data page, leave exception state (ERET to
        the continuation, EL1h, interrupts masked), take the runtime stack and call
        w2m_exception. For a guest fault it returns once it has dispatched the exception (Sprint 053):
        the state block holds the GPRs and redirect_rip the x86 address to go on at (a handler's call
        stub, or a resumed context), which are loaded as a bridge loads them before it dispatches.
        Translated code keeps nothing in SP, and x16 and x17 are temporaries, so the routine may use
        them; x19-x28 (flags and the monitor's registers) survive the call."""
        e = Assembler(MONITOR_VA + EXCEPTION_ENTRY)
        # First, a write to a page behind translated code (Sprint 065 item 9e): ask the host, which drops the blocks
        # and makes the page writable, then return to the store. The faulting code may hold live values in x16 and
        # x17, so x16 waits in SP (translated code keeps nothing there) and x17 in the monitor data.
        other, back = e.label(), e.label()
        e.emit(a64.addi(31, 16, 0), a64.mov_imm(16, MONITOR_VA + MONITOR_DATA + CODE_WRITE_SPILL),
               a64.str_(8, 17, 16, 0), a64.mrs(17, a64.ESR_EL1), a64.mov_imm(16, CODE_WRITE_ESR_MASK),
               a64.and_(17, 17, 16), a64.mov_imm(16, CODE_WRITE_ESR), a64.cmp(17, 16))
        e.branch(lambda off: a64.bcond("ne", off), other)
        e.emit(a64.mrs(17, a64.FAR_EL1), a64.hvc(HVC_CODE_WRITE))           # x17: 1 when the host dropped blocks
        e.branch(lambda off: a64.cbz(17, off), other)
        e.emit(a64.TLBI_VMALLE1, a64.DSB_ISH, a64.ISB)
        e.branch(a64.b, back)
        e.bind(other)
        e.emit(a64.mov_imm(16, MONITOR_VA + MONITOR_DATA + CODE_WRITE_SPILL), a64.ldr(8, 17, 16, 0),
               a64.addi(16, 31, 0))
        e.emit(a64.mov_imm(16, MONITOR_VA + MONITOR_DATA + SAVE_AREA))
        for i in range(0, 30, 2):
            e.emit(a64.stp(i, i + 1, 16, 8 * i))
        e.emit(a64.str_(8, 30, 16, 8 * 30))
        e.emit(a64.mrs(0, a64.ESR_EL1), a64.mrs(1, a64.FAR_EL1), a64.mrs(2, a64.ELR_EL1),
               a64.stp(0, 1, 16, 0x100), a64.str_(8, 2, 16, 0x110), a64.stp(31, 31, 16, 0x118))
        for i in range(0, 16, 2):                                # XMM0-15 for the CONTEXT
            e.emit(a64.stp_q(i, i + 1, 16, SAVE_VECTORS + 16 * i))
        cont = e.here() + 4 * 8                         # four words of MOV, MSR, MOVZ, MSR, ERET below
        e.emit(a64.movz(17, cont & 0xFFFF), *(a64.movk(17, cont >> 16 * h & 0xFFFF, h) for h in (1, 2, 3)),
               a64.msr(a64.ELR_EL1, 17), a64.movz(17, CPSR_EL1H), a64.msr(a64.SPSR_EL1, 17), a64.ERET)
        assert e.here() == cont
        self.exception_call_va = cont
        e.emit(*count_from_budget())
        e.emit(a64.mov_imm(16, MONITOR_VA + RUNTIME_STACK + RUNTIME_STACK_SIZE), a64.addi(31, 16, 0),
               a64.mov_imm(16, self.runtime.symbols["w2m_exception"]), a64.blr(16))
        e.emit(*budget_from_count())
        e.emit(a64.mov_imm(16, PROCESS_VA + STATE_OFF))
        for i in range(0, 16, 2):
            e.emit(a64.ldp(i, i + 1, 16, 8 * i))
        e.emit(a64.ldr(8, 17, 16, STATE_REDIRECT), a64.str_(8, a64.ZR, 16, STATE_REDIRECT))
        e.branch(a64.b, self.miss_va if self.lockstep else self.dispatch_va)
        e.bind(back)
        e.emit(a64.mov_imm(16, MONITOR_VA + MONITOR_DATA + CODE_WRITE_SPILL), a64.ldr(8, 17, 16, 0),
               a64.addi(16, 31, 0), a64.ERET)
        words = e.finish()
        assert 4 * len(words) <= EXCEPTION_ENTRY_END - EXCEPTION_ENTRY
        self._write_words(EXCEPTION_ENTRY, words)

    def _write_words(self, off: int, words: list[int]) -> None:
        self.mm[off:off + 4 * len(words)] = struct.pack(f"<{len(words)}I", *words)

    # code pages ------------------------------------------------------------------------------
    def _page_descriptor(self, va: int) -> int:
        """The offset in the memory file of the level-3 descriptor that maps page `va`, a 1 GB or 2 MB block over it
        split into the next level's 512 descriptors with the block's attributes (Arm ARM D8.3); 0 when nothing maps
        it."""
        off = TRANSLATION_TABLES
        for level, shift in ((0, 39), (1, 30), (2, 21)):
            at = off + 8 * ((va >> shift) & 0x1FF)
            desc = struct.unpack_from("<Q", self.mm, at)[0]
            if not desc & 1:
                return 0
            if not desc & 2:                                   # a block: split it
                size = 1 << shift
                base = desc & 0x0000_FFFF_FFFF_F000 & ~(size - 1)
                attributes = desc & ~0x0000_FFFF_FFFF_F000 & ~0x3
                new = self._next_table
                if new + PAGE > SPLIT_TABLES + SPLIT_TABLES_SIZE:
                    raise CompileError("stage-1 split table area full")
                self._next_table += PAGE
                low = 0x3 if level == 2 else 0x1                # pages at level 3, blocks at level 2
                for i in range(512):
                    struct.pack_into("<Q", self.mm, new + 8 * i, (base + i * (size >> 9)) | attributes | low)
                struct.pack_into("<Q", self.mm, at, (IPA_BASE + new) | 0x3)
                desc = (IPA_BASE + new) | 0x3
            off = int((desc & 0x0000_FFFF_FFFF_F000) - IPA_BASE)
        return off + 8 * ((va >> 12) & 0x1FF)

    def _protect_code(self, start: int, end: int) -> None:
        """Make the pages under guest bytes [start, end) read-only and note the block over them. The thunks and the
        process area are the host's and never written by the guest; they stay as they are."""
        if start >= PROCESS_VA:
            return
        for page in range(start & ~(PAGE - 1), end, PAGE):
            blocks = self.code_pages.get(page)
            if blocks is None:
                at = self._page_descriptor(page)
                if not at:
                    continue
                desc = struct.unpack_from("<Q", self.mm, at)[0]
                struct.pack_into("<Q", self.mm, at, desc | AP_READ_ONLY)
                blocks = self.code_pages[page] = set()
            blocks.add(start)

    def _code_written(self, address: int) -> bool:
        """A write faulted at `address`: when its page is behind translated code, drop the blocks over the page and
        their dispatch slots and make the page writable again; the next execution translates afresh."""
        page = address & ~(PAGE - 1)
        for at in self.lift_pages.pop(page, ()):
            self.lift_cache.pop(at, None)
            self.flags_cache.pop((at, None), None)
            for kind in NZCV_CONDITIONS:
                self.flags_cache.pop((at, kind), None)
        blocks = self.code_pages.pop(page, None)
        if blocks is None:
            return False
        for start in blocks:
            if self.blocks.pop(start, None) is None:
                continue
            slot = DISPATCH_TABLE + 16 * dispatch_slot(start)
            if struct.unpack_from("<Q", self.mm, slot)[0] == start:
                struct.pack_into("<QQ", self.mm, slot, M64, self.miss_va)
            self.blocks_dropped += 1
        at = self._page_descriptor(page)
        struct.pack_into("<Q", self.mm, at, struct.unpack_from("<Q", self.mm, at)[0] & ~AP_READ_ONLY)
        self.code_writes += 1
        return True

    def _host_wrote(self, address: int, size: int) -> None:
        """The host wrote guest bytes [address, address + size) (a file read, a context, a parameter block): drop the
        translations over them as a guest write would."""
        if self.code_pages:
            for page in range(address & ~(PAGE - 1), address + size, PAGE):
                if page in self.code_pages:
                    self._code_written(page)

    # blocks ----------------------------------------------------------------------------------
    def _compile(self, start: int) -> int:
        if start == self._trace_from and self._trace_from:
            struct.pack_into("<Q", self.mm, self._process(TRACE_OFF), 1)     # the trace opens (call_trace_from)
        a = Assembler(MONITOR_VA + self.code_cursor)
        insns: list[Lifted] = []
        addr = start
        while len(insns) < (1 if self.lockstep else MAX_BLOCK_INSNS):
            if insns and addr in self.ld.thunks:
                break
            try:
                ins = self._lift(addr)
            except (LiftError, GuestFault):
                if not insns:
                    raise
                break
            insns.append(ins)
            addr += ins.length
            if any(op[0] == "br" for op in ins.ops):
                break
        exit_va = self.miss_va if self.lockstep else self.dispatch_va
        pc_marks: list[tuple[int, int]] = [(a.here(), start)]    # the block entry: a guest boundary
        counter = a.label()
        if BLOCK_CENSUS:                                 # x16, x17 dead at a block's entry (the prologue loads x17)
            a.branch(lambda off: a64.adr(16, off), counter)
            a.emit(a64.ldr(8, 17, 16, 0), a64.addi(17, 17, 1), a64.str_(8, 17, 16, 0))
        budget_stop, body = a.label(), a.label()
        a.emit(0xF1000000 | len(insns) << 10 | REG_BUDGET << 5 | REG_BUDGET)       # SUBS x23, x23, #n
        a.branch(lambda off: a64.bcond("mi", off), budget_stop)
        a.bind(body)
        ended = False
        if not self.lockstep:
            self._protect_code(start, addr)              # before the successor scan, which reads the pages it covers
            for ins in insns:
                self._remember(ins)
        live = self._flags_live_out(start, insns[-1], addr) if FLAG_LIVENESS and not self.lockstep else ALL_FLAGS
        lives = [ALL_FLAGS] * len(insns)                 # the flags read after each instruction
        for k in range(len(insns) - 1, -1, -1):
            lives[k] = live
            read, written = self._insn_flags(insns[k], nzcv_setter(insns[k - 1]) if k else None)
            live = live & ~written | read
        previous: Lifted | None = None
        for k, ins in enumerate(insns):
            pc_marks.append((a.here(), ins.address))
            ended = compile_insn(a, ins, exit_va, self.runtime.symbols, nzcv_setter(previous) if previous else None,
                                 lives[k], k + 1 < len(insns) and nzcv_branch(ins, insns[k + 1]))
            previous = ins
        body_end = a.here()                              # the census counts the fall-through exit as block overhead
        if not ended:
            a.emit(a64.mov_imm(17, addr))
            a.branch(a64.b, exit_va)
        a.bind(budget_stop)                              # the budget's stop, out of the block's path: an exit whose
        pc_marks.append((a.here(), start))               # address maps to the block's first instruction, then on in
        a.emit(a64.hvc(4))
        a.branch(a64.b, body)
        if BLOCK_CENSUS:
            overhead = (a.here() - body_end) // 4
            if a.here() % 8:
                a.emit(0xD503201F)                       # NOP: the counter 8-byte aligned
            a.bind(counter)
            a.emit(0, 0)
        words = a.finish()
        if self.code_cursor + 4 * len(words) > CODE_CACHE + CODE_CACHE_SIZE:
            raise CompileError("code cache full")
        self._write_words(self.code_cursor, words)
        va = MONITOR_VA + self.code_cursor
        self.code_cursor += 4 * len(words)
        # the code cache only grows and a block's marks rise, so appending keeps pc_map sorted (Sprint 062: sorting
        # it after every block took 98 of a frontier run's 154 s)
        self.pc_map.extend(pc_marks)
        self._recon_append(pc_marks)
        self.addrs.update(i.address for i in insns)
        self.blocks[start] = va
        if BLOCK_CENSUS:                    # (counter's offset, [(iclass, arm64 words)], prologue and exit words)
            marks = [m for m, _ in pc_marks[1:1 + len(insns)]] + [body_end]
            self.block_census.append((a.labels[counter] - MONITOR_VA,
                                      [(i.iclass, (marks[k + 1] - marks[k]) // 4) for k, i in enumerate(insns)],
                                      ((pc_marks[1][0] - pc_marks[0][0]) // 4 if insns else 0) + overhead))
        self.translations.append((start, addr, "static" if start == self.ld.entry else "runtime_transfer"))
        return va

    def _lift(self, addr: int) -> Lifted:
        """The instruction at `addr`, from lift_cache when a block or a scan has lifted it since its page was written."""
        ins = self.lift_cache.get(addr)
        return ins if ins is not None else lift(self.mem.code_at(addr), addr)

    def _remember(self, ins: Lifted) -> None:
        """Keep `ins` in lift_cache when every page its bytes are on is behind translated code (so a write drops it)."""
        if ins.address in self.lift_cache:
            return
        pages = {ins.address & ~(PAGE - 1), (ins.address + ins.length - 1) & ~(PAGE - 1)}
        if pages <= self.code_pages.keys():
            self.lift_cache[ins.address] = ins
            for page in pages:
                self.lift_pages.setdefault(page, set()).add(ins.address)

    def _insn_flags(self, ins: Lifted, nzcv_kind: str | None) -> tuple[int, int]:
        """insn_flags, kept with the cached lift."""
        key = (ins.address, nzcv_kind)
        got = self.flags_cache.get(key)
        if got is None:
            got = insn_flags(ins, nzcv_kind)
            if self.lift_cache.get(ins.address) is ins:
                self.flags_cache[key] = got
        return got

    def _flags_live_out(self, start: int, last: Lifted, end: int) -> int:
        """The flags a block read after it ends (flag liveness): those its direct successors read before writing,
        each scanned by _flags_live_in. A successor not known when compiling (a return, an indirect branch) reads all."""
        constant = {op[1]: op[2] for op in last.ops if op[0] == "imm"}
        br = next((op for op in last.ops if op[0] == "br"), None)
        if br is None:
            return self._flags_live_in(start, end)
        if br[2] not in constant:
            return ALL_FLAGS
        target = constant[br[2]] & M64
        return self._flags_live_in(start, target) | (0 if br[1] == "always" else self._flags_live_in(start, end))

    def _flags_live_in(self, owner: int, addr: int) -> int:
        """The flags code at `addr` reads before writing them, from at most LIVENESS_SCAN instructions on its straight
        path (following direct jumps). Only bytes on pages already behind translated code are read, and the block at
        `owner` joins each page's blocks, so a write there drops it as it drops the code it covers (_code_written).
        A thunk, the host's process area, a page not yet translated, or the end of the scan: every undecided flag is
        read."""
        live, undecided, previous = 0, ALL_FLAGS, None
        for _ in range(LIVENESS_SCAN):
            if addr in self.ld.thunks or addr >= PROCESS_VA or addr & ~(PAGE - 1) not in self.code_pages:
                break
            try:
                ins = self._lift(addr)
            except (LiftError, GuestFault):
                break
            pages = {addr & ~(PAGE - 1), (addr + ins.length - 1) & ~(PAGE - 1)}
            if not pages <= self.code_pages.keys():
                break
            for page in pages:
                self.code_pages[page].add(owner)
            self._remember(ins)
            read, written = self._insn_flags(ins, nzcv_setter(previous) if previous else None)
            live |= read & undecided
            undecided &= ~(read | written)
            if not undecided:
                return live
            br = next((op for op in ins.ops if op[0] == "br"), None)
            if br is None:
                addr, previous = addr + ins.length, ins
                continue
            constant = {op[1]: op[2] for op in ins.ops if op[0] == "imm"}
            if br[1] != "always" or br[2] not in constant:
                break
            addr, previous = constant[br[2]] & M64, None
        return live | undecided

    def _recon_append(self, marks: list[tuple[int, int]]) -> None:
        """Append (host code VA, x86 address) pairs to the reconstruction table in guest memory.
        The code cache only grows, so the table stays sorted."""
        n = struct.unpack_from("<Q", self.mm, MONITOR_DATA + RECON_COUNT)[0]
        if n + len(marks) > RECON_ENTRIES:
            raise CompileError("reconstruction table full")
        for i, (host, guest) in enumerate(marks):
            struct.pack_into("<QQ", self.mm, RECON_TABLE + 16 * (n + i), host, guest)
        struct.pack_into("<Q", self.mm, MONITOR_DATA + RECON_COUNT, n + len(marks))

    def x86_address_at(self, pc: int) -> int:
        i = bisect_right(self.pc_map, (pc, M64)) - 1
        return self.pc_map[i][1] if i >= 0 else 0

    # run -------------------------------------------------------------------------------------
    def measure_exits(self, n: int = 100_000, through_pipe: int = 10_000) -> dict[str, float]:
        """The cost of a VM exit (Sprint 048; review 2026-10-10 F1): guest code that loops on `hvc #0x7e` runs n exits
        resumed by the monitor in C (`spin`), then `through_pipe` exits each resumed from here over the monitor's text
        pipe, as every host request is today. Returns nanoseconds per exit for both."""
        proc = subprocess.Popen([str(build_monitor()), self.path, f"{IPA_BASE:x}", f"{self.size:x}"],
                                stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True)
        assert proc.stdin and proc.stdout
        pin, pout = proc.stdin, proc.stdout

        def cmd(c: str) -> list[str]:
            pin.write(c + "\n")
            pin.flush()
            return pout.readline().split()

        try:
            for name, v in (("CPACR_EL1", CPACR_FPEN), ("MAIR_EL1", 0xFF), ("TCR_EL1", TCR), ("TTBR0_EL1", self.ttbr),
                            ("VBAR_EL1", MONITOR_VA + VECTORS), ("SCTLR_EL1", SCTLR)):
                cmd(f"sys {SYS_REG_ID[name]:x} {v:x}")
            at = self.code_cursor
            self._write_words(at, [a64.hvc(0x7E), a64.b(-4)])            # hvc #0x7e; b . - 4
            cmd(f"set {MONITOR_VA + at:x} {CPSR_EL1H:x} " + " ".join("0" for _ in range(31)))
            taken, ns = (int(v) for v in cmd(f"spin {n}")[1:3])
            t0 = time.perf_counter_ns()
            for _ in range(through_pipe):
                reply = cmd("run 1000")
                assert reply[0] == "exit" and reply[1] == "hvc", reply
            pipe = (time.perf_counter_ns() - t0) / through_pipe
        finally:
            cmd("quit")
            proc.wait(timeout=5)
        if taken != n:
            raise RuntimeError(f"spin took {taken} of {n} exits")
        return {"raw_ns": ns / n, "pipe_ns": pipe}

    def run(self, max_steps: int = 1_000_000, timeout_ms: int = 10_000,
            on_dispatch: Callable[[VM, list[int]], None] | None = None,
            on_exception: Callable[[VM, dict[str, int]], bool] | None = None,
            interrupt_every_ms: int | None = None,
            on_boundary: Callable[[VM, list[int], int], None] | None = None,
            on_unimplemented: Callable[[VM, list[int], str], bool] | None = None) -> Result:
        """Run alone until the guest exits or stops (steps(), driven to its end)."""
        gen = self.steps(max_steps, timeout_ms, on_dispatch, on_exception, interrupt_every_ms, on_boundary,
                         on_unimplemented)
        try:
            while True:
                next(gen)
        except StopIteration as done:
            result: Result = done.value
            return result

    def steps(self, max_steps: int = 1_000_000, timeout_ms: int = 10_000,
            on_dispatch: Callable[[VM, list[int]], None] | None = None,
            on_exception: Callable[[VM, dict[str, int]], bool] | None = None,
            interrupt_every_ms: int | None = None,
            on_boundary: Callable[[VM, list[int], int], None] | None = None,
            on_unimplemented: Callable[[VM, list[int], str], bool] | None = None) -> Generator[str, None, Result]:
        """Run until the guest exits or stops, yielding to a kernel.Machine (Sprint 055): "idle" when no thread of
        this VM can run, "slice" when a time slice ends and another VM may want the vCPU time. `on_dispatch` sees (and may rewrite) the register
        snapshot at each dispatch miss; x[17] is the x86 address about to run. An unhandled
        guest exception ends the process, as on Windows; `on_exception` sees its records and, by
        returning True, has the VM go on at the next dispatch miss instead (the oracle's driver).

        `interrupt_every_ms` stops the vCPU asynchronously (hv_vcpus_exit) that often, wherever
        it is. The forward rule (specs/guest_state.json 0.2.0): the host does not reconstruct
        state there; it lowers the block-entry stop point and resumes the vCPU unchanged, so the
        vCPU stops at the next block entry, a guest instruction boundary where the state is exact.
        `on_boundary(vm, x, guest_rip)` sees that state; the run then goes on.

        `on_unimplemented(vm, x, import_name)` sees a call to an import w2m lacks; by returning True it
        has the guest return from the call with the state it left in x (a look-ahead map of what a
        program calls next, scripts/frontier_lookahead.py; never a judged run)."""
        proc = subprocess.Popen([str(build_monitor()), self.path, f"{IPA_BASE:x}", f"{self.size:x}"],
                                stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True)
        assert proc.stdin and proc.stdout
        pin, pout = proc.stdin, proc.stdout

        def cmd(c: str) -> list[str]:
            commands = [c]
            if self._vswap is not None:                     # a switch's vectors, in the same write as what follows
                commands.insert(0, "vswap {:x} {:x}".format(*self._vswap))
                self._vswap = None
            pin.write("".join(k + "\n" for k in commands))
            pin.flush()
            for k in commands:
                self.monitor_commands[k[:k.find(" ")] if " " in k else k] += 1
                reply = pout.readline().split()
                if not reply or reply[0] == "err":
                    # an empty reply is the monitor gone: name its exit status (a signal is negative), which
                    # the one such failure seen (class ADD_LOCK, 2026-10-10, once in about 1,600 runs) lacked
                    gone = "" if reply else f" (monitor exit status {proc.wait(timeout=5)})"
                    raise RuntimeError(f"vm_monitor: {k.split()[0]} -> {reply}{gone}")
            return reply

        self._max_steps = max_steps
        self._set_budget()
        forced_stop = False
        runtime_lo, runtime_hi = MONITOR_VA + RUNTIME_IMAGE, MONITOR_VA + RUNTIME_IMAGE + RUNTIME_IMAGE_SIZE
        self._cmd = cmd
        if not self.threads:                              # the first thread (scheduler.py; Sprint 051)
            self.threads[self.current_tid] = GuestThread(self.current_tid, PROCESS_VA + TEB_OFF, self._stack,
                                                         status="ready", start=self.ld.entry)
        if struct.unpack_from("<I", self.mm, MONITOR_DATA + MXCSR_OFF)[0] == 0:
            struct.pack_into("<I", self.mm, MONITOR_DATA + MXCSR_OFF, 0x1F80)      # SDM Vol 1 10.2.3: reset value
        if struct.unpack_from("<H", self.mm, MONITOR_DATA + FCW_OFF)[0] == 0:
            struct.pack_into("<H", self.mm, MONITOR_DATA + FCW_OFF, 0x27F)        # as a Windows thread starts
        cmd(f"vset {' '.join('0' * 32 for _ in range(16))} "
            f"{fpcr_for(struct.unpack_from('<I', self.mm, MONITOR_DATA + MXCSR_OFF)[0]):x} 0")
        for name, v in (("CPACR_EL1", CPACR_FPEN), ("MAIR_EL1", 0xFF), ("TCR_EL1", TCR), ("TTBR0_EL1", self.ttbr),
                        ("VBAR_EL1", MONITOR_VA + VECTORS), ("SCTLR_EL1", SCTLR)):
            cmd(f"sys {SYS_REG_ID[name]:x} {v:x}")
        if TRANSPORT == "channel":
            cmd(f"channel {CHANNEL:x}")
            self._channel = Channel(self.mm)
        x = [0] * 31
        x[4] = self.ld.stack_top - 0x100 - 8                  # as runtime.Runtime: RSP % 16 == 8
        x[17] = self.ld.entry
        x[REG_UNDEFINED_FLAGS] = 0x3F                                       # every flag undefined at entry
        x[28] = MONITOR_VA + MONITOR_DATA
        pc = self.dispatch_va
        cpsr = CPSR_EL1H
        stop = "timeout"
        report_at = time.monotonic() + PROGRESS_SECONDS
        reported = (0, Counter(self.exits), Counter(self.exit_sites), sum(self.calls_total().values()), time.monotonic())
        marked, last_kind = time.perf_counter_ns(), -1
        try:
            while True:
                machine = self.machine if self.machine is not None and self.machine.main is self else None
                if machine is not None and machine.stop_requested:
                    stop = machine.stop_requested
                    break
                if time.monotonic() >= report_at:             # a run that goes quiet says where it is
                    reported = self._progress(reported, x)
                    report_at = time.monotonic() + PROGRESS_SECONDS
                    why = machine.stop_when() if machine is not None and machine.stop_when is not None else None
                    if why:
                        stop = why
                        break
                started = time.perf_counter_ns()
                self.exit_ns[last_kind] += started - marked
                ms = interrupt_every_ms if interrupt_every_ms is not None else timeout_ms
                count, limit = struct.unpack_from("<QQ", self.mm, MONITOR_DATA)
                if not runtime_lo <= pc < runtime_hi:           # in the runtime's C, x23 is the C's own
                    x[REG_BUDGET] = (limit - count) & M64       # the budget left, in x23
                if self._channel is not None:
                    swap, self._vswap = self._vswap, None
                    e, cpsr = self._channel.run(pc, cpsr, x, ms, swap)
                    self.monitor_commands["run"] += 1
                    if swap:
                        self.monitor_commands["vswap"] += 1
                else:
                    cmd(f"set {pc:x} {cpsr:x} " + " ".join(f"{v:x}" for v in x))
                    r = cmd(f"run {ms}")
                    e = VcpuExit(r[1], int(r[2], 16), int(r[3], 16), int(r[5], 16), int(r[6], 16),
                                 int(r[7], 16), [int(v, 16) for v in r[8:39]])
                    cpsr = int(r[4], 16)
                marked = time.perf_counter_ns()
                self.run_ns += marked - started
                last_kind = 0
                x = e.x
                if not runtime_lo <= e.pc < runtime_hi:
                    left = x[REG_BUDGET] - (1 << 64) if x[REG_BUDGET] >> 63 else x[REG_BUDGET]
                    struct.pack_into("<Q", self.mm, MONITOR_DATA, (limit - left) & M64)   # the count, from x23
                if e.kind == "cancel" and interrupt_every_ms is not None and ASYNC_STOP_RULE == "backward":
                    # the rule specs/guest_state.json forbids: restart the interrupted instruction
                    x[17] = self.x86_address_at(e.pc)
                    pc = self.miss_va
                    cpsr = CPSR_EL1H
                    if on_boundary:
                        on_boundary(self, x, x[17])
                    continue
                if e.kind == "cancel" and interrupt_every_ms is not None:
                    # an asynchronous stop at an arbitrary host instruction: stop at the next
                    # block entry instead (set the stop point below the count) and resume as is
                    executed = struct.unpack_from("<Q", self.mm, MONITOR_DATA)[0]
                    struct.pack_into("<Q", self.mm, MONITOR_DATA + 8, executed)
                    forced_stop = True
                    pc = e.pc
                    continue
                cpsr = CPSR_EL1H
                if e.kind == "cancel":
                    stop = "timeout"
                    break
                if e.kind != "hvc":
                    stop = f"VmExit: {e.kind} syndrome 0x{e.syndrome:x} at pc 0x{e.pc:x}"
                    break
                imm = e.syndrome & 0xFFFF
                last_kind = imm
                # a dispatch miss exits from the dispatcher, outside every block: its site is the target it missed
                where = x[17] if imm == 1 else self.x86_address_at(e.pc - 4)
                self.exits[imm] += 1                          # the exit census (Sprint 064): kind, then the site
                self.exit_sites[(imm, where)] += 1
                if imm == 1:
                    if on_dispatch:
                        on_dispatch(self, x)                  # may replace the state in x
                    target = x[17]
                    if target in self.bridges:
                        va = self.bridges[target]
                    else:
                        va = self.blocks.get(target) or self._compile(target)
                    struct.pack_into("<QQ", self.mm, DISPATCH_TABLE + 16 * dispatch_slot(target), target, va)
                    pc = self.miss_va + 4                     # ic iallu; dsb; isb; re-dispatch
                elif imm == 8:
                    if not self._host_request(x):
                        exc = self.exception()
                        if exc is not None and on_exception is not None and on_exception(self, exc):
                            self.exit_code = None
                            self.clear_exception()
                            pc = self.miss_va                 # hvc #1: the driver sets the next state
                            continue
                        stop = ("ExitProcess" if exc is None else
                                f"UnhandledException 0x{exc['code']:x} at x86 0x{exc['rip']:x}")
                        break
                    pc = e.pc
                elif imm == 9:
                    name = self.ld.thunks.get(x[17], hex(x[17]))
                    # the call's return address is at the guest RSP (the bridge has not popped it)
                    ret = int.from_bytes(self.mem.read(x[4], 8), "little")
                    if on_unimplemented is not None and on_unimplemented(self, x, name):
                        x[17], x[4] = ret, x[4] + 8           # ret, with x[RAX] as the hook left it
                        pc = self.miss_va
                        continue
                    self.unimplemented = (name, ret)
                    raise Unimplemented(f"import {name}")
                elif imm == 2:
                    ec = e.esr >> 26 & 0x3F
                    if ec == 0x25:
                        stop = (f"GuestFault: access at 0x{e.far:x} outside mapped memory "
                                f"(x86 0x{self.x86_address_at(e.elr):x})")
                    else:
                        stop = f"VmExit: EL1 exception EC 0x{ec:x} at x86 0x{self.x86_address_at(e.elr):x}"
                    break
                elif imm in (HVC_SLOW_SSE, HVC_SLOW_STEP):
                    pc = self._slow_sse(x, where)
                elif imm == 4 and forced_stop:
                    forced_stop = False
                    self._set_budget()
                    if on_boundary:
                        on_boundary(self, x, where)
                    pc = e.pc                                   # on into the block from its entry
                elif imm == 4:
                    executed = struct.unpack_from("<Q", self.mm, MONITOR_DATA)[0]
                    if executed >= max_steps:
                        stop = "max_steps"
                        break
                    self._set_budget()
                    if self.machine is not None and self.machine.others_ready(self):
                        x[17] = where
                        self._save_thread(self.threads[self.current_tid], x)
                        yield "slice"                            # another VM's turn; this thread goes on after
                        x = self._load_thread(self.threads[self.current_tid])
                        pc = self.resume_va
                    elif len(self.threads) > 1:                 # the time slice ran out: another thread's turn
                        x[17] = where
                        switched = self._schedule(x, 0)
                        while switched is IDLE:
                            yield "idle"
                            switched = self._resume_idle(x)
                        assert switched is not None
                        x = switched
                        pc = self.resume_va
                    else:
                        pc = e.pc
                elif imm == HVC_CODE_WRITE:                     # a write faulted: behind translated code?
                    x[17] = 1 if self._code_written(x[17]) else 0
                    pc = e.pc
                elif imm == HVC_SCHEDULE:                       # a service handed the vCPU over (state.schedule)
                    st = self._process(STATE_OFF)
                    schedule = struct.unpack_from("<I", self.mm, st + STATE_SCHEDULE)[0]
                    struct.pack_into("<I", self.mm, st + STATE_SCHEDULE, 0)
                    switched = self._schedule(x, schedule)
                    while switched is IDLE:
                        yield "idle"
                        switched = self._resume_idle(x)
                    if switched is None:
                        self.exit_code = self.threads[self.current_tid].exit_code
                        stop = "ExitProcess"
                        break
                    x = switched
                    pc = self.resume_va
                elif imm & 0x40:
                    stop = f"UndefinedFlag: {'CZSOP'[imm & 7]} read while undefined at 0x{where:x}"
                    break
                else:
                    stop = f"VmExit: hvc #{imm}"
                    break
        except StopRun:
            stop = "stopped by on_dispatch"
        except (LiftError, DecodeError, GuestFault, Unimplemented, HandlerFailed, CompileError,
                BridgeRequestError) as ex:              # DecodeError: record frontier-...1791800847886016000 crashed
            stop = f"{type(ex).__name__}: {ex}"
        finally:
            if self._channel is not None:
                self._channel.leave()                            # the monitor is back on its pipe
                self._channel = None
            try:
                pin.write("quit\n")
                pin.flush()
            except BrokenPipeError:
                pass
            proc.wait(timeout=10)
        executed = struct.unpack_from("<Q", self.mm, MONITOR_DATA)[0]
        self.final = x
        return Result(self.exit_code, bytes(self.out), executed, len(self.addrs), self.calls(), stop,
                      self.calls_total(), self.calls_dropped())

    def _progress(self, before: tuple[int, Counter[int], Counter[tuple[int, int]], int, float],
                  x: list[int]) -> tuple[int, Counter[int], Counter[tuple[int, int]], int, float]:
        """One progress line on stderr: instructions and their rate since the last line, exits by
        hvc and the busiest exit sites since then, calls made, each thread's status, and the x86
        address the current thread is at."""
        executed = struct.unpack_from("<Q", self.mm, MONITOR_DATA)[0]
        calls = sum(self.calls_total().values())
        now = time.monotonic()
        exits = Counter(self.exits)
        exits.subtract(before[1])
        sites = Counter(self.exit_sites)
        sites.subtract(before[2])
        threads = Counter(t.status for t in self.threads.values())
        print(f"progress: {self.modules.image_name}: {executed} instructions ({(executed - before[0]) / max(now - before[4], 1e-6) / 1e6:.2f}M/s); "
              f"calls {calls} (+{calls - before[3]}); exits {dict(+exits)}; sites "
              + ", ".join(f"hvc {imm} at 0x{at:x} {n}" for (imm, at), n in (+sites).most_common(3))
              + f"; threads {dict(threads)}; thread {self.current_tid} at 0x{x[17]:x}", file=sys.stderr, flush=True)
        if self.slow_sse_classes:
            print("slow sse: " + ", ".join(f"{c} masks 0x{m:x} {n}" for (c, m), n in self.slow_sse_classes.most_common(8)),
                  file=sys.stderr, flush=True)
        print(f"time: {self.modules.image_name}: in run {self.run_ns / 1e9:.1f} s; host by exit "
              + ", ".join(f"hvc {k} {v / 1e9:.1f} s" for k, v in self.exit_ns.most_common(6)), file=sys.stderr, flush=True)
        return executed, Counter(self.exits), Counter(self.exit_sites), calls, now

    def code_census(self) -> tuple[int, int, int, dict[str, tuple[int, int]]]:
        """BLOCK_CENSUS's totals: (x86 instructions run, arm64 words run in instruction bodies, in block prologues and
        fall-through exits, and by class
        (x86 instructions, arm64 words)). Each block's words count once per execution, as if every word ran: an upper
        bound where a block branches out early."""
        x86 = body = prologue = 0
        by: dict[str, list[int]] = {}
        for off, insns, pro in self.block_census:
            n = struct.unpack_from("<Q", self.mm, off)[0]
            if not n:
                continue
            prologue += n * pro
            for ic, w in insns:
                x86 += n
                body += n * w
                row = by.setdefault(ic, [0, 0])
                row[0] += n
                row[1] += n * w
        return x86, body, prologue, {k: (v[0], v[1]) for k, v in by.items()}

    def close(self) -> None:
        self._final_census = self.thread_census()
        self._final_code_census = self.code_census() if self.block_census else None
        for r in self.mem.regions:
            if isinstance(r.data, memoryview):
                r.data.release()
        self.mem.regions.clear()
        self._view.release()
        self.mm.close()
        os.unlink(self.path)


def x86_flags(x: list[int], vm: VM | None = None) -> dict[str, bool | None]:
    """The x86 flags held in a register snapshot, None where undefined; with `vm`, AF and PF too (lazy, from the
    monitor data page; without it PF reads None)."""
    out = {f: None if x[REG_UNDEFINED_FLAGS] >> FLAG_BIT[f] & 1 else bool(x[FLAG_REG[f]]) for f in "CZSO"}
    out["P"] = None if vm is None or x[REG_UNDEFINED_FLAGS] >> FLAG_BIT["P"] & 1 else vm.pf()
    if vm is not None:
        out["A"] = None if x[REG_UNDEFINED_FLAGS] >> FLAG_BIT["A"] & 1 else vm.af()
    return out


@dataclass
class Lockstep:
    compared: int
    differences: list[str]
    vm: Result
    model: Result


LOCKSTEP_COUNT = 0x0123_4567_89AB_CDEF                  # QueryPerformanceCounter during lockstep


def lockstep(data: bytes, max_steps: int = 1_000_000, limit: int = 20) -> Lockstep:
    """Run `data` in the VM one x86 instruction per exit and in runtime.Runtime side by side;
    after every instruction compare RIP, the 16 GPRs, and each flag's defined value or its being
    undefined; at the end compare every guest region byte for byte."""
    from w2m.loader import load
    from w2m.runtime import Runtime

    model = Runtime(load(data), clock=lambda: LOCKSTEP_COUNT)   # one count for both sides
    vm = VM(load(data), lockstep=True, clock=lambda: LOCKSTEP_COUNT)
    diffs: list[str] = []
    n = [0]
    running = [True]

    def check(v: VM, x: list[int]) -> None:
        th = model.th
        where = f"after {n[0]} steps, x86 0x{th.rip:x}"
        if x[17] != th.rip:
            diffs.append(f"{where}: rip vm 0x{x[17]:x}")
        for i in range(16):
            if x[i] != th.regs[i]:
                diffs.append(f"{where}: r{i} vm 0x{x[i]:x} model 0x{th.regs[i]:x}")
        vf = x86_flags(x, v)
        for f in "CZSOPA":
            if vf[f] != th.flags[f]:
                diffs.append(f"{where}: {f}F vm {vf[f]} model {th.flags[f]}")
        if v.df() != th.df:
            diffs.append(f"{where}: DF vm {v.df()} model {th.df}")
        if len(diffs) > limit:
            raise CompileError(f"lockstep: more than {limit} differences")
        n[0] += 1
        if running[0]:
            running[0] = model.step()

    try:
        r = vm.run(max_steps=max_steps, timeout_ms=60_000, on_dispatch=check)
        for a, b in zip(vm.mem.regions, model.mem.regions, strict=False):
            if a.base == b.base and bytes(a.data) != bytes(b.data):
                k = next(i for i in range(len(b.data)) if a.data[i] != b.data[i])
                diffs.append(f"memory {a.name}: first difference at 0x{a.base + k:x}")
        m = Result(model.exit_code, bytes(model.kernel.out), model.steps, len(model.seen),
                   model.calls, "ExitProcess" if not running[0] else "running")
    finally:
        vm.close()
    return Lockstep(n[0], diffs, r, m)


@dataclass
class StateDifferential:
    cases: int
    instructions: int
    differences: list[str]


EDGE = (0, 1, 0x7F, 0x80, 0xFF, 0x7FFF, 0x8000, 0xFFFF, 0x7FFF_FFFF, 0x8000_0000, 0xFFFF_FFFF,
        0x7FFF_FFFF_FFFF_FFFF, 0x8000_0000_0000_0000, M64)


def state_differential(data: bytes, seed: int = 2026_10_08, per_instruction: int = 40,
                       limit: int = 20) -> StateDifferential:
    """Run every x86 instruction the model reaches in `data` from `per_instruction` random
    states, once in the VM (compiled) and once in runtime.Runtime, and compare RIP, the 16 GPRs,
    the five flags, and the stack page each memory operand touches.

    One program path checks few operand values (Sprint 030: a SUB carry compiled as signed
    less-than passed the skeleton's full lockstep run). Register and flag values are drawn from
    sign and width boundaries and uniform bits; every flag starts defined. A memory operand's
    base register is set so the effective address falls in the stack, its index register to 0;
    the compared stack window, and 32 bytes around each effective address, start as the same
    random bytes on both sides.
    """
    import random

    from w2m.loader import load
    from w2m.runtime import Runtime

    reached = Runtime(load(data))
    reached.run()
    addresses = sorted(reached.seen)
    model = Runtime(load(data))
    vm = VM(load(data), lockstep=True)
    rng = random.Random(seed)
    stack_mid = model.ld.stack_top - 0x8_0000
    cases = [(a, k) for a in addresses for k in range(per_instruction)]
    diffs: list[str] = []
    pos = [0]
    pending: list[tuple[int, int]] = []

    def value() -> int:
        r = rng.random()
        if r < 0.4:
            v = rng.choice(EDGE)
        elif r < 0.7:
            v = rng.getrandbits(rng.choice((8, 16, 32)))
        else:
            v = rng.getrandbits(64)
        return v ^ (rng.getrandbits(64) & 0xFFFF_FFFF_0000_0000 if rng.random() < 0.2 else 0)

    img = model.ld.image
    code = [(img.image_base + sec.virtual_address, sec.virtual_size)
            for sec in img.sections if "x" in sec.flags]
    spans: list[int] = []

    def place(addr: int, regs: list[int]) -> None:
        """Point memory operands into the stack; fill 32 bytes around each effective address
        with the same random bytes on both sides (not in code), and remember them to compare."""
        ins = model.cache.get(addr) or lift(model.mem.code_at(addr), addr)
        regs[4] = (stack_mid - 8 * rng.randrange(64)) & ~7
        spans.clear()
        for op in ins.ops:
            if op[0] == "addr":
                if op[2] is not None:
                    if op[3] is not None and op[3] != op[2]:
                        regs[op[3]] = 0
                    regs[op[2]] = (stack_mid + 8 * rng.randrange(-64, 64) - op[5]) & M64
                    ea = (regs[op[2]] + op[5]) & M64
                else:
                    ea = op[5] & M64
                if not any(b <= ea < b + n + 32 for b, n in code):
                    fill = rng.randbytes(32)
                    for mem in (vm.mem, model.mem):
                        mem.write(ea - 8, fill)
                    spans.append(ea - 8)

    def window(mem: Memory) -> bytes:
        return mem.read(stack_mid - 0x1000, 0x2000)

    noise = rng.randbytes(0x2000)                      # zeros would hide a store of zero bytes
    vm.mem.write(stack_mid - 0x1000, noise)
    model.mem.write(stack_mid - 0x1000, noise)

    def check(v: VM, x: list[int]) -> None:
        if pending:
            addr, k = pending.pop()
            th = model.th
            where = f"case {k} of x86 0x{addr:x}"
            got = [x[17], *x[:16]]
            want = [th.rip, *th.regs]
            names = ["rip", *(f"r{i}" for i in range(16))]
            diffs.extend(f"{where}: {n} vm 0x{g:x} model 0x{w:x}"
                         for n, g, w in zip(names, got, want, strict=True) if g != w)
            vf = x86_flags(x, v)
            diffs.extend(f"{where}: {f}F vm {vf[f]} model {th.flags[f]}"
                         for f in "CZSOPA" if vf[f] != th.flags[f])
            if v.df() != th.df:
                diffs.append(f"{where}: DF vm {v.df()} model {th.df}")
            if window(v.mem) != window(model.mem):
                diffs.append(f"{where}: stack window differs")
            for ea in spans:
                if v.mem.read(ea, 32) != model.mem.read(ea, 32):
                    diffs.append(f"{where}: memory at 0x{ea + 8:x} differs")
            if len(diffs) > limit:
                raise StopRun
        if pos[0] == len(cases):
            raise StopRun
        addr, k = cases[pos[0]]
        pos[0] += 1
        regs = [value() for _ in range(16)]
        place(addr, regs)
        flags = {f: bool(rng.getrandbits(1)) for f in "CZSOPA"}
        df = bool(rng.getrandbits(1))
        th = model.th
        th.regs, th.rip, th.flags, th.df = list(regs), addr, dict(flags), df
        try:
            model.step()
        except Exception as e:                          # the model's stop is part of the case
            diffs.append(f"case {k} of x86 0x{addr:x}: model stopped: {type(e).__name__}: {e}")
            raise StopRun from None
        x[:16] = regs
        x[17] = addr
        for f in "CZSOP":
            x[FLAG_REG[f]] = int(flags[f])
        x[REG_UNDEFINED_FLAGS] = 0
        v.set_af_df(flags["A"], df, flags["P"])
        pending.append((addr, k))

    try:
        vm.run(max_steps=1 << 40, timeout_ms=60_000, on_dispatch=check)
    finally:
        vm.close()
    return StateDifferential(pos[0], len(addresses), diffs)


def record_translations(vm: VM, image_sha256: str, runs_root: Path, profile: str) -> Path:
    """One substrate run (session purpose `compilation`) holding the blocks `vm` translated:
    per block CODE_TARGET_DISCOVERED (the entry point is `static`, every other start
    `runtime_transfer`; stage `execution`), then TRANSLATION_PUBLISHED for its guest range at code
    version 0, under the decoder spec version the lifter decodes with (Sprint 035; vocabulary 0.5).
    """
    import time
    from collections.abc import AsyncIterator

    import msgspec
    from substrate.api import threshold_count

    from w2m import VOCAB_VERSION, events
    from w2m.x86dec_tables import SPEC_VERSION
    from w2m.record import producer, run_with_provenance

    run_id = f"translations-{time.time_ns()}"
    blocks = list(vm.translations)

    async def translator(_inp: Any) -> AsyncIterator[msgspec.Struct]:
        t0 = time.monotonic()
        yield events.validate(events.SESSION_INIT(
            vocab_version=VOCAB_VERSION, profile=profile, contract_version=SPEC_VERSION,
            harness_version="0.0.1", purpose="compilation"))
        for start, end, discovery in blocks:
            yield events.validate(events.CODE_TARGET_DISCOVERED(
                image_sha256=image_sha256, guest_address=start, discovery=discovery, stage="execution"))
            yield events.validate(events.TRANSLATION_PUBLISHED(
                translation_id=f"tr-{image_sha256[7:19]}-{start:x}", image_sha256=image_sha256,
                guest_range_start=start, guest_range_end=end, code_version=0, stage="execution",
                contract_version=SPEC_VERSION, artifact_id=f"vm-code-cache:{vm.blocks[start]:x}"))
        yield events.validate(events.SESSION_COMPLETED(
            outcome="completed", discrepancy_count=0, accepted_count=0,
            elapsed_seconds=round(time.monotonic() - t0, 3)))

    def topology(b: Any) -> None:
        b.baseline(image_sha256=image_sha256, runtime_image=vm.runtime.sha256, blocks=len(blocks))
        b.producer_kind("translator", schemas=list(events.HARNESS_EVENTS), schema_version=1,
                        factory=lambda: producer(translator))
        b.initial("translator", input=None)
        b.termination(threshold_count("substrate.ProducerCompleted", 1))

    result = run_with_provenance(runs_root / run_id, topology)
    return Path(result.record_root)
