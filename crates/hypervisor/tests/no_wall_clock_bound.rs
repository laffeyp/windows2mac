// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! No wall-clock upper bound in the correctness run (Sprint 224; reviewer-3's Medium 2 of its review at Sprint 222's
//! close). A test that asserts an elapsed time under a bound fails on the machine's load, not on the code, and stops the
//! tests behind it; the exit time's is ignored there and judged in the gates' timing stage. A lower bound ("at least
//! 50 ms", `stop_from_another_thread.rs`) cannot fail on load and is not named.

use std::fs;
use std::path::{Path, PathBuf};

/// The ignore attribute the timing stage's test carries.
const TIMING: &str = "#[ignore = \"a wall-clock bound";

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else { return };
    for e in entries.flatten() {
        let p = e.path();
        if p.is_dir() {
            rust_files(&p, out);
        } else if p.extension().is_some_and(|x| x == "rs") {
            out.push(p);
        }
    }
}

/// `text` less each function under the timing stage's ignore attribute, from the attribute's line to the function's
/// closing brace at the start of a line: the exemption covers that function, not its file (reviewer-3's Low 2 of its
/// review at Sprint 224).
fn without_timing_tests(text: &str) -> String {
    let mut out = String::new();
    let mut skipping = false;
    for line in text.lines() {
        if line.trim_start().starts_with(TIMING) {
            skipping = true;
        }
        if !skipping {
            out.push_str(line);
            out.push('\n');
        } else if line == "}" {
            skipping = false;
        }
    }
    out
}

/// The `assert!` calls of `text` that hold an elapsed time under a bound: a time (`elapsed()`, `duration_since(`, or a
/// variable bound from either) on the left of `<` or `<=`, or on the right of `>` or `>=`.
fn upper_bounds(text: &str) -> Vec<String> {
    let is_time_source = |s: &str| s.contains("elapsed()") || s.contains("duration_since(");
    let timed: Vec<&str> = text
        .lines()
        .filter(|l| is_time_source(l))
        .filter_map(|l| l.trim_start().strip_prefix("let ")?.split([' ', ':']).next())
        .collect();
    let names_time = |s: &str| {
        let words: Vec<&str> = s.split(|c: char| !(c.is_alphanumeric() || c == '_')).collect();
        is_time_source(s) || timed.iter().any(|t| words.contains(t))
    };
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(i) = rest.find("assert!(") {
        let body = &rest[i..];
        let end = body.find(");").map_or(body.len(), |j| j + 2);
        let call = &body[..end];
        // the condition: the first argument, to its comma at depth 0
        let inner = &call["assert!(".len()..];
        let (mut depth, mut cut) = (0i32, inner.len());
        for (k, c) in inner.char_indices() {
            match c {
                '(' | '[' | '{' => depth += 1,
                ')' | ']' | '}' => depth -= 1,
                ',' if depth == 0 => {
                    cut = k;
                    break;
                }
                _ => {}
            }
        }
        let cond = &inner[..cut];
        let upper = [(" <= ", true), (" >= ", false), (" < ", true), (" > ", false)]
            .iter()
            .find_map(|(op, left)| {
                cond.split_once(op)
                    .map(|(l, r)| if *left { names_time(l) } else { names_time(r) })
            });
        if upper == Some(true) {
            out.push(call.split_whitespace().collect::<Vec<_>>().join(" "));
        }
        rest = &body[end..];
    }
    out
}

#[test]
fn no_correctness_test_holds_an_elapsed_time_under_a_bound() {
    let crates = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut files = Vec::new();
    rust_files(&crates, &mut files);
    files.sort();
    let mut found = Vec::new();
    // this file's fixtures quote upper bounds as strings; it holds no timing test of its own
    files.retain(|f| f.file_name().is_none_or(|n| n != "no_wall_clock_bound.rs"));
    for f in files {
        let text = without_timing_tests(&fs::read_to_string(&f).unwrap());
        for call in upper_bounds(&text) {
            found.push(format!("{}: {call}", f.display()));
        }
    }
    assert!(
        found.is_empty(),
        "wall-clock upper bounds in the correctness run: {found:#?}"
    );
}

#[test]
fn an_upper_bound_is_named_and_a_lower_bound_is_not() {
    let upper = "let ns = started.elapsed().as_nanos();\n    assert!(ns < 5_000.0, \"slow\");\n";
    assert_eq!(upper_bounds(upper).len(), 1);
    let reversed = "    assert!(Duration::from_secs(1) > started.elapsed(), \"slow\");\n";
    assert_eq!(upper_bounds(reversed).len(), 1, "a bound on the left of >");
    let since = "let d = now.duration_since(t0);\n    assert!(d <= limit);\n";
    assert_eq!(upper_bounds(since).len(), 1, "duration_since");
    let lower = "    assert!(\n        started.elapsed() >= Duration::from_millis(50),\n        \"early\"\n    );\n";
    assert!(upper_bounds(lower).is_empty());
    let lower_reversed = "    assert!(Duration::from_millis(50) <= started.elapsed());\n";
    assert!(upper_bounds(lower_reversed).is_empty());
}

#[test]
fn only_the_ignored_function_is_exempt() {
    let text = format!(
        "{TIMING}: x\"]\nfn timed() {{\n    assert!(t.elapsed() < b);\n}}\n\nfn other() {{\n    assert!(t.elapsed() < b);\n}}\n"
    );
    assert_eq!(upper_bounds(&without_timing_tests(&text)).len(), 1);
}
