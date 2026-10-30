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

use serde_json::{Map, Value};

const KNOWN: &str = "store/family_runs/known-2026-10-29.tsv";
const HELD: &str = "store/family_runs/sweep-held-2026-10-29.tsv";

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

type Row = Map<String, Value>;

/// Removes `keys` from the rows whose `s` is one of `names` (every row when `names` is empty).
fn drop_keys(rows: &mut [Row], names: &[&str], keys: &[&str]) {
    for r in rows.iter_mut() {
        if names.is_empty() || names.contains(&name(r)) {
            for k in keys {
                r.remove(*k);
            }
        }
    }
}

fn name(r: &Row) -> &str {
    r.get("s").and_then(Value::as_str).unwrap_or("")
}

fn int(r: &Row, k: &str) -> Result<i64, String> {
    r.get(k)
        .and_then(Value::as_i64)
        .ok_or_else(|| format!("row {}: no integer {k}", name(r)))
}

/// A family's row rule, by the name its lookup row gives (column 8, less " excluded <n>"): the frozen derivation of
/// `scripts/family_run.py` at the tag frozen-python-2026-10-29, ported as written there, its lines cited (Sprint 230;
/// `git show frozen-python-2026-10-29:scripts/family_run.py`). Exclusions change nothing here: an excluded row is not
/// among its contract's rows, and only those are compared. An unknown name is an error, never a pass.
fn derive(family: &str, rule: &str, printed: Vec<Row>) -> Result<Vec<Row>, String> {
    let mut rows = printed;
    match rule {
        "raw" => {}
        // 1841-1844: `w_len` from four file rows, `ret` from the first three
        "module_names_ansi_rows" => {
            drop_keys(
                &mut rows,
                &["file.image", "file.exact", "file.short", "file.one"],
                &["w_len"],
            );
            drop_keys(&mut rows, &["file.image", "file.exact", "file.short"], &["ret"]);
        }
        // 143-153: a row with a CPUID bit becomes whether the answer equals it
        "process_query_rows" => {
            for r in &mut rows {
                if r.contains_key("cpuid") {
                    let value = r.get("value").ok_or_else(|| format!("row {}: no value", name(r)))?;
                    let equals = Value::Bool(value == &r["cpuid"]);
                    *r = Row::from_iter([("s".to_string(), r["s"].clone()), ("equals_cpuid".to_string(), equals)]);
                }
            }
        }
        // 156-160: the child's title keeps no length
        "console_rows" => drop_keys(&mut rows, &["startup_info.child.title"], &["length"]),
        // 163-165: each best-fit row keyed by its UTF-16 unit
        "code_page_rows" => {
            for r in &mut rows {
                if name(r) == "bestfit" {
                    let u = int(r, "u")?;
                    r.insert("s".into(), Value::String(format!("bestfit.{u:04x}")));
                }
            }
        }
        // 218-228: apphelp.dll out of a loader list, with its count
        "loader_rows" => {
            for r in &mut rows {
                let Some(Value::Array(names)) = r.get("names") else {
                    continue;
                };
                if !names.iter().any(|n| n == "apphelp.dll") {
                    continue;
                }
                let kept: Vec<Value> = names.iter().filter(|n| *n != "apphelp.dll").cloned().collect();
                let (count, bases) = (int(r, "count")?, int(r, "bases_equal_module_handles")?);
                r.insert("names".into(), Value::Array(kept));
                r.insert("count".into(), (count - 1).into());
                r.insert("bases_equal_module_handles".into(), (bases - 1).into());
            }
        }
        // 231-239: a released region keeps that it covers the reservation
        "virtual_memory_rows" => {
            for r in &mut rows {
                if name(r) == "query.released" && r.contains_key("region_size") {
                    let size = int(r, "region_size")?;
                    r.remove("region_size");
                    r.insert("region_covers_reservation".into(), Value::Bool(size >= 0x1FFF_F000));
                }
            }
        }
        // 242-246: `version_low` kept as whether it is set
        "version_rows" => {
            for r in &mut rows {
                if r.contains_key("version_low") {
                    let low = int(r, "version_low")?;
                    r.remove("version_low");
                    r.insert("version_low_set".into(), Value::Bool(low != 0));
                }
            }
        }
        // 249-273: the metrics the taskbar moves as relations to the work area; index 93 without a value; the
        // adapter's GUID as a mark
        "display_rows" => {
            let info = rows
                .iter()
                .find(|r| name(r) == "info.wide" && r.contains_key("work_bottom"));
            let work = match info {
                Some(r) => int(r, "work_bottom")? - int(r, "work_top")?,
                None => 0,
            };
            for r in &mut rows {
                match name(r) {
                    "metric.17" => {
                        let v = int(r, "value")?;
                        r.insert("value".into(), (v - (work - 23)).into());
                    }
                    "metric.62" => {
                        let v = int(r, "value")?;
                        r.insert("value".into(), (v - (work + 16)).into());
                    }
                    "metric.93" => {
                        r.remove("value");
                    }
                    _ => {}
                }
                if r.contains_key("work_bottom") {
                    let monitor = match r.get("monitor_bottom") {
                        Some(_) => int(r, "monitor_bottom")?,
                        None => 768,
                    };
                    let below = int(r, "work_bottom")? < monitor;
                    r.insert("work_bottom".into(), Value::Bool(below));
                }
                if name(r) == "devices.adapter_0"
                    && let Some(Value::String(key)) = r.get("key")
                {
                    let key = guid_marked(key);
                    r.insert("key".into(), Value::String(key));
                }
            }
        }
        // 276-281: the child's command line as "child <id>"
        "processes_rows" => {
            for r in &mut rows {
                if name(r) != "child.command_line" {
                    continue;
                }
                let tail = r
                    .get("tail")
                    .and_then(Value::as_str)
                    .ok_or("row child.command_line: no tail")?;
                let digits = tail.strip_prefix("child ").unwrap_or("");
                if !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()) {
                    r.insert("tail".into(), Value::String("child <id>".into()));
                }
            }
        }
        // 284-308: ProcessParameters' rows as relations where a run decides the value
        "parameters_rows" => {
            for r in &mut rows {
                let s = name(r).to_string();
                if s.ends_with(".header") {
                    let (maximum, length) = (r.remove("maximum_length"), r.remove("length"));
                    let (Some(maximum), Some(length)) = (maximum, length) else {
                        return Err(format!("row {s}: no maximum_length or length"));
                    };
                    r.insert("length_is_maximum".into(), Value::Bool(maximum == length));
                }
                if s.ends_with(".block") {
                    r.remove("region_size");
                }
                if s == "parent.window" {
                    r.retain(|k, _| ["s", "flags_are_startup", "show_is_startup"].contains(&k.as_str()));
                }
                if s == "parent.strings" {
                    r.remove("title_is_module");
                }
                if s == "parent.environment" {
                    r.remove("version");
                }
                if s == "parent.standard_handles" {
                    r.retain(|k, _| !k.ends_with("_null"));
                }
            }
        }
        // 399-408: the child's family keeps only error_mode.child_inherited; the parent's every row
        "time_rows.derive" => {
            if family == "time_and_error_mode_child" {
                rows.retain(|r| name(r) == "error_mode.child_inherited");
            }
        }
        // 443-458: the caches' sizes and the OS's module entry dropped
        "topology_rows" => {
            drop_keys(&mut rows, &["machine"], &["l1d_size", "l2_size", "l3_size"]);
            drop_keys(&mut rows, &["ex.query.65535"], &["modules", "size_other"]);
        }
        // 461-473: the sequence as the count of pushes it stands for, the step push.1's
        "slist_rows" => {
            let push = rows
                .iter()
                .find(|r| name(r) == "push.1")
                .ok_or("no row push.1, whose sequence is the step")?;
            let step = match int(push, "sequence")? {
                0 => 1,
                n => n,
            };
            for r in &mut rows {
                for (from, to) in [("sequence", "pushes"), ("sequence_after", "pushes_after")] {
                    if r.contains_key(from) {
                        let v = int(r, from)?;
                        r.remove(from);
                        r.insert(to.into(), v.div_euclid(step).into());
                    }
                }
            }
        }
        // 476-490: no file part for "." and ".."; the drive after \\?\ written <d>
        "paths_rows" => {
            drop_keys(&mut rows, &["full.dot", "full.dotdot_alone"], &["part_from_end"]);
            for r in &mut rows {
                for k in ["in", "out"] {
                    let Some(Value::String(v)) = r.get(k) else { continue };
                    let chars: Vec<char> = v.chars().collect();
                    if v.starts_with("\\\\?\\") && chars.get(5) == Some(&':') {
                        let marked = format!("\\\\?\\<d>{}", chars[5..].iter().collect::<String>());
                        r.insert(k.into(), Value::String(marked));
                    }
                }
            }
        }
        // 834-837: the steps between two blocks dropped
        "no_steps" => drop_keys(&mut rows, &[], &["step_forward", "step_back", "b_after_a"]),
        // 1733-1734
        "temp_path_rows" => drop_keys(&mut rows, &["temp.relative"], &["ret"]),
        // 1786-1787: `rid` as whether it is SECURITY_MANDATORY_LOW_RID
        "integrity_level_rows" => {
            for r in &mut rows {
                if r.contains_key("rid") {
                    let rid = int(r, "rid")?;
                    r.remove("rid");
                    r.insert("low".into(), Value::Bool(rid == 0x1000));
                }
            }
        }
        // 1929-1931
        "thread_stack_rows" => drop_keys(&mut rows, &["query.sp", "query.base_minus_one"], &["base_address"]),
        // 1966-1969: 1 or 2 committed pages of a new thread's TEB, timing, as "1-2"
        "thread_stacks_rows" => {
            for r in &mut rows {
                let pages = r.get("committed_pages").and_then(Value::as_i64);
                if ["default.teb", "reserve.teb"].contains(&name(r)) && matches!(pages, Some(1 | 2)) {
                    r.insert("committed_pages".into(), Value::String("1-2".into()));
                }
            }
        }
        // 2043-2046
        "heap_query_rows" => drop_keys(
            &mut rows,
            &["query.small", "query.after_small_region"],
            &["allocation_base_below_block"],
        ),
        // 2082-2083: each sweep row named by its size
        "files_more_rows" => {
            for r in &mut rows {
                if name(r) == "standard.sweep" {
                    let size = int(r, "size")?;
                    r.insert("s".into(), Value::String(format!("standard.sweep.{size}")));
                }
            }
        }
        // 2179-2180
        "file_mapping_rows" => drop_keys(&mut rows, &["query.after_view", "query.granule_end"], &["region_size"]),
        // 2217: the dump rows dropped
        "format_message_rows" => rows.retain(|r| name(r) != "dump"),
        // 168-183 (Sprint 231): the start's rows, unprefixed; the other start's dropped, the rest kept
        "child_rows.derive" => {
            let prefix = match family {
                "environment_set" => "set.",
                "environment_empty" => "empty.",
                other => return Err(format!("child_rows: no start for family {other}")),
            };
            let mut kept = Vec::new();
            for mut r in rows {
                let s = name(&r).to_string();
                if let Some(rest) = s.strip_prefix(prefix) {
                    r.insert("s".into(), Value::String(rest.to_string()));
                    kept.push(r);
                } else if !s.starts_with("set.") && !s.starts_with("empty.") {
                    kept.push(r);
                }
            }
            rows = kept;
        }
        // 338-390 (Sprint 231): the windows' places as relations to the desktop
        "windows_rows" => rows = windows_rows(rows)?,
        other => return Err(format!("row rule {other} is not ported")),
    }
    Ok(rows)
}

