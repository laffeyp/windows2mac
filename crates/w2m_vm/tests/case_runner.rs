// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! Sprint 145's judge: the CPU cases run in a Rust VM, one instruction per block, each completed case judged against
//! the Intel run Windows recorded for its set; from Sprint 147, each case Windows ended in a fault too, at the VM's
//! fault through the guest's exception records; from Sprint 148, on the whole recorded state.
//!
//! Sprint 149 binds it (reviewer-2's link-3 review): the instruction's words come only from the frozen Python
//! lowering's fixture, pinned by its header and its committed sha256; a refusal fails; each run's counts equal
//! `store/case_runner/expected.tsv`; and with `W2M_REQUIRE_CASE_RUNS=1` (`scripts/gates.sh`) a missing input fails
//! rather than skips. CI (Linux) does not compile this test: it needs Hypervisor.framework and the downloaded results.
//! The test also reports the rule modules the judged cases reach, link 3's revisit measure.
#![cfg(all(target_os = "macos", target_arch = "aarch64"))]

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use cpu_cases::{read_cases, read_results, recorded_sha256, sha256};
use guest_runtime::layout::{IPA_BASE, RUNTIME_IMAGE_VA, runtime_defines};
use hypervisor::{Vcpu, Vm, memory};
use w2m_vm::cases::{Compiled, case_memory, run_cases};
use w2m_vm::guest::{Guest, vm_compiler_symbols};
use w2m_vm::process::Start;

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

/// The case files by their SHA-256, and each downloaded Intel result whose hash and set are recorded: (run, set file,
/// result bytes).
fn runs() -> Vec<(String, PathBuf, Vec<u8>)> {
    let mut sets: BTreeMap<String, PathBuf> = BTreeMap::new();
    for e in fs::read_dir(root().join("probes")).unwrap() {
        let p = e.unwrap().path().join("cases.bin");
        if p.exists()
            && p.parent()
                .unwrap()
                .file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("semantics")
        {
            sets.insert(sha256(&fs::read(&p).unwrap()), p);
        }
    }
    let mut out = Vec::new();
    let Ok(dirs) = fs::read_dir(root().join("build/probe-runs")) else {
        return out;
    };
    for e in dirs {
        let dir = e.unwrap().path();
        let run = dir.file_name().unwrap().to_string_lossy().to_string();
        let store = root().join("store/probe-runs").join(&run);
        let (Ok(raw), Ok(sums), Ok(case_sums), Ok(cpu)) = (
            fs::read(dir.join("result-1.bin")),
            fs::read_to_string(store.join("results.sha256")),
            fs::read_to_string(store.join("cases.sha256")),
            fs::read_to_string(store.join("cpu.txt")),
        ) else {
            continue;
        };
        if !cpu.to_lowercase().contains("intel") || recorded_sha256(&sums, "result-1.bin") != Some(sha256(&raw)) {
            continue;
        }
        if let Some(set) = recorded_sha256(&case_sums, "cases.bin").and_then(|h| sets.get(&h)) {
            out.push((run, set.clone(), raw));
        }
    }
    out.sort();
    out
}

/// The fixture's header: `# base 0x.. exit 0x.. symbols <sha256> src <commit>`.
struct Header {
    base: u64,
    exit: u64,
    symbols: String,
    src: String,
}

fn parse_header(line: &str) -> Header {
    let p: Vec<&str> = line.split_whitespace().collect();
    assert!(
        p.len() == 9 && p[0] == "#" && p[1] == "base" && p[3] == "exit" && p[5] == "symbols" && p[7] == "src",
        "the fixture's header is not `# base .. exit .. symbols .. src ..`: {line}"
    );
    let hex = |s: &str| u64::from_str_radix(s.trim_start_matches("0x"), 16).unwrap();
    Header {
        base: hex(p[2]),
        exit: hex(p[4]),
        symbols: p[6].to_string(),
        src: p[8].to_string(),
    }
}

/// The sha256 of a symbol table as the fixture's generator writes it: `name=0x<address>` lines, sorted by name.
fn symbols_sha256(symbols: &BTreeMap<String, u64>) -> String {
    let text: String = symbols.iter().map(|(n, v)| format!("{n}=0x{v:x}\n")).collect();
    sha256(text.as_bytes())
}

/// One run's committed counts (`store/case_runner/expected.tsv`).
#[derive(Debug, PartialEq, Eq)]
struct Counts {
    set: String,
    cases: usize,
    judged: usize,
    windows_faults: usize,
    faults_judged: usize,
    model: usize,
    undefined_memory: usize,
}

fn expected() -> BTreeMap<String, Counts> {
    let text = fs::read_to_string(root().join("store/case_runner/expected.tsv")).unwrap();
    text.lines()
        .skip(1)
        .map(|l| {
            let p: Vec<&str> = l.split('\t').collect();
            let n = |i: usize| p[i].parse::<usize>().unwrap();
            (
                p[0].to_string(),
                Counts {
                    set: p[1].to_string(),
                    cases: n(2),
                    judged: n(3),
                    windows_faults: n(4),
                    faults_judged: n(5),
                    model: n(6),
                    undefined_memory: n(7),
                },
            )
        })
        .collect()
}

