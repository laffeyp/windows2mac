// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! Stage-1 translation tables: `vm_backend._write_translation_tables` (`src/w2m/vm_backend.py` 2204-2242), carried
//! line for line. Four levels in the 4 KB granule, a level-1 block (1 GB) where the map is 1 GB aligned, a level-2
//! block (2 MB) where it is 2 MB aligned, 4 KB pages otherwise. Descriptor layout: Arm ARM D8.3.

use guest_runtime::layout::{BLOCK, IPA_BASE, PAGE, TRANSLATION_TABLES};

use crate::{GIGA_BLOCK, PAGE_ATTRIBUTES};

/// The table area's size (vm_backend 132).
pub const TRANSLATION_TABLES_SIZE: u64 = 0x40000;

/// Why tables could not be written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TableError {
    /// "stage-1 table area full" (vm_backend 2217).
    Full,
}

fn read(mem: &[u8], off: u64) -> u64 {
    let o = off as usize;
    u64::from_le_bytes(mem[o..o + 8].try_into().unwrap())
}

fn write(mem: &mut [u8], off: u64, v: u64) {
    let o = off as usize;
    mem[o..o + 8].copy_from_slice(&v.to_le_bytes());
}

/// Writes tables for `(va, ipa, size)` maps into `mem` (the guest memory file, the monitor area at offset 0 and
/// `IPA_BASE`), and returns the root table's IPA and the offset of the next free table.
pub fn write_translation_tables(mem: &mut [u8], maps: &[(u64, u64, u64)]) -> Result<(u64, u64), TableError> {
    let mut nxt = TRANSLATION_TABLES + PAGE;
    let mut table_at = |mem: &mut [u8], off: u64, idx: u64| -> Result<u64, TableError> {
        let desc = read(mem, off + 8 * idx);
        if desc != 0 {
            return Ok((desc & 0x0000_FFFF_FFFF_F000) - IPA_BASE);
        }
        let new = nxt;
        nxt += PAGE;
        if nxt > TRANSLATION_TABLES + TRANSLATION_TABLES_SIZE {
            return Err(TableError::Full);
        }
        write(mem, off + 8 * idx, (IPA_BASE + new) | 0x3);
        Ok(new)
    };
    for &(va, ipa, size) in maps {
        if va.is_multiple_of(GIGA_BLOCK) && ipa.is_multiple_of(GIGA_BLOCK) && size.is_multiple_of(GIGA_BLOCK) {
            for p in (0..size).step_by(GIGA_BLOCK as usize) {
                let v = va + p;
                let off = table_at(mem, TRANSLATION_TABLES, (v >> 39) & 0x1FF)?;
                write(mem, off + 8 * ((v >> 30) & 0x1FF), (ipa + p) | PAGE_ATTRIBUTES | 0x1);
            }
            continue;
        }
        if va.is_multiple_of(BLOCK) && ipa.is_multiple_of(BLOCK) && size.is_multiple_of(BLOCK) {
            for p in (0..size).step_by(BLOCK as usize) {
                let v = va + p;
                let mut off = TRANSLATION_TABLES;
                for level in 0..2 {
                    off = table_at(mem, off, (v >> (39 - 9 * level)) & 0x1FF)?;
                }
                write(mem, off + 8 * ((v >> 21) & 0x1FF), (ipa + p) | PAGE_ATTRIBUTES | 0x1);
            }
            continue;
        }
        for p in (0..size).step_by(PAGE as usize) {
            let v = va + p;
            let mut off = TRANSLATION_TABLES;
            for level in 0..3 {
                off = table_at(mem, off, (v >> (39 - 9 * level)) & 0x1FF)?;
            }
            write(mem, off + 8 * ((v >> 12) & 0x1FF), (ipa + p) | PAGE_ATTRIBUTES | 0x3);
        }
    }
    Ok((IPA_BASE + TRANSLATION_TABLES, nxt))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Walks the tables for `va` as the MMU does: the IPA it reaches, if any.
    fn walk(mem: &[u8], root: u64, va: u64) -> Option<u64> {
        let mut table = root - IPA_BASE;
        for level in 0..4u64 {
            let desc = read(mem, table + 8 * ((va >> (39 - 9 * level)) & 0x1FF));
            if desc & 1 == 0 {
                return None;
            }
            let out = desc & 0x0000_FFFF_FFFF_F000;
            let is_table = desc & 2 != 0 && level < 3;
            if !is_table {
                let span = 1u64 << (39 - 9 * level);
                let base = out & !(span - 1);
                return Some(base | (va & (span - 1)));
            }
            table = out - IPA_BASE;
        }
        None
    }

    #[test]
    fn blocks_and_pages_translate_where_the_maps_say() {
        let mut mem = vec![0u8; (TRANSLATION_TABLES + TRANSLATION_TABLES_SIZE) as usize];
        let maps = [
            (0x7F00_0000_0000, IPA_BASE, 4 * BLOCK),            // 2 MB blocks
            (0x7FFE_0000_0000, IPA_BASE + 4 * BLOCK, 3 * PAGE), // pages
            (0x10_0000_0000, 2 * GIGA_BLOCK, GIGA_BLOCK),       // a 1 GB block
        ];
        let (root, next) = write_translation_tables(&mut mem, &maps).unwrap();
        assert_eq!(root, IPA_BASE + TRANSLATION_TABLES);
        assert_eq!(walk(&mem, root, 0x7F00_0000_1234), Some(IPA_BASE + 0x1234));
        assert_eq!(walk(&mem, root, 0x7F00_0060_0008), Some(IPA_BASE + 0x60_0008));
        assert_eq!(walk(&mem, root, 0x7FFE_0000_2010), Some(IPA_BASE + 4 * BLOCK + 0x2010));
        assert_eq!(walk(&mem, root, 0x7FFE_0000_3000), None);
        assert_eq!(walk(&mem, root, 0x10_2345_6789), Some(2 * GIGA_BLOCK + 0x2345_6789));
        // level 0 and the 2 MB chain share their tables: root, one L1 and one L2 for the blocks, the pages' L1 to L3,
        // and the 1 GB block's L1
        assert_eq!(next, TRANSLATION_TABLES + PAGE * 7);
    }

    #[test]
    fn a_full_area_is_refused() {
        let mut mem = vec![0u8; (TRANSLATION_TABLES + TRANSLATION_TABLES_SIZE) as usize];
        let maps: Vec<(u64, u64, u64)> = (0..100).map(|i| ((i as u64) << 39, IPA_BASE, PAGE)).collect();
        assert_eq!(write_translation_tables(&mut mem, &maps), Err(TableError::Full));
    }
}
