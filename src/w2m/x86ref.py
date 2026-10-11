# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""x86ref.py — reference decoding of reachable x86-64 code with Capstone (Sprint 014).

The first of two independent decoders (user decision 2026-10-08): Capstone is the existing
reference; w2m's own decoder is generated from its own specification and compared against it,
instruction by instruction. Disagreements are the work queue.

Traversal: start from every statically known code target in the PE (entry, exports, TLS
callbacks, .pdata function starts); follow fall-through and direct branch/call targets inside
executable sections; stop at ret, unconditional jmp, int3, ud2, hlt; count indirect transfers as
unresolved (v0.2 section 5.1: their targets are discovered at run time). Calls through the IAT
resolve to "dll!name". A RIP-relative LEA of an address in an executable section is a function whose address
is taken (a thread's start routine handed to CreateThread, a callback): it is a root too (Sprint 051: a no-CRT
probe has no .pdata to list it).
"""

from __future__ import annotations

from collections import Counter
from dataclasses import dataclass, field

import capstone
from capstone import x86 as cx86

from w2m.pe import SCN_MEM_EXECUTE, PeImage

STOP = {"ret", "retf", "jmp", "int3", "ud2", "hlt"}
# Imports documented to end the calling thread or process; a call to one does not return, so the
# bytes after it are not reached through it (Andriesse et al., USENIX Security 2016: missed
# non-returning calls are a main source of recursive-disassembly error). Sources: Microsoft Learn
# pages for ExitProcess, ExitThread, FreeLibraryAndExitThread, RaiseFailFastException; C standard
# exit/_Exit/quick_exit/abort; MSVC CRT _invalid_parameter_noinfo_noreturn, _invoke_watson.
NORETURN = frozenset({
    "ExitProcess", "ExitThread", "FreeLibraryAndExitThread", "RaiseFailFastException",
    "RtlExitUserThread", "RtlExitUserProcess", "exit", "_exit", "_Exit", "quick_exit", "abort",
    "_invalid_parameter_noinfo_noreturn", "_invoke_watson", "_CxxThrowException",
})
MAX_INSN = 15


@dataclass(frozen=True)
class Insn:
    rva: int
    size: int
    mnemonic: str
    op_str: str
    raw: bytes


@dataclass
class Reachable:
    insns: dict[int, Insn] = field(default_factory=dict)
    roots: Counter[str] = field(default_factory=Counter)
    unresolved_indirect: list[int] = field(default_factory=list)
    imports_called: Counter[str] = field(default_factory=Counter)      # call [rip+slot]
    imports_referenced: Counter[str] = field(default_factory=Counter)  # any rip-relative use of a slot
    undecodable: list[int] = field(default_factory=list)

    def mnemonics(self) -> Counter[str]:
        return Counter(i.mnemonic for i in self.insns.values())


def _exec_ranges(img: PeImage) -> list[tuple[int, int]]:
    return [(s.virtual_address, s.virtual_address + s.virtual_size)
            for s in img.sections if s.characteristics & SCN_MEM_EXECUTE]


def walk(img: PeImage, data: bytes) -> Reachable:
    """Recursive-traversal disassembly of everything statically reachable."""
    md = capstone.Cs(capstone.CS_ARCH_X86, capstone.CS_MODE_64)
    md.detail = True
    ranges = _exec_ranges(img)

    def executable(rva: int) -> bool:
        return any(a <= rva < b for a, b in ranges)

    out = Reachable()
    work: list[int] = []
    for kind, rva in img.code_targets():
        if executable(rva):
            out.roots[kind] += 1
            work.append(rva)
    while work:
        rva = work.pop()
        while executable(rva) and rva not in out.insns:
            raw = img.read_rva(data, rva, MAX_INSN)
            got = next(md.disasm(raw, img.image_base + rva, 1), None)
            if got is None:
                out.undecodable.append(rva)
                break
            ins = Insn(rva, got.size, got.mnemonic, got.op_str, bytes(got.bytes))
            out.insns[rva] = ins
            nxt = rva + got.size
            for op in got.operands:
                if (op.type == cx86.X86_OP_MEM and op.mem.base == cx86.X86_REG_RIP
                        and nxt + op.mem.disp in img.iat):
                    out.imports_referenced[img.iat[nxt + op.mem.disp]] += 1
                if (got.mnemonic == "lea" and op.type == cx86.X86_OP_MEM and op.mem.base == cx86.X86_REG_RIP
                        and op.mem.index == 0 and executable(nxt + op.mem.disp)
                        and nxt + op.mem.disp not in out.insns):
                    out.roots["address_taken"] += 1
                    work.append(nxt + op.mem.disp)
            groups = set(got.groups)
            is_branch = cx86.X86_GRP_JUMP in groups or cx86.X86_GRP_CALL in groups
            if is_branch:
                op = got.operands[0] if got.operands else None
                if op is not None and op.type == cx86.X86_OP_IMM:
                    work.append(op.imm - img.image_base)
                elif (op is not None and op.type == cx86.X86_OP_MEM
                      and op.mem.base == cx86.X86_REG_RIP and op.mem.index == 0):
                    slot = nxt + op.mem.disp
                    if slot in img.iat:
                        out.imports_called[img.iat[slot]] += 1
                        if (cx86.X86_GRP_CALL in groups
                                and img.iat[slot].rsplit("!", 1)[1] in NORETURN):
                            break
                    else:
                        out.unresolved_indirect.append(rva)
                else:
                    out.unresolved_indirect.append(rva)
            if got.mnemonic in STOP or (got.mnemonic == "int" and got.op_str == "0x29"):
                break                                 # int 0x29 is __fastfail: never returns
            rva = nxt
    return out


def text_coverage(img: PeImage, r: Reachable) -> float:
    """Fraction of executable-section bytes covered by reachable instructions."""
    total = sum(b - a for a, b in _exec_ranges(img))
    return sum(i.size for i in r.insns.values()) / total if total else 0.0
