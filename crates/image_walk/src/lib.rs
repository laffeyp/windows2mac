// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! A PE32+ image's code found statically (Sprint 253; `docs/design-translation-cache-2026-10-30.md`, choice A, card
//! 2), by the frozen census's method (`scripts/class_census.py` 52-186 and `src/w2m/x86ref.py` 29-147 at
//! `python-final`) with w2m's own decoder where the census used Capstone's: a recursive walk from the image's code
//! targets, MSVC switch tables walked until no new one appears, and a sweep of each `.pdata` function. Beside the
//! instructions it keeps the block starts the translation cache compiles from: every root, every branch and table
//! target, and every fall-through after a conditional branch or a call.

use std::collections::{BTreeMap, BTreeSet};

use pe_image::PeImage;
use x86_decoder::{Decoded, decode};

/// IMAGE_SCN_MEM_EXECUTE.
const SCN_MEM_EXECUTE: u32 = 0x2000_0000;
/// x86ref 40: an instruction's longest encoding.
const MAX_INSN: u64 = 15;
/// x86ref 35-39: imports documented to end the calling thread or process; the bytes after a call to one are not
/// reached through it.
pub const NORETURN: [&str; 14] = [
    "ExitProcess",
    "ExitThread",
    "FreeLibraryAndExitThread",
    "RaiseFailFastException",
    "RtlExitUserThread",
    "RtlExitUserProcess",
    "exit",
    "_exit",
    "_Exit",
    "quick_exit",
    "abort",
    "_invalid_parameter_noinfo_noreturn",
    "_invoke_watson",
    "_CxxThrowException",
];
/// x86ref 29 (`ret`, `retf`, `jmp`, `int3`, `ud2`, `hlt`) by w2m's classes: no fall-through.
const WALK_STOPS: [&str; 6] = ["RET_NEAR", "RET_FAR", "JMP", "INT3", "UD2", "HLT"];
/// class_census 93: the sweep's stops.
const SWEEP_STOPS: [&str; 5] = ["RET_NEAR", "JMP", "INT3", "UD2", "HLT"];
/// Capstone's jump group with its call group, by w2m's classes: a branch whose target the walk follows.
const CONDITIONAL: [&str; 19] = [
    "JB", "JBE", "JL", "JLE", "JNB", "JNBE", "JNL", "JNLE", "JNO", "JNP", "JNS", "JNZ", "JO", "JP", "JRCXZ", "JS",
    "JZ", "LOOP", "LOOPE",
];
const LOOPNE: &str = "LOOPNE";

/// What the walk found, by RVA.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Walk {
    /// each instruction's length
    pub insns: BTreeMap<u32, u8>,
    /// the instructions only the sweep found
    pub swept_only: BTreeSet<u32>,
    /// each switch table's entries
    pub tables: BTreeMap<u32, Vec<u32>>,
    /// where a block starts
    pub block_starts: BTreeSet<u32>,
    /// where the walk met bytes the decoder refuses
    pub undecodable: BTreeMap<u32, String>,
}

fn executable(img: &PeImage) -> Vec<(u64, u64)> {
    img.sections
        .iter()
        .filter(|s| s.characteristics & SCN_MEM_EXECUTE != 0)
        .map(|s| {
            (
                u64::from(s.virtual_address),
                u64::from(s.virtual_address) + u64::from(s.virtual_size),
            )
        })
        .collect()
}

fn functions(img: &PeImage) -> Vec<(u64, u64)> {
    let mut f: Vec<(u64, u64)> = img
        .function_starts
        .iter()
        .zip(&img.function_ends)
        .map(|(s, e)| (u64::from(*s), u64::from(*e)))
        .collect();
    f.sort_unstable();
    f
}

/// The `.pdata` function holding `rva`, if any.
fn function_at(functions: &[(u64, u64)], rva: u64) -> Option<(u64, u64)> {
    let i = functions.partition_point(|(s, _)| *s <= rva);
    (i > 0 && rva < functions[i - 1].1).then(|| functions[i - 1])
}

/// The immediate, sign-extended.
fn immediate(d: &Decoded) -> Option<i64> {
    let b = &d.fields.imm;
    match b.len() {
        1 => Some(i64::from(b[0] as i8)),
        2 => Some(i64::from(i16::from_le_bytes([b[0], b[1]]))),
        4 => Some(i64::from(i32::from_le_bytes([b[0], b[1], b[2], b[3]]))),
        _ => None,
    }
}

