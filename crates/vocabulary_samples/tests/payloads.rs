// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! Every tag's generated type against the vocabulary's own sample payloads (Sprint 126): each reads back what it
//! writes, and refuses a missing field, an unknown field, and a value outside its field's type, naming the field.

use std::path::Path;

use events::{GuestLine, HarnessEvent, Map, Value};
use vocabulary_generator::{Tag, Vocabulary, load_current, sample_payload};

fn vocabulary() -> Vocabulary {
    load_current(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")).expect("the current vocabulary passes check")
}

/// Read a payload as `tag`'s generated type, and the line that type writes back again.
fn read(tag: &Tag, payload: &Map<String, Value>) -> Result<String, events::FieldError> {
    if tag.is_guest() {
        let e = GuestLine::from_tag(&tag.name, payload)?;
        assert_eq!(
            GuestLine::parse(&e.line()).as_ref(),
            Ok(&e),
            "{} does not read back",
            tag.name
        );
        Ok(e.line())
    } else {
        let e = HarnessEvent::from_kind(&tag.name, &Value::Object(payload.clone()))?;
        assert_eq!(
            HarnessEvent::parse(&e.line()).as_ref(),
            Ok(&e),
            "{} does not read back",
            tag.name
        );
        Ok(e.line())
    }
}

/// A value of the wrong kind for a field's type, and the path the refusal must name.
fn bad_value(ty: &str, guest: bool, field: &str, v: &Vocabulary) -> (Value, String) {
    if let Some(inner) = ty.strip_prefix("list<").and_then(|t| t.strip_suffix('>')) {
        let (item, _) = bad_value(inner, guest, field, v);
        return (Value::Array(vec![item]), format!("{field}[0]"));
    }
    let bad = match (ty, guest) {
        (t, _) if t.starts_with("enum:") => Value::from("not_a_value"),
        ("sha256", _) => Value::from("sha256:XYZ"),
        ("id", _) => Value::from(""),
        ("contract_id", _) => Value::from("nodots"),
        ("version", _) => Value::from("nine"),
        ("u64", false) => Value::from(1u64 << 53),
        ("i64", false) => Value::from(-(1i64 << 53)),
        ("u64", true) => Value::from(-1),
        ("i64", true) => Value::from(u64::MAX),
        ("float", _) => Value::from("0.5"),
        ("bool", _) => Value::from("yes"),
        ("text", _) => Value::from(1),
        _ => panic!("no bad value for type {ty} (vocabulary {})", v.version),
    };
    (bad, field.to_string())
}

#[test]
fn every_tag_reads_back_what_it_writes_with_and_without_its_optional_fields() {
    let v = vocabulary();
    for tag in v.tags.iter().filter(|t| t.is_live()) {
        for with_optional in [true, false] {
            read(tag, &sample_payload(tag, with_optional, &v)).unwrap_or_else(|e| panic!("{}: {e}", tag.name));
        }
    }
    assert_eq!(v.tags.len(), 62);
    assert_eq!(v.tags.iter().filter(|t| t.is_live()).count(), 38);
}

#[test]
fn every_deprecated_tag_is_refused_by_name_on_both_planes() {
    let v = vocabulary();
    let deprecated: Vec<&Tag> = v.deprecated_tags().collect();
    assert_eq!(deprecated.len(), 24);
    for tag in deprecated {
        let refused = if tag.is_guest() {
            GuestLine::parse(&format!("{{\"tag\": \"{}\"}}", tag.name)).unwrap_err()
        } else {
            HarnessEvent::parse(&format!("{{\"kind\": \"{}\", \"payload\": {{}}}}", tag.name)).unwrap_err()
        };
        assert_eq!(
            (refused.path.as_str(), refused.reason.as_str()),
            (
                if tag.is_guest() { "tag" } else { "kind" },
                format!("{:?} is deprecated in vocabulary 0.10", tag.name).as_str()
            )
        );
    }
}

#[test]
fn every_required_field_left_out_is_refused_by_name() {
    let v = vocabulary();
    let mut reach = 0;
    for tag in v.tags.iter().filter(|t| t.is_live()) {
        for field in &tag.required {
            let mut payload = sample_payload(tag, true, &v);
            payload.remove(field);
            let refused = read(tag, &payload).expect_err(&format!("{}.{field} missing was accepted", tag.name));
            assert_eq!(
                (refused.path.as_str(), refused.reason.as_str()),
                (format!("{}.{field}", tag.name).as_str(), "missing")
            );
            reach += 1;
        }
    }
    assert!(reach > 0);
}

#[test]
fn an_unknown_field_is_refused_on_every_tag() {
    let v = vocabulary();
    for tag in v.tags.iter().filter(|t| t.is_live()) {
        let mut payload = sample_payload(tag, true, &v);
        payload.insert("not_a_field".to_string(), Value::from(1));
        let refused = read(tag, &payload).expect_err(&format!("{} took an unknown field", tag.name));
        assert_eq!(refused.path, format!("{}.not_a_field", tag.name));
    }
}

#[test]
fn every_field_refuses_a_value_outside_its_type_by_name() {
    let v = vocabulary();
    let mut reach = 0;
    for tag in v.tags.iter().filter(|t| t.is_live()) {
        for (field, _) in tag.fields() {
            let (bad, path) = bad_value(&tag.types[field], tag.is_guest(), field, &v);
            let mut payload = sample_payload(tag, true, &v);
            payload.insert(field.to_string(), bad.clone());
            let refused = read(tag, &payload).expect_err(&format!("{}.{field} took {bad}", tag.name));
            assert_eq!(refused.path, format!("{}.{path}", tag.name), "{}", refused.reason);
            reach += 1;
        }
    }
    let fields: usize = v
        .tags
        .iter()
        .filter(|t| t.is_live())
        .map(|t| t.required.len() + t.optional.len())
        .sum();
    assert_eq!(reach, fields);
}

#[test]
fn the_harness_plane_takes_its_integer_limit_and_the_guest_plane_the_full_64_bits() {
    use events::{Field, HarnessI64, HarnessU64};
    assert!(HarnessU64::new((1 << 53) - 1).is_ok() && HarnessU64::new(1 << 53).is_err());
    assert!(HarnessI64::new(-((1 << 53) - 1)).is_ok() && HarnessI64::new(-(1 << 53)).is_err());
    assert_eq!(u64::read_json(&Value::from(u64::MAX), "x"), Ok(u64::MAX));
    assert_eq!(i64::read_json(&Value::from(i64::MIN), "x"), Ok(i64::MIN));
}

#[test]
fn a_float_is_finite_and_a_whole_one_keeps_its_point() {
    use events::{Field, FiniteF64};
    assert!(FiniteF64::new(f64::NAN).is_err() && FiniteF64::new(f64::INFINITY).is_err());
    let mut out = String::new();
    FiniteF64::new(2.0).unwrap().write_json(&mut out);
    assert_eq!(out, "2.0");
}

#[test]
fn a_line_that_is_not_an_event_is_refused() {
    assert_eq!(GuestLine::parse("not json").unwrap_err().path, "line");
    assert_eq!(GuestLine::parse("{\"tag\": \"NOT_A_TAG\"}").unwrap_err().path, "tag");
    assert_eq!(
        HarnessEvent::parse("{\"kind\": \"SESSION_INIT\"}").unwrap_err().path,
        "payload"
    );
    assert_eq!(
        HarnessEvent::parse("{\"kind\": \"NOT_A_TAG\", \"payload\": {}}")
            .unwrap_err()
            .path,
        "kind"
    );
    assert_eq!(
        HarnessEvent::parse("{\"kind\": \"X\", \"payload\": {}, \"seq\": 1}")
            .unwrap_err()
            .path,
        "seq"
    );
}
