// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! PE32+ images mapped into guest memory and their imports bound (Sprint 138), carried from `src/w2m/loader.py`
//! (Sprints 029, 046). Each item cites the lines it carries.
//!
//! The program's image is mapped at its preferred base; headers and each section's raw data are copied, and the rest
//! of SizeOfImage reads as zero (PE Format, Section Table). A DLL the program imports that the caller can supply is
//! mapped too: at its preferred base when that range is free, as Windows placed the loader probe's DLL on every runner
//! (`contracts/windows.loader.static_import`, row `dll.base`), else at `DLL_BASE` and up, 64 KB aligned, with its base
//! relocations applied. An import from such a DLL is bound to the DLL's export, and a missing export refuses the load,
//! as Windows refuses to start the process. Every other import slot receives a host thunk's address. DLL entry points
//! run before the program's, deepest dependency first, through a short x86 stub in the "loader" region.
//!
//! Where the Python's bytearray slice assignment would grow a region (a section, a relocation or an IAT slot past
//! SizeOfImage), this crate refuses the load instead.

use std::collections::BTreeMap;
use std::fmt;

use pe_image::{PeError, PeImage, parse};

/// loader.py 31-42.
pub const THUNK_BASE: u64 = 0x7FFE_0000_0000;
pub const DLL_BASE: u64 = 0x7FF8_0000_0000;
pub const LOADER_STUB: u64 = 0x7FFD_0000_0000;
pub const TLS_VA: u64 = 0x7FFC_0000_0000;
/// Each thread's pointer array has room for this many modules: a DLL loaded later fills its slot in place (probe run
/// 38229055834: the array does not move).
pub const TLS_SLOTS: usize = 64;
pub const DLL_PROCESS_ATTACH: u32 = 1;
pub const HEAP_VA: u64 = 0x2_0000_0000;
pub const HEAP_SIZE: u64 = 0x1000_0000;
pub const MONITOR_VA: u64 = 0x7F00_0000_0000;
/// A thunk no service serves: the run stops by name.
pub const DLL_INIT_FAILED: &str = "w2m-loader!DllMain returned FALSE";
pub const STACK_TOP: u64 = 0x0000_0001_0000_0000;
pub const STACK_SIZE: u64 = 1 << 20;

/// A guest fault the runtime stops on and reports: by default an access outside every mapped region, `code` the
/// Windows status the guest would see (loader.py 45-52).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuestFault {
    pub message: String,
    pub code: u32,
}

/// STATUS_ACCESS_VIOLATION.
pub const ACCESS_VIOLATION: u32 = 0xC000_0005;

/// loader.py 55-59.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Region {
    pub base: u64,
    pub data: Vec<u8>,
    pub name: String,
}

impl Region {
    fn end(&self) -> u64 {
        self.base + self.data.len() as u64
    }

    /// `data[off..off + n]`, refused past the region's end.
    fn slice_mut(&mut self, off: u64, n: usize, what: &str) -> Result<&mut [u8], LoadError> {
        let len = self.data.len();
        self.data.get_mut(off as usize..off as usize + n).ok_or_else(|| {
            LoadError::Load(format!(
                "{what}: {n} bytes at offset 0x{off:x} past the region's 0x{len:x}"
            ))
        })
    }
}

/// loader.py 62-95. The Python's `on_write` (the VM told of each write) belongs to the VM, which this crate does not
/// know.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Memory {
    pub regions: Vec<Region>,
}

impl Memory {
    pub fn map(&mut self, base: u64, size: u64, name: &str) -> &mut Region {
        self.regions.push(Region {
            base,
            data: vec![0; size as usize],
            name: name.to_string(),
        });
        self.regions.last_mut().unwrap()
    }

    fn find(&self, addr: u64, n: u64) -> Result<(usize, u64), GuestFault> {
        for (i, r) in self.regions.iter().enumerate() {
            if r.base <= addr && addr + n <= r.end() {
                return Ok((i, addr - r.base));
            }
        }
        Err(GuestFault {
            message: format!("access of {n} bytes at 0x{addr:x} outside mapped memory"),
            code: ACCESS_VIOLATION,
        })
    }

