// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! Sprint 374: the nine probe families whose Windows rows GitHub's runner made from 2026-11-01 to 2026-11-27
//! (`docs/postmortem-windows-runs-off-the-laptop-2026-11-27.md`, "What the wrong machine reached"), each runner run's
//! rows against the laptop run of the same family (Sprint 372's batch `lap-20261127T221622950Z-62b931c2` and the
//! rebuild `lap-20261127T225150596Z-62b931c2`), every body by bytes, with one of four outcomes named per family. Any
//! other difference fails, naming the family and the row: a finding about the reference, judged before any card relies
//! on it.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// How a family's runner rows stand to its laptop rows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Outcome {
    /// Every runner body equal by bytes to every laptop body, the laptop running the runner's binary.
    EqualOnTheSameBinary,
    /// Every body equal by bytes, the laptop running its own build of the probe, whose source has one commit.
    EqualOnALaptopRebuildOfOneSource,
    /// Every body equal by bytes but the one row named, which the user's rule of 2026-10-13 calls scheduling (a count
    /// of how the threads' loops met) and `run_loop.rs`'s judge leaves out.
    EqualButForASchedulingRow(&'static str),
    /// No laptop run of the family is stored: Smart App Control refused its binaries on the laptop (Sprint 372).
    NoLaptopRows,
}

/// The nine families: family, runner run, laptop run, outcome.
const FAMILIES: [(&str, &str, Option<&str>, Outcome); 9] = [
    (
        "locked_race",
        "41326239822",
        Some("lap-20261127T221622950Z-62b931c2-001"),
        Outcome::EqualOnTheSameBinary,
    ),
    (
        "thread_values_race",
        "45760720489",
        Some("lap-20261127T221622950Z-62b931c2-002"),
        Outcome::EqualButForASchedulingRow("overlap"),
    ),
    ("thread_slots_race", "45763850660", None, Outcome::NoLaptopRows),
    (
        "tls_free_reach",
        "45780352907",
        Some("lap-20261127T221622950Z-62b931c2-004"),
        Outcome::EqualOnTheSameBinary,
    ),
    (
        "tls_expansion_first",
        "45783238565",
        Some("lap-20261127T221622950Z-62b931c2-005"),
        Outcome::EqualOnTheSameBinary,
    ),
    (
        "wait_closed_handle",
        "45797800625",
        Some("lap-20261127T225150596Z-62b931c2"),
        Outcome::EqualOnALaptopRebuildOfOneSource,
    ),
    (
        "wait_closed_kinds",
        "45813040124",
        Some("lap-20261127T221622950Z-62b931c2-007"),
        Outcome::EqualOnTheSameBinary,
    ),
    (
        "mutex_abandoned",
        "45871497648",
        Some("lap-20261127T221622950Z-62b931c2-008"),
        Outcome::EqualOnTheSameBinary,
    ),
    (
        "named_sync_objects",
        "46053401854",
        Some("lap-20261127T221622950Z-62b931c2-009"),
        Outcome::EqualOnTheSameBinary,
    ),
];

