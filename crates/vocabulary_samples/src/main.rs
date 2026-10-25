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
    let (mut root, mut plant) = (PathBuf::from("."), false);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--root" => root = args.next().map(PathBuf::from).ok_or("--root needs a directory")?,
            "--plant-trailing-newline" => plant = true,
            other => return Err(format!("unknown argument {other}")),
        }
    }
    // the Producer's input arrives as one JSON line; the samples take none, but the line is read so the pipe closes
    let mut input = String::new();
    io::stdin().lock().read_line(&mut input).map_err(|e| e.to_string())?;
    let vocab = load_current(&root)?;
    let mut out = io::stdout().lock();
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