    pub fn read(&self, addr: u64, n: u64) -> Result<Vec<u8>, GuestFault> {
        let (i, off) = self.find(addr, n)?;
        Ok(self.regions[i].data[off as usize..(off + n) as usize].to_vec())
    }

    pub fn write(&mut self, addr: u64, data: &[u8]) -> Result<(), GuestFault> {
        let (i, off) = self.find(addr, data.len() as u64)?;
        self.regions[i].data[off as usize..off as usize + data.len()].copy_from_slice(data);
        Ok(())
    }

    /// Up to `limit` bytes from `addr`, stopping at the end of its region.
    pub fn code_at(&self, addr: u64, limit: u64) -> Result<Vec<u8>, GuestFault> {
        for r in &self.regions {
            if r.base <= addr && addr < r.end() {
                let off = (addr - r.base) as usize;
                return Ok(r.data[off..(off + limit as usize).min(r.data.len())].to_vec());
            }
        }
        Err(GuestFault {
            message: format!("instruction fetch at 0x{addr:x} outside mapped memory"),
            code: ACCESS_VIOLATION,
        })
    }

    /// loader.py 161-162.
    fn region(&mut self, base: u64) -> &mut Region {
        self.regions.iter_mut().find(|r| r.base == base).unwrap()
    }
}

/// The images cannot be loaded as Windows would load them (loader.py 98-100), or a file is not a PE image.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoadError {
    Pe(PeError),
    Load(String),
}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LoadError::Pe(e) => write!(f, "{e}"),
            LoadError::Load(m) => f.write_str(m),
        }
    }
}

impl std::error::Error for LoadError {}

impl From<PeError> for LoadError {
    fn from(e: PeError) -> Self {
        LoadError::Pe(e)
    }
}

/// loader.py 103-117.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Image {
    /// as imported ("UnityPlayer.dll"); the program's: its file name
    pub name: String,
    pub pe: PeImage,
    pub base: u64,
}

impl Image {
    pub fn entry(&self) -> Option<u64> {
        (self.pe.entry_rva != 0).then(|| self.base + u64::from(self.pe.entry_rva))
    }

    /// The export's address; a forwarded export is refused, its RVA being a string, not code (none of the games' 145
    /// images forward).
    pub fn export(&self, name: &str) -> Result<Option<u64>, LoadError> {
        if let Some(target) = self.pe.forwarders.get(name) {
            return Err(LoadError::Load(format!(
                "{}!{name} is forwarded to {target}",
                self.name
            )));
        }
        Ok(self
            .pe
            .exports
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, rva)| self.base + u64::from(*rva)))
    }
}

/// loader.py 120-131. `init_order` and `dependency_order` index `images`. `image` is `None` for a bare memory with no
/// program, as the CPU cases' runner builds (`scripts/vm_oracle.py` 183: `Loaded(None, mem, ...)`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Loaded {
    pub image: Option<PeImage>,
    pub memory: Memory,
    /// where the thread starts: the program's entry, or the loader stub
    pub entry: u64,
    /// thunk address -> "DLL!Name"
    pub thunks: BTreeMap<u64, String>,
    pub stack_top: u64,
    /// the program first, then DLLs in load order
    pub images: Vec<Image>,
    /// the program's own entry point
    pub image_entry: u64,
    /// DLLs whose entry runs, in the order it runs
    pub init_order: Vec<usize>,
    /// every DLL, dependencies first (the PEB's initialization list)
    pub dependency_order: Vec<usize>,
    /// TEB.ThreadLocalStoragePointer, set on Windows even when no image has static TLS
    pub tls_array: u64,
}

impl Loaded {
    /// A memory with no program: the dataclass's defaults (loader.py 127-131) beside the fields given, as
    /// `scripts/vm_oracle.py` 183 builds it for the CPU cases.
    pub fn bare(memory: Memory, entry: u64, stack_top: u64) -> Loaded {
        Loaded {
            image: None,
            memory,
            entry,
            thunks: BTreeMap::new(),
            stack_top,
            images: Vec::new(),
            image_entry: 0,
            init_order: Vec::new(),
            dependency_order: Vec::new(),
            tls_array: 0,
        }
    }
}

