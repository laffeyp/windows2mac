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
/// The frozen host's exits to the game's first MAP_VIEW (WORKING_AGREEMENT gives its generator): the independent
/// count when the Rust run stops at that request; past it (Sprint 211) no frozen-host count is on disk until the
/// fixture runs further, and `exit_ledger.rs` holds the Rust ledger's rows to it up to MAP_VIEW
const FIXTURE: &str = "build/exit_ledger/python.tsv";
/// The run's held stop, whose exit counts hold the record's past the fixture
const HELD: &str = "store/batch_run/first-stop-2026-10-28.tsv";
/// The empty trace's sha256 (P22)
const EMPTY: &str = "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

/// What the record is held to besides the reference: the run's stop line, the exits by kind and the instructions at
/// the stop (the frozen host's where its fixture stops at the same request, else the held stop file's exits and no
/// instruction count), the Windows calls in flight at the stop ("DLL!Function"), and the calls traced.
struct Facts {
    stop: String,
    exits: BTreeMap<String, f64>,
    /// the source of `exits`, named in a failure
    exits_from: &'static str,
    executed: Option<u64>,
    in_flight: Vec<String>,
    traced: u64,
}

/// The end a stop names, as `run_frontier.py` 400-407 maps it.
fn expected_status(stop: &str) -> &'static str {
    if stop.starts_with("HostRequest op ") || stop.starts_with("StepLimit: ") || stop.starts_with("ReferenceEnd: ") {
        "killed"
    } else if stop.starts_with("ExitProcess") {
        "exited"
    } else if stop.contains("the run's time limit") {
        "timed_out"
    } else {
        "faulted"
    }
}
/// The reference's kinds the batch run's record does not hold: the Rust host has no guest-plane trace to ingest (its
/// call stream is compared from Sprint 214, COMPARISON_COMPLETED, by the second Producer from Sprint 217)
const NOT_WRITTEN: [&str; 1] = ["TRACE_INGESTED"];
/// The measurement fields both hosts set alike (run_frontier.py 174-177); not `artifact_state`, "warm" on the
/// reference, whose translation cache had hits, and "cold" on the Rust host, which keeps none (Sprint 240)
const CONVENTIONS: [&str; 4] = ["phase", "backend", "application_profile_id", "destination_profile_id"];

