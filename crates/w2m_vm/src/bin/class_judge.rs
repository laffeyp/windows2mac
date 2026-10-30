// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! The class judge on the Rust host (Sprint 222; census row 3; plan step 4, "456 of 458 classes"): each class of the
//! frozen index `store/runs/class-runs.json` judged by the VM on the cases of the reference runs its frozen record names,
//! as `scripts/class_run.py` judged it.
//!
//! Substrate runs this binary as a Producer (`harness/class-judge.toml`). The frozen records together name 21 reference
//! runs, one a case set (each record's RunStarted baseline, `reference_run`); a case's class is its decoded class
//! (`_built`, 70-82), so one pass of the case runner over the 21 runs gives every class its own cases. Per class, as
//! `item_run` (128-257) wrote less the model's comparison (the Python rules interpreter, frozen under the oracle):
//! CANDIDATE_GENERATED; COMPARISON_COMPLETED, executor `managed_vm`, its rule_ref "<n> cases of <class>"; then
//! CANDIDATE_ACCEPTED, or REDUCTION_COMPLETED, DISCREPANCY_REPORTED and DISCREPANCY_ROUTED for each difference. One
//! SESSION_INIT and one SESSION_COMPLETED hold the classes, as `case_runs` holds its units, where the Python wrote a
//! record a class. A class conforms when no case differs and every case it read is judged or set aside for undefined
//! memory; a case the compiler or the model's slice refuses counts against it ("not judged"), as `class_run.py` 164-170
//! counts "not compiled".
//!
//! `--index RECORD` (not a Producer) writes the Rust index from a record: per class its record, its case count and its
//! `managed_vm` verdict, the frozen index's form less `model`.

#[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
fn main() {
    eprintln!("class_judge needs Hypervisor.framework on Apple silicon");
    std::process::exit(2);
}

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
fn main() -> std::process::ExitCode {
    match run::main() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("class_judge: {e}");
            std::process::ExitCode::FAILURE
        }
    }
}

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
mod run {
    use std::collections::{BTreeMap, BTreeSet};
    use std::fs;
    use std::io::{self, BufRead, Write};
    use std::path::{Path, PathBuf};
    use std::time::{Instant, SystemTime, UNIX_EPOCH};

    use cpu_cases::{case_hashes, read_cases, read_results};
    use events::{
        CandidateAccepted, CandidateGenerated, ComparisonCompleted, ContractId, DiscrepancyReported, DiscrepancyRouted,
        Event, FiniteF64, HarnessU64, Id, ReductionCompleted, Relation, RepairPath, SessionCompleted, SessionInit,
        SessionOutcome, SessionPurpose, Sha256, VOCAB_VERSION, Verdict, Version, harness_line,
    };
    use serde_json::{Value, json};
    use w2m_vm::case_inputs::{CaseCompiler, case_runtime_calls, runs, translator_sha, with_case_vm};
    use w2m_vm::cases::run_cases;

    const INDEX: &str = "store/runs/class-runs.json";
    const HARNESS_VERSION: &str = env!("CARGO_PKG_VERSION");
    const EXECUTOR: &str = "managed_vm";

