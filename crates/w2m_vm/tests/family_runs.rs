// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! The families judged on the Rust host as the gate counts them (census row 6; plan step 4, "68 of 68 families";
//! Sprint 221). The population is every run `store/runs/family-runs.json` judged, 273 in 68 families, read from the
//! lookup `store/family_runs/known-2026-10-29.tsv` (`family_known`): each run's binary, the contract and version its
//! family record judged it by, its beside DLL, and its row rule. `scripts/gates.sh` runs each row in `w2m-vm` as
//! probe.exe with its beside DLL and an instruction budget, writing build/family_runs/<family>-<run>.txt and .stop.
//!
//! A run that ends at ExitProcess under a row rule this test carries (raw, or `module_names_ansi_rows`) is held to
//! its contract version's rows: each row printed exactly, by key `s` (a printed row the contract lacks is not
//! compared). Every other run is held in `HELD` by what it waits on: "ended; row rule <rule> not ported; rows sha256
//! <hex>", the hash of its printed rows, so an ended run's output cannot change unseen while its rule waits (reviewer-3's
//! Medium 1 of its project review, 2026-10-29), or the stop it makes ("stop: <stop>", a temporary drive's path and file
//! name written as fixed marks). A run that conforms must not be held, and a held run that conforms or moves fails the
//! test until the card that moved it rewrites the file.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;

const KNOWN: &str = "store/family_runs/known-2026-10-29.tsv";
const HELD: &str = "store/family_runs/sweep-held-2026-10-29.tsv";
/// The row rules this test carries: the rows as printed, and names_ansi's (family_run.py 1841-1844)
const CARRIED: [&str; 2] = ["raw excluded 0", "module_names_ansi_rows excluded 0"];

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

/// `module_names_ansi_rows`: drops `w_len` from file.image, file.exact, file.short and file.one, and `ret` from the
/// first three (family_run.py 1841-1844); the raw rule keeps each row as printed.
fn derive(rule: &str, row: Value) -> Value {
    let Value::Object(mut r) = row else { return row };
    let s = r.get("s").and_then(Value::as_str).unwrap_or("").to_string();
    if rule.starts_with("module_names_ansi_rows")
        && ["file.image", "file.exact", "file.short", "file.one"].contains(&s.as_str())
    {
        r.remove("w_len");
        if s != "file.one" {
            r.remove("ret");
        }
    }
    Value::Object(r)
}

/// A stop with the temporary drive's path and the name `GetTempFileNameW` draws written as fixed marks, which move
/// from run to run.
fn steady(stop: &str) -> String {
    let mut out = String::new();
    let mut rest = stop;
    while let Some(i) = rest.find("/w2m-drive-c-") {
        let start = rest[..i].rfind(' ').map_or(0, |j| j + 1);
        out.push_str(&rest[..start]);
        out.push_str("<drive>");
        let after = &rest[i + "/w2m-drive-c-".len()..];
        rest = after.find('/').map_or("", |j| &after[j..]);
    }
    out.push_str(rest);
    // w2m<4 hex digits>.tmp
    let mut steady = String::new();
    let mut s = out.as_str();
    while let Some(i) = s.find("w2m") {
        steady.push_str(&s[..i]);
        let tail = &s[i + 3..];
        if tail.len() >= 8 && tail[..4].bytes().all(|b| b.is_ascii_hexdigit()) && &tail[4..8] == ".tmp" {
            steady.push_str("w2m<tmp>.tmp");
            s = &tail[8..];
        } else {
            steady.push_str("w2m");
            s = tail;
        }
    }
    steady.push_str(s);
    steady
}

/// The table of what each run of the family index will ask is the lookup's output, not a file that drifts from it
/// (Sprint 221): `family_known` run over the tree gives the stored rows, comments left out.
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
#[test]
fn the_stored_lookup_is_family_knowns_output() {
    let root = root();
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_family_known"))
        .arg(&root)
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let stored = fs::read_to_string(root.join(KNOWN)).unwrap();
    let stored: Vec<&str> = stored.lines().filter(|l| !l.starts_with('#')).collect();
    let derived = String::from_utf8(out.stdout).unwrap();
    assert_eq!(stored, derived.lines().collect::<Vec<_>>());
    assert_eq!(stored.len(), 273, "the runs the family index judged");
}

