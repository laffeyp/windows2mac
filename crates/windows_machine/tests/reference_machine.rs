// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! Sprint 373: every stored Windows run is the laptop's, or is on the record by a Decision (cause 6 of
//! `docs/postmortem-windows-runs-off-the-laptop-2026-11-27.md`: "No gate, test or card field asks which machine a
//! probe's rows came from"). The store step refuses a run off the laptop (`windows_machine::store`, `store_game`); this
//! test holds the runs already in `store/probe-runs/` and `store/game-runs/` to the same rule, with its two exceptions.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;

/// The last GitHub runner run before the laptop arrived: 2026-10-12, 21:37:24Z, `code_rewrite` on windows-2022. A
/// runner's run with an id at most this stays on the record by the user's decision of 2026-10-11 ("the runners'
/// existing rows stay on the record").
const LAST_RUNNER_BEFORE_THE_LAPTOP: u64 = 38_311_779_649;
/// The runner's runs made after the laptop arrived, each with its reason: run, family, sprint, reason.
const LISTED: &str = "store/reference_machine/runner-runs-after-the-laptop-2026-11-27.tsv";
/// The stores walked.
const STORES: [&str; 2] = ["store/probe-runs", "store/game-runs"];
/// The runner's runs from before the laptop arrived, 409 probe runs and 10 game runs: a closed set, since no later
/// runner run has a smaller id and no stored run is removed.
const RUNNER_RUNS_BEFORE_THE_LAPTOP: usize = 419;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Which machine a stored run is held to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Machine {
    /// `profile.txt` starts `laptop-`.
    Laptop,
    /// A laptop batch's folder: `profile.json` naming `"machine": "laptop"` and no `profile.txt`.
    LaptopBatch,
    /// A runner's run whose id is at most `LAST_RUNNER_BEFORE_THE_LAPTOP`.
    RunnerBeforeTheLaptop,
    /// A runner's run after it, named in `LISTED` with its reason.
    ListedRunner,
}

/// `LISTED`'s rows: run to reason.
fn listed(root: &Path) -> BTreeMap<String, String> {
    let text = fs::read_to_string(root.join(LISTED)).unwrap_or_else(|e| panic!("{LISTED}: {e}"));
    let mut rows = BTreeMap::new();
    for line in text.lines().filter(|l| !l.starts_with('#') && !l.is_empty()) {
        let c: Vec<&str> = line.split('\t').collect();
        assert!(
            c.len() == 4 && !c[3].trim().is_empty(),
            "{LISTED}: a row is run, family, sprint, reason: {line:?}"
        );
        assert!(
            rows.insert(c[0].to_string(), c[3].to_string()).is_none(),
            "{LISTED}: run {} twice",
            c[0]
        );
    }
    rows
}

/// The machine `run` (a folder of `store`) is held to, or why it is held to none.
fn machine(store: &str, run: &Path, listed: &BTreeMap<String, String>) -> Result<Machine, String> {
    let name = run.file_name().expect("a run has a name").to_string_lossy().to_string();
    let at = format!("{store}/{name}");
    let profile = fs::read_to_string(run.join("profile.txt"))
        .ok()
        .map(|p| p.trim().to_string());
    let json_machine = match fs::read_to_string(run.join("profile.json")) {
        Ok(t) => Some(
            serde_json::from_str::<Value>(&t)
                .map_err(|e| format!("{at}: profile.json: {e}"))?
                .get("machine")
                .and_then(Value::as_str)
                .unwrap_or("(none)")
                .to_string(),
        ),
        Err(_) => None,
    };
    // a profile.json is the laptop's profile.ps1's: it names the laptop, and the profile.txt beside it agrees
    if let Some(m) = &json_machine
        && (m != "laptop" || profile.as_deref().is_some_and(|p| !p.starts_with("laptop-")))
    {
        return Err(format!(
            "{at}: profile.json names machine {m:?} and profile.txt is {profile:?}: they do not both name the laptop"
        ));
    }
    let Some(profile) = profile else {
        return match json_machine {
            Some(_) => Ok(Machine::LaptopBatch),
            None => Err(format!(
                "{at}: no profile.txt and no profile.json: its machine is not named"
            )),
        };
    };
    if profile.starts_with("laptop-") {
        return Ok(Machine::Laptop);
    }
    let id = name.parse::<u64>().ok().filter(|_| profile.starts_with("gha-"));
    match id {
        Some(id) if id <= LAST_RUNNER_BEFORE_THE_LAPTOP => Ok(Machine::RunnerBeforeTheLaptop),
        Some(_) if listed.contains_key(&name) => Ok(Machine::ListedRunner),
        Some(_) => Err(format!(
            "{at}: profile {profile}: a GitHub runner's run made after the laptop arrived (id above \
             {LAST_RUNNER_BEFORE_THE_LAPTOP}) and not named in {LISTED}"
        )),
        None => Err(format!(
            "{at}: profile {profile}: neither the laptop's (laptop-...) nor a GitHub runner's (gha-...) under its numeric \
             run id"
        )),
    }
}

