// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! The batch run against Windows (Sprint 214; census row 7). Two judgements, both against the reference set the frozen
//! host's run of 2026-10-21 names (game run 38216081440 and its ten laps):
//!
//! - the port: the frozen host's whole stream gives the verdict its record holds, "conforms", no divergence that is a
//!   defect, and the 24 values Windows varies its comparison counted (docs/postmortem-2026-10-21.md 56);
//! - the run: the Rust host's stream, as far as it reaches, has the divergences the frozen host's stream has over the
//!   same rows, and no defect past the cut a stopped run makes (`call_compare::verdict`).

use std::fs;
use std::path::{Path, PathBuf};

use call_compare::{Stream, Verdict, Windows};

/// The frozen host's recorded run of 2026-10-21 (`batch_run.rs`'s STREAM) and its string arguments
const STREAM: &str = "store/frontier-calls/frontier-643a6283510c-1792623423346201000";
/// The values Windows varies over the frozen host's stream of 2026-10-21 (docs/postmortem-2026-10-21.md 56)
const VARIES: usize = 24;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

/// A trace's calls, its non-empty lines
fn calls(text: &str) -> Vec<&str> {
    text.lines().filter(|l| !l.is_empty()).collect()
}

/// Every divergence a verdict holds, the scheduling ones too: a stopped run's paired threads end early, so their tails
/// read as scheduling, where a defect that stopped a thread early would also land (reviewer-3's Medium 2 of its review
/// of Sprints 212-214); held whole to the frozen host's same rows, none can hide
fn texts(v: &Verdict) -> Vec<String> {
    v.defects()
        .iter()
        .chain(v.past_cut())
        .chain(v.scheduling())
        .map(|d| d.text())
        .collect()
}

#[test]
fn the_frozen_hosts_stream_conforms_to_windows_as_its_record_says() {
    let root = root();
    let windows = Windows::load(&root).unwrap();
    let trace = fs::read_to_string(root.join(format!("{STREAM}.tsv"))).unwrap();
    let strings = fs::read_to_string(root.join(format!("{STREAM}.strings.tsv"))).unwrap();
    let stream = |calls, strings, stopped| Stream {
        calls,
        strings,
        stopped,
        dropped: 0,
    };
    let v = windows.judge(&stream(&trace, &strings, false)).unwrap();
    assert_eq!((v.verdict().as_str(), v.first_divergence()), ("conforms", None));
    let left_out = windows
        .judge(&Stream {
            dropped: 3,
            ..stream(&trace, &strings, false)
        })
        .unwrap();
    assert_eq!(
        (left_out.verdict().as_str(), left_out.first_divergence().as_deref()),
        ("inconclusive", Some("the trace left out 3 calls")),
        "a trace that left calls out cannot show the calls conform"
    );
    println!(
        "the frozen host's {} calls: {}, {} defects, {} scheduling, {} values Windows varies",
        calls(&trace).len(),
        v.verdict().as_str(),
        v.defects().len(),
        v.scheduling().len(),
        v.varies().len()
    );
    assert!(
        v.defects().is_empty(),
        "the frozen host's stream against Windows:\n{:#?}",
        texts(&v)
    );
    // a capture thread left in, or a varying value held fixed, moves the count
    assert_eq!(
        v.varies().len(),
        VARIES,
        "values Windows varies over the frozen host's stream"
    );
}

#[test]
fn the_rust_hosts_stream_conforms_to_windows_as_far_as_it_runs() {
    let root = root();
    // a stream of other sources is not judged (Sprint 288): the stop file's stamp stands for its run's stream
    if !w2m_vm::built_from::fresh(&root, &root.join("build/batch_run/first-stop.tsv")) {
        return;
    }
    let Ok(ours) = fs::read_to_string(root.join("build/batch_run/first-stop.calls.tsv")) else {
        let what = "no build/batch_run/first-stop.calls.tsv (scripts/gates.sh writes it)";
        if std::env::var("W2M_REQUIRE_CASE_RUNS").as_deref() == Ok("1") {
            panic!("{what}, and W2M_REQUIRE_CASE_RUNS=1 requires it");
        }
        eprintln!("{what}: the Rust host's run is not compared here");
        return;
    };
    let windows = Windows::load(&root).unwrap();
    let stopped = !fs::read_to_string(root.join("build/batch_run/first-stop.tsv"))
        .unwrap()
        .starts_with("stop\tExitProcess");
    let our_strings = fs::read_to_string(root.join("build/batch_run/first-stop.calls.strings.tsv")).unwrap();
    let stream = |calls, strings| Stream {
        calls,
        strings,
        stopped,
        dropped: 0,
    };
    let v = windows.judge(&stream(&ours, &our_strings)).unwrap();
    let ours = calls(&ours);
    // the frozen host's stream over the same rows
    let trace = fs::read_to_string(root.join(format!("{STREAM}.tsv"))).unwrap();
    let theirs: Vec<&str> = calls(&trace).into_iter().take(ours.len()).collect();
    let last: u64 = theirs
        .last()
        .and_then(|l| l.split('\t').next())
        .map_or(0, |s| s.parse().unwrap());
    let theirs: String = theirs.iter().map(|l| format!("{l}\n")).collect();
    let their_strings: String = fs::read_to_string(root.join(format!("{STREAM}.strings.tsv")))
        .unwrap()
        .lines()
        .filter(|l| {
            l.split('\t')
                .next()
                .and_then(|s| s.parse::<u64>().ok())
                .is_some_and(|s| s <= last)
        })
        .map(|l| format!("{l}\n"))
        .collect();
    let w = windows.judge(&stream(&theirs, &their_strings)).unwrap();
    println!(
        "the Rust host's {} calls: {}, {} defects, {} past the cut ({} for the frozen host's same rows), {} \
         scheduling, {} values Windows varies",
        ours.len(),
        v.verdict().as_str(),
        v.defects().len(),
        v.past_cut().len(),
        w.past_cut().len(),
        v.scheduling().len(),
        v.varies().len()
    );
    let (ours_all, theirs_all) = (texts(&v), texts(&w));
    let only_ours: Vec<&String> = ours_all.iter().filter(|d| !theirs_all.contains(d)).collect();
    let only_theirs: Vec<&String> = theirs_all.iter().filter(|d| !ours_all.contains(d)).collect();
    assert!(
        only_ours.is_empty() && only_theirs.is_empty(),
        "divergences only the Rust host shows: {only_ours:#?}\nonly the frozen host's same rows show: {only_theirs:#?}"
    );
    assert!(
        v.defects().is_empty(),
        "the Rust host's stream against Windows:\n{:#?}",
        v.defects().iter().map(|d| d.text()).collect::<Vec<_>>()
    );
}

/// The main-thread calls `w2m-vm --game` stops by are the ones `call_compare` derives from the ending run (Sprint 219):
/// the stored file cannot drift from the reference it is read from.
#[test]
fn the_stored_main_thread_calls_are_the_ending_runs() {
    let root = root();
    let stored: Vec<(String, String, u64)> =
        fs::read_to_string(root.join("store/batch_run/main-thread-calls-2026-10-29.tsv"))
            .unwrap()
            .lines()
            .filter(|l| !l.starts_with('#'))
            .map(|l| {
                let c: Vec<&str> = l.split('\t').collect();
                (c[0].to_string(), c[1].to_string(), c[2].parse().unwrap())
            })
            .collect();
    let derived = call_compare::main_thread_calls(&root).unwrap();
    assert_eq!(stored, derived);
    assert_eq!(
        derived.iter().map(|r| r.2).sum::<u64>(),
        2999,
        "the Windows main thread's calls"
    );
}