#[test]
fn a_moving_stop_is_written_steadily() {
    assert_eq!(
        steady(
            "HostFailure op 6: a name in a folder whose listing is not recorded: /private/var/folders/zy/x/T/w2m-drive-c-84305-0/Windows/w2mFF92.tmp"
        ),
        "HostFailure op 6: a name in a folder whose listing is not recorded: <drive>/Windows/w2m<tmp>.tmp"
    );
    assert_eq!(steady("import a.dll!b"), "import a.dll!b");
}

#[test]
fn every_family_run_conforms_or_is_held_by_what_it_waits_on() {
    let root = root();
    let held: BTreeMap<(String, String), String> = fs::read_to_string(root.join(HELD))
        .unwrap()
        .lines()
        .filter(|l| !l.starts_with('#'))
        .filter_map(|l| {
            let mut c = l.splitn(3, '\t');
            Some(((c.next()?.into(), c.next()?.into()), c.next()?.into()))
        })
        .collect();
    let known = fs::read_to_string(root.join(KNOWN)).unwrap();
    let mut seen = BTreeMap::new();
    let (mut conformed, mut families) = (0, BTreeMap::<String, bool>::new());
    for line in known.lines().filter(|l| !l.starts_with('#')) {
        let c: Vec<&str> = line.split('\t').collect();
        let (family, run, contract, rule) = (c[0], c[1], c[3], c[7]);
        let base = root.join(format!("build/family_runs/{family}-{run}"));
        let Ok(stop) = fs::read_to_string(base.with_extension("stop")) else {
            let what = format!("no build/family_runs/{family}-{run}.stop (scripts/gates.sh writes it)");
            if std::env::var("W2M_REQUIRE_CASE_RUNS").as_deref() == Ok("1") {
                panic!("{what}, and W2M_REQUIRE_CASE_RUNS=1 requires it");
            }
            eprintln!("{what}: the families are not judged here");
            return;
        };
        let stop = stop.lines().next().unwrap_or("").to_string();
        let key = (family.to_string(), run.to_string());
        let state = if stop != "ExitProcess { code: 0 }" {
            format!("stop: {}", steady(&stop))
        } else if !CARRIED.contains(&rule) {
            let printed = fs::read(base.with_extension("txt")).unwrap_or_default();
            format!(
                "ended; row rule {rule} not ported; rows sha256 {}",
                cpu_cases::sha256(&printed)
            )
        } else {
            let (id, version) = contract.split_once('@').unwrap();
            let text = fs::read_to_string(root.join(format!("contracts/{id}/{version}.json"))).unwrap();
            let contract: Value = serde_json::from_str(&text).unwrap();
            let printed = fs::read_to_string(base.with_extension("txt")).unwrap();
            let got: BTreeMap<String, Value> = printed
                .lines()
                .filter_map(|l| serde_json::from_str::<Value>(l).ok())
                .map(|r| derive(rule, r))
                .filter_map(|r| Some((r.get("s")?.as_str()?.to_string(), r)))
                .collect();
            for want in contract["acceptance"]["rows"].as_array().unwrap() {
                let s = want["s"].as_str().unwrap();
                assert_eq!(
                    got.get(s),
                    Some(want),
                    "{family} run {run} ({id} {version}): row {s} differs from the contract's"
                );
            }
            assert!(
                !held.contains_key(&key),
                "{family} run {run} conforms, and {HELD} still holds it"
            );
            conformed += 1;
            families.entry(family.to_string()).or_insert(true);
            seen.insert(key, String::new());
            continue;
        };
        *families.entry(family.to_string()).or_insert(false) = false;
        assert_eq!(
            held.get(&key),
            Some(&state),
            "{family} run {run} is \"{state}\", which {HELD} does not hold"
        );
        seen.insert(key, state);
    }
    for key in held.keys() {
        assert!(
            seen.contains_key(key),
            "{HELD} holds {key:?}, which the lookup does not name"
        );
    }
    println!(
        "{conformed} of {} family runs conform, {} of {} families in every run; the rest held by what they wait on",
        seen.len(),
        families.values().filter(|v| **v).count(),
        families.len()
    );
}
