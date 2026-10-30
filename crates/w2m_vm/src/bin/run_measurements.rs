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
//!
//! The record: SESSION_INIT (purpose measurement), a MEASUREMENT_RECORDED per number, SESSION_COMPLETED. A child that
//! fails, a guest whose served exits do not add up to its own exit tally less its last, or a timed run with more
//! takings than the timing area holds or a time that ran backward fails the Producer by name.

use std::collections::BTreeMap;
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use std::time::Instant;

use events::{
    FiniteF64, HarnessU64, Id, Metric, SessionCompleted, SessionInit, SessionOutcome, SessionPurpose, VOCAB_VERSION,
    Version, harness_line,
};
use w2m_vm::monitor_code::Site;
use w2m_vm::record::{measurement_line, quantile};

const GUESTS: [&str; 2] = ["qpc", "sleeper"];

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
        // the served hvc exits are the run's own tally less its last exit, which ended the run unserved
        let tally: u64 = json["exits"]
            .as_object()
            .ok_or("no exits")?
            .values()
            .filter_map(|v| v.as_u64())
            .sum();
        let served: u64 = kinds
            .iter()
            .filter(|(k, _)| k.starts_with("hvc "))
            .map(|(_, v)| v.len() as u64)
            .sum();
        if served + 1 != tally {
            return Err(format!("{guest}: {served} served hvc exits, the run's tally {tally}"));
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
