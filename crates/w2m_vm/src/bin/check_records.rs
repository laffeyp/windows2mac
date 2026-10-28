// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! `check_records` (Sprint 169): every record checked against the vocabulary's rules, `scripts/check_records.py`
//! (Sprint 109) rule for rule, so a vocabulary can change a rule in Rust (Decision 2026-10-28: P22 changes T4, which the
//! Python holds in code). It reads each record's application events (`w2m_vm::ledger::frames`, without `substrate.*`
//! kinds) and checks T1, T2, T3, T4 and T7 (`temporal_invariants`), S1 to S13 where their tags occur
//! (`state_transitions`; a rule whose payload match is prose is checked for order alone, and its count says so), and E1
//! and E6 (`evidence_constraints`). T5, T6 and S14 are the guest trace's, enforced at ingest.
//!
//! The rules come from `signals/rust-current.json`'s vocabulary; its three rule sections equal 0.9's, which the Python
//! reads (compared 2026-10-28). Prints, per rule, the records it applied to, its violations and what it could not check,
//! as the Python prints them. With `--since YYYY-MM-DD`, exits 1 on a violation in a record made that day or later (a
//! record's time is its directory's nanosecond suffix); older records are reported, never rewritten.
//!
//! Usage: `check_records [--since 2026-10-17] [--show N] [record ...]`

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use serde_json::Value;
use w2m_vm::ledger::frames;

/// b1ad0cfb: class and file runs report a reduction and real hashes (check_records.py 41).
const FIXED_IN_112: u128 = 1792271815 * 1_000_000_000;
const PLACEHOLDER: &str = "sha256:0000000000000000000000000000000000000000000000000000000000000000";

/// Violations known, with their cause, which the gate reports and does not hold (check_records.py 37-48): (rule,
/// record name prefix, where the cause and its fix are recorded, the time in ns of the commit that fixed it or none).
fn known() -> Vec<(&'static str, &'static str, &'static str, Option<u128>)> {
    let e1 = "Sprint 112: runs reported zero program and state hashes before b1ad0cfb";
    vec![
        (
            "S5",
            "family-",
            "BLACKBOARD Deferred 2026-10-17 (Sprint 112): no reducer cuts a compiled probe, so a family run reports \
             with no REDUCTION_COMPLETED",
            None,
        ),
        (
            "S5",
            "class-",
            "Sprint 112: class runs reported with no reduction before b1ad0cfb",
            Some(FIXED_IN_112),
        ),
        ("E1", "class-", e1, Some(FIXED_IN_112)),
        ("E1", "family-", e1, Some(FIXED_IN_112)),
        ("E1", "file-", e1, Some(FIXED_IN_112)),
    ]
}

/// A count per rule.
type Counts = BTreeMap<String, u64>;

fn bump(c: &mut Counts, rule: &str) {
    *c.entry(rule.to_string()).or_default() += 1;
}

/// Python's `str()` of a JSON value as `json.loads` gives it.
fn py_str(v: Option<&Value>) -> String {
    match v {
        None | Some(Value::Null) => "None".into(),
        Some(Value::Bool(b)) => if *b { "True" } else { "False" }.into(),
        Some(Value::String(s)) => s.clone(),
        Some(other) => py_repr(other),
    }
}

/// Python's `repr()` of a JSON value as `json.loads` gives it (the forms the rules' messages print).
fn py_repr(v: &Value) -> String {
    match v {
        Value::Null => "None".into(),
        Value::Bool(b) => if *b { "True" } else { "False" }.into(),
        Value::Number(n) => n.to_string(),
        Value::String(s) => {
            let quote = if s.contains('\'') && !s.contains('"') {
                '"'
            } else {
                '\''
            };
            let mut out = String::from(quote);
            for ch in s.chars() {
                match ch {
                    '\\' => out.push_str("\\\\"),
                    '\n' => out.push_str("\\n"),
                    '\r' => out.push_str("\\r"),
                    '\t' => out.push_str("\\t"),
                    c if c == quote => {
                        out.push('\\');
                        out.push(c);
                    }
                    c => out.push(c),
                }
            }
            out.push(quote);
            out
        }
        Value::Array(a) => format!("[{}]", a.iter().map(py_repr).collect::<Vec<_>>().join(", ")),
        Value::Object(o) => format!(
            "{{{}}}",
            o.iter()
                .map(|(k, x)| format!("{}: {}", py_repr(&Value::String(k.clone())), py_repr(x)))
                .collect::<Vec<_>>()
                .join(", ")
        ),
    }
}

