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
        let va = base + u64::from(start);
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
