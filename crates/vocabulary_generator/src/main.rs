// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! `vocabulary_generator [--check] [--root DIR] [--vocabulary FILE]` (Sprint 126).
//!
//! Reads the vocabulary Rust speaks (`signals/rust-current.json` names its version), checks it, and writes
//! `crates/events/src/generated.rs` and `harness/schemas/harness-events.json`. With `--check` it writes nothing and
//! exits 1 when either committed file differs from a fresh generation, or when a hand-written crate names a deprecated
//! tag. A vocabulary that breaks a rule is refused, each broken rule on standard error, exit 1. `--root` is the
//! repository (default: the working directory); `--vocabulary` reads another vocabulary file in place of the current
//! one.

use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::{env, fs};

use vocabulary_generator::{Vocabulary, check, current_vocabulary_path, render_rust, render_schema};

const RUST_OUT: &str = "crates/events/src/generated.rs";
const SCHEMA_OUT: &str = "harness/schemas/harness-events.json";

/// Every Rust source under `crates/*/src`, but the generated one, that names a deprecated tag as a whole word: a
/// deprecated tag has no emit site (vocabulary 0.10; Sprint 131), and one named in hand-written code is a site waiting.
fn deprecated_emit_sites(root: &Path, vocab: &Vocabulary) -> Result<Vec<String>, String> {
    let deprecated: Vec<&str> = vocab.deprecated_tags().map(|t| t.name.as_str()).collect();
    let mut sources = Vec::new();
    let crates = root.join("crates");
    let mut dirs: Vec<PathBuf> = match fs::read_dir(&crates) {
        Ok(entries) => entries.filter_map(|e| Some(e.ok()?.path().join("src"))).collect(),
        Err(_) => return Ok(Vec::new()),
    };
    while let Some(dir) = dirs.pop() {
        for entry in fs::read_dir(&dir).into_iter().flatten().flatten() {
            let path = entry.path();
            if path.is_dir() {
                dirs.push(path);
            } else if path.extension().is_some_and(|x| x == "rs") && !path.ends_with(RUST_OUT) {
                sources.push(path);
            }
        }
    }
    sources.sort();
    let is_word = |c: Option<char>| c.is_some_and(|c| c.is_ascii_alphanumeric() || c == '_');
    let mut faults = Vec::new();
    for path in sources {
        let text = fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        for tag in &deprecated {
            let named = text.match_indices(tag).any(|(i, _)| {
                !is_word(text[..i].chars().next_back()) && !is_word(text[i + tag.len()..].chars().next())
            });
            if named {
                let shown = path.strip_prefix(root).unwrap_or(&path).display().to_string();
                faults.push(format!(
                    "{shown}: names {tag}, deprecated in vocabulary {}",
                    vocab.version
                ));
            }
        }
    }
    Ok(faults)
}

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
    if check_only {
        for fault in deprecated_emit_sites(&root, &vocab)? {
            eprintln!("{fault}");
            fresh = false;
        }
    }
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