/// `[rip + disp]`: ModRM mod 00, r/m 101, no SIB, in 64-bit addressing.
fn rip_relative(d: &Decoded) -> bool {
    d.fields.asize == 64 && d.fields.sib.is_none() && d.fields.modrm.is_some_and(|m| m >> 6 == 0 && m & 7 == 5)
}

/// The import table slot a branch reads its target from, if it names one outright.
fn slot(d: &Decoded, next: u64) -> Option<u64> {
    rip_relative(d).then(|| next.wrapping_add(d.fields.disp as u64))
}

/// One recursive walk from `roots`: (instructions, block starts, undecodable).
fn walk_from(img: &PeImage, data: &[u8], roots: &[u64]) -> (BTreeMap<u32, u8>, BTreeSet<u32>, BTreeMap<u32, String>) {
    let ranges = executable(img);
    let inside = functions(img);
    let exec = |rva: u64| ranges.iter().any(|(a, b)| (*a..*b).contains(&rva));
    let interior = |rva: u64| function_at(&inside, rva).is_some_and(|(s, _)| s < rva);
    let mut insns = BTreeMap::new();
    let mut starts = BTreeSet::new();
    let mut undecodable = BTreeMap::new();
    let mut work: Vec<u64> = roots.iter().copied().filter(|r| exec(*r)).collect();
    starts.extend(work.iter().map(|r| *r as u32));
    while let Some(mut rva) = work.pop() {
        while exec(rva) && !insns.contains_key(&(rva as u32)) {
            let Ok(raw) = img.read_rva(data, rva, MAX_INSN) else {
                break;
            };
            let d = match decode(&raw, None) {
                Ok(d) => d,
                Err(e) => {
                    undecodable.insert(rva as u32, format!("{e:?}"));
                    break;
                }
            };
            insns.insert(rva as u32, d.length as u8);
            let next = rva + d.length as u64;
            let class = d.iclass.as_str();
            // a code address taken by `lea`, not inside a `.pdata` function, is a root (x86ref 122-126)
            if class == "LEA" && rip_relative(&d) {
                let t = next.wrapping_add(d.fields.disp as u64);
                if exec(t) && !insns.contains_key(&(t as u32)) && !interior(t) {
                    work.push(t);
                    starts.insert(t as u32);
                }
            }
            let call = class == "CALL_NEAR";
            let branch = call || class == "JMP" || class == LOOPNE || CONDITIONAL.contains(&class);
            if branch {
                if let Some(imm) = immediate(&d).filter(|_| d.fields.modrm.is_none()) {
                    let t = next.wrapping_add(imm as u64);
                    work.push(t);
                    starts.insert(t as u32);
                } else if let Some(s) = slot(&d, next).filter(|s| img.iat.contains_key(&(*s as u32)))
                    && call
                    && NORETURN.contains(&img.iat[&(s as u32)].rsplit('!').next().unwrap_or(""))
                {
                    break;
                }
                if call || class == LOOPNE || CONDITIONAL.contains(&class) {
                    starts.insert(next as u32);
                }
            }
            let fastfail = class == "INT" && d.fields.imm.first() == Some(&0x29);
            if WALK_STOPS.contains(&class) || fastfail {
                break;
            }
            rva = next;
        }
    }
    (insns, starts, undecodable)
}

/// class_census 107-128: each switch table a dispatch load names, its entries read while each is an RVA inside the
/// load's own `.pdata` function. The load: `mov`/`movsxd` of a dword at `[base + index*4 + table]`.
pub fn jump_tables(img: &PeImage, data: &[u8], insns: &BTreeMap<u32, u8>) -> BTreeMap<u32, Vec<u32>> {
    let functions = functions(img);
    let mut tables = BTreeMap::new();
    for (&rva, &len) in insns {
        let Ok(raw) = img.read_rva(data, u64::from(rva), u64::from(len)) else {
            continue;
        };
        let Ok(d) = decode(&raw, None) else {
            continue;
        };
        let load = (d.iclass == "MOV" && d.fields.opsize == 32) || d.iclass == "MOVSXD";
        let (Some(modrm), Some(sib)) = (d.fields.modrm, d.fields.sib) else {
            continue;
        };
        let (md, base, index, scale) = (modrm >> 6, sib & 7, (sib >> 3) & 7, sib >> 6);
        let indexed = (index != 4 || d.fields.rex & 2 != 0) && scale == 2;
        let based = !(md == 0 && base == 5);
        if !load || !indexed || !based || md == 3 || d.fields.disp <= 0 {
            continue;
        }
        let Some(func) = function_at(&functions, u64::from(rva)) else {
            continue;
        };
        let table = d.fields.disp as u64;
        let mut entries = Vec::new();
        while entries.len() < 4096 {
            let Ok(w) = img.read_rva(data, table + 4 * entries.len() as u64, 4) else {
                break;
            };
            let word = u64::from(u32::from_le_bytes([w[0], w[1], w[2], w[3]]));
            if !(func.0..func.1).contains(&word) {
                break;
            }
            entries.push(word as u32);
        }
        if !entries.is_empty() {
            tables.insert(table as u32, entries);
        }
    }
    tables
}

