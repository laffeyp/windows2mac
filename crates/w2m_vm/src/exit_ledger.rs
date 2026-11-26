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
//! writes (a mapped DLL, VirtualQuery's record on the guest stack) are in neither. From Sprint 352 a ledger given a
//! `Renaming` hashes the area with each aligned word that points into the runtime image renamed to the frozen image's
//! address of the same symbol and offset (below), so the column reads what the guest holds, not where the linker put
//! the runtime: a pointer into the image moves whenever the C's `__text` crosses a page.

use std::borrow::Cow;
use std::collections::BTreeMap;
use std::ops::Range;

use std::io::Write;

/// The row the next resume writes (`Ledger::pending`).
pub struct Row {
    pub kind: String,
    pub site: u64,
    /// the request's op, four arguments, second and result, read at the exit and after the service
    pub request: Option<[u64; 7]>,
}

/// The runtime image's addresses read by what they name (Sprint 352): a word in the image's range, as `name+0xoffset`
/// from the nearest symbol of the image the run built at or below it, becomes the frozen image's address of that symbol
/// plus the offset, as `batch_run.rs` names a runtime address in the call stream against the frozen image (Sprint 272,
/// its lines 211-217). Before, GetCommandLineA's answer, `&command_line_a`, moved from 0x7F000446E120 to 0x7F000446F120
/// when Sprint 352's C grew `__text` past a page, and every later row's process column parted from the frozen host's
/// (the guest's saved RAX at the state's offset 0 and ring record 121's return).
pub struct Renaming {
    ours: Vec<(u64, String)>,
    frozen: BTreeMap<String, u64>,
    range: Range<u64>,
}

impl Renaming {
    /// The run's image `ours` and the frozen host's `frozen`, both linked at `base`, `size` bytes of address space.
    pub fn new(ours: &guest_runtime::Image, frozen: &guest_runtime::Image, base: u64, size: u64) -> Renaming {
        let mut by_address: Vec<(u64, String)> = ours.symbols.iter().map(|(n, a)| (*a, n.clone())).collect();
        by_address.sort();
        Renaming {
            ours: by_address,
            frozen: frozen.symbols.clone(),
            range: base..base + size,
        }
    }

    /// `word` as the frozen image's address of the same symbol and offset, the offset kept, so two pointers apart stay
    /// apart; unchanged outside the image or below its first symbol. A word naming a symbol the frozen image lacks is
    /// refused by that name, not mapped: no frozen address stands for it, and a guess would hide a parting.
    pub fn word(&self, word: u64) -> Result<u64, String> {
        if !self.range.contains(&word) {
            return Ok(word);
        }
        let Some(i) = self.ours.partition_point(|(a, _)| *a <= word).checked_sub(1) else {
            return Ok(word);
        };
        let (at, name) = &self.ours[i];
        match self.frozen.get(name) {
            Some(f) => Ok(f + (word - at)),
            None => Err(format!(
                "{word:#x} names {name}+{:#x}, a symbol the frozen image lacks",
                word - at
            )),
        }
    }

    /// `process` with each 8-byte word at an offset a multiple of 8 renamed; borrowed when none changes. The area
    /// begins on a page (`PROCESS_VA`), so its aligned offsets are aligned addresses.
    pub fn area<'a>(&self, process: &'a [u8]) -> Result<Cow<'a, [u8]>, String> {
        let mut out: Option<Vec<u8>> = None;
        for (k, chunk) in process.as_chunks::<8>().0.iter().enumerate() {
            let word = u64::from_le_bytes(*chunk);
            if !self.range.contains(&word) {
                continue;
            }
            let renamed = self.word(word)?;
            if renamed != word {
                out.get_or_insert_with(|| process.to_vec())[8 * k..8 * k + 8].copy_from_slice(&renamed.to_le_bytes());
            }
        }
        Ok(out.map_or(Cow::Borrowed(process), Cow::Owned))
    }
}

