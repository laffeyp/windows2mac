// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! The PE32+ (x86-64) image reader (Sprint 137), carried from `src/w2m/pe.py`: headers, sections, imports, delay
//! imports, base relocations, exports and forwarders, TLS, the `.pdata` function table and resources, read from a
//! Windows executable's bytes as Microsoft's "PE Format" specification lays them out. Malformed input is refused with
//! a `PeError` naming the file offset, in the Python's words; nothing is guessed. Each item cites the `pe.py` lines it
//! carries.

use std::collections::BTreeMap;
use std::fmt;

/// pe.py 17-26.
pub const IMAGE_FILE_MACHINE_AMD64: u16 = 0x8664;
pub const IMAGE_FILE_MACHINE_I386: u16 = 0x14C;
pub const PE32_PLUS: u16 = 0x20B;
pub const PE32: u16 = 0x10B;
pub const SCN_MEM_EXECUTE: u32 = 0x2000_0000;
pub const SCN_MEM_READ: u32 = 0x4000_0000;
pub const SCN_MEM_WRITE: u32 = 0x8000_0000;
pub const DIR_EXPORT: usize = 0;
pub const DIR_IMPORT: usize = 1;
pub const DIR_RESOURCE: usize = 2;
pub const DIR_EXCEPTION: usize = 3;
pub const DIR_BASERELOC: usize = 5;
pub const DIR_TLS: usize = 9;
pub const DIR_DELAY: usize = 13;
pub const RT_MANIFEST: u32 = 24;
pub const REL_ABSOLUTE: u16 = 0;
pub const REL_DIR64: u16 = 10;

/// The bytes are not a well-formed PE32+ image (pe.py 29-30).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeError(pub String);

impl fmt::Display for PeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for PeError {}

fn err<T>(message: String) -> Result<T, PeError> {
    Err(PeError(message))
}

/// pe.py 33-46.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Section {
    pub name: String,
    pub virtual_address: u32,
    pub virtual_size: u32,
    pub raw_offset: u32,
    pub raw_size: u32,
    pub characteristics: u32,
}

impl Section {
    /// "r-x" and the like.
    pub fn flags(&self) -> String {
        let c = self.characteristics;
        [(SCN_MEM_READ, 'r'), (SCN_MEM_WRITE, 'w'), (SCN_MEM_EXECUTE, 'x')]
            .iter()
            .map(|(bit, ch)| if c & bit != 0 { *ch } else { '-' })
            .collect()
    }

    /// The span the loader maps: the larger of the virtual and raw sizes.
    fn extent(&self) -> u64 {
        u64::from(self.virtual_size.max(self.raw_size))
    }
}

/// A resource directory key: an ID or a name (PE Format, "The .rsrc Section").
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum ResourceKey {
    Id(u32),
    Name(String),
}

/// One resource: its type, name, language, data RVA and size (pe.py 77).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resource {
    pub rtype: ResourceKey,
    pub name: ResourceKey,
    pub language: u32,
    pub data_rva: u32,
    pub size: u32,
}

/// The TLS directory: template start, end, index (VAs), zero fill (pe.py 71).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tls {
    pub start: u64,
    pub end: u64,
    pub index: u64,
    pub zero_fill: u32,
}

/// pe.py 49-77. The Python's dicts keep insertion order; `imports` and `delay_imports` are lists of (DLL, names) in
/// the directory's order for that reason.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeImage {
    pub machine: u16,
    pub timestamp: u32,
    pub characteristics: u16,
    pub image_base: u64,
    pub entry_rva: u32,
    pub section_alignment: u32,
    pub size_of_image: u32,
    pub subsystem: u16,
    pub dll_characteristics: u16,
    pub sections: Vec<Section>,
    pub imports: Vec<(String, Vec<String>)>,
    /// type -> count, padding (type 0) included
    pub relocations: BTreeMap<u16, u64>,
    /// (RVA, type), type not ABSOLUTE
    pub relocation_sites: Vec<(u32, u16)>,
    /// bound on first call
    pub delay_imports: Vec<(String, Vec<String>)>,
    /// (name, RVA)
    pub exports: Vec<(String, u32)>,
    /// An export whose RVA lies inside the export directory is a forwarder: a string naming another DLL's function
    /// (kernel32 forwards 211 of its named exports, gdi32 294; Sprint 093); keyed by "#ordinal", and by name if named.
    pub forwarders: BTreeMap<String, String>,
    /// ordinal -> RVA, every export not forwarded
    pub export_ordinals: BTreeMap<u32, u32>,
    /// RVAs
    pub tls_callbacks: Vec<u64>,
    pub tls: Option<Tls>,
    /// IAT slot RVA -> "dll!name", delay slots included
    pub iat: BTreeMap<u32, String>,
    /// `.pdata` RUNTIME_FUNCTION BeginAddress and EndAddress, in the table's order
    pub function_starts: Vec<u32>,
    pub function_ends: Vec<u32>,
    /// SizeOfStackReserve, a new thread's default (Sprint 051), and SizeOfStackCommit
    pub stack_reserve: u64,
    pub stack_commit: u64,
    pub resources: Vec<Resource>,
}

