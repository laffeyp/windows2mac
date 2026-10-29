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
//! Rust host's call stream equals the Python's batch run's (`STREAM`) row for row up to the stop, every column, and
//! each guest that ends at ExitProcess prints what the frozen Python host's run printed (the held file's fifth column),
//! but `qpc` and `qpc2`, which print a clock's reading, held by its form.

use std::fs;
use std::path::{Path, PathBuf};

const HELD: &str = "store/batch_run/first-stop-2026-10-28.tsv";
/// The Python batch run's calls, in `scripts/call_compare.py` `write_tsv`'s form (Sprint 108's frontier run)
const STREAM: &str = "store/frontier-calls/frontier-643a6283510c-1792267359516238000.tsv";
/// The cells at which the Python's own runs differ (Sprint 194)
const VARYING: &str = "store/batch_run/stream-varying-2026-10-28.tsv";
/// The Python batch run repeated on the same code (2026-10-28), each 10,516 rows
const REPEATS: [&str; 3] = [
    "store/frontier-calls/frontier-643a6283510c-1792269281973223000.tsv",
    "store/frontier-calls/frontier-643a6283510c-1792273674526334000.tsv",
    "store/frontier-calls/frontier-643a6283510c-1792300672353112000.tsv",
];
/// Guests whose output is a clock's reading, 16 hex digits and a newline
const CLOCK_GUESTS: [&str; 2] = ["qpc", "qpc2"];

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

/// A file `gates.sh` writes under build/batch_run/; None when it has not run here (and the gates do not require it).
fn written(root: &Path, name: &str) -> Option<String> {
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
    // a time limit is a hang, never an expected stop (Sprint 199; reviewer-2's Medium 3 of its review of 196-198)
    for l in &lines {
        assert!(
            !l.contains("the run's time limit"),
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
/// in flight at the stop is the Python's next row but its return and error.
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
    assert!(ours.len() < python.len(), "the Rust stream runs past the Python's");
    for (row, (r, p)) in ours.iter().zip(&python).enumerate() {
        let cells = |l: &'_ str| -> Vec<String> {
            l.split('\t')
                .enumerate()
                .map(|(c, x)| {
                    if skip.contains(&(row, c)) {
                        "*".to_string()
                    } else {
                        x.to_string()
                    }
                })
                .collect()
        };
        assert_eq!(
            cells(r),
            cells(p),
            "row {row} of the call stream differs from the Python batch run's"
        );
    }
    let in_flight = written(&root, "first-stop.in-flight.tsv").unwrap_or_default();
    let Some(first) = in_flight.lines().next() else {
        panic!("no call in flight at the stop");
    };
    let row = ours.len();
    let p: Vec<&str> = python[row].split('\t').collect();
    assert_eq!(
        first.split('\t').collect::<Vec<_>>(),
        p[..13],
        "row {row}, the call in flight at the stop, differs from the Python batch run's"
    );
    println!(
        "the Rust stream equals the Python's rows 0-{}, and row {row} is in flight",
        row - 1
    );
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
