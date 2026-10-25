// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! The Rust judges' discrepancy ledger (Sprint 152), carried from `src/w2m/differential.py` `rebuild_ledger` and
//! `open_on` (105-139) for records whose baseline names a `judge` (the case runner's, Sprint 150). The Python's ledger,
//! `store/runs/discrepancy-ledger.json`, holds the records whose baseline names a `differential`, and
//! `tests/test_differential.py` holds it to the Python rebuild; this one is `store/case_runner/discrepancy-ledger.json`.
//! The ledger is a function of the records: a DISCREPANCY_REPORTED opens an entry, a DISCREPANCY_ROUTED gives its repair
//! path, a DISCREPANCY_RESOLVED closes it; records are read in run-start order, so a resolution follows its report.

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{Map, Value, json};

/// One record's frames, in seq order: sealed segments by number, then the open one (signal_report's reader). A line
/// that is not a JSON object (a torn tail) ends the segment, as substrate's reader stops at it.
pub fn frames(record: &Path) -> Vec<Value> {
    let mut sealed = Vec::new();
    let mut open = Vec::new();
    for entry in fs::read_dir(record).into_iter().flatten().flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with("events-") && name.ends_with(".open.jsonl") {
            open.push(entry.path());
        } else if name.starts_with("events-") && name.ends_with(".jsonl") {
            sealed.push(entry.path());
        }
    }
    sealed.sort();
    open.sort();
    let mut out = Vec::new();
    for segment in sealed.into_iter().chain(open) {
        let Ok(text) = fs::read_to_string(&segment) else {
            continue;
        };
        for line in text.split_terminator('\n') {
            match serde_json::from_str::<Value>(line) {
                Ok(v) if v.is_object() => out.push(v),
                _ => break,
            }
        }
    }
    out
}

/// The record's first frame, if it is seq 0: one line of the first segment.
fn first_frame(record: &Path) -> Option<Value> {
    let mut segments: Vec<PathBuf> = fs::read_dir(record)
        .ok()?
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with("events-") && n.ends_with(".jsonl"))
        })
        .collect();
    segments.sort_by_key(|p| (p.to_string_lossy().ends_with(".open.jsonl"), p.clone()));
    let text = fs::read(segments.first()?).ok()?;
    let end = text.iter().position(|&b| b == b'\n').unwrap_or(text.len());
    let frame: Value = serde_json::from_slice(&text[..end]).ok()?;
    (frame.get("seq").and_then(Value::as_i64) == Some(0)).then_some(frame)
}

