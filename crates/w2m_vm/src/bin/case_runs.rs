// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! The case runner's verdicts on a substrate record (Sprint 150), as `scripts/vm_oracle.py --record` put them there
//! through `src/w2m/differential.py`: one unit per instruction class, the Intel CPU under Windows the reference.
//!
//! Substrate runs this binary as a Producer (`harness/case-runs.toml`; substrate `docs/process-producer.md`). It runs
//! every downloaded Intel run through `w2m_vm::cases::run_cases` with the pinned Python words fixture, and writes
//! `differential.run`'s sequence (its docstring, 9-19) in the vocabulary's types: SESSION_INIT; per run
//! EXECUTION_STARTED and EXECUTION_COMPLETED; per class CANDIDATE_GENERATED, COMPARISON_COMPLETED, then
//! CANDIDATE_ACCEPTED, or CANDIDATE_REJECTED, DISCREPANCY_REPORTED and DISCREPANCY_ROUTED; DISCREPANCY_RESOLVED for
//! each discrepancy the ledger holds open on an accepted class; SESSION_COMPLETED. The Python started each step from the
//! event before it with predicate triggers; a run file's triggers take no predicate, so one Producer writes the
//! sequence in order.
//!
//! `--plant-flip-cf` flips CF in every completed case of the first run's Windows results before judging: the record
//! must then hold discrepancies whose first divergence names a CF class.

