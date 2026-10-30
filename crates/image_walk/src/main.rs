// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! `image_walk <image> [--blocks <file> --base <hex>]` (Sprint 253): the walk's counts as JSON, the frozen census's
//! names where it has them; with an exit ledger's blocks file (`<start>\t<end>` in hex, `Ledger::blocks`) and the base
//! the run loaded the image at, the cover: the run's distinct block starts inside the image, and how many of them are
//! the walk's instructions and how many its block starts.

use std::collections::BTreeSet;
use std::process::ExitCode;

fn run() -> Result<(), String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let path = args
        .first()
        .ok_or("image_walk <image> [--blocks <file> --base <hex>]")?;
    let data = std::fs::read(path).map_err(|e| format!("{path}: {e}"))?;
    let img = pe_image::parse(&data, false).map_err(|e| format!("{path}: {e:?}"))?;
    let w = image_walk::walk(&img, &data);
    let mut out = serde_json::json!({
        "file": std::path::Path::new(path).file_name().map(|n| n.to_string_lossy().to_string()),
        "reachable_instructions": w.insns.len(),
        "walked": w.insns.len() - w.swept_only.len(),
        "swept_only": w.swept_only.len(),
        "jump_tables": w.tables.len(),
        "jump_table_entries": w.tables.values().map(Vec::len).sum::<usize>(),
        "block_starts": w.block_starts.len(),
        "undecodable": w.undecodable.len(),
        "undecodable_first": w.undecodable.iter().take(12).map(|(r, e)| format!("0x{r:x} {e}")).collect::<Vec<_>>(),
    });
    let flag = |name: &str| args.iter().position(|a| a == name).and_then(|i| args.get(i + 1));
    if let (Some(blocks), Some(base)) = (flag("--blocks"), flag("--base")) {
        let base = u64::from_str_radix(base.trim_start_matches("0x"), 16).map_err(|e| format!("--base: {e}"))?;
        let size = u64::from(img.size_of_image);
        let text = std::fs::read_to_string(blocks).map_err(|e| format!("{blocks}: {e}"))?;
        let starts: BTreeSet<u32> = text
            .lines()
            .filter_map(|l| l.split('\t').next())
            .filter_map(|s| u64::from_str_radix(s, 16).ok())
            .filter(|s| (base..base + size).contains(s))
            .map(|s| (s - base) as u32)
            .collect();
        out["run_starts"] = starts.len().into();
        out["run_starts_walked"] = starts.iter().filter(|s| w.insns.contains_key(s)).count().into();
        out["run_starts_block_starts"] = starts.iter().filter(|s| w.block_starts.contains(s)).count().into();
    }
    println!("{}", serde_json::to_string_pretty(&out).map_err(|e| e.to_string())?);
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("image_walk: {e}");
            ExitCode::FAILURE
        }
    }
}