/// Python's truth of a JSON value: false, 0, "", null, [] and {} are false.
fn truthy(v: Option<&Value>) -> bool {
    match v {
        None | Some(Value::Null) => false,
        Some(Value::Bool(b)) => *b,
        Some(Value::Number(n)) => n.as_f64() != Some(0.0),
        Some(Value::String(s)) => !s.is_empty(),
        Some(Value::Array(a)) => !a.is_empty(),
        Some(Value::Object(o)) => !o.is_empty(),
    }
}

/// `without_message_0x60` (311-335): each drain's rows (queue.<drain>.<NN> and their "after" rows) without message
/// 0x60, renumbered; an empty queue's `noremove_err` kept only for the first drain.
fn without_message_0x60(printed: Vec<Row>) -> Result<Vec<Row>, String> {
    let mut dropped = std::collections::BTreeSet::new();
    let mut kept: BTreeMap<String, usize> = BTreeMap::new();
    let mut out = Vec::new();
    for mut r in printed {
        let s = name(&r).to_string();
        let parts: Vec<&str> = s.split('.').collect();
        if parts[0] != "queue" || parts.len() != 3 {
            out.push(r);
            continue;
        }
        let drain = parts[1];
        if truthy(r.get("after")) {
            if dropped.contains(&s) {
                continue;
            }
            let n = kept
                .get(&s)
                .ok_or(format!("row {s}: an after row with no row before it"))?;
            r.insert("s".into(), Value::String(format!("queue.{drain}.{n:02}")));
            out.push(r);
            continue;
        }
        if r.get("msg").and_then(Value::as_i64) == Some(0x60) {
            dropped.insert(s);
            continue;
        }
        let n = kept
            .keys()
            .filter(|k| k.starts_with(&format!("queue.{drain}.")))
            .count();
        kept.insert(s.clone(), n);
        if drain != "first" && !truthy(r.get("noremove")) {
            r.remove("noremove_err");
        }
        r.insert("s".into(), Value::String(format!("queue.{drain}.{n:02}")));
        out.push(r);
    }
    Ok(out)
}

