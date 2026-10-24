// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! w2m's vocabulary as Rust types (Sprint 126). One type per tag of `signals/<version>.json`, generated into
//! `generated.rs` by `vocabulary_generator`; this file holds what the generated code is built from.
//!
//! Each field type makes a bad value unrepresentable: a checked string for `sha256`, `id`, `contract_id` and
//! `version`; a harness-plane integer within RFC 8785's 2^53 - 1, since substrate canonicalizes payloads with it; a
//! finite float, since JSON has no NaN. A value that builds is a value substrate's schema accepts.
//!
//! A harness-plane event is written as substrate's line protocol, `{"kind": TAG, "payload": {...}}`
//! (substrate `docs/process-producer.md`); a guest-plane event as one JSONL trace line, `{"tag": TAG, ...}`
//! (`signals/0.9.json` `transport`). Reading refuses a missing field, an unknown field and a value outside its type,
//! and names the field.

use std::fmt;

pub use serde_json::{Map, Value};

#[rustfmt::skip]
mod generated;
pub use generated::*;

/// The largest integer RFC 8785 admits, 2^53 - 1; substrate refuses a payload with a larger one.
pub const HARNESS_INTEGER_LIMIT: u64 = 9_007_199_254_740_991;

/// Which plane a tag travels on (`signals/0.9.json` `transport`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Plane {
    /// A substrate application event.
    Harness,
    /// A line of a guest-trace file.
    Guest,
}

/// A value outside its type, before a field name is known.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvalidValue(pub String);

impl fmt::Display for InvalidValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for InvalidValue {}

/// A payload refused: the field's path (`TAG.field`, `TAG.field[2]`) and why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldError {
    pub path: String,
    pub reason: String,
}

impl FieldError {
    pub fn new(path: impl Into<String>, reason: impl Into<String>) -> Self {
        FieldError {
            path: path.into(),
            reason: reason.into(),
        }
    }
}

impl fmt::Display for FieldError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.path, self.reason)
    }
}

impl std::error::Error for FieldError {}

/// A field's type: how it is written as JSON and read back.
pub trait Field: Sized {
    fn write_json(&self, out: &mut String);
    fn read_json(value: &Value, path: &str) -> Result<Self, FieldError>;
}

/// A JSON string literal: the quotes, and the escapes RFC 8259 section 7 requires.
pub fn write_json_string(s: &str, out: &mut String) {
    out.push('"');
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
}

fn read_str<'v>(value: &'v Value, path: &str) -> Result<&'v str, FieldError> {
    value
        .as_str()
        .ok_or_else(|| FieldError::new(path, format!("expected a string, got {value}")))
}

