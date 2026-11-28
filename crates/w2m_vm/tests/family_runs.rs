// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! The families judged on the Rust host as the gate counts them (census row 6; plan step 4, "68 of 68 families";
//! Sprint 221). The population is every run `store/runs/family-runs.json` judged, 273 in 68 families, read from the
//! lookup `store/family_runs/known-2026-10-29.tsv` (`family_known`; from Sprint 369 after them the families that joined
//! on the Rust host, `store/family_runs/joined-2026-11-27.tsv`): each run's binary, the contract and version its
//! family record judged it by (from Sprint 378 the version `store/family_runs/retaken-2026-11-27.tsv` re-takes on the
//! laptop, for a frozen family it names), its beside DLL, and its row rule. `scripts/gates.sh` runs each row in `w2m-vm` as
//! probe.exe with its beside DLL and an instruction budget, writing build/family_runs/<family>-<run>.txt and .stop.
//!
//! A run that ends at ExitProcess under a row rule this test carries (raw, or `module_names_ansi_rows`) is held to
//! its contract version's rows: each row printed exactly, by key `s` (a printed row the contract lacks is not
//! compared). Every other run is held in `HELD` by what it waits on: "ended; row rule <rule> not ported; rows sha256
//! <hex>", the hash of its printed rows, so an ended run's output cannot change unseen while its rule waits (reviewer-3's
//! Medium 1 of its project review, 2026-10-29), or the stop it makes ("stop: <stop>", a temporary drive's path and file
//! name written as fixed marks). A run that conforms must not be held, and a held run that conforms or moves fails the
//! test until the card that moved it rewrites the file.
//!
//! From Sprint 351 (census G2) `scripts/gates.sh` runs each row again on two vCPUs, `w2m-vm --vcpus 2`, writing
//! build/family_runs/<family>-<run>-two-vcpus.txt and .stop, and each is held to its one-vCPU run's contract rows or
//! held entry; a run the host refuses on two vCPUs is held instead by name to the refusal, in
//! store/family_runs/two-vcpus-refused-2026-11-26.tsv: one that started a child process, to the refusal of the
//! run-loop design's card 4f, until Sprint 364. Until Sprint 360 a run whose family starts on the instruction clock was
//! held to `vcpus_allowed`'s refusal of that clock; from Sprint 360 the clock stays on one vCPU and such a start runs on
//! the wall clock above one (`w2m-vm`). From Sprint 364 (card 4f3 of
//! `docs/design-a-process-that-starts-another-2026-11-26.md`) a process that starts another runs on two vCPUs, its child
//! on one, what crosses between them answered by the request server, and the six runs the list held are held to their
//! one-vCPU runs' rows; the list holds none. From Sprint 365 (card 4f4) the child runs on two vCPUs too, its parent's
//! count.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

const KNOWN: &str = "store/family_runs/known-2026-10-29.tsv";
const HELD: &str = "store/family_runs/sweep-held-2026-10-29.tsv";
/// The families that joined on the Rust host after the frozen index (Sprint 369), which `family_known` reads after its
/// 273 runs.
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
const JOINED: &str = "store/family_runs/joined-2026-11-27.tsv";

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
        // 77-81 (Sprint 233): a row numbering one of several exceptions under one name gains ".exception<n>"
        "numbered_exceptions" => {
            for r in &mut rows {
                if let Some(n) = r.get("exception") {
                    let n = match n {
                        Value::String(s) => s.clone(),
                        other => other.to_string(),
                    };
                    let s = format!("{}.exception{n}", name(r));
                    r.insert("s".into(), Value::String(s));
                }
            }
        }
        // 1683 (Sprint 233): `err` dropped from every row that holds `hr`
        "known_folders_rows" => {
            for r in &mut rows {
                if r.contains_key("hr") {
                    r.remove("err");
                }
            }
        }
        // 338-390 (Sprint 231): the windows' places as relations to the desktop
        "windows_rows" => rows = windows_rows(rows)?,
        // Sprint 383, the Rust host's own rule for contracts/windows.windows.windows 0.4.0, the laptop's rows:
        // windows_rows, and class.register without after_first, whose second class's atom the laptop gave below the
        // first's in one execution of each of its runs and above it in the other (lap-20261013T231059422Z-136bec80-192
        // body-1 false, body-2 true; -193 body-1 true, body-2 false), as metric.93 keeps no value
        "windows_laptop_rows" => {
            rows = windows_rows(rows)?;
            drop_keys(&mut rows, &["class.register"], &["after_first"]);
        }
        other => return Err(format!("row rule {other} is not ported")),
    }
    Ok(rows)
}