/// `windows_rows` (338-390): message 0x60 out of the drains; each window's geometry from its own origin, positions as
/// relations (x = y, on the cascade of 26), the default-sized windows' heights as their excess over 3/4 of the work
/// area less 21, and the work area's bottom as lying above the screen's.
fn windows_rows(printed: Vec<Row>) -> Result<Vec<Row>, String> {
    let printed = without_message_0x60(printed)?;
    let default_height = match printed.iter().find(|r| name(r) == "screen.work_area") {
        // Python's `(bottom - top) * 3 // 4 - 21`: `//` floors, as `div_euclid` does by a positive divisor
        Some(w) => ((int(w, "rect_bottom")? - int(w, "rect_top")?) * 3).div_euclid(4) - 21,
        None => 0,
    };
    let windows = ["batch", "play", "game", "other"];
    let mut origin: BTreeMap<String, (i64, i64)> = BTreeMap::new();
    for r in &printed {
        let s = name(r);
        if ["msg.batch.1", "msg.play.1", "msg.game.1", "msg.other.1"].contains(&s) && r.contains_key("x") {
            origin.insert(s.split('.').nth(1).unwrap().to_string(), (int(r, "x")?, int(r, "y")?));
        }
    }
    let mut out = Vec::new();
    for mut r in printed {
        if truthy(r.get("after")) {
            r.remove("after");
            let s = format!("{}.after", name(&r));
            r.insert("s".into(), Value::String(s));
        }
        let s = name(&r).to_string();
        if s == "screen.work_area" {
            let mut w = Row::new();
            w.insert("s".into(), Value::String(s));
            w.insert("left".into(), r.get("rect_left").cloned().unwrap_or(Value::Null));
            w.insert("top".into(), r.get("rect_top").cloned().unwrap_or(Value::Null));
            w.insert("right".into(), r.get("rect_right").cloned().unwrap_or(Value::Null));
            w.insert("bottom_above_768".into(), Value::Bool(int(&r, "rect_bottom")? < 768));
            out.push(w);
            continue;
        }
        let which = windows
            .iter()
            .find(|w| s.starts_with(&format!("msg.{w}.")) || s.starts_with(&format!("{w}.")));
        let Some((x, y)) = which.and_then(|w| origin.get(*w)).copied() else {
            out.push(r);
            continue;
        };
        let tall = which != Some(&"play");
        if r.contains_key("x") {
            r.retain(|k, v| k != "x" && k != "y" && !v.is_null());
            r.insert("x_equals_y".into(), Value::Bool(x == y));
            r.insert("x_in_cascade".into(), Value::Bool(x > 0 && x % 26 == 0));
        }
        for prefix in ["in", "out", "rect"] {
            if !r.contains_key(&format!("{prefix}_left")) {
                continue;
            }
            let mut take = |k: &str| -> Result<i64, String> {
                let v = int(&r, &format!("{prefix}_{k}"))?;
                r.remove(&format!("{prefix}_{k}"));
                Ok(v)
            };
            let (left, top, right, bottom) = (take("left")?, take("top")?, take("right")?, take("bottom")?);
            let (left0, top0) = if s.ends_with(".client_rect") { (0, 0) } else { (x, y) };
            let height = bottom - top0;
            r.insert(format!("{prefix}_left"), (left - left0).into());
            r.insert(format!("{prefix}_top"), (top - top0).into());
            r.insert(format!("{prefix}_right"), (right - left0).into());
            r.insert(
                format!("{prefix}_bottom"),
                (if tall { height - default_height } else { height }).into(),
            );
        }
        out.push(r);
    }
    Ok(out)
}