    fn root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .unwrap()
    }

    fn id(s: impl Into<String>) -> Id {
        Id::new(s).expect("a non-empty id")
    }

    fn u(n: usize) -> HarnessU64 {
        HarnessU64::new(n as u64).expect("a count in range")
    }

    fn emit<E: Event>(out: &mut impl Write, e: &E) -> io::Result<()> {
        writeln!(out, "{}", harness_line(e))
    }

    /// A record's events, substrate's frames, from its event files in name order.
    fn record_events(dir: &Path) -> Result<Vec<Value>, String> {
        let mut files: Vec<PathBuf> = fs::read_dir(dir)
            .map_err(|e| format!("{}: {e}", dir.display()))?
            .map(|e| e.unwrap().path())
            .filter(|p| p.file_name().unwrap().to_string_lossy().starts_with("events-"))
            .collect();
        files.sort();
        let mut out = Vec::new();
        for f in files {
            for line in fs::read_to_string(&f).map_err(|e| e.to_string())?.lines() {
                out.push(serde_json::from_str(line).map_err(|e| format!("{}: {e}", f.display()))?);
            }
        }
        Ok(out)
    }

    /// A frozen class: its reference runs (the record's baseline) and the contract version its comparison names.
    struct Frozen {
        runs: Vec<String>,
        version: String,
    }

    fn frozen(root: &Path) -> Result<BTreeMap<String, Frozen>, String> {
        let index: Value = serde_json::from_str(&fs::read_to_string(root.join(INDEX)).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
        let mut out = BTreeMap::new();
        for entry in index.as_array().ok_or("the frozen index is not a list")? {
            let iclass = entry["iclass"].as_str().ok_or("an entry with no iclass")?;
            let events = record_events(&root.join(entry["record"].as_str().ok_or("an entry with no record")?))?;
            let runs = events
                .iter()
                .find_map(|e| e["payload"]["baseline"]["reference_run"].as_str())
                .ok_or(format!("{iclass}: its record names no reference_run"))?;
            let version = events
                .iter()
                .find(|e| e["kind"] == "COMPARISON_COMPLETED")
                .and_then(|e| e["payload"]["contract_version"].as_str())
                .ok_or(format!("{iclass}: its record holds no comparison"))?;
            out.insert(
                iclass.to_string(),
                Frozen {
                    runs: runs.split('+').map(str::to_string).collect(),
                    version: version.to_string(),
                },
            );
        }
        Ok(out)
    }

    /// One class over its runs: the cases read, judged and set aside; its measured differences, each with its count,
    /// its line and the first case's program and initial state; and its cases read and not judged (the compiler's or
    /// the model's refusals), each run's count and first case. A difference is measured against Windows; an unjudged
    /// case is not, so it makes the class inconclusive, never a discrepancy (reviewer-3's High 1 of its review at 222).
    #[derive(Default)]
    struct Class {
        read: usize,
        judged: usize,
        undefined: usize,
        differences: Vec<(usize, String, (String, String))>,
        unjudged: Vec<(usize, String)>,
    }

    impl Class {
        fn verdict(&self) -> Verdict {
            if !self.differences.is_empty() {
                Verdict::Discrepancy
            } else if !self.unjudged.is_empty() {
                Verdict::Inconclusive
            } else {
                Verdict::Conforms
            }
        }
    }

    /// `--index RECORD`: the Rust index from a record's comparisons.
    fn index(root: &Path, record: &Path) -> Result<(), String> {
        let rel = record
            .canonicalize()
            .map_err(|e| e.to_string())?
            .strip_prefix(root)
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| record.display().to_string());
        let mut out = Vec::new();
        for e in record_events(record)? {
            if e["kind"] != "COMPARISON_COMPLETED" {
                continue;
            }
            let rule = e["payload"]["rule_ref"]
                .as_str()
                .ok_or("a comparison with no rule_ref")?;
            let form =
                || format!("rule_ref {rule:?} is not \"<n> cases of <class>; <r> compiled by the Rust translator\"");
            let (cases, rest) = rule.split_once(" cases of ").ok_or_else(form)?;
            let (iclass, rust) = rest.split_once("; ").ok_or_else(form)?;
            let rust = rust.strip_suffix(" compiled by the Rust translator").ok_or_else(form)?;
            out.push(json!({
                "iclass": iclass,
                "record": rel,
                "cases": cases.parse::<u64>().map_err(|e| e.to_string())?,
                "rust_compiled": rust.parse::<u64>().map_err(|e| e.to_string())?,
                "verdicts": {EXECUTOR: e["payload"]["verdict"]},
            }));
        }
        // a record whose Producer failed holds no comparison: no index, not an empty one
        if out.is_empty() {
            return Err(format!("{} holds no comparison", record.display()));
        }
        out.sort_by(|a, b| a["iclass"].as_str().cmp(&b["iclass"].as_str()));
        println!("{}", serde_json::to_string_pretty(&out).map_err(|e| e.to_string())?);
        Ok(())
    }

    pub fn main() -> Result<(), String> {
        let started = Instant::now();
        let args: Vec<String> = std::env::args().skip(1).collect();
        let root = root();
        if let [flag, record] = args.as_slice()
            && flag == "--index"
        {
            return index(&root, Path::new(record));
        }
        // a record is written only by a binary built from the sources it stands for (Sprint 187)
        w2m_vm::built_from::check(&root, "class_judge")?;
        let mut input = String::new();
        io::stdin().lock().read_line(&mut input).map_err(|e| e.to_string())?;

        let frozen = frozen(&root)?;
        let parts = |v: &str| v.split('.').map(|p| p.parse::<u64>().unwrap_or(0)).collect::<Vec<_>>();
        let latest = frozen
            .values()
            .map(|f| f.version.clone())
            .max_by_key(|v| parts(v))
            .ok_or("the frozen index is empty")?;
        let wanted: BTreeSet<&str> = frozen
            .values()
            .flat_map(|f| f.runs.iter().map(String::as_str))
            .collect();
        // the plants (card 222): `--plant-flip-cf` flips CF in every completed case of the first run's results, as
        // `case_runs` does; `--plant-every-run` judges every downloaded run of the sets, not the records' own
        let flag = |f: &str| args.iter().any(|a| a == f);
        let every_run = flag("--plant-every-run");
        let flip_cf = flag("--plant-flip-cf");
        let runs: Vec<_> = runs(&root)
            .into_iter()
            .filter(|r| every_run || wanted.contains(r.id.as_str()))
            .collect();
        let have: BTreeSet<&str> = runs.iter().map(|r| r.id.as_str()).collect();
        if let Some(missing) = wanted.difference(&have).next() {
            return Err(format!(
                "reference run {missing} is not downloaded under build/probe-runs"
            ));
        }
        let profile_of = |run: &str| {
            let read = |f: &str| {
                fs::read_to_string(root.join("store/probe-runs").join(run).join(f))
                    .map(|t| t.trim().to_string())
                    .unwrap_or_default()
            };
            format!("{}; {}", read("cpu.txt"), read("profile.txt"))
        };
        let profiles: BTreeSet<String> = runs.iter().map(|r| profile_of(&r.id)).collect();
        let profile = profiles.into_iter().collect::<Vec<_>>().join(" + ");

        let compiler = CaseCompiler::new(case_runtime_calls(&root)?);
        let translator = translator_sha(&root)?;
        let compile = |code: &[u8], a: u64, base: u64, exit: u64| compiler.compile(code, a, base, exit);
        let mut out = io::stdout().lock();
        let w = |e: io::Error| e.to_string();
        let session_ns = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let session = format!("class-judge-{session_ns}");
        emit(
            &mut out,
            &SessionInit {
                vocab_version: Version::new(VOCAB_VERSION).expect("the vocabulary's own version"),
                profile: id(profile.clone()),
                // the session's: the latest case set's version any class names (0.65.2, set 065c), each class's own
                // on its comparison
                contract_version: Version::new(latest.clone()).map_err(|e| e.to_string())?,
                harness_version: Version::new(HARNESS_VERSION).expect("Cargo's package version"),
                purpose: SessionPurpose::Conformance,
            },
        )
        .map_err(w)?;

        // the 21 runs through the case runner, each case's class its decoded class
        let mut classes: BTreeMap<String, Class> = BTreeMap::new();
        let runs_ref = &runs;
        with_case_vm(&root, |guest, vcpu| -> Result<(), String> {
            for (k, run) in runs_ref.iter().enumerate() {
                let (fmt, cases) =
                    read_cases(&fs::read(&run.set).map_err(|e| e.to_string())?).map_err(|e| format!("{e:?}"))?;
                let mut results = read_results(&run.result, fmt).map_err(|e| format!("{e:?}"))?;
                if flip_cf && k == 0 {
                    for r in results.iter_mut().filter(|r| r.marker != 2) {
                        r.rflags ^= 1;
                    }
                }
                for c in &cases {
                    let iclass =
                        x86_decoder::decode(&c.code, None).map_or_else(|e| format!("decode: {e}"), |d| d.iclass);
                    classes.entry(iclass).or_default().read += 1;
                }
                let t = run_cases(guest, vcpu, &cases, &results, &compile).map_err(|e| format!("{e:?}"))?;
                for (iclass, n) in &t.by_class {
                    classes.entry(iclass.clone()).or_default().judged += n;
                }
                for (iclass, n) in &t.undefined_by_class {
                    classes.entry(iclass.clone()).or_default().undefined += n;
                }
                for (key, (n, example)) in &t.differences {
                    let iclass = key.split(' ').next().unwrap_or(key).to_string();
                    let line = format!("{key}: {n} cases; run {}: {example}", run.id);
                    let hashes = case_hashes(&cases[t.first_case[key]]);
                    classes.entry(iclass).or_default().differences.push((*n, line, hashes));
                }
                for (iclass, (n, first)) in &t.unjudged_by_class {
                    let line = format!("{iclass}: {n} cases not judged; run {}: case {first}", run.id);
                    classes.entry(iclass.clone()).or_default().unjudged.push((*n, line));
                }
            }
            Ok(())
        })??;

        let (mut accepted, mut reported, mut inconclusive) = (0, 0, 0);
        for (iclass, f) in &frozen {
            let mut v = classes.remove(iclass).unwrap_or_default();
            let unjudged: usize = v.unjudged.iter().map(|u| u.0).sum();
            if v.differences.is_empty() && v.judged + v.undefined + unjudged != v.read {
                return Err(format!(
                    "{iclass}: {} cases read, {} judged, {} set aside and {unjudged} not judged, with no difference",
                    v.read, v.judged, v.undefined
                ));
            }
            v.differences.sort_by_key(|d| std::cmp::Reverse(d.0));
            v.unjudged.sort_by_key(|u| std::cmp::Reverse(u.0));
            let verdict = v.verdict();
            let differences = v.differences;
            // the cases whose words the Rust translator made (Medium 1), every one from Sprint 238
            let rust_compiled = compiler.compiled.borrow().get(iclass).copied().unwrap_or(0);
            let contract = ContractId::new(format!("x86.semantics.{}", iclass.to_lowercase())).expect("a dotted id");
            let version = Version::new(&f.version).map_err(|e| format!("{iclass}: {e}"))?;
            // the candidate is the Rust translator (as `case_runs`)
            let (generator, candidate) = (
                "the Rust translator (crates/translator), run by the Rust case runner",
                translator.as_str(),
            );
            let cand = format!("class-judge:{iclass}@{}", &candidate[..12]);
            emit(
                &mut out,
                &CandidateGenerated {
                    candidate_id: id(cand.clone()),
                    contract_id: contract.clone(),
                    contract_version: version.clone(),
                    generator_ref: generator.into(),
                    attempt: u(1),
                    candidate_sha256: Sha256::new(format!("sha256:{candidate}")).expect("a sha256"),
                    parent_candidate_id: None,
                    discrepancy_id: None,
                },
            )
            .map_err(w)?;
            let comparison = format!("cmp-{cand}-{EXECUTOR}");
            let first = differences
                .first()
                .map(|(_, l, _)| l.as_str())
                .or(v.unjudged.first().map(|(_, l)| l.as_str()))
                .map(|l| l.chars().take(500).collect::<String>());
            emit(
                &mut out,
                &ComparisonCompleted {
                    comparison_id: id(comparison.clone()),
                    contract_id: contract.clone(),
                    contract_version: version.clone(),
                    relation: Relation::Exact,
                    reference_ref: id(f.runs.join("+")),
                    target_execution_id: id(format!("{EXECUTOR}-{session}")),
                    verdict,
                    rule_ref: Some(format!(
                        "{} cases of {iclass}; {rust_compiled} compiled by the Rust translator",
                        v.read
                    )),
                    first_divergence: first,
                },
            )
            .map_err(w)?;
            if verdict == Verdict::Inconclusive {
                // nothing compared: no acceptance and no discrepancy, the ledger opens no entry
                inconclusive += 1;
                continue;
            }
            if verdict == Verdict::Conforms {
                accepted += 1;
                emit(
                    &mut out,
                    &CandidateAccepted {
                        candidate_id: id(cand),
                        contract_id: contract,
                        contract_version: version,
                        comparison_ids: vec![id(comparison)],
                    },
                )
                .map_err(w)?;
                continue;
            }
            reported += 1;
            for (k, (_, line, (program, initial))) in differences.iter().enumerate() {
                // the class's first failing case: one instruction, so nothing can be cut from it (class_run.py 195-202)
                let did = format!("{session}-{}-d{}", iclass.to_lowercase(), k + 1);
                emit(
                    &mut out,
                    &ReductionCompleted {
                        reduction_id: id(format!("{did}-reduction")),
                        comparison_id: id(comparison.clone()),
                        source_program_sha256: Sha256::new(program).unwrap(),
                        minimized_program_sha256: Sha256::new(program).unwrap(),
                        steps: u(0),
                        preserved_mismatch: true,
                    },
                )
                .map_err(w)?;
                emit(
                    &mut out,
                    &DiscrepancyReported {
                        discrepancy_id: id(did.clone()),
                        contract_id: contract.clone(),
                        contract_version: version.clone(),
                        mismatch_class: Relation::Exact,
                        minimized_program_sha256: Sha256::new(program).unwrap(),
                        initial_state_sha256: Sha256::new(initial).unwrap(),
                        // "<class>: …", the unit the judges' ledger reads before the first colon (ledger.rs 95)
                        first_divergence: format!("{iclass}: {EXECUTOR}: {line}").chars().take(500).collect(),
                        reference_profile_id: id(profile.clone()),
                        destination_profile_id: id(format!("w2m-{EXECUTOR}")),
                        comparison_id: id(comparison.clone()),
                    },
                )
                .map_err(w)?;
                emit(
                    &mut out,
                    &DiscrepancyRouted {
                        discrepancy_id: id(did),
                        repair_path: RepairPath::ImplementationRepair,
                        reference_conforms: true,
                        grounds: "the CPU's recorded result is the reference; the translator or the runtime differs"
                            .into(),
                    },
                )
                .map_err(w)?;
            }
        }
        eprintln!(
            "class_judge: {} classes, {accepted} accepted, {reported} with discrepancies, {inconclusive} inconclusive",
            frozen.len()
        );
        emit(
            &mut out,
            &SessionCompleted {
                outcome: if reported > 0 {
                    SessionOutcome::CompletedWithDiscrepancies
                } else {
                    SessionOutcome::Completed
                },
                discrepancy_count: u(reported),
                accepted_count: u(accepted),
                elapsed_seconds: FiniteF64::new(started.elapsed().as_secs_f64()).unwrap(),
            },
        )
        .map_err(w)?;
        Ok(())
    }
}