/// Python's `==` over parsed JSON (Sprint 233): `True == 1`, `1 == 1.0`, lists and dicts element by element, where
/// `serde_json`'s equality holds a boolean apart from a number and an integer apart from a float.
fn py_eq(a: &Value, b: &Value) -> bool {
    let number = |v: &Value| match v {
        Value::Bool(b) => Some(f64::from(u8::from(*b))),
        Value::Number(n) => n.as_f64(),
        _ => None,
    };
    match (a, b) {
        (Value::Array(x), Value::Array(y)) => x.len() == y.len() && x.iter().zip(y).all(|(p, q)| py_eq(p, q)),
        (Value::Object(x), Value::Object(y)) => {
            x.len() == y.len() && x.iter().all(|(k, v)| y.get(k).is_some_and(|w| py_eq(v, w)))
        }
        (Value::Number(x), Value::Number(y)) if x.is_i64() && y.is_i64() => x.as_i64() == y.as_i64(),
        (Value::Number(x), Value::Number(y)) if x.is_u64() && y.is_u64() => x.as_u64() == y.as_u64(),
        _ => match (number(a), number(b)) {
            (Some(x), Some(y)) => x == y,
            _ => a == b,
        },
    }
}

/// `compare_exact_rows` (`src/w2m/compare.py` 168-203 at the tag): the contract's rows in order, the first row
/// missing, a different name in a row's place, a field unequal on either side's keys, or rows past the contract's,
/// as the divergence; None when the rows conform.
fn exact_rows(want: &[Value], got: &[Row]) -> Option<String> {
    let none = Value::Null;
    for (i, w) in want.iter().enumerate() {
        let w = w.as_object()?;
        let name = w.get("s").unwrap_or(&none);
        let Some(row) = got.get(i) else {
            return Some(format!(
                "row {name}: missing ({} rows printed, {} expected)",
                got.len(),
                want.len()
            ));
        };
        let theirs = row.get("s").unwrap_or(&none);
        if !py_eq(theirs, name) {
            return Some(format!("row {name}: target printed {theirs} in its place"));
        }
        let keys: std::collections::BTreeSet<&String> = w.keys().chain(row.keys()).collect();
        for k in keys {
            let (a, b) = (w.get(k).unwrap_or(&none), row.get(k).unwrap_or(&none));
            if !py_eq(a, b) {
                return Some(format!("row {name}: {k} reference {a}, target {b}"));
            }
        }
    }
    (got.len() > want.len()).then(|| format!("{} rows past the reference's {}", got.len() - want.len(), want.len()))
}

