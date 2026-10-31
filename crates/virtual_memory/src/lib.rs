// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! VirtualAlloc, VirtualFree, VirtualProtect and VirtualQuery for one process (Sprint 146), carried from
//! `src/w2m/virtual_memory.py`.
//!
//! The guest runtime (`tools/guest_runtime/virtual_memory.c`) passes each call to the host, and this crate answers as
//! `contracts/windows.memory.virtual` records Windows answering on the runners: an arena mapped into the VM holds the
//! reservations, 64 KB aligned, each page with a state (reserved, committed) and a protection; a commit gives zeroed
//! pages, a decommit zeroes them again; VirtualQuery reports the run of pages with the same attributes from the page
//! asked for, and on an image's pages the image's section protections. Protections are recorded, not enforced.
//!
//! A form the rows do not show (an address outside the arena and the images, a flag combination not observed) stops
//! by name with `Unimplemented`. Each item cites the `virtual_memory.py` lines it carries.

use std::collections::BTreeMap;
use std::fmt;

use pe_image::{PeImage, SCN_MEM_EXECUTE, SCN_MEM_READ, SCN_MEM_WRITE};

/// virtual_memory.py 26-36.
pub use guest_layout::PAGE;
pub const GRANULARITY: u64 = 0x10000;
pub const MEM_COMMIT: u32 = 0x1000;
pub const MEM_RESERVE: u32 = 0x2000;
pub const MEM_DECOMMIT: u32 = 0x4000;
pub const MEM_RELEASE: u32 = 0x8000;
pub const MEM_FREE: u32 = 0x10000;
pub const MEM_PRIVATE: u32 = 0x20000;
pub const MEM_MAPPED: u32 = 0x40000;
pub const MEM_IMAGE: u32 = 0x100_0000;
pub const PAGE_NOACCESS: u32 = 0x01;
pub const PAGE_READONLY: u32 = 0x02;
pub const PAGE_READWRITE: u32 = 0x04;
pub const PAGE_WRITECOPY: u32 = 0x08;
pub const PAGE_EXECUTE: u32 = 0x10;
pub const PAGE_EXECUTE_READ: u32 = 0x20;
pub const PAGE_EXECUTE_READWRITE: u32 = 0x40;
pub const PAGE_EXECUTE_WRITECOPY: u32 = 0x80;
pub const PROTECTIONS: [u32; 8] = [
    PAGE_NOACCESS,
    PAGE_READONLY,
    PAGE_READWRITE,
    PAGE_WRITECOPY,
    PAGE_EXECUTE,
    PAGE_EXECUTE_READ,
    PAGE_EXECUTE_READWRITE,
    PAGE_EXECUTE_WRITECOPY,
];
pub const ERROR_INVALID_PARAMETER: u32 = 87;
pub const ERROR_NOT_SUPPORTED: u32 = 50;
pub const ERROR_INVALID_ADDRESS: u32 = 487;
pub const PAGE_GUARD: u32 = 0x100;
/// the guard below StackLimit (contracts/windows.memory.thread_stack, Sprint 049)
pub const STACK_GUARD_PAGES: u64 = 3;

/// virtual_memory.py 341-343: filled from the rows, what VirtualQuery reports on an image.
pub const IMAGE_ALLOCATION_PROTECT: u32 = PAGE_EXECUTE_WRITECOPY;
pub const IMAGE_HEADERS_PROTECT: u32 = PAGE_READONLY;

const RESERVED: u8 = 1;
const COMMITTED: u8 = 2;

/// A form the rows do not show: the run stops by name (kernel32.py's `Unimplemented`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unimplemented(pub String);

impl fmt::Display for Unimplemented {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Unimplemented {}

/// Guest memory as the arena touches it: the Python's `zero` and `read` callables (virtual_memory.py 79, 81).
pub trait GuestPages {
    /// zeros over [va, va + n)
    fn zero(&mut self, va: u64, n: u64);
    /// the n bytes at va
    fn read(&self, va: u64, n: u64) -> Vec<u8>;
}

fn down(v: u64, to: u64) -> u64 {
    v & !(to - 1)
}

fn up(v: u64, to: u64) -> u64 {
    (v + to - 1) & !(to - 1)
}

/// virtual_memory.py 47-60: a reservation, page-rounded, each page's state and its protection while committed (16
/// bits: PAGE_GUARD with the rest, a stack's guard).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reservation {
    pub base: u64,
    pub size: u64,
    pub allocation_protect: u32,
    pub state: Vec<u8>,
    pub protect: Vec<u16>,
    /// MEM_MAPPED for a file view (Sprint 049)
    pub kind: u32,
}

