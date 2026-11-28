// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! The batch run on the Rust host to its first stop, and each tests/data guest's stop (Sprint 186; reviewer-2's High 1
//! of its review of Sprints 182-184: no card had run the Rust host on the batch run). `scripts/gates.sh` runs `w2m-vm
//! --game` on the batch run's depot and `w2m-vm` on each guest, writing under build/batch_run/ the stop, the call
//! stream and each guest's stop and output; this test holds them to `store/batch_run/first-stop-2026-10-28.tsv`, so a
//! change that moves the first stop, or a guest's, fails here until the held file is rewritten with the card that moved
//! it. The run's seconds are not held.
//!
//! From Sprint 191 (reviewer-2's High 1 and Medium 4 of its review of 185-189) it also holds what the run answered: the
//! Rust host's call stream equals the Python's batch run's (`STREAM`) row for row up to the stop, every column (a call's
//! arguments, return value and last error only: what a host writes through pointers and the clock it keeps are not in
//! the stream, and `tests/exit_ledger.rs` holds them from Sprint 206; reviewer-2's Medium 1 of 199-205), and
//! each guest that ends at ExitProcess prints what the frozen Python host's run printed (the held file's fifth column),
//! but `qpc` and `qpc2`, which print a clock's reading, held by its form.
//!
//! From Sprint 382 the frozen stream is the reference to `LAST_HELD_ROW` alone, and the Rust host's own one-vCPU run's
//! stream, stored at Sprint 381's closing tree (`RUST_STREAM`), is the reference in every row: the run's stream equals
//! it row for row and cell for cell but the varying cells, and a card that changes the stream regenerates it.

use std::fs;
use std::path::{Path, PathBuf};

use guest_runtime::AddressNames;
use guest_runtime::layout::{RUNTIME_IMAGE_SIZE, RUNTIME_IMAGE_VA, runtime_defines};

