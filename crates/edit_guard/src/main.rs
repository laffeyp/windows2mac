// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! The hook: reads Claude Code's PreToolUse JSON on stdin and, for a Bash command `edit_guard::refusal` names, prints
//! the reason on stderr and exits 2, which blocks the call (code.claude.com/docs/en/hooks, "Exit code 2 behavior").
//! Any other input exits 0.

use std::io::Read;
use std::process::ExitCode;

fn main() -> ExitCode {
    let mut input = String::new();
    if std::io::stdin().read_to_string(&mut input).is_err() {
        return ExitCode::SUCCESS;
    }
    let Ok(event) = serde_json::from_str::<serde_json::Value>(&input) else {
        return ExitCode::SUCCESS;
    };
    if event["tool_name"] != "Bash" {
        return ExitCode::SUCCESS;
    }
    let Some(command) = event["tool_input"]["command"].as_str() else {
        return ExitCode::SUCCESS;
    };
    match edit_guard::refusal(command) {
        Some(why) => {
            eprintln!("edit_guard: refused, {why}");
            ExitCode::from(2)
        }
        None => ExitCode::SUCCESS,
    }
}