/// `re.sub(r"\{[0-9A-F-]{36}\}", "{GUID}", key)` (display_rows, 271-272).
fn guid_marked(key: &str) -> String {
    let b = key.as_bytes();
    let mut out = String::new();
    let mut i = 0;
    while i < key.len() {
        let is_guid = b[i] == b'{'
            && b.get(i + 37) == Some(&b'}')
            && b[i + 1..i + 37]
                .iter()
                .all(|c| c.is_ascii_digit() || (b'A'..=b'F').contains(c) || *c == b'-');
        if is_guid {
            out.push_str("{GUID}");
            i += 38;
        } else {
            let c = key[i..].chars().next().unwrap();
            out.push(c);
            i += c.len_utf8();
        }
    }
    out
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
    // w2m<1 to 4 hex digits>.tmp: GetTempFileNameW writes its 16-bit number in hex without leading zeros
    // (tools/guest_runtime/file.c 475-479; Sprint 227 met `w2m417.tmp`, which a mask of exactly four missed)
    let mut steady = String::new();
    let mut s = out.as_str();
    while let Some(i) = s.find("w2m") {
        steady.push_str(&s[..i]);
        let tail = &s[i + 3..];
        let digits = tail.bytes().take(4).take_while(u8::is_ascii_hexdigit).count();
        if digits >= 1 && tail[digits..].starts_with(".tmp") {
            steady.push_str("w2m<tmp>.tmp");
            s = &tail[digits + 4..];
        } else {
            steady.push_str("w2m");
            s = tail;
        }
    }
    steady.push_str(s);
    steady
}