impl Reservation {
    fn new(base: u64, size: u64, allocation_protect: u32, kind: u32) -> Reservation {
        let pages = (size / PAGE) as usize;
        Reservation {
            base,
            size,
            allocation_protect,
            state: vec![RESERVED; pages],
            protect: vec![0; pages],
            kind,
        }
    }
}

/// virtual_memory.py 63-72: MEMORY_BASIC_INFORMATION's fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Info {
    pub base: u64,
    pub allocation_base: u64,
    pub allocation_protect: u32,
    pub region_size: u64,
    pub state: u32,
    pub protect: u32,
    pub kind: u32,
}

/// virtual_memory.py 75-91: one process's arena, its images, and the stacks and heap VirtualQuery reports.
#[derive(Debug, Clone, Default)]
pub struct VirtualMemory {
    /// the VA of the arena the VM maps (vm_backend.VIRTUAL_VA)
    pub arena: u64,
    pub arena_size: u64,
    /// (base, image)
    pub images: Vec<(u64, PeImage)>,
    /// writable image page VA -> its bytes at load
    pub pristine: BTreeMap<u64, Vec<u8>>,
    pub reservations: BTreeMap<u64, Reservation>,
    /// page VA -> a protection VirtualProtect set
    pub image_protect: BTreeMap<u64, u32>,
    /// the main thread's (DeallocationStack, StackLimit, StackBase)
    pub stack: Option<(u64, u64, u64)>,
    /// every other thread's (Sprint 051): their reservations hold no guard pages, all committed above StackLimit
    pub stacks: Vec<(u64, u64, u64)>,
    /// every range the VM maps, (base, size): what is outside all of them is free address space (Sprint 049)
    pub mapped: Vec<(u64, u64)>,
    /// the process heap's reservation (base, size); heap.c keeps its high water mark at base + 8
    pub heap: Option<(u64, u64)>,
}

impl VirtualMemory {
    pub fn new(arena: u64, arena_size: u64, images: Vec<(u64, PeImage)>) -> VirtualMemory {
        VirtualMemory {
            arena,
            arena_size,
            images,
            ..VirtualMemory::default()
        }
    }

    // ---- lookups (virtual_memory.py 93-121) ----------------------------------------------------------------------

    fn reservation_at(&self, va: u64) -> Option<u64> {
        self.reservations
            .values()
            .find(|r| r.base <= va && va < r.base + r.size)
            .map(|r| r.base)
    }

    fn image_at(&self, va: u64) -> Option<usize> {
        self.images
            .iter()
            .position(|(base, pe)| *base <= va && va < base + pe.size_of_image as u64)
    }

    fn in_arena(&self, va: u64, size: u64) -> bool {
        self.arena <= va && va + size <= self.arena + self.arena_size
    }

    /// The lowest 64 KB-aligned arena address with `size` bytes free (reservations and images loaded into the arena
    /// both take space).
    fn free_range(&self, size: u64) -> Result<u64, Unimplemented> {
        let mut at = self.arena;
        let mut taken: Vec<(u64, u64)> = self.reservations.values().map(|r| (r.base, r.size)).collect();
        taken.extend(
            self.images
                .iter()
                .filter(|(b, _)| self.in_arena(*b, 1))
                .map(|(b, pe)| (*b, pe.size_of_image as u64)),
        );
        taken.sort();
        for (base, n) in taken {
            if at + size <= base {
                return Ok(at);
            }
            at = at.max(up(base + n, GRANULARITY));
        }
        if at + size > self.arena + self.arena_size {
            return Err(Unimplemented(format!(
                "VirtualAlloc of 0x{size:x} bytes: the arena (0x{:x}) is full",
                self.arena_size
            )));
        }
        Ok(at)
    }

    // ---- the four calls (virtual_memory.py 123-254) ----------------------------------------------------------------

