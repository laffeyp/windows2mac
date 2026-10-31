// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! The translation cache's file (Sprint 254; `docs/design-translation-cache-2026-10-30.md`, B, B', D, F, card 3): an
//! image's walked blocks (`image_walk`), each lifted as the host compiles a block (`compile.rs`: until a branch or
//! `MAX_BLOCK_INSNS`, a lift refusal ending it before the refused instruction), its flags live from its successors
//! over the image's whole code (`translator::liveness`), assembled by the Rust translator at the next address of the
//! code region `CACHE_CODE_VA`. The file holds its key, the blocks, their words and marks, and the page-to-starts index
//! over every page a block's bytes touch. Nothing here runs a guest: the addresses the words embed are given.

use std::collections::{BTreeMap, BTreeSet};

use guest_runtime::layout::{CACHE_CODE_SIZE, CACHE_CODE_VA, PAGE};
use pe_image::PeImage;
use translator::lift::{Lifted, Op, lift};
use translator::lower::{MAX_BLOCK_INSNS, RuntimeCalls, assemble_block, insn_flags};

/// The image the batch run's cache holds (Sprint 254): 149,308 of its 274,837 compiles (Sprint 249).
pub const CACHED_IMAGE: &str = "UnityPlayer.dll";

const MAGIC: &[u8; 8] = b"W2MCACHE";
const VERSION: u32 = 2; // 2: each block's fix-ups (Sprint 261)

/// What a block's words embed besides the image's bytes: the dispatcher, the slice stub, the state block and the
/// runtime's functions, all fixed by a guest's setup.
#[derive(Debug, Clone, Copy)]
pub struct Exits {
    pub exit_va: u64,
    pub slice_va: u64,
    pub state_va: u64,
    pub runtime: RuntimeCalls,
}

/// One cached block, by the image's RVAs; its words and marks are `words[word..word + nwords]`, `marks[mark..mark +
/// nmarks]`, its marks' host addresses offsets into the code region.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CachedBlock {
    pub start: u32,
    pub end: u32,
    pub word: u32,
    pub nwords: u32,
    pub mark: u32,
    pub nmarks: u32,
    pub live: u32,
    pub insns: u32,
    /// its fix-ups, `fixups[fixup..fixup + nfixups]` (version 2, Sprint 261)
    pub fixup: u32,
    pub nfixups: u32,
}

/// The cache of one image.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CacheFile {
    pub key: String,
    pub base: u64,
    pub blocks: Vec<CachedBlock>,
    pub words: Vec<u32>,
    /// (host offset into the code region, x86 RVA)
    pub marks: Vec<(u32, u32)>,
    /// each page an RVA's 4 KB page, and the indices into `blocks` of every block whose bytes touch it
    pub pages: BTreeMap<u32, Vec<u32>>,
    /// each block's fix-ups, the indices in its words of a `B` or `BL` whose target leaves the block, which an install
    /// re-encodes (`branch_leaving`; version 2, Sprint 261, card 5's II)
    pub fixups: Vec<u32>,
}

/// The walked starts `make` did not make, by why: the maker's report, not part of the file.
pub type Refused = BTreeMap<String, u32>;

/// What the cached words are made from (Sprint 265), whose tree id the key hashes: the maker's crates, `guest_runtime`,
/// `pe_image`, `translator` and `image_walk`, with every crate they reach by a `path =` dependency (`a64_encoder`,
/// `x86_decoder`); this file; `specs/`, which `x86_decoder`'s build script reads; the lockfile and the toolchain. The rest
/// of `w2m_vm` changes no cached word: the addresses it places that the words embed are hashed apart, as `Exits`.
/// `tests/cache_file.rs` holds the crates against their manifests.
pub const KEY_INPUTS: [&str; 10] = [
    "crates/guest_runtime",
    "crates/pe_image",
    "crates/translator",
    "crates/image_walk",
    "crates/a64_encoder",
    "crates/x86_decoder",
    "crates/w2m_vm/src/cache_file.rs",
    "specs",
    "Cargo.lock",
    "rust-toolchain.toml",
];

/// The key: the image, its base, the tree of `KEY_INPUTS`, and the addresses the words embed.
pub fn key(image: &[u8], base: u64, translator_tree: &str, exits: &Exits) -> String {
    let text = format!(
        "image {}\nbase {base:x}\ntranslator {translator_tree}\nexit {:x}\nslice {:x}\nstate {:x}\nruntime {:?}\n",
        guest_runtime::sha256::hex(image),
        exits.exit_va,
        exits.slice_va,
        exits.state_va,
        exits.runtime
    );
    guest_runtime::sha256::hex(text.as_bytes())
}

