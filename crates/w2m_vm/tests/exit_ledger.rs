// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! The exit ledger's judge (Sprint 206; reviewer-2's High 1 and Medium 1 of its review of Sprints 199-205): the frozen
//! Python host's batch run and the Rust host's, each process's host exits to the game's first MAP_VIEW 29 (to mono's
//! load before Sprint 207, which loads mono on the Rust host), compared row by row. The
//! frozen host's ledger is a fixture (`build/exit_ledger/python.tsv` and a child's `.<pid>` beside it, pinned by
//! `store/exit_ledger/python.sha256`; WORKING_AGREEMENT gives its generator); `scripts/gates.sh` writes the Rust host's
//! (`w2m-vm --exit-ledger build/exit_ledger/rust.tsv`). Every differing (process, row, column) is held to
//! `store/exit_ledger/differences-2026-10-29.tsv` (Sprint 209), so a change in where the hosts part fails by name until
//! the file is rewritten with the card that moved it. From Sprint 272 the rows are aligned first: a Rust host compile
//! (`hvc 1`) the frozen host did not take, its next row meeting the frozen row, is held as an extra by its site, the C
//! compiler declining a memory XCHG to the host, and the rows after it compare from where they meet again.
//!
//! The last row is the stop, the game's first MAP_VIEW request, which the fixture records as it arrives, before the
//! request's answer exists: it is compared without its site and answer columns. From Sprint 211 the Rust host serves
//! MAP_VIEW and runs on: the game's rows past the fixture's stop are not held, and the stop row is compared without
//! the columns read when the guest resumes (the executed count, the warp, the process area), until the fixture runs
//! further.

use std::fs;
use std::path::{Path, PathBuf};

const HELD: &str = "store/exit_ledger/differences-2026-10-29.tsv";
const PINS: &str = "store/exit_ledger/python.sha256";
const COLUMNS: [&str; 14] = [
    "seq", "kind", "site", "executed", "warp", "op", "a0", "a1", "a2", "a3", "second", "result", "written", "process",
];
/// The stop row's columns that the fixture cannot know: the site, the second, the result and the written ranges.
const UNKNOWN_AT_STOP: [usize; 4] = [2, 10, 11, 12];
/// The columns a row takes when the guest resumes after it (`Ledger::resume`): the executed count, the warp, the
/// process area's sha256. The fixture's stop row reads them at its stop; a Rust run that goes past that stop (Sprint
/// 211, MAP_VIEW served) reads them at its next exit, so they too are unknown there.
const READ_AT_RESUME: [usize; 3] = [3, 4, 13];
/// The most host compiles in a row the alignment takes for extras (Sprint 272): a declined XCHG and the block cut before
/// it, with room; a longer run is a parting, named cell by cell.
const EXTRA_RUN: usize = 8;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn rows(path: &Path) -> Vec<Vec<String>> {
    fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("{}: {e}", path.display()))
        .lines()
        .map(|l| l.split('\t').map(str::to_string).collect())
        .collect()
}

