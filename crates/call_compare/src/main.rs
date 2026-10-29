// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! The batch run's comparison against Windows as a Producer (Sprint 217; reviewer-3's Medium 5 of its review of Sprints
//! 212-214): a trigger on the run's EXECUTION_COMPLETED starts it with that event's payload as its one input line
//! (substrate `adapters/topology_file.py` 140). It reads the call stream `w2m-vm` wrote and the stop file's
//! `calls dropped` row, judges the stream (`Windows::judge`), and writes COMPARISON_COMPLETED and the session's
//! SESSION_COMPLETED, which check_records' T2 holds last.
//!
//!   call_compare --root DIR --calls FILE --strings FILE --stop FILE
//!
//! Each FILE is under DIR, the repository root. A run whose `exit_status` is not `exited` stopped before ExitProcess.
//! SESSION_COMPLETED's `elapsed_seconds` is the execution's `elapsed_seconds` and this Producer's own, a lower bound:
//! `w2m-vm`'s execution clock starts before the depot is read and loaded, so it holds the setup; the sum leaves out the
//! process start and signing before that clock (`scripts/sign_and_run.sh`) and substrate's dispatch to this Producer,
//! which the trigger's input does not carry. Its `outcome` follows the verdict (`Verdict::outcome`). Its
//! `discrepancy_count` counts DISCREPANCY_REPORTED, as the vocabulary's note says, and this Producer writes none: 0, as
//! Sprint 214's record wrote; `run_frontier.py`'s closer counts the divergences there, which is not followed.

use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};
use std::time::Instant;

use call_compare::{Stream, Windows};
use events::{
    ComparisonCompleted, ContractId, FiniteF64, HarnessU64, Id, Relation, SessionCompleted, Version, harness_line,
};
use serde_json::Value;

struct Arguments {
    root: PathBuf,
    calls: PathBuf,
    strings: PathBuf,
    stop: PathBuf,
}

fn arguments() -> Result<Arguments, String> {
    let mut found: [Option<String>; 4] = Default::default();
    let names = ["--root", "--calls", "--strings", "--stop"];
    let mut args = std::env::args().skip(1);
    while let Some(flag) = args.next() {
        let i = names
            .iter()
            .position(|n| *n == flag)
            .ok_or(format!("unknown argument {flag:?}"))?;
        found[i] = Some(args.next().ok_or(format!("{flag} takes a path"))?);
    }
    let [root, calls, strings, stop] = found;
    let need = |v: Option<String>, name: &str| v.ok_or(format!("{name} is required"));
    let root = PathBuf::from(need(root, "--root")?);
    Ok(Arguments {
        calls: root.join(need(calls, "--calls")?),
        strings: root.join(need(strings, "--strings")?),
        stop: root.join(need(stop, "--stop")?),
        root,
    })
}

fn text(path: &Path) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))
}

/// The stop file's `calls dropped` row (`w2m-vm --game`'s), the calls the full trace left out.
fn dropped(stop: &str) -> Result<u64, String> {
    stop.lines()
        .find_map(|l| l.strip_prefix("calls dropped\t"))
        .ok_or("the stop file has no calls dropped row")?
        .parse()
        .map_err(|e| format!("calls dropped: {e}"))
}

fn run() -> Result<(), String> {
    let started = Instant::now();
    let a = arguments()?;
    let mut line = String::new();
    io::stdin().lock().read_line(&mut line).map_err(|e| e.to_string())?;
    let completed: Value = serde_json::from_str(&line).map_err(|e| format!("the input line: {e}"))?;
    let execution = completed["execution_id"]
        .as_str()
        .ok_or("the input names no execution_id")?;
    let status = completed["exit_status"]
        .as_str()
        .ok_or("the input names no exit_status")?;
    let execution_seconds = completed["elapsed_seconds"]
        .as_f64()
        .ok_or("the input names no elapsed_seconds")?;
    let windows = Windows::load(&a.root)?;
    let (calls, strings) = (text(&a.calls)?, text(&a.strings)?);
    let verdict = windows.judge(&Stream {
        calls: &calls,
        strings: &strings,
        stopped: status != "exited",
        dropped: dropped(&text(&a.stop)?)?,
    })?;
    let comparison = ComparisonCompleted {
        comparison_id: Id::new(format!("cmp-set-{execution}")).map_err(|e| e.to_string())?,
        contract_id: ContractId::new(windows.contract_id.clone()).map_err(|e| e.to_string())?,
        contract_version: Version::new(windows.contract_version.clone()).map_err(|e| e.to_string())?,
        relation: Relation::StateRelation,
        reference_ref: Id::new(windows.reference_ref()).map_err(|e| e.to_string())?,
        target_execution_id: Id::new(execution).map_err(|e| e.to_string())?,
        verdict: verdict.verdict(),
        rule_ref: None,
        first_divergence: verdict.first_divergence(),
    };
    let seconds = execution_seconds + started.elapsed().as_secs_f64();
    let zero = HarnessU64::new(0).expect("0 is in range");
    let close = SessionCompleted {
        outcome: verdict.outcome(),
        discrepancy_count: zero,
        accepted_count: zero,
        elapsed_seconds: FiniteF64::new((seconds * 1000.0).round() / 1000.0).map_err(|e| e.to_string())?,
    };
    let mut out = io::stdout().lock();
    for l in [harness_line(&comparison), harness_line(&close)] {
        writeln!(out, "{l}").map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn main() {
    if let Err(why) = run() {
        eprintln!("call_compare: {why}");
        std::process::exit(1);
    }
}
