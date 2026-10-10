"""loader.py — map a PE32+ image into guest memory and bind its imports (Sprint 029).

The image is mapped at its preferred base (ImageBase); headers and each section's raw data are
copied, and the rest of SizeOfImage reads as zero (PE Format, Section Table: "If this value
[VirtualSize] is greater than SizeOfRawData, the section is zero-padded"). Each import address
table slot receives the address of a host entry point: a thunk address outside the image that the
runtime recognizes and services in the host (src/w2m/runtime.py). Base relocations are not applied
because the image is loaded at its preferred base; a load elsewhere is refused, not attempted.
"""

from __future__ import annotations

from dataclasses import dataclass, field

from w2m.pe import PeImage, parse

THUNK_BASE = 0x7FFE_0000_0000          # host entry points live here, 16 bytes apart
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


@dataclass
class Loaded:
    image: PeImage
    memory: Memory
    entry: int
    thunks: dict[int, str]               # thunk address -> "DLL!Name"
    stack_top: int


def load(data: bytes) -> Loaded:
    img = parse(data)
    mem = Memory()
    region = mem.map(img.image_base, img.size_of_image, "image")
    headers = min((s.raw_offset for s in img.sections if s.raw_offset), default=0x400)
    region.data[:headers] = data[:headers]
    for s in img.sections:
        raw = data[s.raw_offset:s.raw_offset + min(s.raw_size, max(s.virtual_size, s.raw_size))]
        region.data[s.virtual_address:s.virtual_address + len(raw)] = raw
    thunks: dict[int, str] = {}
    for i, (slot, name) in enumerate(sorted(img.iat.items())):
        addr = THUNK_BASE + 16 * i
        thunks[addr] = name
        region.data[slot:slot + 8] = addr.to_bytes(8, "little")
    mem.map(THUNK_BASE, 16 * max(len(thunks), 1), "thunks")
    mem.map(STACK_TOP - STACK_SIZE, STACK_SIZE, "stack")
    return Loaded(img, mem, img.image_base + img.entry_rva, thunks, STACK_TOP)