/// A statically known entry into guest code: its discovery kind and RVA (pe.py 100-106).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodeTarget {
    Static(u64),
    Export(u64),
    Callback(u64),
    Pdata(u64),
}

impl PeImage {
    /// Up to `size` bytes of the image at `rva`, from file `data` (pe.py 79-91). Bytes past a section's raw data but
    /// inside its virtual size read as zero, as the loader maps them; the result stops at the section's end and never
    /// invents bytes beyond it, so a decoder sees a truncated instruction as such.
    pub fn read_rva(&self, data: &[u8], rva: u64, size: u64) -> Result<Vec<u8>, PeError> {
        for s in &self.sections {
            let va = u64::from(s.virtual_address);
            if va <= rva && rva < va + s.extent() {
                let delta = rva - va;
                let size = size.min(s.extent() - delta);
                let from = (u64::from(s.raw_offset) + delta) as usize;
                let to = (u64::from(s.raw_offset) + u64::from(s.raw_size).min(delta + size)) as usize;
                // a Python slice: clamped to the data, empty when reversed
                let raw = data.get(from.min(data.len())..to.min(data.len())).unwrap_or(&[]);
                let mut out = raw.to_vec();
                out.resize(size as usize, 0);
                return Ok(out);
            }
        }
        err(format!("RVA 0x{rva:x} is in no section"))
    }

    /// The bytes of a resource of type `rtype` (and `name`; any name when `None`), its first language (pe.py 93-98).
    pub fn resource(
        &self,
        data: &[u8],
        rtype: &ResourceKey,
        name: Option<&ResourceKey>,
    ) -> Result<Option<Vec<u8>>, PeError> {
        for r in &self.resources {
            if &r.rtype == rtype && name.is_none_or(|n| *n == r.name) {
                return self.read_rva(data, u64::from(r.data_rva), u64::from(r.size)).map(Some);
            }
        }
        Ok(None)
    }

    /// Statically known entries into guest code (pe.py 100-106).
    pub fn code_targets(&self) -> Vec<CodeTarget> {
        let mut out = Vec::new();
        if self.entry_rva != 0 {
            out.push(CodeTarget::Static(u64::from(self.entry_rva)));
        }
        out.extend(self.exports.iter().map(|(_, rva)| CodeTarget::Export(u64::from(*rva))));
        out.extend(self.tls_callbacks.iter().map(|rva| CodeTarget::Callback(*rva)));
        out.extend(
            self.function_starts
                .iter()
                .map(|rva| CodeTarget::Pdata(u64::from(*rva))),
        );
        out
    }
}

/// pe.py 109-128.
struct Reader<'a> {
    data: &'a [u8],
}

