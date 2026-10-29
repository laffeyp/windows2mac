// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! The batch run's record (Sprint 210; the Decision "Sequence after Sprint 208"): `scripts/gates.sh` runs `w2m-vm
//! --game` under `harness/w2m-vm-game.toml`, which writes the run's execution record to build/batch_run/record-<ns>. The
//! frozen host writes its measurements and its completion at a run's end, past where the Rust run stops, so the record
//! is held two ways. In form, to the frozen host's recorded run of 2026-10-21 (`REFERENCE`): EXECUTION_STARTED's fields
//! but the run's id, each measurement's metric among the reference's and in its unit, with its conventions (phase,
//! backend, profiles, artifact state), and the record's kinds the reference's but TRACE_INGESTED and
//! COMPARISON_COMPLETED (the Rust host writes no guest-plane trace; census row 14), SERVICE_UNIMPLEMENTED at a stop
//! added. In count, to the frozen host's own run to the same stop, the exit ledger fixture (`FIXTURE`, Sprint 206):
//! `vm_exits` by hvc and the instructions at the stop (reviewer-3's Medium 2 of its review of Sprint 210: the held stop
//! file is this host's earlier output). In kind, to the run's stop and its calls in flight: the end its stop names, the
//! service the Windows call in flight there, `guest_trace` false with the empty trace (0.12, P22).

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;
use w2m_vm::record::w2m_events;

const REFERENCE: &str = "store/runs/frontier-643a6283510c-1792623423346201000";
/// The frozen host's exits to the game's first MAP_VIEW, the Rust run's stop (WORKING_AGREEMENT gives its generator)
const FIXTURE: &str = "build/exit_ledger/python.tsv";
/// The empty trace's sha256 (P22)
const EMPTY: &str = "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

/// What the record is held to besides the reference: the run's stop line, the frozen host's exits by kind and its
/// instructions at the stop, the Windows calls in flight at the stop ("DLL!Function"), and the calls traced.
struct Facts {
    stop: String,
    exits: BTreeMap<String, f64>,
    executed: u64,
    in_flight: Vec<String>,
    traced: u64,
}

/// The end a stop names, as `run_frontier.py` 400-407 maps it.
fn expected_status(stop: &str) -> &'static str {
    if stop.starts_with("HostRequest op ") {
        "killed"
    } else if stop.starts_with("ExitProcess") {
        "exited"
    } else if stop.contains("the run's time limit") {
        "timed_out"
    } else {
        "faulted"
    }
}
/// The reference's kinds the Rust host does not write: it has no guest-plane trace to ingest or compare
const NOT_WRITTEN: [&str; 2] = ["TRACE_INGESTED", "COMPARISON_COMPLETED"];
/// The measurement fields both hosts set alike (run_frontier.py 174-177)
const CONVENTIONS: [&str; 5] = [
    "phase",
    "backend",
    "application_profile_id",
    "artifact_state",
    "destination_profile_id",
];

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

