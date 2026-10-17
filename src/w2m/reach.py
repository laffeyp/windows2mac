# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""reach.py — what an import reaches through Microsoft's DLLs: system calls and their information classes (Sprint 093,
E2; docs/solution-2026-10-16.md 4.2).

The method is NTFUZZ's (Choi et al., IEEE S&P 2021): each function is read once into a summary — the functions and
imports it calls, and at each call site the arguments it passes, each a constant, one of its own parameters, or
unknown — and summaries compose bottom-up, so a class passed in by a caller is named at the caller. NTFUZZ read 32-bit
Windows 10 17134 and dropped indirect calls; this reads build 26200's 64-bit DLLs (System32) and 32-bit ones
(SysWOW64) and counts each indirect call site it cannot resolve.

What the walk handles, each measured on the copied build before it was written (sprints/sprint-093-reach-from-
imports.md): imports through API sets (w2m.apiset), forwarders and imports by ordinal (pe.py), stubs found by their
bytes (six of ntdll's 494 are unexported copies), tail jumps (kernel32's exports are `jmp [IAT]`), imports held in a
register (`mov rax, [IAT]` before CFG's dispatch call; 32-bit `call esi`), and 32-bit IAT slots named by absolute
address. Values: `mov reg, imm`, `xor reg, reg`, register copies, `lea reg, [reg + disp]`, add/sub/or of constants,
stores of constants to the outgoing argument area (64-bit), pushes (32-bit). Only a function's entry block knows its
parameters (64-bit: rcx, rdx, r8, r9); every other block starts unknown.
"""

from __future__ import annotations

import re
import struct
from collections.abc import Callable, Iterator
from dataclasses import dataclass, field
from pathlib import Path

import capstone
from capstone import x86 as cx86

from w2m import apiset
from w2m.pe import SCN_MEM_EXECUTE, PeError, PeImage, parse
from w2m.x86ref import NORETURN

Value = tuple[str, int | str] | None        # ("const", n) | ("param", k) | ("import", "dll!name") | ("code", RVA) | None
NodeKey = tuple[str, str, int]              # (arch "x64" | "x86", dll lower case, RVA)
PARAMS64 = ("rcx", "rdx", "r8", "r9")
RAX = -1                                     # rax's "parameter" index: CFG's dispatch stub is `jmp rax` (d3d11, Sprint 093)
VOLATILE64 = ("rax", "rcx", "rdx", "r8", "r9", "r10", "r11")
VOLATILE32 = ("eax", "ecx", "edx")
MAX_ARGS = 12
STRING = re.compile(rb"[A-Za-z_][A-Za-z0-9_]{3,63}(?=\0)")            # as scripts/static_census.py
DLL_STRING = re.compile(rb"[A-Za-z0-9_.+-]{2,60}\.(?:dll|DLL|Dll)(?=\0)")
DLL_STRING16 = re.compile(rb"(?:[A-Za-z0-9_.+-]\0){2,60}\.\0(?:d\0l\0l\0|D\0L\0L\0)(?=\0\0)")


def _parents() -> dict[str, str]:
    out: dict[str, str] = {}
    for r in ("ax", "bx", "cx", "dx"):
        for name in (f"r{r}", f"e{r}", r, f"{r[0]}l", f"{r[0]}h"):
            out[name] = f"r{r}"
    for r in ("si", "di", "bp", "sp"):
        for name in (f"r{r}", f"e{r}", r, f"{r}l"):
            out[name] = f"r{r}"
    for n in range(8, 16):
        for suffix in ("", "d", "w", "b"):
            out[f"r{n}{suffix}"] = f"r{n}"
    return out


PARENT = _parents()


@dataclass(frozen=True)
class Stub:
    number: int
    table: str                                # "nt" (ntdll's numbers) or "win32k" (bit 12 set)
    name: str


@dataclass
class Site:
    target: tuple[str, int | str]             # ("fn", RVA) in the same image | ("import", "dll!name") | ("param", k)
    args: tuple[Value, ...]
    rax: Value = None


@dataclass
class Summary:
    stub: Stub | None = None
    sites: list[Site] = field(default_factory=list)
    indirect: int = 0                         # call or jmp sites whose target is not known
    undecodable: int = 0
    taken: set[int] = field(default_factory=set)   # code addresses the function takes (lea), callable by pointer
    imports_taken: set[str] = field(default_factory=set)  # imports read as a value (`mov reg, [IAT]`): called, or
    # stored for others to call (d3d11 fills the driver's callback table this way: `mov [rdi + 0x250], rax`)


@dataclass
class Image:
    arch: str
    name: str
    pe: PeImage
    data: bytes
    exec_ranges: list[tuple[int, int]]
    by_rva: dict[int, str]                    # export RVA -> a name (for naming stubs)

    def executable(self, rva: int) -> bool:
        return any(a <= rva < b for a, b in self.exec_ranges)


class Windows:
    """The copied C:\\Windows: images by architecture and name, the API set map, and function summaries."""

    def __init__(self, root: Path) -> None:
        self.dirs = {"x64": root / "System32", "x86": root / "SysWOW64"}
        self.files = {arch: {p.name.lower(): p for p in d.iterdir()} for arch, d in self.dirs.items()}
        schema = (root / "System32" / "apisetschema.dll").read_bytes()
        self.apisets = apiset.read(parse(schema), schema)
        self.images: dict[tuple[str, str], Image | None] = {}
        self.summaries: dict[NodeKey, Summary] = {}
        self.md = {"x64": capstone.Cs(capstone.CS_ARCH_X86, capstone.CS_MODE_64),
                   "x86": capstone.Cs(capstone.CS_ARCH_X86, capstone.CS_MODE_32)}
        for md in self.md.values():
            md.detail = True

    def image(self, arch: str, name: str) -> Image | None:
        key = (arch, name.lower())
        if key not in self.images:
            path = self.files[arch].get(name.lower())
            image = None
            if path is not None:
                data = path.read_bytes()
                try:
                    pe = parse(data, x86=arch == "x86")
                except PeError:
                    pe = None
                if pe is not None:
                    image = Image(arch, name.lower(), pe, data,
                                  [(s.virtual_address, s.virtual_address + s.virtual_size)
                                   for s in pe.sections if s.characteristics & SCN_MEM_EXECUTE],
                                  {rva: n for n, rva in sorted(pe.exports, reverse=True)})
            self.images[key] = image
        return self.images[key]

    def resolve(self, arch: str, importer: str, ref: str, depth: int = 0) -> NodeKey | str:
        """Where `dll!name` (or `dll!#ordinal`) lands: a node, or the reason it lands nowhere."""
        dll, _, name = ref.rpartition("!")
        dll = dll.lower() if dll.lower().endswith(".dll") else dll.lower() + ".dll"
        host = self.apisets.host(dll, importer) if dll.startswith(("api-", "ext-")) else dll
        if host is None:
            return f"api set {dll} has no host"
        image = self.image(arch, host)
        if image is None:
            return f"{host} not in the copy"
        if name in image.pe.forwarders:
            if depth > 8:
                return f"forwarder chain from {ref}"
            target_dll, _, target = image.pe.forwarders[name].rpartition(".")
            return self.resolve(arch, host, f"{target_dll}!{target}", depth + 1)
        if name.startswith("#"):
            rva = image.pe.export_ordinals.get(int(name[1:]))
        else:
            rva = next((r for n, r in image.pe.exports if n == name), None)
        if rva is None:
            return f"{host} exports no {name}"
        if not image.executable(rva):
            return f"{host}!{name} is data"
        return (arch, host, rva)

    def data_code_pointers(self, image: Image) -> set[int]:
        """Code addresses stored in the image's data: each relocated pointer (DIR64, or HIGHLOW in 32-bit code) that
        lies outside executable sections and points into one (vtables, callback tables)."""
        out: set[int] = set()
        width, kind = (8, 10) if image.arch == "x64" else (4, 3)
        for rva, t in image.pe.relocation_sites:
            if t != kind or image.executable(rva):
                continue
            try:
                raw = image.pe.read_rva(image.data, rva, width)
            except PeError:
                continue
            if len(raw) < width:
                continue
            target = int.from_bytes(raw, "little") - image.pe.image_base
            if image.executable(target):
                out.add(target)
        return out

    def lookups(self, image: Image) -> list[str]:
        """Run-time lookups the image can make, as `dll!name` references: when it imports GetProcAddress, each ASCII
        string equal to an export of a DLL its strings or imports name (scripts/static_census.py's rule for the
        games, applied inside Windows: d3d11 and dxgi look up D3DKMTGetDeviceState by name)."""
        if not any(ref.rsplit("!", 1)[1] == "GetProcAddress" for ref in image.pe.iat.values()):
            return []
        names = {s.decode() for s in STRING.findall(image.data)}
        dlls = {m.decode("latin-1").lower() for m in DLL_STRING.findall(image.data)}
        dlls |= {m.decode("utf-16-le").lower() for m in DLL_STRING16.findall(image.data)}
        dlls |= {d.lower() for d in image.pe.imports}
        out: set[str] = set()
        for dll in sorted(dlls):
            other = self.image(image.arch, self.apisets.host(dll, image.name) or dll)
            if other is None:
                continue
            exported = {n for n, _ in other.pe.exports} | {n for n in other.pe.forwarders if not n.startswith("#")}
            out |= {f"{other.name}!{n}" for n in names & exported}
        return sorted(out)

    def stub_at(self, image: Image, rva: int) -> Stub | None:
        try:
            b = image.pe.read_rva(image.data, rva, 16)
        except PeError:
            return None
        number = None
        if image.arch == "x64" and b[:4] == b"\x4c\x8b\xd1\xb8" and (
                b[8:16] == b"\xf6\x04\x25\x08\x03\xfe\x7f\x01" or b[8:10] == b"\x0f\x05"):
            number = struct.unpack_from("<I", b, 4)[0]
        if image.arch == "x86" and b[:1] == b"\xb8" and (
                (b[5:6] == b"\xba" and b[10:12] == b"\xff\xd2") or b[5:11] == b"\xe8\x00\x00\x00\x00\x5a"):
            number = struct.unpack_from("<I", b, 1)[0] & 0xFFFF   # WoW64's stubs carry more in the high word (NtClose: 0x3000f)
        if number is None or number >= 0x2000:
            return None
        name = image.by_rva.get(rva, f"{image.name}+{rva:#x}")
        return Stub(number, "win32k" if number & 0x1000 else "nt", "Nt" + name[2:] if name[:2] == "Zw" else name)

    def summary(self, key: NodeKey) -> Summary:
        if key not in self.summaries:
            image = self.image(key[0], key[1])
            assert image is not None
            self.summaries[key] = self._read(image, key[2])
        return self.summaries[key]

    # -- one function -------------------------------------------------------------------------------------------
    def _read(self, image: Image, start: int) -> Summary:
        out = Summary(stub=self.stub_at(image, start))
        if out.stub is not None:
            return out
        x64 = image.arch == "x64"
        md = self.md[image.arch]
        inside = self._extent(image, start)
        entry_state: dict[str, Value] = {}
        if x64:
            entry_state = {r: ("param", k) for k, r in enumerate(PARAMS64)}
            entry_state["rax"] = ("param", RAX)
        work: list[tuple[int, dict[str, Value]]] = [(start, entry_state)]
        seen: set[int] = set()
        while work:
            rva, regs = work.pop()
            regs = dict(regs)
            stack: dict[int, Value] = {}
            pushes: list[Value] = []
            while image.executable(rva) and rva not in seen:
                seen.add(rva)
                raw = image.pe.read_rva(image.data, rva, 15)
                ins = next(md.disasm(raw, image.pe.image_base + rva, 1), None)
                if ins is None:
                    out.undecodable += 1
                    break
                nxt = rva + ins.size
                groups = set(ins.groups)
                call = cx86.X86_GRP_CALL in groups
                jump = cx86.X86_GRP_JUMP in groups
                if call or (jump and ins.mnemonic == "jmp"):
                    target = self._target(image, ins, nxt, regs)
                    tail = jump and target is not None and (target[0] == "import" or not inside(int(target[1])))
                    if call or tail:
                        if target is None:
                            out.indirect += 1
                        else:
                            out.sites.append(Site(target, self._args(x64, regs, stack, pushes), regs.get("rax")))
                            if target[0] == "import" and str(target[1]).rsplit("!", 1)[1] in NORETURN:
                                break
                        if jump:
                            break
                        for r in (VOLATILE64 if x64 else VOLATILE32):
                            regs.pop(PARENT[r], None)
                        pushes = []
                        rva = nxt
                        continue
                    if target is None:
                        out.indirect += 1            # a jump table or a computed jump inside the function
                        break
                    rva = int(target[1])             # a jump inside the function
                    continue
                if jump:                             # a conditional branch: both ways, the taken one with no knowledge
                    op = ins.operands[0] if ins.operands else None
                    if op is not None and op.type == cx86.X86_OP_IMM:
                        work.append((op.imm - image.pe.image_base, {}))
                    rva = nxt
                    continue
                if ins.mnemonic in ("ret", "retf", "int3", "ud2", "hlt") or (
                        ins.mnemonic == "int" and ins.op_str == "0x29"):
                    break
                if ins.mnemonic == "mov" and len(ins.operands) == 2 and ins.operands[0].type == cx86.X86_OP_REG:
                    slot = self._slot(image, ins.operands[1], nxt)
                    if slot is not None and slot in image.pe.iat:
                        out.imports_taken.add(image.pe.iat[slot])
                if ins.mnemonic == "lea" and len(ins.operands) == 2:
                    slot = self._slot(image, ins.operands[1], nxt)
                    if slot is not None and image.executable(slot):
                        out.taken.add(slot)
                self._effect(image, ins, nxt, regs, stack, pushes, x64)
                rva = nxt
        return out

    def _extent(self, image: Image, start: int) -> Callable[[int], bool]:
        """A test of whether an RVA is inside the function at `start`: its .pdata range, or, for a function .pdata does
        not describe (a leaf), anything that is not another described function's start or an export."""
        starts, ends = image.pe.function_starts, image.pe.function_ends
        rng = next(((a, b) for a, b in zip(starts, ends, strict=True) if a == start), None)
        others = set(starts) | set(image.by_rva)
        if rng is not None:
            return lambda rva: rng[0] <= rva < rng[1]
        return lambda rva: rva == start or rva not in others

    def _slot(self, image: Image, op: cx86.X86Op, nxt: int) -> int | None:
        if op.type != cx86.X86_OP_MEM or op.mem.index != 0:
            return None
        if op.mem.base == cx86.X86_REG_RIP:
            return nxt + int(op.mem.disp)
        if image.arch == "x86" and op.mem.base == 0 and op.mem.segment == 0:
            return int(op.mem.disp & 0xFFFFFFFF) - image.pe.image_base
        return None

    def _target(self, image: Image, ins: capstone.CsInsn, nxt: int,
                regs: dict[str, Value]) -> tuple[str, int | str] | None:
        if not ins.operands:
            return None
        op = ins.operands[0]
        if op.type == cx86.X86_OP_IMM:
            return ("fn", op.imm - image.pe.image_base)
        if op.type == cx86.X86_OP_REG:
            v = regs.get(PARENT.get(ins.reg_name(op.reg), ""))
            if v is not None and v[0] == "code":
                return ("fn", int(v[1]))
            return (v[0], v[1]) if v is not None and v[0] in ("import", "param") else None
        slot = self._slot(image, op, nxt)
        if slot is not None and slot in image.pe.iat:
            return ("import", image.pe.iat[slot])
        if slot is not None and image.arch == "x64":
            v = regs.get("rax")                      # CFG's dispatch: `call [__guard_dispatch_icall_fptr]` calls rax
            if v is not None and v[0] in ("import", "param"):
                return (v[0], v[1])
            if v is not None and v[0] == "code":
                return ("fn", int(v[1]))
        return None

    def _args(self, x64: bool, regs: dict[str, Value], stack: dict[int, Value],
              pushes: list[Value]) -> tuple[Value, ...]:
        if x64:
            return tuple(regs.get(r) for r in PARAMS64) + tuple(stack.get(0x20 + 8 * i) for i in range(MAX_ARGS - 4))
        return tuple(reversed(pushes[-MAX_ARGS:]))

    def _effect(self, image: Image, ins: capstone.CsInsn, nxt: int, regs: dict[str, Value],
                stack: dict[int, Value], pushes: list[Value], x64: bool) -> None:
        ops = ins.operands
        m = ins.mnemonic
        if m == "push" and not x64 and ops:
            op = ops[0]
            if op.type == cx86.X86_OP_IMM:
                pushes.append(("const", op.imm & 0xFFFFFFFF))
            elif op.type == cx86.X86_OP_REG:
                pushes.append(regs.get(PARENT.get(ins.reg_name(op.reg), "")))
            else:
                slot = self._slot(image, op, nxt)
                pushes.append(("import", image.pe.iat[slot]) if slot in image.pe.iat else None)
            return
        if m == "pop" and pushes:
            pushes.pop()
        dest = ops[0] if ops else None
        if dest is not None and dest.type == cx86.X86_OP_MEM and x64 and dest.mem.base == cx86.X86_REG_RSP \
                and dest.mem.index == 0 and m == "mov" and len(ops) == 2:
            src = ops[1]
            stack[dest.mem.disp] = (("const", src.imm & 0xFFFFFFFFFFFFFFFF) if src.type == cx86.X86_OP_IMM
                                    else regs.get(PARENT.get(ins.reg_name(src.reg), ""))
                                    if src.type == cx86.X86_OP_REG else None)
            return
        _, written = ins.regs_access()
        names = {PARENT.get(ins.reg_name(r), "") for r in written}
        value: Value = None
        if dest is not None and dest.type == cx86.X86_OP_REG and len(ops) == 2:
            src = ops[1]
            d = PARENT.get(ins.reg_name(dest.reg), "")
            if m == "mov" and src.type == cx86.X86_OP_IMM:
                value = ("const", src.imm & 0xFFFFFFFFFFFFFFFF)
            elif m == "mov" and src.type == cx86.X86_OP_REG:
                value = regs.get(PARENT.get(ins.reg_name(src.reg), ""))
            elif m == "mov" and src.type == cx86.X86_OP_MEM:
                slot = self._slot(image, src, nxt)
                value = ("import", image.pe.iat[slot]) if slot in image.pe.iat else None
            elif m == "xor" and src.type == cx86.X86_OP_REG and src.reg == dest.reg:
                value = ("const", 0)
            elif m == "lea" and self._slot(image, src, nxt) is not None:
                slot = self._slot(image, src, nxt)
                value = ("code", slot) if slot is not None and image.executable(slot) else None
            elif m == "lea" and src.mem.index == 0 and src.mem.base != cx86.X86_REG_RIP:
                base = regs.get(PARENT.get(ins.reg_name(src.mem.base), ""))
                if base is not None and base[0] == "const":
                    value = ("const", int(base[1]) + src.mem.disp)
                elif src.mem.disp == 0:
                    value = base
            elif m in ("add", "sub", "or", "and") and src.type == cx86.X86_OP_IMM:
                cur = regs.get(d)
                if cur is not None and cur[0] == "const":
                    a, b = int(cur[1]), src.imm
                    value = ("const", {"add": a + b, "sub": a - b, "or": a | b, "and": a & b}[m])
            for n in names:
                regs.pop(n, None)
            if value is not None:
                regs[d] = value
            return
        for n in names:
            regs.pop(n, None)


# -- reach and classes over the graph -------------------------------------------------------------------------------
@dataclass
class Reach:
    stubs: dict[Stub, set[str]] = field(default_factory=dict)       # stub -> roots reaching it
    classes: dict[tuple[Stub, int], set[str]] = field(default_factory=dict)   # (stub, class value) -> roots
    class_from_caller: dict[tuple[Stub, int], set[str]] = field(default_factory=dict)  # class is the root's arg k
    class_unknown: dict[Stub, set[str]] = field(default_factory=dict)
    unresolved: dict[str, int] = field(default_factory=dict)        # reason -> count of import edges
    indirect: int = 0
    functions: int = 0
    cycles_cut: int = 0


def callee(win: Windows, key: NodeKey, site: Site) -> NodeKey | str | None:
    kind, target = site.target
    if kind == "fn":
        return (key[0], key[1], int(target))
    if kind == "import":
        return win.resolve(key[0], key[1], str(target))
    return None                                   # a call through a parameter: the caller names the target


def site_value(site: Site, k: int) -> Value:
    return site.rax if k == RAX else site.args[k] if k < len(site.args) else None


def called_params(win: Windows, key: NodeKey, memo: dict[NodeKey, frozenset[int]],
                  active: set[NodeKey]) -> frozenset[int]:
    """The parameters (RAX for rax) that `key` calls through, itself or by passing them on to a callee that does."""
    if key in memo:
        return memo[key]
    if key in active:
        return frozenset()
    active.add(key)
    out: set[int] = set()
    for site in win.summary(key).sites:
        if site.target[0] == "param":
            out.add(int(site.target[1]))
            continue
        c = callee(win, key, site)
        if isinstance(c, tuple):
            for j in called_params(win, c, memo, active):
                v = site_value(site, j)
                if v is not None and v[0] == "param":
                    out.add(int(v[1]))
    active.discard(key)
    memo[key] = frozenset(out)
    return memo[key]


def edges(win: Windows, key: NodeKey, memo: dict[NodeKey, frozenset[int]] | None = None) -> Iterator[NodeKey | str]:
    memo = {} if memo is None else memo
    for site in win.summary(key).sites:
        c = callee(win, key, site)
        if c is None:
            continue
        yield c
        if isinstance(c, str):
            continue
        for j in called_params(win, c, memo, set()):  # a pointer handed to a callee that calls it
            v = site_value(site, j)
            if v is not None and v[0] == "import":
                yield win.resolve(key[0], key[1], str(v[1]))
            elif v is not None and v[0] == "code":
                yield (key[0], key[1], int(v[1]))


def effects(win: Windows, key: NodeKey, cls: dict[str, int], memo: dict[NodeKey, frozenset[tuple[Stub, Value]]],
            active: set[NodeKey], counts: Reach) -> frozenset[tuple[Stub, Value]]:
    """(stub, class value) pairs a call to `key` can make, values in terms of `key`'s own parameters."""
    if key in memo:
        return memo[key]
    if key in active:
        counts.cycles_cut += 1
        return frozenset()
    s = win.summary(key)
    if s.stub is not None:
        k = cls.get(s.stub.name)
        result = frozenset({(s.stub, ("param", k) if k is not None else ("none", 0))})
        memo[key] = result
        return result
    active.add(key)
    out: set[tuple[Stub, Value]] = set()
    for site in s.sites:
        c = callee(win, key, site)
        if not isinstance(c, tuple):
            continue
        for stub, v in effects(win, c, cls, memo, active, counts):
            if v is not None and v[0] == "param":
                v = site_value(site, int(v[1]))
            out.add((stub, v))
    active.discard(key)
    memo[key] = frozenset(out)
    return memo[key]
