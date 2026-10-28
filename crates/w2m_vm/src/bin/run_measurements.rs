// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! `run_measurements` (Sprint 163): two of step 3's measurements (`docs/rust-port-plan-2026-10-24.md` step 3, amended
//! 2026-10-27) on a substrate record, as a Producer (`harness/w2m-vm-measurements.toml`). One VM a process, so it runs
//! each measurement as a `w2m-vm` child through `scripts/sign_and_run.sh`:
//! - `qpc` and `sleeper` (`tests/data/timed_waits`), each to ExitProcess with `--serve-times`: per exit kind, the
//!   served exits' host time as `exit_ns_p50` and `exit_ns_p99` (nearest rank) and their count as `vm_exits`;
//! - `--wfe-probe`: `wfe_exits` for `wfe` and `wfi`, the WFx traps of 1,000 each.
//!
//! The record: SESSION_INIT (purpose measurement), a MEASUREMENT_RECORDED per number, SESSION_COMPLETED. A child that
//! fails, or a guest whose served exits do not add up to its own exit tally less its last, fails the Producer by name.

use std::collections::BTreeMap;
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use std::time::Instant;

use events::{
    FiniteF64, HarnessU64, Id, Metric, SessionCompleted, SessionInit, SessionOutcome, SessionPurpose, VOCAB_VERSION,
    Version, harness_line,
};
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
