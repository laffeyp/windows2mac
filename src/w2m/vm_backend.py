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

import mmap
import os
import struct
import subprocess
import tempfile
from bisect import bisect_right
from collections.abc import Callable
from dataclasses import dataclass
from pathlib import Path
from typing import Literal

from w2m import a64_encoder as a64
from w2m import guest_runtime
from w2m.kernel32 import M64, HandlerFailed, Thread, Unimplemented, monotonic_count
from w2m.lift import Lifted, LiftError, Op, lift
from w2m.loader import GuestFault, Loaded, Memory, Region
from w2m.runtime import Result

ROOT = Path(__file__).resolve().parents[2]
MONITOR_SOURCE = ROOT / "tools" / "vm_monitor" / "vm_monitor.c"
MONITOR_ENTITLEMENTS = ROOT / "tools" / "vm_monitor" / "entitlements.plist"
MONITOR_BINARY = ROOT / "build" / "vm_monitor"

IPA_BASE = 0x1000_0000
MONITOR_VA = 0x7F00_0000_0000
PAGE = 0x1000
# monitor layout (offsets into the shared file and from MONITOR_VA)
MONITOR_DATA = 0x00000              # [0] instructions executed, [8] limit, [16] dispatch table VA
VECTORS = 0x01000               # vector table, 16 slots of 0x80
DISPATCHER = 0x01800
DISPATCH_TABLE = 0x02000             # 4096 entries of (x86 address, code VA)
DISPATCH_ENTRIES = 4096
TRANSLATION_TABLES = 0x12000            # stage-1 translation tables
TRANSLATION_TABLES_SIZE = 0x40000
CODE_CACHE = 0x60000
CODE_CACHE_SIZE = 0x400000
RUNTIME_IMAGE = 0x460000              # guest runtime (tools/guest_runtime), linked at MONITOR_VA + this
RUNTIME_IMAGE_SIZE = 0x20000
RUNTIME_STACK = 0x480000              # the runtime's AArch64 stack (SP set by each bridge)
RUNTIME_STACK_SIZE = 0x10000
MONITOR_SIZE = RUNTIME_STACK + RUNTIME_STACK_SIZE
# process area: Windows-visible state at specs/guest_state.json offsets (Sprint 034)
PROCESS_VA = 0x7FFE_0000_0000
TEB_OFF, PEB_OFF, STATE_OFF, TRANSPORT_OFF, RING_OFF = 0x0000, 0x2000, 0x3000, 0x4000, 0x5000
RING_SIZE = 0x4000
PROCESS_SIZE = RING_OFF + RING_SIZE
STDOUT_HANDLE = 0x74
# kernel32 services in the guest runtime: import name -> (C symbol, sets RAX)
SERVICES = {"GetCurrentThreadId": ("k32_GetCurrentThreadId", True), "SetLastError": ("k32_SetLastError", False),
            "GetLastError": ("k32_GetLastError", True), "GetStdHandle": ("k32_GetStdHandle", True),
            "WriteFile": ("k32_WriteFile", True), "ExitProcess": ("k32_ExitProcess", False),
            "QueryPerformanceCounter": ("k32_QueryPerformanceCounter", True)}
HOST_WRITE, HOST_EXIT, HOST_CLOCK, HOST_UNIMPLEMENTED = 1, 2, 3, 4
Discovery = Literal["static", "runtime_transfer"]           # vocabulary 0.5 enum `discovery`, the values used

SYS_REG_ID = {"SCTLR_EL1": 0xC080, "TTBR0_EL1": 0xC100, "TCR_EL1": 0xC102, "MAIR_EL1": 0xC510,
       "VBAR_EL1": 0xC600}
PAGE_ATTRIBUTES = (3 << 8) | (1 << 10)                 # AttrIndx 0 (MAIR byte 0 = 0xFF, Normal WB), ISH, AF
SCTLR = 0x30D01805                          # M, C, I; alignment checks off (Sprint 012 value)
TCR = 16 | (1 << 8) | (1 << 10) | (3 << 12) | (1 << 23) | (1 << 32)   # T0SZ 16, WB, ISH, EPD1, IPS 36
CPSR_EL1H = 0x3C5