/// loader.py 134-151: headers, sections, and the base relocations for `base`.
pub fn map_image(mem: &mut Memory, data: &[u8], img: &PeImage, base: u64, name: &str) -> Result<(), LoadError> {
    let region = mem.map(base, u64::from(img.size_of_image), name);
    let headers = img
        .sections
        .iter()
        .map(|s| s.raw_offset)
        .filter(|o| *o != 0)
        .min()
        .unwrap_or(0x400) as usize;
    let headers = headers.min(data.len());
    region.slice_mut(0, headers, name)?.copy_from_slice(&data[..headers]);
    for s in &img.sections {
        let from = (s.raw_offset as usize).min(data.len());
        let to = (s.raw_offset as usize + s.raw_size as usize).min(data.len());
        let raw = &data[from..to];
        region
            .slice_mut(u64::from(s.virtual_address), raw.len(), name)?
            .copy_from_slice(raw);
    }
    let delta = base.wrapping_sub(img.image_base);
    if delta != 0 {
        for &(rva, kind) in &img.relocation_sites {
            match kind {
                // IMAGE_REL_BASED_DIR64
                10 => {
                    let at = region.slice_mut(u64::from(rva), 8, name)?;
                    let v = u64::from_le_bytes((&*at).try_into().unwrap());
                    at.copy_from_slice(&v.wrapping_add(delta).to_le_bytes());
                }
                // IMAGE_REL_BASED_HIGHLOW
                3 => {
                    let at = region.slice_mut(u64::from(rva), 4, name)?;
                    let v = u32::from_le_bytes((&*at).try_into().unwrap());
                    at.copy_from_slice(&v.wrapping_add(delta as u32).to_le_bytes());
                }
                _ => {
                    return Err(LoadError::Load(format!(
                        "{name}: base relocation type {kind} at RVA 0x{rva:x}"
                    )));
                }
            }
        }
    }
    Ok(())
}

/// loader.py 154-158: no region and no reserved range overlaps `[base, base + size)`.
fn free(mem: &Memory, base: u64, size: u64) -> bool {
    let reserved = [
        (THUNK_BASE, 1 << 32),
        (LOADER_STUB, 1 << 32),
        (TLS_VA, 1 << 32),
        (STACK_TOP - STACK_SIZE, STACK_SIZE),
        (HEAP_VA, HEAP_SIZE),
        (MONITOR_VA, 1 << 36),
    ];
    mem.regions
        .iter()
        .map(|r| (r.base, r.data.len() as u64))
        .chain(reserved)
        .all(|(b, n)| base + size <= b || b + n <= base)
}

/// `sub rsp, 0x28; mov rcx, base; mov edx, DLL_PROCESS_ATTACH; mov r8, LOADER_STUB; mov rax, target; call rax;
/// add rsp, 0x28`: a call with the shadow space and a non-NULL lpReserved.
fn attach_call(out: &mut Vec<u8>, base: u64, target: u64) {
    out.extend([0x48, 0x83, 0xEC, 0x28]);
    out.extend([0x48, 0xB9]);
    out.extend(base.to_le_bytes());
    out.push(0xBA);
    out.extend(DLL_PROCESS_ATTACH.to_le_bytes());
    out.extend([0x49, 0xB8]);
    out.extend(LOADER_STUB.to_le_bytes());
    out.extend([0x48, 0xB8]);
    out.extend(target.to_le_bytes());
    out.extend([0xFF, 0xD0]);
    out.extend([0x48, 0x83, 0xC4, 0x28]);
}

/// loader.py 165-177: each TLS callback of `image` with (base, DLL_PROCESS_ATTACH, non-NULL), before its DllMain
/// (Sprint 051, contracts/windows.thread.threads row events.start).
fn tls_calls(out: &mut Vec<u8>, image: &Image) {
    for rva in &image.pe.tls_callbacks {
        attach_call(out, image.base, image.base + rva);
    }
}

