// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! The Rust judges' discrepancy ledger (Sprint 152), carried from `src/w2m/differential.py` `rebuild_ledger` and
//! `open_on` (105-139) for records whose baseline names a `judge` (the case runner's, Sprint 150). The Python's ledger,
//! `store/runs/discrepancy-ledger.json`, holds the records whose baseline names a `differential`, and
//! `tests/test_differential.py` holds it to the Python rebuild; this one is `store/case_runner/discrepancy-ledger.json`.
//! The ledger is a function of the records, read in run-start order so a resolution follows its report. An entry is a
//! defect, not a report (Sprint 223; reviewer-3's Medium 1 of its review at Sprint 222's close: 20 entries stood for
//! FCOS and FSIN): its key is the unit and the difference's field ("FCOS st0"), not which judge saw it first or on which
//! case, which depends on the runs a judge reads and their order (its Medium 2 of the refreshed review). Each
//! DISCREPANCY_REPORTED adds a report to its defect's entry, opening it, or reopening one a record resolved; a
//! DISCREPANCY_ROUTED gives its report's repair path; a DISCREPANCY_RESOLVED resolves the report it names, and the entry
//! closes when every report in it is resolved. A judge resolves only what it reported, on the profile and runs it
//! judged, so one judge's acceptance on its profile does not close another's report on another (reviewer-3's Medium 1
//! of its review at Sprint 223).
//!
//! The contract is not in the key: `case_runs` reports a class under `w2m.guest_state.fault_context` and `class_judge`
//! under `x86.semantics.<class>`, two contracts over one instruction's result, and FCOS's last-digit ST0 is one defect
//! in both. Each report keeps its contract; a judge of another kind of unit names units of its own (reviewer-3's Low 1).

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
pub fn first_frame(record: &Path) -> Option<Value> {
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

/// A report's defect, from its first divergence: the unit before the first colon, and the field, the first colon-ended
/// segment after it that begins with the unit and a space ("FCOS: FCOS st0: …" from `case_runs`, "FCOS: managed_vm:
/// FCOS st0: …" from `class_judge`, both "FCOS st0"); a divergence with no such segment is its own field.
pub fn defect(first_divergence: &str) -> String {
    let unit = first_divergence.split(':').next().unwrap_or_default();
    first_divergence
        .split(": ")
        .skip(1)
        .find(|s| s.starts_with(&format!("{unit} ")))
        .map_or_else(|| first_divergence.to_string(), str::to_string)
}

/// `rebuild_ledger` (105-134) over the records under `runs` whose baseline names a `judge`: each defect a judge
/// reported, keyed by `defect`, with its reports and the record that resolved it (or null).
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
    // a discrepancy id -> its defect's key
    let mut of: std::collections::HashMap<String, String> = Default::default();
    for (_, root, judge) in started {
        let name = root.file_name().unwrap().to_string_lossy().to_string();
        for e in frames(&root) {
            let p = &e["payload"];
            let id = p["discrepancy_id"].as_str().unwrap_or_default().to_string();
            match e["kind"].as_str() {
                Some("DISCREPANCY_REPORTED") => {
                    let first = p["first_divergence"].as_str().unwrap_or_default();
                    let key = defect(first);
                    let entry = ledger.entry(key.clone()).or_insert_with(|| {
                        json!({
                            "unit": first.split(':').next().unwrap_or_default(),
                            "reports": [],
                            "resolved_in": null,
                            "resolving_ref": null,
                        })
                    });
                    // a defect reported again after a resolution is open again
                    entry["resolved_in"] = Value::Null;
                    entry["resolving_ref"] = Value::Null;
                    entry["reports"].as_array_mut().unwrap().push(json!({
                        "discrepancy_id": id,
                        "judge": judge,
                        "contract_id": p["contract_id"],
                        "reported_in": name,
                        "first_divergence": first,
                        "repair_path": null,
                        "resolved_in": null,
                        "resolving_ref": null,
                    }));
                    of.insert(id, key);
                }
                Some("DISCREPANCY_ROUTED") => {
                    if let Some(entry) = of.get(&id).and_then(|k| ledger.get_mut(k)) {
                        for r in entry["reports"].as_array_mut().unwrap() {
                            if r["discrepancy_id"] == id.as_str() {
                                r["repair_path"] = p["repair_path"].clone();
                            }
                        }
                    }
                }
                Some("DISCREPANCY_RESOLVED") => {
                    if let Some(entry) = of.get(&id).and_then(|k| ledger.get_mut(k)) {
                        let reports = entry["reports"].as_array_mut().unwrap();
                        for r in reports.iter_mut().filter(|r| r["discrepancy_id"] == id.as_str()) {
                            r["resolved_in"] = json!(name);
                            r["resolving_ref"] = p["resolving_ref"].clone();
                        }
                        // the defect closes with its last open report
                        if reports.iter().all(|r| !r["resolved_in"].is_null()) {
                            entry["resolved_in"] = json!(name);
                            entry["resolving_ref"] = p["resolving_ref"].clone();
                        }
                    }
                }
                _ => {}
            }
        }
    }
    ledger
}

