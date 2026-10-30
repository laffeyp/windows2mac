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

const MAGIC: &[u8; 8] = b"W2MCACHE";
const VERSION: u32 = 1;

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
}

/// The walked starts `make` did not make, by why: the maker's report, not part of the file.
pub type Refused = BTreeMap<String, u32>;

/// The key: the image, its base, the translator's tree, and the addresses the words embed.
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
            let target = pc.wrapping_add((field(26, 0) * 4) as u64);
            if outside(target) {
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
    let (mut words, mut marks) = (Vec::new(), Vec::new());
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
        blocks.push(CachedBlock {
            start,
            end: (addr - base) as u32,
            word: words.len() as u32,
            nwords: w.len() as u32,
            mark: marks.len() as u32,
            nmarks: m.len() as u32,
            live,
            insns: insns.len() as u32,
        });
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

/// The file's bytes: the magic, the version, the key, the base, then the counts and the tables, little-endian.
pub fn to_bytes(c: &CacheFile) -> Vec<u8> {
    let mut out = MAGIC.to_vec();
    put(&mut out, VERSION);
    out.extend_from_slice(c.key.as_bytes());
    out.extend_from_slice(&c.base.to_le_bytes());
    for n in [c.blocks.len(), c.words.len(), c.marks.len(), c.pages.len()] {
        put(&mut out, n as u32);
    }
    for b in &c.blocks {
        for v in [b.start, b.end, b.word, b.nwords, b.mark, b.nmarks, b.live, b.insns] {
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
    let mut n = [0usize; 4];
    for x in &mut n {
        *x = u32_of(take(4)?) as usize;
    }
    let mut c = CacheFile {
        key,
        base,
        ..CacheFile::default()
    };
    for _ in 0..n[0] {
        let mut v = [0u32; 8];
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
