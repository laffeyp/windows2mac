// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! `family_known [ROOT]` (Sprint 221): what each runner run of the family index will ask of the Rust host, read from
//! the files before any run (the user's rule of 2026-10-29, "no searching for already knowable answers that you can
//! look up at once"; KIT_DIARY #87).
//!
//! The population is the gate's (plan step 4, "68 of 68 families"): every run `store/runs/family-runs.json` judged,
//! 273 in 68 families, each with the contract id and version its family record's COMPARISON_COMPLETED names, its
//! binary and the DLL beside it from `scripts/family_run.py`'s `Run`, the frozen host's data, stored in
//! `store/frozen_python/family-run-2026-10-29.tsv` when the Python left the tree (Sprint 226). For each
//! run it prints, tab-separated: the family, the run, the binary, the contract and version, the beside DLL as
//! "<name>:<sha256>" (its name alone where the run names no file, "-" where there is none), the imports no runtime
//! service answers (`service_for` is None: `hvc #9` at the call), the family's start options (`vm_inputs`, 2889-2902:
//! a JSON object of their values from Sprint 231, `START_KEYS`, "-" for none), and its row rule (the contract's
//! derivation or "raw", and its excluded rows' count), from Sprint 233 the excluded rows' names and the expected stop
//! (`Family.excluded`, `Family.expected_stop`); lists space-separated or "-". An import is not a call: a run
//! stops on one only if it calls it. `store/family_runs/known-2026-10-29.tsv`
//! holds the output; `tests/family_runs.rs` holds the file to it.
//!
//! The extract was written by this binary's text reader from `family_run.py` at the tag frozen-python-2026-10-29, and the
//! table it gives was held equal to the stored one before the Python went. A column of the imported services whose C body refuses some argument form was tried and
//! dropped: it named `WriteFile` and `GetStdHandle` on all 273 runs, the 14 that conform among them, so it told no run
//! from another; whether a refused form is reached depends on the arguments, which only a run shows.

#[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
fn main() {
    eprintln!("family_known runs on Apple silicon only, where the guest runtime's services are read");
    std::process::exit(2);
}

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
fn main() {
    if let Err(why) = known::run() {
        eprintln!("family_known: {why}");
        std::process::exit(1);
    }
}

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
mod known {
    use std::collections::{BTreeMap, BTreeSet};
    use std::fs;
    use std::path::{Path, PathBuf};

    use serde_json::Value;
    use w2m_vm::guest::service_for;

    /// A run of `family_run.py`: its binary, and the file beside it on the runner.
    struct Run {
        binary: String,
        dll: Option<String>,
    }

    /// The frozen host's families and runs, stored once from `scripts/family_run.py`'s text at the tag
    /// frozen-python-2026-10-29 (Sprint 226): the Python has left the tree, and its data is read here.
    const EXTRACT: &str = "store/frozen_python/family-run-2026-10-29.tsv";

    /// The extract's rows: `family <key> <beside DLL or -> <start options or -> [<excluded rows or -> <expected stop
    /// or ->]` and `run <id> <binary> <DLL or ->`; the last two family columns from Sprint 233, where a family has
    /// either.
    fn families(text: &str) -> Result<Families, String> {
        let dash = |s: &str| (s != "-").then(|| s.to_string());
        let (mut dll_names, mut runs, mut options) = (BTreeMap::new(), BTreeMap::new(), BTreeMap::new());
        let mut judged = BTreeMap::new();
        for line in text.lines().filter(|l| !l.starts_with('#')) {
            match line.split('\t').collect::<Vec<_>>()[..] {
                ["family", key, dll, start, ref rest @ ..] => {
                    if let Some(dll) = dash(dll) {
                        dll_names.insert(key.to_string(), dll);
                    }
                    if let Some(start) = dash(start) {
                        options.insert(key.to_string(), start_options(key, &start)?);
                    }
                    match rest {
                        [] => {}
                        [excluded, stop] => {
                            let names =
                                dash(excluded).map_or_else(Vec::new, |e| e.split(' ').map(str::to_string).collect());
                            judged.insert(key.to_string(), (names, dash(stop)));
                        }
                        _ => {
                            return Err(format!(
                                "{EXTRACT}: {key}'s row has {} columns, not 4 or 6",
                                4 + rest.len()
                            ));
                        }
                    }
                }
                ["run", id, binary, dll] => {
                    runs.insert(
                        id.to_string(),
                        Run {
                            binary: binary.to_string(),
                            dll: dash(dll),
                        },
                    );
                }
                _ => return Err(format!("{EXTRACT}: a row of neither form: {line:?}")),
            }
        }
        Ok(Families {
            dll_names,
            runs,
            options,
            judged,
        })
    }