impl Reader<'_> {
    fn bytes(&self, off: u64, size: u64) -> Result<&[u8], PeError> {
        if off + size > self.data.len() as u64 {
            return err(format!(
                "read of {size} bytes at offset 0x{off:x} beyond file size 0x{:x}",
                self.data.len()
            ));
        }
        Ok(&self.data[off as usize..(off + size) as usize])
    }

    fn u16(&self, off: u64) -> Result<u16, PeError> {
        Ok(u16::from_le_bytes(self.bytes(off, 2)?.try_into().unwrap()))
    }

    fn u32(&self, off: u64) -> Result<u32, PeError> {
        Ok(u32::from_le_bytes(self.bytes(off, 4)?.try_into().unwrap()))
    }

    fn u64(&self, off: u64) -> Result<u64, PeError> {
        Ok(u64::from_le_bytes(self.bytes(off, 8)?.try_into().unwrap()))
    }

    /// A pointer-sized word: 8 bytes in PE32+, 4 in PE32.
    fn word(&self, off: u64, wide: bool) -> Result<u64, PeError> {
        if wide {
            self.u64(off)
        } else {
            self.u32(off).map(u64::from)
        }
    }

    fn cstr(&self, off: u64) -> Result<String, PeError> {
        const LIMIT: u64 = 4096;
        if off >= self.data.len() as u64 {
            return err(format!("string at offset 0x{off:x} beyond file size"));
        }
        let window = &self.data[off as usize..(off + LIMIT).min(self.data.len() as u64) as usize];
        let Some(end) = window.iter().position(|b| *b == 0) else {
            return err(format!("unterminated string at offset 0x{off:x}"));
        };
        let s = &window[..end];
        if !s.is_ascii() {
            // found by tests/test_fuzz.py (Sprint 027)
            return err(format!("non-ASCII string at offset 0x{off:x}"));
        }
        Ok(String::from_utf8(s.to_vec()).unwrap())
    }
}

/// pe.py 131-139.
fn rva_to_offset(sections: &[Section], rva: u64) -> Result<u64, PeError> {
    for s in sections {
        let va = u64::from(s.virtual_address);
        if va <= rva && rva < va + s.extent() {
            let delta = rva - va;
            if delta >= u64::from(s.raw_size) {
                return err(format!("RVA 0x{rva:x} lies in uninitialized data of {}", s.name));
            }
            return Ok(u64::from(s.raw_offset) + delta);
        }
    }
    err(format!("RVA 0x{rva:x} is in no section"))
}

/// A section name: trailing NULs dropped, a byte past ASCII read as U+FFFD (the Python's `errors="replace"`).
fn section_name(raw: &[u8]) -> String {
    let end = raw.iter().rposition(|b| *b != 0).map_or(0, |i| i + 1);
    raw[..end]
        .iter()
        .map(|b| if b.is_ascii() { char::from(*b) } else { '\u{FFFD}' })
        .collect()
}

/// An import-name-table walk (pe.py 199-211, 229-238): the names, and each IAT slot's "dll!name".
fn thunks(
    r: &Reader<'_>,
    sections: &[Section],
    dll: &str,
    mut thunk: u64,
    mut slot: u64,
    wide: bool,
    iat: &mut BTreeMap<u32, String>,
) -> Result<Vec<String>, PeError> {
    let (size, ordinal_bit) = if wide { (8, 63) } else { (4, 31) };
    let mut names = Vec::new();
    loop {
        let t = r.word(thunk, wide)?;
        if t == 0 {
            return Ok(names);
        }
        let name = if t >> ordinal_bit != 0 {
            format!("#{}", t & 0xFFFF)
        } else {
            r.cstr(rva_to_offset(sections, t & 0x7FFF_FFFF)? + 2)?
        };
        iat.insert(slot as u32, format!("{dll}!{name}"));
        names.push(name);
        thunk += size;
        slot += size;
    }
}