/// The instruction at `va` from the image's bytes, when `va` lies in its executable sections.
fn lifted(img: &PeImage, data: &[u8], base: u64, exec: &[(u64, u64)], va: u64) -> Option<Lifted> {
    let rva = va.checked_sub(base)?;
    if !exec.iter().any(|(a, b)| (*a..*b).contains(&rva)) {
        return None;
    }
    lift(&img.read_rva(data, rva, 15).ok()?, va).ok()
}

/// The block at `va` as the host compiles one (`compile.rs`): instructions until a branch or `MAX_BLOCK_INSNS`, one
/// that does not lift ending it before itself; with the address it ends at.
fn lift_block(at: &impl Fn(u64) -> Option<Lifted>, va: u64) -> (Vec<Lifted>, u64) {
    let mut insns: Vec<Lifted> = Vec::new();
    let mut addr = va;
    while insns.len() < MAX_BLOCK_INSNS {
        let Some(ins) = at(addr) else { break };
        addr += ins.length;
        let ends = ins.ops.iter().any(|op| matches!(op, Op::Br { .. }));
        insns.push(ins);
        if ends {
            break;
        }
    }
    (insns, addr)
}

/// A word `relocate` cannot move: its index in the block and its kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Unmovable {
    pub word: usize,
    pub kind: &'static str,
}

/// A block's words moved from `from` to `to` (Sprint 256): a `B` or `BL` whose target lies outside the block keeps its
/// target, its offset re-encoded; a branch inside the block keeps its offset. Err names the first word of another kind
/// whose meaning depends on the block's address (a conditional or compare branch, an `ADR`/`ADRP`, a literal load
/// reaching outside the block), which no re-encoding here moves.
/// The target of word `k` of a block of `len` words assembled at `from`, when the word is a `B` or `BL` whose target
/// leaves the block: the one rule `relocate` re-encodes by and `fixups` lists by.
pub fn branch_leaving(w: u32, k: usize, len: usize, from: u64) -> Option<u64> {
    if w & 0x7C00_0000 != 0x1400_0000 {
        return None;
    }
    let offset = (i64::from(w & 0x03FF_FFFF) << 38) >> 36; // imm26, sign-extended, times 4
    let target = (from + 4 * k as u64).wrapping_add(offset as u64);
    (!(from..from + 4 * len as u64).contains(&target)).then_some(target)
}

/// The indices of a block's words that `relocate` re-encodes (version 2's fix-ups).
pub fn fixups(words: &[u32], from: u64) -> Vec<u32> {
    (0..words.len())
        .filter(|&k| branch_leaving(words[k], k, words.len(), from).is_some())
        .map(|k| k as u32)
        .collect()
}

pub fn relocate(words: &[u32], from: u64, to: u64) -> Result<Vec<u32>, Unmovable> {
    let end = from + 4 * words.len() as u64;
    let outside = |target: u64| !(from..end).contains(&target);
    let mut out = words.to_vec();
    for (k, w) in words.iter().enumerate() {
        let pc = from + 4 * k as u64;
        let field = |bits: u32, shift: u32| -> i64 {
            let v = (w >> shift) & ((1 << bits) - 1);
            (i64::from(v) << (64 - bits)) >> (64 - bits)
        };
        if w & 0x7C00_0000 == 0x1400_0000 {
            if let Some(target) = branch_leaving(*w, k, words.len(), from) {
                let off = (target.wrapping_sub(to + 4 * k as u64) as i64) / 4;
                if !(-(1 << 25)..(1 << 25)).contains(&off) {
                    return Err(Unmovable {
                        word: k,
                        kind: "a branch past its reach",
                    });
                }
                out[k] = (w & 0xFC00_0000) | (off as u32 & 0x03FF_FFFF);
            }
            continue;
        }
        let reach = if w & 0xFF00_0010 == 0x5400_0000 || w & 0x7E00_0000 == 0x3400_0000 {
            Some((
                "a conditional or compare branch",
                pc.wrapping_add((field(19, 5) * 4) as u64),
            ))
        } else if w & 0x7E00_0000 == 0x3600_0000 {
            Some(("a test branch", pc.wrapping_add((field(14, 5) * 4) as u64)))
        } else if w & 0x1F00_0000 == 0x1000_0000 {
            Some(("an ADR or ADRP", 0))
        } else if w & 0x3B00_0000 == 0x1800_0000 {
            Some(("a literal load", pc.wrapping_add((field(19, 5) * 4) as u64)))
        } else {
            None
        };
        if let Some((kind, target)) = reach
            && (kind == "an ADR or ADRP" || outside(target))
        {
            return Err(Unmovable { word: k, kind });
        }
    }
    Ok(out)
}

