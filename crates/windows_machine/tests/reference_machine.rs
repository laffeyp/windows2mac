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

/// The family tables, each run in its second column (Sprint 374).
const FAMILY_TABLES: [&str; 2] = [
    "store/family_runs/known-2026-10-29.tsv",
    "store/family_runs/joined-2026-11-27.tsv",
];

/// The runs a contract's text cites: every GitHub run id, eleven digits standing alone, and every laptop run id,
/// `lap-<8 digits>T<9 digits>Z-<8 hex>` with its batch number if any, a range written "<id>-01 to -10" giving each
/// number between.
fn cited_in(text: &str) -> Vec<String> {
    let b = text.as_bytes();
    let alnum = |i: usize| b.get(i).is_some_and(|c| c.is_ascii_alphanumeric());
    let digits_at = |i: usize| b[i..].iter().take_while(|c| c.is_ascii_digit()).count();
    let mut runs = Vec::new();
    let mut i = 0;
    while i < b.len() {
        if b[i..].starts_with(b"lap-") && (i == 0 || !alnum(i - 1)) {
            // lap-YYYYMMDD T HHMMSSmmm Z - 8 hex
            let s = i + 4;
            let ok = digits_at(s) == 8
                && b.get(s + 8) == Some(&b'T')
                && digits_at(s + 9) == 9
                && b.get(s + 18) == Some(&b'Z')
                && b.get(s + 19) == Some(&b'-')
                && b.len() >= s + 28
                && b[s + 20..s + 28].iter().all(|c| c.is_ascii_hexdigit());
            if ok {
                let mut end = s + 28;
                let base = text[i..end].to_string();
                let mut number = None;
                if b.get(end) == Some(&b'-') && digits_at(end + 1) > 0 {
                    let n = digits_at(end + 1);
                    number = Some(text[end + 1..end + 1 + n].to_string());
                    end += 1 + n;
                }
                match &number {
                    None => runs.push(base),
                    Some(first) => {
                        // "<id>-NN to -MM": each number from NN to MM, at NN's width
                        let tail = &text[end..];
                        let last = tail
                            .strip_prefix(" to -")
                            .map(|t| t.chars().take_while(char::is_ascii_digit).collect::<String>())
                            .filter(|t| !t.is_empty());
                        let (from, to): (u64, u64) = (
                            first.parse().unwrap(),
                            last.as_deref().map_or(first.parse().unwrap(), |l| l.parse().unwrap()),
                        );
                        for n in from..=to {
                            runs.push(format!("{base}-{n:0w$}", w = first.len()));
                        }
                    }
                }
                i = end;
                continue;
            }
        }
        if b[i].is_ascii_digit() && (i == 0 || !alnum(i - 1)) {
            let n = digits_at(i);
            if n == 11 && !alnum(i + n) {
                runs.push(text[i..i + n].to_string());
            }
            i += n;
            continue;
        }
        i += 1;
    }
    runs
}

/// Every run a family table or a contract under `contracts/` cites is held to the walk's rule (Sprint 374; the Decision
/// "A contract's evidence runs held to the reference machine too"): a stored run as `machine` holds it, a run not
/// stored only when it is a GitHub run at or below `LAST_RUNNER_BEFORE_THE_LAPTOP`. Each failure names the file and
/// the run; the count of each kind is printed.
#[test]
fn every_cited_run_is_held_to_the_reference_rule() {
    let root = root();
    let listed = listed(&root);
    // (citing file, run)
    let mut cited: Vec<(String, String)> = Vec::new();
    for table in FAMILY_TABLES {
        let text = fs::read_to_string(root.join(table)).unwrap_or_else(|e| panic!("{table}: {e}"));
        for line in text.lines().filter(|l| !l.starts_with('#') && !l.is_empty()) {
            let run = line
                .split('\t')
                .nth(1)
                .unwrap_or_else(|| panic!("{table}: a row with no run: {line:?}"));
            cited.push((table.to_string(), run.to_string()));
        }
    }
    let mut contracts: Vec<PathBuf> = fs::read_dir(root.join("contracts"))
        .unwrap()
        .flat_map(|d| fs::read_dir(d.unwrap().path()).unwrap())
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|x| x == "json"))
        .collect();
    contracts.sort();
    for path in &contracts {
        let name = path.strip_prefix(&root).unwrap().to_string_lossy().to_string();
        let text = fs::read_to_string(path).unwrap_or_else(|e| panic!("{name}: {e}"));
        let mut runs = cited_in(&text);
        runs.sort();
        runs.dedup();
        cited.extend(runs.into_iter().map(|r| (name.clone(), r)));
    }
    let mut kinds: BTreeMap<String, usize> = BTreeMap::new();
    let mut failed = Vec::new();
    for (by, run) in &cited {
        let stored = STORES.iter().find(|s| root.join(s).join(run).is_dir());
        let kind = match stored {
            Some(store) => machine(store, &root.join(store).join(run), &listed).map(|m| format!("{m:?}")),
            None => match run.parse::<u64>() {
                Ok(id) if id <= LAST_RUNNER_BEFORE_THE_LAPTOP => Ok("RunnerBeforeTheLaptopNotStored".to_string()),
                Ok(_) => Err(format!(
                    "{by} cites run {run}: a GitHub run after {LAST_RUNNER_BEFORE_THE_LAPTOP}, not stored"
                )),
                Err(_) => Err(format!("{by} cites run {run}: not stored under {STORES:?}")),
            },
        };
        match kind {
            Ok(k) => *kinds.entry(k).or_default() += 1,
            Err(why) if why.starts_with(by.as_str()) => failed.push(why),
            Err(why) => failed.push(format!("{by} cites run {run}: {why}")),
        }
    }
    let by_tables = cited
        .iter()
        .filter(|(by, _)| FAMILY_TABLES.contains(&by.as_str()))
        .count();
    println!(
        "{} citations, {by_tables} by the family tables and {} by {} contract files: {kinds:?}; failing {}",
        cited.len(),
        cited.len() - by_tables,
        contracts.len(),
        failed.len()
    );
    assert!(
        failed.is_empty(),
        "cited runs off the reference rule:\n{}",
        failed.join("\n")
    );
    assert!(
        by_tables > 0 && cited.len() > by_tables,
        "no citation found by a table or a contract"
    );
}

/// The citation reader on the forms the contracts write.
#[test]
fn the_citation_reader_reads_the_contracts_forms() {
    assert_eq!(
        cited_in("run 46053401854: gha, x 4294967295 and 1791622152926 and frontier 17919422957926"),
        ["46053401854"]
    );
    assert_eq!(
        cited_in("(laptop runs lap-20261014T032442034Z-c8814547-01 to -03: 0)"),
        [
            "lap-20261014T032442034Z-c8814547-01",
            "lap-20261014T032442034Z-c8814547-02",
            "lap-20261014T032442034Z-c8814547-03"
        ]
    );
    assert_eq!(
        cited_in("\"run lap-20261014T094031671Z-93f645c3: laptop\", (lap-20261013T231059422Z-136bec80-168)"),
        [
            "lap-20261014T094031671Z-93f645c3",
            "lap-20261013T231059422Z-136bec80-168"
        ]
    );
}
