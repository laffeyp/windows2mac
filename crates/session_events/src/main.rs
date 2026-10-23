// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! A w2m harness session's first and last events, SESSION_INIT and SESSION_COMPLETED (vocabulary 0.9), written to
//! standard output in substrate's line protocol (substrate `docs/process-producer.md`): one
//! `{"kind": ..., "payload": {...}}` a line. Substrate runs this binary as a Producer, checks each line against
//! `harness/schemas/session.json`, and records it (Sprint 125: the first Rust program on a w2m record).
//!
//! `--plant-missing-field` leaves `accepted_count` out of SESSION_COMPLETED, the sprint's planted fault: substrate
//! must fail the Producer and name the field.

use std::io::{self, BufRead, Write};
use std::time::Instant;

const VOCAB_VERSION: &str = "0.9";
const PROFILE: &str = "w2m-session-check";
const CONTRACT_VERSION: &str = "0.0";
const HARNESS_VERSION: &str = env!("CARGO_PKG_VERSION");

/// A JSON string literal: the quotes, and the escapes RFC 8259 section 7 requires.
fn json_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
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
    out
}

/// One event line: the kind and the payload's fields, each already a JSON value.
fn event_line(kind: &str, fields: &[(&str, String)]) -> String {
    let payload: Vec<String> = fields
        .iter()
        .map(|(k, v)| format!("{}: {}", json_string(k), v))
        .collect();
    format!(
        "{{\"kind\": {}, \"payload\": {{{}}}}}",
        json_string(kind),
        payload.join(", ")
    )
}

fn session_init() -> String {
    event_line(
        "SESSION_INIT",
        &[
            ("vocab_version", json_string(VOCAB_VERSION)),
            ("profile", json_string(PROFILE)),
            ("contract_version", json_string(CONTRACT_VERSION)),
            ("harness_version", json_string(HARNESS_VERSION)),
            ("purpose", json_string("conformance")),
        ],
    )
}

fn session_completed(elapsed_seconds: f64, plant_missing_field: bool) -> String {
    let mut fields = vec![
        ("outcome", json_string("completed")),
        ("discrepancy_count", "0".to_string()),
        ("accepted_count", "0".to_string()),
        ("elapsed_seconds", format!("{elapsed_seconds:?}")),
    ];
    if plant_missing_field {
        fields.retain(|(k, _)| *k != "accepted_count");
    }
    event_line("SESSION_COMPLETED", &fields)
}

fn main() -> io::Result<()> {
    let started = Instant::now();
    let plant = std::env::args().skip(1).any(|a| a == "--plant-missing-field");
    // the Producer's input arrives as one JSON line; this session takes none, but the line is read so the pipe closes
    let mut input = String::new();
    io::stdin().lock().read_line(&mut input)?;
    let mut out = io::stdout().lock();
    writeln!(out, "{}", session_init())?;
    writeln!(out, "{}", session_completed(started.elapsed().as_secs_f64(), plant))?;
    out.flush()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_string_with_quotes_and_controls_is_escaped() {
        assert_eq!(json_string("a\"b\\c\n\u{1}"), "\"a\\\"b\\\\c\\n\\u0001\"");
    }

    #[test]
    fn session_init_is_one_line_with_its_five_fields() {
        let line = session_init();
        assert!(!line.contains('\n'));
        assert_eq!(
            line,
            format!(
                "{{\"kind\": \"SESSION_INIT\", \"payload\": {{\"vocab_version\": \"0.9\", \"profile\": \
                 \"w2m-session-check\", \"contract_version\": \"0.0\", \"harness_version\": \"{HARNESS_VERSION}\", \
                 \"purpose\": \"conformance\"}}}}"
            )
        );
    }

    #[test]
    fn elapsed_seconds_is_written_as_a_float_even_when_whole() {
        assert!(session_completed(2.0, false).contains("\"elapsed_seconds\": 2.0"));
    }

    #[test]
    fn the_plant_leaves_out_accepted_count_and_nothing_else() {
        let planted = session_completed(0.5, true);
        assert!(!planted.contains("accepted_count"));
        assert!(planted.contains("discrepancy_count") && planted.contains("outcome"));
    }
}
