"""loader.py — map PE32+ images into guest memory and bind their imports (Sprints 029, 046).

The program's image is mapped at its preferred base (ImageBase); headers and each section's raw data
are copied, and the rest of SizeOfImage reads as zero (PE Format, Section Table: "If this value
[VirtualSize] is greater than SizeOfRawData, the section is zero-padded").

A DLL the program imports that `dlls` can supply (the program's directory: UnityPlayer.dll for the
launcher) is mapped too: at its preferred base when that range is free, as Windows placed the loader
probe's ASLR-enabled DLL on every runner (contracts/windows.loader.static_import, row dll.base), else
at DLL_BASE and up, 64 KB aligned, with its base relocations applied (PE Format, The .reloc Section: DIR64 adds the 64-bit delta, HIGHLOW its low 32 bits;
any other type refuses the load). An import from such a DLL is bound to the DLL's export; a missing
export refuses the load, as Windows refuses to start the process. Every other import slot receives
the address of a host entry point: a thunk outside the images that the runtime services in the host
(src/w2m/runtime.py) or the VM bridges to the guest runtime.

DLL entry points run before the program's, deepest dependency first (Dynamic-Link Library Entry-Point
Function: DLL_PROCESS_ATTACH at process start-up): `entry` is then a short x86 stub in the "loader"
region that calls each DllMain(instance, DLL_PROCESS_ATTACH, reserved) and jumps to the program's
entry; a DllMain that returns FALSE calls a thunk that stops the run by name.
"""

from __future__ import annotations

from collections.abc import Callable
from dataclasses import dataclass, field

from w2m.pe import PeImage, parse

THUNK_BASE = 0x7FFE_0000_0000          # host entry points live here, 16 bytes apart
DLL_BASE = 0x7FF8_0000_0000            # the DLLs w2m loads, in load order
LOADER_STUB = 0x7FFD_0000_0000         # the entry stub that runs DllMains, then the program's entry
DLL_PROCESS_ATTACH = 1
HEAP_VA, HEAP_SIZE = 0x2_0000_0000, 0x1000_0000   # the VM's process heap (vm_backend.HEAP_VA, HEAP_SIZE)
MONITOR_VA = 0x7F00_0000_0000                     # the VM's monitor and process areas (vm_backend)
DLL_INIT_FAILED = "w2m-loader!DllMain returned FALSE"     # a thunk no service serves: the run stops by name
STACK_TOP = 0x0000_0001_0000_0000
STACK_SIZE = 1 << 20


class GuestFault(RuntimeError):
    """A guest fault the runtime stops on and reports: by default a memory access outside every
    mapped region (an access violation); `code` is the Windows status the guest would see
    (Sprint 044: DIV's STATUS_INTEGER_DIVIDE_BY_ZERO and STATUS_INTEGER_OVERFLOW)."""

    def __init__(self, message: str, code: int = 0xC000_0005) -> None:
        super().__init__(message)
        self.code = code


@dataclass
class Region:
    base: int
    data: bytearray | memoryview          # memoryview: guest memory shared with the VM (vm.py)
    name: str


@dataclass
class Memory:
    regions: list[Region] = field(default_factory=list)

    def map(self, base: int, size: int, name: str) -> Region:
        r = Region(base, bytearray(size), name)
        self.regions.append(r)
        return r

    def _find(self, addr: int, n: int) -> tuple[Region, int]:
        for r in self.regions:
            if r.base <= addr and addr + n <= r.base + len(r.data):
                return r, addr - r.base
        raise GuestFault(f"access of {n} bytes at 0x{addr:x} outside mapped memory")

    def read(self, addr: int, n: int) -> bytes:
        r, off = self._find(addr, n)
        return bytes(r.data[off:off + n])

    def write(self, addr: int, data: bytes) -> None:
        r, off = self._find(addr, len(data))
        r.data[off:off + len(data)] = data

    def code_at(self, addr: int, limit: int = 15) -> bytes:
        """Up to `limit` bytes from `addr`, stopping at the end of its region."""
        for r in self.regions:
            if r.base <= addr < r.base + len(r.data):
                off = addr - r.base
                return bytes(r.data[off:off + limit])
        raise GuestFault(f"instruction fetch at 0x{addr:x} outside mapped memory")


class LoadError(RuntimeError):
    """The images cannot be loaded as Windows would load them (a relocation type w2m does not apply, an
    import the supplying DLL does not export): Windows refuses to start such a process."""


@dataclass
class Image:
    name: str                            # as imported ("UnityPlayer.dll"); the program's: its file name
    pe: PeImage
    base: int

    @property
    def entry(self) -> int | None:
        return self.base + self.pe.entry_rva if self.pe.entry_rva else None

    def export(self, name: str) -> int | None:
        rva = next((r for n, r in self.pe.exports if n == name), None)
        return None if rva is None else self.base + rva


@dataclass
class Loaded:
    image: PeImage
    memory: Memory
    entry: int                           # where the thread starts: the program's entry, or the loader stub
    thunks: dict[int, str]               # thunk address -> "DLL!Name"
    stack_top: int
    images: list[Image] = field(default_factory=list)    # the program first, then DLLs in load order
    image_entry: int = 0                 # the program's own entry point
    init_order: list[Image] = field(default_factory=list)   # DLLs whose entry runs, in the order it runs
    dependency_order: list[Image] = field(default_factory=list)   # every DLL, dependencies first (PEB init list)


