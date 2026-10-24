// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! The vocabulary, checked and generated (Sprint 126). `check` holds a vocabulary to the structural rules
//! `scripts/check_vocab.py` held it to, and to the rules Rust names need; `render_rust` writes the `events` crate's
//! `generated.rs`; `render_schema` writes substrate's schema file for the harness-plane tags
//! (substrate `docs/process-producer.md`).

use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

/// The harness plane's integer limit, 2^53 - 1 (RFC 8785; `signals/0.9.json` `types.u64`).
const HARNESS_INTEGER_LIMIT: u64 = 9_007_199_254_740_991;

/// Verbs that are past tense without ending in ED (`check_vocab.py`; INIT is the SESSION_INIT pattern, TECHNIQUES.md
/// #9).
const IRREGULAR_PAST: [&str; 3] = ["READ", "BUILT", "INIT"];

const STRATA: [&str; 4] = ["event", "ambient", "summary", "incident"];

/// A foreign-key field's stem and the ontology entity it names (`check_vocab.py` FK_ALIAS).
const FOREIGN_KEY_ENTITY: &[(&str, &str)] = &[
    ("evidence", "evidence"),
    ("deciding_evidence", "evidence"),
    ("superseded_by", "evidence"),
    ("contract", "contract"),
    ("probe", "probe"),
    ("parent_probe", "probe"),
    ("execution", "execution"),
    ("target_execution", "execution"),
    ("model_check", "modelcheck"),
    ("comparison", "comparison"),
    ("reduction", "discrepancy"),
    ("discrepancy", "discrepancy"),
    ("triggering_discrepancy", "discrepancy"),
    ("candidate", "candidate"),
    ("parent_candidate", "candidate"),
    ("translation", "translation"),
    ("artifact", "artifact"),
    ("evaluation", "backend"),
    ("measurement", "measurement"),
    ("application_profile", "applicationprofile"),
    ("destination_profile", "destinationprofile"),
    ("host", "destinationprofile"),
    ("reference_profile", "referenceprofile"),
    ("profile", "referenceprofile"),
    ("bundle", "bundle"),
    ("app", "distribution"),
    ("build", "distribution"),
    ("from_build", "distribution"),
    ("to_build", "distribution"),
    ("depot", "distribution"),
    ("thread", "guestthread"),
    ("parent_thread", "guestthread"),
    ("subject_thread", "guestthread"),
];

/// Rust's keywords, strict and reserved (The Rust Reference, "Keywords", edition 2024); a field may not be one.
const RUST_KEYWORDS: &[&str] = &[
    "as", "break", "const", "continue", "crate", "else", "enum", "extern", "false", "fn", "for", "if", "impl", "in",
    "let", "loop", "match", "mod", "move", "mut", "pub", "ref", "return", "self", "Self", "static", "struct", "super",
    "trait", "true", "type", "unsafe", "use", "where", "while", "async", "await", "dyn", "abstract", "become", "box",
    "do", "final", "macro", "override", "priv", "typeof", "unsized", "virtual", "yield", "gen",
];

/// Type names the `events` crate writes by hand; no tag or enum may take one.
const HAND_WRITTEN_TYPES: &[&str] = &[
    "Sha256",
    "Id",
    "ContractId",
    "Version",
    "HarnessU64",
    "HarnessI64",
    "FiniteF64",
    "Plane",
    "InvalidValue",
    "FieldError",
    "Field",
    "FieldWriter",
    "FieldReader",
    "Event",
    "GuestLine",
    "HarnessEvent",
];

/// One tag, its fields in the vocabulary's order.
#[derive(Debug, Clone)]
pub struct Tag {
    pub name: String,
    pub category: String,
    pub stratum: String,
    pub required: Vec<String>,
    pub optional: Vec<String>,
    pub types: BTreeMap<String, String>,
    pub note: String,
}

impl Tag {
    pub fn is_guest(&self) -> bool {
        self.category == "guest"
    }

    /// Required fields, then optional ones, each with whether it is optional.
    pub fn fields(&self) -> impl Iterator<Item = (&str, bool)> {
        self.required
            .iter()
            .map(|f| (f.as_str(), false))
            .chain(self.optional.iter().map(|f| (f.as_str(), true)))
    }
}

/// A vocabulary file, read.
#[derive(Debug, Clone)]
pub struct Vocabulary {
    pub version: String,
    pub tags: Vec<Tag>,
    pub enums: BTreeMap<String, Vec<String>>,
    pub guest_plane_versions: Vec<String>,
    raw: Value,
}