/// Python's `repr()` of a float: the shortest form that reads back, with `.0` on a whole number.
fn py_float(x: f64) -> String {
    let s = format!("{x:?}");
    if s.contains(['.', 'e', 'i', 'N']) {
        s
    } else {
        format!("{s}.0")
    }
}

/// Python's `repr()` of the tuple `key(e, fields)` builds.
fn py_tuple(values: &[Option<&Value>]) -> String {
    let items: Vec<String> = values.iter().map(|v| v.map_or("None".into(), py_repr)).collect();
    if items.len() == 1 {
        format!("({},)", items[0])
    } else {
        format!("({})", items.join(", "))
    }
}

struct Event {
    kind: String,
    seq: i64,
    t: f64,
    payload: Value,
}

impl Event {
    fn get(&self, f: &str) -> Option<&Value> {
        self.payload.get(f).filter(|v| !v.is_null())
    }

    /// `key(e, fields)`: `payload.get(f)` for each field, a missing one as None.
    fn key(&self, fields: &[String]) -> Vec<Option<&Value>> {
        fields.iter().map(|f| self.payload.get(f.as_str())).collect()
    }
}

/// A rule's payload match: the fields both events share, and the `field=value` filters on the `from` event.
type PayloadMatch = (Vec<String>, Vec<(String, String)>);

/// `plain_fields` (57-62): a rule's payload match as the fields both events share and `field=value` filters on the
/// `from` event; None when any entry is prose.
fn plain_fields(matches: &[String]) -> Option<PayloadMatch> {
    let word = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_lowercase() || b == b'_');
    let ok = matches.iter().all(|m| match m.split_once('=') {
        Some((a, b)) => word(a) && word(b),
        None => word(m),
    });
    if !ok {
        return None;
    }
    let fields = matches.iter().filter(|m| !m.contains('=')).cloned().collect();
    let wanted = matches
        .iter()
        .filter_map(|m| m.split_once('='))
        .map(|(a, b)| (a.to_string(), b.to_string()))
        .collect();
    Some((fields, wanted))
}

fn strings(v: &Value, key: &str) -> Vec<String> {
    v.get(key)
        .and_then(Value::as_array)
        .map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect())
        .unwrap_or_default()
}

/// `Checker` (69-181): the rules, read once from the vocabulary.
struct Checker {
    states: Vec<Value>,
    sha_fields: BTreeMap<String, Vec<String>>,
}

impl Checker {
    fn new(vocab: &Value) -> Checker {
        let states = vocab["state_transitions"]["rules"]
            .as_array()
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter(|r| !r["from_tag"].as_str().unwrap_or("").starts_with("GUEST_"))
            .collect();
        let mut sha_fields = BTreeMap::new();
        for t in vocab["tags"].as_array().into_iter().flatten() {
            // in the order the tag declares its fields, as the Python's `types.items()` keeps the file's order (a
            // serde_json map sorts its keys)
            let fields = strings(t, "payload")
                .into_iter()
                .chain(strings(t, "optional_payload"))
                .filter(|f| t["types"][f.as_str()].as_str() == Some("sha256"))
                .collect();
            sha_fields.insert(t["name"].as_str().unwrap_or("").to_string(), fields);
        }
        Checker { states, sha_fields }
    }

    fn check(&self, events: &[Event], found: &mut Vec<(String, String)>, applied: &mut Counts, unchecked: &mut Counts) {
        let Some((first, last)) = events.first().zip(events.last()) else {
            return;
        };
        bump(applied, "T1");
        if first.kind != "SESSION_INIT" {
            found.push(("T1".into(), format!("first event {}", first.kind)));
        }
        bump(applied, "T2");
        if last.kind != "SESSION_COMPLETED" {
            found.push(("T2".into(), format!("last event {}", last.kind)));
        }
        t3(events, found, applied, unchecked);
        t4(events, found, applied);
        after(
            "T7",
            events,
            "CONTRACT_REVISED",
            &["DEPENDENTS_INVALIDATED"],
            &["contract_id".into(), "to_version".into()],
            found,
            applied,
        );
        for rule in &self.states {
            state(rule, events, found, applied, unchecked);
        }
        for e in events {
            for f in self.sha_fields.get(&e.kind).into_iter().flatten() {
                let Some(v) = e.get(f) else { continue };
                bump(applied, "E1");
                match v.as_str() {
                    Some(s) if is_sha256(s) => {
                        if s == PLACEHOLDER {
                            found.push(("E1".into(), format!("{}.{f} is the zero placeholder", e.kind)));
                        }
                    }
                    _ => found.push(("E1".into(), format!("{}.{f} = {}", e.kind, py_repr(v)))),
                }
            }
            if e.kind == "COMPARISON_COMPLETED" && e.get("verdict").and_then(Value::as_str) == Some("discrepancy") {
                bump(applied, "E6");
                let empty = match e.get("first_divergence") {
                    None => true,
                    Some(Value::String(s)) => s.is_empty(),
                    Some(Value::Array(a)) => a.is_empty(),
                    Some(Value::Object(o)) => o.is_empty(),
                    Some(Value::Bool(b)) => !b,
                    Some(Value::Number(n)) => n.as_f64() == Some(0.0),
                    Some(Value::Null) => true,
                };
                if empty {
                    found.push((
                        "E6".into(),
                        format!(
                            "{}: a discrepancy with no first_divergence",
                            py_str(e.payload.get("comparison_id"))
                        ),
                    ));
                }
            }
        }
    }
}

