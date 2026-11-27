// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! `windows_machine` (Sprint 371): a probe, or a stored binary, run on the Windows laptop, its results brought back and
//! stored; the `probe` and `binary` kinds of `scripts/windows_machine.py` and the store step of
//! `scripts/store_probe_runs.py` at the tag frozen-python-2026-10-29, with no Python.
//!
//! ```text
//! windows_machine probe <probe> [--repeat N] [--pin performance|efficiency]
//! windows_machine binary <family> [--run <stored run id>] [--repeat N] [--pin performance|efficiency]
//! windows_machine store <run>
//! ```
//!
//! `probe` builds `probes/<probe>` on the laptop from the committed tree; `binary` runs a family's stored binary
//! unchanged (its first run in `store/family_runs/known-2026-10-29.tsv`, or `--run`'s). `--repeat` defaults to 2, as the
//! Python's did. A run that comes back whole with every execution's exit 0 is stored; `store` stores a run already in
//! `build/probe-runs/`. The repository is the one the working directory is in.

use std::path::PathBuf;
use std::process::ExitCode;

use windows_machine::{Kind, Request, git, run_job, store, stored_run};

const USAGE: &str = "usage: windows_machine probe <probe> [--repeat N] [--pin performance|efficiency]\n       \
                     windows_machine binary <family> [--run <stored run id>] [--repeat N] [--pin performance|efficiency]\n       \
                     windows_machine store <run>";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let root = match git(
        &std::env::current_dir().unwrap_or_default(),
        &["rev-parse", "--show-toplevel"],
    ) {
        Ok(out) => PathBuf::from(String::from_utf8_lossy(&out).trim()),
        Err(why) => {
            eprintln!("windows_machine: not in the repository: {why}");
            return ExitCode::from(1);
        }
    };
    let (kind, rest) = match args.split_first() {
        Some((k, rest)) => (k.as_str(), rest),
        None => return usage("no kind"),
    };
    if kind == "store" {
        return match rest {
            [run] => match store(&root, run) {
                Ok(line) => {
                    println!("{line}");
                    ExitCode::SUCCESS
                }
                Err(why) => {
                    eprintln!("windows_machine: {why}");
                    ExitCode::from(1)
                }
            },
            _ => usage("store takes one run id"),
        };
    }
    let kind = match kind {
        "probe" => Kind::Probe,
        "binary" => Kind::Binary,
        other => return usage(&format!("no kind {other}: probe, binary or store")),
    };
    let (mut name, mut repeat, mut pin, mut run) = (None, 2u32, None, None);
    let mut it = rest.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--repeat" => match it.next().and_then(|n| n.parse().ok()) {
                Some(n) if n > 0 => repeat = n,
                _ => return usage("--repeat takes a count of 1 or more"),
            },
            "--pin" => match it.next().map(String::as_str) {
                Some(c @ ("performance" | "efficiency")) => pin = Some(c.to_string()),
                _ => return usage("--pin takes performance or efficiency"),
            },
            "--run" if kind == Kind::Binary => match it.next() {
                Some(id) => run = Some(id.clone()),
                None => return usage("--run takes a stored run id"),
            },
            flag if flag.starts_with("--") => return usage(&format!("no option {flag}")),
            _ if name.is_none() => name = Some(a.clone()),
            _ => return usage("one probe or family"),
        }
    }
    let Some(name) = name else {
        return usage("no probe or family named");
    };
    let request = match kind {
        Kind::Probe => Request {
            kind,
            probe: name,
            binary: None,
            dll: None,
            repeat,
            pin,
        },
        Kind::Binary => match stored_run(&root, &name, run.as_deref()) {
            Ok(r) => {
                println!("{name}: run {}'s binary {} (probe {})", r.run, r.binary, r.probe);
                Request {
                    kind,
                    probe: r.probe,
                    binary: Some(r.binary),
                    dll: r.dll,
                    repeat,
                    pin,
                }
            }
            Err(why) => {
                eprintln!("windows_machine: {why}");
                return ExitCode::from(1);
            }
        },
    };
    match run_job(&root, &request) {
        0 => ExitCode::SUCCESS,
        code => ExitCode::from(u8::try_from(code.clamp(1, 255)).unwrap_or(1)),
    }
}

fn usage(why: &str) -> ExitCode {
    eprintln!("windows_machine: {why}\n{USAGE}");
    ExitCode::from(2)
}
