// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! The decode tables, generated from `specs/x86_decode.json` at build time (Sprint 143): `scripts/gen_decoder.py`'s
//! `build_table` (36-58) and the Python tables' constants (74-77), written as Rust into `OUT_DIR/tables.rs`. The spec
//! is the authored artifact; the tables are derived, so a build can never run on stale ones.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::path::Path;

use serde_json::Value;

/// Python's `json.dumps(v, sort_keys=True)`: ", " and ": " separators, keys sorted, non-ASCII and control characters
/// escaped as `\uXXXX` (`ensure_ascii`), the row identity's input (gen_decoder 41, 55).
fn python_dumps(v: &Value, out: &mut String) {
    match v {
        Value::Null => out.push_str("null"),
        Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Value::Number(n) => out.push_str(&n.to_string()),
        Value::String(s) => {
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
                            write!(out, "\\u{u:04x}").unwrap();
                        }
                    }
                    c => out.push(c),
                }
            }
            out.push('"');
        }
        Value::Array(a) => {
            out.push('[');
            for (i, x) in a.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                python_dumps(x, out);
            }
            out.push(']');
        }
        Value::Object(m) => {
            let sorted: BTreeMap<&String, &Value> = m.iter().collect();
            out.push('{');
            for (i, (k, x)) in sorted.into_iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                python_dumps(&Value::String(k.clone()), out);
                out.push_str(": ");
                python_dumps(x, out);
            }
            out.push('}');
        }
    }
}

fn opt_str(v: Option<&Value>) -> String {
    match v.and_then(Value::as_str) {
        Some(s) => format!("Some({s:?})"),
        None => "None".into(),
    }
}

fn opt_int(v: Option<&Value>) -> String {
    match v.and_then(Value::as_u64) {
        Some(n) => format!("Some({n})"),
        None => "None".into(),
    }
}

/// A `{size: name}` map, sorted by size as `tuple(sorted(e["sizes"].items()))`.
fn sizes(v: Option<&Value>) -> String {
    match v.and_then(Value::as_object) {
        Some(m) => {
            let sorted: BTreeMap<&String, &Value> = m.iter().collect();
            let items: Vec<String> = sorted
                .iter()
                .map(|(k, x)| format!("({:?}, {:?})", k, x.as_str().unwrap()))
                .collect();
            format!("Some(&[{}])", items.join(", "))
        }
        None => "None".into(),
    }
}

