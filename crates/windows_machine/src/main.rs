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
//! windows_machine batch [<family> | <family>@<run> ...] [--repeat N] [--pin performance|efficiency]
//!                       [--window hidden|minimized|normal]
//! windows_machine game [--game G] [--exe E] [--executions N] [--seconds S] [--mode batch|play] [--audio on|off]
//!                      [--input NAME] [--record] [--person] [--capture on|off] [--initial-state S] [--plant-state]
//! ```
//!
//! `probe` builds `probes/<probe>` on the laptop from the committed tree; `binary` runs a family's stored binary
//! unchanged (its first run in `store/family_runs/known-2026-10-29.tsv`, or `--run`'s). `--repeat` defaults to 2, as the
//! Python's did. A run that comes back whole with every execution's exit 0 is stored; `store` stores a run already in
//! `build/probe-runs/`. The repository is the one the working directory is in.
//!
//! `batch` (Sprint 372) runs every distinct stored binary of each family named, from the family tables, or a stored
//! run's binary named `<family>@<run>`, in one job, and stores each binary's run when the whole batch came back with
//! every execution's exit 0; no family, every family the tables hold. `game` runs the game `--executions` times
//! (default 10) for `--seconds` each (default 180), stored when every execution came back; the Python's `--pmc` and
//! `--pmc-sample` are refused, their summaries having run Python on the laptop.

use std::path::PathBuf;
use std::process::ExitCode;

use windows_machine::{Game, Kind, Request, batch_items, git, run_batch, run_game, run_job, store, stored_run};

const USAGE: &str = "usage: windows_machine probe <probe> [--repeat N] [--pin performance|efficiency]\n       \
                     windows_machine binary <family> [--run <stored run id>] [--repeat N] [--pin performance|efficiency]\n       \
                     windows_machine store <run>\n       \
                     windows_machine batch [<family> | <family>@<run> ...] [--repeat N] [--pin performance|efficiency] \
                     [--window hidden|minimized|normal]\n       \
                     windows_machine game [--game G] [--exe E] [--executions N] [--seconds S] [--mode batch|play] \
                     [--audio on|off] [--input NAME] [--record] [--person] [--capture on|off] [--initial-state S] \
                     [--plant-state]";

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
    if kind == "batch" {
        return batch(&root, rest);
    }
    if kind == "game" {
        return game(&root, rest);
    }
    let kind = match kind {
        "probe" => Kind::Probe,
        "binary" => Kind::Binary,
        other => return usage(&format!("no kind {other}: probe, binary, store, batch or game")),
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

fn status(code: i32) -> ExitCode {
    match code {
        0 => ExitCode::SUCCESS,
        code => ExitCode::from(u8::try_from(code.clamp(1, 255)).unwrap_or(1)),
    }
}

/// `batch [<family> | <family>@<run> ...] [--repeat N] [--pin ...] [--window ...]`.
fn batch(root: &std::path::Path, rest: &[String]) -> ExitCode {
    let (mut names, mut repeat, mut pin, mut window) = (Vec::new(), 2u32, None, "hidden".to_string());
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
            "--window" => match it.next().map(String::as_str) {
                Some(w @ ("hidden" | "minimized" | "normal")) => window = w.to_string(),
                _ => return usage("--window takes hidden, minimized or normal"),
            },
            flag if flag.starts_with("--") => return usage(&format!("no option {flag}")),
            _ => names.push(a.clone()),
        }
    }
    let items = match batch_items(root, &names) {
        Ok(items) => items,
        Err(why) => {
            eprintln!("windows_machine: {why}");
            return ExitCode::from(1);
        }
    };
    let mut families: Vec<&str> = items.iter().map(|i| i.family.as_str()).collect();
    families.dedup();
    println!("batch: {} binaries of {} families", items.len(), families.len());
    status(run_batch(root, items, repeat, pin, &window))
}

/// `game [--game G] [--exe E] [--executions N] [--seconds S] [--mode ...] [--audio ...] [--input NAME] [--record]
/// [--person] [--capture ...] [--initial-state S] [--plant-state]`.
fn game(root: &std::path::Path, rest: &[String]) -> ExitCode {
    let mut g = Game::default();
    let mut it = rest.iter();
    while let Some(a) = it.next() {
        let mut value = |what: &str| it.next().cloned().ok_or(format!("{a} takes {what}"));
        let set = match a.as_str() {
            "--game" => value("a game").map(|v| g.game = v),
            "--exe" => value("the game's exe name").map(|v| g.exe = v),
            "--input" => value("an input script's name").map(|v| g.input_script = Some(v)),
            "--initial-state" => value("a state of games/<game>/state.json").map(|v| g.initial_state = v),
            "--executions" => match value("a count").map(|v| v.parse::<u32>()) {
                Ok(Ok(n)) if n > 0 => {
                    g.executions = n;
                    Ok(())
                }
                _ => Err("--executions takes a count of 1 or more".to_string()),
            },
            "--seconds" => match value("seconds").map(|v| v.parse::<u32>()) {
                Ok(Ok(n)) if n > 0 => {
                    g.seconds = n;
                    Ok(())
                }
                _ => Err("--seconds takes a count of 1 or more".to_string()),
            },
            "--mode" => match value("batch or play").as_deref() {
                Ok(m @ ("batch" | "play")) => {
                    g.mode = m.to_string();
                    Ok(())
                }
                _ => Err("--mode takes batch or play".to_string()),
            },
            "--audio" | "--capture" => match value("on or off").as_deref() {
                Ok(v @ ("on" | "off")) => {
                    if a == "--audio" {
                        g.audio = v.to_string();
                    } else {
                        g.capture = v.to_string();
                    }
                    Ok(())
                }
                _ => Err(format!("{a} takes on or off")),
            },
            "--record" => {
                g.record = true;
                Ok(())
            }
            "--person" => {
                g.person = true;
                Ok(())
            }
            "--plant-state" => {
                g.plant_state = true;
                Ok(())
            }
            "--pmc" | "--pmc-sample" => Err(format!(
                "{a} is refused: its summary ran Python on the laptop (pmc_summary.py, pmc_images.py), and no Python \
                 runs (Decision 2026-10-29)"
            )),
            other => Err(format!("no option {other}")),
        };
        if let Err(why) = set {
            return usage(&why);
        }
    }
    status(run_game(root, &g))
}