const HELD: &str = "store/batch_run/first-stop-2026-10-28.tsv";
/// The Python batch run's calls, in `scripts/call_compare.py` `write_tsv`'s form: the frozen host's recorded run of
/// 2026-10-21 (store/runs/frontier-643a6283510c-1792623423346201000, whose COMPARISON_COMPLETED against the Windows
/// references has no divergence; 10,505 rows). Sprint 207: the stream held before, Sprint 108's run of 2026-10-17,
/// predates Sprint 120's catch-up of a child at its start (2026-10-18), and its mono cookie (row 939) was
/// 0x13fc8e8cff34 where the frozen host gives 0x165d8e8cc734.
const STREAM: &str = "store/frontier-calls/frontier-643a6283510c-1792623423346201000.tsv";
/// The cells at which the Python's own runs differ (Sprint 194); the Rust host's runs differ there too, at rows 426 and
/// 438, columns 9-11 or 9-12 (Sprint 382, each run measured against the stored one), so the stored Rust stream is held
/// outside them as well
const VARYING: &str = "store/batch_run/stream-varying-2026-10-28.tsv";
/// The frozen host's batch run run again on 2026-10-28 (`scripts/run_frontier.py --scratch --max-steps
/// 1000000000000`), each 10,505 rows and equal to `STREAM` but in the varying cells (Sprint 207)
const REPEATS: [&str; 3] = [
    "store/frontier-calls/frontier-643a6283510c-1793247683722736000.tsv",
    "store/frontier-calls/frontier-643a6283510c-1793247683722772000.tsv",
    "store/frontier-calls/frontier-643a6283510c-1793247803946454000.tsv",
];
/// The first call of `STREAM` whose answer a card re-took from the laptop and which moved the game (Sprint 379):
/// GetWindowsDirectoryW, now `C:\windows` where the frozen host answered `C:\Windows`. The frozen stream is the
/// reference row for row up to and including it; past it the rows are judged against Windows alone
/// (`crates/call_compare/tests/call_compare.rs`), as past the stream's end. Measured (Sprint 379's result): with the
/// directory reverted every row matches; with it re-taken the first difference past it is in argument slots past the
/// function's parameters in specs/signatures.json (ResumeThread's a7, WaitForSingleObjectEx's a6, ReleaseSemaphore's
/// a7 and a4), and the calls' order changes from row 6881; the answers re-taken earlier in the stream
/// (GlobalMemoryStatusEx's total, DllGetVersion's revision, RtlVerifyVersionInfo's product type) leave every row up
/// to it equal. That cut held rows 0-6364 to Sprint 380.
///
/// From Sprint 381 the cut is a row, `LAST_HELD_ROW`: the profile re-taken, C:\Users\w2m where the frozen host gave the
/// runners' C:\Users\runneradmin, changes what the game is given before any traced call, its start environment's
/// USERPROFILE, which ucrtbase's environment copy (untraced) reads after row 28. Measured (Sprint 381's result, each
/// change reverted in place, one batch run at a time): with USERPROFILE re-taken the first difference is row 29,
/// EnterCriticalSection's a3, a register past its one parameter holding a heap block 0x20 lower (0x200009d20 for
/// 0x200009d40); with it reverted and the known folder path re-taken, row 967, CoTaskMemFree's a2 after
/// SHGetKnownFolderPath (row 966); with both reverted and the temp fallback and the system path re-taken, every row
/// matches. The frozen stream is the reference to row 28; past it the rows are judged against Windows alone.
///
/// From Sprint 382 every row, the cut's included, is also held to `RUST_STREAM`.
const LAST_HELD_ROW: usize = 28;
/// The Rust host's one-vCPU batch run's call stream, `build/batch_run/first-stop.calls.tsv` from `scripts/gates.sh`'s
/// batch-run step at the tree of 758f9f900, Sprint 381's close (Sprint 382; the Decision "The runner's strings
/// re-taken, and the frozen host's reference after them", its revisit): 10,515 rows after its `#` header, whose lines
/// name the run each regeneration came from. Another run differs from it only in the varying cells.
const RUST_STREAM: &str = "store/batch_run/first-stop-calls-2026-11-28.tsv";
/// The calls `STREAM` holds at `LAST_HELD_ROW` and after it, by which the row is checked to be the one measured.
const AT_THE_CUT: [&str; 2] = ["LeaveCriticalSection", "EnterCriticalSection"];
/// Guests whose output is a clock's reading, 16 hex digits and a newline (`two_counters`' the ticks its two counters
/// took, Sprint 354)
const CLOCK_GUESTS: [&str; 3] = ["qpc", "qpc2", "two_counters"];
/// The game's stop file on one vCPU, and on two (Sprint 368: `scripts/batch-run-record.sh --vcpus 2` names its files
/// so), under build/batch_run/
const STOP: &str = "first-stop.tsv";
const STOP_TWO_VCPUS: &str = "first-stop-2-vcpus.tsv";

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

/// The run outputs whose stamps stand for `name` (Sprint 288): the game's stop file for the files its run writes
/// beside it under its name, each guest's stop for `guests.tsv`, which `gates.sh` gathers from them, and any other
/// file, a guest's output among them, its own.
fn stamped_for(root: &Path, name: &str) -> Vec<PathBuf> {
    let dir = root.join("build/batch_run");
    // the one-vCPU run's stop and the two-vCPU run's (Sprint 368), each for its own companions
    for stop in [STOP, STOP_TWO_VCPUS] {
        if name == stop
            || w2m_vm::run_outputs::STOP_COMPANIONS
                .iter()
                .any(|s| name == w2m_vm::run_outputs::companion(stop, s))
        {
            return vec![dir.join(stop)];
        }
    }
    if name == "guests.tsv" {
        let mut stops: Vec<PathBuf> = fs::read_dir(&dir)
            .into_iter()
            .flatten()
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.file_name().is_some_and(|n| n.to_string_lossy().starts_with("guest-")))
            .filter(|p| p.extension().is_some_and(|e| e == "stop"))
            .collect();
        stops.sort();
        return stops;
    }
    vec![dir.join(name)]
}