/// Each ported rule on rows the family runs do not print, the expected rows worked by hand from the frozen code's lines
/// (Sprint 230): no loader run prints apphelp.dll, so only this test holds `loader_rows`' drop.
#[test]
fn the_row_rules_derive_as_the_frozen_code_does() {
    let rows = |v: Value| -> Vec<Row> {
        v.as_array()
            .unwrap()
            .iter()
            .map(|r| r.as_object().unwrap().clone())
            .collect()
    };
    let derived = |family: &str, rule: &str, v: Value| Value::from(derive(family, rule, rows(v)).unwrap());
    assert_eq!(
        derived(
            "loader",
            "loader_rows",
            serde_json::json!([{"s": "ldr.load_order", "names": ["probe.exe", "apphelp.dll", "ntdll.dll"], "count": 3,
                "bases_equal_module_handles": 3}])
        ),
        serde_json::json!([{"s": "ldr.load_order", "names": ["probe.exe", "ntdll.dll"], "count": 2,
            "bases_equal_module_handles": 2}])
    );
    assert_eq!(
        derived(
            "code_pages",
            "code_page_rows",
            serde_json::json!([{"s": "bestfit", "u": 255, "b": 63}])
        ),
        serde_json::json!([{"s": "bestfit.00ff", "u": 255, "b": 63}])
    );
    assert_eq!(
        derived(
            "interlocked_lists",
            "slist_rows",
            serde_json::json!([{"s": "push.1", "sequence": 2}, {"s": "flush", "sequence": 6, "sequence_after": 8}])
        ),
        serde_json::json!([{"s": "push.1", "pushes": 1}, {"s": "flush", "pushes": 3, "pushes_after": 4}])
    );
    assert_eq!(
        derived(
            "display",
            "display_rows",
            serde_json::json!([
                {"s": "info.wide", "work_top": 0, "work_bottom": 720, "monitor_bottom": 768},
                {"s": "metric.17", "value": 697}, {"s": "metric.62", "value": 736}, {"s": "metric.93", "value": 5852},
                {"s": "devices.adapter_0", "key": "x\\{4D36E968-E325-11CE-BFC1-08002BE10318}\\0000"}
            ])
        ),
        serde_json::json!([
            {"s": "info.wide", "work_top": 0, "work_bottom": true, "monitor_bottom": 768},
            {"s": "metric.17", "value": 0}, {"s": "metric.62", "value": 0}, {"s": "metric.93"},
            {"s": "devices.adapter_0", "key": "x\\{GUID}\\0000"}
        ])
    );
    assert_eq!(
        derived(
            "paths",
            "paths_rows",
            serde_json::json!([{"s": "unc", "in": "\\\\?\\D:\\a", "out": "D:\\a"}])
        ),
        serde_json::json!([{"s": "unc", "in": "\\\\?\\<d>:\\a", "out": "D:\\a"}])
    );
    assert_eq!(
        derived(
            "processes",
            "processes_rows",
            serde_json::json!([{"s": "child.command_line", "tail": "child 4242"}])
        ),
        serde_json::json!([{"s": "child.command_line", "tail": "child <id>"}])
    );
    assert_eq!(
        derived(
            "time_and_error_mode_child",
            "time_rows.derive",
            serde_json::json!([{"s": "time_zone", "bias": 0}, {"s": "error_mode.child_inherited", "mode": 3}])
        ),
        serde_json::json!([{"s": "error_mode.child_inherited", "mode": 3}])
    );
    // Sprint 231: child_rows keeps its start's rows unprefixed; windows_rows renumbers the drains without message
    // 0x60 and places a window from its origin (default height 720 * 3 // 4 - 21 = 519)
    assert_eq!(
        derived(
            "environment_set",
            "child_rows.derive",
            serde_json::json!([{"s": "set.count", "n": 5}, {"s": "empty.count", "n": 0}, {"s": "start", "ok": 1}])
        ),
        serde_json::json!([{"s": "count", "n": 5}, {"s": "start", "ok": 1}])
    );
    assert_eq!(
        derived(
            "windows",
            "windows_rows",
            serde_json::json!([
                {"s": "screen.work_area", "rect_left": 0, "rect_top": 0, "rect_right": 1024, "rect_bottom": 720},
                {"s": "msg.batch.1", "x": 52, "y": 52, "rect_left": 52, "rect_top": 52, "rect_right": 852,
                    "rect_bottom": 571},
                {"s": "queue.first.00", "msg": 96},
                {"s": "queue.first.00", "after": true},
                {"s": "queue.first.01", "msg": 15, "noremove_err": 0},
                {"s": "queue.first.01", "after": true, "x": 1},
                {"s": "queue.later.00", "msg": 15, "noremove_err": 5}
            ])
        ),
        serde_json::json!([
            {"s": "screen.work_area", "left": 0, "top": 0, "right": 1024, "bottom_above_768": true},
            {"s": "msg.batch.1", "x_equals_y": true, "x_in_cascade": true, "rect_left": 0, "rect_top": 0,
                "rect_right": 800, "rect_bottom": 0},
            {"s": "queue.first.00", "msg": 15, "noremove_err": 0},
            {"s": "queue.first.00.after", "x": 1},
            {"s": "queue.later.00", "msg": 15}
        ])
    );
    let unknown = derive("threads", "thread_rows", Vec::new()).unwrap_err();
    assert_eq!(unknown, "row rule thread_rows is not ported");
}