TEMP_REGS = (18, 25, 26, 27, 29, 30)
REG_CF, REG_ZF, REG_SF, REG_OF, REG_PF = 19, 20, 21, 22, 23
REG_UNDEFINED_FLAGS = 24
FLAG_BIT = {"C": 0, "Z": 1, "S": 2, "O": 3, "P": 4}
U_CF, U_ZF, U_SF, U_OF, U_PF = (1 << FLAG_BIT[f] for f in "CZSOP")     # bits of REG_UNDEFINED_FLAGS
FLAG_REG = {"C": REG_CF, "Z": REG_ZF, "S": REG_SF, "O": REG_OF, "P": REG_PF}
MAX_BLOCK_INSNS = 64


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


def _set_zf_sf_pf(a: Assembler, r: int, bits: int) -> None:
    a.emit(a64.cmpi(r, 0), a64.cset(REG_ZF, "eq"), a64.ubfx(REG_SF, r, bits - 1, 1),
           a64.ubfx(16, r, 0, 8), a64.eor(16, 16, 16, a64.LSR, 4), a64.eor(16, 16, 16, a64.LSR, 2),
           a64.eor(16, 16, 16, a64.LSR, 1), a64.ubfx(16, 16, 0, 1), a64.cmpi(16, 0),
           a64.cset(REG_PF, "eq"))


def _set_flags(a: Assembler, kind: str, ra: int, rb: int, rr: int, bits: int) -> None:
    """runtime.Runtime._set_flags, compiled."""
    if kind == "mul":                                          # SDM MUL: SF ZF AF PF undefined
        a.emit(a64.cmpi(rr, 0), a64.cset(REG_CF, "ne"), a64.mov(REG_OF, REG_CF))
        _mark_defined(a, U_CF | U_OF)
        _mark_undefined(a, U_ZF | U_SF | U_PF)
        return
    if kind == "shr":
        done, undef_o, zsp = a.label(), a.label(), a.label()
        a.emit(a64.ubfx(16, rb, 0, 6 if bits == 64 else 5))
        a.branch(lambda off: a64.cbz(16, off), done)           # count 0: flags unaffected
        a.emit(a64.subi(17, 16, 1), a64.lsrv(17, ra, 17), a64.ubfx(REG_CF, 17, 0, 1), a64.cmpi(16, 1))
        a.branch(lambda off: a64.bcond("ne", off), undef_o)
        a.emit(a64.ubfx(REG_OF, ra, bits - 1, 1))
        _mark_defined(a, U_CF | U_ZF | U_SF | U_OF | U_PF)
        a.branch(a64.b, zsp)
        a.bind(undef_o)
        _mark_defined(a, U_CF | U_ZF | U_SF | U_PF)
        _mark_undefined(a, U_OF)
        a.bind(zsp)
        _set_zf_sf_pf(a, rr, bits)
        a.bind(done)
        return
    if kind == "logic":
        a.emit(a64.mov(REG_CF, a64.ZR), a64.mov(REG_OF, a64.ZR))
        _mark_defined(a, U_CF | U_ZF | U_SF | U_OF | U_PF)
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
    _set_zf_sf_pf(a, rr, bits)


def _exit_if_undefined(a: Assembler, f: str) -> None:
    """Exit with HVC 0x40|bit when flag f is undefined (runtime.Runtime._flag)."""
    a.emit(a64.tbz(REG_UNDEFINED_FLAGS, FLAG_BIT[f], 8), a64.hvc(0x40 | FLAG_BIT[f]))


def _eval_condition(a: Assembler, cc: str) -> None:
    """x16 = condition `cc`, reading flags in runtime.Runtime._cond's order."""
    neg = cc.startswith("n")
    key = cc[1:] if neg else cc
    single = {"o": "O", "b": "C", "z": "Z", "s": "S", "p": "P"}
    if key in single:
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
    if k == "set":
        return [op[2][1]] if op[2][0] == "m" else []
    return []


