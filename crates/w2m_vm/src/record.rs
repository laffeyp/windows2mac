// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! The skeleton's record and its golden check (Sprint 158; link 6 of `docs/design-host-compile-path-2026-10-24.md`).
//!
//! The record is `record_translations`' (`src/w2m/vm_backend.py` 5570-5612): one session, purpose `compilation`:
//! SESSION_INIT; per block translated, in compile order, CODE_TARGET_DISCOVERED (discovery `static` for the entry, else
//! `runtime_transfer`; stage `execution`) and TRANSLATION_PUBLISHED (the guest range, code version 0, the decoder spec's
//! version, artifact `vm-code-cache:<code address>`); SESSION_COMPLETED. `w2m-vm` writes it as substrate's line
//! protocol.
//!
//! The golden check reads two records' w2m events (every frame but substrate's own) and holds them equal in order,
//! kind for kind and field for field, but for three fields each implementation sets for itself: `elapsed_seconds` (a
//! time), `harness_version` (the Python's 0.0.1, the crate's own version), and `vocab_version` (the frozen Python
//! writes 0.9, the Rust vocabulary is `VOCAB_VERSION`, 0.12 on 2026-10-29).
//!
//! A run's execution record (Sprint 210) is `run_frontier.py`'s (262-266, 380-407): one session, purpose
//! `application`: SESSION_INIT, EXECUTION_STARTED, MEASUREMENT_RECORDED for what the Rust host measures (the
//! instructions, the seconds in the VM, the exits by hvc), SERVICE_UNIMPLEMENTED at a host request it does not serve,
//! EXECUTION_COMPLETED. The frozen host's measurements by thread, by the host's time per exit, of switches, wakes and
//! load have no Rust counterpart yet and are left out, not written as 0. From Sprint 217 the session goes on in a second
//! Producer, `call_compare`, which a trigger on EXECUTION_COMPLETED starts: COMPARISON_COMPLETED and
//! SESSION_COMPLETED are its, as the judge is not the host's (reviewer-3's Medium 5 of its review of Sprints 212-214).

use std::path::Path;
use std::time::Duration;

use events::{
    CodeTargetDiscovered, Discovery, FiniteF64, HarnessU64, Id, SessionCompleted, SessionInit, SessionOutcome,
    SessionPurpose, Sha256, Stage, TranslationPublished, VOCAB_VERSION, Value, Version, harness_line,
};

use crate::ledger::frames;

/// The profile both hosts' skeleton records name.
pub const PROFILE: &str = "w2m-vm-skeleton";
/// The fields the golden check does not compare, and why: see the module's documentation.
pub const EXEMPT: [&str; 3] = ["elapsed_seconds", "harness_version", "vocab_version"];
const HARNESS_VERSION: &str = env!("CARGO_PKG_VERSION");

fn u(n: u64) -> HarnessU64 {
    HarnessU64::new(n).expect("a guest address below 2^53")
}

/// `record_translations`' events, as substrate lines: the guest image's sha256 (64 hex digits), its entry, each block
/// (start, end, code address) in compile order, and the session's time.
pub fn translation_lines(image_sha256: &str, entry: u64, blocks: &[(u64, u64, u64)], elapsed: Duration) -> Vec<String> {
    let image = Sha256::new(format!("sha256:{image_sha256}")).expect("64 lowercase hex digits");
    let spec = Version::new(x86_decoder::SPEC_VERSION).expect("the decoder spec's major.minor.patch");
    let mut lines = vec![harness_line(&SessionInit {
        vocab_version: Version::new(VOCAB_VERSION).expect("the vocabulary's own version"),
        profile: Id::new(PROFILE).expect("a constant, not empty"),
        contract_version: spec.clone(),
        harness_version: Version::new(HARNESS_VERSION).expect("Cargo's package version"),
        purpose: SessionPurpose::Compilation,
    })];
    for &(start, end, code) in blocks {
        lines.push(harness_line(&CodeTargetDiscovered {
            image_sha256: image.clone(),
            guest_address: u(start),
            discovery: if start == entry {
                Discovery::Static
            } else {
                Discovery::RuntimeTransfer
            },
            stage: Stage::Execution,
        }));
        lines.push(harness_line(&TranslationPublished {
            translation_id: Id::new(format!("tr-{}-{start:x}", &image_sha256[..12])).expect("not empty"),
            image_sha256: image.clone(),
            guest_range_start: u(start),
            guest_range_end: u(end),
            code_version: u(0),
            stage: Stage::Execution,
            contract_version: spec.clone(),
            artifact_id: Id::new(format!("vm-code-cache:{code:x}")).expect("not empty"),
        }));
    }
    let zero = u(0);
    lines.push(harness_line(&SessionCompleted {
        outcome: SessionOutcome::Completed,
        discrepancy_count: zero,
        accepted_count: zero,
        // `round(time.monotonic() - t0, 3)`
        elapsed_seconds: FiniteF64::new((elapsed.as_secs_f64() * 1000.0).round() / 1000.0)
            .expect("a measured duration is finite"),
    }));
    lines
}