fn main() {
    let spec_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../specs/x86_decode.json");
    println!("cargo:rerun-if-changed={}", spec_path.display());
    let raw = std::fs::read(&spec_path).unwrap();
    let spec: Value = serde_json::from_slice(&raw).unwrap();
    let digest = guest_runtime::sha256::hex(&raw);

    // gen_decoder 36-58: (map, opcode, encoding class) -> the rows, in spec order
    let mut table: BTreeMap<(String, u64, String), Vec<String>> = BTreeMap::new();
    let mut seen: BTreeMap<String, usize> = BTreeMap::new();
    for (idx, e) in spec["entries"].as_array().unwrap().iter().enumerate() {
        let mut h = String::new();
        python_dumps(e, &mut h);
        if let Some(first) = seen.get(&h) {
            panic!("spec row {idx} duplicates row {first}: {}", e["mn"]);
        }
        seen.insert(h.clone(), idx);
        let flag = |k: &str| e.get(k).and_then(Value::as_bool).unwrap_or(false);
        let enc = if flag("evex") {
            "evex"
        } else if flag("xop") {
            "xop"
        } else if flag("vex") {
            "vex"
        } else {
            "legacy"
        };
        let key = (
            e["map"].as_str().unwrap().to_string(),
            e["op"].as_u64().unwrap(),
            enc.to_string(),
        );
        let fallback = match e.get("fallback") {
            Some(f) => format!(
                "Some(({:?}, {:?}))",
                f["mn"].as_str().unwrap(),
                f["iclass"].as_str().unwrap()
            ),
            None => "None".into(),
        };
        let np = match e.get("np") {
            Some(Value::Bool(true)) => "Np::Np",
            Some(Value::String(s)) if s == "nfx" => "Np::Nfx",
            Some(Value::Bool(false)) | None => "Np::No",
            Some(other) => panic!("np {other}"),
        };
        let reg_valid = match e.get("reg_valid").and_then(Value::as_array) {
            Some(a) => format!(
                "Some(&[{}])",
                a.iter()
                    .map(|x| x.as_u64().unwrap().to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            None => "None".into(),
        };
        let modrm_byte = match e.get("mod").and_then(Value::as_str) {
            Some(s) => format!("Some(0x{:02x})", u8::from_str_radix(s, 16).unwrap()),
            None => "None".into(),
        };
        let row = format!(
            "Row {{ pfx: {:?}, reg: {}, has_modrm: {}, imm: {}, mn: {:?}, sizes: {}, modrm_byte: {}, w: {}, l: {}, \
             form: {}, iclass: {:?}, iclass_sizes: {}, requires: {}, fallback: {}, lock_ok: {}, vvvv: {}, ops: {}, \
             opsize64: {}, row: \"sha256:{}\", np: {}, reg_valid: {} }}",
            e["pfx"].as_str().unwrap(),
            opt_int(e.get("reg")),
            e["modrm"].as_bool().unwrap(),
            opt_str(e.get("imm")),
            e["mn"].as_str().unwrap(),
            sizes(e.get("sizes")),
            modrm_byte,
            opt_int(e.get("w")),
            opt_int(e.get("l")),
            opt_str(e.get("form")),
            e["iclass"].as_str().unwrap(),
            sizes(e.get("iclass_sizes")),
            opt_str(e.get("requires")),
            fallback,
            flag("lock"),
            opt_str(e.get("vvvv")),
            opt_str(e.get("ops")),
            opt_str(e.get("opsize64")),
            guest_runtime::sha256::hex(h.as_bytes()),
            np,
            reg_valid,
        );
        table.entry(key).or_default().push(row);
    }

    let mut out = String::new();
    writeln!(
        out,
        "// GENERATED by crates/x86_decoder/build.rs from specs/x86_decode.json"
    )
    .unwrap();
    writeln!(
        out,
        "pub const SPEC_VERSION: &str = {:?};",
        spec["version"].as_str().unwrap()
    )
    .unwrap();
    writeln!(out, "pub const SPEC_SHA256: &str = \"sha256:{digest}\";").unwrap();
    let set = |name: &str, key: &str, out: &mut String| {
        let items: BTreeSet<&str> = spec["prefixes"]
            .get(key)
            .and_then(Value::as_array)
            .map(|a| a.iter().map(|x| x.as_str().unwrap()).collect())
            .unwrap_or_default();
        let list: Vec<String> = items.iter().map(|s| format!("{s:?}")).collect();
        writeln!(out, "pub static {name}: &[&str] = &[{}];", list.join(", ")).unwrap();
    };
    set("LOCKABLE", "lockable", &mut out);
    set("REP_STRING", "rep_string", &mut out);
    set("REPE_STRING", "repe_string", &mut out);
    let mut legacy: Vec<String> = Vec::new();
    for x in spec["prefixes"]["legacy_prefix_opcodes"].as_array().unwrap() {
        let op = x["opcode"].as_array().unwrap();
        let prefixes: Vec<String> = x["legacy"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| format!("{:?}", p.as_str().unwrap()))
            .collect();
        legacy.push(format!(
            "(({:?}, {}), &[{}])",
            op[0].as_str().unwrap(),
            op[1].as_u64().unwrap(),
            prefixes.join(", ")
        ));
    }
    legacy.sort();
    writeln!(
        out,
        "pub static LEGACY_PREFIX_OPCODES: &[((&str, u8), &[&str])] = &[{}];",
        legacy.join(", ")
    )
    .unwrap();
    let mut rows = Vec::new();
    let mut index = Vec::new();
    for ((map, op, enc), entries) in &table {
        index.push(format!("(({map:?}, {op}, {enc:?}), {}, {})", rows.len(), entries.len()));
        rows.extend(entries.iter().cloned());
    }
    writeln!(out, "pub static ROWS: &[Row] = &[\n    {},\n];", rows.join(",\n    ")).unwrap();
    writeln!(
        out,
        "pub static INDEX: &[((&str, u8, &str), usize, usize)] = &[{}];",
        index.join(", ")
    )
    .unwrap();
    let dest = Path::new(&std::env::var("OUT_DIR").unwrap()).join("tables.rs");
    std::fs::write(dest, out).unwrap();
}