/// The relocation measured on a cache (Sprint 256): each block's words, made at the code region, relocated to `to`
/// and held to the block assembled there; the blocks equal, those the relocation refuses by why, those that differ.
pub fn relocation_census(
    img: &PeImage,
    data: &[u8],
    cache: &CacheFile,
    exits: &Exits,
    to: u64,
) -> (u64, BTreeMap<String, u64>, Vec<u32>) {
    let exec: Vec<(u64, u64)> = img
        .sections
        .iter()
        .filter(|s| s.characteristics & 0x2000_0000 != 0)
        .map(|s| {
            (
                u64::from(s.virtual_address),
                u64::from(s.virtual_address) + u64::from(s.virtual_size),
            )
        })
        .collect();
    let at = |va: u64| lifted(img, data, cache.base, &exec, va);
    let (mut equal, mut refused, mut differ) = (0, BTreeMap::new(), Vec::new());
    for b in &cache.blocks {
        let words = &cache.words[b.word as usize..(b.word + b.nwords) as usize];
        let from = CACHE_CODE_VA + 4 * u64::from(b.word);
        let (insns, _) = lift_block(&at, cache.base + u64::from(b.start));
        let Ok((there, _)) = assemble_block(
            &insns,
            to,
            exits.exit_va,
            exits.slice_va,
            exits.state_va,
            &exits.runtime,
            b.live,
        ) else {
            *refused.entry("the second assembly fails".to_string()).or_default() += 1;
            continue;
        };
        match relocate(words, from, to) {
            Ok(moved) if moved == there => equal += 1,
            Ok(_) => differ.push(b.start),
            Err(why) => {
                *refused.entry(why.kind.to_string()).or_default() += 1;
            }
        }
    }
    (equal, refused, differ)
}

/// The cache of `img` loaded at `base`, from its walk's block starts, until the code region is full.
pub fn make(img: &PeImage, data: &[u8], base: u64, exits: &Exits, key: String) -> (CacheFile, Refused) {
    let walk = image_walk::walk(img, data);
    let exec: Vec<(u64, u64)> = img
        .sections
        .iter()
        .filter(|s| s.characteristics & 0x2000_0000 != 0)
        .map(|s| {
            (
                u64::from(s.virtual_address),
                u64::from(s.virtual_address) + u64::from(s.virtual_size),
            )
        })
        .collect();
    let at = |va: u64| lifted(img, data, base, &exec, va);
    let mut refused = Refused::new();
    let mut refuse = |why: &str| *refused.entry(why.to_string()).or_default() += 1;
    let mut blocks = Vec::new();
    let (mut words, mut marks, mut fix) = (Vec::new(), Vec::new(), Vec::new());
    for &start in &walk.block_starts {
        let (insns, addr) = lift_block(&at, base + u64::from(start));
        if insns.is_empty() {
            refuse("the first instruction does not lift");
            continue;
        }
        let mut code = |a: u64| at(a);
        let Ok(live) = translator::liveness::live_out(insns.last().unwrap(), addr, &mut code) else {
            refuse("a successor's flags are not named");
            continue;
        };
        let offset = 4 * words.len() as u64;
        let Ok((w, m)) = assemble_block(
            &insns,
            CACHE_CODE_VA + offset,
            exits.exit_va,
            exits.slice_va,
            exits.state_va,
            &exits.runtime,
            live,
        ) else {
            refuse("the translator refuses it");
            continue;
        };
        if offset + 4 * w.len() as u64 > CACHE_CODE_SIZE {
            refuse("the code region is full");
            continue;
        }
        let block_fixups = fixups(&w, CACHE_CODE_VA + offset);
        blocks.push(CachedBlock {
            start,
            end: (addr - base) as u32,
            word: words.len() as u32,
            nwords: w.len() as u32,
            mark: marks.len() as u32,
            nmarks: m.len() as u32,
            live,
            insns: insns.len() as u32,
            fixup: fix.len() as u32,
            nfixups: block_fixups.len() as u32,
        });
        fix.extend(block_fixups);
        words.extend(w);
        marks.extend(m.iter().map(|(h, x)| ((h - CACHE_CODE_VA) as u32, (x - base) as u32)));
    }
    let mut pages: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
    for (i, b) in blocks.iter().enumerate() {
        let mut page = u64::from(b.start) & !(PAGE - 1);
        while page < u64::from(b.end) {
            pages.entry(page as u32).or_default().push(i as u32);
            page += PAGE;
        }
    }
    let cache = CacheFile {
        key,
        base,
        blocks,
        words,
        marks,
        pages,
        fixups: fix,
    };
    (cache, refused)
}

