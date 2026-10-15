# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""successor_scan_judge.py — the successor scan in C (vm_compiler.c vmc_live_in, vmc_live_out; Sprint 082) judged
against the host's (vm_backend.VM._flags_live_in, _flags_live_out), both run over one model of guest memory.

A `Model` holds mapped pages and, among them, the pages behind translated code. The host's functions run on it as
they run on a VM (a stand-in with the attributes they read: the thunks, the code pages, a lift, insn_flags); the C
reads it through its vmc_memory callbacks. For each scan the judge compares the flags and the pages scanned:

- `equal`: the C found a form for every instruction the host's scan read; flags and pages equal (anything else is
  `differ`, a defect);
- `superset`: some instruction the host read has no form in the table, so the C stopped there with its undecided
  flags live; its flags hold all the host's (anything short of them is `short`, a defect).

Two sources: the record's memory (every recorded instruction's bytes at its address; tests/data/scan_samples.txt.gz
keeps the instructions a sample of scans read), and spec-row random encodings laid out with direct jumps between
them (from outside the table's sources, KIT_DIARY #59).
"""

from __future__ import annotations

import ctypes
import random
import sys
from collections import Counter
from pathlib import Path
from typing import Any, cast

ROOT = Path(__file__).resolve().parent.parent
sys.path[:0] = [str(ROOT / "src"), str(ROOT / "scripts"), str(ROOT / "tests")]

from w2m import vm_backend, x86dec  # noqa: E402
from w2m.lift import LiftError, lift  # noqa: E402
from w2m.loader import GuestFault  # noqa: E402

PAGE = vm_backend.PAGE
U64, INT = ctypes.c_uint64, ctypes.c_int
FILL = 0xCC                                              # bytes no recorded instruction covers (int3, MSVC's padding)


class Model:
    """Guest memory for a scan: mapped pages, and those of them behind translated code."""

    def __init__(self, thunks: tuple[int, ...] = ()) -> None:
        self.pages: dict[int, bytearray] = {}
        self.code: set[int] = set()
        self.thunks = tuple(sorted(thunks))

    def place(self, address: int, data: bytes, code: bool = True) -> None:
        for i, b in enumerate(data):
            page = (address + i) & ~(PAGE - 1)
            if page not in self.pages:
                self.pages[page] = bytearray([FILL]) * PAGE
            if code:
                self.code.add(page)
            self.pages[page][(address + i) & (PAGE - 1)] = b

    def read(self, address: int, limit: int, pages: set[int] | dict[int, bytearray]) -> bytes:
        """Up to `limit` bytes from `address` while their pages are in `pages`."""
        out = bytearray()
        while len(out) < limit and (address + len(out)) & ~(PAGE - 1) in pages:
            a = address + len(out)
            page = self.pages[a & ~(PAGE - 1)]
            take = min(limit - len(out), PAGE - (a & (PAGE - 1)))
            out += page[a & (PAGE - 1):(a & (PAGE - 1)) + take]
        return bytes(out)


class _Host:
    """What VM._flags_live_in and _flags_live_out read of a VM, over a Model."""

    def __init__(self, model: Model) -> None:
        self.model = model
        self.ld = type("Loaded", (), {"thunks": set(model.thunks)})()
        self.code_pages: dict[int, set[int]] = {}
        self._scan_log: list[tuple[int, int]] | None = None
        self.lifts: dict[int, Any] = {}

    def _lift(self, addr: int) -> Any:
        if addr not in self.lifts:
            if addr & ~(PAGE - 1) not in self.model.pages:
                raise GuestFault(f"instruction fetch at 0x{addr:x} outside mapped memory")
            self.lifts[addr] = lift(self.model.read(addr, 15, self.model.pages), addr)
        return self.lifts[addr]

    def _remember(self, ins: Any) -> None:
        pass

    def _flags_live_in(self, owner: int, addr: int) -> int:
        return vm_backend.VM._flags_live_in(cast(Any, self), owner, addr)

    def _flags_live_out(self, start: int, last: Any, end: int) -> int:
        return vm_backend.VM._flags_live_out(cast(Any, self), start, last, end)

    def _insn_flags(self, ins: Any, kind: str | None) -> tuple[int, int]:
        return vm_backend.insn_flags(ins, kind)

    def scan(self, last: Any | None, address: int) -> tuple[int, set[int], list[tuple[int, int]]]:
        """live_out after `last` ending at `address` (live_in at `address` when last is None): the flags, the pages the
        scan made the block an owner of, the instructions it read."""
        self.code_pages = {p: set() for p in self.model.code}
        self._scan_log = []
        live = self._flags_live_in(1, address) if last is None else self._flags_live_out(1, last, address)
        return live, {p for p, owners in self.code_pages.items() if owners}, self._scan_log


CODE = ctypes.CFUNCTYPE(INT, ctypes.c_void_p, U64, ctypes.POINTER(ctypes.POINTER(ctypes.c_uint8)))
SCANNED = ctypes.CFUNCTYPE(None, ctypes.c_void_p, U64)


class Memory(ctypes.Structure):
    _fields_ = [("code", CODE), ("scanned", SCANNED), ("ctx", ctypes.c_void_p), ("thunks", ctypes.POINTER(U64)),
                ("nthunks", INT)]


class Insn(ctypes.Structure):
    _fields_ = [("form", ctypes.c_void_p), ("address", U64), ("values", U64 * 24), ("nvalues", INT), ("length", INT)]


class CScan:
    """vmc_live_in and vmc_live_out of a vm_compiler.c built as a shared library, over a Model."""

    def __init__(self, lib: Any) -> None:
        self.lib = lib
        lib.vmc_live_in.argtypes = [ctypes.POINTER(Memory), U64]
        lib.vmc_live_out.argtypes = [ctypes.POINTER(Memory), ctypes.POINTER(Insn), U64]
        lib.vmc_decode.argtypes = [ctypes.c_char_p, INT, U64, ctypes.POINTER(Insn)]
        self.buffer = (ctypes.c_uint8 * 16)()
        self.model = Model()
        self.pages: set[int] = set()
        self._code, self._scanned = CODE(self._code_cb), SCANNED(self._scanned_cb)

    def _code_cb(self, _ctx: Any, address: int, out: Any) -> int:
        data = self.model.read(address, 15, self.model.code)
        ctypes.memmove(self.buffer, data, len(data))
        out[0] = ctypes.cast(self.buffer, ctypes.POINTER(ctypes.c_uint8))
        return len(data)

    def _scanned_cb(self, _ctx: Any, page: int) -> None:
        self.pages.add(page)

    def decode(self, model: Model, address: int) -> Insn | None:
        ins = Insn()
        data = model.read(address, 15, model.pages)
        return ins if self.lib.vmc_decode(data, len(data), U64(address), ctypes.byref(ins)) == 0 else None

    def scan(self, model: Model, last: Insn | None, address: int) -> tuple[int, set[int]]:
        self.model, self.pages = model, set()
        thunks = (U64 * max(len(model.thunks), 1))(*model.thunks)
        m = Memory(self._code, self._scanned, None, thunks, len(model.thunks))
        live = (self.lib.vmc_live_in(ctypes.byref(m), U64(address)) if last is None else
                self.lib.vmc_live_out(ctypes.byref(m), ctypes.byref(last), U64(address)))
        return live, self.pages


def judge(c: CScan, model: Model, scans: list[tuple[int | None, int]], counts: Counter[str],
          examples: list[str]) -> None:
    """Each scan (the last instruction's address, or None for live_in; the address scanned from) by both."""
    host = _Host(model)
    for last_at, address in scans:
        last = c_last = None
        if last_at is not None:
            c_last = c.decode(model, last_at)
            if c_last is None:
                counts["last has no form"] += 1
                continue
            try:
                last = host._lift(last_at)
            except (LiftError, GuestFault):
                counts["last does not lift"] += 1
                continue
        py, py_pages, read = host.scan(last, address)
        got, pages = c.scan(model, c_last, address)
        every_form = all(c.decode(model, a) is not None for a, _ in read)
        if got == py and pages == py_pages and every_form:
            counts["equal"] += 1
        elif py & ~got:
            counts["short"] += 1
            examples.append(f"short {last_at and hex(last_at)} {address:x}: C {got:#x} host {py:#x}")
        elif every_form:
            counts["differ"] += 1
            examples.append(f"differ {last_at and hex(last_at)} {address:x}: C {got:#x} {sorted(pages)} host "
                            f"{py:#x} {sorted(py_pages)}")
        else:
            counts["superset" if got != py else "equal flags, a form missing"] += 1


# the record's memory ---------------------------------------------------------------------------------------------
def record_model(record: dict[str, Any]) -> Model:
    """Every recorded instruction's bytes at its address (the first of its byte strings)."""
    model = Model()
    for ins, codes in sorted(record["bytes"].items(), key=lambda e: e[0].address):
        model.place(ins.address, min(codes))
    return model


def record_scans(record: dict[str, Any]) -> list[tuple[int | None, int]]:
    """A scan after every recorded instruction with a branch (a block's end), and at every address one follows."""
    out: list[tuple[int | None, int]] = []
    for ins in sorted(record["bytes"], key=lambda i: i.address):
        if any(op[0] == "br" for op in ins.ops):
            out.append((ins.address, ins.address + ins.length))
        out.append((None, ins.address + ins.length))
    return out


def scan_samples(record: dict[str, Any], n: int, seed: int) -> list[str]:
    """For tests/data/scan_samples.txt.gz: n scans drawn from record_scans, each as the instructions the host's scan
    read in the record's memory (and the last one): "last|- address addr:bytes ...". A Model made of a line's
    instructions gives the host's scan the same reads."""
    model = record_model(record)
    host = _Host(model)
    rng = random.Random(seed)
    lines = []
    for last_at, address in rng.sample(record_scans(record), n):
        last = None if last_at is None else host._lift(last_at)
        _, _, read = host.scan(last, address)
        at = sorted({*(a for a, _ in read), *([last_at] if last_at is not None else [])})
        lengths = dict(read) | ({last_at: last.length} if last is not None else {})
        insns = " ".join(f"{a:x}:{model.read(a, lengths[a], model.pages).hex()}" for a in at)
        lines.append(f"{'-' if last_at is None else f'{last_at:x}'} {address:x} {insns}")
    return lines


def sample_model(line: str) -> tuple[Model, tuple[int | None, int]]:
    last, address, *insns = line.split()
    model = Model()
    for item in insns:
        a, code = item.split(":")
        model.place(int(a, 16), bytes.fromhex(code))
    return model, (None if last == "-" else int(last, 16), int(address, 16))


# spec-row encodings, laid out with jumps -------------------------------------------------------------------------
JUMPS = (b"\xe9", b"\xe8", *(bytes([0x0F, 0x80 + cc]) for cc in range(16)))   # JMP, CALL, Jcc rel32


def spec_layout(rng: random.Random, encodings: list[bytes], segments: int = 4) -> tuple[Model, list[tuple[int | None, int]]]:
    """`segments` runs of 1-12 encodings at random places near page ends, each but the last ending in a JMP, CALL or
    Jcc rel32 to another's start; one page in four after a run is mapped but not code. The scans: live_in at each
    encoding, live_out after each jump."""
    model = Model()
    starts = [0x1_4000_0000 + rng.randrange(1 << 12) * 4 * PAGE + PAGE - rng.randrange(1, 48)
              for _ in range(segments)]
    scans: list[tuple[int | None, int]] = []
    for s, start in enumerate(starts):
        at = start
        for _ in range(rng.randrange(1, 13)):
            code = rng.choice(encodings)
            model.place(at, code)
            scans.append((None, at))
            at += len(code)
        if s + 1 < len(starts):
            op = rng.choice(JUMPS)
            target = rng.choice(starts)
            model.place(at, op + ((target - (at + len(op) + 4)) & 0xFFFF_FFFF).to_bytes(4, "little"))
            scans.append((at, at + len(op) + 4))
            at += len(op) + 4
        if rng.random() < 0.25:
            model.place(at, rng.randbytes(16), code=False)
    return model, scans


def spec_encodings(rng: random.Random, per_row: int) -> list[bytes]:
    """Random encodings of every decoder spec row (the review's E1 generator) that the host lifts."""
    import test_x86_decode_c
    out = []
    for raw in test_x86_decode_c.row_samples(rng, per_row):
        try:
            code = raw[:x86dec.decode(raw).length]
            lift(code, 0x1_4000_0000)
        except Exception:
            continue
        out.append(code)
    return out
