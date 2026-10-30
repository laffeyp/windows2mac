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
//! service answers (`service_for` is None: `hvc #9` at the call), the family's start options (`vm_inputs`, 2888-2905:
//! gui, foreground_held, instruction_clock, arguments, output_file, environment), and its row rule (the contract's
//! derivation or "raw", and its excluded rows' count); lists space-separated or "-". An import is not a call: a run
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

    /// The extract's rows: `family <key> <beside DLL or -> <start options or ->` and `run <id> <binary> <DLL or ->`.
    fn families(text: &str) -> Result<Families, String> {
        let dash = |s: &str| (s != "-").then(|| s.to_string());
        let (mut dll_names, mut runs, mut options) = (BTreeMap::new(), BTreeMap::new(), BTreeMap::new());
        for line in text.lines().filter(|l| !l.starts_with('#')) {
            match line.split('\t').collect::<Vec<_>>()[..] {
                ["family", key, dll, start] => {
                    if let Some(dll) = dash(dll) {
                        dll_names.insert(key.to_string(), dll);
                    }
                    let set: BTreeSet<String> = dash(start)
                        .map(|s| s.split(' ').map(str::to_string).collect())
                        .unwrap_or_default();
                    options.insert(key.to_string(), set);
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
        })
    }

    /// `family_run.py`'s families as the lookup reads them: each family's beside DLL and start options, every run by id.
    struct Families {
        dll_names: BTreeMap<String, String>,
        runs: BTreeMap<String, Run>,
        options: BTreeMap<String, BTreeSet<String>>,
    }

    /// The row rule a contract states: its derivation's name (`acceptance.derivation`, "scripts/family_run.py <name>:
    /// ...") or "raw", and the rows it excludes (`sections.excluded`, "<row>: <why>; ..." or "none").
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
        let excluded = match c["sections"]["excluded"].as_str() {
            None | Some("none") => 0,
            Some(e) => e.split("; ").count(),
        };
        Ok(format!("{derivation} excluded {excluded}"))
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
            let rule = rule(&root, &contract)?;
            let start = options.get(family).map_or_else(BTreeSet::new, |o| {
                o.iter().map(|s| s.to_string()).collect::<BTreeSet<_>>()
            });
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
                    "{family}\t{run_id}\t{}\t{contract}\t{beside_file}\t{}\t{}\t{rule}",
                    run.binary,
                    list(&unserved),
                    list(&start)
                );
            }
        }
        Ok(())
    }
}
