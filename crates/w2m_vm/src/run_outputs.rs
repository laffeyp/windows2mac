// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! The files a `w2m-vm` run writes, in one place (Sprint 289; reviewer-3's Low 1 at 288): the flags that name them,
//! the files the game run writes beside its stop under its name, and their clearing at a run's start, so a stamp
//! (`built_from`) stands only beside files the run that wrote it wrote.

use std::path::{Path, PathBuf};

/// The flags whose next argument names a file the run writes, stamped beside it (Sprint 288).
pub const WRITTEN: [&str; 4] = ["--output", "--stop", "--exit-ledger", "--judge-blocks"];

/// What the game run writes beside its stop file, each the stop's name less `.tsv` and this; the stop, written after
/// them, stands for them, its stamp theirs.
pub const STOP_COMPANIONS: [&str; 4] = [".unity.log", ".calls.tsv", ".calls.strings.tsv", ".in-flight.tsv"];

/// The companion `suffix` (one of `STOP_COMPANIONS`) of the stop file `stop`.
pub fn companion(stop: &str, suffix: &str) -> String {
    debug_assert!(STOP_COMPANIONS.contains(&suffix), "{suffix} is no stop companion");
    format!("{}{suffix}", stop.trim_end_matches(".tsv"))
}

/// The value after `flag` in `args`, as given.
fn flagged<'a>(args: &'a [String], flag: &str) -> Option<&'a str> {
    args.windows(2).find(|w| w[0] == flag).map(|w| w[1].as_str())
}

/// The files `args` tell a run to write, each from `root`.
pub fn written(root: &Path, args: &[String]) -> Vec<PathBuf> {
    args.windows(2)
        .filter(|w| WRITTEN.contains(&w[0].as_str()))
        .map(|w| root.join(&w[1]))
        .collect()
}

/// Removes `f`, there or not.
fn gone(f: &Path) -> Result<(), String> {
    match std::fs::remove_file(f) {
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(format!("{}: {e}", f.display())),
        _ => Ok(()),
    }
}

/// At a run's start, every file of an earlier run that this run's stamps would stand beside: each file `args` name
/// with its stamp, the stop's companions, and an exit ledger's children's ledgers, `<ledger>.<pid>`.
pub fn clear(root: &Path, args: &[String]) -> Result<(), String> {
    for file in written(root, args) {
        gone(&file)?;
        crate::built_from::unstamp(&file)?;
    }
    if let Some(stop) = flagged(args, "--stop") {
        for suffix in STOP_COMPANIONS {
            gone(&root.join(companion(stop, suffix)))?;
        }
    }
    if let Some(ledger) = flagged(args, "--exit-ledger").map(|l| root.join(l))
        && let (Some(dir), Some(name)) = (ledger.parent(), ledger.file_name())
    {
        let children = format!("{}.", name.to_string_lossy());
        for entry in std::fs::read_dir(dir).into_iter().flatten().flatten() {
            let n = entry.file_name().to_string_lossy().to_string();
            if n.strip_prefix(&children)
                .is_some_and(|pid| !pid.is_empty() && pid.bytes().all(|b| b.is_ascii_digit()))
            {
                gone(&entry.path())?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{STOP_COMPANIONS, clear, companion};

    /// `clear` leaves nothing an earlier run wrote that this run's stamps would stand beside, and keeps what no flag
    /// names: a stop, its four companions, each named file's stamp, an exit ledger, two children's ledgers, and the
    /// ledger's `.blocks` and a file no flag names, which stay.
    #[test]
    fn a_runs_start_clears_an_earlier_runs_files() {
        let root = std::env::temp_dir().join(format!("w2m-run-outputs-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("b")).unwrap();
        std::fs::create_dir_all(root.join("l")).unwrap();
        let mut files = vec![
            "b/stop.tsv",
            "b/out.txt",
            "b/stop.tsv.tree",
            "b/out.txt.tree",
            "l/rust.tsv",
        ];
        files.extend([
            "l/rust.tsv.tree",
            "l/rust.tsv.4242",
            "l/rust.tsv.17",
            "l/rust.tsv.blocks",
            "b/other.tsv",
        ]);
        let companions: Vec<String> = STOP_COMPANIONS.iter().map(|s| companion("b/stop.tsv", s)).collect();
        for f in files.iter().copied().chain(companions.iter().map(String::as_str)) {
            std::fs::write(root.join(f), "an earlier run's\n").unwrap();
        }
        let args: Vec<String> = ["--game", "d", "g.exe", "--output", "b/out.txt", "--stop", "b/stop.tsv"]
            .into_iter()
            .chain(["--exit-ledger", "l/rust.tsv"])
            .map(String::from)
            .collect();
        clear(&root, &args).unwrap();
        let mut left: Vec<String> = ["b", "l"]
            .iter()
            .flat_map(|d| std::fs::read_dir(root.join(d)).unwrap())
            .map(|e| {
                e.unwrap()
                    .path()
                    .strip_prefix(&root)
                    .unwrap()
                    .to_string_lossy()
                    .to_string()
            })
            .collect();
        left.sort();
        assert_eq!(left, ["b/other.tsv", "l/rust.tsv.blocks"]);
        assert_eq!(companions[1], "b/stop.calls.tsv");
        let _ = std::fs::remove_dir_all(&root);
    }
}