def _temps_written(op: Op) -> list[int]:
    return [op[1]] if op[0] in ("imm", "addr", "ld", "alu") else []


def compile_insn(a: Assembler, ins: Lifted, exit_va: int) -> bool:
    """Append `ins` to `a`. A taken branch puts the x86 target in x17 and jumps to `exit_va`.
    Returns True when control never falls through (an unconditional branch)."""
    last: dict[int, int] = {}
    for i, op in enumerate(ins.ops):
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
        else:
            a.emit(a64.str_(loc[2], rt, reg_of(loc[1])))

    ends = False
    for i, op in enumerate(ins.ops):
        for t in _temps_written(op):
            if not free:
                raise CompileError(f"more than {len(TEMP_REGS)} live temporaries at 0x{ins.address:x}")
            reg[t] = free.pop(0)
        k = op[0]
        if k == "imm":
            a.emit(a64.mov_imm(reg_of(op[1]), op[2]))
        elif k == "addr":
            _, d, base, index, scale, disp, abits = op
            rd = reg_of(d)
            a.emit(a64.mov_imm(rd, disp & M64))
            if base is not None:
                a.emit(a64.add(rd, rd, base))
            if index is not None:
                a.emit(a64.add(rd, rd, index, a64.LSL, scale.bit_length() - 1))
            _mask(a, rd, abits)
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
            else:
                a.emit(a64.ldr(loc[2], rt, reg_of(loc[1])))
        elif k == "st":
            store(op[1], reg_of(op[2]))
        elif k == "alu":
            _, d, o, x, y, bits = op
            rd, rx, ry = reg_of(d), reg_of(x), reg_of(y)
            if o in ("add", "sub", "and", "or", "xor"):
                f = {"add": a64.add, "sub": a64.sub, "and": a64.and_, "or": a64.orr, "xor": a64.eor}[o]
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
            else:
                raise CompileError(f"alu {o}")
            _mask(a, rd, bits)
        elif k == "flags":
            _, kind, x, y, r, bits = op
            _set_flags(a, kind, reg_of(x), reg_of(y), reg_of(r), bits)
        elif k == "set":
            _eval_condition(a, op[1])
            store(op[2], 16)
        elif k == "br":
            if op[1] == "always":
                a.emit(a64.mov(17, reg_of(op[2])))
                a.branch(a64.b, exit_va)
                ends = True
            else:
                skip = a.label()
                _eval_condition(a, op[1])
                a.branch(lambda off: a64.cbz(16, off), skip)
                a.emit(a64.mov(17, reg_of(op[2])))
                a.branch(a64.b, exit_va)
                a.bind(skip)
        else:
            raise CompileError(f"operation {k}")
        for t in [*_temps_read(op), *_temps_written(op)]:
            if t in reg and last.get(t, -1) <= i:
                free.append(reg.pop(t))
    return ends


# ---------------------------------------------------------------------------------------------
# host driver

@dataclass
class VcpuExit:
    kind: str
    syndrome: int
    pc: int
    esr: int
    far: int
    elr: int
    x: list[int]


def _write_translation_tables(mm: mmap.mmap, maps: list[tuple[int, int, int]]) -> int:
    """Write 4-level stage-1 tables for (va, ipa, size) maps into the TRANSLATION_TABLES area; return the
    root table's IPA. Descriptor layout: Arm ARM D8.3 (VMSAv8-64 translation table format)."""
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
        for p in range(0, size, PAGE):
            v = va + p
            off = TRANSLATION_TABLES
            for level in (0, 1, 2):
                off = table_at(off, (v >> (39 - 9 * level)) & 0x1FF)
            struct.pack_into("<Q", mm, off + 8 * ((v >> 12) & 0x1FF), (ipa + p) | PAGE_ATTRIBUTES | 0x3)
    return IPA_BASE + TRANSLATION_TABLES


def _round_up(n: int, to: int = PAGE) -> int:
    return (n + to - 1) & ~(to - 1)