/// The blocks whose live flags miss a flag a direct successor's first instruction reads (the design's card 3 check,
/// reviewer-3's Low 1 at 251): each block's taken and fall-through successors, lifted from the image, their reads
/// held within the block's live flags. Empty when every block covers its successors.
pub fn uncovered_reads(img: &PeImage, data: &[u8], cache: &CacheFile) -> Vec<u32> {
    let exec: Vec<(u64, u64)> = img
        .sections
        .iter()
        .filter(|s| s.characteristics & 0x2000_0000 != 0)
        .map(|s| {
            (
                u64::from(s.virtual_address),
                u64::from(s.virtual_address) + u64::from(s.virtual_size),
            )
        })
        .collect();
    let at = |va: u64| lifted(img, data, cache.base, &exec, va);
    let mut wrong = Vec::new();
    for b in &cache.blocks {
        let end = cache.base + u64::from(b.end);
        // the block's last instruction: lift the block again to its end
        let mut last = None;
        let mut a = cache.base + u64::from(b.start);
        for _ in 0..b.insns {
            let Some(ins) = at(a) else { break };
            a += ins.length;
            last = Some(ins);
        }
        let Some(last) = last else { continue };
        let br = last.ops.iter().find_map(|op| match op {
            Op::Br { cond, t } => Some((*cond, *t)),
            _ => None,
        });
        let mut successors = BTreeSet::new();
        match br {
            None => {
                successors.insert(end);
            }
            Some((cond, t)) => {
                if let Some(target) = last.ops.iter().find_map(|op| match op {
                    Op::Imm { t: it, value } if *it == t => Some(*value),
                    _ => None,
                }) {
                    successors.insert(target);
                }
                if cond != "always" {
                    successors.insert(end);
                }
            }
        }
        for s in successors {
            if let Some(ins) = at(s)
                && let Ok((read, _)) = insn_flags(&ins, None)
                && read & !b.live != 0
            {
                wrong.push(b.start);
            }
        }
    }
    wrong
}

fn put(out: &mut Vec<u8>, v: u32) {
    out.extend_from_slice(&v.to_le_bytes());
}

/// Where a file's tables lie in its bytes (Sprint 263, card 5's I): the image's base, the block count, and the offsets
/// of the block table (`BLOCK_BYTES` an entry), the words, the marks and the fix-ups; what the installer inside the VM
/// reads the file by.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sections {
    pub base: u64,
    pub nblocks: u64,
    pub blocks: u64,
    pub words: u64,
    pub marks: u64,
    pub fixups: u64,
}

/// A block's entry in the file: ten `u32`s.
pub const BLOCK_BYTES: u64 = 40;

/// The sections of a file's bytes, `to_bytes`' layout read without its tables; Err when the bytes are not a version 2
/// file whose tables end where it does.
pub fn sections(b: &[u8]) -> Result<Sections, String> {
    let u32_at = |at: u64| -> Result<u64, String> {
        b.get(at as usize..at as usize + 4)
            .map(|s| u64::from(u32::from_le_bytes(s.try_into().unwrap())))
            .ok_or_else(|| format!("the file ends before byte {at}"))
    };
    if b.get(..8) != Some(MAGIC.as_slice()) || u32_at(8)? != u64::from(VERSION) {
        return Err(format!("not a version {VERSION} w2m translation cache"));
    }
    let base = u64::from_le_bytes(b.get(76..84).ok_or("the file ends in its header")?.try_into().unwrap());
    let n: Vec<u64> = (0..5).map(|k| u32_at(84 + 4 * k)).collect::<Result<_, _>>()?;
    let blocks = 104;
    let words = blocks + BLOCK_BYTES * n[0];
    let marks = words + 4 * n[1];
    let mut at = marks + 8 * n[2];
    for _ in 0..n[3] {
        at += 8 + 4 * u32_at(at + 4)?;
    }
    if at + 4 * n[4] != b.len() as u64 {
        return Err(format!(
            "the fix-ups end at byte {}, the file at {}",
            at + 4 * n[4],
            b.len()
        ));
    }
    Ok(Sections {
        base,
        nblocks: n[0],
        blocks,
        words,
        marks,
        fixups: at,
    })
}