/// A file `gates.sh` writes under build/batch_run/; None when it has not run here, or ran on other sources than the
/// work tree's (Sprint 288), and the gates do not require it.
fn written(root: &Path, name: &str) -> Option<String> {
    if !stamped_for(root, name)
        .iter()
        .all(|f| w2m_vm::built_from::fresh(root, f))
    {
        return None;
    }
    match fs::read_to_string(root.join("build/batch_run").join(name)) {
        Ok(text) => Some(text),
        Err(_) => {
            let what = format!("no build/batch_run/{name} (scripts/gates.sh writes it)");
            if std::env::var("W2M_REQUIRE_CASE_RUNS").as_deref() == Ok("1") {
                panic!("{what}, and W2M_REQUIRE_CASE_RUNS=1 requires it");
            }
            eprintln!("{what}: the batch run's stop is not held here");
            None
        }
    }
}

/// The held file's lines but comments, a guest's line cut to (guest, name, Rust stop).
fn held(root: &Path) -> (Vec<String>, Vec<Vec<String>>) {
    let text = fs::read_to_string(root.join(HELD)).unwrap();
    let mut lines = Vec::new();
    let mut guests = Vec::new();
    for l in text.lines().filter(|l| !l.starts_with('#')) {
        if l.starts_with("guest\t") {
            let cols: Vec<String> = l.split('\t').map(str::to_string).collect();
            lines.push(cols[..3].join("\t"));
            guests.push(cols);
        } else {
            lines.push(l.to_string());
        }
    }
    (lines, guests)
}

#[test]
fn the_batch_run_and_the_guests_stop_where_held() {
    let root = root();
    let (Some(game), Some(guests)) = (written(&root, "first-stop.tsv"), written(&root, "guests.tsv")) else {
        return;
    };
    let mut ours: Vec<String> = game
        .lines()
        .filter(|l| !l.starts_with("seconds\t"))
        .map(str::to_string)
        .collect();
    ours.extend(guests.lines().map(|l| format!("guest\t{l}")));
    let (lines, _) = held(&root);
    // a time limit is a hang, never an expected stop (Sprint 199; reviewer-2's Medium 3 of its review of 196-198); and
    // once the batch run ends where its reference did (Sprint 219), a step-budget stop is a game that never filled its
    // log, not an end (reviewer-3's Concern 3 of its review of Sprint 218)
    for l in &lines {
        assert!(
            !l.contains("the run's time limit") && !l.starts_with("stop\tStepLimit"),
            "{HELD} holds a hang as a stop: {l}"
        );
    }
    println!("{}", ours.join("\n"));
    assert_eq!(
        ours, lines,
        "the batch run's first stop or a guest's stop moved from {HELD}"
    );
}

/// The (row, column) cells at which the Python's own runs of the same code differ (Sprint 194; reviewer-2's Medium 2
/// of its review of 190-192): GetFileAttributesExW's a4-a7 at rows 426 and 438, a FILETIME the call leaves on the stack.
fn varying(root: &Path) -> Vec<(usize, usize)> {
    fs::read_to_string(root.join(VARYING))
        .unwrap()
        .lines()
        .skip(1)
        .map(|l| {
            let (row, column) = l.split_once('\t').unwrap();
            (row.parse().unwrap(), column.parse().unwrap())
        })
        .collect()
}

/// The varying list is the cells at which the repeat runs differ from `STREAM`, every one and no other.
#[test]
fn the_varying_cells_are_where_the_python_runs_differ() {
    let root = root();
    let reference = fs::read_to_string(root.join(STREAM)).unwrap();
    let mut differ = Vec::new();
    for run in REPEATS {
        let other = fs::read_to_string(root.join(run)).unwrap();
        assert_eq!(
            other.lines().count(),
            reference.lines().count(),
            "{run} has another row count"
        );
        for (row, (a, b)) in reference.lines().zip(other.lines()).enumerate() {
            for (column, (x, y)) in a.split('\t').zip(b.split('\t')).enumerate() {
                if x != y && !differ.contains(&(row, column)) {
                    differ.push((row, column));
                }
            }
        }
    }
    differ.sort();
    let held = varying(&root);
    for cell in &differ {
        assert!(held.contains(cell), "{VARYING} lacks row {} column {}", cell.0, cell.1);
    }
    for cell in &held {
        assert!(
            differ.contains(cell),
            "{VARYING} names row {} column {}, where the runs agree",
            cell.0,
            cell.1
        );
    }
}

