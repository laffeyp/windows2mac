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
//! `_GuestView` (the file bridge, the kernel requests, HOST_MODULE_PATH's path) and HOST_RANDOM's bytes. The last
//! column (Sprint 209) is the sha256 of the process area below the trace (`PROCESS_VA` to `TRACE_OFF`: the PEB, the
//! loader lists, the TEBs, the system images, the transport), which holds the host's writes no wrap reaches, the
//! loader's and the threads'; the trace's records are the call stream's, which `batch_run.rs` judges. The arena's
//! writes (a mapped DLL, VirtualQuery's record on the guest stack) are in neither.

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
    pub fn resume(&mut self, executed: u64, warp: u64, process: &[u8]) -> Result<(), String> {
        let Some(row) = self.pending.take() else {
            return Ok(());
        };
        let process = guest_runtime::sha256::hex(process);
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
            "{}\t{}\t{:x}\t{executed}\t{warp}\t{request}\t{process}",
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

/// A host request as the guest asked it (Sprint 258): its ledger row, its op and four argument words, and the
/// instruction count its bridge stored on entering the service. A row's monitor word is the one the next exit finds,
/// so a request's own count is the previous row's; two runs whose other exits differ (a cache's `hvc 1`s) show
/// different words on the request's own row for the same count.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Asked {
    pub row: u64,
    pub words: [u64; 5],
    pub count: u64,
}

/// A ledger's requests in order, from its text.
pub fn asked(text: &str) -> Result<Vec<Asked>, String> {
    let mut out = Vec::new();
    let mut previous = 0u64;
    for (n, line) in text.lines().enumerate() {
        let cells: Vec<&str> = line.split('\t').collect();
        let bad = || format!("ledger line {}: {line:?}", n + 1);
        if cells.len() < 14 {
            return Err(bad());
        }
        if cells[1] == "hvc 8" {
            let mut words = [0u64; 5];
            for (w, cell) in words.iter_mut().zip(&cells[5..10]) {
                *w = u64::from_str_radix(cell, 16).map_err(|_| bad())?;
            }
            let row = cells[0].parse().map_err(|_| bad())?;
            out.push(Asked {
                row,
                words,
                count: previous,
            });
        }
        previous = cells[3].parse().map_err(|_| bad())?;
    }
    Ok(out)
}

/// The first request two runs' guests asked differently, by op, argument or entry count, as its index and both sides;
/// None when one run's requests begin the other's. The host's answers are left out: its words and the bytes it wrote
/// carry the host's file times and inode numbers, which differ in every run (Sprint 258: `C:\w2m`'s times, a file's
/// inode).
pub fn first_parting(a: &[Asked], b: &[Asked]) -> Option<(usize, Asked, Asked)> {
    a.iter()
        .zip(b)
        .enumerate()
        .find(|(_, (x, y))| x.words != y.words || x.count != y.count)
        .map(|(i, (x, y))| (i, *x, *y))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(seq: u64, kind: &str, executed: u64, request: &str) -> String {
        format!("{seq}\t{kind}\t1800\t{executed}\t0\t{request}\tfeed")
    }

    /// Two ledgers alike but for the answers, the sites and an extra `hvc 1`, then one planted request with another
    /// entry count: `first_parting` names the planted request and no earlier one.
    #[test]
    fn the_first_parting_is_the_first_request_asked_differently() {
        let ask = |a2: u64, answer: &str| format!("9\tce\t10\t{a2:x}\t0\t0\t800\t{answer}");
        // a row's count is the one the next exit finds: the requests' own counts are 0, 100 and 250 against 0, 100 and
        // 309, b's second request reached through one `hvc 1` more
        let none = ["-"; 8].join("\t");
        let a = [
            row(0, "hvc 8", 50, &ask(0, "aa")),
            row(1, "hvc 1", 100, &none),
            row(2, "hvc 8", 250, &ask(800, "bb")),
            row(3, "hvc 8", 300, &ask(1000, "cc")),
        ]
        .join("\n");
        let b = [
            row(0, "hvc 8", 50, &ask(0, "dd")),
            row(1, "hvc 1", 70, &none),
            row(2, "hvc 1", 100, &none),
            row(3, "hvc 8", 309, &ask(800, "ee")),
            row(4, "hvc 8", 360, &ask(1000, "ff")),
        ]
        .join("\n");
        let (a, b) = (asked(&a).unwrap(), asked(&b).unwrap());
        assert_eq!(a.len(), 3);
        let (i, x, y) = first_parting(&a, &b).expect("a parting");
        assert_eq!(i, 2, "the request at offset 0x1000, asked at another count");
        assert_eq!((x.count, y.count), (250, 309));
        assert_eq!(first_parting(&a[..2], &b[..2]), None);
    }
}
