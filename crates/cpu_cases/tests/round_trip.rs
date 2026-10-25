// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! Sprint 144's judge: every committed case file and fault extract read and written back byte for byte, each case file
//! the one its probe runs recorded, every downloaded result file that matches its run's hash read whole, and
//! `undefined_memory` refusing the cases the frozen Python refuses.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use cpu_cases::{
    read_cases, read_fault_results, read_results, recorded_sha256, sha256, undefined_memory, write_cases,
    write_fault_results,
};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

fn case_files() -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = fs::read_dir(root().join("probes"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.file_name().unwrap().to_string_lossy().starts_with("semantics"))
        .map(|p| p.join("cases.bin"))
        .filter(|p| p.exists())
        .collect();
    out.sort();
    out
}

/// Each probe run's directory under `store/probe-runs/` with its `cases.sha256`.
fn runs_by_cases_hash() -> BTreeMap<String, Vec<PathBuf>> {
    let mut out: BTreeMap<String, Vec<PathBuf>> = BTreeMap::new();
    for e in fs::read_dir(root().join("store/probe-runs")).unwrap() {
        let dir = e.unwrap().path();
        if let Ok(sums) = fs::read_to_string(dir.join("cases.sha256"))
            && let Some(h) = recorded_sha256(&sums, "cases.bin")
        {
            out.entry(h).or_default().push(dir);
        }
    }
    out
}

/// Every case file reads and writes back byte for byte, and is the file a probe run recorded.
#[test]
fn case_files_round_trip_and_match_their_runs() {
    let runs = runs_by_cases_hash();
    let files = case_files();
    assert_eq!(files.len(), 21);
    let mut total = 0;
    for path in &files {
        let raw = fs::read(path).unwrap();
        let (fmt, cases) = read_cases(&raw).unwrap();
        let back = write_cases(&cases, fmt);
        let first = raw.iter().zip(&back).position(|(a, b)| a != b);
        assert!(
            back == raw,
            "{}: the round trip differs at byte {first:?} (case {:?})",
            path.display(),
            first.map(|b| (b.saturating_sub(12)) / cpu_cases::case_size(fmt))
        );
        let h = sha256(&raw);
        let ran = runs.get(&h).map_or(0, Vec::len);
        assert!(ran > 0, "{}: no probe run recorded sha256 {h}", path.display());
        println!(
            "{}: format {fmt}, {} cases, run by {ran} probe runs",
            path.display(),
            cases.len()
        );
        total += cases.len();
    }
    println!("{total} cases in {} files", files.len());
}

/// Every committed fault extract reads and writes back byte for byte (vm_oracle 70-83).
#[test]
fn fault_extracts_round_trip() {
    let mut n = 0;
    for e in fs::read_dir(root().join("store/probe-runs")).unwrap() {
        let path = e.unwrap().path().join("result-1.faults.bin");
        let Ok(raw) = fs::read(&path) else { continue };
        let (fmt, faults) = read_fault_results(&raw).unwrap();
        assert_eq!(write_fault_results(&faults), raw, "{}", path.display());
        assert!(
            faults.iter().all(|(_, r)| r.marker == 2),
            "{}: a fault extract holds only faults",
            path.display()
        );
        println!("{}: format {fmt}, {} faults", path.display(), faults.len());
        n += 1;
    }
    assert_eq!(n, 2, "the committed fault extracts");
}

/// Every result file a run downloaded (build/probe-runs, untracked) that matches the run's results.sha256 reads whole,
/// one result per case of the set the run ran.
#[test]
fn downloaded_results_match_their_sets() {
    let cases: BTreeMap<String, (u32, usize)> = case_files()
        .iter()
        .map(|p| {
            let raw = fs::read(p).unwrap();
            let (fmt, cs) = read_cases(&raw).unwrap();
            (sha256(&raw), (fmt, cs.len()))
        })
        .collect();
    let (mut read, mut stale) = (0, 0);
    let Ok(dirs) = fs::read_dir(root().join("build/probe-runs")) else {
        println!("no downloaded results here (build/probe-runs absent)");
        return;
    };
    for e in dirs {
        let dir = e.unwrap().path();
        let run = dir.file_name().unwrap().to_string_lossy().to_string();
        let store = root().join("store/probe-runs").join(&run);
        let (Ok(raw), Ok(sums), Ok(case_sums)) = (
            fs::read(dir.join("result-1.bin")),
            fs::read_to_string(store.join("results.sha256")),
            fs::read_to_string(store.join("cases.sha256")),
        ) else {
            continue;
        };
        if recorded_sha256(&sums, "result-1.bin") != Some(sha256(&raw)) {
            stale += 1; // a download that is not the recorded file: vm_oracle.result_file fetches it again
            continue;
        }
        let Some(&(fmt, count)) = recorded_sha256(&case_sums, "cases.bin").and_then(|h| cases.get(&h)) else {
            continue;
        };
        let results = read_results(&raw, fmt).unwrap();
        assert_eq!(results.len(), count, "run {run}: one result per case");
        read += 1;
    }
    println!("{read} downloaded result files read whole, {stale} not the recorded file");
}

/// `undefined_memory`'s refusals per set equal the frozen Python's (`semantics_differential.undefined_memory` run over
/// the committed files, 2026-10-24): 43 in set 044, 11 in set 047, none elsewhere.
#[test]
fn undefined_memory_refuses_what_the_python_refused() {
    let expected: BTreeMap<&str, usize> = [("semantics_044", 43), ("semantics_047", 11)].into();
    for path in case_files() {
        let set = path
            .parent()
            .unwrap()
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string();
        let (_, cases) = read_cases(&fs::read(&path).unwrap()).unwrap();
        let refused = cases.iter().filter(|c| undefined_memory(c).is_some()).count();
        assert_eq!(refused, expected.get(set.as_str()).copied().unwrap_or(0), "{set}");
    }
}