/// The Rust run's call stream is the Python's, row for row, every cell but the varying ones, up to the stop; the call
/// in flight at the stop, when there is one, is the Python's next row but its return and error. From Sprint 218 the
/// Rust run can trace past the Python's stream, which ends where `run_frontier.py`'s harness stopped its run ("the log
/// holds the reference's 168 lines and the main thread its calls", 306); then every Python row is held, and the rows
/// past it are judged against Windows alone (`crates/call_compare/tests/call_compare.rs`).
#[test]
fn the_batch_runs_calls_are_the_python_runs_up_to_the_stop() {
    let root = root();
    let Some(calls) = written(&root, "first-stop.calls.tsv") else {
        return;
    };
    let stream = fs::read_to_string(root.join(STREAM)).unwrap();
    let skip = varying(&root);
    let ours: Vec<&str> = calls.lines().collect();
    let python: Vec<&str> = stream.lines().take(ours.len() + 1).collect();
    assert!(!ours.is_empty(), "the Rust run traced no call");
    let past = ours.len() >= python.len();
    // an address in the runtime image read by what it is, `name+0xoffset`, the Rust run's by the image it ran and the
    // Python's by the frozen image, so a C change that moves the image's code compares equal where the code it names
    // is the same (Sprint 272; reviewer-3's Medium 1 at 272: the C runtime changes in step 6, and row 2535 holds
    // `create_step`'s address in a DefWindowProcW argument)
    let names = |image: &guest_runtime::Image| AddressNames::new(image, RUNTIME_IMAGE_VA, RUNTIME_IMAGE_SIZE);
    let (_, ran) = guest_runtime::build(&root, &runtime_defines(), RUNTIME_IMAGE_VA).unwrap();
    let (_, frozen) = guest_runtime::frozen_python_image(&root, &runtime_defines(), RUNTIME_IMAGE_VA).unwrap();
    let (ours_names, python_names) = (names(&ran), names(&frozen));
    let cells = |row: usize, l: &str, names: &AddressNames| -> Vec<String> {
        l.split('\t')
            .enumerate()
            .map(|(c, x)| {
                if skip.contains(&(row, c)) {
                    "*".to_string()
                } else {
                    u64::from_str_radix(x, 16)
                        .ok()
                        .and_then(|a| names.name(a))
                        .unwrap_or_else(|| x.to_string())
                }
            })
            .collect()
    };
    // the cut: the last row before the first answer re-taken from the laptop reaches the game (Sprint 381; Sprint 379's
    // was a call found by name), its two calls the stream's
    let cut = LAST_HELD_ROW;
    let at: Vec<&str> = stream
        .lines()
        .skip(cut)
        .take(2)
        .map(|l| l.split('\t').nth(4).unwrap_or(""))
        .collect();
    assert_eq!(
        at,
        AT_THE_CUT,
        "{STREAM}'s rows {cut} and {} are not the calls the cut was measured at",
        cut + 1
    );
    for (row, (r, p)) in ours.iter().zip(&python).enumerate().take(cut + 1) {
        assert_eq!(
            cells(row, r, &ours_names),
            cells(row, p, &python_names),
            "row {row} of the call stream differs from the Python batch run's"
        );
    }
    if ours.len() > cut {
        println!(
            "the Rust stream equals the Python's rows 0-{cut} of {}, to the start environment re-taken from the laptop \
             ({:.1}% held); its {} rows past it are held to {RUST_STREAM} and judged against Windows",
            stream.lines().count(),
            100.0 * (cut + 1) as f64 / stream.lines().count() as f64,
            ours.len() - cut - 1
        );
        return;
    }
    // a stop inside a traced call leaves it in flight; a stop between calls (a created thread's stack growing, Sprint
    // 211) leaves none, and the stream's rows are all that is held; checked whether or not the stream reaches the
    // Python's end (reviewer-3's Low 2 of its review of Sprint 218), and the row count is the held stop file's
    let in_flight = written(&root, "first-stop.in-flight.tsv").unwrap_or_default();
    let Some(first) = in_flight.lines().next() else {
        println!(
            "the Rust stream equals the Python's rows 0-{}{}, and no call is in flight at the stop",
            python.len().min(ours.len()) - 1,
            if ours.len() > python.len() {
                format!(" and runs {} rows past them", ours.len() - python.len())
            } else {
                String::new()
            }
        );
        return;
    };
    let row = ours.len();
    assert!(
        !past,
        "a call is in flight at row {row}, past the Python's {} rows",
        python.len()
    );
    let p = cells(row, python[row], &python_names);
    assert_eq!(
        cells(row, first, &ours_names),
        p[..13],
        "row {row}, the call in flight at the stop, differs from the Python batch run's"
    );
    println!(
        "the Rust stream equals the Python's rows 0-{}, and row {row} is in flight",
        row - 1
    );
}