/// `EMPTY` (run_frontier.py 63): the sha256 of no bytes, a run's initial state and injected inputs.
pub const EMPTY_SHA256: &str = "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

/// The variants of `compile_critical_path_ms` beside "vm" (Sprint 248), which `batch_record.rs` reads by these names.
pub const LONGEST: &str = "vm longest compile";
pub const MOST_IN_A_FRAME: &str = "vm most in a 60 Hz frame";

/// A timed run's compile figures in ms (Sprint 248): the total, the longest compile, the most in a frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CompileMs {
    pub total: f64,
    pub longest: f64,
    pub most_in_a_frame: f64,
}

/// One run's facts for its execution record.
pub struct Execution<'a> {
    /// `frontier-<the subject's first 12 hex digits>-<ns>`, as `run_frontier.py` names a run
    pub id: &'a str,
    /// the program's sha256, 64 hex digits
    pub subject_sha256: &'a str,
    /// the program's file stem, for the measurements' `application_profile_id`
    pub program_stem: &'a str,
    pub timeout_seconds: u64,
    pub kill_grace_seconds: u64,
    pub executed: u64,
    /// the seconds in the VM, `hv_vcpu_run`'s calls summed, and in the host serving the exits
    pub run_seconds: f64,
    pub host_seconds: f64,
    /// the host's seconds by the exit's kind (`hvc <k>`, `vtimer`, `ec 0x<class>`), summing to `host_seconds`, as
    /// `run_frontier.py` 167-168 wrote them by hvc (Sprint 246)
    pub host_by_kind: Vec<(String, f64)>,
    /// the in-VM compiler's counts at the stop, `VMC_COUNT_NAMES` in order (`block_compiler.c` 25-27), as
    /// `vm_compile_counts` read them (vm_backend 2604-2607; Sprint 246)
    pub compile_counts: Vec<(&'a str, u64)>,
    /// the in-VM compiler's time on the critical path (Sprint 247: `w2m_vm_compile` bracketed by the virtual counter),
    /// with the longest compile and the most in a frame (Sprint 248); None on an untimed run
    pub compile_ms: Option<CompileMs>,
    /// the exits by hvc number (0: an exit other than an hvc)
    pub exits: Vec<(u16, u64)>,
    pub end: RunEnd,
    /// the call stream's sha256 (64 hex digits) and rows, named in `unobserved` (the Rust host writes no guest trace)
    pub calls: (&'a str, u64),
    pub stop: &'a str,
    pub elapsed: Duration,
    /// the 1, 5 and 15 minute load averages at the run's end, as `run_frontier.py` 173 writes them (Sprint 240); the
    /// state at its start is the baseline's `load_at_start`, which `scripts/batch-run-record.sh` writes
    pub load_at_end: Option<[f64; 3]>,
}

/// How a run ended, as `run_frontier.py` 400-407 names it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunEnd {
    /// ExitProcess, with its code: exited
    Exited(u32),
    /// the run's time limit: timed_out
    TimedOut,
    /// the VM's step budget spent (`max_steps`), a limit the harness chose and not the guest's fault: killed, as
    /// `run_frontier.py` 71 and 405 class it (`HARNESS_STOPS`; Sprint 218)
    StepLimit,
    /// the batch run's end by its reference's rule (`run_frontier.py` 306-310, "the log holds"): killed, a stop the
    /// harness chose (`HARNESS_STOPS`; Sprint 219)
    ReferenceEnd,
    /// a request the host does not serve, made inside a Windows call: SERVICE_UNIMPLEMENTED with the call ("DLL!Function")
    /// and its return address, then killed
    Unimplemented { service: String, return_address: u64 },
    /// a request the host does not serve outside any Windows call it can name (a created thread's stack growing, Sprint
    /// 211): killed, with no SERVICE_UNIMPLEMENTED, which vocabulary 0.12 keeps for a Windows service the guest called;
    /// the frozen host writes one for any refusal (vm_backend 3802-3805), which is not followed
    Unserved,
    /// anything else: faulted
    Faulted,
}