/// Parses a PE32+ x86-64 image, refusing anything malformed (pe.py 142-352). With `x86`, a PE32 x86 (i386) image is
/// read too (Sprint 085, the static census): its optional header holds BaseOfData and a 32-bit ImageBase, 32-bit
/// stack and heap sizes, the data directories at +96, and 32-bit import, delay-import and TLS entries. The loader
/// takes PE32+ only.
pub fn parse(data: &[u8], x86: bool) -> Result<PeImage, PeError> {
    let r = Reader { data };
    if data.get(..2) != Some(b"MZ") {
        return err("no MZ signature at offset 0".into());
    }
    let lfanew = u64::from(r.u32(0x3C)?);
    if data.get(lfanew as usize..lfanew as usize + 4) != Some(b"PE\0\0") {
        return err(format!("no PE signature at e_lfanew 0x{lfanew:x}"));
    }
    let coff = lfanew + 4;
    r.bytes(coff, 20)?;
    let machine = r.u16(coff)?;
    let nsec = u64::from(r.u16(coff + 2)?);
    let timestamp = r.u32(coff + 4)?;
    let opt_size = u64::from(r.u16(coff + 16)?);
    let chars = r.u16(coff + 18)?;
    let wide = machine == IMAGE_FILE_MACHINE_AMD64;
    if !wide && !(x86 && machine == IMAGE_FILE_MACHINE_I386) {
        return err(format!("machine 0x{machine:x} is not AMD64 (0x8664)"));
    }
    let opt = coff + 20;
    let magic = r.u16(opt)?;
    if magic != if wide { PE32_PLUS } else { PE32 } {
        let want = if wide { "PE32+ (0x20b)" } else { "PE32 (0x10b)" };
        return err(format!("optional header magic 0x{magic:x} is not {want}"));
    }
    let entry = r.u32(opt + 16)?;
    let image_base = if wide {
        r.u64(opt + 24)?
    } else {
        u64::from(r.u32(opt + 28)?)
    };
    r.bytes(opt + 32, 8)?;
    let sect_align = r.u32(opt + 32)?;
    r.bytes(opt + 56, 8)?;
    let size_of_image = r.u32(opt + 56)?;
    r.bytes(opt + 68, 4)?;
    let subsystem = r.u16(opt + 68)?;
    let dllchars = r.u16(opt + 70)?;
    // PE Format, Optional Header Windows-Specific Fields: four stack and heap sizes, 8 bytes each in PE32+, 4 in PE32
    let (stack_reserve, stack_commit) = if wide {
        r.bytes(opt + 72, 16)?;
        (r.u64(opt + 72)?, r.u64(opt + 80)?)
    } else {
        r.bytes(opt + 72, 8)?;
        (u64::from(r.u32(opt + 72)?), u64::from(r.u32(opt + 76)?))
    };
    let dir_at = opt + if wide { 112 } else { 96 };
    let ndirs = u64::from(r.u32(dir_at - 4)?);
    let mut dirs = Vec::new();
    for i in 0..ndirs.min(16) {
        r.bytes(dir_at + 8 * i, 8)?;
        dirs.push((u64::from(r.u32(dir_at + 8 * i)?), u64::from(r.u32(dir_at + 8 * i + 4)?)));
    }

    let mut sections = Vec::new();
    for i in 0..nsec {
        let o = opt + opt_size + 40 * i;
        let raw_name = data.get(o as usize..(o as usize + 8).min(data.len())).unwrap_or(&[]);
        sections.push(Section {
            name: section_name(raw_name),
            virtual_size: r.u32(o + 8)?,
            virtual_address: r.u32(o + 12)?,
            raw_size: r.u32(o + 16)?,
            raw_offset: r.u32(o + 20)?,
            characteristics: r.u32(o + 36)?,
        });
    }
    let d = |i: usize| dirs.get(i).copied().unwrap_or((0, 0));

    let mut imports = Vec::new();
    let mut iat = BTreeMap::new();
    let (rva, _) = d(DIR_IMPORT);
    if rva != 0 {
        let mut off = rva_to_offset(&sections, rva)?;
        loop {
            r.bytes(off, 20)?;
            let (oft, name_rva, ft) = (r.u32(off)?, r.u32(off + 12)?, r.u32(off + 16)?);
            if (oft, name_rva, ft) == (0, 0, 0) {
                break;
            }
            let dll = r.cstr(rva_to_offset(&sections, name_rva.into())?)?;
            let thunk = rva_to_offset(&sections, u64::from(if oft != 0 { oft } else { ft }))?;
            let names = thunks(&r, &sections, &dll, thunk, ft.into(), wide, &mut iat)?;
            upsert(&mut imports, dll, names);
            off += 20;
        }
    }

    // Delay-load imports (PE Format, "Delay-Load Directory Table"; descriptor field order from LLVM Object/COFF.h
    // delay_import_directory_table_entry). The loader binds a slot on its first call, so calls through it land on the
    // named function exactly as an ordinary import does.
    let mut delay = Vec::new();
    let (rva, _) = d(DIR_DELAY);
    if rva != 0 {
        let mut off = rva_to_offset(&sections, rva)?;
        loop {
            r.bytes(off, 32)?;
            let (attrs, name_rva, diat, dint) = (r.u32(off)?, r.u32(off + 4)?, r.u32(off + 12)?, r.u32(off + 16)?);
            if name_rva == 0 {
                break;
            }
            if attrs & 1 == 0 {
                // pre-VC7 descriptors hold VAs, not RVAs (delayimp.h dlattrRva)
                return err(format!(
                    "delay-load descriptor at offset 0x{off:x} uses VAs (attributes 0)"
                ));
            }
            let dll = r.cstr(rva_to_offset(&sections, name_rva.into())?)?;
            let thunk = rva_to_offset(&sections, dint.into())?;
            let names = thunks(&r, &sections, &dll, thunk, diat.into(), wide, &mut iat)?;
            upsert(&mut delay, dll, names);
            off += 32;
        }
    }

    let mut relocs: BTreeMap<u16, u64> = BTreeMap::new();
    let mut sites = Vec::new();
    let (rva, size) = d(DIR_BASERELOC);
    if rva != 0 {
        let mut off = rva_to_offset(&sections, rva)?;
        let end = off + size;
        while off < end {
            r.bytes(off, 8)?;
            let (page, block) = (r.u32(off)?, r.u32(off + 4)?);
            if block < 8 {
                return err(format!("relocation block size {block} at offset 0x{off:x}"));
            }
            for i in 0..u64::from((block - 8) / 2) {
                let e = r.u16(off + 8 + 2 * i)?;
                *relocs.entry(e >> 12).or_default() += 1;
                if e >> 12 != 0 {
                    // type 0 (ABSOLUTE) is block padding
                    sites.push((page.wrapping_add(u32::from(e & 0xFFF)), e >> 12));
                }
            }
            off += u64::from(block);
        }
    }

    let mut exports = Vec::new();
    let mut forwarders = BTreeMap::new();
    let mut export_ordinals = BTreeMap::new();
    let (rva, size) = d(DIR_EXPORT);
    if rva != 0 {
        let e = rva_to_offset(&sections, rva)?;
        r.bytes(e, 40)?;
        let ordinal_base = r.u32(e + 16)?;
        let (nfuncs, nnames) = (r.u32(e + 20)?, r.u32(e + 24)?);
        let (funcs, names_rva, ords) = (r.u32(e + 28)?, r.u32(e + 32)?, r.u32(e + 36)?);
        let mut named: BTreeMap<u32, String> = BTreeMap::new();
        for i in 0..u64::from(nnames) {
            let nrva = r.u32(rva_to_offset(&sections, names_rva.into())? + 4 * i)?;
            let ordinal = u32::from(r.u16(rva_to_offset(&sections, ords.into())? + 2 * i)?);
            let frva = r.u32(rva_to_offset(&sections, funcs.into())? + 4 * u64::from(ordinal))?;
            let name = r.cstr(rva_to_offset(&sections, nrva.into())?)?;
            named.insert(ordinal, name.clone());
            exports.push((name, frva));
        }
        for index in 0..nfuncs {
            let frva = r.u32(rva_to_offset(&sections, funcs.into())? + 4 * u64::from(index))?;
            if frva == 0 {
                continue; // an unused slot
            }
            let frva64 = u64::from(frva);
            if rva <= frva64 && frva64 < rva + size {
                // under its ordinal always, and its name if it has one (d3d11 imports dxcore's named forwarders by
                // ordinal)
                let target = r.cstr(rva_to_offset(&sections, frva64)?)?;
                forwarders.insert(format!("#{}", ordinal_base.wrapping_add(index)), target.clone());
                if let Some(name) = named.get(&index) {
                    forwarders.insert(name.clone(), target);
                }
            } else {
                export_ordinals.insert(ordinal_base.wrapping_add(index), frva);
            }
        }
    }

    let mut callbacks = Vec::new();
    let mut tls = None;
    let (rva, _) = d(DIR_TLS);
    if rva != 0 {
        let at = rva_to_offset(&sections, rva)?;
        let w = if wide { 8 } else { 4 };
        r.bytes(at, 4 * w + 4)?;
        let (start, end, index, cb_va) = (
            r.word(at, wide)?,
            r.word(at + w, wide)?,
            r.word(at + 2 * w, wide)?,
            r.word(at + 3 * w, wide)?,
        );
        tls = Some(Tls {
            start,
            end,
            index,
            zero_fill: r.u32(at + 4 * w)?,
        });
        if cb_va != 0 {
            let Some(cb_rva) = cb_va.checked_sub(image_base) else {
                return err(format!(
                    "TLS callback table at 0x{cb_va:x} below the image base 0x{image_base:x}"
                ));
            };
            let mut off = rva_to_offset(&sections, cb_rva)?;
            loop {
                let va = r.word(off, wide)?;
                if va == 0 {
                    break;
                }
                let Some(cb) = va.checked_sub(image_base) else {
                    return err(format!("TLS callback 0x{va:x} below the image base 0x{image_base:x}"));
                };
                callbacks.push(cb);
                off += w;
            }
        }
    }

    let mut starts = Vec::new();
    let mut ends = Vec::new();
    let (rva, size) = d(DIR_EXCEPTION);
    if rva != 0 && wide {
        // x86-64's RUNTIME_FUNCTION table; PE32 has none of its kind
        let off = rva_to_offset(&sections, rva)?;
        for i in 0..size / 12 {
            r.bytes(off + 12 * i, 12)?;
            starts.push(r.u32(off + 12 * i)?);
            ends.push(r.u32(off + 12 * i + 4)?);
        }
    }

    let mut resources = Vec::new();
    let (rva, _) = d(DIR_RESOURCE);
    if rva != 0 {
        let root = rva_to_offset(&sections, rva)?;
        // PE Format, "The .rsrc Section": a directory table, its named entries, then its ID entries
        let entries = |off: u64| -> Result<Vec<(ResourceKey, u32)>, PeError> {
            r.bytes(off + 12, 4)?;
            let count = u64::from(r.u16(off + 12)?) + u64::from(r.u16(off + 14)?);
            let mut out = Vec::new();
            for i in 0..count {
                r.bytes(off + 16 + 8 * i, 8)?;
                let (key, target) = (r.u32(off + 16 + 8 * i)?, r.u32(off + 20 + 8 * i)?);
                if key & 0x8000_0000 != 0 {
                    let at = root + u64::from(key & 0x7FFF_FFFF);
                    let length = u64::from(r.u16(at)?);
                    let start = at + 2;
                    if start + 2 * length > data.len() as u64 {
                        return err(format!("resource name at 0x{start:x} beyond the file"));
                    }
                    let units: Vec<u16> = data[start as usize..(start + 2 * length) as usize]
                        .as_chunks::<2>()
                        .0
                        .iter()
                        .map(|c| u16::from_le_bytes(*c))
                        .collect();
                    out.push((ResourceKey::Name(String::from_utf16_lossy(&units)), target));
                } else {
                    out.push((ResourceKey::Id(key), target));
                }
            }
            Ok(out)
        };
        for (rtype, t) in entries(root)? {
            if t & 0x8000_0000 == 0 {
                return err("a resource type entry is not a directory".into());
            }
            for (name, n) in entries(root + u64::from(t & 0x7FFF_FFFF))? {
                if n & 0x8000_0000 == 0 {
                    return err("a resource name entry is not a directory".into());
                }
                for (lang, leaf) in entries(root + u64::from(n & 0x7FFF_FFFF))? {
                    let ResourceKey::Id(language) = lang else {
                        return err("a resource language entry is not a data entry".into());
                    };
                    if leaf & 0x8000_0000 != 0 {
                        return err("a resource language entry is not a data entry".into());
                    }
                    r.bytes(root + u64::from(leaf), 8)?;
                    resources.push(Resource {
                        rtype: rtype.clone(),
                        name: name.clone(),
                        language,
                        data_rva: r.u32(root + u64::from(leaf))?,
                        size: r.u32(root + u64::from(leaf) + 4)?,
                    });
                }
            }
        }
    }

    if entry >= size_of_image {
        return err(format!("entry RVA 0x{entry:x} outside image size 0x{size_of_image:x}"));
    }
    Ok(PeImage {
        machine,
        timestamp,
        characteristics: chars,
        image_base,
        entry_rva: entry,
        section_alignment: sect_align,
        size_of_image,
        subsystem,
        dll_characteristics: dllchars,
        sections,
        imports,
        relocations: relocs,
        relocation_sites: sites,
        delay_imports: delay,
        exports,
        forwarders,
        export_ordinals,
        tls_callbacks: callbacks,
        tls,
        iat,
        function_starts: starts,
        function_ends: ends,
        stack_reserve,
        stack_commit,
        resources,
    })
}

/// A Python dict's `d[key] = value`: replaced in place when the key is there, appended when not.
fn upsert(list: &mut Vec<(String, Vec<String>)>, key: String, value: Vec<String>) {
    match list.iter_mut().find(|(k, _)| *k == key) {
        Some(entry) => entry.1 = value,
        None => list.push((key, value)),
    }
}