macro_rules! checked_string {
    ($(#[$doc:meta])* $name:ident, $check:expr) => {
        $(#[$doc])*
        #[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub struct $name(String);

        impl $name {
            pub fn new(s: impl Into<String>) -> Result<Self, InvalidValue> {
                let s = s.into();
                let check: fn(&str) -> Result<(), String> = $check;
                check(&s).map_err(InvalidValue)?;
                Ok($name(s))
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }

        impl Field for $name {
            fn write_json(&self, out: &mut String) {
                write_json_string(&self.0, out);
            }

            fn read_json(value: &Value, path: &str) -> Result<Self, FieldError> {
                $name::new(read_str(value, path)?).map_err(|e| FieldError::new(path, e.0))
            }
        }
    };
}

checked_string!(
    /// `sha256`: `sha256:` and 64 lowercase hexadecimal digits (substrate's content_hash format).
    Sha256,
    |s| match s.strip_prefix("sha256:") {
        Some(hex) if hex.len() == 64 && hex.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')) => Ok(()),
        _ => Err(format!("{s:?} is not sha256: and 64 lowercase hexadecimal digits")),
    }
);

checked_string!(
    /// `id`: a ULID or stable slug, not empty.
    Id,
    |s| if s.is_empty() { Err("an id is not empty".to_string()) } else { Ok(()) }
);

checked_string!(
    /// `contract_id`: two or more dot-separated parts of `[a-z0-9_]`, e.g. `windows.event.auto_reset.single_signal`.
    ContractId,
    |s| {
        let parts: Vec<&str> = s.split('.').collect();
        let part_ok = |p: &&str| !p.is_empty() && p.bytes().all(|b| matches!(b, b'a'..=b'z' | b'0'..=b'9' | b'_'));
        if parts.len() >= 2 && parts.iter().all(part_ok) {
            Ok(())
        } else {
            Err(format!("{s:?} is not two or more dot-separated parts of [a-z0-9_]"))
        }
    }
);

checked_string!(
    /// `version`: major.minor or major.minor.patch, each decimal digits.
    Version,
    |s| {
        let parts: Vec<&str> = s.split('.').collect();
        if (parts.len() == 2 || parts.len() == 3) && parts.iter().all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit())) {
            Ok(())
        } else {
            Err(format!("{s:?} is not major.minor[.patch]"))
        }
    }
);

impl Field for String {
    fn write_json(&self, out: &mut String) {
        write_json_string(self, out);
    }

    fn read_json(value: &Value, path: &str) -> Result<Self, FieldError> {
        read_str(value, path).map(str::to_string)
    }
}

impl Field for bool {
    fn write_json(&self, out: &mut String) {
        out.push_str(if *self { "true" } else { "false" });
    }

    fn read_json(value: &Value, path: &str) -> Result<Self, FieldError> {
        value
            .as_bool()
            .ok_or_else(|| FieldError::new(path, format!("expected a boolean, got {value}")))
    }
}

/// A guest-plane `u64`: the full 64-bit range.
impl Field for u64 {
    fn write_json(&self, out: &mut String) {
        out.push_str(&self.to_string());
    }

    fn read_json(value: &Value, path: &str) -> Result<Self, FieldError> {
        value
            .as_u64()
            .ok_or_else(|| FieldError::new(path, format!("expected an integer in [0, 2^64), got {value}")))
    }
}

/// A guest-plane `i64`: the full 64-bit range.
impl Field for i64 {
    fn write_json(&self, out: &mut String) {
        out.push_str(&self.to_string());
    }

    fn read_json(value: &Value, path: &str) -> Result<Self, FieldError> {
        value
            .as_i64()
            .ok_or_else(|| FieldError::new(path, format!("expected a 64-bit integer, got {value}")))
    }
}

/// A harness-plane `u64`: 0 to 2^53 - 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct HarnessU64(u64);

impl HarnessU64 {
    pub fn new(n: u64) -> Result<Self, InvalidValue> {
        if n <= HARNESS_INTEGER_LIMIT {
            Ok(HarnessU64(n))
        } else {
            Err(InvalidValue(format!(
                "{n} is past 2^53 - 1, the harness plane's limit (RFC 8785)"
            )))
        }
    }

    pub fn get(self) -> u64 {
        self.0
    }
}

impl Field for HarnessU64 {
    fn write_json(&self, out: &mut String) {
        out.push_str(&self.0.to_string());
    }

    fn read_json(value: &Value, path: &str) -> Result<Self, FieldError> {
        HarnessU64::new(u64::read_json(value, path)?).map_err(|e| FieldError::new(path, e.0))
    }
}

/// A harness-plane `i64`: -(2^53 - 1) to 2^53 - 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct HarnessI64(i64);

impl HarnessI64 {
    pub fn new(n: i64) -> Result<Self, InvalidValue> {
        if n.unsigned_abs() <= HARNESS_INTEGER_LIMIT {
            Ok(HarnessI64(n))
        } else {
            Err(InvalidValue(format!(
                "{n} is past ±(2^53 - 1), the harness plane's limit (RFC 8785)"
            )))
        }
    }

    pub fn get(self) -> i64 {
        self.0
    }
}

impl Field for HarnessI64 {
    fn write_json(&self, out: &mut String) {
        out.push_str(&self.0.to_string());
    }

    fn read_json(value: &Value, path: &str) -> Result<Self, FieldError> {
        HarnessI64::new(i64::read_json(value, path)?).map_err(|e| FieldError::new(path, e.0))
    }
}

/// `float`: a finite f64; JSON has no NaN or infinity.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct FiniteF64(f64);

impl FiniteF64 {
    pub fn new(x: f64) -> Result<Self, InvalidValue> {
        if x.is_finite() {
            Ok(FiniteF64(x))
        } else {
            Err(InvalidValue(format!("{x} is not a finite number")))
        }
    }

    pub fn get(self) -> f64 {
        self.0
    }
}

impl Field for FiniteF64 {
    /// Debug, not Display: a whole number keeps its `.0` (`2.0`), so it reads back as a float.
    fn write_json(&self, out: &mut String) {
        out.push_str(&format!("{:?}", self.0));
    }

    fn read_json(value: &Value, path: &str) -> Result<Self, FieldError> {
        let x = value
            .as_f64()
            .ok_or_else(|| FieldError::new(path, format!("expected a number, got {value}")))?;
        FiniteF64::new(x).map_err(|e| FieldError::new(path, e.0))
    }
}

/// `list<T>`: a JSON array of T, each item read at its index.
impl<T: Field> Field for Vec<T> {
    fn write_json(&self, out: &mut String) {
        out.push('[');
        for (i, item) in self.iter().enumerate() {
            if i > 0 {
                out.push_str(", ");
            }
            item.write_json(out);
        }
        out.push(']');
    }

    fn read_json(value: &Value, path: &str) -> Result<Self, FieldError> {
        let items = value
            .as_array()
            .ok_or_else(|| FieldError::new(path, format!("expected an array, got {value}")))?;
        items
            .iter()
            .enumerate()
            .map(|(i, item)| T::read_json(item, &format!("{path}[{i}]")))
            .collect()
    }
}

/// An event's fields being written: `"name": value` pairs, comma-separated.
pub struct FieldWriter<'o> {
    out: &'o mut String,
    first: bool,
}