    /// VirtualAlloc (124-159): (address, 0) or (0, error).
    pub fn alloc(
        &mut self,
        pages: &mut dyn GuestPages,
        address: u64,
        size: u64,
        kind: u32,
        protect: u32,
    ) -> Result<(u64, u32), Unimplemented> {
        if size == 0 {
            return Ok((0, ERROR_INVALID_PARAMETER));
        }
        if !PROTECTIONS.contains(&protect) {
            return Ok((0, ERROR_NOT_SUPPORTED)); // row alloc.bad_protect (0x12345)
        }
        if kind & !(MEM_COMMIT | MEM_RESERVE) != 0 || kind == 0 {
            return Err(Unimplemented(format!("VirtualAlloc type 0x{kind:x}")));
        }
        if kind & MEM_RESERVE != 0 || address == 0 {
            // MEM_COMMIT alone at NULL reserves too (row alloc.commit_null)
            let (base, reserved) = if address != 0 {
                let base = down(address, GRANULARITY);
                let end = up(address + size, PAGE);
                if !self.in_arena(base, end - base) {
                    return Err(Unimplemented(format!(
                        "VirtualAlloc reserve at 0x{address:x}, outside w2m's arena"
                    )));
                }
                // a DLL loaded at run time (Sprint 065: mono reserved its code block over steam_api64.dll and wrote
                // JIT code there)
                let taken = self
                    .reservations
                    .values()
                    .map(|r| (r.base, r.size))
                    .chain(self.images.iter().map(|(b, pe)| (*b, pe.size_of_image as u64)));
                if taken.into_iter().any(|(b, n)| b < end && base < b + n) {
                    return Ok((0, ERROR_INVALID_ADDRESS));
                }
                (base, end - base)
            } else {
                (self.free_range(up(size, PAGE))?, up(size, PAGE))
            };
            self.reservations
                .insert(base, Reservation::new(base, reserved, protect, MEM_PRIVATE));
            if kind & MEM_COMMIT != 0 {
                self.commit(pages, base, base, reserved, protect);
            }
            return Ok((base, 0));
        }
        let (start, end) = (down(address, PAGE), up(address + size, PAGE));
        let Some(r) = self.reservation_at(address) else {
            if self.in_arena(address, 1) || self.image_at(address).is_none() {
                return Ok((0, ERROR_INVALID_ADDRESS));
            }
            return Err(Unimplemented(format!(
                "VirtualAlloc commit at 0x{address:x}, in an image"
            )));
        };
        let res = &self.reservations[&r];
        if end > res.base + res.size {
            return Ok((0, ERROR_INVALID_ADDRESS));
        }
        self.commit(pages, r, start, end - start, protect);
        Ok((start, 0))
    }

    /// `_commit` (161-167): pages not yet committed are zeroed; every page takes the protection.
    pub fn commit(&mut self, pages: &mut dyn GuestPages, reservation: u64, start: u64, size: u64, protect: u32) {
        let r = self.reservations.get_mut(&reservation).unwrap();
        let first = ((start - r.base) / PAGE) as usize;
        for i in first..first + (size / PAGE) as usize {
            if r.state[i] != COMMITTED {
                pages.zero(r.base + i as u64 * PAGE, PAGE);
                r.state[i] = COMMITTED;
            }
            r.protect[i] = protect as u16;
        }
    }

    /// VirtualFree (169-198): (1, 0) or (0, error).
    pub fn free(
        &mut self,
        pages: &mut dyn GuestPages,
        address: u64,
        size: u64,
        kind: u32,
    ) -> Result<(u64, u32), Unimplemented> {
        if kind == MEM_RELEASE {
            if size != 0 {
                return Ok((0, ERROR_INVALID_PARAMETER));
            }
            let Some(r) = self.reservations.get(&address) else {
                return Ok((0, ERROR_INVALID_ADDRESS));
            };
            if r.kind == MEM_MAPPED {
                // not in the rows
                return Err(Unimplemented(format!("VirtualFree of the file view at 0x{address:x}")));
            }
            for (i, s) in r.state.iter().enumerate() {
                if *s == COMMITTED {
                    pages.zero(r.base + i as u64 * PAGE, PAGE);
                }
            }
            self.reservations.remove(&address);
            return Ok((1, 0));
        }
        if kind == MEM_DECOMMIT {
            let Some(base) = self.reservation_at(address) else {
                return Ok((0, ERROR_INVALID_ADDRESS));
            };
            let r = self.reservations.get_mut(&base).unwrap();
            let start = down(address, PAGE);
            let end = if size == 0 && address == r.base {
                r.base + r.size
            } else {
                up(address + size, PAGE)
            };
            if end > r.base + r.size {
                return Ok((0, ERROR_INVALID_ADDRESS));
            }
            for i in ((start - r.base) / PAGE) as usize..((end - r.base) / PAGE) as usize {
                if r.state[i] == COMMITTED {
                    pages.zero(r.base + i as u64 * PAGE, PAGE);
                }
                r.state[i] = RESERVED;
                r.protect[i] = 0;
            }
            return Ok((1, 0));
        }
        Err(Unimplemented(format!("VirtualFree type 0x{kind:x}")))
    }