/// What a record's events say against the reference's and the facts; empty when they agree.
fn problems(ours: &[(String, Value)], reference: &[(String, Value)], facts: &Facts) -> Vec<String> {
    let mut found = Vec::new();
    let of = |events: &[(String, Value)], kind: &str| -> Vec<Value> {
        events
            .iter()
            .filter(|(k, _)| k == kind)
            .map(|(_, p)| p.clone())
            .collect()
    };
    // the kinds
    let allowed: BTreeSet<&str> = reference
        .iter()
        .map(|(k, _)| k.as_str())
        .filter(|k| !NOT_WRITTEN.contains(k))
        .chain(["SERVICE_UNIMPLEMENTED"])
        .collect();
    for (k, _) in ours {
        if !allowed.contains(k.as_str()) {
            found.push(format!("a {k}, which the reference's record does not hold"));
        }
    }
    for k in [
        "SESSION_INIT",
        "EXECUTION_STARTED",
        "EXECUTION_COMPLETED",
        "SESSION_COMPLETED",
    ] {
        if of(ours, k).len() != 1 {
            found.push(format!("{} {k} where the reference has one", of(ours, k).len()));
        }
    }
    // EXECUTION_STARTED, field for field but the run's id
    if let (Some(a), Some(b)) = (
        of(ours, "EXECUTION_STARTED").first(),
        of(reference, "EXECUTION_STARTED").first(),
    ) {
        let fields = |p: &Value| -> BTreeMap<String, Value> {
            p.as_object()
                .unwrap()
                .iter()
                .filter(|(k, _)| k.as_str() != "execution_id")
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect()
        };
        let (fa, fb) = (fields(a), fields(b));
        for k in fa.keys().chain(fb.keys()).collect::<BTreeSet<_>>() {
            if fa.get(k) != fb.get(k) {
                found.push(format!(
                    "EXECUTION_STARTED {k}: {} where the reference has {}",
                    fa.get(k).map_or("nothing".into(), Value::to_string),
                    fb.get(k).map_or("nothing".into(), Value::to_string)
                ));
            }
        }
    }
    // each measurement: a metric the reference has, in its unit, with its conventions
    let units: BTreeMap<String, Value> = of(reference, "MEASUREMENT_RECORDED")
        .iter()
        .map(|m| (m["metric"].as_str().unwrap().to_string(), m["unit"].clone()))
        .collect();
    let convention = of(reference, "MEASUREMENT_RECORDED")[0].clone();
    for m in of(ours, "MEASUREMENT_RECORDED") {
        let metric = m["metric"].as_str().unwrap_or("");
        match units.get(metric) {
            None => found.push(format!(
                "a measurement of {metric}, which the reference does not measure"
            )),
            Some(unit) if *unit != m["unit"] => {
                found.push(format!("{metric} in {} where the reference has {unit}", m["unit"]))
            }
            Some(_) => {}
        }
        for k in CONVENTIONS {
            if m[k] != convention[k] {
                found.push(format!(
                    "{metric} {}: {k} {} where the reference has {}",
                    m["variant"], m[k], convention[k]
                ));
            }
        }
    }
    // the counts, to the frozen host's run to the same stop
    let exits: BTreeMap<String, f64> = of(ours, "MEASUREMENT_RECORDED")
        .iter()
        .filter(|m| m["metric"] == "vm_exits")
        .map(|m| (m["variant"].as_str().unwrap().to_string(), m["value"].as_f64().unwrap()))
        .collect();
    if exits != facts.exits {
        found.push(format!(
            "vm_exits {exits:?} where the frozen host's run made {:?}",
            facts.exits
        ));
    }
    // the end, by the stop's kind; a service not implemented is the Windows call in flight at the stop
    let status = expected_status(&facts.stop);
    let unimplemented = of(ours, "SERVICE_UNIMPLEMENTED");
    match (status, unimplemented.as_slice()) {
        ("killed", [u]) => {
            let named = u["service"].as_str().unwrap_or("");
            if !facts.in_flight.iter().any(|c| c == named) {
                found.push(format!(
                    "SERVICE_UNIMPLEMENTED {named:?} where the calls in flight are {:?}",
                    facts.in_flight
                ));
            }
            if u["instructions_executed"].as_u64() != Some(facts.executed) {
                found.push(format!(
                    "SERVICE_UNIMPLEMENTED instructions_executed {} where the frozen host's run had {}",
                    u["instructions_executed"], facts.executed
                ));
            }
        }
        ("killed", _) => found.push(format!(
            "{} SERVICE_UNIMPLEMENTED where the stop {:?} needs one",
            unimplemented.len(),
            facts.stop
        )),
        (_, []) => {}
        (_, _) => found.push(format!("a SERVICE_UNIMPLEMENTED where the stop is {:?}", facts.stop)),
    }
    if let Some(done) = of(ours, "EXECUTION_COMPLETED").first() {
        if done["exit_status"] != status {
            found.push(format!(
                "EXECUTION_COMPLETED exit_status {} where the stop {:?} says {status}",
                done["exit_status"], facts.stop
            ));
        }
        // P22: no guest-plane trace, so the empty trace and 0 lines; the call stream named in `unobserved`
        if (
            done["guest_trace"].clone(),
            done["trace_sha256"].clone(),
            done["guest_line_count"].clone(),
        ) != (Value::Bool(false), Value::from(EMPTY), Value::from(0))
        {
            found.push(format!(
                "EXECUTION_COMPLETED guest_trace {}, trace_sha256 {}, guest_line_count {} where P22 gives false, the \
                 empty trace and 0",
                done["guest_trace"], done["trace_sha256"], done["guest_line_count"]
            ));
        }
        let rows = format!("{} rows", facts.traced);
        if !done["unobserved"].as_str().is_some_and(|u| u.contains(&rows)) {
            found.push(format!(
                "EXECUTION_COMPLETED unobserved {} does not name the call stream's {rows}",
                done["unobserved"]
            ));
        }
    }
    found
}

/// The facts from the gates' files: the stop line and the calls traced from the run's stop file, the calls in flight
/// beside it, and the frozen host's exits and instructions from the ledger fixture's rows.
fn facts(root: &Path, stop_file: &str, fixture: &str) -> Facts {
    let row = |name: &str| {
        stop_file
            .lines()
            .find_map(|l| l.strip_prefix(&format!("{name}\t")))
            .unwrap_or("")
            .to_string()
    };
    let mut exits = BTreeMap::new();
    let mut executed = 0;
    for line in fixture.lines() {
        let cells: Vec<&str> = line.split('\t').collect();
        *exits.entry(cells[1].to_string()).or_insert(0.0) += 1.0;
        executed = cells[3].parse().unwrap();
    }
    let in_flight = fs::read_to_string(root.join("build/batch_run/first-stop.in-flight.tsv"))
        .unwrap_or_default()
        .lines()
        .map(|l| {
            let cells: Vec<&str> = l.split('\t').collect();
            format!("{}!{}", cells[3], cells[4])
        })
        .collect();
    Facts {
        stop: row("stop"),
        exits,
        executed,
        in_flight,
        traced: row("calls traced").parse().unwrap_or(0),
    }
}