fn key<'v>(v: &'v Value, k: &str, at: &str) -> Result<&'v Value, String> {
    v.get(k).ok_or_else(|| format!("{at}: missing {k}"))
}

fn string(v: &Value, k: &str, at: &str) -> Result<String, String> {
    key(v, k, at)?
        .as_str()
        .map(str::to_string)
        .ok_or_else(|| format!("{at}.{k}: not a string"))
}

fn array<'v>(v: &'v Value, k: &str, at: &str) -> Result<&'v Vec<Value>, String> {
    key(v, k, at)?
        .as_array()
        .ok_or_else(|| format!("{at}.{k}: not an array"))
}

fn strings(v: &Value, k: &str, at: &str) -> Result<Vec<String>, String> {
    array(v, k, at)?
        .iter()
        .map(|s| {
            s.as_str()
                .map(str::to_string)
                .ok_or_else(|| format!("{at}.{k}: an item is not a string"))
        })
        .collect()
}

impl Vocabulary {
    /// Read a vocabulary's JSON. A shape error (a key missing, a value of the wrong JSON type) is an error here; the
    /// rules are `check`'s.
    pub fn parse(text: &str) -> Result<Vocabulary, Vec<String>> {
        let raw: Value = serde_json::from_str(text).map_err(|e| vec![format!("not JSON: {e}")])?;
        let mut errors = Vec::new();
        let mut tags = Vec::new();
        for (i, t) in raw
            .get("tags")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .enumerate()
        {
            let at = t
                .get("name")
                .and_then(Value::as_str)
                .map_or(format!("tags[{i}]"), str::to_string);
            let mut field = |k: &str| string(t, k, &at).map_err(|e| errors.push(e)).unwrap_or_default();
            let (name, category, stratum, note) = (field("name"), field("category"), field("stratum"), field("note"));
            let required = strings(t, "payload", &at)
                .map_err(|e| errors.push(e))
                .unwrap_or_default();
            let optional = match t.get("optional_payload") {
                None => Vec::new(),
                Some(_) => strings(t, "optional_payload", &at)
                    .map_err(|e| errors.push(e))
                    .unwrap_or_default(),
            };
            let mut types = BTreeMap::new();
            match t.get("types").and_then(Value::as_object) {
                Some(m) => {
                    for (f, ty) in m {
                        match ty.as_str() {
                            Some(ty) => {
                                types.insert(f.clone(), ty.to_string());
                            }
                            None => errors.push(format!("{at}.types.{f}: not a string")),
                        }
                    }
                }
                None => errors.push(format!("{at}: missing types")),
            }
            tags.push(Tag {
                name,
                category,
                stratum,
                required,
                optional,
                types,
                note,
            });
        }
        if raw.get("tags").and_then(Value::as_array).is_none() {
            errors.push("missing tags".to_string());
        }
        let mut enums = BTreeMap::new();
        match raw.get("enums").and_then(Value::as_object) {
            Some(m) => {
                for (name, values) in m {
                    match values
                        .as_array()
                        .map(|vs| vs.iter().map(|v| v.as_str().map(str::to_string)).collect())
                    {
                        Some(Some(vs)) => {
                            enums.insert(name.clone(), vs);
                        }
                        _ => errors.push(format!("enums.{name}: not an array of strings")),
                    }
                }
            }
            None => errors.push("missing enums".to_string()),
        }
        let version = string(&raw, "version", "vocabulary")
            .map_err(|e| errors.push(e))
            .unwrap_or_default();
        let guest_plane_versions = match raw.get("guest_plane_compatible_versions") {
            None => vec![version.clone()],
            Some(_) => strings(&raw, "guest_plane_compatible_versions", "vocabulary")
                .map_err(|e| errors.push(e))
                .unwrap_or_default(),
        };
        if errors.is_empty() {
            Ok(Vocabulary {
                version,
                tags,
                enums,
                guest_plane_versions,
                raw,
            })
        } else {
            Err(errors)
        }
    }

    pub fn tag_count(&self) -> usize {
        self.tags.len()
    }

    pub fn harness_tags(&self) -> impl Iterator<Item = &Tag> {
        self.tags.iter().filter(|t| !t.is_guest())
    }

    pub fn guest_tags(&self) -> impl Iterator<Item = &Tag> {
        self.tags.iter().filter(|t| t.is_guest())
    }
}

