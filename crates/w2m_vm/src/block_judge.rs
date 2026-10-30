// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! The whole-block judge (Sprint 252; `docs/design-translation-cache-2026-10-30.md`, choice F, card 1): each block the
//! in-VM compiler made, compiled again by the Rust translator from the same bytes, at the same code address and with
//! the flags the C left live (`live_used`, `block_compiler.c` 38), its words and marks compared with the C's.
//! `w2m-vm --judge-blocks <file>` asks for it; the run loop calls `judge` before each drain of the compiler's log.

use std::fs::File;
use std::io::{BufWriter, Write};

use guest_runtime::layout::{MONITOR_VA, PROCESS_VA, RECON_TABLE, STATE_OFF, VM_COMPILER, VMC_LOG, VMC_LOG_ENTRY};
use translator::lower::{RuntimeCalls, assemble_block};

use crate::compile::lift_at;
use crate::guest::{Guest, VMC_CURSOR};
use crate::run::VMC_LOG_COUNT;

fn read_u64(mem: &[u8], off: u64) -> u64 {
    u64::from_le_bytes(mem[off as usize..off as usize + 8].try_into().unwrap())
}

fn read_u32(mem: &[u8], off: u64) -> u32 {
    u32::from_le_bytes(mem[off as usize..off as usize + 4].try_into().unwrap())
}

/// The judge's file and its totals.
pub struct BlockJudge {
    out: BufWriter<File>,
    pub judged: u64,
    pub equal: u64,
    pub differing: u64,
}

impl BlockJudge {
    pub fn create(path: &std::path::Path) -> Result<BlockJudge, String> {
        let file = File::create(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let mut out = BufWriter::new(file);
        writeln!(out, "# start\tend\tword\tc\trust\tinstruction\tclass").map_err(|e| e.to_string())?;
        Ok(BlockJudge {
            out,
            judged: 0,
            equal: 0,
            differing: 0,
        })
    }

    /// Every block in the compiler's log, before the drain empties it: its words from its `code` to the next entry's,
    /// the last to the cursor.
    pub fn judge(&mut self, guest: &Guest) -> Result<(), String> {
        let mem = guest.memory.as_slice();
        let n = read_u64(mem, VM_COMPILER + VMC_LOG_COUNT);
        let cursor_va = MONITOR_VA + read_u64(mem, VM_COMPILER + VMC_CURSOR);
        let runtime = RuntimeCalls::from_symbols(|s| guest.symbols.get(s).copied());
        for i in 0..n {
            let at = VM_COMPILER + VMC_LOG + VMC_LOG_ENTRY * i;
            let (start, end, code) = (read_u64(mem, at), read_u64(mem, at + 8), read_u64(mem, at + 16));
            let (first, nmarks, count, live) = (
                u64::from(read_u32(mem, at + 24)),
                read_u32(mem, at + 28) as usize,
                read_u32(mem, at + 32),
                read_u32(mem, at + 44),
            );
            let next = if i + 1 < n {
                read_u64(mem, VM_COMPILER + VMC_LOG + VMC_LOG_ENTRY * (i + 1) + 16)
            } else {
                cursor_va
            };
            let c_words: Vec<u32> = (code..next)
                .step_by(4)
                .map(|va| read_u32(mem, va - MONITOR_VA))
                .collect();
            let c_marks: Vec<(u64, u64)> = (0..nmarks as u64)
                .map(|k| {
                    let e = RECON_TABLE + 16 * (first + k);
                    (read_u64(mem, e), read_u64(mem, e + 8))
                })
                .collect();
            let mut insns = Vec::new();
            let mut addr = start;
            for _ in 0..count {
                let ins = lift_at(guest, addr).map_err(|e| format!("0x{start:x}: {}", e.0))?;
                addr += ins.length;
                insns.push(ins);
            }
            if addr != end {
                return Err(format!(
                    "0x{start:x}: {count} instructions end at 0x{addr:x}, the log says 0x{end:x}"
                ));
            }
            self.judged += 1;
            let line = match assemble_block(
                &insns,
                code,
                guest.code.dispatch_va,
                guest.slice_va,
                PROCESS_VA + STATE_OFF,
                &runtime,
                live,
            ) {
                Err(e) => Some(format!("-\t-\t-\t{e}\tassemble")),
                Ok((words, marks)) => match words.iter().zip(&c_words).position(|(r, c)| r != c) {
                    Some(k) => Some(format!(
                        "{k}\t{:08x}\t{:08x}\t{}\twords",
                        c_words[k],
                        words[k],
                        at_word(&marks, code + 4 * k as u64)
                    )),
                    None if words.len() != c_words.len() => Some(format!(
                        "{}\t-\t-\t{} words, the C's {}\tlength",
                        words.len().min(c_words.len()),
                        words.len(),
                        c_words.len()
                    )),
                    None if marks != c_marks => Some("-\t-\t-\t-\tmarks".to_string()),
                    None => None,
                },
            };
            match line {
                None => self.equal += 1,
                Some(l) => {
                    self.differing += 1;
                    writeln!(self.out, "{start:x}\t{end:x}\t{l}").map_err(|e| e.to_string())?;
                }
            }
        }
        Ok(())
    }

    /// The totals, last, and the file flushed.
    pub fn finish(&mut self) -> Result<(), String> {
        writeln!(
            self.out,
            "# judged\t{}\tequal\t{}\tdiffering\t{}",
            self.judged, self.equal, self.differing
        )
        .map_err(|e| e.to_string())?;
        self.out.flush().map_err(|e| e.to_string())
    }
}

/// The x86 instruction whose words hold the host address `va`, from the marks: the last mark at or before it.
fn at_word(marks: &[(u64, u64)], va: u64) -> String {
    marks
        .iter()
        .rev()
        .find(|(host, _)| *host <= va)
        .map_or_else(|| "-".to_string(), |(_, x86)| format!("0x{x86:x}"))
}