/// Every cell at which `rust` departs from `python`, one line of the held file each (process, row, column, the frozen
/// host's value, the Rust host's, the row's kind), and each row only one side has, by its kind (process, row,
/// "missing" or "extra", its kind): every row of every column judged, so a difference past a held one fails (Sprint
/// 209; reviewer-2's High 1 of its review of 206-209: the held file had named each column's first difference only).
fn differences(process: &str, python: &[Vec<String>], rust: &[Vec<String>]) -> Vec<String> {
    let last = python.len().saturating_sub(1);
    // the Rust run went past the fixture's stop: its rows beyond are not held (the Decision "Sequence after Sprint 210":
    // the ledger stays at the first MAP_VIEW until the fixture runs further), nor the stop row's resume columns
    let past = rust.len() > python.len();
    let cell = |row: &Vec<String>, c: usize| row.get(c).map_or("-", |s| s).to_string();
    // the differing cells of the frozen host's row i against a Rust row `r`; a column read at resume (the executed count,
    // the warp, the process area) matches `r`'s or that of any extra compile after it, `also`, since where an extra
    // exit falls moves which row carries the reading (`count` is the count the next exit finds, Sprint 258); `at_exit`
    // alone compares only the columns written at the exit itself
    let differing = |i: usize, p: &Vec<String>, r: &Vec<String>, also: &[Vec<String>], at_exit: bool| -> Vec<String> {
        COLUMNS
            .iter()
            .enumerate()
            .skip(1)
            .filter(|(c, _)| !(at_exit && READ_AT_RESUME.contains(c)))
            .filter_map(|(c, column)| {
                let unknown = i == last && (UNKNOWN_AT_STOP.contains(&c) || (past && READ_AT_RESUME.contains(&c)));
                let candidates = if READ_AT_RESUME.contains(&c) { also } else { &[][..] };
                let equal = p.get(c) == r.get(c) || candidates.iter().any(|x| p.get(c) == x.get(c));
                (!unknown && p.get(c).is_none_or(|v| v != "*") && !equal).then(|| {
                    format!(
                        "{process}\t{i}\t{column}\t{}\t{}\t{}",
                        cell(p, c),
                        cell(r, c),
                        cell(p, 1)
                    )
                })
            })
            .collect()
    };
    // the alignment (Sprint 272): a Rust `hvc 1` whose next row meets the frozen host's row at the exit, where the
    // `hvc 1` itself does not, is an extra compile, held by its site; the C compiler declines a memory XCHG to the host,
    // which the frozen host's C compiled itself
    let mut matched: Vec<(usize, usize, usize)> = Vec::new();
    let mut out = Vec::new();
    let (mut i, mut j) = (0, 0);
    while i < python.len() && j < rust.len() {
        let p = &python[i];
        let meets = |n: &Vec<String>| differing(i, p, n, &[], true).is_empty();
        // a run of at most `EXTRA_RUN` Rust `hvc 1` rows, the row after it meeting the frozen row: each an extra (a block
        // cut before a declined XCHG compiles at the host as well as the XCHG's own)
        let run = rust[j..]
            .iter()
            .take(EXTRA_RUN)
            .take_while(|n| cell(n, 1) == "hvc 1" && !meets(n))
            .count();
        if run > 0 && rust.get(j + run).is_some_and(meets) {
            for extra in &rust[j..j + run] {
                out.push(format!("{process}\t{i}\textra\thvc 1\t{}", cell(extra, 2)));
            }
            j += run;
            if let Some(m) = matched.last_mut() {
                m.2 = j; // the extras after the matched row, rust[m.1 + 1..m.2]
            }
            continue;
        }
        matched.push((i, j, j + 1));
        i += 1;
        j += 1;
    }
    for (i, j, extras_end) in matched {
        out.extend(differing(i, &python[i], &rust[j], &rust[j + 1..extras_end], false));
    }
    out.sort_by_key(|l| l.split('\t').nth(1).and_then(|n| n.parse::<usize>().ok()).unwrap_or(0));
    for (i, p) in python.iter().enumerate().skip(i) {
        out.push(format!("{process}\t{i}\tmissing\t{}", cell(p, 1)));
    }
    // rows past the fixture's end are not held for any process: the fixture's run stops every process when the game
    // reaches its first MAP_VIEW (Sprint 213: the child, UnityCrashHandler64, ran past its row 190 once the game did;
    // Sprint 211's rule had held a child's extra rows, taking its fixture's last row for the child's own stop)
    out
}

