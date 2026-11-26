// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! `run_measurements` (Sprint 163): two of step 3's measurements (`docs/rust-port-plan-2026-10-24.md` step 3, amended
//! 2026-10-27) on a substrate record, as a Producer (`harness/w2m-vm-measurements.toml`). One VM a process, so it runs
//! each measurement as a `w2m-vm` child through `scripts/sign_and_run.sh`:
//! - `qpc` and `sleeper` (`tests/data/timed_waits`), each to ExitProcess with `--serve-times`: per exit kind, the
//!   served exits' host time as `exit_ns_p50` and `exit_ns_p99` (nearest rank) and their count as `vm_exits`;
//! - each guest again with `--runtime-word` (Sprint 164): the runtime word's hold and wait per taking, by the site
//!   that took it, as `runtime_word_hold_ns_p50/p99` and `runtime_word_wait_ns_p50/p99` (nearest rank over the exact
//!   ticks), the takings an exit interrupted apart;
//! - `--wfe-probe`: `wfe_exits` for `wfe` and `wfi`, the WFx traps of 1,000 each.
//! - step 6's two-vCPU run (Sprint 337): `probes/locked_race`, the binary `store/probe-runs/41326239822/probe.sha256`
//!   names, with `--vcpus 2 --vcpu-figures`: `locked_model_steps` (`count`) and `turn_start_ns_p50` and
//!   `turn_start_ns_p99` (`ns`, nearest rank over each turn's start), each with `vcpus` 2;
//! - the runtime word on two vCPUs while the waits and releases run (Sprint 356, its amendment 2):
//!   `tests/data/timed_waits/semaphore_units` with `--vcpus 2 --runtime-word`, its output "done": the word's hold and
//!   wait per taking by site, each with `vcpus` 2, over the run's first `LOCK_TAKINGS` takings, the takings counted
//!   named in each variant;
//! - the capabilities `vcpus_allowed` and the run-loop design read (Sprint 337), one CAPABILITY_PROBED each:
//!   `feat_lse2` and `feat_bbm_level2` from `w2m-vm --capabilities`, `guest_wfe_exit` from the WFE probe's run.
//!
//! The record: SESSION_INIT (purpose measurement), a MEASUREMENT_RECORDED per number, the three CAPABILITY_PROBED,
//! SESSION_COMPLETED. A child that fails, a guest whose served exits do not add up to its own exit tally less its last,
//! a timed run with more takings than the timing area holds or a time that ran backward, or a two-vCPU run whose rows
//! are not the Windows run's, one of whose vCPUs never ran, or which timed no turn fails the Producer by name.

use std::collections::BTreeMap;
use std::io::{self, BufRead, Write};
use std::num::NonZeroU64;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use std::time::Instant;

use events::{
    Capability, CapabilityOutcome, CapabilityProbed, FiniteF64, HarnessU64, Id, Metric, SessionCompleted, SessionInit,
    SessionOutcome, SessionPurpose, VOCAB_VERSION, Version, harness_line,
};
use w2m_vm::monitor_code::Site;
use w2m_vm::record::{measurement_line, measurement_line_on, quantile};

const GUESTS: [&str; 2] = ["qpc", "sleeper"];
/// The Windows run whose `locked_race` binary and rows the two-vCPU run is held to (Sprint 331).
const LOCKED_RACE_RUN: &str = "store/probe-runs/41326239822";
/// The vCPUs of step 6's run.
const TWO: u32 = 2;

/// This host as the frozen Python's probes named it (`scripts/litmus_mp.py` 59 at the tag frozen-python-2026-10-29):
/// `mac-<hw.model>-macos-<build>`, the build `sw_vers -buildVersion`.
fn host_id() -> Result<String, String> {
    let said = |program: &str, args: &[&str]| -> Result<String, String> {
        let out = Command::new(program)
            .args(args)
            .output()
            .map_err(|e| format!("{program}: {e}"))?;
        if !out.status.success() {
            return Err(format!("{program} {args:?}: {}", out.status));
        }
        Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
    };
    Ok(format!(
        "mac-{}-macos-{}",
        said("sysctl", &["-n", "hw.model"])?,
        said("sw_vers", &["-buildVersion"])?
    ))
}

/// One CAPABILITY_PROBED of this host.
fn probed(capability: Capability, host: &str, outcome: CapabilityOutcome, detail: String) -> String {
    harness_line(&CapabilityProbed {
        capability,
        host_id: Id::new(host).expect("not empty"),
        outcome,
        detail,
        execution_id: None,
    })
}

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