/// `SESSION_INIT` -> `SessionInit`; `session_purpose` -> `SessionPurpose`.
pub fn camel(name: &str) -> String {
    name.split('_')
        .filter(|p| !p.is_empty())
        .map(|p| {
            let lower = p.to_ascii_lowercase();
            let mut cs = lower.chars();
            match cs.next() {
                Some(c) => c.to_ascii_uppercase().to_string() + cs.as_str(),
                None => String::new(),
            }
        })
        .collect()
}

fn is_upper_snake_with_two_words(n: &str) -> bool {
    let parts: Vec<&str> = n.split('_').collect();
    parts.len() >= 2
        && parts
            .iter()
            .all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_uppercase() || b.is_ascii_digit()))
}

fn is_lower_identifier(s: &str) -> bool {
    s.bytes().next().is_some_and(|b| b.is_ascii_lowercase())
        && s.bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
}

/// The element type of `list<T>`, or the type itself.
fn base_type(ty: &str) -> &str {
    ty.strip_prefix("list<").and_then(|t| t.strip_suffix('>')).unwrap_or(ty)
}

/// The UPPER_SNAKE words of a text (two or more parts of [A-Z0-9]), each whole: the tags it names.
fn named_tags(text: &str) -> BTreeSet<String> {
    text.split(|c: char| !(c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_'))
        .filter(|w| is_upper_snake_with_two_words(w))
        .map(str::to_string)
        .collect()
}

fn foreign_key_stem(field: &str) -> Option<&str> {
    field.strip_suffix("_ids").or_else(|| field.strip_suffix("_id"))
}

/// Every rule the vocabulary breaks, and a summary line. Empty errors: the vocabulary may be generated from.
pub fn check(v: &Vocabulary) -> (Vec<String>, String) {
    let raw = &v.raw;
    let mut errs = Vec::new();
    let categories: BTreeSet<&str> = raw
        .get("categories")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|c| c.get("name").and_then(Value::as_str))
        .collect();
    let types: BTreeSet<&str> = raw
        .get("types")
        .and_then(Value::as_object)
        .into_iter()
        .flatten()
        .map(|(k, _)| k.as_str())
        .collect();
    let tag_names: BTreeSet<&str> = v.tags.iter().map(|t| t.name.as_str()).collect();
    if tag_names.len() != v.tags.len() {
        errs.push("two tags share a name".to_string());
    }

    for t in &v.tags {
        let n = &t.name;
        if !categories.contains(t.category.as_str()) {
            errs.push(format!("{n}: unknown category {}", t.category));
        }
        if !STRATA.contains(&t.stratum.as_str()) {
            errs.push(format!("{n}: bad stratum {}", t.stratum));
        }
        let fields: BTreeSet<&str> = t.fields().map(|(f, _)| f).collect();
        let typed: BTreeSet<&str> = t.types.keys().map(String::as_str).collect();
        if fields != typed {
            let diff: Vec<&str> = fields.symmetric_difference(&typed).copied().collect();
            errs.push(format!("{n}: untyped or extra fields {diff:?}"));
        }
        if fields.len() != t.required.len() + t.optional.len() {
            errs.push(format!("{n}: a field is named twice"));
        }
        for (f, ty) in &t.types {
            let base = base_type(ty);
            if base.starts_with("list<") {
                errs.push(format!("{n}.{f}: a list of lists, {ty}"));
            } else if let Some(e) = base.strip_prefix("enum:") {
                if !v.enums.contains_key(e) {
                    errs.push(format!("{n}.{f}: unknown enum {base}"));
                }
            } else if !types.contains(base) {
                errs.push(format!("{n}.{f}: unknown type {ty}"));
            }
        }
        if !is_upper_snake_with_two_words(n) {
            errs.push(format!("{n}: name is not UPPER_SNAKE with an entity and a verb"));
        }
        let last = n.rsplit('_').next().unwrap_or_default();
        if !(last.ends_with("ED") || IRREGULAR_PAST.contains(&last)) {
            errs.push(format!("{n}: verb {last} is not past tense"));
        }
        // the rules Rust names need
        for (f, _) in t.fields() {
            if !is_lower_identifier(f) || RUST_KEYWORDS.contains(&f) {
                errs.push(format!(
                    "{n}.{f}: a field is a lowercase identifier and not a Rust keyword"
                ));
            }
            if t.is_guest() && f == "tag" {
                errs.push(format!(
                    "{n}.tag: a guest line's tag is its `tag` key; no field may take it"
                ));
            }
        }
    }

    let mut emitted = BTreeSet::new();
    let operators = raw
        .pointer("/operators/operators")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    for o in &operators {
        let name = o.get("name").and_then(Value::as_str).unwrap_or("?");
        for (k, is_emit) in [("emits", true), ("consumes", false)] {
            for t in o
                .get(k)
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
            {
                if !tag_names.contains(t) {
                    errs.push(format!("operator {name}: unknown tag {t}"));
                }
                if is_emit {
                    emitted.insert(t);
                }
            }
        }
    }
    for n in &tag_names {
        if !emitted.contains(n) {
            errs.push(format!("{n}: no operator emits it"));
        }
    }
    let known = |t: &str| tag_names.contains(t) || t == "guest:*";
    for r in raw
        .pointer("/state_transitions/rules")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let id = r.get("id").and_then(Value::as_str).unwrap_or("?");
        let from = r.get("from_tag").and_then(Value::as_str).into_iter();
        let to = ["to_tags_allowed", "to_tags_forbidden"].into_iter().flat_map(|k| {
            r.get(k)
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
        });
        for t in from.chain(to) {
            if !known(t) {
                errs.push(format!("{id}: unknown tag {t}"));
            }
        }
    }
    let invariants = raw
        .pointer("/temporal_invariants/invariants")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    for i in &invariants {
        let id = i.get("id").and_then(Value::as_str).unwrap_or("?");
        for t in i
            .get("tags")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
        {
            if !known(t) {
                errs.push(format!("{id}: unknown tag {t}"));
            }
        }
    }
    let constraint = |id: &str| -> String {
        raw.pointer("/evidence_constraints/constraints")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .find(|c| c.get("id").and_then(Value::as_str) == Some(id))
            .and_then(|c| c.get("spec"))
            .map(|s| s.as_str().map_or(s.to_string(), str::to_string))
            .unwrap_or_default()
    };
    // the tags a text names: its UPPER_SNAKE words, matched whole (check_vocab.py matched substrings, so a tag whose
    // name is part of a named one passed; review of Sprint 126)
    let (e7, e8) = (named_tags(&constraint("E7")), named_tags(&constraint("E8")));
    let cadenced = named_tags(
        &invariants
            .iter()
            .filter(|i| i.get("kind").and_then(Value::as_str) == Some("cadence"))
            .filter_map(|i| i.get("rule").and_then(Value::as_str))
            .collect::<Vec<_>>()
            .join(" "),
    );
    for t in &v.tags {
        let n = t.name.as_str();
        match t.stratum.as_str() {
            "incident" if !e7.contains(n) => {
                errs.push(format!("{n}: incident tag without a diagnostic constraint (E7)"))
            }
            "summary" if !e8.contains(n) => errs.push(format!("{n}: summary tag without an outcome constraint (E8)")),
            "ambient" if !cadenced.contains(n) => errs.push(format!("{n}: ambient tag without a cadence invariant")),
            _ => {}
        }
    }
    let entities: BTreeSet<String> = raw
        .pointer("/ontology/entities")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|e| e.get("name").and_then(Value::as_str))
        .map(str::to_ascii_lowercase)
        .collect();
    let entity_of: BTreeMap<&str, &str> = FOREIGN_KEY_ENTITY.iter().copied().collect();
    let foreign_keys: Vec<(&str, &str)> = v
        .tags
        .iter()
        .flat_map(|t| t.types.keys().map(move |f| (t.name.as_str(), f.as_str())))
        .filter(|(_, f)| foreign_key_stem(f).is_some())
        .collect();
    let unresolved: Vec<String> = foreign_keys
        .iter()
        .filter(|(_, f)| {
            foreign_key_stem(f)
                .and_then(|s| entity_of.get(s))
                .is_none_or(|e| !entities.contains(*e))
        })
        .map(|(n, f)| format!("{n}.{f}"))
        .collect();
    if !foreign_keys.is_empty() && unresolved.len() * 100 > foreign_keys.len() * 30 {
        errs.push(format!("foreign-key resolution below 70%: {unresolved:?}"));
    }

    // the rules Rust names need: enum values become variants, tags and enums become types
    let mut type_names: BTreeMap<String, String> = BTreeMap::new();
    for h in HAND_WRITTEN_TYPES {
        type_names.insert(h.to_string(), format!("the hand-written {h}"));
    }
    for (name, values) in &v.enums {
        if !is_lower_identifier(name) {
            errs.push(format!("enum:{name}: an enum's name is a lowercase identifier"));
        }
        if values.is_empty() {
            errs.push(format!("enum:{name}: no values"));
        }
        let mut variants = BTreeSet::new();
        for value in values {
            if !is_lower_identifier(value) {
                errs.push(format!("enum:{name}: value {value:?} is not a lowercase identifier"));
            } else if !variants.insert(camel(value)) {
                errs.push(format!(
                    "enum:{name}: value {value:?} names the same variant as another"
                ));
            }
        }
        if let Some(other) = type_names.insert(camel(name), format!("enum:{name}")) {
            errs.push(format!("enum:{name}: its type {} is also {other}", camel(name)));
        }
    }
    for t in &v.tags {
        if let Some(other) = type_names.insert(camel(&t.name), t.name.clone()) {
            errs.push(format!("{}: its type {} is also {other}", t.name, camel(&t.name)));
        }
    }

    let summary = format!(
        "vocabulary {}: {} tags, {} categories, {} foreign keys ({} resolved), {} operators",
        v.version,
        v.tags.len(),
        categories.len(),
        foreign_keys.len(),
        foreign_keys.len() - unresolved.len(),
        operators.len()
    );
    (errs, summary)
}