/// The fixture's ledgers by process ("main", or the child's pid), each held to its pinned sha256: the first run's,
/// with the cells where the second run (`python-repeat.tsv`) differs set to "*", the varying cells (the review's rule:
/// a cell varies only where the Python's own runs differ; FILE_STAT's written times, which reading a file moves).
fn fixture(root: &Path) -> Option<Vec<(String, Vec<Vec<String>>)>> {
    let pins = fs::read_to_string(root.join(PINS)).unwrap();
    let mut files = std::collections::BTreeMap::new();
    for line in pins.lines().filter(|l| !l.starts_with('#')) {
        let (sha, name) = line.split_once("  ").unwrap();
        let path = root.join("build/exit_ledger").join(name);
        let Ok(bytes) = fs::read(&path) else {
            return None;
        };
        assert_eq!(
            guest_runtime::sha256::hex(&bytes),
            sha,
            "{} is not the pinned frozen-host fixture",
            path.display()
        );
        files.insert(name.to_string(), rows(&path));
    }
    let mut out = Vec::new();
    for (name, first) in &files {
        let Some(suffix) = name.strip_prefix("python.tsv") else {
            continue;
        };
        let repeat = &files[&format!("python-repeat.tsv{suffix}")];
        assert_eq!(
            first.len(),
            repeat.len(),
            "the two frozen-host runs of {name} differ in length"
        );
        let marked = first
            .iter()
            .zip(repeat)
            .map(|(a, b)| {
                a.iter()
                    .zip(b)
                    .map(|(x, y)| if x == y { x.clone() } else { "*".into() })
                    .collect()
            })
            .collect();
        out.push((suffix.strip_prefix('.').unwrap_or("main").to_string(), marked));
    }
    Some(out)
}

#[test]
fn each_process_parts_from_the_frozen_host_where_held() {
    let root = root();
    // the fixture and the Rust ledger exist where the gates made them (macOS, the depot); elsewhere nothing to judge
    let Some(processes) = fixture(&root) else {
        return;
    };
    // the Rust ledger of other sources is not judged (Sprint 288): its stamp stands for its children's ledgers too
    if !w2m_vm::built_from::fresh(&root, &root.join("build/exit_ledger/rust.tsv")) {
        return;
    }
    if !root.join("build/exit_ledger/rust.tsv").is_file() {
        return;
    }
    let mut found = Vec::new();
    for (process, python) in processes {
        let rust = if process == "main" {
            root.join("build/exit_ledger/rust.tsv")
        } else {
            root.join(format!("build/exit_ledger/rust.tsv.{process}"))
        };
        found.extend(differences(&process, &python, &rows(&rust)));
    }
    println!("{}", found.join("\n"));
    let held: Vec<String> = fs::read_to_string(root.join(HELD))
        .unwrap()
        .lines()
        .filter(|l| !l.starts_with('#'))
        .map(str::to_string)
        .collect();
    assert_eq!(
        found, held,
        "where the Rust host parts from the frozen host moved from {HELD}"
    );
}

#[test]
fn a_difference_is_named_by_its_row_and_column() {
    let row = |s: &str| s.split('\t').map(str::to_string).collect::<Vec<_>>();
    let python = vec![
        row("0\thvc 1\t10\t27\t0\t-\t-\t-\t-\t-\t-\t-\t-\taa"),
        row("1\thvc 1\t30\t29\t0\t-\t-\t-\t-\t-\t-\t-\t-\taa"),
        row("2\thvc 8\t20\t30\t0\tf\t1\t2\t3\t4\t0\t0\t-\tbb"),
    ];
    let mut rust = python.clone();
    let found = |rust: &[Vec<String>]| differences("main", &python, rust);
    assert!(found(&rust).is_empty());
    // every differing row is named, a second difference in the same column too
    rust[0][2] = "11".into();
    rust[1][2] = "31".into();
    rust[1][13] = "cc".into();
    assert_eq!(
        found(&rust),
        [
            "main\t0\tsite\t10\t11\thvc 1",
            "main\t1\tsite\t30\t31\thvc 1",
            "main\t1\tprocess\taa\tcc\thvc 1"
        ]
    );
    // the stop row's answer is not compared, its executed count is when the Rust run stops there too; a short ledger
    // names its missing row by its kind
    let mut rust = python.clone();
    rust[2][11] = "10".into();
    assert!(found(&rust).is_empty());
    rust[2][3] = "35".into();
    assert_eq!(found(&rust), ["main\t2\texecuted\t30\t35\thvc 8"]);
    rust[2][3] = "30".into();
    rust.pop();
    assert_eq!(found(&rust), ["main\t2\tmissing\thvc 8"]);
    // a Rust run past the fixture's stop (Sprint 211): its later rows and the stop row's resume columns (executed,
    // warp, process) are not held, a difference before the stop still is, and its other columns at the stop too
    let mut rust = python.clone();
    rust.push(row("3\thvc 1\t40\t31\t0\t-\t-\t-\t-\t-\t-\t-\t-\tdd"));
    rust[2][3] = "35".into();
    rust[2][4] = "1".into();
    rust[2][13] = "ee".into();
    assert!(found(&rust).is_empty(), "{:?}", found(&rust));
    rust[1][3] = "28".into();
    rust[2][6] = "9".into();
    assert_eq!(
        found(&rust),
        ["main\t1\texecuted\t29\t28\thvc 1", "main\t2\ta0\t1\t9\thvc 8"]
    );
    // a child past the fixture's end is held as the game is: its rows before the end, not after (Sprint 213)
    let mut child = python.clone();
    child.push(row("3\thvc 1\t40\t31\t0\t-\t-\t-\t-\t-\t-\t-\t-\tdd"));
    assert!(differences("4100", &python, &child).is_empty());
    child[0][2] = "12".into();
    assert_eq!(differences("4100", &python, &child), ["4100\t0\tsite\t10\t12\thvc 1"]);
}