impl RunEnd {
    /// The end a stop is (Sprint 210; reviewer-3's High 2 of its review of 210; Sprint 211): a request the host does
    /// not serve inside a Windows call is a service not implemented, one outside any is unserved; a failure in serving
    /// a request is a fault, as is any stop but ExitProcess and the time limit. The call is the one `called` resolves
    /// at the word on the guest's stack (`Guest::called_service`) only when the call trace has that call in flight
    /// (`in_flight`, "DLL!Function"): a stop from a fault (a stack growing) leaves an older word there, which named
    /// KERNEL32.dll!RtlUnwindEx at Sprint 211's stop, a call the frozen host's run of that thread never made. The frozen
    /// host reads the word for every refusal (vm_backend 3804-3805), which is not followed.
    pub fn of(stop: &StopKind, called: Option<(u64, Option<String>)>, in_flight: &[String]) -> RunEnd {
        match stop {
            StopKind::ExitProcess(code) => RunEnd::Exited(*code),
            StopKind::TimeLimit => RunEnd::TimedOut,
            StopKind::StepLimit => RunEnd::StepLimit,
            StopKind::ReferenceEnd => RunEnd::ReferenceEnd,
            StopKind::Unserved => match called {
                Some((ret, Some(name))) if in_flight.contains(&name) => RunEnd::Unimplemented {
                    service: name,
                    return_address: ret,
                },
                _ => RunEnd::Unserved,
            },
            StopKind::Other => RunEnd::Faulted,
        }
    }
}

/// A run's stop by the kind its record needs, apart from `run::Stop`, which needs Hypervisor.framework and so is built
/// on the Mac only, so `RunEnd::of` and its test build on CI's Linux too (Sprint 211: CI run 40936387557 failed to
/// compile `record.rs`, "unresolved import `crate::run`").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopKind {
    ExitProcess(u32),
    /// the run's time limit
    TimeLimit,
    /// the VM's step budget
    StepLimit,
    /// the batch run's end by its reference's rule
    ReferenceEnd,
    /// a host request the host does not serve
    Unserved,
    /// anything else, a failure in serving a request among them
    Other,
}

/// `profile_slug` (run_frontier.py 371): the stem lowercased, each run of other characters one hyphen, then the
/// subject's first 12 hex digits.
pub fn application_profile(stem: &str, subject_sha256: &str) -> String {
    let mut slug = String::new();
    for c in stem.to_lowercase().chars() {
        if c.is_ascii_lowercase() || c.is_ascii_digit() {
            slug.push(c);
        } else if !slug.ends_with('-') {
            slug.push('-');
        }
    }
    format!("{}-{}", slug.trim_matches('-'), &subject_sha256[..12])
}