#[test]
fn the_batch_runs_record_is_the_frozen_hosts_in_form_and_its_own_stop_in_count() {
    let root = root();
    // the newest build/batch_run/record-<ns>
    let record = fs::read_dir(root.join("build/batch_run"))
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.is_dir()
                && p.file_name()
                    .is_some_and(|n| n.to_string_lossy().starts_with("record-"))
        })
        .max();
    let Some(record) = record else {
        let what = "no build/batch_run/record-<ns> (scripts/gates.sh writes it)";
        if std::env::var("W2M_REQUIRE_CASE_RUNS").as_deref() == Ok("1") {
            panic!("{what}, and W2M_REQUIRE_CASE_RUNS=1 requires it");
        }
        eprintln!("{what}: the batch run's record is not held here");
        return;
    };
    // the run's stop file and the frozen host's fixture, both written by the gates; their absence fails under them
    let (Ok(stop_file), Ok(fixture)) = (
        fs::read_to_string(root.join("build/batch_run/first-stop.tsv")),
        fs::read_to_string(root.join(FIXTURE)),
    ) else {
        panic!("build/batch_run/first-stop.tsv and {FIXTURE} are written beside the record, and one is missing");
    };
    let ours = w2m_events(&record);
    let reference = w2m_events(&root.join(REFERENCE));
    let found = problems(&ours, &reference, &facts(&root, &stop_file, &fixture));
    assert!(found.is_empty(), "the batch run's record:\n{}", found.join("\n"));
}

/// The judge names what it finds: a record that agrees, then one with its exits, its start and its stop each moved.
#[test]
fn a_moved_field_is_named() {
    let reference = w2m_events(&root().join(REFERENCE));
    let started = reference
        .iter()
        .find(|(k, _)| k == "EXECUTION_STARTED")
        .unwrap()
        .1
        .clone();
    let measurement = |variant: &str, value: f64| {
        let mut m = reference
            .iter()
            .find(|(k, _)| k == "MEASUREMENT_RECORDED")
            .unwrap()
            .1
            .clone();
        m["metric"] = "vm_exits".into();
        m["unit"] = "exits".into();
        m["variant"] = variant.into();
        m["value"] = value.into();
        ("MEASUREMENT_RECORDED".to_string(), m)
    };
    let facts = Facts {
        stop: "HostRequest op 29: host request 29: not served by the Rust host yet".into(),
        exits: BTreeMap::from([("hvc 8".to_string(), 779.0)]),
        executed: 3_541_969,
        in_flight: vec!["KERNEL32.dll!MapViewOfFile".into()],
        traced: 925,
    };
    let record = |exits: f64, started: Value, service: &str, status: &str| {
        vec![
            ("SESSION_INIT".to_string(), Value::Null),
            ("EXECUTION_STARTED".to_string(), started),
            measurement("hvc 8", exits),
            (
                "SERVICE_UNIMPLEMENTED".to_string(),
                serde_json::json!({ "service": service, "instructions_executed": 3_541_969 }),
            ),
            (
                "EXECUTION_COMPLETED".to_string(),
                serde_json::json!({ "exit_status": status, "guest_line_count": 0, "guest_trace": false,
                    "trace_sha256": EMPTY, "unobserved": "no guest-plane trace: the Rust host writes the traced \
                    images' calls, 925 rows, sha256 …; the run stopped: …" }),
            ),
            ("SESSION_COMPLETED".to_string(), Value::Null),
        ]
    };
    let good = record(779.0, started.clone(), "KERNEL32.dll!MapViewOfFile", "killed");
    assert_eq!(problems(&good, &reference, &facts), Vec::<String>::new());
    let mut moved = started.clone();
    moved.as_object_mut().unwrap().remove("kill_grace_seconds");
    let found = problems(&record(780.0, moved, "host request 29", "faulted"), &reference, &facts);
    assert_eq!(found.len(), 4, "{found:?}");
    assert!(
        found[0].starts_with("EXECUTION_STARTED kill_grace_seconds: nothing where the reference has 60"),
        "{found:?}"
    );
    assert!(found[1].starts_with("vm_exits {\"hvc 8\": 780.0}"), "{found:?}");
    assert!(
        found[2].starts_with("SERVICE_UNIMPLEMENTED \"host request 29\" where the calls in flight"),
        "{found:?}"
    );
    assert!(
        found[3].starts_with("EXECUTION_COMPLETED exit_status \"faulted\""),
        "{found:?}"
    );
    // the stop's kind decides the end: a failure in serving a request is a fault and names no service
    let failed = Facts {
        stop: "HostFailure op 29: a guest write".into(),
        ..facts
    };
    let found = problems(&good, &reference, &failed);
    assert!(
        found
            .iter()
            .any(|f| f.starts_with("a SERVICE_UNIMPLEMENTED where the stop")),
        "{found:?}"
    );
    assert!(found.iter().any(|f| f.contains("says faulted")), "{found:?}");
}
