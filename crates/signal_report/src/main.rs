// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! `signal_report NNN [--write] [--backfill YYYY-MM-DD] [--root DIR]`, and `signal_report --check [--root DIR]`
//! (Sprint 127). A sprint's Signal Report from its records, in the kit's template (`sdd-kit-3/templates/
//! SIGNAL_REPORT.md`), as `scripts/signal_report.py` wrote it from Sprint 111: the card's scope, the sprint's commits,
//! the signal trace, and the card's Emits checked tag by tag against what fired. The self-grade and the Rubber Duck
//! observations are the BLACKBOARD tail entry's, which it cites.
//!
//! A record belongs to sprint NNN when its first frame's baseline names, as `w2m_commit`, a commit whose subject
//! begins "Sprint NNN"; or, when its baseline names no commit (`substrate run --file`), when one of those commits added
//! it. Records are read here, frame by frame (`events-*.jsonl`, one JSON object a line), for their first frame and the
//! count of each kind; each single record's prose is `substrate narrate`'s; a frame's integrity is
//! `substrate replay`'s, which the gates run over every record.
//!
//! `--check` fails when a sprint from 108 on whose card says `status: done` has no report: the reports stopped
//! unnoticed after 048, around 108, and after 122 (hand-author log 2026-10-24).

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use serde_json::Value;

/// Prefixes of records made by the hundred a sprint: counted, not narrated one by one.
const GROUPED: [&str; 2] = ["class-", "family-"];
/// The first sprint whose close owes a report (108 to 110 were written at 111; hand-author log 2026-10-17).
const FIRST_REPORTED_SPRINT: u32 = 108;
/// A narrated line is cut to this many characters, as the Python report cut it.
const NARRATION_WIDTH: usize = 220;

/// Counts in the order each key was first seen, as Python's Counter keeps them.
#[derive(Default)]
struct Counts(Vec<(String, u64)>);

impl Counts {
    fn add(&mut self, key: &str, n: u64) {
        match self.0.iter_mut().find(|(k, _)| k == key) {
            Some((_, c)) => *c += n,
            None => self.0.push((key.to_string(), n)),
        }
    }

    fn get(&self, key: &str) -> u64 {
        self.0.iter().find(|(k, _)| k == key).map_or(0, |(_, c)| *c)
    }
}

/// One record's frames, in seq order: sealed segments by number, then the open one. A line that is not a JSON object
/// (a torn tail) ends the segment, as substrate's reader stops at it.
fn frames(record: &Path) -> Vec<Value> {
    let mut sealed = Vec::new();
    let mut open = Vec::new();
    for entry in fs::read_dir(record).into_iter().flatten().flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with("events-") && name.ends_with(".open.jsonl") {
            open.push(entry.path());
        } else if name.starts_with("events-") && name.ends_with(".jsonl") {
            sealed.push(entry.path());
        }
    }
    sealed.sort();
    open.sort();
    let mut out = Vec::new();
    for segment in sealed.into_iter().chain(open) {
        let Ok(text) = fs::read_to_string(&segment) else {
            continue;
        };
        for line in text.split_terminator('\n') {
            match serde_json::from_str::<Value>(line) {
                Ok(v) if v.is_object() => out.push(v),
                _ => break,
            }
        }
    }
    out
}

/// The record's first frame, if it is seq 0. Reads one line of the first segment only: 38,580 records on 2026-10-24.
fn first_frame(record: &Path) -> Option<Value> {
    let mut segments: Vec<PathBuf> = fs::read_dir(record)
        .ok()?
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with("events-") && n.ends_with(".jsonl"))
        })
        .collect();
    // sealed before open: `events-000001.jsonl` sorts after `events-000001.open.jsonl`, so order by sealed-ness first
    segments.sort_by_key(|p| (p.to_string_lossy().ends_with(".open.jsonl"), p.clone()));
    let text = fs::read(segments.first()?).ok()?;
    let end = text.iter().position(|&b| b == b'\n')?;
    let frame: Value = serde_json::from_slice(&text[..end]).ok()?;
    (frame.get("seq").and_then(Value::as_i64) == Some(0)).then_some(frame)
}