class VM:
    """One guest thread of `loaded` in a fresh Hypervisor.framework VM.

    lockstep=True compiles one x86 instruction per block and sends every block exit to the host,
    so `on_dispatch` sees the guest state after each instruction (and after each kernel32 call).
    """

    def __init__(self, loaded: Loaded, tid: int = 4, lockstep: bool = False,
                 clock: Callable[[], int] = monotonic_count, last_error_source: str | None = None) -> None:
        self.ld = loaded
        self.lockstep = lockstep
        self.clock = clock
        self.th = Thread(tid)
        self.exit_code: int | None = None
        self.out = bytearray()
        regions = [r for r in loaded.memory.regions if r.name != "thunks"]
        size = MONITOR_SIZE + PROCESS_SIZE
        placed: list[tuple[Region, int]] = []
        for r in regions:
            placed.append((r, size))
            size += _round_up(len(r.data))
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
        self.runtime = guest_runtime.build(MONITOR_VA + RUNTIME_IMAGE, {
            "W2M_TEB_VA": PROCESS_VA + TEB_OFF, "W2M_PEB_VA": PROCESS_VA + PEB_OFF,
            "W2M_TRANSPORT_VA": PROCESS_VA + TRANSPORT_OFF, "W2M_RING_VA": PROCESS_VA + RING_OFF},
            last_error_source)
        if self.runtime.end > MONITOR_VA + RUNTIME_IMAGE + RUNTIME_IMAGE_SIZE:
            raise CompileError(f"guest runtime image ends at 0x{self.runtime.end:x}, past its area")
        for va, data in self.runtime.segments:
            off = va - MONITOR_VA
            self.mm[off:off + len(data)] = data
        self._init_process(tid)
        maps = [(MONITOR_VA, IPA_BASE, MONITOR_SIZE), (PROCESS_VA, IPA_BASE + MONITOR_SIZE, PROCESS_SIZE)]
        maps += [(r.base, IPA_BASE + off, _round_up(len(r.data))) for r, off in placed]
        self.ttbr = _write_translation_tables(self.mm, maps)
        self.code_cursor = CODE_CACHE
        self.blocks: dict[int, int] = {}                 # x86 address -> code VA
        self.addrs: set[int] = set()                     # x86 instructions compiled into blocks
        self.pc_map: list[tuple[int, int]] = []           # (code VA, x86 address), sorted
        self.translations: list[tuple[int, int, Discovery]] = []   # (guest start, guest end, discovery)
        self._write_monitor()
        self.bridges = {thunk: self._bridge(name) for thunk, name in loaded.thunks.items()}

    def _process(self, off: int) -> int:
        """File offset of a process-area offset."""
        return MONITOR_SIZE + off

    def _init_process(self, tid: int) -> None:
        """TEB and PEB fields at specs/guest_state.json offsets; the service ring's header."""
        teb, peb = PROCESS_VA + TEB_OFF, PROCESS_VA + PEB_OFF
        stack = next(r for r in self.mem.regions if r.name == "stack")
        t = self._process(TEB_OFF)
        for field_off, value in ((0x08, self.ld.stack_top), (0x10, stack.base), (0x30, teb),
                                 (0x40, 1), (0x48, tid), (0x60, peb)):
            struct.pack_into("<Q", self.mm, t + field_off, value)
        struct.pack_into("<Q", self.mm, self._process(PEB_OFF) + 0x10, self.ld.image.image_base)
        struct.pack_into("<QQ", self.mm, self._process(RING_OFF), 0, (RING_SIZE - 16) // 24)

    def _bridge(self, import_name: str) -> int:
        """AArch64 stub for one import: save the guest GPRs in the state block, pass RCX, RDX, R8,
        R9 and RSP in x0-x4, call the C service on the runtime stack, put its result in RAX unless
        the service is void, restore the GPRs, and do RET's pop (Microsoft x64 calling convention;
        AAPCS64 keeps x19-x28, so the flag registers survive). An import with no service exits
        with hvc #9."""
        dll, fn = import_name.split("!", 1)
        a = Assembler(MONITOR_VA + self.code_cursor)
        service = SERVICES.get(fn) if dll.upper() == "KERNEL32.DLL" else None
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
            a.emit(a64.mov_imm(16, self.runtime.symbols[symbol]), a64.blr(16))
            a.emit(a64.mov_imm(16, state))
            if sets_rax:
                a.emit(a64.str_(8, 0, 16, 0))
            for i in range(0, 16, 2):
                a.emit(a64.ldp(i, i + 1, 16, 8 * i))
            a.emit(a64.ldr(8, 17, 4, 0), a64.addi(4, 4, 8))
            a.branch(a64.b, self.miss_va if self.lockstep else self.dispatch_va)
        words = a.finish()
        va = MONITOR_VA + self.code_cursor
        self._write_words(self.code_cursor, words)
        self.code_cursor += 4 * len(words)
        return va

    def _host_request(self, x: list[int]) -> bool:
        """Service the transport page (hvc #8). False when the guest exited."""
        t = self._process(TRANSPORT_OFF)
        op, a0, a1 = struct.unpack_from("<QQQ", self.mm, t)
        if op == HOST_WRITE:
            self.out += self.mem.read(a0, a1)
            result = a1
        elif op == HOST_EXIT:
            self.exit_code = a0 & 0xFFFFFFFF
            return False
        elif op == HOST_CLOCK:
            result = self.clock() & M64
        elif op == HOST_UNIMPLEMENTED:
            name = guest_runtime.api_ids().get(a0, f"api {a0}")
            raise Unimplemented(f"{name} argument form 0x{a1:x}")
        else:
            raise CompileError(f"guest runtime: unknown host request {op}")
        struct.pack_into("<Q", self.mm, t + 56, result)
        return True

    def calls(self) -> list[str]:
        """The service calls the guest runtime logged, in order."""
        count, capacity = struct.unpack_from("<QQ", self.mm, self._process(RING_OFF))
        if count > capacity:
            raise CompileError(f"service ring overflowed: {count} calls, capacity {capacity}")
        names = guest_runtime.api_ids()
        return [names[struct.unpack_from("<Q", self.mm, self._process(RING_OFF) + 16 + 24 * i)[0]]
                for i in range(count)]

    def teb_u32(self, offset: int) -> int:
        return int(struct.unpack_from("<I", self.mm, self._process(TEB_OFF) + offset)[0])

    # monitor code ----------------------------------------------------------------------------
    def _write_monitor(self) -> None:
        struct.pack_into("<QQQ", self.mm, MONITOR_DATA, 0, 0, MONITOR_VA + DISPATCH_TABLE)
        vec = Assembler(MONITOR_VA + VECTORS)
        for _ in range(16):
            vec.emit(a64.hvc(2), *[a64.b(0)] * 31)     # each 0x80 slot: hvc #2, then spin
        self._write_words(VECTORS, vec.finish())
        d = Assembler(MONITOR_VA + DISPATCHER)
        top, miss = d.label(), d.label()
        d.bind(top)
        d.emit(a64.ldr(8, 16, 28, 16), a64.ubfx(18, 17, 0, 12), a64.add(16, 16, 18, a64.LSL, 4),
               a64.ldp(18, 16, 16), a64.cmp(18, 17))
        d.branch(lambda off: a64.bcond("ne", off), miss)
        d.emit(a64.br(16))
        d.bind(miss)
        self.miss_va = d.here()
        d.emit(a64.hvc(1), a64.IC_IALLU, a64.DSB_ISH, a64.ISB)
        d.branch(a64.b, top)
        self.dispatch_va = MONITOR_VA + DISPATCHER
        self._write_words(DISPATCHER, d.finish())
        empty = struct.pack("<QQ", M64, self.miss_va)
        self.mm[DISPATCH_TABLE:DISPATCH_TABLE + 16 * DISPATCH_ENTRIES] = empty * DISPATCH_ENTRIES

    def _write_words(self, off: int, words: list[int]) -> None:
        self.mm[off:off + 4 * len(words)] = struct.pack(f"<{len(words)}I", *words)

    # blocks ----------------------------------------------------------------------------------
    def _compile(self, start: int) -> int:
        a = Assembler(MONITOR_VA + self.code_cursor)
        insns: list[Lifted] = []
        addr = start
        while len(insns) < (1 if self.lockstep else MAX_BLOCK_INSNS):
            if insns and addr in self.ld.thunks:
                break
            try:
                ins = lift(self.mem.code_at(addr), addr)
            except (LiftError, GuestFault):
                if not insns:
                    raise
                break
            insns.append(ins)
            addr += ins.length
            if any(op[0] == "br" for op in ins.ops):
                break
        exit_va = self.miss_va if self.lockstep else self.dispatch_va
        a.emit(a64.ldp(17, 18, 28, 0), a64.addi(17, 17, len(insns)), a64.str_(8, 17, 28, 0),
               a64.cmp(17, 18), a64.bcond("ls", 8), a64.hvc(4))
        pc_marks: list[tuple[int, int]] = [(a.here(), start)]
        ended = False
        for ins in insns:
            pc_marks.append((a.here(), ins.address))
            ended = compile_insn(a, ins, exit_va)
        if not ended:
            a.emit(a64.mov_imm(17, addr))
            a.branch(a64.b, exit_va)
        words = a.finish()
        if self.code_cursor + 4 * len(words) > CODE_CACHE + CODE_CACHE_SIZE:
            raise CompileError("code cache full")
        self._write_words(self.code_cursor, words)
        va = MONITOR_VA + self.code_cursor
        self.code_cursor += 4 * len(words)
        self.pc_map.extend(pc_marks)
        self.pc_map.sort()
        self.addrs.update(i.address for i in insns)
        self.blocks[start] = va
        self.translations.append((start, addr, "static" if start == self.ld.entry else "runtime_transfer"))
        return va

    def x86_address_at(self, pc: int) -> int:
        i = bisect_right(self.pc_map, (pc, M64)) - 1
        return self.pc_map[i][1] if i >= 0 else 0

    # run -------------------------------------------------------------------------------------
    def run(self, max_steps: int = 1_000_000, timeout_ms: int = 10_000,
            on_dispatch: Callable[[VM, list[int]], None] | None = None) -> Result:
        """Run until the guest exits or stops. `on_dispatch` sees (and may rewrite) the register
        snapshot at each dispatch miss; x[17] is the x86 address about to run."""
        proc = subprocess.Popen([str(build_monitor()), self.path, f"{IPA_BASE:x}", f"{self.size:x}"],
                                stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True)
        assert proc.stdin and proc.stdout
        pin, pout = proc.stdin, proc.stdout

        def cmd(c: str) -> list[str]:
            pin.write(c + "\n")
            pin.flush()
            reply = pout.readline().split()
            if not reply or reply[0] == "err":
                raise RuntimeError(f"vm_monitor: {c.split()[0]} -> {reply}")
            return reply

        struct.pack_into("<Q", self.mm, MONITOR_DATA + 8, max_steps)
        for name, v in (("MAIR_EL1", 0xFF), ("TCR_EL1", TCR), ("TTBR0_EL1", self.ttbr),
                        ("VBAR_EL1", MONITOR_VA + VECTORS), ("SCTLR_EL1", SCTLR)):
            cmd(f"sys {SYS_REG_ID[name]:x} {v:x}")
        x = [0] * 31
        x[4] = self.ld.stack_top - 0x100 - 8                  # as runtime.Runtime: RSP % 16 == 8
        x[17] = self.ld.entry
        x[REG_UNDEFINED_FLAGS] = 0x1F                                       # every flag undefined at entry
        x[28] = MONITOR_VA + MONITOR_DATA
        pc = self.dispatch_va
        stop = "timeout"
        try:
            while True:
                cmd(f"set {pc:x} {CPSR_EL1H:x} " + " ".join(f"{v:x}" for v in x))
                r = cmd(f"run {timeout_ms}")
                e = VcpuExit(r[1], int(r[2], 16), int(r[3], 16), int(r[5], 16), int(r[6], 16),
                         int(r[7], 16), [int(v, 16) for v in r[8:39]])
                x = e.x
                if e.kind == "cancel":
                    stop = "timeout"
                    break
                if e.kind != "hvc":
                    stop = f"VmExit: {e.kind} syndrome 0x{e.syndrome:x} at pc 0x{e.pc:x}"
                    break
                imm = e.syndrome & 0xFFFF
                where = self.x86_address_at(e.pc - 4)
                if imm == 1:
                    if on_dispatch:
                        on_dispatch(self, x)                  # may replace the state in x
                    target = x[17]
                    if target in self.bridges:
                        va = self.bridges[target]
                    else:
                        va = self.blocks.get(target) or self._compile(target)
                    struct.pack_into("<QQ", self.mm, DISPATCH_TABLE + 16 * (target & (DISPATCH_ENTRIES - 1)),
                                     target, va)
                    pc = self.miss_va + 4                     # ic iallu; dsb; isb; re-dispatch
                elif imm == 8:
                    if not self._host_request(x):
                        stop = "ExitProcess"
                        break
                    pc = e.pc
                elif imm == 9:
                    raise Unimplemented(f"import {self.ld.thunks.get(x[17], hex(x[17]))}")
                elif imm == 2:
                    ec = e.esr >> 26 & 0x3F
                    if ec == 0x25:
                        stop = (f"GuestFault: access at 0x{e.far:x} outside mapped memory "
                                f"(x86 0x{self.x86_address_at(e.elr):x})")
                    else:
                        stop = f"VmExit: EL1 exception EC 0x{ec:x} at x86 0x{self.x86_address_at(e.elr):x}"
                    break
                elif imm == 4:
                    stop = "max_steps"
                    break
                elif imm & 0x40:
                    stop = f"UndefinedFlag: {'CZSOP'[imm & 7]} read while undefined at 0x{where:x}"
                    break
                else:
                    stop = f"VmExit: hvc #{imm}"
                    break
        except StopRun:
            stop = "stopped by on_dispatch"
        except (LiftError, GuestFault, Unimplemented, HandlerFailed, CompileError) as ex:
            stop = f"{type(ex).__name__}: {ex}"
        finally:
            try:
                pin.write("quit\n")
                pin.flush()
            except BrokenPipeError:
                pass
            proc.wait(timeout=10)
        executed = struct.unpack_from("<Q", self.mm, MONITOR_DATA)[0]
        self.final = x
        return Result(self.exit_code, bytes(self.out), executed, len(self.addrs), self.calls(), stop)

    def close(self) -> None:
        for r in self.mem.regions:
            if isinstance(r.data, memoryview):
                r.data.release()
        self.mem.regions.clear()
        self._view.release()
        self.mm.close()
        os.unlink(self.path)


def x86_flags(x: list[int]) -> dict[str, bool | None]:
    """The x86 flags held in a register snapshot, None where undefined."""
    return {f: None if x[REG_UNDEFINED_FLAGS] >> FLAG_BIT[f] & 1 else bool(x[FLAG_REG[f]]) for f in "CZSOP"}


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
        vf = x86_flags(x)
        for f in "CZSOP":
            if vf[f] != th.flags[f]:
                diffs.append(f"{where}: {f}F vm {vf[f]} model {th.flags[f]}")
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
            vf = x86_flags(x)
            diffs.extend(f"{where}: {f}F vm {vf[f]} model {th.flags[f]}"
                         for f in "CZSOP" if vf[f] != th.flags[f])
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
        flags = {f: bool(rng.getrandbits(1)) for f in "CZSOP"}
        th = model.th
        th.regs, th.rip, th.flags = list(regs), addr, dict(flags)
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
    import asyncio
    import time
    from collections.abc import AsyncIterator
    from typing import Any

    import msgspec
    from substrate.api import Runtime as SubstrateRuntime, threshold_count

    from w2m import VOCAB_VERSION, events
    from w2m.x86dec_tables import SPEC_VERSION

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
                        factory=lambda: translator)
        b.initial("translator", input=None)
        b.termination(threshold_count("substrate.ProducerCompleted", 1))

    result = asyncio.run(SubstrateRuntime(runs_root / run_id).run(topology))
    return Path(result.record_root)