    /// VirtualProtect (200-221): (1, old, 0) or (0, old, error); `old` is the first page's protection, PAGE_NOACCESS
    /// for a page only reserved (Windows writes it on that failure too: row protect.reserved_page). A size that
    /// rounds to no page stops by name (the Python indexed an empty range).
    pub fn protect(
        &mut self,
        pages: &dyn GuestPages,
        address: u64,
        size: u64,
        new: u32,
    ) -> Result<(u64, u32, u32), Unimplemented> {
        if !PROTECTIONS.contains(&new) {
            return Ok((0, 0, ERROR_INVALID_PARAMETER));
        }
        let (start, end) = (down(address, PAGE), up(address + size, PAGE));
        if let Some(base) = self.reservation_at(address) {
            let r = self.reservations.get_mut(&base).unwrap();
            let (first, last) = (((start - r.base) / PAGE) as usize, ((end - r.base) / PAGE) as usize);
            if first == last {
                return Err(Unimplemented(format!(
                    "VirtualProtect of 0x{size:x} bytes at 0x{address:x}"
                )));
            }
            let old = if r.state[first] == COMMITTED {
                r.protect[first] as u32
            } else {
                PAGE_NOACCESS
            };
            if end > r.base + r.size || (first..last).any(|i| r.state[i] != COMMITTED) {
                return Ok((0, old, ERROR_INVALID_ADDRESS));
            }
            for i in first..last {
                r.protect[i] = new as u16;
            }
            return Ok((1, old, 0));
        }
        let Some(image) = self.image_at(address) else {
            return Err(Unimplemented(format!(
                "VirtualProtect at 0x{address:x}, outside the arena and the images"
            )));
        };
        let old = self.image_page_protect(pages, image, start);
        for va in (start..end).step_by(PAGE as usize) {
            self.image_protect.insert(va, new);
        }
        Ok((1, old, 0))
    }

    /// VirtualQuery (223-254).
    pub fn query(&self, pages: &dyn GuestPages, address: u64) -> Result<Info, Unimplemented> {
        let page = down(address, PAGE);
        if let Some(stack) = self.stack
            && stack.0 <= address
            && address < stack.2
        {
            return Ok(self.stack_query(page));
        }
        if let Some((base, size)) = self.heap
            && base <= address
            && address < base + size
        {
            return Ok(self.heap_query(pages, page));
        }
        if let Some(base) = self.reservation_at(address) {
            let r = &self.reservations[&base];
            let i = ((page - r.base) / PAGE) as usize;
            let mut j = i;
            while j < r.state.len() && r.state[j] == r.state[i] && r.protect[j] == r.protect[i] {
                j += 1;
            }
            let committed = r.state[i] == COMMITTED;
            return Ok(Info {
                base: page,
                allocation_base: r.base,
                allocation_protect: r.allocation_protect,
                region_size: (j - i) as u64 * PAGE,
                state: if committed { MEM_COMMIT } else { MEM_RESERVE },
                protect: if committed { r.protect[i] as u32 } else { 0 },
                kind: r.kind,
            });
        }
        if let Some(image) = self.image_at(address) {
            let (base, pe) = &self.images[image];
            let p = self.image_page_protect(pages, image, page);
            let mut end = page;
            while end < base + up(pe.size_of_image as u64, PAGE) && self.image_page_protect(pages, image, end) == p {
                end += PAGE;
            }
            return Ok(Info {
                base: page,
                allocation_base: *base,
                allocation_protect: IMAGE_ALLOCATION_PROTECT,
                region_size: end - page,
                state: MEM_COMMIT,
                protect: p,
                kind: MEM_IMAGE,
            });
        }
        let free = |next: u64| Info {
            base: page,
            allocation_base: 0,
            allocation_protect: 0,
            region_size: next - page,
            state: MEM_FREE,
            protect: PAGE_NOACCESS,
            kind: 0,
        };
        if self.in_arena(address, 1) {
            let next = self
                .reservations
                .values()
                .map(|r| r.base)
                .filter(|b| *b > page)
                .min()
                .unwrap_or(self.arena + self.arena_size);
            return Ok(free(next));
        }
        if !self.mapped.is_empty() && !self.mapped.iter().any(|(b, n)| *b <= address && address < b + n) {
            // free address space, as a released range reads (contracts/windows.memory.virtual row query.released):
            // no allocation, PAGE_NOACCESS, up to the next mapped range (Sprint 049: mono's collector walks past the
            // stack's top)
            let next = self
                .mapped
                .iter()
                .map(|(b, _)| *b)
                .filter(|b| *b > page)
                .min()
                .unwrap_or(1 << 47);
            return Ok(free(next));
        }
        Err(Unimplemented(format!(
            "VirtualQuery at 0x{address:x}, outside the arena and the images"
        )))
    }

