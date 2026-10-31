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
/// The cells at which the Python's own runs differ (Sprint 194)
const VARYING: &str = "store/batch_run/stream-varying-2026-10-28.tsv";
/// The frozen host's batch run run again on 2026-10-28 (`scripts/run_frontier.py --scratch --max-steps
/// 1000000000000`), each 10,505 rows and equal to `STREAM` but in the varying cells (Sprint 207)
const REPEATS: [&str; 3] = [
    "store/frontier-calls/frontier-643a6283510c-1793247683722736000.tsv",
    "store/frontier-calls/frontier-643a6283510c-1793247683722772000.tsv",
    "store/frontier-calls/frontier-643a6283510c-1793247803946454000.tsv",
];
/// Guests whose output is a clock's reading, 16 hex digits and a newline
const CLOCK_GUESTS: [&str; 2] = ["qpc", "qpc2"];

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

/// The run outputs whose stamps stand for `name` (Sprint 288): the game's stop file for the files its run writes
/// beside it, each guest's stop for `guests.tsv`, which `gates.sh` gathers from them.
fn stamped_for(root: &Path, name: &str) -> Vec<PathBuf> {
    let dir = root.join("build/batch_run");
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
    vec![dir.join("first-stop.tsv")]
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
    for (row, (r, p)) in ours.iter().zip(&python).enumerate() {
        assert_eq!(
            cells(row, r, &ours_names),
            cells(row, p, &python_names),
            "row {row} of the call stream differs from the Python batch run's"
        );
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
    let reference = fs::read_to_string(root.join("store/game-runs/38216081440/unity.log")).unwrap();
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

/// Each guest that ends at ExitProcess prints what the frozen Python host's run printed (Sprint 191).
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
        if !rust_stop.starts_with("ExitProcess") {
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
