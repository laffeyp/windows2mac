// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! Step 6's figures on a record (Sprint 337): the newest `store/runs/w2m-vm-measurements-*` record, or the one
//! `W2M_MEASUREMENTS_RECORD` names (a plant's scratch record), holds `locked_race`'s run on two vCPUs and the three
//! capabilities `vcpus_allowed` and the run-loop design read. The test reads files only, so it runs on CI's Linux too.

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;
use w2m_vm::record::w2m_events;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The record the test holds: `W2M_MEASUREMENTS_RECORD`, or the measurements record with the largest nanosecond
/// suffix under `store/runs/`.
fn record() -> PathBuf {
    if let Ok(path) = std::env::var("W2M_MEASUREMENTS_RECORD") {
        return PathBuf::from(path);
    }
    fs::read_dir(root().join("store/runs"))
        .unwrap()
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            let ns = name.strip_prefix("w2m-vm-measurements-")?.parse::<u128>().ok()?;
            Some((ns, e.path()))
        })
        .max()
        .expect("no store/runs/w2m-vm-measurements-* record")
        .1
}

/// The payloads of `kind` in the record's events, in order.
fn of_kind(events: &[(String, Value)], kind: &str) -> Vec<Value> {
    events
        .iter()
        .filter(|(k, _)| k == kind)
        .map(|(_, p)| p.clone())
        .collect()
}

/// The one MEASUREMENT_RECORDED of `metric`.
fn one_measurement(measurements: &[Value], metric: &str) -> Value {
    let rows: Vec<&Value> = measurements.iter().filter(|p| p["metric"] == metric).collect();
    assert_eq!(rows.len(), 1, "{metric}: {} rows where one is held", rows.len());
    rows[0].clone()
}

/// The record's two-vCPU rows: `locked_model_steps` is thread A's adds on Windows run 41326239822, each a locked model
/// step; the turn's start has a median above 0 and no above its 99th percentile; each carries `vcpus` 2, and every
/// other measurement carries none, the one-vCPU form.
#[test]
fn the_two_vcpu_run_is_on_the_record() {
    let record = record();
    let events = w2m_events(&record);
    let measurements = of_kind(&events, "MEASUREMENT_RECORDED");
    let windows = fs::read_to_string(root().join("store/probe-runs/41326239822/body-1.jsonl")).unwrap();
    let adds = windows
        .lines()
        .map(|l| serde_json::from_str::<Value>(l).unwrap())
        .find(|r| r["s"] == "a")
        .expect("Windows run 41326239822's row of thread A")["sum"]
        .as_f64()
        .unwrap();
    let locked = one_measurement(&measurements, "locked_model_steps");
    assert_eq!(
        locked["value"].as_f64(),
        Some(adds),
        "{}: locked steps",
        record.display()
    );
    assert_eq!(locked["unit"], "count");
    let p50 = one_measurement(&measurements, "turn_start_ns_p50");
    let p99 = one_measurement(&measurements, "turn_start_ns_p99");
    let (low, high) = (p50["value"].as_f64().unwrap(), p99["value"].as_f64().unwrap());
    assert!(
        0.0 < low && low <= high,
        "{}: the turn's start, median {low} ns and 99th percentile {high} ns",
        record.display()
    );
    for row in [&locked, &p50, &p99] {
        assert_eq!(row["vcpus"], 2, "{}: {}'s vCPUs", record.display(), row["metric"]);
    }
    // Sprint 356 (its amendment 2): semaphore_units' runtime word on two vCPUs, the waits' and releases' run, each row
    // with vcpus 2 and the four runtime-word metrics among them, a hold's median at most its 99th percentile
    let units: Vec<&Value> = measurements
        .iter()
        .filter(|p| p["application_profile_id"] == "timed-waits-semaphore-units")
        .collect();
    for metric in [
        "runtime_word_hold_ns_p50",
        "runtime_word_hold_ns_p99",
        "runtime_word_wait_ns_p50",
        "runtime_word_wait_ns_p99",
    ] {
        assert!(
            units.iter().any(|p| p["metric"] == metric),
            "{}: no semaphore_units {metric} on two vCPUs",
            record.display()
        );
    }
    for row in &units {
        assert_eq!(
            row["vcpus"],
            2,
            "{}: {}'s vCPUs",
            record.display(),
            row["measurement_id"]
        );
    }
    let two_vcpu = ["locked_model_steps", "turn_start_ns_p50", "turn_start_ns_p99"];
    for row in measurements.iter().filter(|p| {
        !two_vcpu.iter().any(|m| p["metric"] == *m) && p["application_profile_id"] != "timed-waits-semaphore-units"
    }) {
        assert!(
            row["vcpus"].is_null(),
            "{}: {} carries vcpus {}, where a one-vCPU measurement carries none",
            record.display(),
            row["measurement_id"],
            row["vcpus"]
        );
    }
}

/// The record's three capabilities, each once: `feat_lse2` present on this Mac (`hypervisor/src/lib.rs` 187),
/// `feat_bbm_level2` present only at a level of 2 or more, the level read from its `detail`, and `guest_wfe_exit`
/// present only when the WFE probe's row on the same record counts a trap.
#[test]
fn the_three_capabilities_are_on_the_record() {
    let record = record();
    let events = w2m_events(&record);
    let probed = of_kind(&events, "CAPABILITY_PROBED");
    let one = |capability: &str| {
        let rows: Vec<&Value> = probed.iter().filter(|p| p["capability"] == capability).collect();
        assert_eq!(rows.len(), 1, "{}: {capability} {} times", record.display(), rows.len());
        rows[0].clone()
    };
    let present = |yes: bool| if yes { "present" } else { "absent" };
    assert_eq!(one("feat_lse2")["outcome"], "present", "{}", record.display());
    let bbm = one("feat_bbm_level2");
    let detail = bbm["detail"].as_str().unwrap();
    let level: u64 = detail
        .strip_prefix("FEAT_BBM level ")
        .and_then(|rest| rest.split(':').next())
        .and_then(|n| n.parse().ok())
        .unwrap_or_else(|| panic!("{}: no level in {detail:?}", record.display()));
    assert_eq!(
        bbm["outcome"],
        present(level >= 2),
        "{}: feat_bbm_level2 at level {level}",
        record.display()
    );
    let measurements = of_kind(&events, "MEASUREMENT_RECORDED");
    let wfe = measurements
        .iter()
        .find(|p| p["metric"] == "wfe_exits" && p["variant"] == "wfe")
        .expect("the WFE probe's wfe row")["value"]
        .as_f64()
        .unwrap();
    assert_eq!(
        one("guest_wfe_exit")["outcome"],
        present(wfe > 0.0),
        "{}: guest_wfe_exit with {wfe} WFE traps",
        record.display()
    );
}