fn read(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn json(path: &Path) -> Value {
    serde_json::from_str(&read(path)).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// A run's bodies, `body-1.jsonl` on, as (name, bytes).
fn bodies(run: &Path) -> Vec<(String, Vec<u8>)> {
    (1..)
        .map(|i| format!("body-{i}.jsonl"))
        .map_while(|name| fs::read(run.join(&name)).ok().map(|bytes| (name, bytes)))
        .collect()
}

/// The family a stored laptop run is of: its `job.json`'s probe for a run sent alone, or for a run split from a batch
/// (`batch.txt` naming the batch) the family of the batch's item whose execution id ends in the run's number.
fn laptop_family(store: &Path, run: &str) -> Option<String> {
    let at = store.join(run);
    if let Ok(text) = fs::read_to_string(at.join("job.json")) {
        let job: Value = serde_json::from_str(&text).unwrap_or_else(|e| panic!("{run}/job.json: {e}"));
        return job["probe"].as_str().map(str::to_string);
    }
    let batch = fs::read_to_string(at.join("batch.txt")).ok()?.trim().to_string();
    let number = run.strip_prefix(&format!("{batch}-"))?;
    let job = json(&store.join(&batch).join("job.json"));
    job["items"]
        .as_array()?
        .iter()
        .find(|item| {
            item["execution_id"]
                .as_str()
                .is_some_and(|e| e.ends_with(&format!("-{number}")))
        })
        .and_then(|item| item["family"].as_str())
        .map(str::to_string)
}

/// A row that carries an `id` beside its `s`, as format_message's "dump" rows do (one a message id): "<s> <id>", the
/// name it is compared and named by.
fn keyed(line: &str) -> Option<String> {
    let row = serde_json::from_str::<Value>(line).ok()?;
    Some(format!("{} {}", row["s"].as_str()?, row["id"].as_u64()?))
}

/// The rows by which two bodies differ, each named by its `s` (or by its line where it has none): the rows with no `id`
/// line for line, a difference in their count named as one; then, from Sprint 380, the rows with one (`keyed`) by
/// name, each the runner's that the laptop's lacks or holds otherwise, then each the laptop's alone, so a row one
/// build has and the other lacks names itself and not every row after it.
fn rows_unlike(runner: &[u8], laptop: &[u8]) -> Vec<String> {
    let (a, b) = (String::from_utf8_lossy(runner), String::from_utf8_lossy(laptop));
    // a body's rows with no id, each with its line's index, and its rows with one, each with its name
    type Parts<'a> = (Vec<(usize, &'a str)>, Vec<(String, &'a str)>);
    fn split(text: &str) -> Parts<'_> {
        let (mut plain, mut named) = (Vec::new(), Vec::new());
        for (n, line) in text.lines().enumerate() {
            match keyed(line) {
                Some(name) => named.push((name, line)),
                None => plain.push((n, line)),
            }
        }
        (plain, named)
    }
    let ((a, a_named), (b, b_named)) = (split(&a), split(&b));
    let mut unlike: Vec<String> = a
        .iter()
        .zip(&b)
        .filter(|((_, x), (_, y))| x != y)
        .map(|((n, x), _)| match serde_json::from_str::<Value>(x) {
            Ok(row) => row["s"].as_str().map_or(format!("line {}", n + 1), str::to_string),
            Err(_) => format!("line {}", n + 1),
        })
        .collect();
    if a.len() != b.len() {
        unlike.push(format!("{} lines against {}", a.len(), b.len()));
    }
    let find = |rows: &[(String, &str)], name: &str| rows.iter().find(|(n, _)| n == name).map(|(_, l)| l.to_string());
    for (name, line) in &a_named {
        if find(&b_named, name).as_deref() != Some(*line) {
            unlike.push(name.clone());
        }
    }
    for (name, _) in &b_named {
        if find(&a_named, name).is_none() {
            unlike.push(name.clone());
        }
    }
    unlike
}

/// The probe's source commits up to `commit`, newest first (`git log -- probes/<probe>`).
fn source_commits(root: &Path, commit: &str, probe: &str) -> Vec<String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["log", "--format=%H", commit, "--", &format!("probes/{probe}")])
        .output()
        .unwrap();
    assert!(out.status.success(), "git log {commit} -- probes/{probe}");
    String::from_utf8(out.stdout)
        .unwrap()
        .lines()
        .map(str::to_string)
        .collect()
}