/// `open_on` (137-139): the reports `judge` filed on `unit` that no record has resolved, by discrepancy id, sorted; the
/// ids that judge's next acceptance of the unit resolves, and no earlier report it already resolved.
pub fn open_on(ledger: &Map<String, Value>, judge: &str, unit: &str) -> Vec<String> {
    let mut out: Vec<String> = ledger
        .values()
        .filter(|v| v["unit"] == unit)
        .flat_map(|v| v["reports"].as_array().into_iter().flatten())
        .filter(|r| r["judge"] == judge && r["resolved_in"].is_null())
        .filter_map(|r| r["discrepancy_id"].as_str().map(str::to_string))
        .collect();
    out.sort();
    out
}

/// The open defects: the entries no record has resolved.
pub fn open(ledger: &Map<String, Value>) -> Vec<&String> {
    ledger
        .iter()
        .filter(|(_, v)| v["resolved_in"].is_null())
        .map(|(k, _)| k)
        .collect()
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
        assert_eq!(ledger.keys().collect::<Vec<_>>(), ["ADD CF"]);
        assert_eq!(open_on(&ledger, "case-runner", "ADD"), ["d1"]);
        assert_eq!(ledger["ADD CF"]["reports"][0]["repair_path"], "implementation_repair");
        // the same defect from a second judge, its executor named before the field, joins the entry (Sprint 223)
        record(
            &dir,
            "class-judge-1",
            1.7,
            Some("class-judge"),
            &[event(
                "DISCREPANCY_REPORTED",
                json!({"discrepancy_id": "c1", "contract_id": "x86.semantics.add",
                       "first_divergence": "ADD: managed_vm: ADD CF: 3 cases"}),
            )],
        );
        let ledger = rebuild(&dir);
        assert_eq!(ledger.keys().collect::<Vec<_>>(), ["ADD CF"]);
        assert_eq!(ledger["ADD CF"]["reports"].as_array().unwrap().len(), 2);
        assert_eq!(open(&ledger), ["ADD CF"]);
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
        // one judge's resolution resolves its own report, and the class judge's report keeps the defect open (Sprint
        // 223, reviewer-3's Medium 1 of its review at it)
        let ledger = rebuild(&dir);
        assert_eq!(ledger["ADD CF"]["reports"][0]["resolved_in"], "case-runner-2");
        assert!(open_on(&ledger, "case-runner", "ADD").is_empty());
        assert_eq!(open_on(&ledger, "class-judge", "ADD"), ["c1"]);
        assert_eq!(open(&ledger), ["ADD CF"], "a defect another judge reported stays open");
        record(
            &dir,
            "class-judge-2",
            3.0,
            Some("class-judge"),
            &[event(
                "DISCREPANCY_RESOLVED",
                json!({"discrepancy_id": "c1", "resolving_ref": "class-judge:ADD@def"}),
            )],
        );
        let ledger = rebuild(&dir);
        assert_eq!(ledger["ADD CF"]["resolved_in"], "class-judge-2");
        assert_eq!(ledger["ADD CF"]["resolving_ref"], "class-judge:ADD@def");
        assert!(
            open(&ledger).is_empty(),
            "the last open report's resolution closes the defect"
        );
        assert!(text(&ledger).starts_with("{\n \"ADD CF\": {\n  \"reports\""));
        fs::remove_dir_all(&dir).unwrap();
    }

    /// A report's defect: the unit and its field, whichever judge wrote it.
    #[test]
    fn a_defect_is_the_unit_and_its_field() {
        assert_eq!(defect("FCOS: FCOS st0: 10 cases; run 1: case 2"), "FCOS st0");
        assert_eq!(defect("FCOS: managed_vm: FCOS FSW: 4 cases; run 1: case 2"), "FCOS FSW");
        assert_eq!(defect("MOV: no"), "MOV: no");
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