#[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
fn main() {
    eprintln!("case_runs needs Hypervisor.framework on Apple silicon");
    std::process::exit(2);
}

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
fn main() -> std::process::ExitCode {
    match run::main() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("case_runs: {e}");
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

    use cpu_cases::{read_cases, read_results, sha256};
    use events::{
        CandidateAccepted, CandidateGenerated, CandidateRejected, ComparisonCompleted, ContractId, DiscrepancyReported,
        DiscrepancyResolution, DiscrepancyResolved, DiscrepancyRouted, Event, ExecutionCompleted, ExecutionStarted,
        ExitStatus, FiniteF64, HarnessU64, Id, Map, ProfileKind, Relation, RepairPath, SessionCompleted, SessionInit,
        SessionOutcome, SessionPurpose, Sha256, SubjectKind, TraceIngested, VOCAB_VERSION, Value, Verdict, Version,
        harness_line,
    };
    use w2m_vm::case_inputs::{Fixture, Hybrid, runs, with_case_vm};
    use w2m_vm::cases::run_cases;

    /// The differential's name: the record's baseline and the ledger's key (differential.py 64).
    const DIFFERENTIAL: &str = "case-runner";
    /// vm_oracle's record (325): the guest state at a fault and after an instruction.
    const CONTRACT_ID: &str = "w2m.guest_state.fault_context";
    const CONTRACT_VERSION: &str = "0.2.0";
    const HARNESS_VERSION: &str = env!("CARGO_PKG_VERSION");
    /// differential.py 43: the sha256 of nothing.
    const EMPTY: &str = "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

    fn root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .unwrap()
    }

    fn id(s: impl Into<String>) -> Id {
        Id::new(s).expect("a non-empty id")
    }

    fn sha(hex: &str) -> Sha256 {
        Sha256::new(format!("sha256:{hex}")).expect("64 lowercase hex digits")
    }

    fn u(n: usize) -> HarnessU64 {
        HarnessU64::new(n as u64).expect("a count in range")
    }

    /// differential.py `_safe` (91-92).
    fn safe(unit: &str) -> String {
        unit.chars()
            .map(|c| {
                if c.is_alphanumeric() || "._-".contains(c) {
                    c
                } else {
                    '-'
                }
            })
            .collect()
    }

    fn emit<E: Event>(out: &mut impl Write, e: &E) -> io::Result<()> {
        writeln!(out, "{}", harness_line(e))
    }

    /// One class's verdict over every run: its cases judged whole, and its difference lines, most frequent first, as
    /// `by_unit` (82-88) writes them.
    #[derive(Default)]
    struct Unit {
        judged: usize,
        differences: Vec<(usize, String)>,
    }

    /// differential.py `open_on` (137-139) over the ledger file `rebuild_ledger` writes: the discrepancies this
    /// differential reported on `unit` that no record has resolved.
    fn open_on(ledger: &Value, judge: &str, unit: &str) -> Vec<String> {
        let mut out: Vec<String> = ledger
            .as_object()
            .map(|m| {
                m.iter()
                    .filter(|(_, v)| v["differential"] == judge && v["unit"] == unit && v["resolved_in"].is_null())
                    .map(|(k, _)| k.clone())
                    .collect()
            })
            .unwrap_or_default();
        out.sort();
        out
    }

    pub fn main() -> Result<(), String> {
        let started = Instant::now();
        let plant = std::env::args().skip(1).any(|a| a == "--plant-flip-cf");
        // the Producer's input arrives as one JSON line; this run takes none, but the line is read so the pipe closes
        let mut input = String::new();
        io::stdin().lock().read_line(&mut input).map_err(|e| e.to_string())?;
        let root = root();
        let runs = runs(&root);
        if runs.is_empty() {
            return Err("no downloaded Intel results in build/probe-runs".into());
        }
        let fixture = Fixture::load(&root).ok_or("no Python words fixture (build/case_runner/python-words.txt)")??;
        // `--rust-translator` (Sprint 151): the Rust translator for the classes link 5's groups carry, the Python words
        // for the rest; the judge's name then says so
        let use_rust = std::env::args().skip(1).any(|a| a == "--rust-translator");
        let judge = if use_rust { "case-runner-rust" } else { DIFFERENTIAL };
        let hybrid = Hybrid::new(&fixture);
        let compile = |code: &[u8], a: u64, base: u64, exit: u64| {
            if use_rust {
                hybrid.compile(code, a, base, exit)
            } else {
                fixture.compile(code, a, base, exit)
            }
        };
        // the translator's identity: its sources, in name order
        let translator_sha = {
            let dir = root.join("crates/translator/src");
            let mut names: Vec<PathBuf> = fs::read_dir(&dir)
                .map_err(|e| e.to_string())?
                .map(|e| e.unwrap().path())
                .filter(|p| p.extension().is_some_and(|x| x == "rs"))
                .collect();
            names.sort();
            let mut all = Vec::new();
            for p in names {
                all.extend(p.file_name().unwrap().to_string_lossy().as_bytes());
                all.push(0);
                all.extend(fs::read(&p).map_err(|e| e.to_string())?);
                all.push(0);
            }
            sha256(&all)
        };
        let store = root.join("store/probe-runs").join(&runs[0].id);
        let cpu = fs::read_to_string(store.join("cpu.txt")).map_err(|e| e.to_string())?;
        let profile_txt = fs::read_to_string(store.join("profile.txt")).unwrap_or_default();
        let profile = format!("{}; {}", cpu.trim(), profile_txt.trim());
        let ledger: Value = fs::read_to_string(root.join("store/runs/discrepancy-ledger.json"))
            .ok()
            .and_then(|t| t.parse().ok())
            .unwrap_or(Value::Null);
        let session = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let mut out = io::stdout().lock();
        let w = |e: io::Error| e.to_string();

        emit(
            &mut out,
            &SessionInit {
                vocab_version: Version::new(VOCAB_VERSION).expect("the vocabulary's own version"),
                profile: id(profile.clone()),
                contract_version: Version::new(CONTRACT_VERSION).expect("a constant major.minor.patch"),
                harness_version: Version::new(HARNESS_VERSION).expect("Cargo's package version"),
                purpose: SessionPurpose::Conformance,
            },
        )
        .map_err(w)?;

        // every run through the runner: per run EXECUTION_STARTED and EXECUTION_COMPLETED, the verdicts gathered by
        // class
        let mut units: BTreeMap<String, Unit> = BTreeMap::new();
        let runs_ref = &runs;
        let outcome = with_case_vm(&root, &fixture, |guest, vcpu| -> Result<(), String> {
            for (k, run) in runs_ref.iter().enumerate() {
                let set_bytes = fs::read(&run.set).map_err(|e| e.to_string())?;
                let (fmt, cases) = read_cases(&set_bytes).map_err(|e| format!("{e:?}"))?;
                let mut results = read_results(&run.result, fmt).map_err(|e| format!("{e:?}"))?;
                if plant && k == 0 {
                    for r in results.iter_mut().filter(|r| r.marker != 2) {
                        r.rflags ^= 1;
                    }
                }
                let execution = format!("case-run-{}", run.id);
                let t0 = Instant::now();
                emit(
                    &mut out,
                    &ExecutionStarted {
                        execution_id: id(execution.clone()),
                        subject_kind: SubjectKind::CandidateRuntime,
                        subject_sha256: sha(&sha256(&set_bytes)),
                        profile_kind: ProfileKind::ManagedVm,
                        profile_id: id("w2m-case-runner"),
                        initial_state_sha256: sha(&fixture.sha256),
                        injected_inputs_sha256: sha(&sha256(&run.result)),
                        timeout_seconds: None,
                        kill_grace_seconds: None,
                        unobserved: None,
                    },
                )
                .map_err(w)?;
                let t = run_cases(guest, vcpu, &cases, &results, &compile).map_err(|e| format!("{e:?}"))?;
                // the run's trace: one JSON line per class judged, its count and its difference classes, kept in
                // store/case_runner/traces by its sha256 (store/traces holds guest traces only), then ingested (rule T4: each EXECUTION_COMPLETED's trace ingested once)
                let mut trace = String::new();
                for (class, n) in &t.by_class {
                    let diffs: Vec<Value> = t
                        .differences
                        .iter()
                        .filter(|(k, _)| k.split(' ').next() == Some(class.as_str()))
                        .map(|(k, (m, example))| Value::from(format!("{k}: {m} cases; {example}")))
                        .collect();
                    let line = Value::Object(Map::from_iter([
                        ("run".to_string(), Value::from(run.id.as_str())),
                        ("class".to_string(), Value::from(class.as_str())),
                        ("judged".to_string(), Value::from(*n)),
                        ("differences".to_string(), Value::Array(diffs)),
                    ]));
                    trace.push_str(&line.to_string());
                    trace.push('\n');
                }
                let trace_hex = sha256(trace.as_bytes());
                let lines = t.by_class.len();
                fs::write(
                    root.join("store/case_runner/traces").join(format!("{trace_hex}.jsonl")),
                    &trace,
                )
                .map_err(|e| e.to_string())?;
                emit(
                    &mut out,
                    &ExecutionCompleted {
                        execution_id: id(execution.clone()),
                        exit_status: ExitStatus::Exited,
                        trace_sha256: sha(&trace_hex),
                        guest_line_count: u(lines),
                        exit_code: None,
                        elapsed_seconds: Some(FiniteF64::new(t0.elapsed().as_secs_f64()).unwrap()),
                        unobserved: Some(format!(
                            "the trace is the runner's verdict per class, not guest output: {} of {} cases judged by \
                             state against run {}; {} ran in the model, not judged; {} refused",
                            t.judged,
                            cases.len(),
                            run.id,
                            t.model,
                            t.refused.values().sum::<usize>()
                        )),
                    },
                )
                .map_err(w)?;
                emit(
                    &mut out,
                    &TraceIngested {
                        trace_sha256: sha(&trace_hex),
                        execution_id: id(execution),
                        line_count: u(lines),
                        thread_count: u(1),
                        vocab_version: Version::new(VOCAB_VERSION).unwrap(),
                    },
                )
                .map_err(w)?;
                for (class, n) in &t.by_class {
                    units.entry(class.clone()).or_default().judged += n;
                }
                for (key, (n, example)) in &t.differences {
                    let class = key.split(' ').next().unwrap_or(key).to_string();
                    let line = format!("{key}: {n} cases; run {}: {example}", run.id);
                    units.entry(class).or_default().differences.push((*n, line));
                }
            }
            Ok(())
        })?;
        outcome?;

        // per class (differential.py 156-209): the candidate, its comparison, its decision
        let contract = ContractId::new(CONTRACT_ID).expect("a dotted id");
        let version = Version::new(CONTRACT_VERSION).unwrap();
        let reference = runs.iter().map(|r| r.id.as_str()).collect::<Vec<_>>().join("+");
        let (mut accepted, mut reported) = (0, 0);
        for (unit, v) in units.iter_mut() {
            v.differences.sort_by_key(|a| std::cmp::Reverse(a.0));
            // a class the Rust translator compiled is its candidate; any other, the Python words' (Sprint 151)
            let by_rust = hybrid.rust.borrow().contains_key(unit);
            let (generator, candidate) = if by_rust {
                (
                    "the Rust translator (crates/translator), run by the Rust case runner",
                    translator_sha.as_str(),
                )
            } else {
                (
                    "the frozen Python lowering's words (build/case_runner/python-words.txt), run by the Rust case runner",
                    fixture.sha256.as_str(),
                )
            };
            let cid = format!("{judge}:{}@{}", safe(unit), &candidate[..12]);
            let repairing = open_on(&ledger, judge, unit);
            emit(
                &mut out,
                &CandidateGenerated {
                    candidate_id: id(cid.clone()),
                    contract_id: contract.clone(),
                    contract_version: version.clone(),
                    generator_ref: generator.into(),
                    attempt: u(1),
                    candidate_sha256: sha(candidate),
                    parent_candidate_id: None,
                    discrepancy_id: repairing.first().map(|d| id(d.clone())),
                },
            )
            .map_err(w)?;
            let comparison = format!("cmp-{cid}");
            let rule = format!("{} cases of {unit} judged whole in {} Intel runs", v.judged, runs.len());
            let first = v.differences.first().map(|(_, l)| {
                let more = v.differences.len() - 1;
                let s = if more > 0 {
                    format!("{unit}: {l} (+{more} more)")
                } else {
                    format!("{unit}: {l}")
                };
                s.chars().take(500).collect::<String>()
            });
            let verdict = if v.differences.is_empty() {
                Verdict::Conforms
            } else {
                Verdict::Discrepancy
            };
            emit(
                &mut out,
                &ComparisonCompleted {
                    comparison_id: id(comparison.clone()),
                    contract_id: contract.clone(),
                    contract_version: version.clone(),
                    relation: Relation::Exact,
                    reference_ref: id(reference.clone()),
                    target_execution_id: id("case-runs"),
                    verdict,
                    rule_ref: Some(rule),
                    first_divergence: first.clone(),
                },
            )
            .map_err(w)?;
            if v.differences.is_empty() {
                accepted += 1;
                emit(
                    &mut out,
                    &CandidateAccepted {
                        candidate_id: id(cid.clone()),
                        contract_id: contract.clone(),
                        contract_version: version.clone(),
                        comparison_ids: vec![id(comparison)],
                    },
                )
                .map_err(w)?;
                // differential.py 219-226: each discrepancy open on this unit from an earlier run is resolved
                for did in repairing {
                    emit(
                        &mut out,
                        &DiscrepancyResolved {
                            discrepancy_id: id(did),
                            resolution: DiscrepancyResolution::ImplementationRepaired,
                            resolving_ref: id(cid.clone()),
                        },
                    )
                    .map_err(w)?;
                }
            } else {
                reported += 1;
                let did = format!("{judge}-{session}-{}", safe(unit));
                emit(
                    &mut out,
                    &CandidateRejected {
                        candidate_id: id(cid.clone()),
                        contract_id: contract.clone(),
                        contract_version: version.clone(),
                        reason: format!("{} differences on {unit}", v.differences.len()),
                        discrepancy_id: Some(id(did.clone())),
                    },
                )
                .map_err(w)?;
                let first_line = format!("{unit}: {}", v.differences[0].1);
                emit(
                    &mut out,
                    &DiscrepancyReported {
                        discrepancy_id: id(did.clone()),
                        contract_id: contract.clone(),
                        contract_version: version.clone(),
                        mismatch_class: Relation::Exact,
                        minimized_program_sha256: Sha256::new(EMPTY).unwrap(),
                        initial_state_sha256: Sha256::new(EMPTY).unwrap(),
                        first_divergence: first_line.chars().take(500).collect(),
                        reference_profile_id: id(profile.clone()),
                        destination_profile_id: id("w2m the Rust case runner on the frozen Python lowering's words"),
                        comparison_id: id(comparison),
                    },
                )
                .map_err(w)?;
                emit(
                    &mut out,
                    &DiscrepancyRouted {
                        discrepancy_id: id(did),
                        repair_path: RepairPath::ImplementationRepair,
                        reference_conforms: true,
                        grounds: "the Intel CPU under Windows is the reference; the VM back end or guest runtime \
                                  differs"
                            .into(),
                    },
                )
                .map_err(w)?;
            }
        }
        let classes: BTreeSet<&String> = units.keys().collect();
        eprintln!(
            "case_runs: {} classes, {accepted} accepted, {reported} with discrepancies",
            classes.len()
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