/// The Rust run's call stream is the stored Rust stream, `RUST_STREAM`, row for row and cell for cell in every row, but
/// the varying cells (Sprint 382): past `LAST_HELD_ROW` the one-vCPU run's order and answers on the instruction clock
/// were held by Windows' judge alone, G3, which tolerates scheduling. A cell is compared as written, an address in the
/// runtime image included: the stored stream ran its own tree's image, and a card that moves the image changes the
/// stream and regenerates the file. A difference fails naming the row, the column and the call.
#[test]
fn the_batch_runs_calls_are_the_stored_rust_runs_in_every_row() {
    let root = root();
    let Some(calls) = written(&root, "first-stop.calls.tsv") else {
        return;
    };
    let stored = fs::read_to_string(root.join(RUST_STREAM)).unwrap();
    let skip = varying(&root);
    let ours: Vec<&str> = calls.lines().collect();
    let held: Vec<&str> = stored.lines().filter(|l| !l.starts_with('#')).collect();
    assert!(!held.is_empty(), "{RUST_STREAM} holds no row");
    for row in 0..ours.len().max(held.len()) {
        let (Some(r), Some(h)) = (ours.get(row), held.get(row)) else {
            panic!(
                "row {row} of the call stream: the run's stream has {} rows and {RUST_STREAM} {}",
                ours.len(),
                held.len()
            );
        };
        let (r, h): (Vec<&str>, Vec<&str>) = (r.split('\t').collect(), h.split('\t').collect());
        let cell = |cols: &[&str], c: usize| cols.get(c).copied().unwrap_or("(no cell)").to_string();
        for column in 0..r.len().max(h.len()) {
            if skip.contains(&(row, column)) {
                continue;
            }
            assert!(
                r.get(column) == h.get(column),
                "row {row} column {column} of the call stream differs from {RUST_STREAM}'s: the run's {}, the stored {} \
                 (the run's call {}, the stored {})",
                cell(&r, column),
                cell(&h, column),
                cell(&r, 4),
                cell(&h, 4)
            );
        }
    }
    println!(
        "the Rust stream equals {RUST_STREAM}'s {} rows, every cell but the {} varying",
        held.len(),
        skip.len()
    );
}

/// The sprint a `# Sprint N...` header line names, None for another line.
fn header_sprint(line: &str) -> Option<u32> {
    let rest = line.strip_prefix("# Sprint ")?;
    let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
    digits.parse().ok()
}

/// The sprint the work tree's active card is (`sprints/sprint-NNN-*.md` holding the line `status: active`), the one
/// `scripts/gates.sh` books its run to; an error when none is active or more than one is.
fn active_sprint(root: &Path) -> Result<u32, String> {
    let mut active: Vec<u32> = fs::read_dir(root.join("sprints"))
        .unwrap()
        .flatten()
        .filter(|e| fs::read_to_string(e.path()).is_ok_and(|t| t.lines().any(|l| l == "status: active")))
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            name.strip_prefix("sprint-")?.get(..3)?.parse().ok()
        })
        .collect();
    active.sort();
    match active[..] {
        [n] => Ok(n),
        [] => Err("no card has status: active".to_string()),
        _ => Err(format!("cards {active:?} each have status: active")),
    }
}