/// The file's bytes: the magic, the version, the key, the base, then the counts and the tables, little-endian.
pub fn to_bytes(c: &CacheFile) -> Vec<u8> {
    // the header holds a sha256 in hex, 64 bytes, which `from_bytes` and `sections` read by that width
    assert_eq!(c.key.len(), 64, "a cache key of {} bytes", c.key.len());
    let mut out = MAGIC.to_vec();
    put(&mut out, VERSION);
    out.extend_from_slice(c.key.as_bytes());
    out.extend_from_slice(&c.base.to_le_bytes());
    for n in [
        c.blocks.len(),
        c.words.len(),
        c.marks.len(),
        c.pages.len(),
        c.fixups.len(),
    ] {
        put(&mut out, n as u32);
    }
    for b in &c.blocks {
        for v in [
            b.start, b.end, b.word, b.nwords, b.mark, b.nmarks, b.live, b.insns, b.fixup, b.nfixups,
        ] {
            put(&mut out, v);
        }
    }
    for w in &c.words {
        put(&mut out, *w);
    }
    for (h, x) in &c.marks {
        put(&mut out, *h);
        put(&mut out, *x);
    }
    for (page, list) in &c.pages {
        put(&mut out, *page);
        put(&mut out, list.len() as u32);
        for i in list {
            put(&mut out, *i);
        }
    }
    for k in &c.fixups {
        put(&mut out, *k);
    }
    out
}

/// The file read back; Err names where it is short or not a cache.
pub fn from_bytes(b: &[u8]) -> Result<CacheFile, String> {
    let mut at = 0usize;
    let mut take = |n: usize| -> Result<&[u8], String> {
        let s = b
            .get(at..at + n)
            .ok_or_else(|| format!("the file ends at byte {at}, {n} more wanted"))?;
        at += n;
        Ok(s)
    };
    if take(8)? != MAGIC {
        return Err("not a w2m translation cache".into());
    }
    let u32_of = |s: &[u8]| u32::from_le_bytes(s.try_into().unwrap());
    let version = u32_of(take(4)?);
    if version != VERSION {
        return Err(format!("version {version}, where {VERSION} is read"));
    }
    let key = String::from_utf8(take(64)?.to_vec()).map_err(|e| e.to_string())?;
    let base = u64::from_le_bytes(take(8)?.try_into().unwrap());
    let mut n = [0usize; 5];
    for x in &mut n {
        *x = u32_of(take(4)?) as usize;
    }
    let mut c = CacheFile {
        key,
        base,
        ..CacheFile::default()
    };
    for _ in 0..n[0] {
        let mut v = [0u32; 10];
        for x in &mut v {
            *x = u32_of(take(4)?);
        }
        c.blocks.push(CachedBlock {
            start: v[0],
            end: v[1],
            word: v[2],
            nwords: v[3],
            mark: v[4],
            nmarks: v[5],
            live: v[6],
            insns: v[7],
            fixup: v[8],
            nfixups: v[9],
        });
    }
    for _ in 0..n[1] {
        c.words.push(u32_of(take(4)?));
    }
    for _ in 0..n[2] {
        let h = u32_of(take(4)?);
        c.marks.push((h, u32_of(take(4)?)));
    }
    for _ in 0..n[3] {
        let page = u32_of(take(4)?);
        let count = u32_of(take(4)?) as usize;
        let mut list = Vec::with_capacity(count);
        for _ in 0..count {
            list.push(u32_of(take(4)?));
        }
        c.pages.insert(page, list);
    }
    for _ in 0..n[4] {
        c.fixups.push(u32_of(take(4)?));
    }
    Ok(c)
}

#[cfg(test)]
mod tests {
    use super::relocate;

    /// Sprint 256, clang's words (`--target=aarch64-linux-gnu`, 2026-10-30): `b .+0x100` 14000040, `b .-4` 17ffffff
    /// (back to the block's first word), `b.eq .+0x200` 54001000, `adr x0, .+8` 10000040. Moved from 0x1000 to 0x5000,
    /// a `B` leaving the block is re-encoded for its target, one inside it kept; a conditional branch leaving it and an
    /// `ADR` are refused by kind.
    #[test]
    fn a_branch_out_of_the_block_is_re_encoded_and_other_kinds_refused() {
        assert_eq!(
            relocate(&[0x1400_0040, 0x17FF_FFFF], 0x1000, 0x5000),
            Ok(vec![0x17FF_F040, 0x17FF_FFFF])
        );
        assert_eq!(
            relocate(&[0x5400_1000], 0x1000, 0x5000).unwrap_err().kind,
            "a conditional or compare branch"
        );
        assert_eq!(
            relocate(&[0x1000_0040], 0x1000, 0x5000).unwrap_err().kind,
            "an ADR or ADRP"
        );
    }
}