/// The extract the start options are read from, and the tag that holds the families they were read from.
const EXTRACT: &str = "store/frozen_python/family-run-2026-10-29.tsv";
const TAG: &str = "frozen-python-2026-10-29";

/// One start option's value at the tag: the lines of `scripts/family_run.py` that show it, each with text it holds,
/// and the Python expression of the value, which one of those texts contains (Sprint 231; reviewer-3's Medium 1 of
/// its review at 231: the extract's seven start rows were entered by hand). `start` is the factory's argument where
/// the family is built by one (`environment_family`, `time_family`), else "".
struct Shown {
    family: &'static str,
    key: &'static str,
    start: &'static str,
    lines: &'static [(usize, &'static str)],
    expr: &'static str,
}

const SHOWN: &[Shown] = &[
    Shown {
        family: "console",
        key: "gui",
        start: "",
        lines: &[(1362, "name=\"console\","), (1381, "gui=True")],
        expr: "True",
    },
    Shown {
        family: "console",
        key: "arguments",
        start: "",
        lines: &[(1362, "name=\"console\","), (1381, "arguments=\"child\"")],
        expr: "\"child\"",
    },
    Shown {
        family: "console",
        key: "output_file",
        start: "",
        lines: &[
            (1362, "name=\"console\","),
            (1381, "output_file=\"w2m-console-child.jsonl\""),
        ],
        expr: "\"w2m-console-child.jsonl\"",
    },
    Shown {
        family: "local_free",
        key: "arguments",
        start: "",
        lines: &[(1591, "name=\"local_free\","), (1598, "arguments=\"safe\"")],
        expr: "\"safe\"",
    },
    Shown {
        family: "windows",
        key: "environment",
        start: "",
        lines: &[
            (2566, "name=\"windows\","),
            (
                2580,
                "environment=(\"TEMP=C:\\\\Users\\\\w2m\\\\AppData\\\\Local\\\\Temp\", \"TMP=C:\\\\Users\\\\w2m\\\\AppData\\\\Local\\\\Temp\")",
            ),
        ],
        expr: "(\"TEMP=C:\\\\Users\\\\w2m\\\\AppData\\\\Local\\\\Temp\", \"TMP=C:\\\\Users\\\\w2m\\\\AppData\\\\Local\\\\Temp\")",
    },
    Shown {
        family: "processes",
        key: "instruction_clock",
        start: "",
        lines: &[(2667, "name=\"processes\","), (2681, "instruction_clock=True")],
        expr: "True",
    },
    Shown {
        family: "time_and_error_mode_child",
        key: "gui",
        start: "child",
        lines: &[
            (1359, "\"time_and_error_mode_child\": time_family(\"child\","),
            (412, "child = start == \"child\""),
            (439, "gui=child"),
        ],
        expr: "child",
    },
    Shown {
        family: "time_and_error_mode_child",
        key: "arguments",
        start: "child",
        lines: &[
            (1359, "\"time_and_error_mode_child\": time_family(\"child\","),
            (439, "arguments=\"child\" if child"),
        ],
        expr: "\"child\"",
    },
    Shown {
        family: "time_and_error_mode_child",
        key: "output_file",
        start: "child",
        lines: &[
            (1359, "\"time_and_error_mode_child\": time_family(\"child\","),
            (439, "output_file=\"w2m-error-mode-child.jsonl\" if child"),
        ],
        expr: "\"w2m-error-mode-child.jsonl\"",
    },
    Shown {
        family: "environment_set",
        key: "gui",
        start: "set",
        lines: &[
            (543, "\"environment_set\": environment_family("),
            (544, "\"set\","),
            (214, "gui=True"),
        ],
        expr: "True",
    },
    Shown {
        family: "environment_set",
        key: "arguments",
        start: "set",
        lines: &[
            (543, "\"environment_set\": environment_family("),
            (544, "\"set\","),
            (214, "arguments=f\"child-{start}\""),
        ],
        expr: "f\"child-{start}\"",
    },
    Shown {
        family: "environment_set",
        key: "output_file",
        start: "set",
        lines: &[
            (543, "\"environment_set\": environment_family("),
            (544, "\"set\","),
            (215, "output_file=f\"w2m-environment-{start}.jsonl\""),
        ],
        expr: "f\"w2m-environment-{start}.jsonl\"",
    },
    Shown {
        family: "environment_set",
        key: "environment",
        start: "set",
        lines: &[
            (543, "\"environment_set\": environment_family("),
            (546, "ENVIRONMENT_SET,"),
            (
                200,
                "ENVIRONMENT_SET = (\"ZED=last\", \"alpha=1\", \"=C:=C:\\\\w2m\", \"SteamAppId=480\", \"Path=C:\\\\Windows\\\\system32\")",
            ),
        ],
        expr: "(\"ZED=last\", \"alpha=1\", \"=C:=C:\\\\w2m\", \"SteamAppId=480\", \"Path=C:\\\\Windows\\\\system32\")",
    },
    Shown {
        family: "environment_empty",
        key: "gui",
        start: "empty",
        lines: &[
            (554, "\"environment_empty\": environment_family("),
            (555, "\"empty\","),
            (214, "gui=True"),
        ],
        expr: "True",
    },
    Shown {
        family: "environment_empty",
        key: "arguments",
        start: "empty",
        lines: &[
            (554, "\"environment_empty\": environment_family("),
            (555, "\"empty\","),
            (214, "arguments=f\"child-{start}\""),
        ],
        expr: "f\"child-{start}\"",
    },
    Shown {
        family: "environment_empty",
        key: "output_file",
        start: "empty",
        lines: &[
            (554, "\"environment_empty\": environment_family("),
            (555, "\"empty\","),
            (215, "output_file=f\"w2m-environment-{start}.jsonl\""),
        ],
        expr: "f\"w2m-environment-{start}.jsonl\"",
    },
    Shown {
        family: "environment_empty",
        key: "environment",
        start: "empty",
        lines: &[(554, "\"environment_empty\": environment_family("), (556, "(),")],
        expr: "()",
    },
];