def _map_image(mem: Memory, data: bytes, img: PeImage, base: int, name: str) -> None:
    region = mem.map(base, img.size_of_image, name)
    headers = min((s.raw_offset for s in img.sections if s.raw_offset), default=0x400)
    region.data[:headers] = data[:headers]
    for s in img.sections:
        raw = data[s.raw_offset:s.raw_offset + min(s.raw_size, max(s.virtual_size, s.raw_size))]
        region.data[s.virtual_address:s.virtual_address + len(raw)] = raw
    delta = base - img.image_base
    if delta:
        for rva, kind in img.relocation_sites:
            if kind == 10:                                            # IMAGE_REL_BASED_DIR64
                v = int.from_bytes(region.data[rva:rva + 8], "little")
                region.data[rva:rva + 8] = ((v + delta) & (1 << 64) - 1).to_bytes(8, "little")
            elif kind == 3:                                           # IMAGE_REL_BASED_HIGHLOW
                v = int.from_bytes(region.data[rva:rva + 4], "little")
                region.data[rva:rva + 4] = ((v + delta) & 0xFFFF_FFFF).to_bytes(4, "little")
            else:
                raise LoadError(f"{name}: base relocation type {kind} at RVA 0x{rva:x}")


def _free(mem: Memory, base: int, size: int) -> bool:
    reserved = [(THUNK_BASE, 1 << 32), (LOADER_STUB, 1 << 32), (STACK_TOP - STACK_SIZE, STACK_SIZE),
                (HEAP_VA, HEAP_SIZE), (MONITOR_VA, 1 << 36)]
    taken = [(r.base, len(r.data)) for r in mem.regions] + reserved
    return all(base + size <= b or b + n <= base for b, n in taken)


def _region(mem: Memory, base: int) -> Region:
    return next(r for r in mem.regions if r.base == base)


def _stub(dlls: list[Image], program_entry: int, failed: int) -> bytes:
    """x86-64: for each DLL, DllMain(base, DLL_PROCESS_ATTACH, non-NULL) with the shadow space; a FALSE
    return calls `failed`; then a jump to the program's entry with RSP as the thread started."""
    out = bytearray()
    for d in dlls:
        assert d.entry is not None
        out += bytes.fromhex("4883ec28")                                   # sub rsp, 0x28
        out += bytes.fromhex("48b9") + d.base.to_bytes(8, "little")        # mov rcx, instance
        out += bytes.fromhex("ba") + DLL_PROCESS_ATTACH.to_bytes(4, "little")   # mov edx, reason
        out += bytes.fromhex("49b8") + LOADER_STUB.to_bytes(8, "little")   # mov r8, lpReserved (non-NULL: static load)
        out += bytes.fromhex("48b8") + d.entry.to_bytes(8, "little")       # mov rax, DllMain
        out += bytes.fromhex("ffd0")                                       # call rax
        out += bytes.fromhex("4883c428")                                   # add rsp, 0x28
        out += bytes.fromhex("85c0")                                       # test eax, eax
        out += bytes.fromhex("750c")                                       # jnz past the failure call
        out += bytes.fromhex("48b8") + failed.to_bytes(8, "little")        # mov rax, the failure thunk
        out += bytes.fromhex("ffd0")                                       # call rax (stops the run)
    out += bytes.fromhex("48b8") + program_entry.to_bytes(8, "little")     # mov rax, the program's entry
    out += bytes.fromhex("ffe0")                                           # jmp rax
    return bytes(out)


def load(data: bytes, dlls: Callable[[str], bytes | None] | None = None, name: str = "app.exe") -> Loaded:
    """The program, and each DLL `dlls` supplies by its imported name (None: not the program's to load)."""
    img = parse(data)
    mem = Memory()
    _map_image(mem, data, img, img.image_base, "image")
    program = Image(name, img, img.image_base)
    images = [program]
    loaded: dict[str, Image] = {}
    order: list[Image] = []
    next_base = DLL_BASE

    def visit(image: Image) -> None:                   # depth first: a DLL's dependencies initialize first
        nonlocal next_base
        for dll in image.pe.imports:
            key = dll.lower()
            if key in loaded or dlls is None:
                continue
            raw = dlls(dll)
            if raw is None:
                continue
            pe = parse(raw)
            if _free(mem, pe.image_base, pe.size_of_image):
                base = pe.image_base
            else:
                base = next_base
                next_base = (base + pe.size_of_image + 0xFFFF) & ~0xFFFF
            _map_image(mem, raw, pe, base, f"image {dll}")
            d = loaded[key] = Image(dll, pe, base)
            images.append(d)
            visit(d)
            order.append(d)

    visit(program)
    thunks: dict[int, str] = {}

    def thunk(target: str) -> int:
        addr = THUNK_BASE + 16 * len(thunks)
        thunks[addr] = target
        return addr

    for image in images:
        region = _region(mem, image.base)
        for slot, target in sorted(image.pe.iat.items()):
            dll, fn = target.split("!", 1)
            supplier = loaded.get(dll.lower())
            if supplier is None:
                va = thunk(target)
            else:
                export = supplier.export(fn)
                if export is None:
                    raise LoadError(f"{image.name} imports {target}, which {supplier.name} does not export")
                va = export
            region.data[slot:slot + 8] = va.to_bytes(8, "little")
    starting = [d for d in order if d.entry is not None]
    entry = program.entry or 0
    if starting:
        stub = _stub(starting, entry, thunk(DLL_INIT_FAILED))
        mem.map(LOADER_STUB, (len(stub) + 0xFFF) & ~0xFFF, "loader").data[:len(stub)] = stub
        entry = LOADER_STUB
    mem.map(THUNK_BASE, 16 * max(len(thunks), 1), "thunks")
    mem.map(STACK_TOP - STACK_SIZE, STACK_SIZE, "stack")
    return Loaded(img, mem, entry, thunks, STACK_TOP, images, program.entry or 0, starting, order)