fn baseline_commit(record: &Path) -> Option<String> {
    first_frame(record)?
        .pointer("/payload/baseline/w2m_commit")
        .and_then(Value::as_str)
        .map(str::to_string)
}

fn git(root: &Path, args: &[&str]) -> Result<String, String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .map_err(|e| format!("git: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "git {}: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

/// True when `text` at byte `at` is not preceded by a word character and the match of length `len` is not followed by
/// one: Python's `\b` around a name of word characters.
fn whole_word_at(text: &str, at: usize, len: usize) -> bool {
    let word = |c: char| c.is_alphanumeric() || c == '_';
    !text[..at].chars().next_back().is_some_and(word) && !text[at + len..].chars().next().is_some_and(word)
}

fn names_whole(text: &str, name: &str) -> bool {
    text.match_indices(name)
        .any(|(at, _)| whole_word_at(text, at, name.len()))
}

/// Full sha and subject of every commit whose subject begins "Sprint NNN", in `git log --all` order.
fn sprint_commits(root: &Path, n: u32) -> Result<Vec<(String, String)>, String> {
    let prefix = format!("Sprint {n:03}");
    Ok(git(root, &["log", "--all", "--format=%H%x09%s"])?
        .lines()
        .filter_map(|l| l.split_once('\t'))
        .filter(|(_, s)| s.starts_with(&prefix) && whole_word_at(s, 0, prefix.len()))
        .map(|(h, s)| (h.to_string(), s.to_string()))
        .collect())
}

/// The record directories under store/runs that any of `commits` added.
fn added_records(root: &Path, commits: &[(String, String)]) -> Result<BTreeSet<String>, String> {
    let mut out = BTreeSet::new();
    for (sha, _) in commits {
        for path in git(
            root,
            &[
                "show",
                "--name-only",
                "--diff-filter=A",
                "--format=",
                sha,
                "--",
                "store/runs",
            ],
        )?
        .lines()
        {
            if let Some(name) = path.strip_prefix("store/runs/").and_then(|p| p.split('/').next()) {
                out.insert(name.to_string());
            }
        }
    }
    Ok(out)
}

fn sprint_records(root: &Path, commits: &[(String, String)]) -> Result<Vec<PathBuf>, String> {
    let shas: BTreeSet<&str> = commits.iter().map(|(h, _)| h.as_str()).collect();
    let added = added_records(root, commits)?;
    let mut dirs: Vec<PathBuf> = fs::read_dir(root.join("store/runs"))
        .map_err(|e| format!("store/runs: {e}"))?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();
    dirs.sort();
    Ok(dirs
        .into_iter()
        .filter(|d| match baseline_commit(d) {
            Some(c) => shas.contains(c.as_str()),
            None => d
                .file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| added.contains(n)),
        })
        .collect())
}

/// The text under a heading, up to the next `## ` or `### ` heading, trimmed.
fn section(text: &str, heading: &str) -> String {
    let mut lines = text.split('\n');
    if !lines.by_ref().any(|l| l == heading) {
        return String::new();
    }
    let body: Vec<&str> = lines
        .take_while(|l| !(l.starts_with("## ") || l.starts_with("### ")))
        .collect();
    body.join("\n").trim().to_string()
}

fn vocabulary_tags(root: &Path) -> Result<BTreeSet<String>, String> {
    let read = |p: PathBuf| -> Result<Value, String> {
        serde_json::from_str(&fs::read_to_string(&p).map_err(|e| format!("{}: {e}", p.display()))?)
            .map_err(|e| format!("{}: {e}", p.display()))
    };
    let version = read(root.join("signals/rust-current.json"))?["version"]
        .as_str()
        .ok_or("signals/rust-current.json: no version")?
        .to_string();
    let vocab = read(root.join(format!("signals/{version}.json")))?;
    Ok(vocab["tags"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|t| t["name"].as_str().map(str::to_string))
        .collect())
}

/// `1234567` -> `1,234,567`.
fn thousands(n: u64) -> String {
    let digits = n.to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

fn emits_check(emits: &str, fired: &Counts, tags: &BTreeSet<String>) -> Vec<String> {
    let named: Vec<&String> = tags.iter().filter(|t| names_whole(emits, t)).collect();
    let mut lines: Vec<String> = named
        .iter()
        .map(|t| match fired.get(t) {
            0 => format!("- `{t}`: NOT FIRED"),
            n => format!("- `{t}`: fired {} time{}", thousands(n), if n > 1 { "s" } else { "" }),
        })
        .collect();
    let mut unnamed: Vec<&(String, u64)> = fired.0.iter().filter(|(t, _)| !named.contains(&t)).collect();
    unnamed.sort();
    if !unnamed.is_empty() {
        let list: Vec<String> = unnamed.iter().map(|(t, n)| format!("`{t}` ({n})")).collect();
        lines.push(format!("- fired, not named in Emits: {}", list.join(", ")));
    }
    if named.is_empty() && !fired.0.is_empty() {
        lines.push("- the card's Emits names no tag, and the sprint's records fired the ones above".to_string());
    }
    if named.is_empty() && fired.0.is_empty() {
        lines.push("- the card's Emits names no tag, and the sprint made no record".to_string());
    }
    lines
}

/// `substrate narrate` of one record: (seq, text) per beat, a beat's continuation lines joined back to it.
fn narrate(root: &Path, record: &Path) -> Result<Vec<(i64, String)>, String> {
    let out = Command::new("uv")
        .current_dir(root)
        .args(["run", "--quiet", "substrate", "narrate"])
        .arg(record)
        .output()
        .map_err(|e| format!("substrate narrate: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "substrate narrate {}: {}",
            record.display(),
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    let mut beats: Vec<(i64, String)> = Vec::new();
    for line in String::from_utf8_lossy(&out.stdout).split_terminator('\n') {
        let parsed = line.strip_prefix("seq ").and_then(|rest| {
            let (seq, text) = rest.trim_start().split_once("  ")?;
            Some((seq.parse::<i64>().ok()?, text.to_string()))
        });
        match (parsed, beats.last_mut()) {
            (Some(beat), _) => beats.push(beat),
            (None, Some((_, text))) => {
                text.push('\n');
                text.push_str(line);
            }
            (None, None) => {
                return Err(format!(
                    "substrate narrate {}: unexpected line {line:?}",
                    record.display()
                ));
            }
        }
    }
    Ok(beats)
}

fn trace(root: &Path, records: &[PathBuf]) -> Result<(Vec<String>, Counts), String> {
    let mut lines = Vec::new();
    let mut fired = Counts::default();
    let mut groups: Vec<(String, Vec<&PathBuf>)> = Vec::new();
    for r in records {
        let name = r.file_name().and_then(|n| n.to_str()).unwrap_or_default();
        let key = GROUPED
            .iter()
            .find(|p| name.starts_with(**p))
            .map_or(name, |p| *p)
            .to_string();
        match groups.iter_mut().find(|(k, _)| *k == key) {
            Some((_, rs)) => rs.push(r),
            None => groups.push((key, vec![r])),
        }
    }
    for (name, rs) in &groups {
        let grouped = GROUPED.contains(&name.as_str());
        let mut events = Counts::default();
        let (mut failed, mut unfinished) = (0, 0);
        let mut first_frames = Vec::new();
        for (i, r) in rs.iter().enumerate() {
            let fs = frames(r);
            let mut finalised = false;
            for f in &fs {
                match f["kind"].as_str().unwrap_or_default() {
                    "substrate.ProducerFailed" => failed += 1,
                    "substrate.RunFinalised" => finalised = true,
                    k if !k.starts_with("substrate.") => events.add(k, 1),
                    _ => {}
                }
            }
            unfinished += u64::from(!finalised);
            if i == 0 {
                first_frames = fs;
            }
        }
        for (k, n) in &events.0 {
            fired.add(k, *n);
        }
        let counted: Vec<String> = events.0.iter().map(|(k, v)| format!("{k} {v}")).collect();
        let head = if grouped {
            format!("{name}* {} records", rs.len())
        } else {
            name.clone()
        };
        lines.push(format!(
            "{head}: {}; producers failed {failed}; not finalised {unfinished}",
            counted.join(", ")
        ));
        if grouped {
            continue;
        }
        let time_of = |seq: i64| -> Option<f64> {
            first_frames
                .iter()
                .find(|f| f["seq"].as_i64() == Some(seq))
                .and_then(|f| f["t"].as_f64())
        };
        let kind_of = |seq: i64| -> &str {
            first_frames
                .iter()
                .find(|f| f["seq"].as_i64() == Some(seq))
                .and_then(|f| f["kind"].as_str())
                .unwrap_or_default()
        };
        let t0 = time_of(0).unwrap_or(0.0);
        let mut repeated = Counts::default();
        for (seq, text) in narrate(root, rs[0])? {
            let kind = kind_of(seq);
            if kind.starts_with("substrate.") {
                continue;
            }
            repeated.add(kind, 1);
            if repeated.get(kind) > 3 {
                continue;
            }
            let t = time_of(seq).ok_or_else(|| format!("{}: seq {seq} has no time", rs[0].display()))?;
            let cut: String = text.chars().take(NARRATION_WIDTH).collect();
            lines.push(format!("  t={:9.3}  {cut}", t - t0));
        }
        for (kind, n) in &repeated.0 {
            if *n > 3 {
                lines.push(format!("  ... {kind} {} more", n - 3));
            }
        }
    }
    Ok((lines, fired))
}

fn card_path(root: &Path, n: u32) -> Result<PathBuf, String> {
    let prefix = format!("sprint-{n:03}-");
    let mut cards: Vec<PathBuf> = fs::read_dir(root.join("sprints"))
        .map_err(|e| format!("sprints: {e}"))?
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|f| f.to_str())
                .is_some_and(|f| f.starts_with(&prefix) && f.ends_with(".md"))
        })
        .collect();
    cards.sort();
    cards
        .into_iter()
        .next()
        .ok_or_else(|| format!("no card sprints/{prefix}*.md"))
}

fn quoted(text: &str) -> String {
    format!("> {}", text.replace('\n', "\n> "))
}

fn report(root: &Path, n: u32, backfill: Option<&str>) -> Result<String, String> {
    let card = fs::read_to_string(card_path(root, n)?).map_err(|e| e.to_string())?;
    let commits = sprint_commits(root, n)?;
    let records = sprint_records(root, &commits)?;
    let (lines, fired) = trace(root, &records)?;
    let emits = section(&card, "### Emits");
    let title = card
        .lines()
        .next()
        .unwrap_or_default()
        .trim_start_matches(['#', ' '])
        .trim();
    let scope_section = section(&card, "## scope");
    let scope = scope_section.split("\n\n").next().unwrap_or_default();
    let when = match backfill {
        Some(date) => format!(" on {date}, after the sprint's close (hand-author log 2026-10-24),"),
        None => String::new(),
    };
    let mut out = vec![
        format!("# SIGNAL_REPORT — {title}"),
        String::new(),
        format!(
            "Made by `signal_report {n}`{when} from {} records under store/runs/ whose baseline names one of the sprint's {} \
             commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail \
             entry's for Sprint {n:03}.",
            records.len(),
            commits.len()
        ),
        String::new(),
        "## 1. Observed".to_string(),
        String::new(),
        "### scope_confirmation".to_string(),
        String::new(),
        quoted(scope),
        String::new(),
        "### work_performed".to_string(),
        String::new(),
    ];
    out.extend(
        commits
            .iter()
            .map(|(sha, subject)| format!("- {} {subject}", &sha[..9])),
    );
    out.extend([
        String::new(),
        "### signal_trace".to_string(),
        String::new(),
        "```".to_string(),
    ]);
    if lines.is_empty() {
        out.push("(no record)".to_string());
    }
    out.extend(lines);
    out.extend([
        "```".to_string(),
        String::new(),
        "### signal_trace_tags".to_string(),
        String::new(),
    ]);
    let mut tags: Vec<&String> = fired.0.iter().map(|(t, _)| t).collect();
    tags.sort();
    out.extend(tags.iter().map(|t| format!("- `{t}`")));
    out.extend([String::new(), "## 2. Expected".to_string(), String::new()]);
    out.push(quoted(if emits.is_empty() {
        "(the card has no Emits)"
    } else {
        &emits
    }));
    out.extend([
        String::new(),
        "## 3. Delta".to_string(),
        String::new(),
        "### emits_check".to_string(),
        String::new(),
    ]);
    out.extend(emits_check(&emits, &fired, &vocabulary_tags(root)?));
    out.extend([
        String::new(),
        "## 4. Hypothesis".to_string(),
        String::new(),
        format!("See the BLACKBOARD tail entry for Sprint {n:03}: its Rubber Duck Pass and close."),
        String::new(),
    ]);
    Ok(out.join("\n"))
}

/// Every sprint from 108 on whose card says `status: done` and has no report.
fn missing_reports(root: &Path) -> Result<Vec<u32>, String> {
    let mut missing = Vec::new();
    for entry in fs::read_dir(root.join("sprints"))
        .map_err(|e| format!("sprints: {e}"))?
        .flatten()
    {
        let name = entry.file_name().to_string_lossy().to_string();
        let Some(n) = name
            .strip_prefix("sprint-")
            .and_then(|r| r.get(..3))
            .and_then(|d| d.parse::<u32>().ok())
        else {
            continue;
        };
        if n < FIRST_REPORTED_SPRINT || !name.ends_with(".md") {
            continue;
        }
        let card = fs::read_to_string(entry.path()).map_err(|e| e.to_string())?;
        let done = card.lines().any(|l| l.trim() == "status: done");
        if done && !root.join(format!("signal-reports/sprint-{n:03}-report.md")).exists() {
            missing.push(n);
        }
    }
    missing.sort_unstable();
    missing.dedup();
    Ok(missing)
}

/// The first sprint whose closed tail entry must name a passing run `scripts/gates.sh` kept (Sprint 179).
const FIRST_GATES_HELD_SPRINT: u32 = 179;

/// The first sprint whose named plants must show their judge's failure in its result (Sprint 171).
const FIRST_QUOTED_PLANT_SPRINT: u32 = 171;

/// Sprints from 171 marked done whose card's observation contract names a planted fault and whose result section has
/// no line that names a plant and quotes, in double quotes, what its judge printed (Sprint 171; reviewer-2's Medium 4 of
/// 2026-10-28: plants recorded without reaching their judge, after KIT_DIARY's H5 said a lesson needs a check).
fn unquoted_plants(root: &Path) -> Result<Vec<u32>, String> {
    let mut unquoted = Vec::new();
    for entry in fs::read_dir(root.join("sprints"))
        .map_err(|e| format!("sprints: {e}"))?
        .flatten()
    {
        let name = entry.file_name().to_string_lossy().to_string();
        let Some(n) = name
            .strip_prefix("sprint-")
            .and_then(|r| r.get(..3))
            .and_then(|d| d.parse::<u32>().ok())
        else {
            continue;
        };
        if n < FIRST_QUOTED_PLANT_SPRINT || !name.ends_with(".md") {
            continue;
        }
        let card = fs::read_to_string(entry.path()).map_err(|e| e.to_string())?;
        if !card.lines().any(|l| l.trim() == "status: done") {
            continue;
        }
        let contract = section(&card, "## observation contract").to_lowercase();
        if !contract.contains("planted fault") {
            continue;
        }
        // a card that numbers its plants needs a quoted judge line for each number (Sprint 176); one that does not,
        // a quoted line for any plant (Sprint 171)
        if !numbered_plants(&section(&card, "## observation contract")).is_empty() {
            if !unquoted_numbers(&card).is_empty() {
                unquoted.push(n);
            }
            continue;
        }
        let result = section(&card, "## result");
        let quoted = result.lines().any(|l| {
            let lower = l.to_lowercase();
            lower.contains("plant") && l.split('"').nth(1).is_some_and(|q| q.trim().len() >= 3)
        });
        if !quoted {
            unquoted.push(n);
        }
    }
    unquoted.sort_unstable();
    unquoted.dedup();
    Ok(unquoted)
}

/// The plants a card's observation contract numbers: "(1)", "(2)" in its "Planted faults" bullet (Sprint 176;
/// reviewer-2's Medium 3 of 2026-10-28: the rule passed a card whose one plant of two was quoted).
fn numbered_plants(contract: &str) -> BTreeSet<u32> {
    let Some(at) = contract.find("Planted faults") else {
        return BTreeSet::new();
    };
    let bullet = contract[at..].split("\n- ").next().unwrap_or("");
    bullet
        .split('(')
        .skip(1)
        .filter_map(|p| p.split_once(')').and_then(|(n, _)| n.parse::<u32>().ok()))
        .collect()
}

/// Text in double quotes of 3 or more characters: a judge's line quoted.
fn quotes(text: &str) -> bool {
    text.split('"').skip(1).step_by(2).any(|q| q.trim().len() >= 3)
}

/// The numbered plants whose result bullet ("- **Plant k**") is missing or quotes no judge. A bullet that says its plant
/// reached no judge counts only if a quote follows that phrase: the replacement plant's failure (Sprint 173's plant 2).
fn unquoted_numbers(card: &str) -> Vec<u32> {
    let numbers = numbered_plants(&section(card, "## observation contract"));
    let result = section(card, "## result");
    let bullets: Vec<&str> = result.split("\n- ").collect();
    numbers
        .into_iter()
        .filter(|k| {
            let head = format!("**plant {k}");
            let Some(b) = bullets
                .iter()
                .find(|b| b.trim_start_matches("- ").to_lowercase().starts_with(&head))
            else {
                return true;
            };
            match b.to_lowercase().find("reached no judge") {
                Some(i) => !quotes(&b[i..]),
                None => !quotes(b),
            }
        })
        .collect()
}

/// BLACKBOARD tail entries headed "### Sprint NNN (<date>, closed)" whose card is not `status: done`, or which hold an
/// unfilled GATES-LINE or CI-LINE (Sprint 176; reviewer-2's Medium 6 of 2026-10-28: Sprints 174 and 175 headed their
/// entries closed before their gates exited).
fn closed_too_early(root: &Path) -> Result<Vec<String>, String> {
    let board = fs::read_to_string(root.join("BLACKBOARD.md")).map_err(|e| format!("BLACKBOARD.md: {e}"))?;
    let tail = board
        .split("\n## Sprint tail\n")
        .nth(1)
        .ok_or("BLACKBOARD.md: no ## Sprint tail")?;
    let cards: Vec<(String, PathBuf)> = fs::read_dir(root.join("sprints"))
        .map_err(|e| format!("sprints: {e}"))?
        .flatten()
        .map(|e| (e.file_name().to_string_lossy().to_string(), e.path()))
        .collect();
    let mut wrong = Vec::new();
    // the passing gates runs `scripts/gates.sh` keeps (Sprint 179); absent on CI and in a fresh clone
    let passes = fs::read_to_string(root.join("build/gates-passes.txt")).ok();
    for entry in tail.split("\n### Sprint ").skip(1) {
        let (head, body) = entry.split_once('\n').unwrap_or((entry, ""));
        let Some(n) = head.get(..3).and_then(|d| d.parse::<u32>().ok()) else {
            continue;
        };
        let card_done = |n: u32| -> Result<bool, String> {
            let prefix = format!("sprint-{n:03}-");
            match cards.iter().find(|(name, _)| name.starts_with(&prefix)) {
                Some((_, path)) => Ok(fs::read_to_string(path)
                    .map_err(|e| e.to_string())?
                    .lines()
                    .any(|l| l.trim() == "status: done")),
                None => Ok(false),
            }
        };
        if !head.ends_with(", closed)") {
            // a card marked done before its tail entry is headed closed (Sprint 179; reviewer-2's Medium 3)
            if card_done(n)? {
                wrong.push(format!(
                    "sprint {n:03}: its card is done and its tail entry is not headed closed"
                ));
            }
            continue;
        }
        let body = body.split("\n\n").next().unwrap_or("");
        // a closed entry's gates line names a run `gates.sh` kept (from Sprint 179)
        if n >= FIRST_GATES_HELD_SPRINT
            && let Some(passes) = &passes
        {
            let named = body.find("exit 0 in ").and_then(|i| {
                let rest = &body[i + "exit 0 in ".len()..];
                let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
                (!digits.is_empty() && rest[digits.len()..].starts_with(" s")).then(|| format!("exit 0 in {digits} s"))
            });
            match named {
                Some(run) if passes.lines().any(|l| l.split('\t').next() == Some(run.as_str())) => {}
                Some(run) => wrong.push(format!(
                    "sprint {n:03}: its tail entry names \"{run}\", which build/gates-passes.txt does not hold"
                )),
                None => wrong.push(format!(
                    "sprint {n:03}: its tail entry names no gates run (\"exit 0 in N s\")"
                )),
            }
        }
        for token in ["GATES-LINE", "CI-LINE"] {
            if body.contains(token) {
                wrong.push(format!(
                    "sprint {n:03}: its tail entry is headed closed and holds {token}"
                ));
            }
        }
        if !card_done(n)? {
            wrong.push(format!(
                "sprint {n:03}: its tail entry is headed closed and its card is not done"
            ));
        }
    }
    Ok(wrong)
}

fn run() -> Result<bool, String> {
    let mut args = std::env::args().skip(1);
    let (mut root, mut sprint, mut write, mut check, mut backfill) = (PathBuf::from("."), None, false, false, None);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--root" => root = args.next().map(PathBuf::from).ok_or("--root needs a directory")?,
            "--write" => write = true,
            "--check" => check = true,
            "--backfill" => {
                backfill = Some(
                    args.next()
                        .ok_or("--backfill needs the date it is written, YYYY-MM-DD")?,
                )
            }
            n => sprint = Some(n.parse::<u32>().map_err(|_| format!("unknown argument {n}"))?),
        }
    }
    if check {
        let missing = missing_reports(&root)?;
        for n in &missing {
            eprintln!("sprint {n:03}: its card says done and signal-reports/sprint-{n:03}-report.md does not exist");
        }
        if missing.is_empty() {
            println!("every sprint from {FIRST_REPORTED_SPRINT:03} marked done has a Signal Report");
        }
        let unquoted = unquoted_plants(&root)?;
        for n in &unquoted {
            eprintln!(
                "sprint {n:03}: its observation contract names a planted fault and its result quotes no judge's \
                 failure for a plant"
            );
        }
        if unquoted.is_empty() {
            println!(
                "every sprint from {FIRST_QUOTED_PLANT_SPRINT:03} marked done that names a plant quotes its judge's \
                 failure"
            );
        }
        let early = closed_too_early(&root)?;
        for w in &early {
            eprintln!("{w}");
        }
        if early.is_empty() {
            println!("every tail entry headed closed has its card done and no unfilled line");
        }
        return Ok(missing.is_empty() && unquoted.is_empty() && early.is_empty());
    }
    let n = sprint.ok_or("usage: signal_report NNN [--write] [--backfill YYYY-MM-DD] | --check")?;
    let text = report(&root, n, backfill.as_deref())?;
    if write {
        let out = format!("signal-reports/sprint-{n:03}-report.md");
        fs::write(root.join(&out), text).map_err(|e| format!("{out}: {e}"))?;
        println!("{out}");
    } else {
        print!("{text}");
    }
    Ok(true)
}

