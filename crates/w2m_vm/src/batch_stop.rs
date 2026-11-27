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
//!
//! On several vCPUs (Sprint 367; the Decision "Step 6's order after G2, weighed again at Sprint 366") the batch run is
//! on the wall clock and the rule is the same: asked after each request any vCPU answers, its state behind the run's
//! lock (`run.rs`, the request's service, which holds it), so the requests are taken one at a time in the lock's order.
//! The instructions are the run's, every vCPU's count summed (`run_instructions`), as `stop_when` reads
//! `vm.instructions_executed()`, the frozen host's sum over its VM's vCPUs (`vm_backend.py` 4125-4127 at the tag): on one
//! vCPU that is every thread's instructions, and summing keeps it so; a main thread's own count is not kept by either
//! host. A main thread that spins after the log fills is then bounded by the other threads: their requests ask the
//! stop, and the sum grows by every vCPU's instructions as each vCPU's monitor word holds them, so the bound is met
//! once the run's instructions since the log reach `REACH_AFTER_LOG`, whichever thread ran them; only a run in which no
//! thread asks anything again goes to the step budget or the run's time limit, as on one vCPU.

use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::VcpuPages;
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

/// The instructions since the log reached its count, `counts` each vCPU's count now and `log_end` the run's sum when the
/// log first held its lines: the run's sum less it, every vCPU's counted. 0 when the sum is below it: a slice's end gives
/// a block's unrun instructions back to its vCPU's word (`run.rs` `give_back`), which another vCPU's check may read
/// before and after, and a wrapped subtraction would end the run at once, or panic under the release profile's overflow
/// checks.
pub fn since_log(counts: &[u64], log_end: u64) -> u64 {
    run_instructions(counts).saturating_sub(log_end)
}

/// The run's instructions: every vCPU's count summed, `vm.instructions_executed()` (`vm_backend.py` 4125-4127).
pub fn run_instructions(counts: &[u64]) -> u64 {
    counts.iter().sum()
}

/// The monitor data page of vCPU `k`, whose first word is its count: vCPU 0's the layout's, each other's in its slot of
/// the vCPU area, where `Guest::private_chain` places it.
fn monitor_of(k: u64) -> u64 {
    match k {
        0 => VcpuPages::FIRST.monitor,
        k => guest_runtime::layout::VCPU_AREA + k * guest_runtime::layout::VCPU_STRIDE,
    }
}

/// Each of the run's vCPUs' counts now, `guest.start.vcpus` of them, read from each vCPU's monitor data page: the
/// count its last exit or service wrote, whichever vCPU asks.
pub fn vcpu_counts(guest: &Guest) -> Vec<u64> {
    let mem = guest.memory_view();
    (0..u64::from(guest.start.vcpus.max(1)))
        .map(|k| mem.read_u64(monitor_of(k)))
        .collect()
}

/// The stop's state through a run.
pub struct BatchStop {
    log: PathBuf,
    lines: usize,
    windows_main: Calls,
    /// the log's size when its lines were last counted
    log_size: Option<u64>,
    /// the run's instructions, every vCPU's, when the log first held its lines (`log_end`)
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