/// `^sha256:[0-9a-f]{64}$` (Python's `$` also matches before a final newline).
fn is_sha256(s: &str) -> bool {
    let s = s.strip_suffix('\n').unwrap_or(s);
    s.strip_prefix("sha256:")
        .is_some_and(|h| h.len() == 64 && h.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)))
}

/// `_t3` (112-128).
fn t3(events: &[Event], found: &mut Vec<(String, String)>, applied: &mut Counts, unchecked: &mut Counts) {
    let ends: Vec<&Event> = events
        .iter()
        .filter(|e| e.kind == "EXECUTION_COMPLETED" || e.kind == "EXECUTION_FAILED")
        .collect();
    for s in events.iter().filter(|e| e.kind == "EXECUTION_STARTED") {
        let xid = s.payload.get("execution_id");
        let closing: Vec<&&Event> = ends
            .iter()
            .filter(|e| e.payload.get("execution_id") == xid && e.seq > s.seq)
            .collect();
        bump(applied, "T3");
        let name = py_str(xid);
        if closing.len() != 1 {
            found.push(("T3".into(), format!("{name}: {} completions", closing.len())));
            continue;
        }
        let Some(limit) = s.get("timeout_seconds") else {
            bump(unchecked, "T3");
            continue;
        };
        // `limit + (kill_grace_seconds or 0)`, printed as Python prints the sum: an int when both are ints
        let grace = s.get("kill_grace_seconds");
        let (within, shown) = match (limit.as_i64(), grace.map_or(Some(0), Value::as_i64)) {
            (Some(l), Some(g)) => ((l + g) as f64, (l + g).to_string()),
            _ => {
                let w = limit.as_f64().unwrap_or(0.0) + grace.and_then(Value::as_f64).unwrap_or(0.0);
                (w, py_float(w))
            }
        };
        let took = closing[0].t - s.t;
        if took > within {
            found.push(("T3".into(), format!("{name}: closed after {took:.1} s, past {shown} s")));
        }
    }
}

/// `_t4` (130-138).
fn t4(events: &[Event], found: &mut Vec<(String, String)>, applied: &mut Counts) {
    let end = events.iter().find(|e| e.kind == "SESSION_COMPLETED").map(|e| e.seq);
    for c in events.iter().filter(|e| e.kind == "EXECUTION_COMPLETED") {
        bump(applied, "T4");
        let sha = c.payload.get("trace_sha256");
        let follow = events
            .iter()
            .filter(|e| {
                (e.kind == "TRACE_INGESTED" || e.kind == "TRACE_REJECTED")
                    && e.seq > c.seq
                    && e.payload.get("trace_sha256") == sha
                    && end.is_none_or(|end| e.seq < end)
            })
            .count();
        if follow != 1 {
            found.push((
                "T4".into(),
                format!(
                    "{}: {follow} ingests of {}",
                    py_str(c.payload.get("execution_id")),
                    py_str(sha)
                ),
            ));
        }
    }
}