#[test]
fn every_judged_case_agrees_with_windows() {
    let root = root();
    let runs = runs();
    if runs.is_empty() {
        return missing("no downloaded Intel results in build/probe-runs");
    }
    let fixture_path = root.join("build/case_runner/python-words.txt");
    let Ok(text) = fs::read_to_string(&fixture_path) else {
        return missing("no Python words fixture (build/case_runner/python-words.txt)");
    };
    let pinned = fs::read_to_string(root.join("store/case_runner/python-words.sha256")).unwrap();
    assert_eq!(
        sha256(text.as_bytes()),
        pinned.trim(),
        "the fixture is not the one committed by its sha256"
    );
    let mut lines = text.lines();
    let header = parse_header(lines.next().unwrap());
    // code -> (ends, words), or None where the Python refused
    let fixture: BTreeMap<Vec<u8>, Option<(bool, Vec<u32>)>> = lines
        .map(|l| {
            let p: Vec<&str> = l.split_whitespace().collect();
            let code = (0..p[0].len())
                .step_by(2)
                .map(|i| u8::from_str_radix(&p[0][i..i + 2], 16).unwrap())
                .collect();
            let words = (p[1] == "ok").then(|| {
                (
                    p[2] == "1",
                    p[3..].iter().map(|w| u32::from_str_radix(w, 16).unwrap()).collect(),
                )
            });
            (code, words)
        })
        .collect();

    let (_, image) = guest_runtime::build(&root, &runtime_defines(), RUNTIME_IMAGE_VA).unwrap();
    // the words call the runtime's symbols: the fixture was made with this symbol table
    assert_eq!(
        symbols_sha256(&image.symbols),
        header.symbols,
        "the fixture's words were made for another runtime symbol table"
    );
    let services = guest_runtime::services::services(&guest_runtime::texts(&root).unwrap()).unwrap();
    let table = fs::read_to_string(root.join("tools/vm_compiler/stencil_table.h")).unwrap();
    let names = vm_compiler_symbols(&table).unwrap();
    let loaded = case_memory();
    let mut guest = Guest::new(
        &loaded,
        &image,
        &services,
        &Start::console("app.exe"),
        &BTreeSet::new(),
        &names,
    )
    .unwrap();
    let vm = Vm::new(40).unwrap();
    vm.map(&guest.memory, IPA_BASE, memory::READ | memory::WRITE | memory::EXEC)
        .unwrap();
    let vcpu = Vcpu::new(&vm).unwrap();
    println!(
        "fixture: {} codes, made at src {} (base 0x{:x}, exit 0x{:x})",
        fixture.len(),
        header.src,
        header.base,
        header.exit
    );

    // The frozen Python lowering's words for every case instruction, compiled for the block body and the miss stub the
    // header names (vm_backend 4611-4618): the compiler that judges the runner itself, until the Rust lowering takes
    // its place.
    let python = |code: &[u8], _address: u64, base: u64, exit_va: u64| -> Result<Compiled, String> {
        assert_eq!(
            (base, exit_va),
            (header.base, header.exit),
            "the fixture's words were made for another layout"
        );
        match fixture.get(code) {
            Some(Some((ends, words))) => Ok(Compiled {
                words: words.clone(),
                ends: *ends,
            }),
            Some(None) => Err("the Python refused it".into()),
            None => Err("not in the fixture".into()),
        }
    };

    let expected = expected();
    let mut failures = Vec::new();
    let mut reached: BTreeSet<String> = BTreeSet::new();
    let mut seen_sets = BTreeSet::new();
    let (mut distinct_judged, mut distinct_cases) = (0, 0);
    for (run, set, raw) in &runs {
        let (fmt, cases) = read_cases(&fs::read(set).unwrap()).unwrap();
        let results = read_results(raw, fmt).unwrap();
        let t = run_cases(&mut guest, &vcpu, &cases, &results, &python).unwrap();
        let set_name = set.parent().unwrap().file_name().unwrap().to_string_lossy().to_string();
        let refused: usize = t.refused.values().sum();
        println!(
            "run {run} ({set_name}): {} cases; judged {} ({:.0}%); Windows faults {} ({} judged at the VM's fault); \
             model {}; refused {refused}; undefined memory {}; {} difference classes; {:.1} s",
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
            failures.push(format!("run {run}: {} difference classes", t.differences.len()));
        }
        if refused != 0 {
            failures.push(format!("run {run}: {refused} refused: {:?}", t.refused));
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
        match expected.get(run) {
            Some(want) if *want == got => {}
            Some(want) => failures.push(format!("run {run}: counts {got:?}, expected {want:?}")),
            None => failures.push(format!("run {run}: not in store/case_runner/expected.tsv")),
        }
        reached.extend(t.by_class.keys().cloned());
        if seen_sets.insert(set_name) {
            distinct_judged += t.judged;
            distinct_cases += cases.len();
        }
    }
    for run in expected.keys() {
        if !runs.iter().any(|(r, _, _)| r == run) {
            failures.push(format!("run {run}: expected, but its result is not downloaded"));
        }
    }
    println!("distinct cases (each set once): {distinct_judged} judged of {distinct_cases}");

    // link 3's revisit measure: the rule modules the iclasses of the cases judged whole reach, through the frozen
    // registry's map (store/rules/registry.tsv)
    let registry: BTreeMap<String, String> = fs::read_to_string(root.join("store/rules/registry.tsv"))
        .unwrap()
        .lines()
        .map(|l| {
            let (ic, m) = l.split_once('\t').unwrap();
            (ic.to_string(), m.to_string())
        })
        .collect();
    let modules: BTreeSet<&String> = registry.values().collect();
    let hit: BTreeSet<&String> = reached.iter().filter_map(|ic| registry.get(ic)).collect();
    let unmapped: Vec<&String> = reached.iter().filter(|ic| !registry.contains_key(*ic)).collect();
    println!(
        "rule modules reached by the judged cases: {} of {} ({} iclasses judged; without a rule module: {unmapped:?})",
        hit.len(),
        modules.len(),
        reached.len()
    );
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
