# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""virtual_memory.py — VirtualAlloc, VirtualFree, VirtualProtect and VirtualQuery for one process (Sprint 046).

The guest runtime (`tools/guest_runtime/virtual_memory.c`) passes each call to the host (HOST_VIRTUAL), and
this module answers as contracts/windows.memory.virtual records Windows answering on the runners: an
arena mapped into the VM holds the reservations, 64 KB aligned, each page with a state (reserved,
committed) and a protection; a commit gives zeroed pages, a decommit zeroes them again; VirtualQuery
reports the run of pages with the same attributes from the page asked for, and on an image's pages the
image's section protections. Protections are recorded, not enforced: a store to a read-only page succeeds
in w2m (enforcement needs the VM's page tables per page; a later sprint).

A form the rows do not show (an address outside the arena and the images, a flag combination not
observed) stops the run by name.
"""

from __future__ import annotations

from collections.abc import Callable
from array import array
from dataclasses import dataclass, field

from w2m.kernel32 import Unimplemented
from w2m.pe import PeImage, SCN_MEM_EXECUTE, SCN_MEM_READ, SCN_MEM_WRITE

PAGE, GRANULARITY = 0x1000, 0x10000
MEM_COMMIT, MEM_RESERVE, MEM_DECOMMIT, MEM_RELEASE = 0x1000, 0x2000, 0x4000, 0x8000
MEM_FREE, MEM_PRIVATE, MEM_MAPPED, MEM_IMAGE = 0x10000, 0x20000, 0x40000, 0x1000000
PAGE_NOACCESS, PAGE_READONLY, PAGE_READWRITE, PAGE_WRITECOPY = 0x01, 0x02, 0x04, 0x08
PAGE_EXECUTE, PAGE_EXECUTE_READ, PAGE_EXECUTE_READWRITE, PAGE_EXECUTE_WRITECOPY = 0x10, 0x20, 0x40, 0x80
PROTECTIONS = {PAGE_NOACCESS, PAGE_READONLY, PAGE_READWRITE, PAGE_WRITECOPY, PAGE_EXECUTE, PAGE_EXECUTE_READ,
               PAGE_EXECUTE_READWRITE, PAGE_EXECUTE_WRITECOPY}
ERROR_INVALID_PARAMETER, ERROR_NOT_SUPPORTED, ERROR_INVALID_ADDRESS = 87, 50, 487
RESERVED, COMMITTED = 1, 2
PAGE_GUARD = 0x100
STACK_GUARD_PAGES = 3          # the guard below StackLimit (contracts/windows.memory.thread_stack, Sprint 049)


def _down(v: int, to: int) -> int:
    return v & ~(to - 1)


def _up(v: int, to: int) -> int:
    return (v + to - 1) & ~(to - 1)


@dataclass
class Reservation:
    base: int
    size: int                        # page-rounded
    allocation_protect: int
    state: bytearray = field(default_factory=bytearray)      # per page: RESERVED or COMMITTED
    protect: array[int] = field(default_factory=lambda: array("H"))   # per page: the protection while committed (16
                                                              # bits: PAGE_GUARD 0x100 with the rest, a stack's guard)
    type: int = MEM_PRIVATE                                   # MEM_MAPPED for a file view (Sprint 049)

    def __post_init__(self) -> None:
        pages = self.size // PAGE
        self.state = bytearray([RESERVED]) * pages
        self.protect = array("H", bytes(2 * pages))


@dataclass
class Info:
    """MEMORY_BASIC_INFORMATION's fields."""
    base: int
    allocation_base: int
    allocation_protect: int
    region_size: int
    state: int
    protect: int
    type: int


@dataclass
class VirtualMemory:
    arena: int                       # VA of the arena the VM maps (vm_backend.VIRTUAL_VA)
    arena_size: int
    zero: Callable[[int, int], None]  # writes zeros over [va, va + n) of guest memory
    images: list[tuple[int, PeImage]] = field(default_factory=list)   # (base, image)
    read: Callable[[int, int], bytes] | None = None    # guest memory, for the copy-on-write test below
    pristine: dict[int, bytes] = field(default_factory=dict)   # writable image page VA -> its bytes at load
    reservations: dict[int, Reservation] = field(default_factory=dict)
    image_protect: dict[int, int] = field(default_factory=dict)       # page VA -> a protection VirtualProtect set
    stack: tuple[int, int, int] | None = None    # the main thread's (DeallocationStack, StackLimit, StackBase)
    stacks: list[tuple[int, int, int]] = field(default_factory=list)   # every other thread's (Sprint 051): their
                                                 # reservations hold no guard pages, all committed above StackLimit
    mapped: list[tuple[int, int]] = field(default_factory=list)   # every range the VM maps, (base, size): what is
                                                 # outside all of them is free address space (Sprint 049)
    heap: tuple[int, int] | None = None          # the process heap's reservation (base, size); heap.c keeps its high
                                                 # water mark at base + 8

    # ---- lookups -------------------------------------------------------------------------------------
    def _reservation(self, va: int) -> Reservation | None:
        for r in self.reservations.values():
            if r.base <= va < r.base + r.size:
                return r
        return None

    def _image(self, va: int) -> tuple[int, PeImage] | None:
        for base, pe in self.images:
            if base <= va < base + pe.size_of_image:
                return base, pe
        return None

    def _in_arena(self, va: int, size: int = 1) -> bool:
        return self.arena <= va and va + size <= self.arena + self.arena_size

    def _free_range(self, size: int) -> int:
        """The lowest 64 KB-aligned arena address with `size` bytes free (reservations and images loaded into the
        arena both take space)."""
        at = self.arena
        taken = [(r.base, r.size) for r in self.reservations.values()]
        taken += [(b, pe.size_of_image) for b, pe in self.images if self._in_arena(b)]
        for base, n in sorted(taken):
            if at + size <= base:
                return at
            at = max(at, _up(base + n, GRANULARITY))
        if at + size > self.arena + self.arena_size:
            raise Unimplemented(f"VirtualAlloc of 0x{size:x} bytes: the arena (0x{self.arena_size:x}) is full")
        return at

    # ---- the four calls ------------------------------------------------------------------------------
    def alloc(self, address: int, size: int, kind: int, protect: int) -> tuple[int, int]:
        """VirtualAlloc: (address, 0) or (0, error)."""
        if size == 0:
            return 0, ERROR_INVALID_PARAMETER
        if protect not in PROTECTIONS:
            return 0, ERROR_NOT_SUPPORTED                    # row alloc.bad_protect (0x12345)
        if kind & ~(MEM_COMMIT | MEM_RESERVE) or not kind:
            raise Unimplemented(f"VirtualAlloc type 0x{kind:x}")
        if kind & MEM_RESERVE or not address:     # MEM_COMMIT alone at NULL reserves too (row alloc.commit_null)
            if address:
                base = _down(address, GRANULARITY)
                end = _up(address + size, PAGE)
                if not self._in_arena(base, end - base):
                    raise Unimplemented(f"VirtualAlloc reserve at 0x{address:x}, outside w2m's arena")
                taken = [(r.base, r.size) for r in self.reservations.values()]
                taken += [(b, pe.size_of_image) for b, pe in self.images]      # a DLL loaded at run time (Sprint 065:
                if any(b < end and base < b + n for b, n in taken):            # mono reserved its code block over
                    return 0, ERROR_INVALID_ADDRESS                            # steam_api64.dll and wrote JIT code there)
            else:
                end = 0
                base = self._free_range(_up(size, PAGE))
            made = Reservation(base, (end - base) if address else _up(size, PAGE), protect)
            self.reservations[base] = made
            if kind & MEM_COMMIT:
                self._commit(made, base, made.size, protect)
            return base, 0
        r = self._reservation(address)
        start, end = _down(address, PAGE), _up(address + size, PAGE)
        if r is None:
            if self._in_arena(address) or self._image(address) is None:
                return 0, ERROR_INVALID_ADDRESS
            raise Unimplemented(f"VirtualAlloc commit at 0x{address:x}, in an image")
        if end > r.base + r.size:
            return 0, ERROR_INVALID_ADDRESS
        self._commit(r, start, end - start, protect)
        return start, 0

    def _commit(self, r: Reservation, start: int, size: int, protect: int) -> None:
        first = (start - r.base) // PAGE
        for i in range(first, first + size // PAGE):
            if r.state[i] != COMMITTED:
                self.zero(r.base + i * PAGE, PAGE)
                r.state[i] = COMMITTED
            r.protect[i] = protect

    def free(self, address: int, size: int, kind: int) -> tuple[int, int]:
        """VirtualFree: (1, 0) or (0, error)."""
        if kind == MEM_RELEASE:
            if size:
                return 0, ERROR_INVALID_PARAMETER
            r = self.reservations.get(address)
            if r is None:
                return 0, ERROR_INVALID_ADDRESS
            if r.type == MEM_MAPPED:
                raise Unimplemented(f"VirtualFree of the file view at 0x{address:x}")      # not in the rows
            for i, s in enumerate(r.state):
                if s == COMMITTED:
                    self.zero(r.base + i * PAGE, PAGE)
            del self.reservations[address]
            return 1, 0
        if kind == MEM_DECOMMIT:
            r = self._reservation(address)
            if r is None:
                return 0, ERROR_INVALID_ADDRESS
            start = _down(address, PAGE)
            end = r.base + r.size if size == 0 and address == r.base else _up(address + size, PAGE)
            if end > r.base + r.size:
                return 0, ERROR_INVALID_ADDRESS
            for i in range((start - r.base) // PAGE, (end - r.base) // PAGE):
                if r.state[i] == COMMITTED:
                    self.zero(r.base + i * PAGE, PAGE)
                r.state[i] = RESERVED
                r.protect[i] = 0
            return 1, 0
        raise Unimplemented(f"VirtualFree type 0x{kind:x}")

    def protect(self, address: int, size: int, new: int) -> tuple[int, int, int]:
        """VirtualProtect: (1, old, 0) or (0, old, error); `old` is the first page's protection, PAGE_NOACCESS
        for a page only reserved (Windows writes it on that failure too: row protect.reserved_page)."""
        if new not in PROTECTIONS:
            return 0, 0, ERROR_INVALID_PARAMETER
        start, end = _down(address, PAGE), _up(address + size, PAGE)
        r = self._reservation(address)
        if r is not None:
            pages = range((start - r.base) // PAGE, (end - r.base) // PAGE)
            old = r.protect[pages[0]] if r.state[pages[0]] == COMMITTED else PAGE_NOACCESS
            if end > r.base + r.size or any(r.state[i] != COMMITTED for i in pages):
                return 0, old, ERROR_INVALID_ADDRESS
            for i in pages:
                r.protect[i] = new
            return 1, old, 0
        image = self._image(address)
        if image is None:
            raise Unimplemented(f"VirtualProtect at 0x{address:x}, outside the arena and the images")
        old = self._image_page_protect(image, start)
        for va in range(start, end, PAGE):
            self.image_protect[va] = new
        return 1, old, 0

    def query(self, address: int) -> Info | None:
        page = _down(address, PAGE)
        if self.stack is not None and self.stack[0] <= address < self.stack[2]:
            return self._stack_query(page)
        if self.heap is not None and self.heap[0] <= address < self.heap[0] + self.heap[1]:
            return self._heap_query(page)
        r = self._reservation(address)
        if r is not None:
            i = (page - r.base) // PAGE
            j = i
            while j < len(r.state) and r.state[j] == r.state[i] and r.protect[j] == r.protect[i]:
                j += 1
            committed = r.state[i] == COMMITTED
            return Info(page, r.base, r.allocation_protect, (j - i) * PAGE, MEM_COMMIT if committed else MEM_RESERVE,
                        r.protect[i] if committed else 0, r.type)
        image = self._image(address)
        if image is not None:
            base, pe = image
            p = self._image_page_protect(image, page)
            end = page
            while end < base + _up(pe.size_of_image, PAGE) and self._image_page_protect(image, end) == p:
                end += PAGE
            return Info(page, base, IMAGE_ALLOCATION_PROTECT, end - page, MEM_COMMIT, p, MEM_IMAGE)
        if self._in_arena(address):
            nxt = min((r.base for r in self.reservations.values() if r.base > page), default=self.arena + self.arena_size)
            return Info(page, 0, 0, nxt - page, MEM_FREE, PAGE_NOACCESS, 0)
        if self.mapped and not any(b <= address < b + n for b, n in self.mapped):
            # free address space, as a released range reads (contracts/windows.memory.virtual row query.released): no allocation, PAGE_NOACCESS, up
            # to the next mapped range (Sprint 049: mono's collector walks past the stack's top)
            nxt = min((b for b, _ in self.mapped if b > page), default=1 << 47)
            return Info(page, 0, 0, nxt - page, MEM_FREE, PAGE_NOACCESS, 0)
        raise Unimplemented(f"VirtualQuery at 0x{address:x}, outside the arena and the images")

    def map_view(self, size: int, protect: int) -> int:
        """A view of a file mapping (contracts/windows.file.file_mapping, Sprint 049): a 64 KB-aligned MEM_MAPPED
        region of the view's pages, committed with `protect`, its allocation protection the same; the rest of the
        64 KB granule is free address space, as on Windows (rows query.after_view, query.granule_end). The caller
        writes the file's bytes over the zeroed pages."""
        pages = _up(size, PAGE)
        base = self._free_range(pages)
        r = Reservation(base, pages, protect, type=MEM_MAPPED)
        self.reservations[base] = r
        self._commit(r, base, pages, protect)
        return base

    def place_image(self, pe: PeImage) -> int:
        """Where a DLL loaded at run time goes (Sprint 049): its preferred base when that lies free in the arena,
        else the lowest free 64 KB-aligned range, relocated there by the caller. The image is then one of
        `images` (VirtualQuery reports MEM_IMAGE); `snapshot_image` records its writable pages once written."""
        size = _up(pe.size_of_image, PAGE)
        base = pe.image_base
        free = self._in_arena(base, size) and base % GRANULARITY == 0 and all(
            b + n <= base or base + size <= b for b, n in
            [(r.base, r.size) for r in self.reservations.values()] + [(b, p.size_of_image) for b, p in self.images])
        if not free:
            base = self._free_range(size)
        self.images.append((base, pe))
        return base

    def snapshot_image(self, base: int, pe: PeImage) -> None:
        assert self.read is not None
        for s in pe.sections:
            if s.characteristics & SCN_MEM_WRITE:
                start = base + s.virtual_address
                for va in range(start, start + _up(max(s.virtual_size, 1), PAGE), PAGE):
                    self.pristine[va] = self.read(va, PAGE)

    def _heap_query(self, page: int) -> Info:
        """The process heap as contracts/windows.memory.heap_query records it (Sprint 049): one MEM_PRIVATE
        reservation whose base is the heap's handle, read-write, committed up to the pages heap.c has used and
        reserved after; BaseAddress is the queried page."""
        assert self.read is not None and self.heap is not None
        base, size = self.heap
        used = int.from_bytes(self.read(base + 8, 8), "little") or base + PAGE      # heap.c's `next`
        committed = _up(max(used, base + PAGE), PAGE)
        if page < committed:
            return Info(page, base, PAGE_READWRITE, committed - page, MEM_COMMIT, PAGE_READWRITE, MEM_PRIVATE)
        return Info(page, base, PAGE_READWRITE, base + size - page, MEM_RESERVE, 0, MEM_PRIVATE)

    def _stack_query(self, page: int) -> Info:
        """The thread's stack as contracts/windows.memory.thread_stack records it (Sprint 049): one MEM_PRIVATE
        reservation from DeallocationStack, read-write: reserved below StackLimit - 3 pages, those 3 pages committed
        with PAGE_GUARD, and committed read-write from StackLimit to StackBase; BaseAddress is the queried page."""
        dealloc, limit, base = self.stack                       # type: ignore[misc]
        guard = limit - STACK_GUARD_PAGES * PAGE
        if page < guard:
            return Info(page, dealloc, PAGE_READWRITE, guard - page, MEM_RESERVE, 0, MEM_PRIVATE)
        if page < limit:
            return Info(page, dealloc, PAGE_READWRITE, limit - page, MEM_COMMIT, PAGE_READWRITE | PAGE_GUARD, MEM_PRIVATE)
        return Info(page, dealloc, PAGE_READWRITE, base - page, MEM_COMMIT, PAGE_READWRITE, MEM_PRIVATE)

    def snapshot(self) -> None:
        """Remember every writable image page as loaded (the copy-on-write test of _image_page_protect)."""
        assert self.read is not None
        for base, pe in self.images:
            for s in pe.sections:
                if s.characteristics & SCN_MEM_WRITE:
                    start = base + s.virtual_address
                    for va in range(start, start + _up(max(s.virtual_size, 1), PAGE), PAGE):
                        self.pristine[va] = self.read(va, PAGE)

    def _image_page_protect(self, image: tuple[int, PeImage], va: int) -> int:
        """A section's protection; a writable page is PAGE_WRITECOPY until the process writes it, then
        PAGE_READWRITE (rows image.section._data, image.rdata_after: only the first .data page, which the
        probe had written, is read-write). Written means its bytes differ from load: a write of the same
        bytes is missed, and relocation fixups, which Windows's loader writes, count as load."""
        if va in self.image_protect:
            return self.image_protect[va]
        base, pe = image
        for s in pe.sections:
            if base + s.virtual_address <= va < base + s.virtual_address + _up(max(s.virtual_size, 1), PAGE):
                p = section_protect(s.characteristics)
                if p == PAGE_READWRITE and va in self.pristine and self.read is not None:
                    return PAGE_READWRITE if self.read(va, PAGE) != self.pristine[va] else PAGE_WRITECOPY
                return p
        return IMAGE_HEADERS_PROTECT


# Filled from the rows (contracts/windows.memory.virtual): what VirtualQuery reports on an image.
IMAGE_ALLOCATION_PROTECT = PAGE_EXECUTE_WRITECOPY
IMAGE_HEADERS_PROTECT = PAGE_READONLY


def section_protect(characteristics: int) -> int:
    """The rows: .text PAGE_EXECUTE_READ, .rdata PAGE_READONLY, .data PAGE_READWRITE once written (PAGE_WRITECOPY
    before: _image_page_protect); a writable executable section is not in them (PAGE_EXECUTE_READWRITE here)."""
    x, r, w = (bool(characteristics & f) for f in (SCN_MEM_EXECUTE, SCN_MEM_READ, SCN_MEM_WRITE))
    if x:
        return PAGE_EXECUTE_READWRITE if w else PAGE_EXECUTE_READ if r else PAGE_EXECUTE
    return PAGE_READWRITE if w else PAGE_READONLY if r else PAGE_NOACCESS