impl FieldWriter<'_> {
    pub fn field<T: Field>(&mut self, name: &str, value: &T) {
        if !self.first {
            self.out.push_str(", ");
        }
        self.first = false;
        write_json_string(name, self.out);
        self.out.push_str(": ");
        value.write_json(self.out);
    }

    /// An optional field left out is written as null, as substrate records it.
    pub fn optional<T: Field>(&mut self, name: &str, value: &Option<T>) {
        match value {
            Some(v) => self.field(name, v),
            None => {
                if !self.first {
                    self.out.push_str(", ");
                }
                self.first = false;
                write_json_string(name, self.out);
                self.out.push_str(": null");
            }
        }
    }
}

/// An event's fields being read from a JSON object whose keys were checked against the tag's fields.
pub struct FieldReader<'m> {
    tag: &'static str,
    fields: &'m Map<String, Value>,
}

impl FieldReader<'_> {
    pub fn required<T: Field>(&self, name: &str) -> Result<T, FieldError> {
        let path = format!("{}.{name}", self.tag);
        match self.fields.get(name) {
            Some(v) => T::read_json(v, &path),
            None => Err(FieldError::new(path, "missing")),
        }
    }

    /// Absent and null both read as None.
    pub fn optional<T: Field>(&self, name: &str) -> Result<Option<T>, FieldError> {
        match self.fields.get(name) {
            None | Some(Value::Null) => Ok(None),
            Some(v) => T::read_json(v, &format!("{}.{name}", self.tag)).map(Some),
        }
    }
}