/// loader.py 180-203: for each DLL in init order, its TLS callbacks and then DllMain (a FALSE return calls `failed`);
/// the program's TLS callbacks; then a jump to the program's entry with RSP as the thread started.
pub fn stub(dlls: &[&Image], program: &Image, program_entry: u64, failed: u64) -> Vec<u8> {
    let mut out = Vec::new();
    for d in dlls {
        tls_calls(&mut out, d);
        let Some(entry) = d.entry() else { continue };
        attach_call(&mut out, d.base, entry);
        out.extend([0x85, 0xC0]); // test eax, eax
        out.extend([0x75, 0x0C]); // jnz past the failure call
        out.extend([0x48, 0xB8]); // mov rax, the failure thunk
        out.extend(failed.to_le_bytes());
        out.extend([0xFF, 0xD0]); // call rax (stops the run)
    }
    tls_calls(&mut out, program);
    out.extend([0x48, 0xB8]); // mov rax, the program's entry
    out.extend(program_entry.to_le_bytes());
    out.extend([0xFF, 0xE0]); // jmp rax
    out
}

/// Supplies a DLL's bytes by its imported name, or `None` when the DLL is not the program's to load.
pub type DllSupply<'a> = &'a dyn Fn(&str) -> Option<Vec<u8>>;

/// The DLLs a load maps: loader.py 213-236's `loaded`, `order` and `next_base`.
struct Placing<'a> {
    dlls: Option<DllSupply<'a>>,
    mem: Memory,
    images: Vec<Image>,
    loaded: BTreeMap<String, usize>,
    order: Vec<usize>,
    next_base: u64,
}

impl Placing<'_> {
    /// loader.py 217-236: depth first, so a DLL's dependencies initialize first.
    fn visit(&mut self, image: usize) -> Result<(), LoadError> {
        let names: Vec<String> = self.images[image].pe.imports.iter().map(|(d, _)| d.clone()).collect();
        for dll in names {
            let key = dll.to_lowercase();
            let Some(supply) = self.dlls else { continue };
            if self.loaded.contains_key(&key) {
                continue;
            }
            let Some(raw) = supply(&dll) else { continue };
            let pe = parse(&raw, false)?;
            let size = u64::from(pe.size_of_image);
            let base = if free(&self.mem, pe.image_base, size) {
                pe.image_base
            } else {
                let base = self.next_base;
                self.next_base = (base + size + 0xFFFF) & !0xFFFF;
                base
            };
            map_image(&mut self.mem, &raw, &pe, base, &format!("image {dll}"))?;
            self.images.push(Image { name: dll, pe, base });
            let d = self.images.len() - 1;
            self.loaded.insert(key, d);
            self.visit(d)?;
            self.order.push(d);
        }
        Ok(())
    }
}

/// loader.py 206-269: the program, and each DLL `dlls` supplies by its imported name (`None`: not the program's to
/// load).
pub fn load(data: &[u8], dlls: Option<DllSupply<'_>>, name: &str) -> Result<Loaded, LoadError> {
    let img = parse(data, false)?;
    let mut mem = Memory::default();
    map_image(&mut mem, data, &img, img.image_base, "image")?;
    let program = Image {
        name: name.to_string(),
        pe: img.clone(),
        base: img.image_base,
    };
    let mut p = Placing {
        dlls,
        mem,
        images: vec![program],
        loaded: BTreeMap::new(),
        order: Vec::new(),
        next_base: DLL_BASE,
    };
    p.visit(0)?;
    let Placing {
        mut mem,
        images,
        loaded,
        order,
        ..
    } = p;

    let mut thunks: BTreeMap<u64, String> = BTreeMap::new();
    let mut thunk = |target: &str| {
        let addr = THUNK_BASE + 16 * thunks.len() as u64;
        thunks.insert(addr, target.to_string());
        addr
    };
    for image in &images {
        for (slot, target) in &image.pe.iat {
            let (dll, function) = target.split_once('!').unwrap();
            let va = match loaded.get(&dll.to_lowercase()) {
                None => thunk(target),
                Some(&s) => images[s].export(function)?.ok_or_else(|| {
                    LoadError::Load(format!(
                        "{} imports {target}, which {} does not export",
                        image.name, images[s].name
                    ))
                })?,
            };
            mem.region(image.base)
                .slice_mut(u64::from(*slot), 8, &image.name)?
                .copy_from_slice(&va.to_le_bytes());
        }
    }
    let tls_array = static_tls(&mut mem, &images)?;
    let starting: Vec<usize> = order.iter().copied().filter(|d| images[*d].entry().is_some()).collect();
    let program_entry = images[0].entry().unwrap_or(0);
    let mut entry = program_entry;
    if !starting.is_empty() || images.iter().any(|i| !i.pe.tls_callbacks.is_empty()) {
        let with_work: Vec<&Image> = order
            .iter()
            .map(|d| &images[*d])
            .filter(|d| d.entry().is_some() || !d.pe.tls_callbacks.is_empty())
            .collect();
        let code = stub(&with_work, &images[0], program_entry, thunk(DLL_INIT_FAILED));
        let region = mem.map(LOADER_STUB, (code.len() as u64 + 0xFFF) & !0xFFF, "loader");
        region.data[..code.len()].copy_from_slice(&code);
        entry = LOADER_STUB;
    }
    mem.map(THUNK_BASE, 16 * thunks.len().max(1) as u64, "thunks");
    mem.map(STACK_TOP - STACK_SIZE, STACK_SIZE, "stack");
    Ok(Loaded {
        image: Some(img),
        memory: mem,
        entry,
        thunks,
        stack_top: STACK_TOP,
        images,
        image_entry: program_entry,
        init_order: starting,
        dependency_order: order,
        tls_array,
    })
}

