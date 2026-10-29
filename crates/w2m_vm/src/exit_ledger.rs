// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! The exit ledger (Sprint 206; reviewer-2's High 1 and Medium 1 of its review of Sprints 199-205): one row per host
//! exit of a process, written just before the guest runs again, so that the frozen Python host's run and the Rust
//! host's can be compared exit by exit. The call stream holds a call's arguments, return value and last error; the
//! ledger holds what the stream cannot see: the instruction count (monitor word 0), the clock's warp, and what a host
//! request wrote into guest memory. The frozen host's fixture writes the same columns (WORKING_AGREEMENT, the exit
//! ledger's fixture), each process to its own file.
//!
//! Columns: the exit's ordinal in the process; its kind ("hvc N", or the exit's name); the x86 site (`x86_address_at`
//! of the hvc, the dispatch target for hvc 1); monitor word 0; the warp; and for hvc 8 the request's op, its four
//! argument words, the second and result words at the transport's 48 and 56, and the sha256 of the writes the host made
//! through the request's memory view, each as its address and length (little-endian u64s) then its bytes, "-" for
//! none. Other exits leave the request columns "-". The written ranges are those the frozen host makes through
//! `_GuestView` (the file bridge, the kernel requests, HOST_MODULE_PATH's path) and HOST_RANDOM's bytes; the arena's
//! zeroing, VirtualQuery's record and the thread requests' writes are not in it yet.

use std::io::Write;

/// The row the next resume writes (`Ledger::pending`).
pub struct Row {
    pub kind: String,
    pub site: u64,
    /// the request's op, four arguments, second and result, read at the exit and after the service
    pub request: Option<[u64; 7]>,
}

pub struct Ledger {
    /// where the rows go; a child's ledger is this path and ".<pid>"
    pub path: std::path::PathBuf,
    /// unbuffered: a child process is stopped when the run ends, and a buffer would be lost with it (a child's ledger
    /// ended at its first 8 KB, mid-row, before Sprint 206 wrote each row through)
    out: std::fs::File,
    seq: u64,
    pending: Option<Row>,
    written: Vec<u8>,
}

impl Ledger {
    pub fn create(path: &std::path::Path) -> Result<Ledger, String> {
        let file = std::fs::File::create(path).map_err(|e| format!("{}: {e}", path.display()))?;
        Ok(Ledger {
            path: path.to_path_buf(),
            out: file,
            seq: 0,
            pending: None,
            written: Vec::new(),
        })
    }

    /// An exit, to be written at the next resume.
    pub fn exit(&mut self, row: Row) {
        self.pending = Some(row);
    }

    /// The request's answer words, read after its service.
    pub fn answered(&mut self, second: u64, result: u64) {
        if let Some(Row { request: Some(r), .. }) = &mut self.pending {
            r[5] = second;
            r[6] = result;
        }
    }

    /// A write a host request made through its memory view.
    pub fn wrote(&mut self, va: u64, data: &[u8]) {
        self.written.extend(va.to_le_bytes());
        self.written.extend((data.len() as u64).to_le_bytes());
        self.written.extend(data);
    }

    /// The pending row, with the count and warp as the guest will resume with them.
    pub fn resume(&mut self, executed: u64, warp: u64) -> Result<(), String> {
        let Some(row) = self.pending.take() else {
            return Ok(());
        };
        let written = std::mem::take(&mut self.written);
        let request = match row.request {
            Some(r) => {
                let hash = if written.is_empty() {
                    "-".to_string()
                } else {
                    guest_runtime::sha256::hex(&written)
                };
                let words: Vec<String> = r.iter().map(|w| format!("{w:x}")).collect();
                format!("{}\t{hash}", words.join("\t"))
            }
            None => ["-"; 8].join("\t"),
        };
        writeln!(
            self.out,
            "{}\t{}\t{:x}\t{executed}\t{warp}\t{request}",
            self.seq, row.kind, row.site
        )
        .map_err(|e| e.to_string())?;
        self.seq += 1;
        Ok(())
    }

    /// Every block compiled in the run, the VM's and the host's, as (start, end) sorted, to the ledger's path and
    /// ".blocks" (Sprint 207: the frozen host's `vm.translations` beside it): where the two hosts cut blocks, which sets
    /// the count a service reads between exits.
    pub fn blocks(&self, blocks: &[(u64, u64)]) -> Result<(), String> {
        let mut sorted = blocks.to_vec();
        sorted.sort_unstable();
        let text: String = sorted.iter().map(|(s, e)| format!("{s:x}\t{e:x}\n")).collect();
        let path = std::path::PathBuf::from(format!("{}.blocks", self.path.display()));
        std::fs::write(&path, text).map_err(|e| format!("{}: {e}", path.display()))
    }

    pub fn flush(&mut self) -> Result<(), String> {
        self.out.flush().map_err(|e| e.to_string())
    }
}