/// class_census 150-186: a linear decode of each `.pdata` function; after a stop it resumes only at an address in
/// `known` inside the function, and it never decodes a table's words.
fn sweep(img: &PeImage, data: &[u8], known: &BTreeSet<u32>, tables: &BTreeMap<u32, Vec<u32>>) -> BTreeMap<u32, u8> {
    let spans: Vec<(u64, u64)> = tables
        .iter()
        .map(|(t, e)| (u64::from(*t), u64::from(*t) + 4 * e.len() as u64))
        .collect();
    let next_known = |after: u64, end: u64| {
        known
            .range((after as u32).saturating_add(1)..)
            .next()
            .map(|k| u64::from(*k))
            .filter(|k| *k < end)
    };
    let in_table = |rva: u64| {
        let i = spans.partition_point(|(s, _)| *s <= rva);
        (i > 0 && rva < spans[i - 1].1).then(|| spans[i - 1].1)
    };
    let mut out = BTreeMap::new();
    for (&start, &end) in img.function_starts.iter().zip(&img.function_ends) {
        let (start, end) = (u64::from(start), u64::from(end));
        let Ok(code) = img.read_rva(data, start, end - start) else {
            continue;
        };
        let mut at = Some(start);
        while let Some(a) = at.filter(|a| *a < end) {
            if let Some(table_end) = in_table(a) {
                at = next_known(table_end - 1, end);
                continue;
            }
            let from = (a - start) as usize;
            let mut window = code[from..(from + MAX_INSN as usize).min(code.len())].to_vec();
            window.resize(window.len() + MAX_INSN as usize, 0);
            let Ok(d) = decode(&window, None) else {
                break;
            };
            out.insert(a as u32, d.length as u8);
            at = Some(a + d.length as u64);
            if SWEEP_STOPS.contains(&d.iclass.as_str()) {
                at = next_known(a + d.length as u64 - 1, end);
            }
        }
    }
    out
}

/// class_census 131-147: the walk from the image's code targets, the tables' targets walked too until no new table
/// appears (four rounds at most), joined with the sweep.
pub fn walk(img: &PeImage, data: &[u8]) -> Walk {
    use pe_image::CodeTarget;
    let roots: Vec<u64> = img
        .code_targets()
        .iter()
        .map(|t| match t {
            CodeTarget::Static(r) | CodeTarget::Export(r) | CodeTarget::Callback(r) | CodeTarget::Pdata(r) => *r,
        })
        .collect();
    let (mut insns, mut starts, mut undecodable) = walk_from(img, data, &roots);
    let mut tables = jump_tables(img, data, &insns);
    for _ in 0..4 {
        let targets: BTreeSet<u32> = tables.values().flatten().copied().collect();
        if targets.iter().all(|t| insns.contains_key(t)) {
            break;
        }
        let mut more = roots.clone();
        more.extend(targets.iter().map(|t| u64::from(*t)));
        (insns, starts, undecodable) = walk_from(img, data, &more);
        tables = jump_tables(img, data, &insns);
    }
    starts.extend(tables.values().flatten().copied());
    let mut known: BTreeSet<u32> = insns.keys().copied().collect();
    known.extend(tables.values().flatten().copied());
    let swept = sweep(img, data, &known, &tables);
    let swept_only: BTreeSet<u32> = swept.keys().filter(|r| !insns.contains_key(r)).copied().collect();
    for r in &swept_only {
        insns.insert(*r, swept[r]);
    }
    Walk {
        insns,
        swept_only,
        tables,
        block_starts: starts,
        undecodable,
    }
}