/// One family judged: its outcome as found, or every way it is not the outcome named.
fn judge(
    root: &Path,
    family: &str,
    runner: &str,
    laptop: Option<&str>,
    outcome: Outcome,
) -> Result<String, Vec<String>> {
    let store = root.join("store/probe-runs");
    let mut wrong = Vec::new();
    let r = store.join(runner);
    let runner_profile = read(&r.join("profile.txt")).trim().to_string();
    if !runner_profile.starts_with("gha-") {
        wrong.push(format!("{family}: runner run {runner}'s profile is {runner_profile}"));
    }
    let runner_binary = read(&r.join("probe.sha256")).trim().to_string();
    let runner_bodies = bodies(&r);
    if runner_bodies.len() < 2 {
        wrong.push(format!(
            "{family}: runner run {runner} has {} bodies",
            runner_bodies.len()
        ));
    }
    let Some(laptop) = laptop else {
        // no laptop rows: no stored laptop run is of this family
        let theirs: Vec<String> = fs::read_dir(&store)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().to_string())
            .filter(|n| n.starts_with("lap-") && store.join(n).join("profile.txt").is_file())
            .filter(|n| laptop_family(&store, n).as_deref() == Some(family))
            .collect();
        if !theirs.is_empty() {
            wrong.push(format!(
                "{family}: named as having no laptop rows, and stored laptop runs of it exist: {theirs:?}"
            ));
        }
        return if wrong.is_empty() {
            Ok(format!(
                "{family}: no laptop rows; runner run {runner} (binary {runner_binary}) stays its reference"
            ))
        } else {
            Err(wrong)
        };
    };
    let l = store.join(laptop);
    let laptop_profile = read(&l.join("profile.txt"));
    if !windows_machine::laptop_profile(&laptop_profile) {
        wrong.push(format!(
            "{family}: laptop run {laptop}'s profile is {laptop_profile:?}, not {}",
            windows_machine::LAPTOP_PROFILE
        ));
    }
    if laptop_family(&store, laptop).as_deref() != Some(family) {
        wrong.push(format!(
            "{family}: laptop run {laptop} is of family {:?}",
            laptop_family(&store, laptop)
        ));
    }
    let laptop_binary = read(&l.join("probe.sha256")).trim().to_string();
    let laptop_bodies = bodies(&l);
    if laptop_bodies.len() < 2 {
        wrong.push(format!(
            "{family}: laptop run {laptop} has {} bodies",
            laptop_bodies.len()
        ));
    }
    // the binaries: the runner's, or the laptop's build of the probe's one source
    match outcome {
        Outcome::EqualOnALaptopRebuildOfOneSource => {
            if laptop_binary == runner_binary {
                wrong.push(format!(
                    "{family}: named a rebuild, and both runs ran binary {runner_binary}"
                ));
            }
            for sha in [&runner_binary, &laptop_binary] {
                if !root.join(format!("store/binaries/{sha}.exe")).is_file() {
                    wrong.push(format!("{family}: binary {sha} is not under store/binaries/"));
                }
            }
            let commit = json(&l.join("job.json"))["commit"].as_str().unwrap_or("").to_string();
            let commits = source_commits(root, &commit, family);
            if commits.len() != 1 {
                wrong.push(format!(
                    "{family}: probes/{family} has {} commits up to the laptop's build at {commit}, not one source: \
                     {commits:?}",
                    commits.len()
                ));
            }
        }
        _ => {
            if laptop_binary != runner_binary {
                wrong.push(format!(
                    "{family}: laptop run {laptop} ran binary {laptop_binary}, runner run {runner} {runner_binary}"
                ));
            }
        }
    }
    // the bodies, each runner body against each laptop body
    let allowed: Vec<&str> = match outcome {
        Outcome::EqualButForASchedulingRow(row) => vec![row],
        _ => vec![],
    };
    let mut scheduling_seen = false;
    for (rn, rb) in &runner_bodies {
        for (ln, lb) in &laptop_bodies {
            if rb == lb {
                continue;
            }
            let unlike = rows_unlike(rb, lb);
            let outside: Vec<&String> = unlike.iter().filter(|u| !allowed.contains(&u.as_str())).collect();
            if outside.is_empty() && !unlike.is_empty() {
                scheduling_seen = true;
            } else {
                wrong.push(format!(
                    "{family}: runner {runner}/{rn} against laptop {laptop}/{ln}: rows unlike {outside:?}{}",
                    if outside.is_empty() {
                        " (bytes unlike, every line equal)"
                    } else {
                        ""
                    }
                ));
            }
        }
    }
    if let Outcome::EqualButForASchedulingRow(row) = outcome {
        // the row the judge leaves out is this one, and it did differ
        let judge = read(&root.join("crates/w2m_vm/tests/run_loop.rs"));
        if !judge.contains(&format!("l.starts_with(\"{{\\\"s\\\":\\\"{row}\\\",\")")) {
            wrong.push(format!("{family}: run_loop.rs's judge leaves out no row {row}"));
        }
        if !scheduling_seen {
            wrong.push(format!("{family}: named equal but for {row}, and every body is equal"));
        }
    }
    if !wrong.is_empty() {
        return Err(wrong);
    }
    let how = match outcome {
        Outcome::EqualOnTheSameBinary => format!("equal on the same binary {runner_binary}"),
        Outcome::EqualOnALaptopRebuildOfOneSource => format!(
            "equal on a laptop rebuild of one source, {} against {}",
            &laptop_binary[..8],
            &runner_binary[..8]
        ),
        Outcome::EqualButForASchedulingRow(row) => {
            format!("equal but for the scheduling row {row}, binary {runner_binary}")
        }
        Outcome::NoLaptopRows => unreachable!(),
    };
    Ok(format!(
        "{family}: {how}; runner {runner} ({} bodies, {runner_profile}) against laptop {laptop} ({} bodies, \
         {laptop_profile})",
        runner_bodies.len(),
        laptop_bodies.len()
    ))
}

