// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! The records check's E15 (vocabulary 0.13, P29; Sprint 336): a MEASUREMENT_RECORDED's `holder` rides only on the
//! runtime word's four hold and wait metrics. The built `check_records` runs on two small records written here, each a
//! session with one measurement, named with a nanosecond suffix after 2026-10-17 so `--since` holds what it finds.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

/// A record under the system's temporary directory, holding SESSION_INIT, one MEASUREMENT_RECORDED of `metric` with
/// holder `service`, and SESSION_COMPLETED, its seq from 0; removed when dropped.
struct Record(PathBuf);

impl Record {
    fn new(name: &str, metric: &str) -> Record {
        let ns = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let dir = std::env::temp_dir()
            .join(format!("w2m-records-check-{}", std::process::id()))
            .join(format!("{name}-{ns}"));
        fs::create_dir_all(&dir).unwrap();
        let frames = [
            (
                "SESSION_INIT",
                serde_json::json!({"vocab_version": "0.13", "profile": "w2m-vm", "contract_version": "0.13",
                    "harness_version": "0.1.0", "purpose": "measurement"}),
            ),
            (
                "MEASUREMENT_RECORDED",
                serde_json::json!({"measurement_id": "e15-sample", "metric": metric, "value": 1.0, "unit": "ns",
                    "phase": "discovery", "backend": "managed_vm", "application_profile_id": "e15",
                    "artifact_state": "cold", "destination_profile_id": "w2m-vm", "variant": null, "vcpus": null,
                    "holder": "service"}),
            ),
            (
                "SESSION_COMPLETED",
                serde_json::json!({"outcome": "completed", "discrepancy_count": 0, "accepted_count": 0,
                    "elapsed_seconds": 0.5}),
            ),
        ];
        let lines: String = frames
            .iter()
            .enumerate()
            .map(|(seq, (kind, payload))| {
                format!(
                    "{}\n",
                    serde_json::json!({"kind": kind, "payload": payload, "seq": seq, "t": seq})
                )
            })
            .collect();
        fs::write(dir.join("events-000001.open.jsonl"), lines).unwrap();
        Record(dir)
    }

    fn check(&self) -> Output {
        Command::new(env!("CARGO_BIN_EXE_check_records"))
            .args(["--since", "2026-10-17", "--show", "3"])
            .arg(&self.0)
            .output()
            .unwrap()
    }
}

impl Drop for Record {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// The line `check_records` prints for one rule, its spaces collapsed.
fn rule_line(stdout: &str, rule: &str) -> Option<String> {
    stdout
        .lines()
        .find(|l| l.split_whitespace().next() == Some(rule))
        .map(|l| l.split_whitespace().collect::<Vec<_>>().join(" "))
}

fn name_of(record: &Record) -> String {
    Path::new(&record.0).file_name().unwrap().to_string_lossy().to_string()
}

#[test]
fn a_holder_rides_only_on_a_runtime_word_metric_sprint_336() {
    let on_word = Record::new("e15-holder-on-word", "runtime_word_hold_ns_p50");
    let out = on_word.check();
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success(), "{stdout}{}", String::from_utf8_lossy(&out.stderr));
    assert_eq!(
        rule_line(&stdout, "E15").as_deref(),
        Some("E15 applied 1 violations 0 unchecked 0"),
        "{stdout}"
    );

    let off_word = Record::new("e15-holder-off-word", "instructions_per_second");
    let out = off_word.check();
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(out.status.code(), Some(1), "{stdout}");
    assert_eq!(
        rule_line(&stdout, "E15").as_deref(),
        Some("E15 applied 1 violations 1 unchecked 0"),
        "{stdout}"
    );
    let named = format!(
        "{}: e15-sample: holder service on metric instructions_per_second",
        name_of(&off_word)
    );
    assert!(stdout.contains(&named), "{stdout}");
}