/// Every run in `store/probe-runs/` and `store/game-runs/` is the laptop's, a laptop batch's folder, a runner's run
/// from before the laptop arrived, or a runner's run named in the dated list with its reason; each list row names
/// such a run. Prints the count of each kind.
#[test]
fn every_stored_windows_run_is_the_laptops_or_on_the_list() {
    let root = root();
    let listed = listed(&root);
    let mut kinds: BTreeMap<Machine, usize> = BTreeMap::new();
    let mut used: Vec<String> = Vec::new();
    let mut failed: Vec<String> = Vec::new();
    let mut not_runs: Vec<String> = Vec::new();
    for store in STORES {
        let mut runs: Vec<PathBuf> = fs::read_dir(root.join(store))
            .unwrap_or_else(|e| panic!("{store}: {e}"))
            .map(|e| e.unwrap().path())
            .collect();
        runs.sort();
        for run in runs {
            if !run.is_dir() {
                not_runs.push(format!("{store}/{}", run.file_name().unwrap().to_string_lossy()));
                continue;
            }
            match machine(store, &run, &listed) {
                Ok(kind) => {
                    *kinds.entry(kind).or_default() += 1;
                    if kind == Machine::ListedRunner {
                        used.push(run.file_name().unwrap().to_string_lossy().to_string());
                    }
                }
                Err(why) => failed.push(why),
            }
        }
    }
    let count = |k| kinds.get(&k).copied().unwrap_or(0);
    println!(
        "walked {} runs: laptop {}, laptop batch {}, runner before the laptop {}, listed runner {}, failing {}; not a \
         run: {}",
        kinds.values().sum::<usize>() + failed.len(),
        count(Machine::Laptop),
        count(Machine::LaptopBatch),
        count(Machine::RunnerBeforeTheLaptop),
        count(Machine::ListedRunner),
        failed.len(),
        if not_runs.is_empty() {
            "none".to_string()
        } else {
            not_runs.join(", ")
        }
    );
    assert!(
        failed.is_empty(),
        "{} stored runs not the laptop's and not on the record:\n{}",
        failed.len(),
        failed.join("\n")
    );
    let unused: Vec<&String> = listed.keys().filter(|r| !used.contains(r)).collect();
    assert!(
        unused.is_empty(),
        "{LISTED} names runs that are not stored runner runs after {LAST_RUNNER_BEFORE_THE_LAPTOP}: {unused:?}"
    );
    assert_eq!(
        count(Machine::RunnerBeforeTheLaptop),
        RUNNER_RUNS_BEFORE_THE_LAPTOP,
        "the runner's runs from before the laptop arrived are a closed set"
    );
    assert!(
        count(Machine::Laptop) > 0 && count(Machine::LaptopBatch) > 0,
        "the walk found no laptop run or no laptop batch: {kinds:?}"
    );
}
