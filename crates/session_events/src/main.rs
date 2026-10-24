// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! A w2m harness session's first and last events, SESSION_INIT and SESSION_COMPLETED, written to standard output in
//! substrate's line protocol (substrate `docs/process-producer.md`): one `{"kind": ..., "payload": {...}}` a line.
//! Substrate runs this binary as a Producer, checks each line against `harness/schemas/harness-events.json`, and
//! records it (Sprint 125: the first Rust program on a w2m record). Both events are the vocabulary's generated types
//! (Sprint 126), so a field the vocabulary lacks, or a value outside a field's type, does not build.
//!
//! `--plant-missing-field` takes `accepted_count` out of the written SESSION_COMPLETED line, the planted fault of
//! Sprint 125: substrate must fail the Producer and name the field. The types cannot leave a field out, so the plant
//! edits the line after it is written.

use std::io::{self, BufRead, Write};
use std::time::Instant;

use events::{
    FiniteF64, HarnessU64, Id, SessionCompleted, SessionInit, SessionOutcome, SessionPurpose, VOCAB_VERSION, Value,
    Version, harness_line,
};

const PROFILE: &str = "w2m-session-check";
const CONTRACT_VERSION: &str = "0.0";
const HARNESS_VERSION: &str = env!("CARGO_PKG_VERSION");

fn session_init() -> SessionInit {
    SessionInit {
        vocab_version: Version::new(VOCAB_VERSION).expect("the vocabulary's own version"),
        profile: Id::new(PROFILE).expect("a constant, not empty"),
        contract_version: Version::new(CONTRACT_VERSION).expect("a constant major.minor"),
        harness_version: Version::new(HARNESS_VERSION).expect("Cargo's package version is major.minor.patch"),
        purpose: SessionPurpose::Conformance,
    }
}

fn session_completed(elapsed_seconds: f64) -> SessionCompleted {
    let zero = HarnessU64::new(0).expect("0 is in range");
    SessionCompleted {
        outcome: SessionOutcome::Completed,
        discrepancy_count: zero,
        accepted_count: zero,
        elapsed_seconds: FiniteF64::new(elapsed_seconds).expect("a measured duration is finite"),
    }
}

/// The written line with one payload field taken out.
fn without_field(line: &str, field: &str) -> String {
    let mut value: Value = line.parse().expect("harness_line writes JSON");
    if let Some(payload) = value.get_mut("payload").and_then(Value::as_object_mut) {
        payload.remove(field);
    }
    value.to_string()
}

fn main() -> io::Result<()> {
    let started = Instant::now();
    let plant = std::env::args().skip(1).any(|a| a == "--plant-missing-field");
    // the Producer's input arrives as one JSON line; this session takes none, but the line is read so the pipe closes
    let mut input = String::new();
    io::stdin().lock().read_line(&mut input)?;
    let mut completed = harness_line(&session_completed(started.elapsed().as_secs_f64()));
    if plant {
        completed = without_field(&completed, "accepted_count");
    }
    let mut out = io::stdout().lock();
    writeln!(out, "{}", harness_line(&session_init()))?;
    writeln!(out, "{completed}")?;
    out.flush()
}

#[cfg(test)]
mod tests {
    use super::*;
    use events::{Event, HarnessEvent};

    #[test]
    fn session_init_is_one_line_with_its_five_fields() {
        assert_eq!(
            harness_line(&session_init()),
            format!(
                "{{\"kind\": \"SESSION_INIT\", \"payload\": {{\"vocab_version\": \"0.9\", \"profile\": \
                 \"w2m-session-check\", \"contract_version\": \"0.0\", \"harness_version\": \"{HARNESS_VERSION}\", \
                 \"purpose\": \"conformance\"}}}}"
            )
        );
    }

    #[test]
    fn elapsed_seconds_is_written_as_a_float_even_when_whole() {
        assert!(harness_line(&session_completed(2.0)).contains("\"elapsed_seconds\": 2.0"));
    }

    #[test]
    fn both_lines_read_back_as_the_events_written() {
        for event in [
            HarnessEvent::SessionInit(session_init()),
            HarnessEvent::SessionCompleted(session_completed(0.5)),
        ] {
            assert_eq!(HarnessEvent::parse(&event.line()), Ok(event));
        }
    }

    #[test]
    fn the_plant_leaves_out_accepted_count_and_nothing_else() {
        let planted = without_field(&harness_line(&session_completed(0.5)), "accepted_count");
        let payload: Value = planted.parse::<Value>().unwrap()["payload"].clone();
        let mut keys: Vec<&str> = payload.as_object().unwrap().keys().map(String::as_str).collect();
        keys.sort_unstable();
        assert_eq!(keys, ["discrepancy_count", "elapsed_seconds", "outcome"]);
        let refused = SessionCompleted::from_payload(&payload).unwrap_err();
        assert_eq!(refused.path, "SESSION_COMPLETED.accepted_count");
    }
}
