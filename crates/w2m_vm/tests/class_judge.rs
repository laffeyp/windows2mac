// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! The class judge's population (Sprint 222; census row 3; plan step 4, "456 of 458 classes"). `scripts/class_run.py`
//! judged each class of `store/runs/class-runs.json` on the cases of every reference set that covers it, each case
//! labelled by its decoded class (`_built`, 70-82; `cases_for`, 94-103). Those sets are the 21 under `probes/semantics*`,
//! so a class's cases are every case of the 21 files that decodes to it.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

/// Each decoded class's case count over the 21 reference sets' case files, or None where some are absent: 13 are
/// tracked and 8 built on this Mac, and CI, which runs no Python, does not build them (Decision 2026-10-29, "No Python
/// runs"); the gates require them (`W2M_REQUIRE_CASE_RUNS=1`).
fn counts(root: &Path) -> Option<BTreeMap<String, usize>> {
    let mut sets: Vec<PathBuf> = fs::read_dir(root.join("probes"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.file_name().unwrap().to_string_lossy().starts_with("semantics"))
        .map(|p| p.join("cases.bin"))
        .filter(|p| p.exists())
        .collect();
    sets.sort();
    if sets.len() != 21 {
        let what = format!("{} of the 21 reference sets' case files", sets.len());
        if std::env::var("W2M_REQUIRE_CASE_RUNS").as_deref() == Ok("1") {
            panic!("{what}, and W2M_REQUIRE_CASE_RUNS=1 requires all");
        }
        eprintln!("{what}: the case counts are not judged here");
        return None;
    }
    let mut out = BTreeMap::new();
    for set in sets {
        let (_, cases) = cpu_cases::read_cases(&fs::read(&set).unwrap()).unwrap();
        for c in cases {
            let iclass = x86_decoder::decode(&c.code, None).map_or_else(|e| format!("decode: {e}"), |d| d.iclass);
            *out.entry(iclass).or_default() += 1;
        }
    }
    Some(out)
}

/// Every class the frozen index judged has, in the 21 case files, the case count the index records.
#[test]
fn every_indexed_class_has_its_frozen_case_count() {
    let root = root();
    let index: Value =
        serde_json::from_str(&fs::read_to_string(root.join("store/runs/class-runs.json")).unwrap()).unwrap();
    let Some(counts) = counts(&root) else { return };
    let mut differ = Vec::new();
    for entry in index.as_array().unwrap() {
        let iclass = entry["iclass"].as_str().unwrap();
        let want = entry["cases"].as_u64().unwrap() as usize;
        let got = counts.get(iclass).copied().unwrap_or(0);
        if got != want {
            differ.push(format!("{iclass}: {got} cases decoded, the index {want}"));
        }
    }
    assert!(differ.is_empty(), "{} of 458 classes differ: {differ:?}", differ.len());
}

const RUST_INDEX: &str = "store/case_runner/class-runs-2026-10-29.json";
const HELD: &str = "store/case_runner/class-held-2026-10-29.tsv";

fn index(root: &Path, path: &str) -> BTreeMap<String, Value> {
    let v: Value = serde_json::from_str(&fs::read_to_string(root.join(path)).unwrap()).unwrap();
    v.as_array()
        .unwrap()
        .iter()
        .map(|e| (e["iclass"].as_str().unwrap().to_string(), e.clone()))
        .collect()
}

/// The Rust class judge's index holds every class the frozen index judged, with its case count and, but for the
/// classes `HELD` names with what each waits on, its `managed_vm` verdict (plan step 4, "456 of 458 classes"). A held
/// class whose verdict comes to equal the frozen one fails until the card that moved it rewrites the file.
#[test]
fn the_rust_index_is_the_frozen_one_or_held() {
    let root = root();
    let frozen = index(&root, "store/runs/class-runs.json");
    let rust = index(&root, RUST_INDEX);
    let held: BTreeMap<String, String> = fs::read_to_string(root.join(HELD))
        .unwrap()
        .lines()
        .filter(|l| !l.starts_with('#'))
        .filter_map(|l| {
            let mut c = l.splitn(3, '\t');
            Some((c.next()?.to_string(), c.next()?.to_string()))
        })
        .collect();
    let mut faults = Vec::new();
    for (iclass, f) in &frozen {
        let Some(r) = rust.get(iclass) else {
            faults.push(format!("{iclass}: not in {RUST_INDEX}"));
            continue;
        };
        if r["cases"] != f["cases"] {
            faults.push(format!(
                "{iclass}: {} cases, the frozen index {}",
                r["cases"], f["cases"]
            ));
        }
        let (got, want) = (
            r["verdicts"]["managed_vm"].as_str().unwrap(),
            f["verdicts"]["managed_vm"].as_str().unwrap(),
        );
        match held.get(iclass) {
            Some(_) if got == want => faults.push(format!("{iclass}: {got} as the frozen index, and {HELD} holds it")),
            Some(h) if h != got => faults.push(format!("{iclass}: {got}, and {HELD} holds {h}")),
            None if got != want => faults.push(format!("{iclass}: {got}, the frozen index {want}")),
            _ => {}
        }
    }
    for iclass in rust.keys().chain(held.keys()) {
        if !frozen.contains_key(iclass) {
            faults.push(format!("{iclass}: not in the frozen index"));
        }
    }
    // the judges' ledger opens an entry for a measured difference only: every entry the class judge reported names a
    // class whose verdict is a discrepancy, never an inconclusive one (reviewer-3's High 1 of its review at 222)
    let ledger: Value =
        serde_json::from_str(&fs::read_to_string(root.join("store/case_runner/discrepancy-ledger.json")).unwrap())
            .unwrap();
    for (id, entry) in ledger.as_object().unwrap() {
        let reports = entry["reports"].as_array().unwrap();
        if !reports.iter().any(|r| r["judge"] == "class-judge") {
            continue;
        }
        let unit = entry["unit"].as_str().unwrap();
        let verdict = rust.get(unit).map(|r| r["verdicts"]["managed_vm"].clone());
        assert_eq!(
            verdict,
            Some(Value::from("discrepancy")),
            "ledger entry {id} names {unit}, whose Rust verdict is {verdict:?}"
        );
    }
    let conform: Vec<&Value> = rust
        .values()
        .filter(|r| r["verdicts"]["managed_vm"] == "conforms")
        .collect();
    // a conforming class judged on words the Rust translator made for every case it judged, the rest on the frozen
    // Python lowering's in some case (reviewer-3's Medium 1)
    let python: Vec<&str> = conform
        .iter()
        .filter(|r| r["rust_compiled"] == 0)
        .map(|r| r["iclass"].as_str().unwrap())
        .collect();
    println!(
        "{} of {} classes conform on Rust, {} on no Rust-compiled case ({python:?}); {} held",
        conform.len(),
        frozen.len(),
        python.len(),
        held.len()
    );
    // the faults were collected and never asserted from Sprint 222 to 235, so no class could fail this test; found
    // when 17 held classes came to conform and the test still passed (Sprint 235)
    assert!(faults.is_empty(), "{}", faults.join("\n"));
}

/// The stored Rust index is what `class_judge --index` gives on the record it names.
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
#[test]
fn the_stored_rust_index_is_its_records() {
    let root = root();
    let stored = fs::read_to_string(root.join(RUST_INDEX)).unwrap();
    let rust = index(&root, RUST_INDEX);
    let record = rust.values().next().unwrap()["record"].as_str().unwrap().to_string();
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_class_judge"))
        .args(["--index", &record])
        .current_dir(&root)
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    assert_eq!(
        stored,
        String::from_utf8(out.stdout).unwrap(),
        "{RUST_INDEX} is not its record's index"
    );
}

/// No judge reads the frozen Python lowering's words (Sprint 238): the case runner's harnesses' baselines name the
/// Rust translator alone, and so do the baseline and the candidates of the record the Rust index is written from.
#[test]
fn the_class_index_and_the_case_harnesses_name_no_python_words() {
    let root = root();
    let named = |what: &str, text: &str| {
        for word in ["Python", "python-words"] {
            assert!(!text.contains(word), "{what} names the frozen Python lowering's words: {text}");
        }
    };
    for harness in ["class-judge.toml", "case-runs-rust.toml", "case-runs-planted.toml"] {
        let text = fs::read_to_string(root.join("harness").join(harness)).unwrap();
        let line = text.lines().find(|l| l.starts_with("implementation")).unwrap();
        named(&format!("harness/{harness}'s baseline"), line);
    }
    let rust = index(&root, RUST_INDEX);
    let record = root.join(rust.values().next().unwrap()["record"].as_str().unwrap());
    let mut events = 0;
    for entry in fs::read_dir(&record).unwrap() {
        let path = entry.unwrap().path();
        if !path.file_name().unwrap().to_string_lossy().starts_with("events-") {
            continue;
        }
        for line in fs::read_to_string(&path).unwrap().lines() {
            let e: Value = serde_json::from_str(line).unwrap();
            if let Some(b) = e.pointer("/payload/baseline/implementation") {
                named(&format!("{}'s baseline", record.display()), b.as_str().unwrap());
                events += 1;
            }
            if let Some(g) = e.pointer("/payload/generator_ref") {
                named(&format!("{}'s candidate", record.display()), g.as_str().unwrap());
                events += 1;
            }
        }
    }
    assert!(events > 456, "{} holds {events} baselines and candidates", record.display());
}