    /// The start options `vm_inputs` gives a family's run (`family_run.py` 2889-2902 at the tag), each key the
    /// `Family` field it comes from (Sprint 231): `arguments` after the quoted module path, `environment` the block,
    /// `gui`, `foreground_held`, `instruction_clock`, and `output_file`, the file in the program's directory its rows
    /// are read from. `w2m-vm --start` takes the same object.
    const START_KEYS: [&str; 6] = [
        "arguments",
        "environment",
        "foreground_held",
        "gui",
        "instruction_clock",
        "output_file",
    ];

    /// A family's start options from the extract: a JSON object of `START_KEYS` only.
    fn start_options(family: &str, text: &str) -> Result<Value, String> {
        let options: Value = serde_json::from_str(text).map_err(|e| format!("{EXTRACT}: {family}'s start: {e}"))?;
        let keys = options
            .as_object()
            .ok_or(format!("{EXTRACT}: {family}'s start is not an object"))?;
        match keys.keys().find(|k| !START_KEYS.contains(&k.as_str())) {
            Some(k) => Err(format!(
                "{EXTRACT}: {family}'s start names {k}, which vm_inputs does not give"
            )),
            None => Ok(options),
        }
    }

    /// `family_run.py`'s families as the lookup reads them: each family's beside DLL and start options, every run by id.
    struct Families {
        dll_names: BTreeMap<String, String>,
        runs: BTreeMap<String, Run>,
        options: BTreeMap<String, Value>,
        /// each family's excluded rows (`Family.excluded`'s keys) and expected stop (`Family.expected_stop`)
        judged: BTreeMap<String, (Vec<String>, Option<String>)>,
    }

    /// The row rule a contract states: its derivation's name (`acceptance.derivation`, "scripts/family_run.py <name>:
    /// ...") or "raw". Its excluded rows are the extract's, not the contract's text, whose reasons hold "; " too
    /// (Sprint 233: `windows.file.attributes`' 2 rows were counted 4).
    fn rule(root: &Path, contract: &str) -> Result<String, String> {
        let (id, version) = contract.split_once('@').ok_or(format!("{contract}: no version"))?;
        let path = root.join(format!("contracts/{id}/{version}.json"));
        let c: Value =
            serde_json::from_str(&fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?)
                .map_err(|e| format!("{}: {e}", path.display()))?;
        let derivation = c["acceptance"]["derivation"]
            .as_str()
            .and_then(|d| d.strip_prefix("scripts/family_run.py "))
            .and_then(|d| d.split(':').next())
            .unwrap_or("raw");
        Ok(derivation.to_string())
    }

