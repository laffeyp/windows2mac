"""x86ref.py — reference decoding of reachable x86-64 code with Capstone (Sprint 014).

The first of two independent decoders (user decision 2026-10-08): Capstone is the existing
reference; w2m's own decoder is generated from its own specification and compared against it,
instruction by instruction. Disagreements are the work queue.

Traversal: start from every statically known code target in the PE (entry, exports, TLS
callbacks, .pdata function starts); follow fall-through and direct branch/call targets inside
executable sections; stop at ret, unconditional jmp, int3, ud2, hlt; count indirect transfers as
unresolved (v0.2 section 5.1: their targets are discovered at run time). Calls through the IAT
resolve to "dll!name".
"""

from __future__ import annotations

from collections import Counter
from dataclasses import dataclass, field

import capstone
from capstone import x86 as cx86

from w2m.pe import SCN_MEM_EXECUTE, PeImage

STOP = {"ret", "retf", "jmp", "int3", "ud2", "hlt"}
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
                    else:
                        out.unresolved_indirect.append(rva)
                else:
                    out.unresolved_indirect.append(rva)
            if got.mnemonic in STOP:
                break
            rva = nxt
    return out


def text_coverage(img: PeImage, r: Reachable) -> float:
    """Fraction of executable-section bytes covered by reachable instructions."""
    total = sum(b - a for a, b in _exec_ranges(img))
    return sum(i.size for i in r.insns.values()) / total if total else 0.0