pub struct Ledger {
    /// where the rows go; a child's ledger is this path and ".<pid>"
    pub path: std::path::PathBuf,
    /// the runtime image's addresses renamed before the process area is hashed (Sprint 352); None hashes it as it is
    pub renaming: Option<Renaming>,
    /// unbuffered: a child process is stopped when the run ends, and a buffer would be lost with it (a child's ledger
    /// ended at its first 8 KB, mid-row, before Sprint 206 wrote each row through)
    out: std::fs::File,
    seq: u64,
    pending: Option<Row>,
    written: Vec<u8>,
    /// a write's address the renaming refused, reported at the row's resume
    refused: Option<String>,
}

impl Ledger {
    pub fn create(path: &std::path::Path) -> Result<Ledger, String> {
        let file = std::fs::File::create(path).map_err(|e| format!("{}: {e}", path.display()))?;
        Ok(Ledger {
            path: path.to_path_buf(),
            renaming: None,
            out: file,
            seq: 0,
            pending: None,
            written: Vec::new(),
            refused: None,
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

    /// `word` renamed when the ledger has a `Renaming` (Sprint 352): a request's words and a write's address, which name
    /// the runtime's own buffers (a1 0x7F0004479DBA, a0 0x7F000446F128 at Sprint 352's layout), as the process area.
    fn named(&self, word: u64) -> Result<u64, String> {
        self.renaming.as_ref().map_or(Ok(word), |r| r.word(word))
    }

    /// A write a host request made through its memory view; a refused address is kept for the row's resume to report.
    pub fn wrote(&mut self, va: u64, data: &[u8]) {
        let va = match self.named(va) {
            Ok(v) => v,
            Err(why) => {
                self.refused.get_or_insert(why);
                va
            }
        };
        self.written.extend(va.to_le_bytes());
        self.written.extend((data.len() as u64).to_le_bytes());
        self.written.extend(data);
    }

    /// The pending row, with the count and warp as the guest will resume with them.
    pub fn resume(&mut self, executed: u64, warp: u64, process: &[u8]) -> Result<(), String> {
        let Some(row) = self.pending.take() else {
            return Ok(());
        };
        if let Some(why) = self.refused.take() {
            return Err(why);
        }
        let process = match &self.renaming {
            Some(r) => guest_runtime::sha256::hex(&r.area(process)?),
            None => guest_runtime::sha256::hex(process),
        };
        let written = std::mem::take(&mut self.written);
        let request = match row.request {
            Some(r) => {
                let hash = if written.is_empty() {
                    "-".to_string()
                } else {
                    guest_runtime::sha256::hex(&written)
                };
                let words = r
                    .iter()
                    .map(|w| self.named(*w).map(|n| format!("{n:x}")))
                    .collect::<Result<Vec<String>, String>>()?;
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

    /// Sprint 352: a pointer into the runtime image, `command_line_a+0x10` one page apart in two layouts (as Sprint 352's
    /// C moved it, 0x7F000446E120 to 0x7F000446F120), hashes the same once renamed against the frozen image; a word
    /// outside the image's range is hashed unchanged; and without the renaming the two areas hash apart.
    #[test]
    fn a_runtime_pointer_hashes_the_same_at_two_layouts_and_other_words_unchanged() {
        let base = 0x7F00_0417_0000u64;
        // command_line_w 0x200 past command_line_a in both; `added_since` in ours alone, as a symbol new since the frozen
        // tag
        let image = |data: u64, added: bool| {
            let mut symbols = BTreeMap::from([
                ("w2m_image_entry".to_string(), base),
                ("command_line_a".to_string(), data),
                ("command_line_w".to_string(), data + 0x200),
            ]);
            if added {
                symbols.insert("added_since".to_string(), data + 0x400);
            }
            guest_runtime::Image {
                segments: Vec::new(),
                symbols,
                file: std::path::PathBuf::new(),
            }
        };
        let (frozen, ours) = (image(0x7F00_0446_E110, false), image(0x7F00_0446_F110, true));
        let renaming = Renaming::new(&ours, &frozen, base, 0x80_0000);
        // the negative case (reviewer-4's twenty-second review, Medium 1): pointers that differ stay apart once renamed,
        // by symbol and by offset, and a symbol the frozen image lacks is refused by its name
        let hashed = |pointer: u64| {
            let mut a = vec![0u8; 16];
            a[8..16].copy_from_slice(&pointer.to_le_bytes());
            guest_runtime::sha256::hex(&renaming.area(&a).unwrap())
        };
        let (a, w) = (0x7F00_0446_F120, 0x7F00_0446_F320);
        assert_ne!(
            hashed(a),
            hashed(w),
            "command_line_a+0x10 and command_line_w+0x10, renamed"
        );
        assert_ne!(
            hashed(a),
            hashed(a + 8),
            "command_line_a+0x10 and command_line_a+0x18, renamed"
        );
        assert_eq!(renaming.word(a + 8), Ok(0x7F00_0446_E128), "the offset kept");
        assert_eq!(
            renaming.word(0x7F00_0446_F518),
            Err("0x7f000446f518 names added_since+0x8, a symbol the frozen image lacks".to_string()),
            "a symbol the frozen image lacks"
        );
        let area = |pointer: u64, other: u64| -> Vec<u8> {
            let mut a = vec![0u8; 64];
            a[8..16].copy_from_slice(&pointer.to_le_bytes());
            a[24..32].copy_from_slice(&other.to_le_bytes());
            a
        };
        let frozen_area = area(0x7F00_0446_E120, 0x1_4000_1000);
        let our_area = area(0x7F00_0446_F120, 0x1_4000_1000);
        assert_eq!(
            guest_runtime::sha256::hex(&renaming.area(&our_area).unwrap()),
            guest_runtime::sha256::hex(&frozen_area),
            "the pointer renamed by symbol and offset"
        );
        assert_ne!(
            guest_runtime::sha256::hex(&our_area),
            guest_runtime::sha256::hex(&frozen_area),
            "the two layouts' areas, not renamed"
        );
        assert_eq!(
            renaming.word(0x1_4000_1000),
            Ok(0x1_4000_1000),
            "a word outside the image"
        );
        assert_eq!(
            renaming.word(base + 0x80_0000),
            Ok(base + 0x80_0000),
            "the word past the image's end"
        );
        let outside = area(0, 0x7F00_0200_0000);
        assert!(
            matches!(renaming.area(&outside), Ok(Cow::Borrowed(_))),
            "an area with no word in the image is hashed as it is"
        );
        // a request's words and a write's address, renamed alike: the row a ledger writes at our layout is the frozen
        // layout's
        let path = std::env::temp_dir().join(format!("w2m-renaming-{}.tsv", std::process::id()));
        let mut ledger = Ledger::create(&path).unwrap();
        ledger.renaming = Some(renaming);
        ledger.exit(Row {
            kind: "hvc 8".into(),
            site: 0x1800,
            request: Some([0x18, 0x7F00_0446_F128, 6, 0x7F00_0416_FF68, 0, 0, 2]),
        });
        ledger.wrote(0x7F00_0446_F128, b"abc");
        ledger.resume(10, 0, &our_area).unwrap();
        ledger.flush().unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        let _ = std::fs::remove_file(&path);
        let mut written = 0x7F00_0446_E128u64.to_le_bytes().to_vec();
        written.extend(3u64.to_le_bytes());
        written.extend(b"abc");
        assert_eq!(
            text,
            format!(
                "0\thvc 8\t1800\t10\t0\t18\t7f000446e128\t6\t7f000416ff68\t0\t0\t2\t{}\t{}\n",
                guest_runtime::sha256::hex(&written),
                guest_runtime::sha256::hex(&frozen_area)
            ),
            "the row at our layout, named as the frozen layout's"
        );
    }
}
