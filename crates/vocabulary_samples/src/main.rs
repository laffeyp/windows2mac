// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! `vocabulary_samples [--root DIR]` (Sprint 126): every harness-plane tag of the current vocabulary at sample
//! values, twice (optional fields present, then left out), read into its generated type and written by it in
//! substrate's line protocol. Substrate runs it as a Producer against `harness/schemas/harness-events.json`
//! (`harness/vocabulary-samples.toml`), so each kind's schema meets what its Rust type writes, the integers at
//! 2^53 - 1. A sample the generated type refuses is a fault of the generator: exit 1, naming the field.
//!
//! `--plant-trailing-newline` writes one SESSION_INIT whose `vocab_version` ends in a newline, which the Rust `Version`
//! refuses and so substrate's schema must too (its patterns end in `\Z`): the gates run it and expect the Producer failed.

use std::io::{self, BufRead, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use events::{HarnessEvent, Value};
use vocabulary_generator::{load_current, sample_payload};

fn run() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let (mut root, mut plant, mut plant_metric, mut plant_guest_trace) = (PathBuf::from("."), false, false, false);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--root" => root = args.next().map(PathBuf::from).ok_or("--root needs a directory")?,
            "--plant-trailing-newline" => plant = true,
            "--plant-unknown-metric" => plant_metric = true,
            "--plant-string-guest-trace" => plant_guest_trace = true,
            other => return Err(format!("unknown argument {other}")),
        }
    }
    // the Producer's input arrives as one JSON line; the samples take none, but the line is read so the pipe closes
    let mut input = String::new();
    io::stdin().lock().read_line(&mut input).map_err(|e| e.to_string())?;
    let vocab = load_current(&root)?;
    let mut out = io::stdout().lock();
    if plant_metric {
        // Sprint 162: a MEASUREMENT_RECORDED whose metric the vocabulary does not name; its Rust type must refuse it,
        // and the line, written past the type, substrate's schema must refuse too
        let tag = vocab
            .harness_tags()
            .find(|t| t.name == "MEASUREMENT_RECORDED")
            .ok_or("no MEASUREMENT_RECORDED")?;
        let mut payload = sample_payload(tag, true, &vocab);
        payload.insert("metric".to_string(), Value::from("exit_ns_p95"));
        let payload = Value::Object(payload);
        match HarnessEvent::from_kind("MEASUREMENT_RECORDED", &payload) {
            Ok(_) => return Err("the Rust type accepted metric exit_ns_p95".into()),
            Err(e) => eprintln!("vocabulary_samples: the Rust type refuses the plant: {e}"),
        }
        let line = format!("{{\"kind\": \"MEASUREMENT_RECORDED\", \"payload\": {payload}}}");
        writeln!(out, "{line}").map_err(|e| e.to_string())?;
        return out.flush().map_err(|e| e.to_string());
    }
    if plant_guest_trace {
        // Sprint 171 (reviewer-2's Medium 4): an EXECUTION_COMPLETED whose guest_trace is the string "false", where 0.12
        // types it bool (P22); its Rust type refuses it, and the line, written past the type, substrate's schema must
        // refuse too (Sprint 170's plant met the Rust type first and never reached the schema)
        let tag = vocab
            .harness_tags()
            .find(|t| t.name == "EXECUTION_COMPLETED")
            .ok_or("no EXECUTION_COMPLETED")?;
        let mut payload = sample_payload(tag, true, &vocab);
        payload.insert("guest_trace".to_string(), Value::from("false"));
        let payload = Value::Object(payload);
        match HarnessEvent::from_kind("EXECUTION_COMPLETED", &payload) {
            Ok(_) => return Err("the Rust type accepted guest_trace \"false\"".into()),
            Err(e) => eprintln!("vocabulary_samples: the Rust type refuses the plant: {e}"),
        }
        let line = format!("{{\"kind\": \"EXECUTION_COMPLETED\", \"payload\": {payload}}}");
        writeln!(out, "{line}").map_err(|e| e.to_string())?;
        return out.flush().map_err(|e| e.to_string());
    }
    if plant {
        let tag = vocab
            .harness_tags()
            .find(|t| t.name == "SESSION_INIT")
            .ok_or("no SESSION_INIT")?;
        let mut payload = sample_payload(tag, true, &vocab);
        payload.insert("vocab_version".to_string(), Value::from("0.10\n"));
        let line = format!(
            "{{\"kind\": \"SESSION_INIT\", \"payload\": {}}}",
            Value::Object(payload)
        );
        writeln!(out, "{line}").map_err(|e| e.to_string())?;
        return out.flush().map_err(|e| e.to_string());
    }
    for tag in vocab.harness_tags() {
        for with_optional in [true, false] {
            let payload = Value::Object(sample_payload(tag, with_optional, &vocab));
            let event = HarnessEvent::from_kind(&tag.name, &payload).map_err(|e| e.to_string())?;
            writeln!(out, "{}", event.line()).map_err(|e| e.to_string())?;
        }
    }
    // every `metric` value once more, so each one the vocabulary admits (P24's among them, Sprint 162) is read and
    // written by MEASUREMENT_RECORDED's type and met by substrate's schema
    let measurement = vocab
        .harness_tags()
        .find(|t| t.name == "MEASUREMENT_RECORDED")
        .ok_or("no MEASUREMENT_RECORDED")?;
    for metric in vocab.enums.get("metric").ok_or("no metric enum")? {
        let mut payload = sample_payload(measurement, true, &vocab);
        payload.insert("metric".to_string(), Value::from(metric.as_str()));
        let event =
            HarnessEvent::from_kind("MEASUREMENT_RECORDED", &Value::Object(payload)).map_err(|e| e.to_string())?;
        writeln!(out, "{}", event.line()).map_err(|e| e.to_string())?;
    }
    // an execution judged by its end state (P22, Sprint 170): `guest_trace` false, which the samples' bools (true)
    // never write, read and written by EXECUTION_COMPLETED's type and met by substrate's schema
    let completed = vocab
        .harness_tags()
        .find(|t| t.name == "EXECUTION_COMPLETED")
        .ok_or("no EXECUTION_COMPLETED")?;
    let mut payload = sample_payload(completed, true, &vocab);
    payload.insert("guest_trace".to_string(), Value::from(false));
    let event = HarnessEvent::from_kind("EXECUTION_COMPLETED", &Value::Object(payload)).map_err(|e| e.to_string())?;
    writeln!(out, "{}", event.line()).map_err(|e| e.to_string())?;
    out.flush().map_err(|e| e.to_string())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("vocabulary_samples: {e}");
            ExitCode::FAILURE
        }
    }
}
