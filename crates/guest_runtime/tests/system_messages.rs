// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! Sprint 380's judge of the system message table FormatMessageW answers from: the newest version
//! (`guest_runtime::headers::SYSTEM_MESSAGES`) is the "dump" rows of the stored laptop run its `source` names, every
//! body of that run; those rows differ from the runner's, which 0.1.0 holds, in the ids named here and no others; 0.1.0
//! stays as Sprint 049 committed it; and the header the build writes is generated from the newest table. The family's
//! row rule drops the dump rows (`crates/w2m_vm/tests/family_runs.rs`, `format_message_rows`), so no family run judges
//! the table's texts; this file does.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use guest_runtime::headers::{SYSTEM_MESSAGES, system_messages, system_messages_table};
use serde_json::Value;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

/// The ids in which the laptop's rows (0.2.0) differ from the runner's (0.1.0, run 38120951889, Server 2025 build
/// 26100), as `diff` of the two bodies showed them: the runner's only, the laptop's only, a text unlike.
const RUNNER_ONLY: [u64; 2] = [4566, 8661];
const LAPTOP_ONLY: [u64; 2] = [15680, 15681];

/// The commit that added `specs/system_messages.json` at 0.1.0 (Sprint 049).
const FIRST_COMMIT: &str = "3b132e94d";

fn json(path: &Path) -> Value {
    serde_json::from_str(&fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display())))
        .unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// A table's rows, (id, text), in its order.
fn table_rows(spec: &Value, table: &str) -> Vec<(u64, String)> {
    spec["messages"]
        .as_array()
        .unwrap_or_else(|| panic!("{table}: no messages"))
        .iter()
        .map(|m| (m[0].as_u64().unwrap(), m[1].as_str().unwrap().to_string()))
        .collect()
}

/// The stored run a table's `source` names, "store/probe-runs/<run>/body-1.jsonl (...)".
fn source_run(spec: &Value, table: &str) -> String {
    let source = spec["source"].as_str().unwrap_or_else(|| panic!("{table}: no source"));
    source
        .strip_prefix("store/probe-runs/")
        .and_then(|s| s.split('/').next())
        .unwrap_or_else(|| panic!("{table}: source {source:?} names no stored run"))
        .to_string()
}

/// A run's bodies, each as its "dump" rows, (id, text), in its order.
fn dump_bodies(root: &Path, run: &str) -> Vec<(String, Vec<(u64, String)>)> {
    (1..)
        .map(|i| format!("body-{i}.jsonl"))
        .map_while(|name| {
            fs::read_to_string(root.join("store/probe-runs").join(run).join(&name))
                .ok()
                .map(|text| (name, text))
        })
        .map(|(name, text)| {
            let rows = text
                .lines()
                .map(|l| serde_json::from_str::<Value>(l).unwrap_or_else(|e| panic!("{run}/{name}: {e}: {l}")))
                .filter(|r| r["s"] == "dump")
                .map(|r| (r["id"].as_u64().unwrap(), r["text"].as_str().unwrap().to_string()))
                .collect();
            (name, rows)
        })
        .collect()
}

/// The ids of `a` that `b` lacks, those of `b` that `a` lacks, and those both hold with unlike texts.
fn unlike(a: &[(u64, String)], b: &[(u64, String)]) -> (Vec<u64>, Vec<u64>, Vec<u64>) {
    let find = |rows: &[(u64, String)], id: u64| rows.iter().find(|(i, _)| *i == id).map(|(_, t)| t.clone());
    let only = |x: &[(u64, String)], y: &[(u64, String)]| {
        x.iter().map(|(i, _)| *i).filter(|i| find(y, *i).is_none()).collect()
    };
    let texts = a
        .iter()
        .filter(|(i, t)| find(b, *i).is_some_and(|u| &u != t))
        .map(|(i, _)| *i)
        .collect();
    (only(a, b), only(b, a), texts)
}

/// The laptop's profile at build 26200, as `windows_machine::LAPTOP_PROFILE` names it.
fn laptop_profile(profile: &str) -> bool {
    profile
        .trim()
        .strip_prefix("laptop-x64-10.0.26200.")
        .and_then(|r| r.strip_suffix("-Client"))
        .is_some_and(|r| !r.is_empty() && r.bytes().all(|b| b.is_ascii_digit()))
}

