// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! Where the batch run ends (Sprint 219; the Decision "Sequence after Sprint 214"): `run_frontier.py`'s `stop_when`
//! (284-310). All eight batch-mode game runs on Windows end at the same 168 log lines, so past them nothing is judged.
//! The run stops once the game's unity.log on the bridge's drive holds the reference's line count and the main thread
//! has made, for every function it calls at all, as many calls as the Windows main thread made; or, failing the calls,
//! `REACH_AFTER_LOG` instructions after the log reached its count. A function the w2m main thread never calls is left
//! to the comparison, which reports it.
//!
//! The frozen host asks once a wall-clock second (`kernel.py` 68), so where it stops depends on the host's speed; this
//! asks after each host request the run answers, points on the instruction clock, so one tree stops at the same row on
//! every run (reviewer-3's Concern 5 of its review of Sprint 218). So `REACH_AFTER_LOG` is measured only at a request
//! too: a main thread that spins in the VM after the log fills, asking nothing, never meets the bound and runs to the
//! step budget, which `batch_run.rs` and `batch_record.rs` fail (reviewer-3's Low 3 of its review of Sprint 219). The
//! inputs are plain, a line count and the Windows
//! main thread's calls (`store/batch_run/main-thread-calls-2026-10-29.tsv`, which `call_compare` derives): the host
//! reads no contract and no reference run (its Medium 4).

use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::guest::Guest;

/// `REACH_AFTER_LOG` (`run_frontier.py` 70): the instructions a run goes on past the reference's last line for its main
/// thread's calls.
pub const REACH_AFTER_LOG: u64 = 20_000_000_000;

/// A function's calls on a main thread, by (calling image in lower case, function).
pub type Calls = BTreeMap<(String, String), u64>;

/// The decision once the log holds its `lines` (`stop_when`, 302-309): end when no function the w2m main thread calls
/// is short of the Windows main thread's count (`0 < mine < windows`), or when `since_log` instructions have run since
/// the log reached its count and `REACH_AFTER_LOG` is spent; else go on.
pub fn decide(lines: usize, since_log: u64, mine: &Calls, windows: &Calls) -> Option<String> {
    let short = windows
        .iter()
        .filter(|(f, n)| mine.get(*f).is_some_and(|m| 0 < *m && m < *n))
        .count();
    if short == 0 {
        Some(format!(
            "the log holds the reference's {lines} lines and the main thread its calls"
        ))
    } else if since_log >= REACH_AFTER_LOG {
        Some(format!(
            "the log holds the reference's {lines} lines; the main thread short of {short} functions' calls \
             {REACH_AFTER_LOG} instructions later"
        ))
    } else {
        None
    }
}

/// The stop's state through a run.
pub struct BatchStop {
    log: PathBuf,
    lines: usize,
    windows_main: Calls,
    /// the log's size when its lines were last counted
    log_size: Option<u64>,
    /// the instructions executed when the log first held its lines (`log_end`)
    log_end: Option<u64>,
    /// the w2m main thread's returned calls counted so far, the next record to read, and records read in flight
    mine: Calls,
    cursor: u64,
    pending: Vec<u64>,
}

/// A text's lines as Python's `bytes.splitlines` counts them for "\n" and "\r\n" endings: empty lines count, and a last
/// line without its ending does too.
pub fn lines(text: &[u8]) -> usize {
    text.iter().filter(|b| **b == b'\n').count() + usize::from(text.last().is_some_and(|b| *b != b'\n'))
}

/// `store/batch_run/main-thread-calls-2026-10-29.tsv`'s rows: image, function, count; `#` lines are comments.
pub fn read_calls(text: &str) -> Result<Calls, String> {
    text.lines()
        .filter(|l| !l.starts_with('#') && !l.is_empty())
        .map(|l| {
            let c: Vec<&str> = l.split('\t').collect();
            match c.as_slice() {
                [image, function, n] => Ok((
                    (image.to_string(), function.to_string()),
                    n.parse().map_err(|_| format!("main-thread calls: {l:?}"))?,
                )),
                _ => Err(format!("main-thread calls: {l:?}")),
            }
        })
        .collect()
}

