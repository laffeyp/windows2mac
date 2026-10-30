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

/// The class gate's comparison (Sprint 244; reviewer-3's APOSD point at 235: the faults were gathered over 25 lines
/// and asserted once, and from Sprint 222 to 235 that assertion was missing): every class the frozen index judged is in
/// the Rust index with its case count and, but for the classes `held` names with what each waits on, its `managed_vm`
/// verdict; a held class whose verdict comes to equal the frozen one is a fault until its card rewrites the held file;
/// a class the frozen index lacks is one. `Ok` only when nothing differs.
fn class_faults(
    frozen: &BTreeMap<String, Value>,
    rust: &BTreeMap<String, Value>,
    held: &BTreeMap<String, String>,
) -> Result<(), Vec<String>> {
    let mut faults = Vec::new();
    for (iclass, f) in frozen {
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
    if faults.is_empty() { Ok(()) } else { Err(faults) }
}

fn held(root: &Path) -> BTreeMap<String, String> {
    fs::read_to_string(root.join(HELD))
        .unwrap()
        .lines()
        .filter(|l| !l.starts_with('#'))
        .filter_map(|l| {
            let mut c = l.splitn(3, '\t');
            Some((c.next()?.to_string(), c.next()?.to_string()))
        })
        .collect()
}

/// The Rust class judge's index holds every class the frozen index judged, as `class_faults` compares them (plan step
/// 4, "456 of 458 classes"), and the judges' ledger names a class judge's entry only for a discrepancy.
#[test]
fn the_rust_index_is_the_frozen_one_or_held() {
    let root = root();
    let frozen = index(&root, "store/runs/class-runs.json");
    let rust = index(&root, RUST_INDEX);
    let held = held(&root);
    if let Err(faults) = class_faults(&frozen, &rust, &held) {
        panic!("{} classes differ:\n{}", faults.len(), faults.join("\n"));
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
    // a conforming class with no case the Rust translator compiled (reviewer-3's Medium 1 at 222); from Sprint 238 the
    // translator compiles every case, so the list is empty unless a class is judged on no case at all
    let uncompiled: Vec<&str> = conform
        .iter()
        .filter(|r| r["rust_compiled"] == 0)
        .map(|r| r["iclass"].as_str().unwrap())
        .collect();
    println!(
        "{} of {} classes conform on Rust, {} on no Rust-compiled case ({uncompiled:?}); {} held",
        conform.len(),
        frozen.len(),
        uncompiled.len(),
        held.len()
    );
}

/// `class_faults` names each kind of fault and passes equal indexes (Sprint 244).
#[test]
fn each_kind_of_class_fault_is_named() {
    let class = |cases: u64, verdict: &str| serde_json::json!({"cases": cases, "verdicts": {"managed_vm": verdict}});
    let frozen: BTreeMap<String, Value> = [
        ("ADD".to_string(), class(10, "conforms")),
        ("FCOS".to_string(), class(5, "discrepancy")),
        ("MUL".to_string(), class(7, "conforms")),
    ]
    .into();
    assert_eq!(class_faults(&frozen, &frozen, &BTreeMap::new()), Ok(()));
    let mut rust = frozen.clone();
    rust.insert("ADD".into(), class(11, "conforms"));
    rust.insert("FCOS".into(), class(5, "conforms"));
    rust.insert("NEW".into(), class(1, "conforms"));
    let held: BTreeMap<String, String> = [("MUL".to_string(), "inconclusive".to_string())].into();
    let faults = class_faults(&frozen, &rust, &held).unwrap_err();
    assert_eq!(
        faults,
        [
            "ADD: 11 cases, the frozen index 10",
            "FCOS: conforms, the frozen index discrepancy",
            "MUL: conforms as the frozen index, and store/case_runner/class-held-2026-10-29.tsv holds it",
            "NEW: not in the frozen index",
        ]
    );
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
            assert!(
                !text.contains(word),
                "{what} names the frozen Python lowering's words: {text}"
            );
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
    assert!(
        events > 456,
        "{} holds {events} baselines and candidates",
        record.display()
    );
}