/// Sprint 272: a Rust `hvc 1` the frozen host did not take is an extra compile, held by its site, alone or in a run,
/// and the rows after it compare from where they meet again; a frozen row's resume reading may fall on an extra after
/// its match. An `hvc 1` whose next row does not meet the frozen row is a parting, named cell by cell.
#[test]
fn an_extra_compile_is_held_by_its_site_and_the_rows_meet_again() {
    let row = |s: &str| s.split('\t').map(str::to_string).collect::<Vec<_>>();
    let python = vec![
        row("0\thvc 1\t10\t27\t0\t-\t-\t-\t-\t-\t-\t-\t-\taa"),
        row("1\thvc 8\t20\t30\t0\tf\t1\t2\t3\t4\t0\t0\t-\tbb"),
        row("2\thvc 1\t30\t40\t0\t-\t-\t-\t-\t-\t-\t-\t-\tcc"),
        // the stop, whose site the fixture cannot know; the extras below come before row 2, not at the stop
        row("3\thvc 8\t70\t41\t0\tf\t1\t2\t3\t4\t0\t0\t-\tdd"),
    ];
    let one = vec![
        python[0].clone(),
        row("1\thvc 1\t50\t28\t0\t-\t-\t-\t-\t-\t-\t-\t-\taa"),
        python[1].clone(),
        python[2].clone(),
        python[3].clone(),
    ];
    assert_eq!(differences("main", &python, &one), ["main\t1\textra\thvc 1\t50"]);
    // two in a run, and the frozen row 1's executed count read on the extra after its match
    let mut two = vec![python[0].clone(), python[1].clone()];
    two[1][3] = "29".into();
    two.push(row("2\thvc 1\t60\t30\t0\t-\t-\t-\t-\t-\t-\t-\t-\tbb"));
    two.push(row("3\thvc 1\t61\t30\t0\t-\t-\t-\t-\t-\t-\t-\t-\tbb"));
    two.push(python[2].clone());
    two.push(python[3].clone());
    assert_eq!(
        differences("main", &python, &two),
        ["main\t2\textra\thvc 1\t60", "main\t2\textra\thvc 1\t61"]
    );
    // an hvc 1 whose next row does not meet the frozen row: a parting, named cell by cell from its row, not an extra
    let mut parting = one.clone();
    parting[2][2] = "21".into();
    let found = differences("main", &python, &parting);
    assert_eq!(
        found.first().map(String::as_str),
        Some("main\t1\tkind\thvc 8\thvc 1\thvc 8"),
        "{found:?}"
    );
}
