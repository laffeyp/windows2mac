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
        other => return Err(format!("row rule {other} is not ported")),
    }
    Ok(rows)
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
    let unknown = derive("windows", "windows_rows", Vec::new()).unwrap_err();
    assert_eq!(unknown, "row rule windows_rows is not ported");
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
        // the family's start options (column 7) that `scripts/gates.sh` does not give `w2m-vm`: all but the
        // instruction clock (Sprint 230); a run started without them is not the family's run, and is not judged
        let unapplied: Vec<&str> = c[6]
            .split(' ')
            .filter(|o| !["-", "instruction_clock"].contains(o))
            .collect();
        let state = if stop != "ExitProcess { code: 0 }" {
            format!("stop: {}", steady(&stop))
        } else if !unapplied.is_empty() {
            format!("ended; start options {} not applied", unapplied.join(" "))
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