/// `rebuild_ledger` (105-134) over the records under `runs` whose baseline names a `judge`: every discrepancy a judge
/// reported, with the record that resolved it (or null).
pub fn rebuild(runs: &Path) -> Map<String, Value> {
    let mut started: Vec<(f64, PathBuf, String)> = Vec::new();
    for entry in fs::read_dir(runs).into_iter().flatten().flatten() {
        let root = entry.path();
        if !root.is_dir() {
            continue;
        }
        let Some(first) = first_frame(&root) else {
            continue;
        };
        if let Some(judge) = first.pointer("/payload/baseline/judge").and_then(Value::as_str) {
            let t = first.get("t").and_then(Value::as_f64).unwrap_or(0.0);
            started.push((t, root, judge.to_string()));
        }
    }
    started.sort_by(|a, b| a.0.total_cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
    let mut ledger = Map::new();
    for (_, root, judge) in started {
        let name = root.file_name().unwrap().to_string_lossy().to_string();
        for e in frames(&root) {
            let p = &e["payload"];
            let id = p["discrepancy_id"].as_str().unwrap_or_default().to_string();
            match e["kind"].as_str() {
                Some("DISCREPANCY_REPORTED") => {
                    let first = p["first_divergence"].as_str().unwrap_or_default();
                    ledger.insert(
                        id,
                        json!({
                            "judge": judge,
                            "unit": first.split(':').next().unwrap_or_default(),
                            "contract_id": p["contract_id"],
                            "reported_in": name,
                            "first_divergence": first,
                            "repair_path": null,
                            "resolved_in": null,
                            "resolving_ref": null,
                        }),
                    );
                }
                Some("DISCREPANCY_ROUTED") => {
                    if let Some(entry) = ledger.get_mut(&id) {
                        entry["repair_path"] = p["repair_path"].clone();
                    }
                }
                Some("DISCREPANCY_RESOLVED") => {
                    if let Some(entry) = ledger.get_mut(&id) {
                        entry["resolved_in"] = json!(name);
                        entry["resolving_ref"] = p["resolving_ref"].clone();
                    }
                }
                _ => {}
            }
        }
    }
    ledger
}

/// `open_on` (137-139): the discrepancies `judge` reported on `unit` that no record has resolved, sorted.
pub fn open_on(ledger: &Map<String, Value>, judge: &str, unit: &str) -> Vec<String> {
    let mut out: Vec<String> = ledger
        .iter()
        .filter(|(_, v)| v["judge"] == judge && v["unit"] == unit && v["resolved_in"].is_null())
        .map(|(k, _)| k.clone())
        .collect();
    out.sort();
    out
}

/// The ledger as `differential.run` writes its own (281-282): `json.dumps(ledger, indent=1, sort_keys=True)` and a
/// newline.
pub fn text(ledger: &Map<String, Value>) -> String {
    let mut s = String::new();
    write_value(&mut s, &Value::Object(ledger.clone()), 0);
    s.push('\n');
    s
}

/// `json.dumps`' string: ASCII only (`ensure_ascii`), a character past it as `\uXXXX`, in surrogate pairs past the BMP.
fn write_string(out: &mut String, s: &str) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            c if (c as u32) < 0x20 || (c as u32) > 0x7E => {
                let mut units = [0u16; 2];
                for u in c.encode_utf16(&mut units) {
                    out.push_str(&format!("\\u{u:04x}"));
                }
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

/// `json.dumps(indent=1, sort_keys=True)`: a key and its value on one line, nested one space deeper; an empty object or
/// list as `{}` or `[]`.
fn write_value(out: &mut String, v: &Value, depth: usize) {
    let pad = |out: &mut String, d: usize| out.push_str(&" ".repeat(d));
    match v {
        Value::Object(m) if m.is_empty() => out.push_str("{}"),
        Value::Object(m) => {
            out.push_str("{\n");
            let mut keys: Vec<&String> = m.keys().collect();
            keys.sort();
            for (i, k) in keys.iter().enumerate() {
                pad(out, depth + 1);
                write_string(out, k);
                out.push_str(": ");
                write_value(out, &m[*k], depth + 1);
                out.push_str(if i + 1 < keys.len() { ",\n" } else { "\n" });
            }
            pad(out, depth);
            out.push('}');
        }
        Value::Array(a) if a.is_empty() => out.push_str("[]"),
        Value::Array(a) => {
            out.push_str("[\n");
            for (i, x) in a.iter().enumerate() {
                pad(out, depth + 1);
                write_value(out, x, depth + 1);
                out.push_str(if i + 1 < a.len() { ",\n" } else { "\n" });
            }
            pad(out, depth);
            out.push(']');
        }
        Value::String(s) => write_string(out, s),
        other => out.push_str(&other.to_string()), // null, true, false, numbers as JSON writes them
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(dir: &Path, name: &str, t: f64, judge: Option<&str>, events: &[Value]) {
        let root = dir.join(name);
        fs::create_dir_all(&root).unwrap();
        let baseline = match judge {
            Some(j) => json!({"judge": j}),
            None => json!({"differential": "decoder-xed"}),
        };
        let mut lines =
            vec![json!({"seq": 0, "t": t, "kind": "substrate.RunStarted", "payload": {"baseline": baseline}})];
        for (i, e) in events.iter().enumerate() {
            let mut e = e.clone();
            e["seq"] = json!(i + 1);
            lines.push(e);
        }
        let text: String = lines.iter().map(|l| format!("{l}\n")).collect();
        fs::write(root.join("events-000000.jsonl"), text).unwrap();
    }

    fn event(kind: &str, payload: Value) -> Value {
        json!({"kind": kind, "payload": payload})
    }

    /// differential.py's chain (tests/test_differential.py 49-82, carried): a report opens an entry, its route gives the
    /// repair path, a later record's resolution closes it; a record without a `judge` is not read.
    #[test]
    fn a_discrepancy_is_resolved_by_the_run_that_fixed_it() {
        let dir = std::env::temp_dir().join(format!("w2m-ledger-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        record(
            &dir,
            "case-runner-1",
            1.0,
            Some("case-runner"),
            &[
                event(
                    "DISCREPANCY_REPORTED",
                    json!({"discrepancy_id": "d1", "contract_id": "w2m.guest_state.fault_context",
                           "first_divergence": "ADD: ADD CF: 3 cases"}),
                ),
                event(
                    "DISCREPANCY_ROUTED",
                    json!({"discrepancy_id": "d1", "repair_path": "implementation_repair"}),
                ),
            ],
        );
        record(
            &dir,
            "decoder-1",
            1.5,
            None,
            &[event(
                "DISCREPANCY_REPORTED",
                json!({"discrepancy_id": "x", "first_divergence": "MOV: no"}),
            )],
        );
        let ledger = rebuild(&dir);
        assert_eq!(ledger.keys().collect::<Vec<_>>(), ["d1"]);
        assert_eq!(open_on(&ledger, "case-runner", "ADD"), ["d1"]);
        assert_eq!(ledger["d1"]["repair_path"], "implementation_repair");
        record(
            &dir,
            "case-runner-2",
            2.0,
            Some("case-runner"),
            &[event(
                "DISCREPANCY_RESOLVED",
                json!({"discrepancy_id": "d1", "resolving_ref": "case-runner:ADD@abc"}),
            )],
        );
        let ledger = rebuild(&dir);
        assert_eq!(ledger["d1"]["resolved_in"], "case-runner-2");
        assert_eq!(ledger["d1"]["resolving_ref"], "case-runner:ADD@abc");
        assert!(open_on(&ledger, "case-runner", "ADD").is_empty());
        assert!(text(&ledger).starts_with("{\n \"d1\": {\n  \"contract_id\""));
        fs::remove_dir_all(&dir).unwrap();
    }

    /// `json.dumps(..., indent=1, sort_keys=True)` of this map, as the frozen Python printed it on 2026-10-25.
    #[test]
    fn the_ledger_text_is_python_s_json() {
        let v = json!({"e": {}, "d1": {"resolved_in": null, "n": [], "contract_id": "x"}});
        let want =
            "{\n \"d1\": {\n  \"contract_id\": \"x\",\n  \"n\": [],\n  \"resolved_in\": null\n },\n \"e\": {}\n}\n";
        assert_eq!(text(v.as_object().unwrap()), want);
    }
}