/// The nine runner runs are the list's (`store/reference_machine/runner-runs-after-the-laptop-2026-11-27.tsv`), and
/// each family's rows stand to its laptop run's as its outcome names; any other difference fails by family and row.
#[test]
fn each_runner_familys_rows_stand_to_its_laptop_rows_as_named() {
    let root = root();
    let listed: Vec<(String, String)> =
        read(&root.join("store/reference_machine/runner-runs-after-the-laptop-2026-11-27.tsv"))
            .lines()
            .filter(|l| !l.starts_with('#') && !l.is_empty())
            .map(|l| {
                let c: Vec<&str> = l.split('\t').collect();
                (c[0].to_string(), c[1].to_string())
            })
            .collect();
    let named: Vec<(String, String)> = FAMILIES
        .iter()
        .map(|(f, r, _, _)| (r.to_string(), f.to_string()))
        .collect();
    assert_eq!(
        named, listed,
        "the nine families against the list of runner runs after the laptop arrived"
    );
    let mut wrong = Vec::new();
    for (family, runner, laptop, outcome) in FAMILIES {
        match judge(&root, family, runner, laptop, outcome) {
            Ok(said) => println!("{said}"),
            Err(why) => wrong.extend(why),
        }
    }
    assert!(
        wrong.is_empty(),
        "the runner's rows against the laptop's:\n{}",
        wrong.join("\n")
    );
}

/// `named_sync_objects`' references on the laptop: its joined table row names its laptop run and contract 0.2.0, whose
/// rows are 0.1.0's and the laptop run's rows, raw, and whose evidence cites that run.
#[test]
fn named_sync_objects_is_held_to_its_laptop_run() {
    let root = root();
    let laptop = "lap-20261127T221622950Z-62b931c2-009";
    let joined = read(&root.join("store/family_runs/joined-2026-11-27.tsv"));
    let row: Vec<&str> = joined
        .lines()
        .find(|l| l.starts_with("named_sync_objects\t"))
        .expect("named_sync_objects' joined row")
        .split('\t')
        .collect();
    assert_eq!((row[1], row[3]), (laptop, "windows.sync.named_objects@0.2.0"));
    let new = json(&root.join("contracts/windows.sync.named_objects/0.2.0.json"));
    let old = json(&root.join("contracts/windows.sync.named_objects/0.1.0.json"));
    assert_eq!(new["acceptance"], old["acceptance"], "0.2.0's rows against 0.1.0's");
    let body = read(&root.join("store/probe-runs").join(laptop).join("body-1.jsonl"));
    let rows: Vec<Value> = body.lines().map(|l| serde_json::from_str(l).unwrap()).collect();
    assert_eq!(
        new["acceptance"]["rows"].as_array().unwrap(),
        &rows,
        "0.2.0's rows against laptop run {laptop}'s"
    );
    let evidence = new["evidence"].to_string();
    assert!(
        evidence.contains(laptop),
        "0.2.0's evidence cites no {laptop}: {evidence}"
    );
    assert!(
        !evidence.contains("46053401854"),
        "0.2.0's evidence cites the runner's run"
    );
}

/// The frozen families `store/family_runs/retaken-2026-11-27.tsv` re-takes on the laptop (Sprint 379), each with the rows
/// by which its laptop run's bodies differ from those of every runner run of the same binary, as Sprint 378's result
/// measured them: a workstation's product type, suite and build (version, and DllGetVersion's shell32 revision, which
/// its row rule keeps as set), the product type and the system directory's spelling (system_info), the Windows directory
/// in lower case (files_more); from Sprint 380 the system message table's four ids, runner 38120951889's 4566 and 8661
/// and the laptop's 15680 and 15681, every form row alike (format_message); from Sprint 381 GetTempPathW's fallback to
/// the Windows directory in lower case (temp_path) and GetModuleFileNameW's path of SHELL32, `C:\windows\System32\`
/// (system_modules).
const RETAKEN_DIFFERENCES: [(&str, &[&str]); 6] = [
    (
        "version",
        &[
            "verify.product_server",
            "verify.product_workstation",
            "get_version.ex",
            "get_version.plain",
            "dll_get_version.info2",
        ],
    ),
    ("files_more", &["windows.full", "windows.exact"]),
    ("system_info", &["version_ex", "system_directory"]),
    (
        "format_message",
        &["dump 4566", "dump 8661", "dump 15680", "dump 15681"],
    ),
    ("temp_path", &["temp.none"]),
    ("system_modules", &["file_name"]),
];