/// The execution record's events, as substrate lines (see the module's documentation).
pub fn execution_lines(e: &Execution) -> Vec<String> {
    use events::{
        ExecutionCompleted, ExecutionStarted, ExitStatus, HarnessI64, Metric, ProfileKind, ServiceUnimplemented,
        SubjectKind,
    };
    let id = || Id::new(e.id).expect("not empty");
    let empty = || Sha256::new(EMPTY_SHA256).expect("the empty sha256");
    let seconds = |s: u64| Some(FiniteF64::new(s as f64).expect("finite"));
    let mut lines = vec![harness_line(&SessionInit {
        vocab_version: Version::new(VOCAB_VERSION).expect("the vocabulary's own version"),
        profile: Id::new("w2m-vm").expect("a constant, not empty"),
        contract_version: Version::new(VOCAB_VERSION).expect("the vocabulary's own version"),
        harness_version: Version::new(HARNESS_VERSION).expect("Cargo's package version"),
        purpose: SessionPurpose::Application,
    })];
    lines.push(harness_line(&ExecutionStarted {
        execution_id: id(),
        subject_kind: SubjectKind::Application,
        subject_sha256: Sha256::new(format!("sha256:{}", e.subject_sha256)).expect("64 lowercase hex digits"),
        profile_kind: ProfileKind::ManagedVm,
        profile_id: Id::new("w2m-vm").expect("not empty"),
        initial_state_sha256: empty(),
        injected_inputs_sha256: empty(),
        timeout_seconds: seconds(e.timeout_seconds),
        kill_grace_seconds: seconds(e.kill_grace_seconds),
        unobserved: None,
    }));
    // run_measurements' ids and variants (run_frontier.py 159-177), for the rows the Rust host measures
    let application = application_profile(e.program_stem, e.subject_sha256);
    // the rate over the seconds in the VM and in the host, as run_frontier.py 164-165 takes it
    let wall = e.run_seconds + e.host_seconds;
    let mut rows = vec![
        (
            Metric::InstructionsRetired,
            "process".to_string(),
            e.executed as f64,
            "instructions",
        ),
        (
            Metric::InstructionsPerSecond,
            "process".to_string(),
            if wall > 0.0 { e.executed as f64 / wall } else { 0.0 },
            "instructions/s",
        ),
        (Metric::RunSeconds, "vm".to_string(), e.run_seconds, "s"),
    ];
    // the host's seconds, which the rate divides by (Sprint 240), by kind as the frozen host wrote them (Sprint 246)
    for (kind, seconds) in &e.host_by_kind {
        rows.push((Metric::RunSeconds, format!("host {kind}"), *seconds, "s"));
    }
    // the compiles at run time, step 5's baseline (Sprint 246; `run_frontier.py` 385-390)
    for (cause, n) in &e.compile_counts {
        rows.push((Metric::VmCompileBlocks, cause.to_string(), *n as f64, "blocks"));
    }
    // compilation on the critical path (v0.2 456; Sprint 247), the VM's part; the host's is `run_seconds` "host hvc 1";
    // how it bunches beside it (Sprint 248)
    if let Some(ms) = e.compile_ms {
        rows.push((Metric::CompileCriticalPathMs, "vm".to_string(), ms.total, "ms"));
        rows.push((Metric::CompileCriticalPathMs, LONGEST.to_string(), ms.longest, "ms"));
        rows.push((
            Metric::CompileCriticalPathMs,
            MOST_IN_A_FRAME.to_string(),
            ms.most_in_a_frame,
            "ms",
        ));
    }
    for (k, n) in &e.exits {
        rows.push((Metric::VmExits, format!("hvc {k}"), *n as f64, "exits"));
    }
    for (span, load) in ["1 min", "5 min", "15 min"]
        .iter()
        .zip(e.load_at_end.into_iter().flatten())
    {
        rows.push((Metric::LoadAverage, span.to_string(), load, "load"));
    }
    for (metric, variant, value, unit) in rows {
        let slug = variant.to_lowercase().replace(' ', "_");
        lines.push(measurement_line(
            &format!("{}-{}-{slug}", e.id, metric.as_str()),
            metric,
            value,
            unit,
            &application,
            Some(&variant),
        ));
    }
    if let RunEnd::Unimplemented {
        service,
        return_address,
    } = &e.end
    {
        lines.push(harness_line(&ServiceUnimplemented {
            execution_id: id(),
            service: service.clone(),
            guest_address: u(*return_address),
            instructions_executed: u(e.executed),
        }));
    }
    let (exit_status, exit_code) = match &e.end {
        RunEnd::Exited(code) => (ExitStatus::Exited, Some(*code)),
        RunEnd::TimedOut => (ExitStatus::TimedOut, None),
        RunEnd::Unimplemented { .. } | RunEnd::Unserved | RunEnd::StepLimit | RunEnd::ReferenceEnd => {
            (ExitStatus::Killed, None)
        }
        RunEnd::Faulted => (ExitStatus::Faulted, None),
    };
    let elapsed =
        FiniteF64::new((e.elapsed.as_secs_f64() * 1000.0).round() / 1000.0).expect("a measured duration is finite");
    // `guest_trace` false (0.12, P22): the Rust host writes no guest-plane trace, so the trace is the empty one and its
    // lines 0; the call stream, which `batch_run.rs` judges, is named in `unobserved`
    lines.push(harness_line(&ExecutionCompleted {
        execution_id: id(),
        exit_status,
        trace_sha256: empty(),
        guest_line_count: u(0),
        exit_code: exit_code.map(|c| HarnessI64::new(i64::from(c)).expect("a 32-bit code")),
        elapsed_seconds: Some(elapsed),
        unobserved: Some(format!(
            "no guest-plane trace: the Rust host writes the traced images' calls, {} rows, sha256 {}; the run \
             stopped: {}",
            e.calls.1, e.calls.0, e.stop
        )),
        guest_trace: Some(false),
    }));
    lines
}