/// `_after` (140-146): every `first` is followed by one of `then` with the same `fields`.
fn after(
    rule: &str,
    events: &[Event],
    first: &str,
    then: &[&str],
    fields: &[String],
    found: &mut Vec<(String, String)>,
    applied: &mut Counts,
) {
    for a in events.iter().filter(|e| e.kind == first) {
        bump(applied, rule);
        if !events
            .iter()
            .any(|e| then.contains(&e.kind.as_str()) && e.seq > a.seq && e.key(fields) == a.key(fields))
        {
            let mut names: Vec<&str> = then.to_vec();
            names.sort();
            found.push((
                rule.into(),
                format!(
                    "{first} {} with no {} after it",
                    py_tuple(&a.key(fields)),
                    names.join("/")
                ),
            ));
        }
    }
}

/// `_state` (148-181): S1 to S13 by kind.
fn state(
    rule: &Value,
    events: &[Event],
    found: &mut Vec<(String, String)>,
    applied: &mut Counts,
    unchecked: &mut Counts,
) {
    let rid = rule["id"].as_str().unwrap_or("");
    let src = rule["from_tag"].as_str().unwrap_or("");
    let parsed = plain_fields(&strings(rule, "payload_match"));
    let to_allowed = strings(rule, "to_tags_allowed");
    let to_forbidden = strings(rule, "to_tags_forbidden");
    if !events
        .iter()
        .any(|e| to_allowed.contains(&e.kind) || to_forbidden.contains(&e.kind))
    {
        return;
    }
    bump(applied, rid);
    if parsed.is_none() {
        bump(unchecked, rid);
    }
    let (fields, wanted) = parsed.map_or((None, Vec::new()), |(f, w)| (Some(f), w));
    let source = |e: &Event| e.kind == src && wanted.iter().all(|(f, v)| py_str(e.payload.get(f.as_str())) == *v);
    let same = |a: &Event, b: &Event| fields.as_ref().is_none_or(|f| a.key(f) == b.key(f));
    match rule["kind"].as_str().unwrap_or("") {
        "pairing_ordering" | "allowed_set" => {
            for b in events.iter().filter(|e| to_allowed.contains(&e.kind)) {
                if !events.iter().any(|a| source(a) && a.seq < b.seq && same(a, b)) {
                    found.push((rid.into(), format!("{} with no {src} before it", b.kind)));
                }
            }
        }
        "forbidden_after" => {
            for a in events.iter().filter(|e| source(e)) {
                for b in events {
                    if to_forbidden.contains(&b.kind) && b.seq > a.seq && same(a, b) {
                        found.push((rid.into(), format!("{} after {src}", b.kind)));
                    }
                }
            }
        }
        "forced_next" => {
            for a in events.iter().filter(|e| source(e)) {
                let next = events.iter().find(|b| b.seq > a.seq && b.kind != src && same(a, b));
                if next.is_none_or(|b| !to_allowed.contains(&b.kind)) {
                    found.push((
                        rid.into(),
                        format!("{src} followed by {}", next.map_or("nothing", |b| b.kind.as_str())),
                    ));
                }
            }
        }
        _ => {}
    }
}

/// `application_events` (184-185). A sequence that skips a number is refused, as substrate's `read_record` raises
/// RecordGapError ("a sealed segment was lost or truncated"; Sprint 169's first plant found the Rust reader did not).
fn application_events(record: &Path) -> Result<Vec<Event>, String> {
    let frames = frames(record);
    for (expected, f) in frames.iter().enumerate() {
        let got = f.get("seq").and_then(Value::as_i64);
        if got != Some(expected as i64) {
            return Err(format!(
                "{}: seq gap: expected {expected}, got {}",
                record.display(),
                got.map_or("none".into(), |g| g.to_string())
            ));
        }
    }
    Ok(frames
        .into_iter()
        .filter_map(|f| {
            let kind = f.get("kind")?.as_str()?.to_string();
            if kind.starts_with("substrate.") {
                return None;
            }
            Some(Event {
                kind,
                seq: f.get("seq")?.as_i64()?,
                t: f.get("t").and_then(Value::as_f64).unwrap_or(0.0),
                payload: f.get("payload").cloned().unwrap_or(Value::Null),
            })
        })
        .collect())
}

/// `made_ns` (188-190): a 19-digit suffix after the name's last `-`.
fn made_ns(record: &Path) -> Option<u128> {
    let name = record.file_name()?.to_str()?;
    let (_, digits) = name.rsplit_once('-')?;
    (digits.len() == 19 && digits.bytes().all(|b| b.is_ascii_digit())).then(|| digits.parse().ok())?
}

/// Days since 1970-01-01 of a `YYYY-MM-DD` date (the civil calendar, as `datetime.date`).
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