fn git(root: &Path, args: &[&str]) -> std::process::Output {
    std::process::Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .unwrap()
}

/// The stored stream's regeneration rule held in code (Sprint 382; reviewer-4's fifty-seventh review): the header's
/// last `# Sprint N` line names the card that last changed the file. While the file differs from HEAD's, or HEAD does
/// not hold it, that is the active card, so a card that regenerates the stream and adds no header line fails here,
/// naming its own sprint; once committed, it is the sprint whose commit last changed the file, the "Sprint N" its
/// subject begins with. The stored stream is a change detector, not a correctness check: G3 judges the batch run
/// against Windows, and the frozen host holds the port's equivalence on rows 0-28 alone.
#[test]
fn the_stored_streams_header_names_the_sprint_that_last_changed_it() {
    let root = root();
    let text = fs::read(root.join(RUST_STREAM)).unwrap();
    let named = String::from_utf8_lossy(&text)
        .lines()
        .take_while(|l| l.starts_with('#'))
        .filter_map(header_sprint)
        .last();
    let at_head = git(&root, &["show", &format!("HEAD:{RUST_STREAM}")]);
    let changed = !at_head.status.success() || at_head.stdout != text;
    let (expected, why) = if changed {
        let n = active_sprint(&root).unwrap_or_else(|e| panic!("{RUST_STREAM} differs from HEAD's and {e}"));
        (n, "the active card, the file differing from HEAD's or not in HEAD")
    } else {
        let log = git(&root, &["log", "-1", "--format=%s", "--", RUST_STREAM]);
        let subject = String::from_utf8_lossy(&log.stdout).trim().to_string();
        let n = header_sprint(&format!("# {subject}"))
            .unwrap_or_else(|| panic!("the last commit changing {RUST_STREAM} is {subject:?}, which names no sprint"));
        (n, "the sprint of the last commit that changed it")
    };
    assert_eq!(
        named,
        Some(expected),
        "{RUST_STREAM}'s header's last \"# Sprint N\" line names {named:?}; expected Sprint {expected}, {why}"
    );
    println!("{RUST_STREAM}'s header names Sprint {expected}, {why}");
}

/// A line with each run of digits, decimal point included, as one mark: the numbers a log line carries vary by run
/// (reload and unload times), the words do not.
fn without_numbers(line: &str) -> String {
    let mut out = String::new();
    let mut in_number = false;
    for c in line.trim_end_matches('\r').chars() {
        let digit = c.is_ascii_digit() || (in_number && c == '.');
        if digit && !in_number {
            out.push('#');
        } else if !digit {
            out.push(c);
        }
        in_number = digit;
    }
    out
}

/// The batch run's unity.log, which the run ends by, equals the reference's line for line, numbers the varying cells
/// (Sprint 219; reviewer-3's High 1 of its review of Sprint 218: the log reached the reference's 168 lines and nothing
/// read it). `run_frontier.py` counts the lines only; 168 wrong lines would pass that.
#[test]
fn the_batch_runs_log_is_the_references_line_for_line() {
    let root = root();
    let Some(ours) = written(&root, "first-stop.unity.log") else {
        return;
    };
    assert_log_is_the_references(&root, &ours);
}

