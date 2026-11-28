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
//!
//! From Sprint 375 the reference set is the rules' (`call_compare::reference_set_of`), and three more commands serve it:
//!
//!   call_compare reference-set ROOT          the set, each stored game run left out with its reason
//!   call_compare record-game-runs ROOT       each run left out only for its missing record put on the record
//!   call_compare game-run-events --root DIR --run RUN    that record's Producer

use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

use call_compare::{Stream, Windows};
use events::{
    ComparisonCompleted, ContractId, FiniteF64, HarnessU64, Id, Relation, SessionCompleted, SessionInit,
    SessionOutcome, SessionPurpose, VOCAB_VERSION, Version, harness_line,
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

/// `call_compare main-thread-calls ROOT`: the Windows main thread's calls in the run the batch run ends by, the rows
/// `store/batch_run/main-thread-calls-2026-10-29.tsv` holds (`call_compare::main_thread_calls`; Sprint 219), on
/// standard output: image, function, count.
fn main_thread_calls(root: &str) -> Result<(), String> {
    let mut out = io::stdout().lock();
    for (image, function, n) in call_compare::main_thread_calls(Path::new(root))? {
        writeln!(out, "{image}\t{function}\t{n}").map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// `call_compare reference-set ROOT`: the reference set the rules choose from ROOT's stored game runs
/// (`call_compare::reference_set_of`; Sprint 375), as `call_compare.py`'s `main` printed it: each run left out with its
/// reason, then the set.
fn reference_set(root: &str) -> Result<(), String> {
    let set = call_compare::reference_set_of(Path::new(root))?;
    let mut out = io::stdout().lock();
    for (run, reason) in &set.outside {
        writeln!(out, "run {run} not in the set: {reason}").map_err(|e| e.to_string())?;
    }
    writeln!(out, "reference set {}", set.members.join(", ")).map_err(|e| e.to_string())
}

/// `call_compare game-run-events --root DIR --run RUN`: a stored game run's session as a Producer, the events
/// `scripts/record_game_run.py`'s `one_step_run` wrote (Sprint 375): SESSION_INIT on the windows_reference profile for
/// an application, the run's EXECUTION_STARTED and EXECUTION_COMPLETED (`call_compare::game_run_execution`), and
/// SESSION_COMPLETED. Its harness version is this crate's, as the Rust host's records give theirs; the frozen host wrote
/// "0.0.1".
fn game_run_events(args: &[String]) -> Result<(), String> {
    let started = Instant::now();
    let flag = |name: &str| {
        args.iter()
            .position(|a| a == name)
            .and_then(|i| args.get(i + 1))
            .ok_or(format!("game-run-events: {name} PATH is required"))
    };
    let (root, run) = (flag("--root")?, flag("--run")?);
    let (execution, completed) = call_compare::game_run_execution(Path::new(root), run)?;
    let invalid = |e: events::InvalidValue| e.to_string();
    let vocabulary = Version::new(VOCAB_VERSION).map_err(invalid)?;
    let init = SessionInit {
        vocab_version: vocabulary.clone(),
        profile: Id::new("windows_reference").map_err(invalid)?,
        contract_version: vocabulary,
        harness_version: Version::new(env!("CARGO_PKG_VERSION")).map_err(invalid)?,
        purpose: SessionPurpose::Application,
    };
    let zero = HarnessU64::new(0).map_err(invalid)?;
    let close = SessionCompleted {
        outcome: SessionOutcome::Completed,
        discrepancy_count: zero,
        accepted_count: zero,
        elapsed_seconds: FiniteF64::new((started.elapsed().as_secs_f64() * 1000.0).round() / 1000.0)
            .map_err(invalid)?,
    };
    let mut out = io::stdout().lock();
    for l in [
        harness_line(&init),
        harness_line(&execution),
        harness_line(&completed),
        harness_line(&close),
    ] {
        writeln!(out, "{l}").map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn git(repo: &Path, args: &[&str]) -> Result<String, String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .output()
        .map_err(|e| format!("git: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "git -C {} {}: {}",
            repo.display(),
            args.join(" "),
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// `w2m.record.uncommitted`: the tracked files that differ from HEAD in the repository and in its substrate clone, and
/// the untracked files under src/ and tools/, but a sweep's run index (store/runs/<file>.json); a record in store/runs
/// names the commit that ran, so it is written from a committed tree.
fn uncommitted(root: &Path) -> Result<Vec<String>, String> {
    let mut changed = Vec::new();
    for (prefix, repo) in [("", root.to_path_buf()), ("substrate/", root.join("substrate"))] {
        for line in git(&repo, &["status", "--porcelain", "--untracked-files=no"])?.lines() {
            let path = line.get(3..).unwrap_or_default();
            let index = path.starts_with("store/runs/") && path.ends_with(".json") && path.matches('/').count() == 2;
            if !line.trim().is_empty() && !index {
                changed.push(format!("{prefix}{path}"));
            }
        }
        for path in git(
            &repo,
            &["ls-files", "--others", "--exclude-standard", "--", "src", "tools"],
        )?
        .lines()
        {
            changed.push(format!("{prefix}{path}"));
        }
    }
    Ok(changed)
}

/// The run file that puts one stored game run on the record: its baseline names the capture, the game and the commits
/// that ran (`w2m.record.provenance`), and its one Producer is `game-run-events`.
fn game_run_file(root: &Path, run: &str, exe: &Path, commits: (&str, &str)) -> String {
    let quoted = |s: &str| serde_json::Value::String(s.to_string()).to_string();
    format!(
        "# A stored game run on the record (Sprint 375; scripts/record_game_run.py at the tag frozen-python-2026-10-29), \
         written by\n# `call_compare record-game-runs` beside the repository's run files and removed after the run\n\n\
         [baseline]\ncapture = {}\ngame = \"hail-to-the-rainbow\"\nw2m_commit = {}\nsubstrate_commit = {}\n\n\
         [[producer]]\nkind = \"game_run\"\ncommand = [{}, \"game-run-events\", \"--root\", {}, \"--run\", {}]\n\
         schemas = \"schemas/harness-events.json\"\n\n[[initial]]\nkind = \"game_run\"\n\n[termination]\n\
         recipe = \"all_completed\"\n",
        quoted(&format!("{}/{run}", call_compare::GAME_RUNS)),
        quoted(commits.0),
        quoted(commits.1),
        quoted(&exe.to_string_lossy()),
        quoted(&root.to_string_lossy()),
        quoted(run),
    )
}

/// `call_compare record-game-runs ROOT`: each stored game run the rules leave out only for its missing record put on the
/// record, as `scripts/record_game_run.py` put one (Sprint 375): a record of its own under store/runs/game-run-<run>,
/// written by substrate from a run file beside the repository's (harness/.game-run-<pid>.toml, removed after), from a
/// committed tree. A run already on the record is not written again; the runs the rules leave out for another reason
/// are not the rules' to read.
fn record_game_runs(root: &str) -> Result<(), String> {
    let root = Path::new(root).canonicalize().map_err(|e| format!("{root}: {e}"))?;
    let set = call_compare::reference_set_of(&root)?;
    let unrecorded: Vec<&String> = set
        .outside
        .iter()
        .filter(|(run, reason)| **reason == format!("not on the record (game-run-{run})"))
        .map(|(run, _)| run)
        .collect();
    let mut out = io::stdout().lock();
    writeln!(
        out,
        "{} stored game runs to put on the record; the rules read the record of {}, each on it",
        unrecorded.len(),
        set.members.len()
            + set
                .outside
                .values()
                .filter(|r| r.starts_with("another game build") || r.starts_with("no windows_reference"))
                .count()
    )
    .map_err(|e| e.to_string())?;
    if unrecorded.is_empty() {
        return Ok(());
    }
    let changed = uncommitted(&root)?;
    if !changed.is_empty() {
        let first: Vec<&str> = changed.iter().take(10).map(String::as_str).collect();
        return Err(format!(
            "refused: store/runs/ records name the commit that ran; uncommitted: {}",
            first.join(", ")
        ));
    }
    let head = |repo: &Path| git(repo, &["rev-parse", "HEAD"]).map(|s| s.trim().to_string());
    let commits = (head(&root)?, head(&root.join("substrate"))?);
    let exe = std::env::current_exe().map_err(|e| format!("this binary's path: {e}"))?;
    for run in unrecorded {
        let record = root.join(call_compare::RECORDS).join(format!("game-run-{run}"));
        let file = root.join(format!("harness/.game-run-{}.toml", std::process::id()));
        std::fs::write(&file, game_run_file(&root, run, &exe, (&commits.0, &commits.1)))
            .map_err(|e| format!("{}: {e}", file.display()))?;
        let status = Command::new("uv")
            .current_dir(&root)
            .args(["run", "substrate", "run", "--file"])
            .arg(&file)
            .arg("--root")
            .arg(&record)
            .arg("--strict")
            .status();
        let _ = std::fs::remove_file(&file);
        let status = status.map_err(|e| format!("uv run substrate: {e}"))?;
        if !status.success() {
            return Err(format!("{run}: substrate run exited {status}"));
        }
        writeln!(out, "{}", record.display()).map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let done = match args.get(1).map(String::as_str) {
        Some("main-thread-calls") => main_thread_calls(args.get(2).map_or(".", String::as_str)),
        Some("reference-set") => reference_set(args.get(2).map_or(".", String::as_str)),
        Some("game-run-events") => game_run_events(&args[2..]),
        Some("record-game-runs") => record_game_runs(args.get(2).map_or(".", String::as_str)),
        _ => run(),
    };
    if let Err(why) = done {
        eprintln!("call_compare: {why}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The run file `record-game-runs` hands substrate: the baseline `record_game_run.py` wrote (capture, game) and the
    /// commits `one_step_run` added, one Producer, `game-run-events`, closing when it completes. Sprint 375 ran this text,
    /// with the repository's paths, through `uv run substrate run --strict` and `substrate replay --level 2`, exit 0.
    #[test]
    fn a_game_run_is_recorded_from_this_run_file() {
        let file = game_run_file(
            Path::new("/w2m"),
            "lap-1",
            Path::new("/w2m/target/release/call_compare"),
            ("c0", "c1"),
        );
        assert_eq!(
            file,
            "# A stored game run on the record (Sprint 375; scripts/record_game_run.py at the tag \
             frozen-python-2026-10-29), written by\n# `call_compare record-game-runs` beside the repository's run files \
             and removed after the run\n\n[baseline]\ncapture = \"store/game-runs/lap-1\"\ngame = \
             \"hail-to-the-rainbow\"\nw2m_commit = \"c0\"\nsubstrate_commit = \"c1\"\n\n[[producer]]\nkind = \
             \"game_run\"\ncommand = [\"/w2m/target/release/call_compare\", \"game-run-events\", \"--root\", \"/w2m\", \
             \"--run\", \"lap-1\"]\nschemas = \"schemas/harness-events.json\"\n\n[[initial]]\nkind = \"game_run\"\n\n\
             [termination]\nrecipe = \"all_completed\"\n"
        );
    }
}