/// The table a build reads is 0.2.0, and it is the "dump" rows of every body of the stored laptop run its `source`
/// names, id for id and text for text; a difference is named by its id.
#[test]
fn the_newest_table_is_the_laptop_runs_rows_it_names() {
    let root = root();
    let table = system_messages_table(&root).unwrap();
    assert_eq!(table, SYSTEM_MESSAGES[0], "the table a build of the working tree reads");
    let spec = json(&root.join(table));
    assert_eq!(spec["version"], "0.2.0", "{table}'s version");
    let run = source_run(&spec, table);
    let profile = fs::read_to_string(root.join("store/probe-runs").join(&run).join("profile.txt")).unwrap_or_default();
    assert!(
        laptop_profile(&profile),
        "{table}'s source names run {run}, whose profile {:?} is not the laptop's",
        profile.trim()
    );
    let rows = table_rows(&spec, table);
    let bodies = dump_bodies(&root, &run);
    assert!(!bodies.is_empty(), "run {run} has no body");
    let mut wrong = Vec::new();
    for (name, dumped) in &bodies {
        let (table_only, run_only, texts) = unlike(&rows, dumped);
        for id in table_only {
            wrong.push(format!("id {id}: in {table}, not in {run}/{name}"));
        }
        for id in run_only {
            wrong.push(format!("id {id}: in {run}/{name}, not in {table}"));
        }
        for id in texts {
            wrong.push(format!("id {id}: {table}'s text is not {run}/{name}'s"));
        }
        if wrong.is_empty() && &rows != dumped {
            wrong.push(format!("{table}: its ids in another order than {run}/{name}'s"));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
    println!(
        "{table}: {} ids, the dump rows of {run}'s {} bodies",
        rows.len(),
        bodies.len()
    );
}

/// The laptop's rows (0.2.0) differ from the runner's (0.1.0, itself the dump rows of the runner run its source names)
/// in `RUNNER_ONLY` and `LAPTOP_ONLY` alone, no text of an id both hold unlike: any other difference is a finding about
/// the reference.
#[test]
fn the_laptops_rows_differ_from_the_runners_as_named() {
    let root = root();
    let (new, old) = (SYSTEM_MESSAGES[0], SYSTEM_MESSAGES[1]);
    let (new_spec, old_spec) = (json(&root.join(new)), json(&root.join(old)));
    let (laptop, runner) = (table_rows(&new_spec, new), table_rows(&old_spec, old));
    let run = source_run(&old_spec, old);
    for (name, dumped) in dump_bodies(&root, &run) {
        assert_eq!(dumped, runner, "{old} against {run}/{name}'s dump rows");
    }
    let (runner_only, laptop_only, texts) = unlike(&runner, &laptop);
    assert_eq!(
        (runner_only.as_slice(), laptop_only.as_slice(), texts.as_slice()),
        (RUNNER_ONLY.as_slice(), LAPTOP_ONLY.as_slice(), [].as_slice()),
        "the ids {old} ({run}) holds alone, {new} holds alone, and whose texts are unlike"
    );
    println!(
        "{old}: {} ids, {new}: {} ids; the runner's alone {runner_only:?}, the laptop's alone {laptop_only:?}",
        runner.len(),
        laptop.len()
    );
}

/// 0.1.0 is kept as Sprint 049 committed it (the Decision of 2026-10-07: a change of evidence is a new minor version,
/// the old file kept unchanged).
#[test]
fn the_table_0_1_0_is_unchanged() {
    let root = root();
    let path = SYSTEM_MESSAGES[1];
    let out = Command::new("git")
        .arg("-C")
        .arg(&root)
        .args(["show", &format!("{FIRST_COMMIT}:{path}")])
        .output()
        .unwrap();
    assert!(out.status.success(), "git show {FIRST_COMMIT}:{path}");
    assert_eq!(
        fs::read(root.join(path)).unwrap(),
        out.stdout,
        "{path} differs from {FIRST_COMMIT}'s"
    );
}

/// The header the build writes from the working tree is generated from 0.2.0 and holds its every id.
#[test]
fn the_header_is_generated_from_the_newest_table() {
    let root = root();
    let header = system_messages(&root).unwrap();
    let mut lines = header.lines();
    assert_eq!(
        lines.next(),
        Some(
            format!(
                "/* generated by src/w2m/system_messages.py from {} */",
                SYSTEM_MESSAGES[0]
            )
            .as_str()
        )
    );
    let count = table_rows(&json(&root.join(SYSTEM_MESSAGES[0])), SYSTEM_MESSAGES[0]).len();
    assert_eq!(
        lines.next(),
        Some(format!("#define W2M_MESSAGE_COUNT {count}").as_str())
    );
}