/// loader.py 272-307. Index 0 is the program's, whether or not it has a TLS directory (a program without one
/// importing a DLL with one: the DLL's index is 1 on every runner, `contracts/windows.loader.static_tls`); each DLL
/// with a TLS directory gets the next index in load order, written at its AddressOfIndex. The thread's block for an
/// image copies its template, after relocation, and SizeOfZeroFill zero bytes; the program's slot is NULL when it has
/// no TLS. The array of block pointers is TEB.ThreadLocalStoragePointer, set even when no image has static TLS (probe
/// run 38229311268; Sprint 065).
fn static_tls(mem: &mut Memory, images: &[Image]) -> Result<u64, LoadError> {
    let with_tls = std::iter::once(&images[0]).chain(images[1..].iter().filter(|i| i.pe.tls.is_some()));
    let mut blocks: Vec<Option<Vec<u8>>> = vec![None];
    for (index, image) in with_tls.enumerate() {
        let Some(tls) = image.pe.tls else { continue };
        // a VA at the preferred base, as an offset in the image wherever it was placed
        let at = |va: u64| va.wrapping_sub(image.pe.image_base);
        let region = mem.region(image.base);
        region
            .slice_mut(at(tls.index), 4, &image.name)?
            .copy_from_slice(&(index as u32).to_le_bytes());
        let (start, end) = (at(tls.start) as usize, at(tls.end) as usize);
        let mut block = region
            .data
            .get(start..end)
            .ok_or_else(|| LoadError::Load(format!("{}: TLS template past the image", image.name)))?
            .to_vec();
        block.resize(block.len() + tls.zero_fill as usize, 0);
        if index == 0 {
            blocks[0] = Some(block);
        } else {
            blocks.push(Some(block));
        }
    }
    if blocks.len() > TLS_SLOTS {
        return Err(LoadError::Load(format!(
            "{} modules with static TLS: the array holds {TLS_SLOTS}",
            blocks.len()
        )));
    }
    let round = |n: usize| (n + 15) & !15;
    let array_size = 8 * TLS_SLOTS;
    let size = array_size + blocks.iter().flatten().map(|b| round(b.len())).sum::<usize>();
    let tls = mem.map(TLS_VA, (size as u64 + 0xFFF) & !0xFFF, "tls");
    let mut at = array_size;
    for (index, slot) in blocks.iter().enumerate() {
        // the program's slot, when it has no TLS, stays NULL
        let Some(block) = slot else { continue };
        tls.data[8 * index..8 * index + 8].copy_from_slice(&(TLS_VA + at as u64).to_le_bytes());
        tls.data[at..at + block.len()].copy_from_slice(block);
        at += round(block.len());
    }
    Ok(TLS_VA)
}