    /// The contract id and version a family record's first COMPARISON_COMPLETED names.
    fn contract(root: &Path, record: &str) -> Result<String, String> {
        let dir = root.join(record);
        let mut segments: Vec<PathBuf> = fs::read_dir(&dir)
            .map_err(|e| format!("{}: {e}", dir.display()))?
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| {
                p.file_name()
                    .is_some_and(|n| n.to_string_lossy().starts_with("events-"))
            })
            .collect();
        segments.sort();
        for s in segments {
            let text = fs::read_to_string(&s).map_err(|e| format!("{}: {e}", s.display()))?;
            for line in text.lines() {
                let Ok(v) = serde_json::from_str::<Value>(line) else {
                    continue;
                };
                if v["kind"] == "COMPARISON_COMPLETED" {
                    let p = &v["payload"];
                    return Ok(format!(
                        "{}@{}",
                        p["contract_id"].as_str().unwrap_or("?"),
                        p["contract_version"].as_str().unwrap_or("?")
                    ));
                }
            }
        }
        Err(format!("{record}: no COMPARISON_COMPLETED"))
    }

    /// An image's imports as "dll!name", delay imports included (`pe_image`'s `iat`).
    fn imports(path: &Path) -> Result<BTreeSet<String>, String> {
        let raw = fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let pe = pe_image::parse(&raw, false).map_err(|e| format!("{}: {e:?}", path.display()))?;
        Ok(pe.iat.values().cloned().collect())
    }

    pub fn run() -> Result<(), String> {
        let root = PathBuf::from(std::env::args().nth(1).unwrap_or_else(|| ".".into()));
        let read = |p: &str| fs::read_to_string(root.join(p)).map_err(|e| format!("{p}: {e}"));
        let index: Value = serde_json::from_str(&read("store/runs/family-runs.json")?)
            .map_err(|e| format!("family-runs.json: {e}"))?;
        let Families {
            dll_names,
            runs: all_runs,
            options,
            judged: excluded_and_stops,
        } = families(&read(EXTRACT)?)?;
        let texts = guest_runtime::texts(&root)?;
        let services = guest_runtime::services::services(&texts)?;
        let list = |s: &BTreeSet<String>| {
            if s.is_empty() {
                "-".to_string()
            } else {
                s.iter().cloned().collect::<Vec<_>>().join(" ")
            }
        };
        let index = index.as_object().ok_or("family-runs.json: not an object")?;
        for (family, entry) in index {
            let record = entry["record"].as_str().ok_or(format!("{family}: no record"))?;
            let contract = contract(&root, record)?;
            let (excluded, stop) = excluded_and_stops.get(family).cloned().unwrap_or_default();
            let rule = format!("{} excluded {}", rule(&root, &contract)?, excluded.len());
            let excluded = if excluded.is_empty() {
                "-".to_string()
            } else {
                excluded.join(" ")
            };
            let stop = stop.unwrap_or_else(|| "-".to_string());
            let start = options.get(family).map_or_else(|| "-".to_string(), Value::to_string);
            let dll_name = dll_names.get(family);
            let judged = entry["divergence"]
                .as_object()
                .ok_or(format!("{family}: no divergence"))?;
            for run_id in judged.keys() {
                let run = all_runs
                    .get(run_id)
                    .ok_or(format!("{family} run {run_id}: no run in {EXTRACT}"))?;
                let mut asked = imports(&root.join(format!("store/binaries/{}.exe", run.binary)))?;
                if let Some(dll) = &run.dll {
                    asked.extend(imports(&root.join(format!("store/binaries/{dll}.dll")))?);
                }
                let beside = dll_name.map(|n| n.to_lowercase());
                let unserved: BTreeSet<String> = asked
                    .iter()
                    .filter(|import| {
                        let (dll, function) = import.split_once('!').unwrap_or((import, ""));
                        beside.as_deref() != Some(dll.to_lowercase().as_str())
                            && service_for(&services, dll, function).is_none()
                    })
                    .cloned()
                    .collect();
                let beside_file = match (dll_name, &run.dll) {
                    (Some(name), Some(sha)) => format!("{name}:{sha}"),
                    (Some(name), None) => name.clone(),
                    _ => "-".to_string(),
                };
                println!(
                    "{family}\t{run_id}\t{}\t{contract}\t{beside_file}\t{}\t{start}\t{rule}\t{excluded}\t{stop}",
                    run.binary,
                    list(&unserved),
                );
            }
        }
        Ok(())
    }
}