/// The `q` quantile of `sorted` (ascending, not empty) by nearest rank: the smallest value with at least `q` of the
/// sample at or below it, the ceil(q·n)-th (Sprint 163, step 3's `exit_ns_p50` and `exit_ns_p99`).
pub fn quantile(sorted: &[u64], q: f64) -> u64 {
    let rank = ((q * sorted.len() as f64).ceil() as usize).clamp(1, sorted.len());
    sorted[rank - 1]
}

/// One MEASUREMENT_RECORDED line of the step-3 measurements' record (Sprint 163), in a Python batch run's conventions
/// (`destination_profile_id` `w2m-vm`, `backend` `managed_vm`, `phase` `discovery`). `artifact_state` is `cold` by the
/// frozen host's rule, "warm" only when its translation cache had hits (`run_frontier.py` 383): the Rust host keeps no
/// translation cache, so every block of every run is compiled afresh (Sprint 240; the records before it say `warm`).
pub fn measurement_line(
    measurement_id: &str,
    metric: events::Metric,
    value: f64,
    unit: &str,
    application: &str,
    variant: Option<&str>,
) -> String {
    harness_line(&events::MeasurementRecorded {
        measurement_id: Id::new(measurement_id).expect("not empty"),
        metric,
        value: FiniteF64::new(value).expect("a measured number is finite"),
        unit: unit.to_string(),
        phase: events::MeasurementPhase::Discovery,
        backend: events::Backend::ManagedVm,
        application_profile_id: Id::new(application).expect("not empty"),
        artifact_state: events::ArtifactState::Cold,
        destination_profile_id: Id::new("w2m-vm").expect("not empty"),
        variant: variant.map(str::to_string),
    })
}

/// A record's w2m events, in order: (kind, payload) of every frame whose kind is not substrate's.
pub fn w2m_events(record: &Path) -> Vec<(String, Value)> {
    frames(record)
        .into_iter()
        .filter_map(|f| {
            let kind = f["kind"].as_str()?.to_string();
            (!kind.starts_with("substrate.")).then(|| (kind, f["payload"].clone()))
        })
        .collect()
}