impl BatchStop {
    /// The stop for a run whose log is `log` on the host, ended at `lines` lines, its main thread held to `windows_main`.
    pub fn new(log: PathBuf, lines: usize, windows_main: Calls) -> BatchStop {
        BatchStop {
            log,
            lines,
            windows_main,
            log_size: None,
            log_end: None,
            mine: Calls::new(),
            cursor: 0,
            pending: Vec::new(),
        }
    }

    /// The stop's reason once the run should end, None before; `executed` is the monitor's instruction count. The main
    /// thread is the run's first thread, whose start is the program's entry (`min(vm.threads)` in the Python).
    pub fn check(&mut self, guest: &Guest, executed: u64) -> Result<Option<String>, String> {
        if self.log_end.is_none() {
            let Ok(meta) = std::fs::metadata(&self.log) else {
                return Ok(None);
            };
            if self.log_size == Some(meta.len()) {
                return Ok(None);
            }
            self.log_size = Some(meta.len());
            let text = std::fs::read(&self.log).map_err(|e| format!("{}: {e}", self.log.display()))?;
            if lines(&text) < self.lines {
                return Ok(None);
            }
            self.log_end = Some(executed);
        }
        self.count_main_thread(guest)?;
        Ok(decide(
            self.lines,
            executed - self.log_end.unwrap_or(executed),
            &self.mine,
            &self.windows_main,
        ))
    }

    /// The main thread's returned calls in the records not read yet, and in those read while in flight.
    fn count_main_thread(&mut self, guest: &Guest) -> Result<(), String> {
        let main = guest.start.tid;
        let (count, _) = guest.trace_count()?;
        let mut still = Vec::new();
        for k in std::mem::take(&mut self.pending).into_iter().chain(self.cursor..count) {
            let r = guest.trace_record(k)?;
            if !r.ready {
                still.push(k);
            } else if r.tid == main {
                *self
                    .mine
                    .entry((r.label.to_lowercase(), r.function.to_string()))
                    .or_default() += 1;
            }
        }
        self.cursor = count;
        self.pending = still;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{Calls, REACH_AFTER_LOG, decide, lines, read_calls};

    fn calls(rows: &[(&str, u64)]) -> Calls {
        rows.iter()
            .map(|(f, n)| (("unityplayer.dll".to_string(), f.to_string()), *n))
            .collect()
    }

    /// Lines as Python's `splitlines` counts "\r\n" endings: empty lines count, a last line without its ending counts.
    #[test]
    fn a_log_is_counted_as_python_splits_it() {
        assert_eq!(lines(b"a\r\nb\r\n"), 2);
        assert_eq!(lines(b"a\r\n\r\nb"), 3);
        assert_eq!(lines(b""), 0);
    }

    /// The main thread's calls end the run once none it makes is short of Windows'; a function it never calls is the
    /// comparison's; a short one holds the run until `REACH_AFTER_LOG` instructions after the log, and not before.
    #[test]
    fn the_run_ends_on_the_main_threads_calls_or_the_bound() {
        let windows = calls(&[("Sleep", 3), ("GetTickCount", 2), ("CreateEventW", 1)]);
        let all = calls(&[("Sleep", 3), ("GetTickCount", 2)]);
        assert_eq!(
            decide(168, 0, &all, &windows).as_deref(),
            Some("the log holds the reference's 168 lines and the main thread its calls")
        );
        let short = calls(&[("Sleep", 2), ("GetTickCount", 2)]);
        assert_eq!(decide(168, REACH_AFTER_LOG - 1, &short, &windows), None);
        assert_eq!(
            decide(168, REACH_AFTER_LOG, &short, &windows).as_deref(),
            Some(
                "the log holds the reference's 168 lines; the main thread short of 1 functions' calls 20000000000 \
                 instructions later"
            )
        );
    }

    /// The stored rows read as image, function, count, comments left out; a malformed row is an error.
    #[test]
    fn the_main_thread_calls_are_read_as_stored() {
        let read = read_calls("# a comment\nunityplayer.dll\tSleep\t3\n").unwrap();
        assert_eq!(read, calls(&[("Sleep", 3)]));
        assert!(read_calls("unityplayer.dll\tSleep\n").is_err());
    }
}