    /// `map_view` (256-266): a view of a file mapping (contracts/windows.file.file_mapping, Sprint 049), a 64
    /// KB-aligned MEM_MAPPED region of the view's pages, committed with `protect`, its allocation protection the same;
    /// the rest of the 64 KB granule is free address space. The caller writes the file's bytes over the zeroed pages.
    pub fn map_view(&mut self, pages: &mut dyn GuestPages, size: u64, protect: u32) -> Result<u64, Unimplemented> {
        let n = up(size, PAGE);
        let base = self.free_range(n)?;
        self.reservations
            .insert(base, Reservation::new(base, n, protect, MEM_MAPPED));
        self.commit(pages, base, base, n, protect);
        Ok(base)
    }

    /// `place_image` (268-280): where a DLL loaded at run time goes (Sprint 049), its preferred base when that lies
    /// free in the arena, else the lowest free 64 KB-aligned range, relocated there by the caller. The image is then
    /// one of `images`; `snapshot_image` records its writable pages once written.
    pub fn place_image(&mut self, pe: PeImage) -> Result<u64, Unimplemented> {
        let size = up(pe.size_of_image as u64, PAGE);
        let mut base = pe.image_base;
        let free = self.in_arena(base, size)
            && base.is_multiple_of(GRANULARITY)
            && self
                .reservations
                .values()
                .map(|r| (r.base, r.size))
                .chain(self.images.iter().map(|(b, p)| (*b, p.size_of_image as u64)))
                .all(|(b, n)| b + n <= base || base + size <= b);
        if !free {
            base = self.free_range(size)?;
        }
        self.images.push((base, pe));
        Ok(base)
    }

    fn writable_pages(base: u64, pe: &PeImage) -> Vec<u64> {
        let mut out = Vec::new();
        for s in &pe.sections {
            if s.characteristics & SCN_MEM_WRITE != 0 {
                let start = base + s.virtual_address as u64;
                let end = start + up((s.virtual_size as u64).max(1), PAGE);
                out.extend((start..end).step_by(PAGE as usize));
            }
        }
        out
    }

    /// `snapshot_image` (282-288).
    pub fn snapshot_image(&mut self, pages: &dyn GuestPages, base: u64, pe: &PeImage) {
        for va in Self::writable_pages(base, pe) {
            self.pristine.insert(va, pages.read(va, PAGE));
        }
    }

    /// `snapshot` (314-322): every writable image page as loaded, for the copy-on-write test of `image_page_protect`.
    pub fn snapshot(&mut self, pages: &dyn GuestPages) {
        let all: Vec<u64> = self
            .images
            .iter()
            .flat_map(|(base, pe)| Self::writable_pages(*base, pe))
            .collect();
        for va in all {
            self.pristine.insert(va, pages.read(va, PAGE));
        }
    }

