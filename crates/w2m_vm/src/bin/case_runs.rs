// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! The case runner's verdicts on a substrate record (Sprint 150; Sprint 152's form), as `scripts/vm_oracle.py --record`
//! put them there through `src/w2m/differential.py`: one unit per instruction class, one reference CPU under Windows a
//! session.
//!
//! Substrate runs this binary as a Producer (`harness/case-runs.toml`, `harness/case-runs-rust.toml`; substrate
//! `docs/process-producer.md`). It runs the downloaded Intel runs of the reference profile `W2M_CASE_PROFILE` names
//! (`<cpu>; <profile>`, as `store/probe-runs/<run>/cpu.txt` and `profile.txt` spell them) through
//! `w2m_vm::cases::run_cases`, and writes `differential.run`'s sequence (its docstring, 9-19) in the vocabulary's
//! types: SESSION_INIT; per class CANDIDATE_GENERATED,
//! COMPARISON_COMPLETED, then CANDIDATE_ACCEPTED, or CANDIDATE_REJECTED, DISCREPANCY_REPORTED and DISCREPANCY_ROUTED;
//! DISCREPANCY_RESOLVED for each discrepancy the judges' ledger (`w2m_vm::ledger`) holds open on an accepted class;
//! SESSION_COMPLETED. As in the Python, no execution or trace event: the runs are judged by state, and no guest trace is
//! made. The Python started each step from the event before it with predicate triggers; a run file's triggers take no
//! predicate, so one Producer writes the sequence in order.
//!
//! `--rust-translator` (Sprint 151) compiles the classes link 5's groups carry with the Rust translator, the rest with
//! the Python words. `--plant-flip-cf` flips CF in every completed case of the session's first run's Windows results:
//! the record must then hold discrepancies whose first divergence names a CF class. `--write-ledger` (not a Producer)
//! rebuilds the judges' ledger from the records and writes `store/case_runner/discrepancy-ledger.json`, after a run;
//! `--check-ledger` (the gate) fails when that file is not what the records give.

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
    use std::collections::BTreeMap;
    use std::fs;
    use std::io::{self, BufRead, Write};
    use std::path::{Path, PathBuf};
    use std::time::{Instant, SystemTime, UNIX_EPOCH};

    use cpu_cases::{read_cases, read_results, sha256};
    use events::{
        CandidateAccepted, CandidateGenerated, CandidateRejected, ComparisonCompleted, ContractId, DiscrepancyReported,
        DiscrepancyResolution, DiscrepancyResolved, DiscrepancyRouted, Event, FiniteF64, HarnessU64, Id, Relation,
        RepairPath, SessionCompleted, SessionInit, SessionOutcome, SessionPurpose, Sha256, VOCAB_VERSION, Verdict,
        Version, harness_line,
    };
    use w2m_vm::case_inputs::{Fixture, Hybrid, case_cpuid_va, runs, with_case_vm};
    use w2m_vm::cases::run_cases;
    use w2m_vm::ledger;

    /// The judge's name: the record's baseline and the ledger's key.
    const JUDGE: &str = "case-runner";
    const JUDGE_RUST: &str = "case-runner-rust";
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

    /// One class's verdict over the session's runs: its cases judged whole, and its difference lines, most frequent
    /// first, as `by_unit` (82-88) writes them.
    #[derive(Default)]
    struct Unit {
        judged: usize,
        differences: Vec<(usize, String)>,
    }

    /// The translator's identity: its sources, in name order.
    fn translator_sha(root: &Path) -> Result<String, String> {
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
        Ok(sha256(&all))
    }

    pub fn main() -> Result<(), String> {
        let started = Instant::now();
        let args: Vec<String> = std::env::args().skip(1).collect();
        let flag = |f: &str| args.iter().any(|a| a == f);
        let root = root();
        // the records the ledger is rebuilt from and the file it is written to: the store's, or a scratch pair for the
        // ledger chain's demonstration (`W2M_RUNS`, `W2M_LEDGER`), where a planted record may stand
        let runs_dir = std::env::var("W2M_RUNS").map_or_else(|_| root.join("store/runs"), PathBuf::from);
        let ledger_file = std::env::var("W2M_LEDGER").map_or_else(
            |_| root.join("store/case_runner/discrepancy-ledger.json"),
            PathBuf::from,
        );
        if flag("--write-ledger") {
            // differential.run 281-282: the ledger rebuilt from every record, written after a run
            let ledger = ledger::rebuild(&runs_dir);
            fs::write(&ledger_file, ledger::text(&ledger)).map_err(|e| e.to_string())?;
            eprintln!("case_runs: {} discrepancies in the judges' ledger", ledger.len());
            return Ok(());
        }
        if flag("--check-ledger") {
            // the gate (Sprint 154, review of Sprints 151-152): the committed ledger is what the records say
            let want = ledger::text(&ledger::rebuild(&runs_dir));
            let have = fs::read_to_string(&ledger_file).map_err(|e| format!("{}: {e}", ledger_file.display()))?;
            if have != want {
                return Err(format!(
                    "{} is not the ledger the records give; `case_runs --write-ledger` rebuilds it",
                    ledger_file.display()
                ));
            }
            return Ok(());
        }
        let plant = flag("--plant-flip-cf");
        let use_rust = flag("--rust-translator");
        let judge = if use_rust { JUDGE_RUST } else { JUDGE };
        // the Producer's input arrives as one JSON line; this run takes none, but the line is read so the pipe closes
        let mut input = String::new();
        io::stdin().lock().read_line(&mut input).map_err(|e| e.to_string())?;
        // the reference profile: the CPU and the Windows image, as each run's cpu.txt and profile.txt name them
        // (`<cpu>; <profile>`); one session a profile (review of Sprints 149-150, Medium 1)
        let profile = std::env::var("W2M_CASE_PROFILE")
            .map_err(|_| "W2M_CASE_PROFILE names no reference profile (`<cpu>; <profile>`)".to_string())?;
        let profile_of = |run: &str| {
            let read = |f: &str| {
                fs::read_to_string(root.join("store/probe-runs").join(run).join(f))
                    .map(|t| t.trim().to_string())
                    .unwrap_or_default()
            };
            format!("{}; {}", read("cpu.txt"), read("profile.txt"))
        };
        let runs: Vec<_> = runs(&root)
            .into_iter()
            .filter(|r| profile_of(&r.id) == profile)
            .collect();
        if runs.is_empty() {
            return Err(format!("no downloaded Intel result of a run on {profile}"));
        }
        let fixture = Fixture::load(&root).ok_or("no Python words fixture (build/case_runner/python-words.txt)")??;
        let hybrid = Hybrid::new(&fixture, case_cpuid_va(&root)?);
        let compile = |code: &[u8], a: u64, base: u64, exit: u64| {
            if use_rust {
                hybrid.compile(code, a, base, exit)
            } else {
                fixture.compile(code, a, base, exit)
            }
        };
        let translator_sha = translator_sha(&root)?;
        // the judges' ledger as the records say (differential.run 144)
        let ledger = ledger::rebuild(&runs_dir);
        let session_ns = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let session = format!("{judge}-{session_ns}");
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

        // the session's runs through the runner, the verdicts gathered by class
        let mut units: BTreeMap<String, Unit> = BTreeMap::new();
        let mut per_run = Vec::new();
        let runs_ref = &runs;
        let outcome = with_case_vm(&root, &fixture, |guest, vcpu| -> Result<(), String> {
            for (k, run) in runs_ref.iter().enumerate() {
                let (fmt, cases) =
                    read_cases(&fs::read(&run.set).map_err(|e| e.to_string())?).map_err(|e| format!("{e:?}"))?;
                let mut results = read_results(&run.result, fmt).map_err(|e| format!("{e:?}"))?;
                if plant && k == 0 {
                    for r in results.iter_mut().filter(|r| r.marker != 2) {
                        r.rflags ^= 1;
                    }
                }
                let t = run_cases(guest, vcpu, &cases, &results, &compile).map_err(|e| format!("{e:?}"))?;
                per_run.push(format!("{} {}/{}", run.id, t.judged, cases.len()));
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
        eprintln!("case_runs: {judge} on {profile}: cases judged per run {per_run:?}");

        // per class (differential.py 156-226): the candidate, its comparison, its decision
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
            let repairing = ledger::open_on(&ledger, judge, unit);
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
            let rule = format!(
                "{} cases of {unit} judged whole in {} Intel runs on {profile}",
                v.judged,
                runs.len()
            );
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
                    target_execution_id: id(session.clone()),
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
                let did = format!("{session}-{}", safe(unit));
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
                        destination_profile_id: id(format!("w2m {generator}")),
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
        eprintln!(
            "case_runs: {} classes, {accepted} accepted, {reported} with discrepancies",
            units.len()
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