/// The Python string literal at the start of `text` (a quote, then `\\` and `\"` escapes to the closing quote), and
/// the text after it.
fn python_literal(text: &str) -> Result<(String, &str), String> {
    let body = text.strip_prefix('"').ok_or(format!("not a string literal: {text}"))?;
    let (mut out, mut chars) = (String::new(), body.char_indices());
    while let Some((i, c)) = chars.next() {
        match c {
            '"' => return Ok((out, &body[i + 1..])),
            '\\' => out.push(chars.next().ok_or("a literal ends in a backslash")?.1),
            c => out.push(c),
        }
    }
    Err(format!("an unclosed string literal: {text}"))
}

/// The value of the few Python expressions a start option is written as: `True`, `child` (the time family's
/// `start == "child"`), a string literal, an f-string over `{start}`, a tuple of string literals.
fn python_value(expr: &str, start: &str) -> Result<Value, String> {
    match expr {
        "True" => Ok(Value::Bool(true)),
        "child" => Ok(Value::Bool(start == "child")),
        _ if expr.starts_with("f\"") => Ok(Value::String(python_literal(&expr[1..])?.0.replace("{start}", start))),
        _ if expr.starts_with('"') => Ok(Value::String(python_literal(expr)?.0)),
        _ if expr.starts_with('(') => {
            let mut items = Vec::new();
            let mut rest = expr[1..].trim_start();
            while rest.starts_with('"') {
                let (item, after) = python_literal(rest)?;
                items.push(Value::String(item));
                rest = after.trim_start().trim_start_matches(',').trim_start();
            }
            if rest != ")" {
                return Err(format!("not a tuple of string literals: {expr}"));
            }
            Ok(Value::Array(items))
        }
        _ => Err(format!("an expression this reader does not evaluate: {expr}")),
    }
}