    /// The stop's reason once the run should end, None before, asked at a request any vCPU answers; the instructions
    /// are the run's, every vCPU's count (`vcpu_counts`). The main thread is the run's first thread, whose start is the
    /// program's entry (`min(vm.threads)` in the Python).
    pub fn check(&mut self, guest: &Guest) -> Result<Option<String>, String> {
        let counts = vcpu_counts(guest);
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
            self.log_end = Some(run_instructions(&counts));
        }
        self.count_main_thread(guest)?;
        Ok(decide(
            self.lines,
            since_log(&counts, self.log_end.unwrap_or(0)),
            &self.mine,
            &self.windows_main,
        ))
    }

    /// The main thread's returned calls in the records not read yet, and in those read while in flight. A record read
    /// in flight whose slot is now at or past the count was given back (`call_trace.c` 131-134: a traced service that
    /// retries, a wait that asked the host and then blocked) and is not read again: its call is recorded anew when the
    /// thread enters it again, and a read past the count is an error that would end the run (Sprint 367).
    fn count_main_thread(&mut self, guest: &Guest) -> Result<(), String> {
        let main = guest.start.tid;
        let (count, _) = guest.trace_count()?;
        let mut still = Vec::new();
        let pending = std::mem::take(&mut self.pending).into_iter().filter(|k| *k < count);
        for k in pending.chain(self.cursor..count) {
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

    /// The decision on several vCPUs' counts (Sprint 367): the instructions since the log are the run's, every vCPU's
    /// summed, so a main thread short of its calls is held until the vCPUs together have run `REACH_AFTER_LOG` past the
    /// log, though neither has alone; a sum below the log's, a give-back read on another vCPU, is 0, not a wrap.
    #[test]
    fn the_bound_counts_every_vcpus_instructions() {
        use super::since_log;
        let windows = calls(&[("Sleep", 3)]);
        let short = calls(&[("Sleep", 1)]);
        let log_end = 2_000_000_000;
        // vCPU 0 and vCPU 1 have each run 10,000,000,000 instructions past the log
        let counts = [1_000_000_000 + 10_000_000_000, 1_000_000_000 + 10_000_000_000];
        assert_eq!(
            since_log(&counts, log_end),
            REACH_AFTER_LOG,
            "the vCPUs' instructions since the log"
        );
        assert_eq!(
            decide(168, since_log(&counts, log_end), &short, &windows).as_deref(),
            Some(
                "the log holds the reference's 168 lines; the main thread short of 1 functions' calls 20000000000 \
                 instructions later"
            )
        );
        assert_eq!(decide(168, since_log(&counts[..1], log_end), &short, &windows), None);
        assert_eq!(
            since_log(&[999_999_000, 1_000_000_000], log_end),
            0,
            "a sum below the log's"
        );
    }

    /// A guest of `counts.len()` vCPUs whose monitor words hold `counts`, served as vCPU `asking`, its trace holding a
    /// main-thread record of one function for each of `ready` (returned or in flight); with that function's key, as the
    /// stop counts it (the "GetProcAddress" label, no traced image named).
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    fn guest_on(counts: &[u64], asking: u64, ready: &[bool]) -> (crate::guest::Guest, (String, String)) {
        use guest_runtime::layout::{MONITOR_SIZE, TRACE_OFF, TRACE_RECORD, TRACE_RECORDS_OFF};
        let (mut g, _) = crate::threads::tests::guest_with_a_thread();
        g.start.vcpus = counts.len() as u32;
        // each other vCPU's pages made first: the chain copies vCPU 0's monitor page into its own
        for k in 1..counts.len() as u64 {
            let pages = g.private_chain(k).unwrap().1;
            if k == asking {
                g.served.vcpu = pages;
            }
        }
        let mem = g.memory_view();
        for (k, n) in counts.iter().enumerate() {
            mem.write_u64(super::monitor_of(k as u64), *n);
        }
        let (&thunk, name) = g.served.thunks.iter().next().expect("a thunk");
        let function = name.split_once('!').unwrap().1.to_string();
        let t = MONITOR_SIZE + TRACE_OFF;
        mem.write_u64(t + 16, ready.len() as u64);
        mem.write_u64(t + 24, 64);
        for (k, done) in ready.iter().enumerate() {
            let r = t + TRACE_RECORDS_OFF + TRACE_RECORD * k as u64;
            mem.write_u64(r, u64::from(*done));
            mem.write_u64(r + 16, g.start.tid);
            mem.write_u64(r + 24, 9); // no traced image: the "GetProcAddress" label
            mem.write_u64(r + 32, (thunk - pe_loader::THUNK_BASE) / 16);
        }
        (g, ("getprocaddress".to_string(), function))
    }

    /// A log of `lines` lines in a directory of its own under the system's temporary directory.
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    fn log_of(lines: usize, name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("w2m-batch-stop-{name}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let log = dir.join("unity.log");
        std::fs::write(&log, "a line\r\n".repeat(lines)).unwrap();
        log
    }

    /// vCPU `k`'s monitor data page, where the stop reads its count, is the one `Guest::private_chain` places; and the
    /// stop asked on vCPU 1 of two counts both vCPUs (Sprint 367): with the log full at 1,000,000,000 instructions on
    /// each and the main thread short of a function's calls, the run goes on at 10,000,000,000 more on each, though the
    /// asking vCPU alone has run only half the bound, and ends at the bound's sum. Today's form, the asking vCPU's own
    /// count, held it to that vCPU's half.
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    #[test]
    fn the_stop_asked_on_one_vcpu_counts_every_vcpus_instructions() {
        use super::{BatchStop, monitor_of, vcpu_counts};
        let (mut g, function) = guest_on(&[1_000_000_000, 1_000_000_000], 1, &[true]);
        assert_eq!(
            monitor_of(1),
            g.served.vcpu.monitor,
            "vCPU 1's monitor data page, as its chain places it"
        );
        let windows: Calls = [(function, 3)].into();
        let mut stop = BatchStop::new(log_of(168, "bound"), 168, windows);
        assert_eq!(
            stop.check(&g),
            Ok(None),
            "the log full, the main thread short of its calls"
        );
        let mem = g.memory_view();
        mem.write_u64(monitor_of(0), 11_000_000_000);
        mem.write_u64(monitor_of(1), 11_000_000_000);
        assert_eq!(
            vcpu_counts(&g),
            [11_000_000_000, 11_000_000_000],
            "every vCPU's count, asked on vCPU 1"
        );
        assert_eq!(
            stop.check(&g),
            Ok(Some(
                "the log holds the reference's 168 lines; the main thread short of 1 functions' calls 20000000000 \
                 instructions later"
                    .to_string()
            )),
            "asked on vCPU 1, 10,000,000,000 instructions past the log on each of two vCPUs"
        );
        g.start.vcpus = 1;
        assert_eq!(vcpu_counts(&g), [11_000_000_000], "one vCPU: vCPU 0's count alone");
    }

    /// A record read in flight whose slot the trace gives back (`call_trace.c` 131-134) is not read again (Sprint 367):
    /// with the log full, the main thread's record 0 returned and record 1 in flight, the stop holds 1 to read again;
    /// once the trace's count falls to 1, its slot given back, the next check reads no record past the count, where it
    /// stopped the run on "trace record 1: the trace holds 1".
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    #[test]
    fn a_record_given_back_is_not_read_again() {
        use super::BatchStop;
        use guest_runtime::layout::{MONITOR_SIZE, TRACE_OFF};
        let (g, function) = guest_on(&[1_000_000_000], 0, &[true, false]);
        let windows: Calls = [(function, 3)].into();
        let mut stop = BatchStop::new(log_of(168, "given-back"), 168, windows);
        assert_eq!(stop.check(&g), Ok(None), "record 1 in flight");
        g.memory_view().write_u64(MONITOR_SIZE + TRACE_OFF + 16, 1);
        assert_eq!(stop.check(&g), Ok(None), "record 1's slot given back");
    }
}