/// The game's batch run on two vCPUs (Sprint 368; census G3), on the wall clock, which `scripts/gates.sh` runs beside
/// the one-vCPU run: it stops at the reference's row, the held stop line (the log at the reference's 168 lines and the
/// main thread its calls), with the held images, never at a time or step limit, its stop file naming its two vCPUs, and
/// its unity.log the reference's line for line. Its exits, compiles and calls are not the one-vCPU run's and are not
/// held here: its record holds its own exits (`batch_record.rs`), and `call_compare` judges its calls against Windows.
#[test]
fn the_two_vcpu_batch_run_stops_at_the_references_row_with_its_log() {
    let root = root();
    let (Some(game), Some(log)) = (
        written(&root, STOP_TWO_VCPUS),
        written(&root, &w2m_vm::run_outputs::companion(STOP_TWO_VCPUS, ".unity.log")),
    ) else {
        return;
    };
    let row = |text: &str, name: &str| {
        text.lines()
            .find_map(|l| l.strip_prefix(&format!("{name}\t")))
            .map(str::to_string)
    };
    let (held_lines, _) = held(&root);
    let held_text = held_lines.join("\n");
    println!("{game}");
    assert_eq!(row(&game, "vcpus").as_deref(), Some("2"), "the stop file's vCPUs");
    assert_eq!(
        (row(&game, "stop"), row(&game, "images")),
        (row(&held_text, "stop"), row(&held_text, "images")),
        "the two-vCPU batch run's stop or images moved from {HELD}'s"
    );
    assert_log_is_the_references(&root, &log);
}

/// The reference's log (Sprint 219: game run 38216081440's 168 lines). From Sprint 383 a laptop game run's: with the
/// laptop's display the game keeps the laptop's 17 sizes and logs 16 "Res" lines where the runner's 13 gave 12, 180
/// lines, as nine of the ten `lap-20261014T032442034Z-c8814547` runs and the three `lap-20261128T014720644Z-33b6423b`
/// runs log them, numbers aside (-05 logs one more "New_Input - DONE" line and its file line). The run's end is left
/// as it was: the log at 168 lines or more (`--log-lines 168`) and the runner's main-thread calls.
const REFERENCE_LOG: &str = "store/game-runs/lap-20261014T032442034Z-c8814547-01/unity.log";

/// A batch run's unity.log, `ours`, equals the reference's line for line, numbers the varying cells.
fn assert_log_is_the_references(root: &Path, ours: &str) {
    let reference = fs::read_to_string(root.join(REFERENCE_LOG)).unwrap();
    let (ours, reference): (Vec<String>, Vec<String>) = (
        ours.lines().map(without_numbers).collect(),
        reference.lines().map(without_numbers).collect(),
    );
    for (n, (a, b)) in ours.iter().zip(&reference).enumerate() {
        assert_eq!(
            a,
            b,
            "line {} of the batch run's unity.log differs from the reference's",
            n + 1
        );
    }
    assert_eq!(ours.len(), reference.len(), "the batch run's unity.log lines");
}

/// Each guest that ends at ExitProcess prints what the frozen Python host's run printed (Sprint 191); a guest the frozen
/// host never ran ("-" in both its columns, from Sprint 364) is held to its stop alone here (Sprint 365:
/// `two_process_pipe_pingpong` exits 0 only when its 10,000 rounds and its child's exit code are right, and
/// `run_loop.rs` holds its output on two vCPUs).
#[test]
fn each_guest_that_ends_prints_what_the_python_host_printed() {
    let root = root();
    if written(&root, "guests.tsv").is_none() {
        return;
    }
    let (_, guests) = held(&root);
    let mut compared = 0;
    for cols in guests {
        let (name, rust_stop, python_output) = (&cols[1], &cols[2], cols[4].replace("\\n", "\n"));
        if !rust_stop.starts_with("ExitProcess") || (cols[3] == "-" && cols[4] == "-") {
            continue;
        }
        let ours = written(&root, &format!("guest-{name}.txt")).unwrap();
        if CLOCK_GUESTS.contains(&name.as_str()) {
            let digits = ours.strip_suffix('\n').unwrap_or("");
            assert!(
                digits.len() == 16 && digits.chars().all(|c| c.is_ascii_hexdigit()),
                "{name} printed {ours:?}, not a clock's 16 hex digits"
            );
        } else {
            assert_eq!(ours, python_output, "{name}'s output differs from the Python host's");
        }
        compared += 1;
    }
    println!("{compared} guests' output compared");
}