/// The golden check: `ours` holds `stored`'s events in order, kind for kind, field for field but for `EXEMPT`; the
/// first difference named.
pub fn compare(ours: &[(String, Value)], stored: &[(String, Value)]) -> Result<(), String> {
    if ours.is_empty() {
        return Err("the record holds no w2m event".into());
    }
    for (i, (a, b)) in ours.iter().zip(stored).enumerate() {
        if a.0 != b.0 {
            return Err(format!("event {i}: {} where the stored record has {}", a.0, b.0));
        }
        let (Some(pa), Some(pb)) = (a.1.as_object(), b.1.as_object()) else {
            return Err(format!("event {i} ({}): a payload that is not an object", a.0));
        };
        let keys = |p: &serde_json::Map<String, Value>| {
            p.keys()
                .filter(|k| !EXEMPT.contains(&k.as_str()))
                .cloned()
                .collect::<Vec<_>>()
        };
        if keys(pa) != keys(pb) {
            return Err(format!(
                "event {i} ({}): fields {:?} where the stored record has {:?}",
                a.0,
                keys(pa),
                keys(pb)
            ));
        }
        for k in keys(pa) {
            if pa[&k] != pb[&k] {
                return Err(format!(
                    "event {i} ({}): {k} is {} where the stored record has {}",
                    a.0, pa[&k], pb[&k]
                ));
            }
        }
    }
    if ours.len() != stored.len() {
        return Err(format!(
            "{} w2m events where the stored record has {}",
            ours.len(),
            stored.len()
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn events(lines: &[String]) -> Vec<(String, Value)> {
        lines
            .iter()
            .map(|l| {
                let v: Value = l.parse().unwrap();
                (v["kind"].as_str().unwrap().to_string(), v["payload"].clone())
            })
            .collect()
    }

    const IMAGE: &str = "6863d5ecd3a14fb5df0f1b194135fecd9e59a153aa2564ddfc2db034181d37de";

    /// Nearest rank: of four, the median is the second, not the third an index of n·q would take.
    #[test]
    fn the_quantile_is_the_nearest_rank() {
        let sample = [10, 20, 30, 40];
        assert_eq!(quantile(&sample, 0.5), 20, "p50");
        assert_eq!(quantile(&sample, 0.99), 40, "p99");
        assert_eq!(quantile(&[7], 0.5), 7, "one value");
    }

    #[test]
    fn a_time_is_not_compared_and_a_block_is() {
        let a = events(&translation_lines(
            IMAGE,
            0x10,
            &[(0x10, 0x20, 0x7f00)],
            Duration::from_millis(5),
        ));
        let b = events(&translation_lines(
            IMAGE,
            0x10,
            &[(0x10, 0x20, 0x7f00)],
            Duration::from_millis(9),
        ));
        assert_eq!(compare(&a, &b), Ok(()));
        let c = events(&translation_lines(
            IMAGE,
            0x10,
            &[(0x10, 0x21, 0x7f00)],
            Duration::from_millis(5),
        ));
        assert_eq!(
            compare(&a, &c),
            Err("event 2 (TRANSLATION_PUBLISHED): guest_range_end is 32 where the stored record has 33".into())
        );
        assert!(
            compare(&a[..3], &a)
                .unwrap_err()
                .contains("3 w2m events where the stored record has 4")
        );
    }

    /// `execution_lines`' four ends (run_frontier.py 400-407) and its ids (Sprint 210): an exit code is exited, the
    /// time limit timed_out, an unserved request killed after its SERVICE_UNIMPLEMENTED, anything else faulted; each
    /// measurement's id is the run's, the metric's and the variant's, its application the program's slug.
    #[test]
    fn a_run_ends_as_the_frozen_host_names_it() {
        let run = |end: RunEnd| {
            events(&execution_lines(&Execution {
                id: "frontier-643a6283510c-1",
                subject_sha256: "643a6283510c88e3483212c9d0358de925ac415b83fa0302dced6381f412f974",
                program_stem: "Hail to the Rainbow",
                timeout_seconds: 10_800,
                kill_grace_seconds: 60,
                executed: 3_541_969,
                run_seconds: 0.25,
                host_seconds: 0.75,
                host_by_kind: vec![("hvc 8".into(), 0.75)],
                compile_counts: vec![("compiled", 40), ("not a code page", 2)],
                compile_ms: None,
                exits: vec![(8, 779)],
                end,
                calls: (IMAGE, 925),
                stop: "a stop",
                elapsed: Duration::from_millis(1536),
                load_at_end: Some([1.5, 2.25, 3.0]),
            }))
        };
        let status = |events: &[(String, Value)]| {
            let done = events.iter().find(|(k, _)| k == "EXECUTION_COMPLETED").unwrap();
            done.1["exit_status"].as_str().unwrap().to_string()
        };
        let kinds = |events: &[(String, Value)]| events.iter().map(|(k, _)| k.clone()).collect::<Vec<_>>();
        let exited = run(RunEnd::Exited(0));
        assert_eq!(status(&exited), "exited");
        assert!(!kinds(&exited).contains(&"SERVICE_UNIMPLEMENTED".to_string()));
        assert_eq!(status(&run(RunEnd::TimedOut)), "timed_out");
        assert_eq!(
            status(&run(RunEnd::StepLimit)),
            "killed",
            "the step budget is the harness's stop"
        );
        assert_eq!(status(&run(RunEnd::Faulted)), "faulted");
        let unserved = run(RunEnd::Unserved);
        assert_eq!(status(&unserved), "killed");
        assert!(!kinds(&unserved).contains(&"SERVICE_UNIMPLEMENTED".to_string()));
        let killed = run(RunEnd::Unimplemented {
            service: "KERNEL32.dll!MapViewOfFile".into(),
            return_address: 0x11_8002_72bf,
        });
        assert_eq!(status(&killed), "killed");
        // P22: no guest trace, so the empty one and 0 lines; the call stream's rows named
        let done = &killed.iter().find(|(k, _)| k == "EXECUTION_COMPLETED").unwrap().1;
        assert_eq!(
            (done["trace_sha256"].as_str(), done["guest_line_count"].as_u64()),
            (Some(EMPTY_SHA256), Some(0))
        );
        assert!(done["unobserved"].as_str().unwrap().contains("925 rows"));
        assert_eq!(
            kinds(&killed),
            [
                "SESSION_INIT",
                "EXECUTION_STARTED",
                // instructions, the rate, the VM's seconds, the host's on hvc 8, two compile counts, the exits, the
                // three load averages
                "MEASUREMENT_RECORDED",
                "MEASUREMENT_RECORDED",
                "MEASUREMENT_RECORDED",
                "MEASUREMENT_RECORDED",
                "MEASUREMENT_RECORDED",
                "MEASUREMENT_RECORDED",
                "MEASUREMENT_RECORDED",
                "MEASUREMENT_RECORDED",
                "MEASUREMENT_RECORDED",
                "MEASUREMENT_RECORDED",
                "SERVICE_UNIMPLEMENTED",
                "EXECUTION_COMPLETED"
            ],
            "the session's close is the judge's Producer's (Sprint 217)"
        );
        // the rate over the VM's and the host's seconds: 3,541,969 instructions in 1 s
        assert_eq!(killed[3].1["value"], 3_541_969.0);
        // the host's seconds the rate divides by, by kind (Sprints 240, 246), the compile counts (Sprint 246), and the
        // load at the end (Sprint 240)
        assert_eq!(
            killed[5].1["measurement_id"],
            "frontier-643a6283510c-1-run_seconds-host_hvc_8"
        );
        assert_eq!(killed[5].1["value"], 0.75);
        assert_eq!(
            (
                killed[7].1["metric"].as_str(),
                killed[7].1["variant"].as_str(),
                killed[7].1["value"].as_f64()
            ),
            (Some("vm_compile_blocks"), Some("not a code page"), Some(2.0))
        );
        assert_eq!(killed[7].1["unit"], "blocks");
        let exit_row = &killed[8].1;
        assert_eq!(exit_row["measurement_id"], "frontier-643a6283510c-1-vm_exits-hvc_8");
        assert_eq!(exit_row["application_profile_id"], "hail-to-the-rainbow-643a6283510c");
        assert_eq!(exit_row["artifact_state"], "cold");
        assert_eq!(
            (
                killed[11].1["metric"].as_str(),
                killed[11].1["variant"].as_str(),
                killed[11].1["value"].as_f64()
            ),
            (Some("load_average"), Some("15 min"), Some(3.0))
        );
        assert_eq!(killed[12].1["instructions_executed"], 3_541_969);
        assert_eq!(killed[12].1["guest_address"], 0x11_8002_72bfu64);
    }

    /// The in-VM compile time (Sprint 247): a timed run writes `compile_critical_path_ms` "vm" in ms, with the longest
    /// compile and the most in a frame (Sprint 248), an untimed run none; the gates' run is untimed, so this test is
    /// what holds the writer. The frame's variant names the monitor code's frames a second.
    #[test]
    fn a_timed_run_writes_its_compile_time() {
        assert!(MOST_IN_A_FRAME.contains(&format!(" {} Hz ", crate::monitor_code::FRAMES_A_SECOND)));
        let times = |compile_ms: Option<CompileMs>| {
            events(&execution_lines(&Execution {
                id: "frontier-643a6283510c-1",
                subject_sha256: "643a6283510c88e3483212c9d0358de925ac415b83fa0302dced6381f412f974",
                program_stem: "Hail to the Rainbow",
                timeout_seconds: 10_800,
                kill_grace_seconds: 60,
                executed: 3_541_969,
                run_seconds: 0.25,
                host_seconds: 0.75,
                host_by_kind: vec![("hvc 1".into(), 0.75)],
                compile_counts: vec![("compiled", 40)],
                compile_ms,
                exits: vec![(1, 40)],
                end: RunEnd::Exited(0),
                calls: (IMAGE, 925),
                stop: "a stop",
                elapsed: Duration::from_millis(1536),
                load_at_end: None,
            }))
            .into_iter()
            .filter(|(k, p)| k == "MEASUREMENT_RECORDED" && p["metric"] == "compile_critical_path_ms")
            .map(|(_, p)| (p["variant"].clone(), p["value"].clone(), p["unit"].clone()))
            .collect::<Vec<_>>()
        };
        let ms = CompileMs {
            total: 1725.5,
            longest: 2.25,
            most_in_a_frame: 9.5,
        };
        assert_eq!(
            times(Some(ms)),
            vec![
                ("vm".into(), 1725.5.into(), "ms".into()),
                (LONGEST.into(), 2.25.into(), "ms".into()),
                (MOST_IN_A_FRAME.into(), 9.5.into(), "ms".into()),
            ]
        );
        assert_eq!(times(None), vec![]);
    }

    /// `RunEnd::of`: an unserved request inside a Windows call names the call; one outside any call it can name (no
    /// call resolved, or no readable return address) is unserved, with no service; a failure in serving a request and
    /// any other stop are faults; ExitProcess and the time limit are their own ends.
    #[test]
    fn a_stop_is_the_end_its_kind_names() {
        let unserved = StopKind::Unserved;
        let call = || Some((0x11_8002_72bf, Some("KERNEL32.dll!MapViewOfFile".to_string())));
        let flying = ["KERNEL32.dll!MapViewOfFile".to_string()];
        assert_eq!(
            RunEnd::of(&unserved, call(), &flying),
            RunEnd::Unimplemented {
                service: "KERNEL32.dll!MapViewOfFile".into(),
                return_address: 0x11_8002_72bf
            }
        );
        // the word on the stack names a call the trace does not have in flight: Sprint 211's stop, a stack growing
        let stale = Some((0x11_8032_a699, Some("KERNEL32.dll!RtlUnwindEx".to_string())));
        assert_eq!(RunEnd::of(&unserved, stale, &[]), RunEnd::Unserved);
        assert_eq!(RunEnd::of(&unserved, Some((0x40, None)), &flying), RunEnd::Unserved);
        assert_eq!(RunEnd::of(&unserved, None, &flying), RunEnd::Unserved);
        // a failure in serving a request is `Other` (`Stop::HostFailure`'s kind), a fault even inside a call in flight
        assert_eq!(RunEnd::of(&StopKind::Other, call(), &flying), RunEnd::Faulted);
        assert_eq!(RunEnd::of(&StopKind::ExitProcess(3), None, &[]), RunEnd::Exited(3));
        assert_eq!(RunEnd::of(&StopKind::TimeLimit, None, &[]), RunEnd::TimedOut);
    }
}
