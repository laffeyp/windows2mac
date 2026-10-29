// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! The module families judged on the Rust host (Sprint 196; census row 6, the family judge; reviewer-2's Medium 5 of
//! its review of 190-192: nothing before UnityMain was judged). `scripts/gates.sh` runs each runner binary a family's
//! contract names (its evidence "runner-rows", "probe sha256:...") in `w2m-vm` as probe.exe, writing under
//! build/family_runs/ the rows it printed and its stop. This test holds a run that ends at ExitProcess to the contract:
//! its rows, derived as `scripts/family_run.py`'s `Family.rows` derives them, equal the contract's `acceptance.rows`
//! by key `s` (`unequal_rows`: each contract row must be printed exactly; a row the contract lacks is not compared);
//! and a run that stops to its stop in `STOPS`, so a family that newly ends or stops elsewhere fails until the file is
//! rewritten with the card that moved it.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;

const STOPS: &str = "store/family_runs/module-stops-2026-10-28.tsv";
/// The families: the gates' short name and the contract; the loader's late TLS from Sprint 218, its rows raw
/// (`family_run.py`'s late_tls family derives and excludes nothing)
const FAMILIES: [(&str, &str); 5] = [
    ("queries", "windows.module.queries"),
    ("system", "windows.module.system"),
    ("names_ansi", "windows.module.names_ansi"),
    ("search", "windows.module.search"),
    ("late_tls", "windows.loader.late_tls"),
];

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

/// The runner binaries a contract's evidence names, in order, each once.
fn binaries(contract: &Value) -> Vec<String> {
    let mut out = Vec::new();
    for e in contract["evidence"].as_array().into_iter().flatten() {
        if e["evidence_id"] != "runner-rows" {
            continue;
        }
        for piece in e["source_ref"].as_str().unwrap_or("").split("probe sha256:").skip(1) {
            let b: String = piece.chars().take_while(char::is_ascii_hexdigit).collect();
            if !out.contains(&b) {
                out.push(b);
            }
        }
    }
    out
}

/// `Family.rows` for these families: names_ansi drops `w_len` from file.image, file.exact, file.short and file.one, and
/// `ret` from the first three (family_run.py 1841-1844); the others are raw. None of the four excludes a row.
fn derive(family: &str, row: Value) -> Value {
    let Value::Object(mut r) = row else { return row };
    let s = r.get("s").and_then(Value::as_str).unwrap_or("").to_string();
    if family == "names_ansi" && ["file.image", "file.exact", "file.short", "file.one"].contains(&s.as_str()) {
        r.remove("w_len");
        if s != "file.one" {
            r.remove("ret");
        }
    }
    Value::Object(r)
}

#[test]
fn each_module_family_conforms_or_stops_where_held() {
    let root = root();
    let held: BTreeMap<(String, String), String> = fs::read_to_string(root.join(STOPS))
        .unwrap()
        .lines()
        .filter(|l| !l.starts_with('#'))
        .filter_map(|l| {
            let mut c = l.splitn(3, '\t');
            Some(((c.next()?.into(), c.next()?.into()), c.next()?.into()))
        })
        .collect();
    // a time limit is a hang, never an expected stop (Sprint 199; reviewer-2's Medium 3 of its review of 196-198)
    for stop in held.values() {
        assert!(
            !stop.contains("the run's time limit"),
            "{STOPS} holds a hang as a stop: {stop}"
        );
    }
    let mut seen = Vec::new();
    let mut conformed = 0;
    for (family, contract_id) in FAMILIES {
        let text = fs::read_to_string(root.join(format!("contracts/{contract_id}/0.1.0.json"))).unwrap();
        let contract: Value = serde_json::from_str(&text).unwrap();
        let rows = contract["acceptance"]["rows"].as_array().unwrap();
        let bins = binaries(&contract);
        assert!(!bins.is_empty(), "{contract_id} names no runner binary");
        for b in bins {
            let base = root.join(format!("build/family_runs/{family}-{b}"));
            let Ok(stop) = fs::read_to_string(base.with_extension("stop")) else {
                let what = format!("no build/family_runs/{family}-{b}.stop (scripts/gates.sh writes it)");
                if std::env::var("W2M_REQUIRE_CASE_RUNS").as_deref() == Ok("1") {
                    panic!("{what}, and W2M_REQUIRE_CASE_RUNS=1 requires it");
                }
                eprintln!("{what}: the module families are not judged here");
                return;
            };
            let stop = stop.lines().next().unwrap_or("").to_string();
            let key = (family.to_string(), b.clone());
            seen.push(key.clone());
            if stop != "ExitProcess { code: 0 }" {
                assert_eq!(
                    held.get(&key),
                    Some(&stop),
                    "{contract_id} binary {b} stopped at \"{stop}\", which {STOPS} does not hold"
                );
                continue;
            }
            assert!(
                !held.contains_key(&key),
                "{contract_id} binary {b} ends, and {STOPS} still holds a stop for it"
            );
            let printed = fs::read_to_string(base.with_extension("txt")).unwrap();
            let got: BTreeMap<String, Value> = printed
                .lines()
                .filter_map(|l| serde_json::from_str::<Value>(l).ok())
                .map(|r| derive(family, r))
                .filter_map(|r| Some((r.get("s")?.as_str()?.to_string(), r)))
                .collect();
            for want in rows {
                let s = want["s"].as_str().unwrap();
                assert_eq!(
                    got.get(s),
                    Some(want),
                    "{contract_id} binary {b}: row {s} differs from the contract's"
                );
            }
            conformed += 1;
        }
    }
    for key in held.keys() {
        assert!(seen.contains(key), "{STOPS} holds {key:?}, which no contract names");
    }
    println!(
        "{conformed} of {} module-family runs conform; the rest stop where held",
        seen.len()
    );
}