/// The extract's start options are the tag's: every value of every family's JSON object is shown at the lines `SHOWN`
/// cites, read from `scripts/family_run.py` at the tag, and every shown value is in the extract (Sprint 231).
#[test]
fn the_extract_start_options_are_the_tags() {
    let root = root();
    let out = std::process::Command::new("git")
        .arg("-C")
        .arg(&root)
        .args(["show", &format!("{TAG}:scripts/family_run.py")])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "git show: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let source = String::from_utf8(out.stdout).unwrap();
    let lines: Vec<&str> = source.lines().collect();
    let mut stored: BTreeMap<(String, String), Value> = BTreeMap::new();
    for line in fs::read_to_string(root.join(EXTRACT)).unwrap().lines() {
        if let ["family", family, _, start] = line.split('\t').collect::<Vec<_>>()[..]
            && start != "-"
        {
            let options: Value = serde_json::from_str(start).unwrap();
            for (k, v) in options.as_object().unwrap() {
                stored.insert((family.to_string(), k.clone()), v.clone());
            }
        }
    }
    let mut wrong = Vec::new();
    for s in SHOWN {
        for (n, text) in s.lines {
            if !lines.get(n - 1).is_some_and(|l| l.contains(text)) {
                wrong.push(format!(
                    "{} {}: family_run.py {n} at {TAG} does not hold {text:?}",
                    s.family, s.key
                ));
            }
        }
        if !s.lines.iter().any(|(_, text)| text.contains(s.expr)) {
            wrong.push(format!(
                "{} {}: {:?} is in none of its cited texts",
                s.family, s.key, s.expr
            ));
        }
        let want = python_value(s.expr, s.start).unwrap_or_else(|e| panic!("{} {}: {e}", s.family, s.key));
        match stored.remove(&(s.family.to_string(), s.key.to_string())) {
            Some(got) if got == want => {}
            Some(got) => wrong.push(format!("{} {}: {EXTRACT} has {got}, {TAG} {want}", s.family, s.key)),
            None => wrong.push(format!(
                "{} {}: {TAG} shows {want}, and {EXTRACT} has none",
                s.family, s.key
            )),
        }
    }
    wrong.extend(
        stored
            .keys()
            .map(|(family, key)| format!("{family} {key}: in {EXTRACT}, shown at no line of {TAG}")),
    );
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
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
    // fewer than four digits, as the runtime writes a small number (Sprint 227), and five, which is not its name
    assert_eq!(steady("x <drive>/Windows/w2m417.tmp"), "x <drive>/Windows/w2m<tmp>.tmp");
    assert_eq!(steady("w2mA.tmp"), "w2m<tmp>.tmp");
    assert_eq!(steady("w2m12345.tmp"), "w2m12345.tmp");
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
    let (mut seen, mut wrong) = (BTreeMap::new(), Vec::new());
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
        // from Sprint 231 every run starts as its family starts (column 7, given to `w2m-vm` as --start), so every run
        // that ends is judged
        let state = if stop != "ExitProcess { code: 0 }" {
            format!("stop: {}", steady(&stop))
        } else {
            let (id, version) = contract.split_once('@').unwrap();
            let text = fs::read_to_string(root.join(format!("contracts/{id}/{version}.json"))).unwrap();
            let contract: Value = serde_json::from_str(&text).unwrap();
            let printed: Vec<Row> = fs::read_to_string(base.with_extension("txt"))
                .unwrap()
                .lines()
                .filter_map(|l| match serde_json::from_str::<Value>(l) {
                    Ok(Value::Object(r)) => Some(r),
                    _ => None,
                })
                .collect();
            // the rule's name, less " excluded <n>": an excluded row is not among the contract's rows
            let name = rule.split(" excluded ").next().unwrap();
            let derived = derive(family, name, printed).unwrap_or_else(|e| panic!("{family} run {run}: {e}"));
            let got: BTreeMap<String, Value> = derived
                .into_iter()
                .filter_map(|r| Some((r.get("s")?.as_str()?.to_string(), Value::Object(r))))
                .collect();
            let differs = contract["acceptance"]["rows"]
                .as_array()
                .unwrap()
                .iter()
                .find(|want| got.get(want["s"].as_str().unwrap()) != Some(want));
            if let Some(want) = differs {
                format!("ended; row {} differs from {id}@{version}", want["s"].as_str().unwrap())
            } else {
                if held.contains_key(&key) {
                    wrong.push(format!("{family} run {run} conforms, and {HELD} still holds it"));
                }
                conformed += 1;
                families.entry(family.to_string()).or_insert(true);
                seen.insert(key, String::new());
                continue;
            }
        };
        *families.entry(family.to_string()).or_insert(false) = false;
        if held.get(&key) != Some(&state) {
            wrong.push(format!("{family}\t{run}\t{state}\t(which {HELD} does not hold)"));
        }
        seen.insert(key, state);
    }
    wrong.extend(
        held.keys()
            .filter(|key| !seen.contains_key(*key))
            .map(|key| format!("{HELD} holds {key:?}, which the lookup does not name")),
    );
    // every run's difference at once, so one gates run shows them all (Sprint 230)
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
    println!(
        "{conformed} of {} family runs conform, {} of {} families in every run; the rest held by what they wait on",
        seen.len(),
        families.values().filter(|v| **v).count(),
        families.len()
    );
}
