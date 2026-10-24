// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! `vocabulary_generator [--check] [--root DIR] [--vocabulary FILE]` (Sprint 126).
//!
//! Reads the current vocabulary (`signals/current.json` names its version), checks it, and writes
//! `crates/events/src/generated.rs` and `harness/schemas/harness-events.json`. With `--check` it writes nothing and
//! exits 1 when either committed file differs from a fresh generation. A vocabulary that breaks a rule is refused,
//! each broken rule on standard error, exit 1. `--root` is the repository (default: the working directory);
//! `--vocabulary` reads another vocabulary file in place of the current one.

use std::path::PathBuf;
use std::process::ExitCode;
use std::{env, fs};

use vocabulary_generator::{Vocabulary, check, current_vocabulary_path, render_rust, render_schema};

const RUST_OUT: &str = "crates/events/src/generated.rs";
const SCHEMA_OUT: &str = "harness/schemas/harness-events.json";

fn run() -> Result<bool, String> {
    let mut args = env::args().skip(1);
    let (mut check_only, mut root, mut vocabulary) = (false, PathBuf::from("."), None);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--check" => check_only = true,
            "--root" => root = args.next().map(PathBuf::from).ok_or("--root needs a directory")?,
            "--vocabulary" => vocabulary = Some(args.next().map(PathBuf::from).ok_or("--vocabulary needs a file")?),
            other => return Err(format!("unknown argument {other}")),
        }
    }
    let path = match vocabulary {
        Some(p) => p,
        None => current_vocabulary_path(&root)?,
    };
    let text = fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let vocab = Vocabulary::parse(&text).map_err(|errs| format!("{}: {}", path.display(), errs.join("; ")))?;
    let (errors, summary) = check(&vocab);
    println!("{}: {summary}", path.display());
    if !errors.is_empty() {
        for e in &errors {
            eprintln!("{e}");
        }
        return Ok(false);
    }
    let mut fresh = true;
    for (out, text) in [(RUST_OUT, render_rust(&vocab)), (SCHEMA_OUT, render_schema(&vocab))] {
        let target = root.join(out);
        if check_only {
            if fs::read_to_string(&target).ok().as_deref() == Some(text.as_str()) {
                println!("{out}: up to date");
            } else {
                eprintln!(
                    "stale: {out} differs from a fresh generation; run `cargo run --release -p vocabulary_generator`"
                );
                fresh = false;
            }
        } else {
            fs::write(&target, text).map_err(|e| format!("{}: {e}", target.display()))?;
            println!("wrote {out}");
        }
    }
    Ok(fresh)
}

fn main() -> ExitCode {
    match run() {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(e) => {
            eprintln!("vocabulary_generator: {e}");
            ExitCode::FAILURE
        }
    }
}