fn main() -> ExitCode {
    match run() {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(e) => {
            eprintln!("signal_report: {e}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thousands_are_separated_as_python_formats_them() {
        assert_eq!(thousands(7), "7");
        assert_eq!(thousands(1054), "1,054");
        assert_eq!(thousands(1_234_567), "1,234,567");
    }

    #[test]
    fn a_tag_inside_a_longer_name_is_not_named() {
        assert!(names_whole("SESSION_INIT (purpose)", "SESSION_INIT"));
        assert!(!names_whole("XSESSION_INIT", "SESSION_INIT"));
        assert!(!names_whole("SESSION_INITIAL", "SESSION_INIT"));
    }

    #[test]
    fn a_section_ends_at_the_next_heading_of_level_two_or_three() {
        let card =
            "# T\n\n## scope\n\nfirst\n\nsecond\n\n### Emits\n\n- SESSION_INIT\n\n#### deeper\n\nstill\n## next\n";
        assert_eq!(section(card, "### Emits"), "- SESSION_INIT\n\n#### deeper\n\nstill");
        assert_eq!(section(card, "## scope"), "first\n\nsecond");
        assert_eq!(section(card, "## absent"), "");
    }

    const CARD: &str = "## observation contract\n\n- **Planted faults:** (1) one; (2) two (Sprint 173).\n\n\
                        ## result\n\n- **Plant 1** (one): the judge failed, \"88 RCR CF: case 206\".\n\
                        - **Plant 2** (two) reached no judge. Replaced: the judge failed, \"91 ADD_LOCK fault rip\".\n";

    #[test]
    fn each_numbered_plant_needs_its_own_quote() {
        assert_eq!(
            numbered_plants(&section(CARD, "## observation contract")),
            [1, 2].into()
        );
        assert!(unquoted_numbers(CARD).is_empty());
        // plant 1's quote removed: the card fails on 1 though plant 2 is quoted
        let one_unquoted = CARD.replace("\"88 RCR CF: case 206\"", "88 RCR CF");
        assert_eq!(unquoted_numbers(&one_unquoted), [1]);
        // a plant that reached no judge, with no replacement quoted after it
        let no_replacement = CARD.replace("Replaced: the judge failed, \"91 ADD_LOCK fault rip\".", "");
        assert_eq!(unquoted_numbers(&no_replacement), [2]);
    }

    #[test]
    fn a_tag_named_and_not_fired_is_reported_not_fired() {
        let tags: BTreeSet<String> = ["SESSION_INIT", "SESSION_COMPLETED"].map(String::from).into();
        let mut fired = Counts::default();
        fired.add("SESSION_INIT", 1);
        fired.add("OTHER_EVENT", 2);
        assert_eq!(
            emits_check("SESSION_INIT and SESSION_COMPLETED", &fired, &tags),
            [
                "- `SESSION_COMPLETED`: NOT FIRED",
                "- `SESSION_INIT`: fired 1 time",
                "- fired, not named in Emits: `OTHER_EVENT` (2)"
            ]
        );
    }
}