/// Each re-taken family's laptop run, as the re-taken table names it, is a stored laptop run of that family that ran a
/// runner binary of the family's lookup rows, and its every body differs from every body of each runner run of that
/// binary in exactly the rows `RETAKEN_DIFFERENCES` names (Sprint 379): any other difference is a finding about the
/// reference.
#[test]
fn each_retaken_familys_laptop_rows_differ_from_the_runners_as_named() {
    let root = root();
    let store = root.join("store/probe-runs");
    let retaken = read(&root.join("store/family_runs/retaken-2026-11-27.tsv"));
    let known = read(&root.join("store/family_runs/known-2026-10-29.tsv"));
    let rows: Vec<Vec<&str>> = retaken
        .lines()
        .filter(|l| !l.starts_with('#') && !l.is_empty())
        .map(|l| l.split('\t').collect())
        .collect();
    let families: Vec<&str> = rows.iter().map(|c| c[0]).collect();
    let named: Vec<&str> = RETAKEN_DIFFERENCES.iter().map(|(f, _)| *f).collect();
    assert_eq!(
        families, named,
        "the re-taken table's families against those named here"
    );
    let mut wrong = Vec::new();
    for (c, (_, differing)) in rows.iter().zip(RETAKEN_DIFFERENCES) {
        let (family, laptop) = (c[0], c[2]);
        let l = store.join(laptop);
        if !windows_machine::laptop_profile(&read(&l.join("profile.txt"))) {
            wrong.push(format!("{family}: laptop run {laptop}'s profile is not the laptop's"));
        }
        if laptop_family(&store, laptop).as_deref() != Some(family) {
            wrong.push(format!(
                "{family}: laptop run {laptop} is of family {:?}",
                laptop_family(&store, laptop)
            ));
        }
        let binary = read(&l.join("probe.sha256")).trim().to_string();
        let runners: Vec<&str> = known
            .lines()
            .filter(|k| k.starts_with(&format!("{family}\t")) && k.split('\t').nth(2) == Some(binary.as_str()))
            .map(|k| k.split('\t').nth(1).unwrap())
            .collect();
        if runners.is_empty() {
            wrong.push(format!(
                "{family}: laptop run {laptop} ran binary {binary}, which no runner run of the family ran"
            ));
        }
        let laptop_bodies = bodies(&l);
        let mut compared = 0;
        for runner in &runners {
            for (rn, rb) in bodies(&store.join(runner)) {
                for (ln, lb) in &laptop_bodies {
                    let unlike = rows_unlike(&rb, lb);
                    if unlike != differing.to_vec() {
                        wrong.push(format!(
                            "{family}: runner {runner}/{rn} against laptop {laptop}/{ln}: rows unlike {unlike:?}, named \
                             {differing:?}"
                        ));
                    }
                    compared += 1;
                }
            }
        }
        println!(
            "{family}: laptop {laptop} against {} runner runs of binary {binary}, {compared} pairs",
            runners.len()
        );
        if compared == 0 {
            wrong.push(format!("{family}: no body compared"));
        }
    }
    assert!(
        wrong.is_empty(),
        "the re-taken families' laptop rows:\n{}",
        wrong.join("\n")
    );
}

/// Contract 0.1.0 stays as Sprint 369 committed it, 53449a78a (the Decision of 2026-10-07: a change of evidence is a new
/// minor version, the old file kept unchanged).
#[test]
fn the_named_objects_contract_0_1_0_is_unchanged() {
    let root = root();
    let path = "contracts/windows.sync.named_objects/0.1.0.json";
    let out = Command::new("git")
        .arg("-C")
        .arg(&root)
        .args(["show", &format!("53449a78a:{path}")])
        .output()
        .unwrap();
    assert!(out.status.success(), "git show 53449a78a:{path}");
    assert_eq!(
        fs::read(root.join(path)).unwrap(),
        out.stdout,
        "{path} differs from 53449a78a's"
    );
}
