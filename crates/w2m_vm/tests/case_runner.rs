// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! Sprint 145's judge: the CPU cases run in a Rust VM, one instruction per block, each completed case judged against
//! the Intel run Windows recorded for its set; from Sprint 147, each case Windows ended in a fault too, at the VM's
//! fault through the guest's exception records; from Sprint 148, on the whole recorded state.
//!
//! Sprint 149 binds it (reviewer-2's link-3 review): the instruction's words come only from the frozen Python
//! lowering's fixture, pinned by its header and its committed sha256 (`w2m_vm::case_inputs`); a refusal fails; each
//! run's counts equal `store/case_runner/expected.tsv`; and with `W2M_REQUIRE_CASE_RUNS=1` (`scripts/gates.sh`) a
//! missing input fails rather than skips. CI (Linux) does not compile this test: it needs Hypervisor.framework and the
//! downloaded results. The test also reports the rule modules the judged cases reach, link 3's revisit measure. The
//! same verdicts go on a record through `case_runs` (Sprint 150).
#![cfg(all(target_os = "macos", target_arch = "aarch64"))]

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use cpu_cases::{read_cases, read_results};
use w2m_vm::case_inputs::{Counts, Fixture, Hybrid, expected, registry, runs, with_case_vm};
use w2m_vm::cases::run_cases;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

/// A missing input: a failure under `W2M_REQUIRE_CASE_RUNS=1`, else a skip said on stderr.
fn missing(what: &str) {
    if std::env::var("W2M_REQUIRE_CASE_RUNS").as_deref() == Ok("1") {
        panic!("{what}, and W2M_REQUIRE_CASE_RUNS=1 requires the case runs");
    }
    eprintln!("{what}: the case runner is not run here");
}

/// The Python words for every case.
#[test]
fn every_judged_case_agrees_with_windows() {
    judge(false);
}

/// Sprint 151: the Rust translator for the classes link 5's groups have carried, the Python words for the rest. The
/// same counts and zero differences, and the Rust words equal the Python's for every code the Rust compiled.
#[test]
fn every_judged_case_agrees_with_windows_with_the_rust_translator() {
    judge(true);
}

/// One VM a process (`hv_vm_create`): the two tests run on parallel threads, so each holds this while its VM lives
/// (the first gates run of Sprint 151 failed the second with HV_BUSY, -85377016).
static ONE_VM: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn judge(use_rust: bool) {
    let _one = ONE_VM.lock().unwrap_or_else(|e| e.into_inner());
    let root = root();
    let runs = runs(&root);
    if runs.is_empty() {
        return missing("no downloaded Intel results in build/probe-runs");
    }
    let Some(fixture) = Fixture::load(&root) else {
        return missing("no Python words fixture (build/case_runner/python-words.txt)");
    };
    let fixture = fixture.unwrap();
    println!(
        "fixture: {} codes, made at src {} (base 0x{:x}, exit 0x{:x})",
        fixture.words.len(),
        fixture.header.src,
        fixture.header.base,
        fixture.header.exit
    );
    let hybrid = Hybrid::new(&fixture);
    let compile = |code: &[u8], address: u64, base: u64, exit_va: u64| {
        if use_rust {
            hybrid.compile(code, address, base, exit_va)
        } else {
            fixture.compile(code, address, base, exit_va)
        }
    };
    let expected = expected(&root);

    let (failures, reached, distinct) = with_case_vm(&root, &fixture, |guest, vcpu| {
        let mut failures = Vec::new();
        let mut reached: BTreeSet<String> = BTreeSet::new();
        let mut seen_sets = BTreeSet::new();
        let mut distinct = (0, 0);
        for run in &runs {
            let (fmt, cases) = read_cases(&fs::read(&run.set).unwrap()).unwrap();
            let results = read_results(&run.result, fmt).unwrap();
            let t = run_cases(guest, vcpu, &cases, &results, &compile).unwrap();
            let set_name = run.set_name();
            let refused: usize = t.refused.values().sum();
            println!(
                "run {} ({set_name}): {} cases; judged {} ({:.0}%); Windows faults {} ({} judged at the VM's fault); \
                 model {}; refused {refused}; undefined memory {}; {} difference classes; {:.1} s",
                run.id,
                cases.len(),
                t.judged,
                100.0 * t.judged as f64 / cases.len() as f64,
                t.windows_faults,
                t.faults_judged,
                t.model,
                t.undefined_memory,
                t.differences.len(),
                t.seconds
            );
            println!("    host requests by op: {:?}", t.host_requests);
            println!(
                "    judged with YMM upper halves {}, with x87 state {}",
                t.ymm_judged, t.x87_judged
            );
            for (k, (n, example)) in t.differences.iter().take(12) {
                println!("    {n:6} {k}: {example}");
            }
            if !t.differences.is_empty() {
                failures.push(format!("run {}: {} difference classes", run.id, t.differences.len()));
            }
            if refused != 0 {
                failures.push(format!("run {}: {refused} refused: {:?}", run.id, t.refused));
            }
            let got = Counts {
                set: set_name.clone(),
                cases: cases.len(),
                judged: t.judged,
                windows_faults: t.windows_faults,
                faults_judged: t.faults_judged,
                model: t.model,
                undefined_memory: t.undefined_memory,
            };
            match expected.get(&run.id) {
                Some(want) if *want == got => {}
                Some(want) => failures.push(format!("run {}: counts {got:?}, expected {want:?}", run.id)),
                None => failures.push(format!("run {}: not in store/case_runner/expected.tsv", run.id)),
            }
            reached.extend(t.by_class.keys().cloned());
            if seen_sets.insert(set_name) {
                distinct.0 += t.judged;
                distinct.1 += cases.len();
            }
        }
        (failures, reached, distinct)
    })
    .unwrap();
    let mut failures = failures;
    for run in expected.keys() {
        if !runs.iter().any(|r| &r.id == run) {
            failures.push(format!("run {run}: expected, but its result is not downloaded"));
        }
    }
    println!(
        "distinct cases (each set once): {} judged of {}",
        distinct.0, distinct.1
    );

    // link 3's revisit measure: the rule modules the iclasses of the cases judged whole reach, through the frozen
    // registry's map (store/rules/registry.tsv)
    let registry = registry(&root);
    let modules: BTreeSet<&String> = registry.values().collect();
    let hit: BTreeSet<&String> = reached.iter().filter_map(|ic| registry.get(ic)).collect();
    let unmapped: Vec<&String> = reached.iter().filter(|ic| !registry.contains_key(*ic)).collect();
    println!(
        "rule modules reached by the judged cases: {} of {} ({} iclasses judged; without a rule module: {unmapped:?})",
        hit.len(),
        modules.len(),
        reached.len()
    );
    if use_rust {
        let rust = hybrid.rust.borrow();
        let unlike = hybrid.unlike.borrow();
        println!(
            "the Rust translator compiled {} cases in {} classes: {rust:?}",
            rust.values().sum::<usize>(),
            rust.len()
        );
        for (code, class, d) in unlike.iter().take(12) {
            println!("    {class} {code:02x?}: {d}");
        }
        if rust.is_empty() {
            failures.push("the Rust translator compiled no case".into());
        }
        if !unlike.is_empty() {
            failures.push(format!(
                "{} cases whose Rust words differ from the Python's",
                unlike.len()
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