/// One `w2m-vm` child, signed with the Hypervisor entitlement; its standard output.
fn child(root: &Path, args: &[&str]) -> Result<String, String> {
    let mut c = Command::new(root.join("scripts/sign_and_run.sh"))
        .arg(root.join("target/release/w2m-vm"))
        .args(args)
        .current_dir(root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("w2m-vm {args:?}: {e}"))?;
    // the input line `w2m-vm` reads as a Producer would
    c.stdin.take().unwrap().write_all(b"{}\n").map_err(|e| e.to_string())?;
    let out = c.wait_with_output().map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Err(format!(
            "w2m-vm {args:?}: {}: {}",
            out.status,
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

fn run() -> Result<(), String> {
    let started = Instant::now();
    let mut input = String::new();
    io::stdin().lock().read_line(&mut input).map_err(|e| e.to_string())?;
    let root = root();
    std::fs::create_dir_all(root.join("build/measurements")).map_err(|e| e.to_string())?;
    let mut lines = vec![harness_line(&SessionInit {
        vocab_version: Version::new(VOCAB_VERSION).expect("the vocabulary's own version"),
        profile: Id::new("w2m-vm-measurements").expect("not empty"),
        contract_version: Version::new("0.0.0").expect("a version"),
        harness_version: Version::new(env!("CARGO_PKG_VERSION")).expect("Cargo's package version"),
        purpose: SessionPurpose::Measurement,
    })];

    for guest in GUESTS {
        let times = format!("build/measurements/{guest}-serve.json");
        let exe = format!("tests/data/timed_waits/{guest}.exe");
        let output = format!("build/measurements/{guest}-stdout.txt");
        child(&root, &[&exe, "--output", &output, "--serve-times", &times])?;
        let json: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(root.join(&times)).map_err(|e| format!("{times}: {e}"))?)
                .map_err(|e| format!("{times}: {e}"))?;
        let mut kinds: BTreeMap<String, Vec<u64>> = BTreeMap::new();
        for (kind, ns) in json["kinds"].as_object().ok_or("no kinds")? {
            let mut v: Vec<u64> = ns
                .as_array()
                .ok_or("kind not a list")?
                .iter()
                .filter_map(|x| x.as_u64())
                .collect();
            v.sort_unstable();
            kinds.insert(kind.clone(), v);
        }
        // the served hvc exits are the run's own tally less its last exit, which ended the run unserved, and less each
        // flush stub's `hvc #15` (Sprint 305), an exit inside a split's service that the run loop tallies (`run.rs`'s
        // turn start) and times as that service's part, not as its own (Sprint 337: sleeper's three flushes failed the
        // run, 22 served of a tally of 26)
        let exits = json["exits"].as_object().ok_or("no exits")?;
        let tally: u64 = exits.values().filter_map(|v| v.as_u64()).sum();
        let flushes = exits
            .get(&w2m_vm::monitor_code::HVC_TLB_FLUSHED.to_string())
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let served: u64 = kinds
            .iter()
            .filter(|(k, _)| k.starts_with("hvc "))
            .map(|(_, v)| v.len() as u64)
            .sum();
        if served + 1 + flushes != tally {
            return Err(format!(
                "{guest}: {served} served hvc exits and {flushes} flushes, the run's tally {tally}"
            ));
        }
        let application = format!("timed-waits-{guest}");
        for (kind, ns) in &kinds {
            let tag = kind.replace(' ', "-");
            lines.push(measurement_line(
                &format!("{guest}-exits-{tag}"),
                Metric::VmExits,
                ns.len() as f64,
                "exits",
                &application,
                Some(kind),
            ));
            for (metric, q, name) in [(Metric::ExitNsP50, 0.5, "p50"), (Metric::ExitNsP99, 0.99, "p99")] {
                lines.push(measurement_line(
                    &format!("{guest}-exit-ns-{name}-{tag}"),
                    metric,
                    quantile(ns, q) as f64,
                    "ns",
                    &application,
                    Some(kind),
                ));
            }
        }

        // the runtime word's hold and wait times (Sprint 164): a second run with the takings timed in the guest, apart
        // from the first, so the timing's few instructions are not in the exit times above
        let word = format!("build/measurements/{guest}-word.json");
        let timed_output = format!("build/measurements/{guest}-timed-stdout.txt");
        child(&root, &[&exe, "--output", &timed_output, "--runtime-word", &word])?;
        let (plain, timed) = (
            std::fs::read(root.join(&output)).map_err(|e| e.to_string())?,
            std::fs::read(root.join(&timed_output)).map_err(|e| e.to_string())?,
        );
        if guest == "sleeper" && plain != timed {
            return Err(format!(
                "{guest}: the timed run's output differs from the untimed run's"
            ));
        }
        let json: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(root.join(&word)).map_err(|e| format!("{word}: {e}"))?)
                .map_err(|e| format!("{word}: {e}"))?;
        let counted = json["counted"].as_u64().ok_or(format!("{word}: no count"))?;
        let takings: Vec<[u64; 7]> =
            serde_json::from_value(json["takings"].clone()).map_err(|e| format!("{word}: {e}"))?;
        if counted != takings.len() as u64 {
            return Err(format!(
                "{guest}: {counted} takings counted, {} written: the timing area is full",
                takings.len()
            ));
        }
        // [site, wait start, taken, released, exits at each]: a time that ran backward fails the run by name
        let backward = takings.iter().filter(|t| t[2] < t[1] || t[3] < t[2]).count();
        if backward != 0 {
            return Err(format!("{guest}: {counted} takings, {backward} backward"));
        }
        let ns_per_tick = 1e9 / w2m_vm::host_state::counter_frequency()? as f64;
        for site in Site::ALL {
            let at: Vec<&[u64; 7]> = takings.iter().filter(|t| t[0] == site as u64).collect();
            // a hold or a wait within one run of the vCPU, apart from one an exit interrupted, which holds the host's
            // time too (a bridge's C service exits by hvc #8 while it holds the word)
            for (which, [p50, p99], start, end) in [
                (
                    "hold",
                    [Metric::RuntimeWordHoldNsP50, Metric::RuntimeWordHoldNsP99],
                    2,
                    3,
                ),
                (
                    "wait",
                    [Metric::RuntimeWordWaitNsP50, Metric::RuntimeWordWaitNsP99],
                    1,
                    2,
                ),
            ] {
                for across in [false, true] {
                    let mut ticks: Vec<u64> = at
                        .iter()
                        .filter(|t| (t[start + 3] != t[end + 3]) == across)
                        .map(|t| t[end] - t[start])
                        .collect();
                    if ticks.is_empty() {
                        continue;
                    }
                    ticks.sort_unstable();
                    let n = ticks.len();
                    let (tag, variant) = if across {
                        ("-across-exits", format!("{}; across an exit; {n} takings", site.name()))
                    } else {
                        ("", format!("{}; {n} takings", site.name()))
                    };
                    for (metric, q, name) in [(p50, 0.5, "p50"), (p99, 0.99, "p99")] {
                        lines.push(measurement_line(
                            &format!("{guest}-runtime-word-{}-{which}-ns-{name}{tag}", site.name()),
                            metric,
                            (quantile(&ticks, q) as f64 * ns_per_tick).round(),
                            "ns",
                            &application,
                            Some(&variant),
                        ));
                    }
                }
            }
        }
    }

    let probe: serde_json::Value =
        serde_json::from_str(child(&root, &["--wfe-probe"])?.trim()).map_err(|e| format!("the WFE probe: {e}"))?;
    for which in ["wfe", "wfi"] {
        let exits = probe[which]["exits"]
            .as_u64()
            .ok_or(format!("the probe gave no {which} exits"))?;
        if probe[which]["ended"] != true {
            return Err(format!("the probe's {which} run did not reach its hvc #0"));
        }
        lines.push(measurement_line(
            &format!("wfe-probe-{which}"),
            Metric::WfeExits,
            exits as f64,
            "count",
            "wfe-probe",
            Some(which),
        ));
    }

    // step 6's two-vCPU run (Sprint 337): locked_race, its rows held to the Windows run's as `run_loop.rs`'s test
    // holds them, so a run that lost an add is not measured
    let binary = std::fs::read_to_string(root.join(LOCKED_RACE_RUN).join("probe.sha256"))
        .map_err(|e| format!("{LOCKED_RACE_RUN}/probe.sha256: {e}"))?;
    let program = format!("store/binaries/{}.exe", binary.trim());
    let (race_output, race_figures) = (
        "build/measurements/locked_race-stdout.txt",
        "build/measurements/locked_race-vcpus.json",
    );
    let two = TWO.to_string();
    child(
        &root,
        &[
            &program,
            "--vcpus",
            &two,
            "--output",
            race_output,
            "--vcpu-figures",
            race_figures,
        ],
    )?;
    let windows = std::fs::read(root.join(LOCKED_RACE_RUN).join("body-1.jsonl")).map_err(|e| e.to_string())?;
    if std::fs::read(root.join(race_output)).map_err(|e| format!("{race_output}: {e}"))? != windows {
        return Err(format!(
            "locked_race on {TWO} vCPUs: its rows are not Windows run {LOCKED_RACE_RUN}'s"
        ));
    }
    let figures: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(root.join(race_figures)).map_err(|e| format!("{race_figures}: {e}"))?,
    )
    .map_err(|e| format!("{race_figures}: {e}"))?;
    // the count the run was given, which every two-vCPU row carries; nonzero by type (E14)
    let vcpus = figures["vcpus"]
        .as_u64()
        .and_then(NonZeroU64::new)
        .ok_or(format!("{race_figures}: no vCPU count above 0"))?;
    let runs: Vec<u64> = serde_json::from_value(figures["runs_by_vcpu"].clone()).map_err(|e| e.to_string())?;
    if runs.len() as u64 != vcpus.get() || runs.contains(&0) {
        return Err(format!(
            "locked_race on {vcpus} vCPUs: runs by vCPU {runs:?}, a vCPU that never ran"
        ));
    }
    let locked = figures["locked_steps"]
        .as_u64()
        .ok_or(format!("{race_figures}: no locked steps"))?;
    let mut turns: Vec<u64> = serde_json::from_value(figures["turn_start_ns"].clone()).map_err(|e| e.to_string())?;
    if turns.is_empty() {
        return Err(format!("locked_race on {vcpus} vCPUs: no turn's start timed"));
    }
    turns.sort_unstable();
    let race = "locked-race";
    lines.push(measurement_line_on(
        vcpus,
        "locked-race-locked-model-steps",
        Metric::LockedModelSteps,
        locked as f64,
        "count",
        race,
        None,
    ));
    let n = turns.len();
    for (metric, q, name) in [
        (Metric::TurnStartNsP50, 0.5, "p50"),
        (Metric::TurnStartNsP99, 0.99, "p99"),
    ] {
        lines.push(measurement_line_on(
            vcpus,
            &format!("locked-race-turn-start-ns-{name}"),
            metric,
            quantile(&turns, q) as f64,
            "ns",
            race,
            Some(&format!("{n} turns")),
        ));
    }

    // the runtime word on two vCPUs while the waits and releases run (Sprint 356, its amendment 2):
    // `tests/data/timed_waits/semaphore_units`, its output "done", with the takings timed; its takings pass the timing
    // area's LOCK_TAKINGS, so the figures are over the takings the area holds, the run's first, and each variant names
    // them and the takings counted
    let (units_output, units_word) = (
        "build/measurements/semaphore_units-stdout.txt",
        "build/measurements/semaphore_units-word.json",
    );
    child(
        &root,
        &[
            "tests/data/timed_waits/semaphore_units.exe",
            "--vcpus",
            &two,
            "--output",
            units_output,
            "--runtime-word",
            units_word,
        ],
    )?;
    if std::fs::read(root.join(units_output)).map_err(|e| format!("{units_output}: {e}"))? != b"done\n" {
        return Err(format!("semaphore_units on {TWO} vCPUs: its output is not \"done\""));
    }
    let json: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(root.join(units_word)).map_err(|e| format!("{units_word}: {e}"))?,
    )
    .map_err(|e| format!("{units_word}: {e}"))?;
    let counted = json["counted"].as_u64().ok_or(format!("{units_word}: no count"))?;
    let takings: Vec<[u64; 7]> =
        serde_json::from_value(json["takings"].clone()).map_err(|e| format!("{units_word}: {e}"))?;
    if takings.len() as u64 != counted.min(w2m_vm::monitor_code::LOCK_TAKINGS) {
        return Err(format!(
            "semaphore_units on {TWO} vCPUs: {counted} takings counted, {} written",
            takings.len()
        ));
    }
    // a hold, taken to released, is written by the word's holder alone; a wait's start is the timing area's one header
    // word (+0), which a second vCPU waiting writes over while the first holds the word, moving the start later: so on
    // two vCPUs a wait figure is a lower bound, not trusted, a wait whose start passed its taking (the overwrite's
    // evidence, counted in each variant) is left out, and the holds and the takings counted are the figures (a wait
    // start a vCPU is the per-lock vocabulary card's)
    if takings.iter().any(|t| t[3] < t[2]) {
        return Err(format!("semaphore_units on {TWO} vCPUs: a hold's time ran backward"));
    }
    let overwritten = takings.iter().filter(|t| t[2] < t[1]).count();
    let ns_per_tick = 1e9 / w2m_vm::host_state::counter_frequency()? as f64;
    let units = "timed-waits-semaphore-units";
    for site in Site::ALL {
        let at: Vec<&[u64; 7]> = takings.iter().filter(|t| t[0] == site as u64).collect();
        for (which, [p50, p99], start, end) in [
            (
                "hold",
                [Metric::RuntimeWordHoldNsP50, Metric::RuntimeWordHoldNsP99],
                2,
                3,
            ),
            (
                "wait",
                [Metric::RuntimeWordWaitNsP50, Metric::RuntimeWordWaitNsP99],
                1,
                2,
            ),
        ] {
            let mut ticks: Vec<u64> = at
                .iter()
                .filter(|t| t[end] >= t[start])
                .map(|t| t[end] - t[start])
                .collect();
            if ticks.is_empty() {
                continue;
            }
            ticks.sort_unstable();
            let left_out = if which == "wait" {
                format!(
                    "; a lower bound, untrusted: another vCPU's wait writes the one wait start, {overwritten} of the \
                     {} passing their taking, left out",
                    takings.len()
                )
            } else {
                String::new()
            };
            let variant = format!(
                "{}; {} of the run's first {} takings, of {counted} counted{left_out}",
                site.name(),
                ticks.len(),
                takings.len()
            );
            for (metric, q, name) in [(p50, 0.5, "p50"), (p99, 0.99, "p99")] {
                lines.push(measurement_line_on(
                    vcpus,
                    &format!("semaphore-units-runtime-word-{}-{which}-ns-{name}", site.name()),
                    metric,
                    (quantile(&ticks, q) as f64 * ns_per_tick).round(),
                    "ns",
                    units,
                    Some(&variant),
                ));
            }
        }
    }

    // the capabilities `vcpus_allowed` and the run-loop design read (Sprint 337; vocabulary 0.13, P28)
    let host = host_id()?;
    let capabilities: serde_json::Value = serde_json::from_str(child(&root, &["--capabilities"])?.trim())
        .map_err(|e| format!("w2m-vm --capabilities: {e}"))?;
    let (outcome, detail) = match capabilities["feat_lse2"].as_i64() {
        Some(1) => (CapabilityOutcome::Present, "hw.optional.arm.FEAT_LSE2 is 1".to_string()),
        Some(v) => (CapabilityOutcome::Absent, format!("hw.optional.arm.FEAT_LSE2 is {v}")),
        // a kernel that names no such feature shows no absence (v0.2 8.1)
        None => (
            CapabilityOutcome::NotEstablished,
            "sysctlbyname names no hw.optional.arm.FEAT_LSE2".to_string(),
        ),
    };
    lines.push(probed(
        Capability::FeatLse2,
        &host,
        outcome,
        format!("{detail} (sysctlbyname), which vcpus_allowed reads before it admits a second vCPU"),
    ));
    let level = capabilities["feat_bbm_level"]
        .as_u64()
        .ok_or("w2m-vm --capabilities: no FEAT_BBM level")?;
    let register = capabilities["id_aa64mmfr2_el1"].as_str().unwrap_or("unread");
    lines.push(probed(
        Capability::FeatBbmLevel2,
        &host,
        if level >= 2 {
            CapabilityOutcome::Present
        } else {
            CapabilityOutcome::Absent
        },
        format!(
            "FEAT_BBM level {level}: ID_AA64MMFR2_EL1 {register}, bits 55:52, as a vCPU reads it; level 2 or more lets \
             a change of block size alone go without a break"
        ),
    ));
    let wfe = probe["wfe"]["exits"].as_u64().ok_or("the probe gave no wfe exits")?;
    lines.push(probed(
        Capability::GuestWfeExit,
        &host,
        if wfe > 0 {
            CapabilityOutcome::Present
        } else {
            CapabilityOutcome::Absent
        },
        format!(
            "the WFE probe's run of 1,000 WFE ended at its hvc #0 with {wfe} WFx traps (ESR EC 0x01) to the host; its \
             run of 1,000 WFI with {}",
            probe["wfi"]["exits"]
        ),
    ));

    let zero = HarnessU64::new(0).expect("0");
    lines.push(harness_line(&SessionCompleted {
        outcome: SessionOutcome::Completed,
        discrepancy_count: zero,
        accepted_count: zero,
        elapsed_seconds: FiniteF64::new((started.elapsed().as_secs_f64() * 1000.0).round() / 1000.0)
            .expect("a measured duration is finite"),
    }));
    let mut out = io::stdout().lock();
    for line in lines {
        writeln!(out, "{line}").map_err(|e| e.to_string())?;
    }
    out.flush().map_err(|e| e.to_string())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("run_measurements: {e}");
            ExitCode::FAILURE
        }
    }
}