/// `measurement_problems` (run_frontier.py 189-200), the rate's half (Sprint 240): `instructions_per_second` times the
/// record's `run_seconds`, every variant, is its `instructions_retired`, to one part in a million; and each measurement
/// is `cold`, the Rust host having no translation cache.
fn rate_problems(measurements: &[Value]) -> Vec<String> {
    let mut found = Vec::new();
    let value = |metric: &str, variant: &str| {
        measurements
            .iter()
            .find(|m| m["metric"] == metric && m["variant"] == variant)
            .and_then(|m| m["value"].as_f64())
    };
    let seconds: f64 = measurements
        .iter()
        .filter(|m| m["metric"] == "run_seconds")
        .filter_map(|m| m["value"].as_f64())
        .sum();
    match (
        value("instructions_per_second", "process"),
        value("instructions_retired", "process"),
    ) {
        (Some(rate), Some(instructions)) if (rate * seconds - instructions).abs() <= 1e-6 * instructions => {}
        (Some(rate), Some(instructions)) => found.push(format!(
            "instructions_per_second {rate} over {seconds} s is {} instructions, where {instructions} retired",
            rate * seconds
        )),
        _ => found.push("no instructions_per_second or instructions_retired for the process".into()),
    }
    // the load at the end, as run_frontier.py 173 writes it
    for span in ["1 min", "5 min", "15 min"] {
        if value("load_average", span).is_none() {
            found.push(format!("no load_average {span} at the run's end"));
        }
    }
    for m in measurements {
        if m["artifact_state"] != "cold" {
            found.push(format!(
                "{} {}: artifact_state {}, where the Rust host keeps no translation cache",
                m["metric"], m["variant"], m["artifact_state"]
            ));
        }
    }
    found
}

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
        "COMPARISON_COMPLETED",
        "SESSION_COMPLETED",
    ] {
        if of(ours, k).len() != 1 {
            found.push(format!("{} {k} where the reference has one", of(ours, k).len()));
        }
    }
    // the session closes after the comparison, which the second Producer writes after the run's completion (Sprint
    // 217), and the comparison judges this run
    let order: Vec<&str> = ours.iter().map(|(k, _)| k.as_str()).collect();
    if order.last() != Some(&"SESSION_COMPLETED")
        || order.iter().position(|k| *k == "COMPARISON_COMPLETED")
            < order.iter().position(|k| *k == "EXECUTION_COMPLETED")
    {
        found.push(format!(
            "the record's events end {:?}",
            &order[order.len().saturating_sub(3)..]
        ));
    }
    if let (Some(c), Some(done)) = (
        of(ours, "COMPARISON_COMPLETED").first(),
        of(ours, "EXECUTION_COMPLETED").first(),
    ) && c["target_execution_id"] != done["execution_id"]
    {
        found.push(format!(
            "COMPARISON_COMPLETED judges {} where the run is {}",
            c["target_execution_id"], done["execution_id"]
        ));
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
            "vm_exits {exits:?} where {} holds {:?}",
            facts.exits_from, facts.exits
        ));
    }
    // the end, by the stop's kind; a service not implemented is the Windows call in flight at the stop, and a stop with
    // no call in flight (a created thread's stack growing, Sprint 211) names none
    let status = expected_status(&facts.stop);
    // the batch run ends where its reference did (Sprint 219); a step-budget stop is a game that never filled its log
    // (reviewer-3's Concern 3 of its review of Sprint 218)
    if facts.stop.starts_with("StepLimit: ") {
        found.push(format!(
            "the batch run spent the step budget, {:?}, before its log held the reference's lines",
            facts.stop
        ));
    }
    let unimplemented = of(ours, "SERVICE_UNIMPLEMENTED");
    match (status, unimplemented.as_slice()) {
        ("killed", []) if facts.in_flight.is_empty() => {}
        ("killed", [_]) if facts.in_flight.is_empty() => found.push(format!(
            "a SERVICE_UNIMPLEMENTED where no Windows call is in flight at the stop {:?}",
            facts.stop
        )),
        ("killed", [u]) => {
            let named = u["service"].as_str().unwrap_or("");
            if !facts.in_flight.iter().any(|c| c == named) {
                found.push(format!(
                    "SERVICE_UNIMPLEMENTED {named:?} where the calls in flight are {:?}",
                    facts.in_flight
                ));
            }
            if let Some(executed) = facts.executed
                && u["instructions_executed"].as_u64() != Some(executed)
            {
                found.push(format!(
                    "SERVICE_UNIMPLEMENTED instructions_executed {} where the frozen host's run had {executed}",
                    u["instructions_executed"]
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
    // the comparison against Windows (Sprint 214): the reference's contract and reference set, a verdict of conforms
    // (`call_compare.rs` holds the divergences themselves)
    if let (Some(c), Some(r)) = (
        of(ours, "COMPARISON_COMPLETED").first(),
        of(reference, "COMPARISON_COMPLETED").first(),
    ) {
        for k in ["contract_id", "contract_version", "relation", "reference_ref"] {
            if c[k] != r[k] {
                found.push(format!(
                    "COMPARISON_COMPLETED {k} {} where the reference has {}",
                    c[k], r[k]
                ));
            }
        }
        if c["verdict"] != "conforms" {
            found.push(format!(
                "COMPARISON_COMPLETED verdict {}, first divergence {}",
                c["verdict"], c["first_divergence"]
            ));
        }
    }
    found
}

/// The facts from the gates' files: the stop line and the calls traced from the run's stop file, the calls in flight
/// beside it, and the exits and instructions: the ledger fixture's when its last row is the request the run stopped at
/// (its op, the fifth column, hex), else the held stop file's exits.
fn facts(root: &Path, stop_file: &str, fixture: &str, held: &str) -> Facts {
    let row = |name: &str| {
        stop_file
            .lines()
            .find_map(|l| l.strip_prefix(&format!("{name}\t")))
            .unwrap_or("")
            .to_string()
    };
    let stop = row("stop");
    let stop_op = stop
        .strip_prefix("HostRequest op ")
        .and_then(|s| s.split(':').next())
        .and_then(|op| op.parse::<u64>().ok());
    let fixture_op = fixture
        .lines()
        .last()
        .and_then(|l| l.split('\t').nth(5))
        .and_then(|op| u64::from_str_radix(op, 16).ok());
    let (exits, exits_from, executed) = if stop_op.is_some() && stop_op == fixture_op {
        let mut exits = BTreeMap::new();
        let mut executed = 0;
        for line in fixture.lines() {
            let cells: Vec<&str> = line.split('\t').collect();
            *exits.entry(cells[1].to_string()).or_insert(0.0) += 1.0;
            executed = cells[3].parse().unwrap();
        }
        (exits, "the frozen host's run to the same stop", Some(executed))
    } else {
        let exits = held
            .lines()
            .filter(|l| l.starts_with("hvc "))
            .filter_map(|l| l.split_once('\t'))
            .map(|(k, v)| (k.to_string(), v.parse().unwrap()))
            .collect();
        (exits, "the held stop file", None)
    };
    let in_flight = fs::read_to_string(root.join("build/batch_run/first-stop.in-flight.tsv"))
        .unwrap_or_default()
        .lines()
        .map(|l| {
            let cells: Vec<&str> = l.split('\t').collect();
            format!("{}!{}", cells[3], cells[4])
        })
        .collect();
    Facts {
        stop,
        exits,
        exits_from,
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
    let held = fs::read_to_string(root.join(HELD)).unwrap();
    let mut found = problems(&ours, &reference, &facts(&root, &stop_file, &fixture, &held));
    found.extend(rate_problems(&measurements(&ours)));
    found.extend(condition_problems(&record));
    assert!(found.is_empty(), "the batch run's record:\n{}", found.join("\n"));
}

/// The machine's state at the run's start, in the baseline `scripts/batch-run-record.sh` writes (Sprint 240; reviewer-3's
/// Medium 1 of its review at 240): `load_at_start`, run_frontier.py's three parts as one string ("load averages 5.10
/// 3.65 3.39; busy: …; other w2m: …"), the QoS class and the cores.
fn condition_problems(record: &Path) -> Vec<String> {
    let first = fs::read_dir(record)
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.file_name().unwrap().to_string_lossy().starts_with("events-"))
        .min()
        .and_then(|p| fs::read_to_string(p).ok())
        .and_then(|t| t.lines().next().map(str::to_string))
        .unwrap_or_default();
    let first: Value = serde_json::from_str(&first).unwrap_or(Value::Null);
    let b = &first["payload"]["baseline"];
    let mut found = Vec::new();
    let load = b["load_at_start"].as_str().unwrap_or("");
    let parts: Vec<&str> = load.split("; ").collect();
    let averages = parts.first().and_then(|p| p.strip_prefix("load averages ")).map(|p| {
        p.split(' ')
            .filter(|n| n.parse::<f64>().is_ok_and(|v| v >= 0.0))
            .count()
    });
    if parts.len() != 3
        || averages != Some(3)
        || !parts[1].starts_with("busy: ")
        || !parts[2].starts_with("other w2m: ")
    {
        found.push(format!(
            "load_at_start is {:?}, not \"load averages <1> <5> <15>; busy: …; other w2m: …\"",
            b["load_at_start"]
        ));
    }
    for k in ["qos", "cores"] {
        if !b[k].as_str().is_some_and(|s| !s.is_empty()) {
            found.push(format!("the baseline's {k} is {}", b[k]));
        }
    }
    found
}

fn measurements(events: &[(String, Value)]) -> Vec<Value> {
    events
        .iter()
        .filter(|(k, _)| k == "MEASUREMENT_RECORDED")
        .map(|(_, p)| p.clone())
        .collect()
}

/// Every committed batch-run record of the Rust host (baseline `host` `w2m-vm`, under `store/runs/`; Sprint 240's speed
/// records): its rate is its instructions over its seconds, every measurement cold, and its comparison conforms.
#[test]
fn every_stored_rust_batch_record_holds_its_rate() {
    let root = root();
    let mut held = 0;
    for entry in fs::read_dir(root.join("store/runs")).unwrap() {
        let dir = entry.unwrap().path();
        if !dir.file_name().unwrap().to_string_lossy().starts_with("w2m-vm-game-") {
            continue;
        }
        let ours = w2m_events(&dir);
        let mut found = rate_problems(&measurements(&ours));
        found.extend(condition_problems(&dir));
        for (k, p) in &ours {
            if k == "COMPARISON_COMPLETED" && p["verdict"] != "conforms" {
                found.push(format!("COMPARISON_COMPLETED verdict {}", p["verdict"]));
            }
        }
        assert!(found.is_empty(), "{}:\n{}", dir.display(), found.join("\n"));
        held += 1;
    }
    assert!(
        held >= 3,
        "{held} records store/runs/w2m-vm-game-<ns>, where Sprint 240 committed three"
    );
}

/// The rate check names a rate that is not the instructions over the seconds, and a warm measurement.
#[test]
fn a_rate_off_by_a_part_in_a_thousand_is_named() {
    let m = |metric: &str, variant: &str, value: f64| serde_json::json!({"metric": metric, "variant": variant, "value": value, "artifact_state": "cold"});
    let good = vec![
        m("instructions_retired", "process", 3.0e9),
        m("instructions_per_second", "process", 1.0e9),
        m("run_seconds", "vm", 2.5),
        m("run_seconds", "host", 0.5),
        m("load_average", "1 min", 1.5),
        m("load_average", "5 min", 2.0),
        m("load_average", "15 min", 2.5),
    ];
    assert_eq!(rate_problems(&good), Vec::<String>::new());
    let mut off = good.clone();
    off[1]["value"] = 1.001e9.into();
    off[2]["artifact_state"] = "warm".into();
    let found = rate_problems(&off);
    assert_eq!(found.len(), 2, "{found:?}");
    assert!(
        found[0].starts_with("instructions_per_second 1001000000 over 3 s"),
        "{found:?}"
    );
    assert!(found[1].contains("artifact_state \"warm\""), "{found:?}");
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
        exits_from: "the frozen host's run to the same stop",
        executed: Some(3_541_969),
        in_flight: vec!["KERNEL32.dll!MapViewOfFile".into()],
        traced: 925,
    };
    // the reference's comparison, judging this record's run
    let mut comparison = reference
        .iter()
        .find(|(k, _)| k == "COMPARISON_COMPLETED")
        .unwrap()
        .1
        .clone();
    comparison["target_execution_id"] = "frontier-643a6283510c-1".into();
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
                serde_json::json!({ "execution_id": "frontier-643a6283510c-1", "exit_status": status,
                    "guest_line_count": 0, "guest_trace": false, "trace_sha256": EMPTY,
                    "unobserved": "no guest-plane trace: the Rust host writes the traced images' calls, 925 rows, \
                    sha256 …; the run stopped: …" }),
            ),
            ("COMPARISON_COMPLETED".to_string(), comparison.clone()),
            ("SESSION_COMPLETED".to_string(), Value::Null),
        ]
    };
    let good = record(779.0, started.clone(), "KERNEL32.dll!MapViewOfFile", "killed");
    assert_eq!(problems(&good, &reference, &facts), Vec::<String>::new());
    // the judge's Producer not started: no comparison, the session closed by nothing after the run
    let unjudged: Vec<(String, Value)> = good
        .iter()
        .filter(|(k, _)| k != "COMPARISON_COMPLETED")
        .cloned()
        .collect();
    assert_eq!(
        problems(&unjudged, &reference, &facts),
        [
            "0 COMPARISON_COMPLETED where the reference has one",
            "the record's events end [\"SERVICE_UNIMPLEMENTED\", \"EXECUTION_COMPLETED\", \"SESSION_COMPLETED\"]"
        ]
    );
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
    // a request unserved with no Windows call in flight names no service (Sprint 211's stop)
    let between_calls = Facts {
        stop: "HostRequest op 30: thread 8's stack grown to 0x1180dce5f0".into(),
        exits: BTreeMap::from([("hvc 8".to_string(), 779.0)]),
        exits_from: "the frozen host's run to the same stop",
        executed: Some(3_541_969),
        in_flight: Vec::new(),
        traced: 925,
    };
    let found = problems(&good, &reference, &between_calls);
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(
        found[0].starts_with("a SERVICE_UNIMPLEMENTED where no Windows call is in flight"),
        "{found:?}"
    );
}