fn run() -> Result<bool, String> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut args = std::env::args().skip(1);
    let (mut since, mut show, mut records): (Option<i64>, usize, Vec<PathBuf>) = (None, 3, Vec::new());
    while let Some(a) = args.next() {
        match a.as_str() {
            "--since" => {
                let d = args.next().ok_or("--since needs a date")?;
                let p: Vec<i64> = d
                    .split('-')
                    .map(|x| x.parse().map_err(|_| format!("--since {d}")))
                    .collect::<Result<_, _>>()?;
                let [y, m, dd] = p[..] else {
                    return Err(format!("--since {d}"));
                };
                since = Some(days_from_civil(y, m, dd));
            }
            "--show" => {
                show = args
                    .next()
                    .ok_or("--show needs a number")?
                    .parse()
                    .map_err(|_| "--show")?
            }
            _ => records.push(PathBuf::from(a)),
        }
    }
    let current: Value = serde_json::from_str(
        &fs::read_to_string(root.join("signals/rust-current.json")).map_err(|e| format!("rust-current.json: {e}"))?,
    )
    .map_err(|e| e.to_string())?;
    let version = current["version"]
        .as_str()
        .ok_or("rust-current.json names no version")?;
    let vocab: Value = serde_json::from_str(
        &fs::read_to_string(root.join(format!("signals/{version}.json"))).map_err(|e| format!("{version}: {e}"))?,
    )
    .map_err(|e| e.to_string())?;
    let checker = Checker::new(&vocab);
    if records.is_empty() {
        let mut dirs: Vec<PathBuf> = fs::read_dir(root.join("store/runs"))
            .map_err(|e| e.to_string())?
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.is_dir())
            .collect();
        dirs.sort();
        records = dirs;
    }
    let known = known();
    let (mut violations, mut applied, mut unchecked) = (Counts::new(), Counts::new(), Counts::new());
    let mut examples: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut held: Vec<String> = Vec::new();
    let mut known_counts: BTreeMap<&str, u64> = BTreeMap::new();
    for r in &records {
        let mut found = Vec::new();
        checker.check(&application_events(r)?, &mut found, &mut applied, &mut unchecked);
        let name = r
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        for (rule, problem) in found {
            bump(&mut violations, &rule);
            examples
                .entry(rule.clone())
                .or_default()
                .push(format!("{name}: {problem}"));
            let made = made_ns(r);
            let day = made.map(|ns| (ns / 1_000_000_000 / 86_400) as i64);
            let (Some(since), Some(day)) = (since, day) else {
                continue;
            };
            if day < since {
                continue;
            }
            let cause = known.iter().find(|(k, prefix, _, fixed)| {
                *k == rule && name.starts_with(prefix) && fixed.is_none_or(|f| made.is_some_and(|m| m < f))
            });
            match cause {
                Some((_, _, c, _)) => *known_counts.entry(c).or_default() += 1,
                None => held.push(format!("{rule} {name}: {problem}")),
            }
        }
    }
    let mut rules: Vec<&String> = applied.keys().collect();
    rules.sort_by_key(|k| (k.chars().next(), k[1..].parse::<u64>().unwrap_or(0)));
    let n = |c: &Counts, k: &str| c.get(k).copied().unwrap_or(0);
    for rule in rules {
        println!(
            "{rule:4} applied {:>7}  violations {:>6}  unchecked {:>6}",
            n(&applied, rule),
            n(&violations, rule),
            n(&unchecked, rule)
        );
        for ex in examples.get(rule).into_iter().flatten().take(show) {
            println!("       {ex}");
        }
    }
    let total: u64 = violations.values().sum();
    let tail = since.map_or(String::new(), |s| {
        let (y, m, d) = civil_from_days(s);
        format!(
            "; in records made on or after {y:04}-{m:02}-{d:02}: {} held, {} known",
            held.len(),
            known_counts.values().sum::<u64>()
        )
    });
    println!("{} records; {total} violations{tail}", records.len());
    for (cause, n) in &known_counts {
        println!("  known, not held ({n}): {cause}");
    }
    for h in held.iter().take(20) {
        println!("  held: {h}");
    }
    Ok(held.is_empty())
}

/// The civil date of a day count since 1970-01-01.
fn civil_from_days(z: i64) -> (i64, i64, i64) {
    let z = z + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (yoe + era * 400 + i64::from(m <= 2), m, d)
}

fn main() -> ExitCode {
    match run() {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(e) => {
            eprintln!("check_records: {e}");
            ExitCode::from(2)
        }
    }
}