fn json_string(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// A Rust string literal; `{:?}` of a str is one.
fn rust_string(s: &str) -> String {
    format!("{s:?}")
}

fn rust_type(ty: &str, guest: bool) -> String {
    if let Some(inner) = ty.strip_prefix("list<").and_then(|t| t.strip_suffix('>')) {
        return format!("Vec<{}>", rust_type(inner, guest));
    }
    if let Some(e) = ty.strip_prefix("enum:") {
        return camel(e);
    }
    match (ty, guest) {
        ("sha256", _) => "Sha256",
        ("id", _) => "Id",
        ("contract_id", _) => "ContractId",
        ("version", _) => "Version",
        ("u64", false) => "HarnessU64",
        ("i64", false) => "HarnessI64",
        ("u64", true) => "u64",
        ("i64", true) => "i64",
        ("float", _) => "FiniteF64",
        ("bool", _) => "bool",
        ("text", _) => "String",
        _ => unreachable!("check refuses type {ty}"),
    }
    .to_string()
}

/// The `events` crate's `generated.rs`.
pub fn render_rust(v: &Vocabulary) -> String {
    let mut o = String::new();
    let line = |o: &mut String, s: &str| {
        o.push_str(s);
        o.push('\n');
    };
    line(&mut o, "// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0");
    line(&mut o, "// Copyright (C) 2026 Peter Laffey");
    line(
        &mut o,
        &format!(
            "// GENERATED by crates/vocabulary_generator from signals/{}.json. Do not edit; regenerate with\n\
             // `cargo run --release -p vocabulary_generator`.",
            v.version
        ),
    );
    line(
        &mut o,
        &format!(
            "//! The {} tags and {} enums of vocabulary {}.",
            v.tags.len(),
            v.enums.len(),
            v.version
        ),
    );
    line(&mut o, "");
    line(&mut o, "use super::*;");
    line(&mut o, "");
    line(
        &mut o,
        &format!("pub const VOCAB_VERSION: &str = {};", rust_string(&v.version)),
    );
    let list = |xs: Vec<&str>| xs.iter().map(|x| rust_string(x)).collect::<Vec<_>>().join(", ");
    line(
        &mut o,
        &format!(
            "pub const GUEST_PLANE_VERSIONS: &[&str] = &[{}];",
            list(v.guest_plane_versions.iter().map(String::as_str).collect())
        ),
    );
    line(
        &mut o,
        &format!(
            "pub const HARNESS_TAGS: &[&str] = &[{}];",
            list(v.harness_tags().map(|t| t.name.as_str()).collect())
        ),
    );
    line(
        &mut o,
        &format!(
            "pub const GUEST_TAGS: &[&str] = &[{}];",
            list(v.guest_tags().map(|t| t.name.as_str()).collect())
        ),
    );

    for (name, values) in &v.enums {
        let ty = camel(name);
        line(&mut o, "");
        line(&mut o, &format!("/// `enum:{name}`."));
        line(
            &mut o,
            "#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]",
        );
        line(&mut o, &format!("pub enum {ty} {{"));
        for value in values {
            line(&mut o, &format!("    {},", camel(value)));
        }
        line(&mut o, "}");
        line(&mut o, "");
        line(&mut o, &format!("impl {ty} {{"));
        let all: Vec<String> = values.iter().map(|x| format!("{ty}::{}", camel(x))).collect();
        line(
            &mut o,
            &format!("    pub const ALL: &'static [{ty}] = &[{}];", all.join(", ")),
        );
        line(&mut o, "");
        line(&mut o, "    pub fn as_str(self) -> &'static str {");
        line(&mut o, "        match self {");
        for value in values {
            line(
                &mut o,
                &format!("            {ty}::{} => {},", camel(value), rust_string(value)),
            );
        }
        line(&mut o, "        }");
        line(&mut o, "    }");
        line(&mut o, "");
        line(&mut o, "    pub fn parse(s: &str) -> Option<Self> {");
        line(&mut o, "        match s {");
        for value in values {
            line(
                &mut o,
                &format!("            {} => Some({ty}::{}),", rust_string(value), camel(value)),
            );
        }
        line(&mut o, "            _ => None,");
        line(&mut o, "        }");
        line(&mut o, "    }");
        line(&mut o, "}");
        line(&mut o, "");
        line(&mut o, &format!("impl Field for {ty} {{"));
        line(&mut o, "    fn write_json(&self, out: &mut String) {");
        line(&mut o, "        write_json_string(self.as_str(), out);");
        line(&mut o, "    }");
        line(&mut o, "");
        line(
            &mut o,
            "    fn read_json(value: &Value, path: &str) -> Result<Self, FieldError> {",
        );
        line(
            &mut o,
            "        let s = value.as_str().ok_or_else(|| FieldError::new(path, format!(\"expected a string, got {value}\")))?;",
        );
        line(
            &mut o,
            &format!(
                "        Self::parse(s).ok_or_else(|| FieldError::new(path, format!(\"{{s:?}} is not a value of enum:{name}\")))"
            ),
        );
        line(&mut o, "    }");
        line(&mut o, "}");
    }

    for t in &v.tags {
        let ty = camel(&t.name);
        let plane = if t.is_guest() { "Guest" } else { "Harness" };
        line(&mut o, "");
        line(&mut o, &format!("/// {} ({}, {}).", t.name, t.category, t.stratum));
        line(&mut o, "///");
        line(&mut o, &format!("/// {}", t.note.replace('\n', " ")));
        line(&mut o, "#[derive(Debug, Clone, PartialEq)]");
        line(&mut o, &format!("pub struct {ty} {{"));
        for (f, optional) in t.fields() {
            let fty = rust_type(&t.types[f], t.is_guest());
            let fty = if optional { format!("Option<{fty}>") } else { fty };
            line(&mut o, &format!("    pub {f}: {fty},"));
        }
        line(&mut o, "}");
        line(&mut o, "");
        line(&mut o, &format!("impl Event for {ty} {{"));
        line(
            &mut o,
            &format!("    const TAG: &'static str = {};", rust_string(&t.name)),
        );
        line(&mut o, &format!("    const PLANE: Plane = Plane::{plane};"));
        line(
            &mut o,
            &format!(
                "    const FIELDS: &'static [&'static str] = &[{}];",
                list(t.fields().map(|(f, _)| f).collect())
            ),
        );
        line(&mut o, "");
        let empty = t.required.is_empty() && t.optional.is_empty();
        line(
            &mut o,
            &format!(
                "    fn write_fields(&self, {}: &mut FieldWriter<'_>) {{",
                if empty { "_w" } else { "w" }
            ),
        );
        for (f, optional) in t.fields() {
            let call = if optional { "optional" } else { "field" };
            line(&mut o, &format!("        w.{call}({}, &self.{f});", rust_string(f)));
        }
        line(&mut o, "    }");
        line(&mut o, "");
        line(
            &mut o,
            &format!(
                "    fn read_fields({}: &FieldReader<'_>) -> Result<Self, FieldError> {{",
                if empty { "_r" } else { "r" }
            ),
        );
        line(&mut o, &format!("        Ok({ty} {{"));
        for (f, optional) in t.fields() {
            let call = if optional { "optional" } else { "required" };
            line(&mut o, &format!("            {f}: r.{call}({})?,", rust_string(f)));
        }
        line(&mut o, "        })");
        line(&mut o, "    }");
        line(&mut o, "}");
    }

    for (plane, enum_name, tags) in [
        ("Guest", "GuestLine", v.guest_tags().collect::<Vec<_>>()),
        ("Harness", "HarnessEvent", v.harness_tags().collect::<Vec<_>>()),
    ] {
        line(&mut o, "");
        line(&mut o, &format!("/// Every {}-plane tag.", plane.to_ascii_lowercase()));
        line(
            &mut o,
            "#[allow(clippy::large_enum_variant, reason = \"one variant per tag, each the tag's own fields\")]",
        );
        line(&mut o, "#[derive(Debug, Clone, PartialEq)]");
        line(&mut o, &format!("pub enum {enum_name} {{"));
        for t in &tags {
            line(&mut o, &format!("    {}({}),", camel(&t.name), camel(&t.name)));
        }
        line(&mut o, "}");
        line(&mut o, "");
        line(&mut o, &format!("impl {enum_name} {{"));
        if plane == "Guest" {
            line(
                &mut o,
                "    /// The event for `tag`, from a line's keys; the `tag` key itself is not a field.",
            );
            line(
                &mut o,
                "    pub fn from_tag(tag: &str, fields: &Map<String, Value>) -> Result<Self, FieldError> {",
            );
            line(&mut o, "        match tag {");
            for t in &tags {
                let c = camel(&t.name);
                line(
                    &mut o,
                    &format!(
                        "            {} => {c}::from_fields(fields, Some(\"tag\")).map({enum_name}::{c}),",
                        rust_string(&t.name)
                    ),
                );
            }
            line(
                &mut o,
                "            _ => Err(FieldError::new(\"tag\", format!(\"{tag:?} is not a guest-plane tag\"))),",
            );
        } else {
            line(&mut o, "    /// The event for `kind`, from its payload.");
            line(
                &mut o,
                "    pub fn from_kind(kind: &str, payload: &Value) -> Result<Self, FieldError> {",
            );
            line(&mut o, "        match kind {");
            for t in &tags {
                let c = camel(&t.name);
                line(
                    &mut o,
                    &format!(
                        "            {} => {c}::from_payload(payload).map({enum_name}::{c}),",
                        rust_string(&t.name)
                    ),
                );
            }
            line(
                &mut o,
                "            _ => Err(FieldError::new(\"kind\", format!(\"{kind:?} is not a harness-plane tag\"))),",
            );
        }
        line(&mut o, "        }");
        line(&mut o, "    }");
        line(&mut o, "");
        line(&mut o, "    pub fn tag(&self) -> &'static str {");
        line(&mut o, "        match self {");
        for t in &tags {
            let c = camel(&t.name);
            line(&mut o, &format!("            {enum_name}::{c}(_) => {c}::TAG,"));
        }
        line(&mut o, "        }");
        line(&mut o, "    }");
        line(&mut o, "");
        line(&mut o, "    pub fn line(&self) -> String {");
        line(&mut o, "        match self {");
        let writer = if plane == "Guest" { "guest_line" } else { "harness_line" };
        for t in &tags {
            line(
                &mut o,
                &format!("            {enum_name}::{}(e) => {writer}(e),", camel(&t.name)),
            );
        }
        line(&mut o, "        }");
        line(&mut o, "    }");
        line(&mut o, "}");
    }
    o
}

/// One field's spec in substrate's schema file, for a harness-plane type. Patterns end in `\Z`, not `$`: Python's `$`
/// (msgspec matches with `re`) also matches before a final newline, which the Rust types refuse (review of Sprint 126).
fn schema_spec(ty: &str, v: &Vocabulary) -> String {
    if let Some(inner) = ty.strip_prefix("list<").and_then(|t| t.strip_suffix('>')) {
        return format!("{{\"type\": \"list\", \"items\": {}}}", schema_spec(inner, v));
    }
    if let Some(e) = ty.strip_prefix("enum:") {
        let values: Vec<String> = v.enums[e].iter().map(|x| json_string(x)).collect();
        return format!("{{\"type\": \"str\", \"enum\": [{}]}}", values.join(", "));
    }
    let limit = HARNESS_INTEGER_LIMIT;
    match ty {
        "sha256" => format!(
            "{{\"type\": \"str\", \"pattern\": {}}}",
            json_string(r"^sha256:[0-9a-f]{64}\Z")
        ),
        "id" => "{\"type\": \"str\", \"min_length\": 1}".to_string(),
        "contract_id" => format!(
            "{{\"type\": \"str\", \"pattern\": {}}}",
            json_string(r"^[a-z0-9_]+(\.[a-z0-9_]+)+\Z")
        ),
        "version" => format!(
            "{{\"type\": \"str\", \"pattern\": {}}}",
            json_string(r"^[0-9]+\.[0-9]+(\.[0-9]+)?\Z")
        ),
        "u64" => format!("{{\"type\": \"int\", \"ge\": 0, \"le\": {limit}}}"),
        "i64" => format!("{{\"type\": \"int\", \"ge\": -{limit}, \"le\": {limit}}}"),
        "float" => "{\"type\": \"float\"}".to_string(),
        "bool" => "{\"type\": \"bool\"}".to_string(),
        "text" => "{\"type\": \"str\"}".to_string(),
        _ => unreachable!("check refuses type {ty}"),
    }
}

/// Substrate's schema file for every harness-plane tag (substrate `docs/process-producer.md`).
pub fn render_schema(v: &Vocabulary) -> String {
    let kinds: Vec<String> = v
        .harness_tags()
        .map(|t| {
            let fields: Vec<String> = t
                .fields()
                .map(|(f, optional)| {
                    let mut spec = schema_spec(&t.types[f], v);
                    if optional {
                        spec.truncate(spec.len() - 1);
                        spec.push_str(", \"optional\": true}");
                    }
                    format!("      {}: {spec}", json_string(f))
                })
                .collect();
            format!("    {}: {{\n{}\n    }}", json_string(&t.name), fields.join(",\n"))
        })
        .collect();
    format!(
        "{{\n  \"schema_version\": 1,\n  \"kinds\": {{\n{}\n  }}\n}}\n",
        kinds.join(",\n")
    )
}

/// A sample value of a field's type, at the type's edge where it has one: the harness plane's integer limits, the
/// guest plane's 64-bit ones, an enum's last value, a text with a quote, a newline and a non-ASCII letter.
pub fn sample_value(ty: &str, guest: bool, v: &Vocabulary) -> Value {
    if let Some(inner) = ty.strip_prefix("list<").and_then(|t| t.strip_suffix('>')) {
        return Value::Array(vec![sample_value(inner, guest, v), sample_value(inner, guest, v)]);
    }
    if let Some(e) = ty.strip_prefix("enum:") {
        return Value::from(v.enums[e].last().cloned().unwrap_or_default());
    }
    match (ty, guest) {
        ("sha256", _) => Value::from(format!("sha256:{}", "0123456789abcdef".repeat(4))),
        ("id", _) => Value::from("01JSAMPLE"),
        ("contract_id", _) => Value::from("windows.sample.contract"),
        ("version", _) => Value::from("0.9.1"),
        ("u64", false) => Value::from(HARNESS_INTEGER_LIMIT),
        ("i64", false) => Value::from(-(HARNESS_INTEGER_LIMIT as i64)),
        ("u64", true) => Value::from(u64::MAX),
        ("i64", true) => Value::from(i64::MIN),
        ("float", _) => Value::from(0.5),
        ("bool", _) => Value::from(true),
        ("text", _) => Value::from("a \"quoted\" line\nthen é"),
        _ => unreachable!("check refuses type {ty}"),
    }
}

/// A payload of every field of `tag` at its sample value; optional fields present or left out.
pub fn sample_payload(tag: &Tag, with_optional: bool, v: &Vocabulary) -> serde_json::Map<String, Value> {
    tag.fields()
        .filter(|(_, optional)| with_optional || !optional)
        .map(|(f, _)| (f.to_string(), sample_value(&tag.types[f], tag.is_guest(), v)))
        .collect()
}

/// The vocabulary file `signals/current.json` names, under the repository `root`.
pub fn current_vocabulary_path(root: &std::path::Path) -> Result<std::path::PathBuf, String> {
    let current = root.join("signals/current.json");
    let text = std::fs::read_to_string(&current).map_err(|e| format!("{}: {e}", current.display()))?;
    let value: Value = serde_json::from_str(&text).map_err(|e| format!("{}: {e}", current.display()))?;
    let version = value
        .get("version")
        .and_then(Value::as_str)
        .ok_or_else(|| format!("{}: no version", current.display()))?;
    Ok(root.join(format!("signals/{version}.json")))
}

/// The current vocabulary, read and checked.
pub fn load_current(root: &std::path::Path) -> Result<Vocabulary, String> {
    let path = current_vocabulary_path(root)?;
    let text = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let vocab = Vocabulary::parse(&text).map_err(|errs| format!("{}: {}", path.display(), errs.join("; ")))?;
    match check(&vocab).0 {
        errors if errors.is_empty() => Ok(vocab),
        errors => Err(format!("{}: {}", path.display(), errors.join("; "))),
    }
}