/// The frozen host's name for a Rust host's stop (Sprint 233): its `Unimplemented(...)` for a service's argument form
/// (`HostRequest op 4`, HOST_UNIMPLEMENTED) and for an import no service answers, which a family's `expected_stop`
/// names by prefix (`family_run.py` 3151 at the tag).
fn frozen_stop(stop: &str) -> Option<String> {
    if let Some(why) = stop.strip_prefix("HostRequest op 4: ") {
        return Some(format!("Unimplemented: {why}"));
    }
    stop.starts_with("import ").then(|| format!("Unimplemented: {stop}"))
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

/// The comparison is `compare_exact_rows` (Sprint 233): order, a missing row, a field on either side, an extra row,
/// Python's `==`; and the frozen host's names for the Rust host's two stops.
#[test]
fn the_comparison_is_the_frozen_comparison() {
    let rows = |v: Value| -> Vec<Row> {
        v.as_array()
            .unwrap()
            .iter()
            .map(|r| r.as_object().unwrap().clone())
            .collect()
    };
    let want = serde_json::json!([{"s": "a", "v": 1}, {"s": "b", "v": true}]);
    let want = want.as_array().unwrap();
    assert_eq!(
        exact_rows(
            want,
            &rows(serde_json::json!([{"s": "a", "v": 1.0}, {"s": "b", "v": 1}]))
        ),
        None
    );
    assert_eq!(
        exact_rows(want, &rows(serde_json::json!([{"s": "b", "v": 1}, {"s": "a", "v": 1}]))),
        Some("row \"a\": target printed \"b\" in its place".into())
    );
    assert_eq!(
        exact_rows(want, &rows(serde_json::json!([{"s": "a", "v": 1}]))),
        Some("row \"b\": missing (1 rows printed, 2 expected)".into())
    );
    assert_eq!(
        exact_rows(
            want,
            &rows(serde_json::json!([{"s": "a", "v": 1, "w": 2}, {"s": "b", "v": 1}]))
        ),
        Some("row \"a\": w reference null, target 2".into())
    );
    assert_eq!(
        exact_rows(
            want,
            &rows(serde_json::json!([{"s": "a", "v": 1}, {"s": "b", "v": 1}, {"s": "c"}]))
        ),
        Some("1 rows past the reference's 2".into())
    );
    assert_eq!(
        frozen_stop("HostRequest op 4: FlsFree argument form 0x1").as_deref(),
        Some("Unimplemented: FlsFree argument form 0x1")
    );
    assert_eq!(
        frozen_stop("import KERNEL32.dll!CreateProcessW").as_deref(),
        Some("Unimplemented: import KERNEL32.dll!CreateProcessW")
    );
    assert_eq!(frozen_stop("Handover: no thread ready"), None);
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
    // Sprint 383: the laptop's rule is windows_rows with class.register's after_first dropped, and that row's alone
    assert_eq!(
        derived(
            "windows",
            "windows_laptop_rows",
            serde_json::json!([
                {"s": "class.register_ex", "atom_in_range": true, "err": 57005},
                {"s": "class.register", "atom_in_range": true, "after_first": false, "err": 57005},
                {"s": "class.other", "after_first": true}
            ])
        ),
        serde_json::json!([
            {"s": "class.register_ex", "atom_in_range": true, "err": 57005},
            {"s": "class.register", "atom_in_range": true, "err": 57005},
            {"s": "class.other", "after_first": true}
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
    // the expected stops (Sprint 233): `Family.expected_stop`, the time parent's through `time_family`'s `child`
    Shown {
        family: "time_and_error_mode",
        key: "expected_stop",
        start: "parent",
        lines: &[
            (1355, "\"time_and_error_mode\": time_family(\"parent\","),
            (412, "child = start == \"child\""),
            (
                438,
                "expected_stop=None if child else \"Unimplemented: import KERNEL32.dll!CreateProcessW\"",
            ),
        ],
        expr: "\"Unimplemented: import KERNEL32.dll!CreateProcessW\"",
    },
    Shown {
        family: "thread_storage",
        key: "expected_stop",
        start: "",
        lines: &[
            (749, "name=\"thread_storage\","),
            (775, "expected_stop=\"Unimplemented: FlsFree\""),
        ],
        expr: "\"Unimplemented: FlsFree\"",
    },
    Shown {
        family: "output_debug_string",
        key: "expected_stop",
        start: "",
        lines: &[
            (1312, "name=\"output_debug_string\","),
            (
                1333,
                "expected_stop=\"Unimplemented: OutputDebugStringW argument form\"",
            ),
        ],
        expr: "\"Unimplemented: OutputDebugStringW argument form\"",
    },
    Shown {
        family: "known_folders",
        key: "expected_stop",
        start: "",
        lines: &[
            (1674, "name=\"known_folders\","),
            (1713, "expected_stop=\"Unimplemented: SHGetKnownFolderPath\""),
        ],
        expr: "\"Unimplemented: SHGetKnownFolderPath\"",
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

/// Each family's excluded rows at the tag (Sprint 233): its `name=` line, its `excluded={` line, and each row's name
/// quoted on its cited line, in the dictionary's order.
struct Excluded {
    family: &'static str,
    owner: (usize, &'static str),
    dict: usize,
    names: &'static [(usize, &'static str)],
}

const EXCLUDED: &[Excluded] = &[
    Excluded {
        family: "process_queries",
        owner: (560, "name=\"process_queries\","),
        dict: 593,
        names: &[(593, "ipfp.60"), (596, "ipfp.21")],
    },
    Excluded {
        family: "thread_storage",
        owner: (749, "name=\"thread_storage\","),
        dict: 772,
        names: &[(772, "fls.free.with_callback")],
    },
    Excluded {
        family: "heap_in_place",
        owner: (821, "name=\"heap_in_place\","),
        dict: 838,
        names: &[
            (838, "limit.fresh.800"),
            (840, "limit.fresh.1000"),
            (841, "limit.warm.1"),
        ],
    },
    Excluded {
        family: "output_debug_string",
        owner: (1312, "name=\"output_debug_string\","),
        dict: 1331,
        names: &[
            (1332, "w.short"),
            (1332, "w.short.exception0"),
            (1332, "w.short.exception1"),
        ],
    },
    Excluded {
        family: "processor_topology",
        owner: (1460, "name=\"processor_topology\","),
        dict: 1484,
        names: &[
            (1484, "ex.size_query.7"),
            (1485, "ex.short_buffer.7"),
            (1486, "ex.query.7"),
        ],
    },
    Excluded {
        family: "file_attributes",
        owner: (1497, "name=\"file_attributes\","),
        dict: 1518,
        names: &[
            (1518, "ex.file.trailing_backslash"),
            (1519, "plain.file.trailing_backslash"),
        ],
    },
    Excluded {
        family: "paths",
        owner: (1528, "name=\"paths\","),
        dict: 1550,
        names: &[(1550, "full.device_name")],
    },
    Excluded {
        family: "local_free",
        owner: (1591, "name=\"local_free\","),
        dict: 1611,
        names: &[(1611, "exit.safe"), (1612, "exit.argv_plus_2")],
    },
    Excluded {
        family: "known_folders",
        owner: (1674, "name=\"known_folders\","),
        dict: 1705,
        names: &[(1705, "unregistered"), (1708, "unregistered.create"), (1709, "proc")],
    },
    Excluded {
        family: "heap_query",
        owner: (2035, "name=\"heap_query\","),
        dict: 2057,
        names: &[(2057, "query.big")],
    },
];

/// The extract's family rows are the tag's: every start option, excluded row and expected stop is shown at the lines
/// `SHOWN` and `EXCLUDED` cite, read from `scripts/family_run.py` at the tag, and every shown value is in the extract
/// (Sprint 231; the excluded rows and expected stops from Sprint 233).
#[test]
fn the_extracts_family_rows_are_the_tags() {
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
        let ["family", family, _, start, ref rest @ ..] = line.split('\t').collect::<Vec<_>>()[..] else {
            continue;
        };
        if start != "-" {
            let options: Value = serde_json::from_str(start).unwrap();
            for (k, v) in options.as_object().unwrap() {
                stored.insert((family.to_string(), k.clone()), v.clone());
            }
        }
        if let [excluded, stop] = rest {
            if *excluded != "-" {
                let names = excluded.split(' ').map(|n| Value::String(n.to_string())).collect();
                stored.insert((family.to_string(), "excluded".into()), Value::Array(names));
            }
            if *stop != "-" {
                stored.insert(
                    (family.to_string(), "expected_stop".into()),
                    Value::String(stop.to_string()),
                );
            }
        }
    }
    let mut wrong = Vec::new();
    for e in EXCLUDED {
        let mut cited = vec![e.owner, (e.dict, "excluded={")];
        cited.extend(e.names.iter().map(|(n, name)| (*n, *name)));
        for (n, text) in cited {
            let quoted = if text == e.owner.1 || text == "excluded={" {
                text.to_string()
            } else {
                format!("\"{text}\"")
            };
            if !lines.get(n - 1).is_some_and(|l| l.contains(&quoted)) {
                wrong.push(format!(
                    "{} excluded: family_run.py {n} at {TAG} does not hold {quoted:?}",
                    e.family
                ));
            }
        }
        let want = Value::Array(
            e.names
                .iter()
                .map(|(_, name)| Value::String(name.to_string()))
                .collect(),
        );
        match stored.remove(&(e.family.to_string(), "excluded".to_string())) {
            Some(got) if got == want => {}
            Some(got) => wrong.push(format!("{} excluded: {EXTRACT} has {got}, {TAG} {want}", e.family)),
            None => wrong.push(format!(
                "{} excluded: {TAG} shows {want}, and {EXTRACT} has none",
                e.family
            )),
        }
    }
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
/// (Sprint 221): `family_known` run over the tree gives the stored rows, comments left out; from Sprint 369 the 273
/// frozen runs and after them the joined families' runs, none a frozen family.
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
    // the frozen index's runs, then the families that joined on the Rust host (Sprint 369), each once
    let joined: Vec<String> = fs::read_to_string(root.join(JOINED))
        .unwrap()
        .lines()
        .filter(|l| !l.starts_with('#') && !l.is_empty())
        .map(|l| l.split('\t').take(2).collect::<Vec<_>>().join("\t"))
        .collect();
    assert_eq!(
        stored.len(),
        273 + joined.len(),
        "the runs the family index judged, and the joined"
    );
    let tail: Vec<String> = stored[273..]
        .iter()
        .map(|l| l.split('\t').take(2).collect::<Vec<_>>().join("\t"))
        .collect();
    assert_eq!(
        tail, joined,
        "the joined runs follow the frozen index's 273, in {JOINED}'s order"
    );
    for family in joined.iter().map(|r| r.split('\t').next().unwrap()) {
        assert!(
            !stored[..273].iter().any(|l| l.split('\t').next() == Some(family)),
            "{family} joins on the Rust host and is a frozen family"
        );
    }
}

/// The frozen families held to a contract re-taken on the laptop (Sprint 378), which `family_known` reads.
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
const RETAKEN: &str = "store/family_runs/retaken-2026-11-27.tsv";

/// A stored laptop run, the third column of the fixtures' rows: `code_rewrite`'s own run in the lookup.
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
const FIXTURE_RUN: &str = "lap-20261014T044509652Z-627bbd99";

/// `family_known` over the tree with a re-taken table of `rows` under the stored table's comment, written to the test's
/// temporary directory as `retaken-<name>.tsv`: whether it exited 0, its output, its errors.
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
fn family_known_with(root: &Path, name: &str, rows: &str) -> (bool, String, String) {
    let comment: String = fs::read_to_string(root.join(RETAKEN))
        .unwrap()
        .lines()
        .filter(|l| l.starts_with('#'))
        .map(|l| format!("{l}\n"))
        .collect();
    let table = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("retaken-{name}.tsv"));
    fs::write(&table, format!("{comment}{rows}\n")).unwrap();
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_family_known"))
        .arg(root)
        .arg(&table)
        .output()
        .unwrap();
    (
        out.status.success(),
        String::from_utf8(out.stdout).unwrap(),
        String::from_utf8_lossy(&out.stderr).to_string(),
    )
}

/// A row of the re-taken table moves its family's contract column in `family_known`'s output and nothing else
/// (Sprint 378). The fixture holds `code_rewrite`, a frozen family of one run whose record names
/// windows.memory.code_rewrite@0.2.0, to that contract's 0.1.0, of row rule raw as 0.2.0 is, so its row's fourth column
/// alone moves and every other line is the stored lookup's. From Sprint 379 the fixture is the stored table's rows, which
/// name no `code_rewrite`, and the fixture's row after them.
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
#[test]
fn a_retaken_row_moves_its_familys_contract_and_nothing_else() {
    let root = root();
    let named: Vec<String> = fs::read_to_string(root.join(RETAKEN))
        .unwrap()
        .lines()
        .filter(|l| !l.starts_with('#') && !l.is_empty())
        .map(str::to_string)
        .collect();
    assert!(
        !named.iter().any(|r| r.starts_with("code_rewrite\t")),
        "{RETAKEN} names code_rewrite, the fixture's family"
    );
    let known = fs::read_to_string(root.join(KNOWN)).unwrap();
    let known: Vec<&str> = known.lines().filter(|l| !l.starts_with('#')).collect();
    let row = format!("code_rewrite\twindows.memory.code_rewrite@0.1.0\t{FIXTURE_RUN}");
    let rows: String = named.iter().map(|r| format!("{r}\n")).collect();
    let (ok, out, err) = family_known_with(&root, "moves", &format!("{rows}{row}"));
    assert!(ok, "family_known with the fixture: {err}");
    let derived: Vec<&str> = out.lines().collect();
    assert_eq!(
        derived.len(),
        known.len(),
        "the fixture's lookup against {KNOWN}, in lines"
    );
    let moved: Vec<String> = known
        .iter()
        .zip(&derived)
        .filter(|(was, now)| was != now)
        .map(|(was, now)| {
            let (a, b): (Vec<&str>, Vec<&str>) = (was.split('\t').collect(), now.split('\t').collect());
            let columns: Vec<usize> = (0..a.len().max(b.len()))
                .filter(|&i| a.get(i) != b.get(i))
                .map(|i| i + 1)
                .collect();
            format!("{} run {}: columns {columns:?} moved", a[0], a[1])
        })
        .collect();
    assert_eq!(
        moved,
        vec![format!("code_rewrite run {FIXTURE_RUN}: columns [4] moved")],
        "the rows the fixture moves"
    );
    let line = derived.iter().find(|l| l.starts_with("code_rewrite\t")).unwrap();
    assert_eq!(line.split('\t').nth(3), Some("windows.memory.code_rewrite@0.1.0"));
}

/// Each contract the re-taken table names holds its laptop runs' rows (Sprint 379): every body of every stored laptop run
/// its evidence cites, the table's run among them, after the contract's row rule, conforms to its rows by
/// `exact_rows`; its evidence cites no runner's run; and the contract version the family record names is unchanged
/// from HEAD's, a change of evidence being a new minor version (the Decision of 2026-10-07). Prints each family's runs
/// and bodies.
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
#[test]
fn each_retaken_contract_holds_its_laptop_runs_rows() {
    let root = root();
    let table = fs::read_to_string(root.join(RETAKEN)).unwrap();
    let rows: Vec<Vec<&str>> = table
        .lines()
        .filter(|l| !l.starts_with('#') && !l.is_empty())
        .map(|l| l.split('\t').collect())
        .collect();
    assert!(!rows.is_empty(), "{RETAKEN} names no family");
    let known = fs::read_to_string(root.join(KNOWN)).unwrap();
    let mut wrong = Vec::new();
    for c in &rows {
        let (family, contract, run) = (c[0], c[1], c[2]);
        let (id, version) = contract.split_once('@').unwrap();
        let file: Value =
            serde_json::from_str(&fs::read_to_string(root.join(format!("contracts/{id}/{version}.json"))).unwrap())
                .unwrap();
        // the frozen host's rule, or from Sprint 383 the Rust host's (family_known's `rule`)
        let rule = file["acceptance"]["derivation"]
            .as_str()
            .and_then(|d| {
                d.strip_prefix("scripts/family_run.py ")
                    .or_else(|| d.strip_prefix("crates/w2m_vm/tests/family_runs.rs "))
            })
            .and_then(|d| d.split(':').next())
            .unwrap_or("raw");
        let want = file["acceptance"]["rows"].as_array().unwrap();
        let evidence = file["evidence"].to_string();
        // the stored laptop runs the evidence names, by their ids
        let mut cited: Vec<&str> = evidence
            .split(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '-'))
            .filter(|t| t.starts_with("lap-") && root.join("store/probe-runs").join(t).join("body-1.jsonl").is_file())
            .collect();
        cited.sort_unstable();
        cited.dedup();
        if !cited.contains(&run) {
            wrong.push(format!(
                "{family}: {contract}'s evidence cites no {run}, the table's run"
            ));
        }
        let mut bodies = 0;
        for laptop in &cited {
            for i in 1.. {
                let Ok(text) = fs::read_to_string(root.join(format!("store/probe-runs/{laptop}/body-{i}.jsonl")))
                else {
                    break;
                };
                let printed: Vec<Row> = text
                    .lines()
                    .map(|l| match serde_json::from_str::<Value>(l) {
                        Ok(Value::Object(r)) => r,
                        _ => panic!("{laptop}/body-{i}.jsonl: a line that is no row: {l}"),
                    })
                    .collect();
                let derived = derive(family, rule, printed).unwrap();
                if let Some(divergence) = exact_rows(want, &derived) {
                    wrong.push(format!(
                        "{family}: {laptop}/body-{i}.jsonl against {contract}: {divergence}"
                    ));
                }
                bodies += 1;
            }
        }
        // the family's runner runs, from the lookup, cited nowhere in the evidence
        for l in known.lines().filter(|l| l.starts_with(&format!("{family}\t"))) {
            let runner = l.split('\t').nth(1).unwrap();
            if !runner.starts_with("lap-") && evidence.contains(runner) {
                wrong.push(format!(
                    "{family}: {contract}'s evidence cites the runner's run {runner}"
                ));
            }
        }
        // the contract's other versions, the record's among them, kept as HEAD holds them
        for earlier in fs::read_dir(root.join(format!("contracts/{id}")))
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().to_string())
            .filter(|n| n != &format!("{version}.json"))
        {
            let path = format!("contracts/{id}/{earlier}");
            let head = std::process::Command::new("git")
                .arg("-C")
                .arg(&root)
                .args(["show", &format!("HEAD:{path}")])
                .output()
                .unwrap();
            if !head.status.success() || head.stdout != fs::read(root.join(&path)).unwrap() {
                wrong.push(format!("{family}: {path} differs from HEAD's"));
            }
        }
        println!("{family}: {contract} ({rule}), laptop runs {cited:?}, {bodies} bodies conforming");
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

/// A re-taken row's fourth column reaches the family's start (Sprint 383): with `code_rewrite`, whose start is "-", the
/// fixture's `{"foreground_held":true}` is its whole start; and the stored table's windows row, whose family starts
/// with an environment, gives that environment and the foreground held together.
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
#[test]
fn a_retaken_rows_start_options_reach_the_familys_start() {
    let root = root();
    let named: String = fs::read_to_string(root.join(RETAKEN))
        .unwrap()
        .lines()
        .filter(|l| !l.starts_with('#') && !l.is_empty())
        .map(|l| format!("{l}\n"))
        .collect();
    let row = format!("code_rewrite\twindows.memory.code_rewrite@0.1.0\t{FIXTURE_RUN}\t{{\"foreground_held\":true}}");
    let (ok, out, err) = family_known_with(&root, "start", &format!("{named}{row}"));
    assert!(ok, "family_known with the fixture: {err}");
    let column = |family: &str, n: usize| -> String {
        let line = out
            .lines()
            .find(|l| l.starts_with(&format!("{family}\t")))
            .unwrap_or_else(|| panic!("no {family} line"));
        line.split('\t').nth(n).unwrap().to_string()
    };
    assert_eq!(column("code_rewrite", 6), r#"{"foreground_held":true}"#);
    assert_eq!(column("code_rewrite", 3), "windows.memory.code_rewrite@0.1.0");
    let windows: Value = serde_json::from_str(&column("windows", 6)).unwrap();
    assert_eq!(
        windows["foreground_held"],
        Value::Bool(true),
        "the windows family's start: {windows}"
    );
    assert!(
        windows["environment"].as_array().is_some_and(|e| !e.is_empty()),
        "the windows family's start keeps its environment: {windows}"
    );
}

/// A re-taken row is refused naming what it names wrongly (Sprint 378): a family the frozen index lacks (a joined
/// family among them, which moves by its own row in the joined table), a contract file that does not exist, another
/// contract than the record's, a run that is no stored laptop run, a family named twice; from Sprint 383 a fourth column
/// naming a key no start has, or not an object, and a row of five columns.
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
#[test]
fn a_retaken_row_is_refused_by_what_it_names() {
    let root = root();
    let ours = format!("code_rewrite\twindows.memory.code_rewrite@0.1.0\t{FIXTURE_RUN}");
    let cases = [
        (
            "family",
            format!("no_such_family\twindows.memory.code_rewrite@0.1.0\t{FIXTURE_RUN}"),
            "no_such_family is no family of the frozen index",
        ),
        (
            "joined",
            format!("named_sync_objects\twindows.sync.named_objects@0.1.0\t{FIXTURE_RUN}"),
            "named_sync_objects is no family of the frozen index",
        ),
        (
            "file",
            format!("code_rewrite\twindows.memory.code_rewrite@0.9.0\t{FIXTURE_RUN}"),
            "contracts/windows.memory.code_rewrite/0.9.0.json does not exist",
        ),
        (
            "contract",
            format!("code_rewrite\twindows.memory.virtual@0.1.0\t{FIXTURE_RUN}"),
            "code_rewrite re-taken under windows.memory.virtual, where its record names windows.memory.code_rewrite@0.2.0",
        ),
        (
            "run",
            "code_rewrite\twindows.memory.code_rewrite@0.1.0\t38173733146".to_string(),
            "code_rewrite's run 38173733146 is no stored laptop run",
        ),
        ("twice", format!("{ours}\n{ours}"), "code_rewrite named twice"),
        // Sprint 383: a fourth column, the start the laptop run adds, holds only the start's keys, as an object
        (
            "start_key",
            format!("{ours}\t{{\"no_such_option\":true}}"),
            "code_rewrite's start names no_such_option, which vm_inputs does not give",
        ),
        (
            "start_object",
            format!("{ours}\t[true]"),
            "code_rewrite's start [true] is not a JSON object",
        ),
        ("columns", format!("{ours}\t{{}}\textra"), "a row of 3 or 4 columns"),
    ];
    let mut wrong = Vec::new();
    for (name, rows, refusal) in cases {
        let (ok, _, err) = family_known_with(&root, name, &rows);
        if ok || !err.contains(refusal) {
            wrong.push(format!("{name}: exit 0 {ok}, stderr {err:?}, not naming {refusal:?}"));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
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

/// A family file's rows by (family, run), its third column the value (`HELD`, `REFUSED`).
fn by_run(root: &Path, file: &str) -> BTreeMap<(String, String), String> {
    fs::read_to_string(root.join(file))
        .unwrap()
        .lines()
        .filter(|l| !l.starts_with('#'))
        .filter_map(|l| {
            let mut c = l.splitn(3, '\t');
            Some(((c.next()?.into(), c.next()?.into()), c.next()?.into()))
        })
        .collect()
}

/// The first line of the stop `w2m-vm` wrote at `<base>.stop`, or None, after a panic naming the missing file when
/// `W2M_REQUIRE_CASE_RUNS=1` (the gates set it) or the reason on stderr, when there is none; None too for a stop not
/// made from the work tree's sources (Sprint 288: its stamp stands for its printed rows).
fn stop_at(root: &Path, base: &Path) -> Option<String> {
    let file = base.with_extension("stop");
    let Ok(stop) = fs::read_to_string(&file) else {
        let what = format!(
            "no {} (scripts/gates.sh writes it)",
            file.strip_prefix(root).unwrap_or(&file).display()
        );
        if std::env::var("W2M_REQUIRE_CASE_RUNS").as_deref() == Ok("1") {
            panic!("{what}, and W2M_REQUIRE_CASE_RUNS=1 requires it");
        }
        eprintln!("{what}: the families are not judged here");
        return None;
    };
    if !w2m_vm::built_from::fresh(root, &file) {
        return None;
    }
    Some(stop.lines().next().unwrap_or("").to_string())
}

/// How a run's outputs at `base` stand to Windows' rows: None when they conform, else the state it is held by.
/// From Sprint 231 every run starts as its family starts (column 7, given to `w2m-vm` as --start). From Sprint 233 a run
/// is judged as the frozen judge judges it (`family_run.py` 3141-3153 at the tag): one that ends, or stops on its
/// family's expected stop (column 10, a prefix of the frozen host's name for the stop), on its rows after its rule with
/// its excluded rows (column 9) dropped, by `exact_rows`; any other stop is held as a stop. `c` is the run's lookup row.
fn state_of(root: &Path, c: &[&str], base: &Path, stop: &str) -> Option<String> {
    let (family, run, contract, rule, excluded, expected) = (c[0], c[1], c[3], c[7], c[8], c[9]);
    let ended = stop == "ExitProcess { code: 0 }";
    let as_expected = expected != "-" && frozen_stop(stop).is_some_and(|frozen| frozen.starts_with(expected));
    if !ended && !as_expected {
        return Some(format!("stop: {}", steady(stop)));
    }
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
    let name = rule.split(" excluded ").next().unwrap();
    let mut derived = derive(family, name, printed).unwrap_or_else(|e| panic!("{family} run {run}: {e}"));
    derived.retain(|r| {
        !excluded
            .split(' ')
            .any(|x| x != "-" && r.get("s").and_then(Value::as_str) == Some(x))
    });
    let want = contract["acceptance"]["rows"].as_array().unwrap();
    let how = if ended { "ended" } else { "stopped as expected" };
    exact_rows(want, &derived).map(|divergence| format!("{how}; {divergence} ({id}@{version})"))
}

/// The runs held on two vCPUs by name to the host's refusal (Sprint 351): of a child process until the run-loop
/// design's card 4f (and of the instruction clock until Sprint 360, from which a family's start on it runs on the wall
/// clock above one vCPU). None from Sprint 364, card 4f3, which lifted the refusal of a child process.
const REFUSED: &str = "store/family_runs/two-vcpus-refused-2026-11-26.tsv";

/// `REFUSED` holds no run (Sprint 364): the host refuses no family run on two vCPUs, a process that starts another
/// running there with its child on one vCPU (on two from Sprint 365), and each of the six it held is held to its
/// one-vCPU run's rows.
#[test]
fn the_refused_list_holds_none() {
    let refused = by_run(&root(), REFUSED);
    assert!(
        refused.is_empty(),
        "{REFUSED} holds {} runs, where the host refuses none on two vCPUs: {:?}",
        refused.len(),
        refused.keys().collect::<Vec<_>>()
    );
}

#[test]
fn every_family_run_conforms_or_is_held_by_what_it_waits_on() {
    let root = root();
    let held = by_run(&root, HELD);
    let refused = by_run(&root, REFUSED);
    let known = fs::read_to_string(root.join(KNOWN)).unwrap();
    let (mut seen, mut wrong) = (BTreeMap::new(), Vec::new());
    let (mut conformed, mut families) = (0, BTreeMap::<String, bool>::new());
    // on two vCPUs: the runs held as on one, those conforming among them, and the refused runs by family
    let (mut as_on_one, mut conformed_on_two) = (0, 0);
    let mut refused_by_family = BTreeMap::<String, usize>::new();
    for line in known.lines().filter(|l| !l.starts_with('#')) {
        let c: Vec<&str> = line.split('\t').collect();
        let (family, run) = (c[0], c[1]);
        let base = root.join(format!("build/family_runs/{family}-{run}"));
        // the same run on two vCPUs (Sprint 351), its outputs beside the one-vCPU run's
        let two = root.join(format!("build/family_runs/{family}-{run}-two-vcpus"));
        let (Some(stop), Some(stop_on_two)) = (stop_at(&root, &base), stop_at(&root, &two)) else {
            return;
        };
        let key = (family.to_string(), run.to_string());
        let state = state_of(&root, &c, &base, &stop);
        // two vCPUs: the refusal of a child process, by name, or the one-vCPU run's rows or held entry
        match refused.get(&key) {
            Some(refusal) if stop_on_two == *refusal => {
                *refused_by_family.entry(family.to_string()).or_default() += 1;
            }
            Some(refusal) => wrong.push(format!(
                "{family} run {run} on two vCPUs stops at {stop_on_two:?}, where {REFUSED} holds it to {refusal:?}"
            )),
            None => {
                let state_on_two = state_of(&root, &c, &two, &stop_on_two);
                if state_on_two == state {
                    as_on_one += 1;
                    conformed_on_two += usize::from(state.is_none());
                } else {
                    let show = |s: &Option<String>| s.clone().unwrap_or_else(|| "conforms".into());
                    wrong.push(format!(
                        "{family} run {run} on two vCPUs: {}; on one vCPU: {}",
                        show(&state_on_two),
                        show(&state)
                    ));
                }
            }
        }
        let Some(state) = state else {
            if held.contains_key(&key) {
                wrong.push(format!("{family} run {run} conforms, and {HELD} still holds it"));
            }
            conformed += 1;
            families.entry(family.to_string()).or_insert(true);
            seen.insert(key, String::new());
            continue;
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
    wrong.extend(
        refused
            .keys()
            .filter(|key| !seen.contains_key(*key))
            .map(|key| format!("{REFUSED} holds {key:?}, which the lookup does not name")),
    );
    // every run's difference at once, so one gates run shows them all (Sprint 230)
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
    println!(
        "{conformed} of {} family runs conform, {} of {} families in every run; the rest held by what they wait on",
        seen.len(),
        families.values().filter(|v| **v).count(),
        families.len()
    );
    // each refused family by its refusal, the child process's naming card 4f, any other by its stop
    let refused_count: usize = refused_by_family.values().sum();
    let by_family: Vec<String> = refused_by_family
        .iter()
        .map(|(f, n)| {
            let stop = refused
                .iter()
                .find(|((family, _), _)| family == f)
                .map_or("", |(_, s)| s.as_str());
            let why = if stop.contains("card 4f") {
                "a child process"
            } else {
                stop
            };
            format!("{f} {n}, {why}")
        })
        .collect();
    println!(
        "on two vCPUs: {as_on_one} of {} held as on one vCPU, {conformed_on_two} of them conforming; {refused_count} \
         refused by name ({})",
        seen.len(),
        by_family.join("; ")
    );
}