/// One tag of the vocabulary. Implemented by the generated types only.
pub trait Event: Sized {
    const TAG: &'static str;
    const PLANE: Plane;
    /// Required fields, then optional ones, in the vocabulary's order.
    const FIELDS: &'static [&'static str];

    fn write_fields(&self, w: &mut FieldWriter<'_>);
    fn read_fields(r: &FieldReader<'_>) -> Result<Self, FieldError>;

    /// The payload as a JSON object.
    fn payload_json(&self) -> String {
        let mut out = String::from("{");
        self.write_fields(&mut FieldWriter {
            out: &mut out,
            first: true,
        });
        out.push('}');
        out
    }

    /// Read a payload, refusing a key the tag does not have. `skip` names a key that is not a field (a guest line's
    /// `tag`).
    fn from_fields(fields: &Map<String, Value>, skip: Option<&str>) -> Result<Self, FieldError> {
        if let Some(extra) = fields
            .keys()
            .find(|k| Some(k.as_str()) != skip && !Self::FIELDS.contains(&k.as_str()))
        {
            return Err(FieldError::new(
                format!("{}.{extra}", Self::TAG),
                "not a field of this tag",
            ));
        }
        Self::read_fields(&FieldReader { tag: Self::TAG, fields })
    }

    fn from_payload(payload: &Value) -> Result<Self, FieldError> {
        let fields = payload
            .as_object()
            .ok_or_else(|| FieldError::new(Self::TAG, format!("a payload is an object, got {payload}")))?;
        Self::from_fields(fields, None)
    }
}

/// A harness-plane event as one line of substrate's line protocol.
pub fn harness_line<E: Event>(event: &E) -> String {
    debug_assert_eq!(E::PLANE, Plane::Harness);
    let mut out = String::from("{\"kind\": ");
    write_json_string(E::TAG, &mut out);
    out.push_str(", \"payload\": ");
    out.push_str(&event.payload_json());
    out.push('}');
    out
}

/// A guest-plane event as one JSONL trace line, its tag in the `tag` key.
pub fn guest_line<E: Event>(event: &E) -> String {
    debug_assert_eq!(E::PLANE, Plane::Guest);
    let mut out = String::from("{\"tag\": ");
    write_json_string(E::TAG, &mut out);
    let payload = event.payload_json();
    if payload.len() > 2 {
        out.push_str(", ");
        out.push_str(&payload[1..payload.len() - 1]);
    }
    out.push('}');
    out
}

fn parse_object(line: &str) -> Result<Map<String, Value>, FieldError> {
    match serde_json::from_str::<Value>(line) {
        Ok(Value::Object(fields)) => Ok(fields),
        Ok(other) => Err(FieldError::new("line", format!("expected an object, got {other}"))),
        Err(e) => Err(FieldError::new("line", format!("not JSON: {e}"))),
    }
}

impl GuestLine {
    /// One guest-trace line, its tag read from the `tag` key.
    pub fn parse(line: &str) -> Result<Self, FieldError> {
        let fields = parse_object(line)?;
        let tag = fields
            .get("tag")
            .and_then(Value::as_str)
            .ok_or_else(|| FieldError::new("tag", "missing"))?;
        GuestLine::from_tag(tag, &fields)
    }
}

impl HarnessEvent {
    /// One line of substrate's line protocol, `{"kind": TAG, "payload": {...}}`.
    pub fn parse(line: &str) -> Result<Self, FieldError> {
        let fields = parse_object(line)?;
        if let Some(extra) = fields.keys().find(|k| *k != "kind" && *k != "payload") {
            return Err(FieldError::new(extra.as_str(), "not a key of a line"));
        }
        let kind = fields
            .get("kind")
            .and_then(Value::as_str)
            .ok_or_else(|| FieldError::new("kind", "missing"))?;
        let payload = fields
            .get("payload")
            .ok_or_else(|| FieldError::new("payload", "missing"))?;
        HarnessEvent::from_kind(kind, payload)
    }
}