    /// `_heap_query` (290-300): the process heap as contracts/windows.memory.heap_query records it (Sprint 049), one
    /// MEM_PRIVATE reservation whose base is the heap's handle, read-write, committed up to the pages heap.c has used
    /// and reserved after; BaseAddress is the queried page.
    fn heap_query(&self, pages: &dyn GuestPages, page: u64) -> Info {
        let (base, size) = self.heap.unwrap();
        let next = u64::from_le_bytes(pages.read(base + 8, 8).try_into().unwrap()); // heap.c's `next`
        let used = if next == 0 { base + PAGE } else { next };
        let committed = up(used.max(base + PAGE), PAGE);
        if page < committed {
            return Info {
                base: page,
                allocation_base: base,
                allocation_protect: PAGE_READWRITE,
                region_size: committed - page,
                state: MEM_COMMIT,
                protect: PAGE_READWRITE,
                kind: MEM_PRIVATE,
            };
        }
        Info {
            base: page,
            allocation_base: base,
            allocation_protect: PAGE_READWRITE,
            region_size: base + size - page,
            state: MEM_RESERVE,
            protect: 0,
            kind: MEM_PRIVATE,
        }
    }

    /// `_stack_query` (302-312): the thread's stack as contracts/windows.memory.thread_stack records it (Sprint 049),
    /// one MEM_PRIVATE reservation from DeallocationStack, read-write: reserved below StackLimit - 3 pages, those 3
    /// pages committed with PAGE_GUARD, and committed read-write from StackLimit to StackBase; BaseAddress is the
    /// queried page.
    fn stack_query(&self, page: u64) -> Info {
        let (dealloc, limit, base) = self.stack.unwrap();
        let guard = limit - STACK_GUARD_PAGES * PAGE;
        let (end, state, protect) = if page < guard {
            (guard, MEM_RESERVE, 0)
        } else if page < limit {
            (limit, MEM_COMMIT, PAGE_READWRITE | PAGE_GUARD)
        } else {
            (base, MEM_COMMIT, PAGE_READWRITE)
        };
        Info {
            base: page,
            allocation_base: dealloc,
            allocation_protect: PAGE_READWRITE,
            region_size: end - page,
            state,
            protect,
            kind: MEM_PRIVATE,
        }
    }

    /// `_image_page_protect` (324-338): a section's protection; a writable page is PAGE_WRITECOPY until the process
    /// writes it, then PAGE_READWRITE (rows image.section._data, image.rdata_after). Written means its bytes differ
    /// from load: a write of the same bytes is missed, and relocation fixups, which Windows's loader writes, count as
    /// load.
    fn image_page_protect(&self, pages: &dyn GuestPages, image: usize, va: u64) -> u32 {
        if let Some(p) = self.image_protect.get(&va) {
            return *p;
        }
        let (base, pe) = &self.images[image];
        for s in &pe.sections {
            let start = base + s.virtual_address as u64;
            if start <= va && va < start + up((s.virtual_size as u64).max(1), PAGE) {
                let p = section_protect(s.characteristics);
                if p == PAGE_READWRITE
                    && let Some(load) = self.pristine.get(&va)
                {
                    return if pages.read(va, PAGE) != *load {
                        PAGE_READWRITE
                    } else {
                        PAGE_WRITECOPY
                    };
                }
                return p;
            }
        }
        IMAGE_HEADERS_PROTECT
    }
}

/// `section_protect` (346-352): the rows give .text PAGE_EXECUTE_READ, .rdata PAGE_READONLY, .data PAGE_READWRITE
/// once written (PAGE_WRITECOPY before); a writable executable section is not in them (PAGE_EXECUTE_READWRITE here).
pub fn section_protect(characteristics: u32) -> u32 {
    let (x, r, w) = (
        characteristics & SCN_MEM_EXECUTE != 0,
        characteristics & SCN_MEM_READ != 0,
        characteristics & SCN_MEM_WRITE != 0,
    );
    match (x, w, r) {
        (true, true, _) => PAGE_EXECUTE_READWRITE,
        (true, false, true) => PAGE_EXECUTE_READ,
        (true, false, false) => PAGE_EXECUTE,
        (false, true, _) => PAGE_READWRITE,
        (false, false, true) => PAGE_READONLY,
        (false, false, false) => PAGE_NOACCESS,
    }
}
