// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! The batch run against Windows (Sprint 214; census row 7). Two judgements, both against the reference set the frozen
//! host's run of 2026-10-21 names (game run 38216081440 and its ten laps):
//!
//! - the port: the frozen host's whole stream gives the verdict its record holds, "conforms", no divergence that is a
//!   defect, and the 24 values Windows varies its comparison counted (docs/postmortem-2026-10-21.md 56);
//! - the run: the Rust host's stream, as far as it reaches, has the divergences the frozen host's stream has over the
//!   same rows, and no defect past the cut a stopped run makes (`call_compare::verdict`).

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;
use w2m_vm::call_compare::{
    Call, Reference, Signature, Verdict, contract_rules, load_reference, read_strings, read_tsv, reference_runs,
    signatures, verdict,
};

/// The frozen host's recorded run of 2026-10-21 (`batch_run.rs`'s STREAM) and its string arguments
const STREAM: &str = "store/frontier-calls/frontier-643a6283510c-1792623423346201000";
/// That run's record, whose RunStarted baseline names the reference set
const RECORD: &str = "store/runs/frontier-643a6283510c-1792623423346201000/events-000001.open.jsonl";
/// The values Windows varies over the frozen host's stream of 2026-10-21 (docs/postmortem-2026-10-21.md 56)
const VARIES: usize = 24;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

struct Set {
    references: Vec<Reference>,
    sigs: BTreeMap<String, Signature>,
    undecided: BTreeSet<(String, String)>,
}

fn set(root: &Path) -> Set {
    let runs = reference_runs(&fs::read_to_string(root.join(RECORD)).unwrap()).unwrap();
    let contract: Value =
        serde_json::from_str(&fs::read_to_string(root.join("contracts/windows.game.calls/0.1.0.json")).unwrap())
            .unwrap();
    let references = runs
        .iter()
        .map(|r| load_reference(&root.join("store/game-runs").join(r), &contract).unwrap())
        .collect();
    let sigs = signatures(&fs::read_to_string(root.join("specs/signatures.json")).unwrap()).unwrap();
    Set {
        references,
        sigs,
        undecided: contract_rules(&contract, "").1,
    }
}

fn judge(s: &Set, calls: &[Call], strings: &str, stopped: bool) -> Verdict {
    verdict(
        &s.references,
        calls,
        &s.sigs,
        &read_strings(strings),
        &s.undecided,
        stopped,
    )
}

/// Every divergence a verdict holds, the scheduling ones too: a stopped run's paired threads end early, so their tails
/// read as scheduling, where a defect that stopped a thread early would also land (reviewer-3's Medium 2 of its review
/// of Sprints 212-214); held whole to the frozen host's same rows, none can hide
fn texts(v: &Verdict) -> Vec<String> {
    v.defects
        .iter()
        .chain(&v.past_cut)
        .chain(&v.scheduling)
        .map(|d| d.text())
        .collect()
}

#[test]
fn the_frozen_hosts_stream_conforms_to_windows_as_its_record_says() {
    let root = root();
    let s = set(&root);
    let calls = read_tsv(&fs::read_to_string(root.join(format!("{STREAM}.tsv"))).unwrap()).unwrap();
    let strings = fs::read_to_string(root.join(format!("{STREAM}.strings.tsv"))).unwrap();
    let v = judge(&s, &calls, &strings, false);
    println!(
        "the frozen host's {} calls: {}, {} defects, {} scheduling, {} values Windows varies",
        calls.len(),
        v.verdict(),
        v.defects.len(),
        v.scheduling.len(),
        v.varies.len()
    );
    assert!(
        v.defects.is_empty(),
        "the frozen host's stream against Windows:\n{:#?}",
        texts(&v)
    );
    // a capture thread left in, or a varying value held fixed, moves the count
    assert_eq!(
        v.varies.len(),
        VARIES,
        "values Windows varies over the frozen host's stream"
    );
}

#[test]
fn the_rust_hosts_stream_conforms_to_windows_as_far_as_it_runs() {
    let root = root();
    let Ok(ours) = fs::read_to_string(root.join("build/batch_run/first-stop.calls.tsv")) else {
        let what = "no build/batch_run/first-stop.calls.tsv (scripts/gates.sh writes it)";
        if std::env::var("W2M_REQUIRE_CASE_RUNS").as_deref() == Ok("1") {
            panic!("{what}, and W2M_REQUIRE_CASE_RUNS=1 requires it");
        }
        eprintln!("{what}: the Rust host's run is not compared here");
        return;
    };
    let s = set(&root);
    let stopped = !fs::read_to_string(root.join("build/batch_run/first-stop.tsv"))
        .unwrap()
        .starts_with("stop\tExitProcess");
    let ours = read_tsv(&ours).unwrap();
    let our_strings = fs::read_to_string(root.join("build/batch_run/first-stop.calls.strings.tsv")).unwrap();
    let v = judge(&s, &ours, &our_strings, stopped);
    // the frozen host's stream over the same rows
    let theirs: Vec<Call> = read_tsv(&fs::read_to_string(root.join(format!("{STREAM}.tsv"))).unwrap())
        .unwrap()
        .into_iter()
        .take(ours.len())
        .collect();
    let last = theirs.last().map_or(0, |c| c.seq);
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
    let w = judge(&s, &theirs, &their_strings, stopped);
    println!(
        "the Rust host's {} calls: {}, {} defects, {} past the cut ({} for the frozen host's same rows), {} \
         scheduling, {} values Windows varies",
        ours.len(),
        v.verdict(),
        v.defects.len(),
        v.past_cut.len(),
        w.past_cut.len(),
        v.scheduling.len(),
        v.varies.len()
    );
    let (ours_all, theirs_all) = (texts(&v), texts(&w));
    let only_ours: Vec<&String> = ours_all.iter().filter(|d| !theirs_all.contains(d)).collect();
    let only_theirs: Vec<&String> = theirs_all.iter().filter(|d| !ours_all.contains(d)).collect();
    assert!(
        only_ours.is_empty() && only_theirs.is_empty(),
        "divergences only the Rust host shows: {only_ours:#?}\nonly the frozen host's same rows show: {only_theirs:#?}"
    );
    assert!(
        v.defects.is_empty(),
        "the Rust host's stream against Windows:\n{:#?}",
        v.defects.iter().map(|d| d.text()).collect::<Vec<_>>()
    );
}
