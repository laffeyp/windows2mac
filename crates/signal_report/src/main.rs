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

/// The first sprint whose closed tail entry must name a kept run of its own sprint, `gates.sh`'s fourth column (Sprint
/// 184; reviewer-2's Medium 3 of its review of 178-181: a seconds count alone can match another sprint's run).
const FIRST_GATES_BOUND_SPRINT: u32 = 184;

/// The first sprint whose closed entry's gates run must have tested its work commit's trees (Sprint 187).
const FIRST_TREE_BOUND_SPRINT: u32 = 187;

/// The passing gates runs `scripts/gates.sh` keeps, tracked from Sprint 187 (until then `build/gates-passes.txt`).
const PASSES: &str = "store/gates/passes.tsv";

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
/// entries closed before their gates exited). An entry headed "closed-partial" needs its card `status: closed-partial`,
/// and a Deferred entry tagged "(Sprint N, partial)" needs card N so (Sprint 184; reviewer-2's Medium 2 of its review of
/// 178-181: Sprint 178 closed partial under a "closed" status). Without the tracked passes file the gates rule fails,
/// on CI too (Sprint 187: the file is committed and CI fetches the whole history), and from Sprint 187 a closed entry's
/// run must have tested its work commit's trees.
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
    // the passing gates runs `scripts/gates.sh` keeps (Sprint 179), tracked from Sprint 187, so CI and a fresh clone
    // read them (reviewer-2's Medium 2 of its review of 182-184)
    let passes = fs::read_to_string(root.join(PASSES)).ok();
    let ci = fs::read_to_string(root.join(CI_RUNS)).ok();
    let mut gates_unread = false;
    let card_status = |n: u32| -> Result<Option<String>, String> {
        let prefix = format!("sprint-{n:03}-");
        match cards.iter().find(|(name, _)| name.starts_with(&prefix)) {
            Some((_, path)) => Ok(fs::read_to_string(path)
                .map_err(|e| e.to_string())?
                .lines()
                .find_map(|l| l.trim().strip_prefix("status: ").map(str::to_string))),
            None => Ok(None),
        }
    };
    let card_done = |n: u32| -> Result<bool, String> { Ok(card_status(n)?.as_deref() == Some("done")) };
    for entry in tail.split("\n### Sprint ").skip(1) {
        let (head, body) = entry.split_once('\n').unwrap_or((entry, ""));
        let Some(n) = head.get(..3).and_then(|d| d.parse::<u32>().ok()) else {
            continue;
        };
        let partial = head.ends_with(", closed-partial)");
        if partial && card_status(n)?.as_deref() != Some("closed-partial") {
            wrong.push(format!(
                "sprint {n:03}: its tail entry is headed closed-partial and its card is not closed-partial"
            ));
        }
        if !head.ends_with(", closed)") && !partial {
            // a card marked done before its tail entry is headed closed (Sprint 179; reviewer-2's Medium 3)
            if card_done(n)? {
                wrong.push(format!(
                    "sprint {n:03}: its card is done and its tail entry is not headed closed"
                ));
            }
            continue;
        }
        let body = body.split("\n\n").next().unwrap_or("");
        // a closed entry's gates line names a run `gates.sh` kept (from Sprint 179), of its own sprint (from 184)
        if n >= FIRST_GATES_HELD_SPRINT {
            match &passes {
                None => gates_unread = true,
                Some(passes) => {
                    let named = body.find("exit 0 in ").and_then(|i| {
                        let rest = &body[i + "exit 0 in ".len()..];
                        let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
                        (!digits.is_empty() && rest[digits.len()..].starts_with(" s"))
                            .then(|| format!("exit 0 in {digits} s"))
                    });
                    let sprint = format!("sprint {n:03}");
                    let kept = |run: &str| {
                        passes.lines().find(|l| {
                            let cols: Vec<&str> = l.split('\t').collect();
                            cols.first() == Some(&run)
                                && (n < FIRST_GATES_BOUND_SPRINT || cols.get(3) == Some(&sprint.as_str()))
                        })
                    };
                    match named {
                        Some(run) if kept(&run).is_some() => {
                            // from Sprint 187 the run's trees are the work commit's (column 7)
                            if n >= FIRST_TREE_BOUND_SPRINT
                                && let Some(w) = trees_moved(root, n, kept(&run).unwrap())?
                            {
                                wrong.push(w);
                            }
                            // from Sprint 192 its close commit changes bookkeeping only, and its claim of the
                            // multi-vCPU stage is the stage's record
                            if n >= FIRST_CLOSE_BOUND_SPRINT {
                                wrong.extend(close_moved(root, n)?);
                                if n >= FIRST_STAGED_SPRINT {
                                    wrong.extend(multi_vcpu_staged(root, n, body, kept(&run).unwrap()));
                                } else if n >= FIRST_WALKED_SPRINT {
                                    wrong.extend(multi_vcpu_count(root, n, body, kept(&run).unwrap()));
                                } else {
                                    wrong.extend(multi_vcpu_claim(root, n, body));
                                }
                            }
                            // from Sprint 195 every commit between its card and the next card is its card, its work
                            // or its close
                            if n >= FIRST_WALKED_SPRINT {
                                wrong.extend(commits_walked(root, n)?);
                            }
                        }
                        Some(run) if n >= FIRST_GATES_BOUND_SPRINT && kept_any(passes, &run) => wrong.push(format!(
                            "sprint {n:03}: its tail entry names \"{run}\", which {PASSES} holds for another sprint"
                        )),
                        Some(run) => wrong.push(format!(
                            "sprint {n:03}: its tail entry names \"{run}\", which {PASSES} does not hold"
                        )),
                        None => wrong.push(format!(
                            "sprint {n:03}: its tail entry names no gates run (\"exit 0 in N s\")"
                        )),
                    }
                }
            }
        }
        // from Sprint 199 the entry's quoted `sed` commands are held to what they do (Sprint 200)
        if n >= FIRST_QUOTED_EDIT_SPRINT {
            wrong.extend(quoted_edits(n, body));
        }
        // from Sprint 199 the run a later entry cites for this sprint's close concluded success, or this entry names
        // it failed (Sprint 212)
        if n >= FIRST_CI_HELD_SPRINT {
            wrong.extend(ci_held(n, body, tail, ci.as_deref()));
        }
        for token in ["GATES-LINE", "CI-LINE"] {
            if body.contains(token) {
                wrong.push(format!(
                    "sprint {n:03}: its tail entry is headed closed and holds {token}"
                ));
            }
        }
        if !partial && !card_done(n)? {
            wrong.push(format!(
                "sprint {n:03}: its tail entry is headed closed and its card is not done"
            ));
        }
    }
    if gates_unread {
        wrong.push(format!(
            "{PASSES} does not exist: no closed entry's gates run can be read"
        ));
    }
    // a Deferred entry tagged "(Sprint N, partial)" names a partial close (Sprint 184)
    let deferred = section(&board, "## Deferred");
    for piece in deferred.split("(Sprint ").skip(1) {
        let Some(n) = piece.split_once(", partial)").and_then(|(d, _)| d.parse::<u32>().ok()) else {
            continue;
        };
        if card_status(n)?.as_deref() != Some("closed-partial") {
            wrong.push(format!(
                "sprint {n:03}: a Deferred entry names it partial and its card is not closed-partial"
            ));
        }
    }
    Ok(wrong)
}

/// The CI runs read with `gh run view` (run id, head, conclusion, time read), tracked from Sprint 212.
const CI_RUNS: &str = "store/gates/ci.tsv";
/// The first sprint whose close is held to its CI run's conclusion (Sprint 212; reviewer-3's High 2 of its review of
/// Sprint 211: Sprint 210 stood closed on a failed run, read once from a watch's trailing `echo`).
const FIRST_CI_HELD_SPRINT: u32 = 199;

/// Sprint `n`'s close against its CI run: the run a later tail entry cites as "Sprint N's CI run <id>" (or "(<id>")
/// must be in `ci` and conclude success, or `body`, the sprint's own entry, names the run and "failed". No later entry
/// citing a run: the close's CI is pending, and nothing is held yet. WORKING_AGREEMENT's "CI on the closing commit".
fn ci_held(n: u32, body: &str, tail: &str, ci: Option<&str>) -> Vec<String> {
    let cited = format!("Sprint {n}'s CI run ");
    let Some(at) = tail.find(&cited) else {
        return Vec::new();
    };
    let digits: String = tail[at + cited.len()..]
        .trim_start_matches('(')
        .chars()
        .take_while(char::is_ascii_digit)
        .collect();
    if digits.is_empty() {
        return vec![format!(
            "sprint {n:03}: a later entry cites its CI run without the run's id"
        )];
    }
    let Some(ci) = ci else {
        return vec![format!(
            "{CI_RUNS} does not exist: sprint {n:03}'s CI run {digits} cannot be read"
        )];
    };
    match ci
        .lines()
        .map(|l| l.split('\t').collect::<Vec<_>>())
        .find(|c| c[0] == digits)
    {
        None => vec![format!(
            "sprint {n:03}: its CI run {digits} is not in {CI_RUNS} (`gh run view {digits} --json conclusion`)"
        )],
        Some(cols) if cols.get(2) == Some(&"success") => Vec::new(),
        Some(_) if body.contains(&digits) && body.contains("failed") => Vec::new(),
        Some(cols) => vec![format!(
            "sprint {n:03}: its tail entry is headed closed and its CI run {digits} concluded {}, which the entry does not \
             name",
            cols.get(2).unwrap_or(&"nothing")
        )],
    }
}

/// The first sprint whose close commit is held to bookkeeping paths and whose multi-vCPU claim to the stage's record
/// (Sprint 192; reviewer-2's Medium 3 and Low 2 of its review of 185-189).
const FIRST_CLOSE_BOUND_SPRINT: u32 = 192;

/// The multi-vCPU stage's results, tracked from Sprint 192 (date, outcome, "sprint NNN", test).
const MULTI_VCPU: &str = "store/gates/multi-vcpu.tsv";

/// Whether `subject` is sprint `n`'s close: "Sprint N closed:", or "Sprint N closed-partial:" for a sprint closed
/// partial (Sprint 206: Sprint 205's partial close failed the commit walk on CI, which knew only the first).
fn is_close(subject: &str, n: u32) -> bool {
    subject.starts_with(&format!("Sprint {n} closed:")) || subject.starts_with(&format!("Sprint {n} closed-partial:"))
}

/// A path of sprint `n`'s close commit ("Sprint N closed: ...") outside `BOOKKEEPING`, named; None when it changes
/// bookkeeping only, or has not been made yet (a tail entry is headed closed in the close commit itself).
fn close_moved(root: &Path, n: u32) -> Result<Option<String>, String> {
    let log = git(root, &["log", "--format=%H%x09%s"])?;
    let Some(commit) = log
        .lines()
        .filter_map(|l| l.split_once('\t'))
        .find(|(_, s)| is_close(s, n))
        .map(|(h, _)| h.to_string())
    else {
        return Ok(None);
    };
    let changed = git(root, &["diff-tree", "--no-commit-id", "--name-only", "-r", &commit])?;
    Ok(changed.lines().find(|p| !is_bookkeeping(p, n)).map(|p| {
        format!(
            "sprint {n:03}: its close commit {} changes {p}, outside the bookkeeping paths",
            &commit[..9]
        )
    }))
}

/// A closed entry that says the multi-vCPU tests passed, held to the stage's last three lines for its sprint.
fn multi_vcpu_claim(root: &Path, n: u32, body: &str) -> Option<String> {
    if !body.contains("multi-vCPU tests passed") {
        return None;
    }
    let text = fs::read_to_string(root.join(MULTI_VCPU)).unwrap_or_default();
    let sprint = format!("sprint {n:03}");
    let lines: Vec<&str> = text
        .lines()
        .filter(|l| l.split('\t').nth(2) == Some(sprint.as_str()))
        .collect();
    let last = &lines[lines.len().saturating_sub(3)..];
    if last.len() == 3 && last.iter().all(|l| l.split('\t').nth(1) == Some("passed")) {
        return None;
    }
    Some(format!(
        "sprint {n:03}: its tail entry says the multi-vCPU tests passed, and {MULTI_VCPU} holds {} of 3 passed",
        last.iter().filter(|l| l.split('\t').nth(1) == Some("passed")).count()
    ))
}

/// The first sprint whose commits are walked from its card to the next card's, whose multi-vCPU claim is a stated
/// count, and whose gated tree keeps the documents `design_check` reads (Sprint 195; reviewer-2's Medium 4 and Low 4 of
/// its review of 190-192).
const FIRST_WALKED_SPRINT: u32 = 195;

/// The documents under `docs/` that `design_check` reads (its `is_checked`: a name beginning `design-`, `solution-`,
/// `recentering-` or `architecture-`, or holding `plan-`); from Sprint 195 they are gated, not bookkeeping, as git
/// pathspecs.
const DESIGN_DOCUMENTS: [&str; 5] = [
    "docs/design-*",
    "docs/solution-*",
    "docs/recentering-*",
    "docs/architecture-*",
    "docs/*plan-*",
];

/// Whether `path` is one sprint `n`'s close commit may change: under `BOOKKEEPING`, and from Sprint 195 not one of
/// `DESIGN_DOCUMENTS`.
fn is_bookkeeping(path: &str, n: u32) -> bool {
    let under = BOOKKEEPING
        .iter()
        .any(|b| path == *b || path.starts_with(&format!("{b}/")));
    let design = path.strip_prefix("docs/").is_some_and(|name| {
        !name.contains('/')
            && (["design-", "solution-", "recentering-", "architecture-"]
                .iter()
                .any(|p| name.starts_with(p))
                || name.contains("plan-"))
    });
    under && !(n >= FIRST_WALKED_SPRINT && design)
}

/// Seconds since 1970 of a "YYYY-MM-DDTHH:MM:SSZ" time, by the civil-from-days formula (Howard Hinnant, "chrono-
/// Compatible Low-Level Date Algorithms", `days_from_civil`).
fn epoch_seconds(time: &str) -> Option<i64> {
    let n = |r: std::ops::Range<usize>| time.get(r)?.parse::<i64>().ok();
    let (y, m, d) = (n(0..4)?, n(5..7)?, n(8..10)?);
    let (hh, mm, ss) = (n(11..13)?, n(14..16)?, n(17..19)?);
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    Some((era * 146_097 + doe - 719_468) * 86_400 + hh * 3600 + mm * 60 + ss)
}

/// From Sprint 195 a closed entry states "multi-vCPU: k of 3 passed", and k is the count of the stage's rows for its
/// sprint, passed, timed inside the cited gates run (the pass line's end less its seconds, to its end); a missing
/// statement or another count is named (reviewer-2's Medium 4 of its review of 190-192: the phrase's absence passed).
fn multi_vcpu_count(root: &Path, n: u32, body: &str, pass: &str) -> Option<String> {
    let stated = body
        .split("multi-vCPU: ")
        .nth(1)
        .and_then(|rest| rest.split_once(" of 3 passed"))
        .and_then(|(k, _)| k.parse::<usize>().ok());
    let Some(stated) = stated else {
        return Some(format!(
            "sprint {n:03}: its tail entry states no \"multi-vCPU: k of 3 passed\""
        ));
    };
    let cols: Vec<&str> = pass.split('\t').collect();
    let seconds: i64 = cols[0]
        .strip_prefix("exit 0 in ")
        .and_then(|s| s.strip_suffix(" s"))
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    let end = cols.get(2).and_then(|t| epoch_seconds(t)).unwrap_or(0);
    let text = fs::read_to_string(root.join(MULTI_VCPU)).unwrap_or_default();
    let sprint = format!("sprint {n:03}");
    let passed = text
        .lines()
        .map(|l| l.split('\t').collect::<Vec<_>>())
        .filter(|c| c.get(2) == Some(&sprint.as_str()) && c.get(1) == Some(&"passed"))
        .filter(|c| {
            c.first()
                .and_then(|t| epoch_seconds(t))
                .is_some_and(|t| end - seconds <= t && t <= end)
        })
        .count();
    (passed != stated).then(|| {
        format!(
            "sprint {n:03}: its tail entry states multi-vCPU {stated} of 3 passed, and {MULTI_VCPU} holds {passed} \
             passed in its gates run"
        )
    })
}

/// The first sprint whose pass line records the stages run (column 8) and whose multi-vCPU claim is held to them
/// (Sprint 198; reviewer-2's Medium 2 of its review of 193-196: "0 of 3 passed" passed alike for a stage skipped and for
/// three failures).
const FIRST_STAGED_SPRINT: u32 = 198;

/// The first sprint whose tail entry quotes every `sed` it ran (Sprint 200; reviewer-2's Medium 4 of its review of
/// 196-198: the rule that a multi-line edit goes through the Edit tool rested on each sprint's own account).
const FIRST_QUOTED_EDIT_SPRINT: u32 = 199;
/// What an entry says of a quoted command `edit_guard` refuses, when it owns the breach.
const OWNED_BREACH: &str = "multi-line edit outside the Edit tool";

/// Each quoted `sed -i` command of `body` (start, end, the command as the shell ran it): a backticked span, or a span in
/// double quotes whose inner `\"` and `\\` are the shell's (Sprint 212; reviewer-3's Medium 5 of its review of Sprint
/// 211: Sprints 210's and 211's multi-line commands were quoted in double quotes, which the backtick scan never read).
fn quoted_commands(body: &str) -> Vec<(usize, usize, String)> {
    let mut found = Vec::new();
    let mut at = 0;
    for (i, span) in body.split('`').enumerate() {
        if i % 2 == 1 && span.starts_with("sed -i") {
            found.push((at, at + span.len(), span.to_string()));
        }
        at += span.len() + 1;
    }
    let bytes = body.as_bytes();
    let mut from = 0;
    while let Some(i) = body[from..].find("\"sed -i") {
        let start = from + i + 1;
        let mut end = start;
        while end < bytes.len() && !(bytes[end] == b'"' && bytes[end - 1] != b'\\') {
            end += 1;
        }
        // a double-quoted `"sed -i` inside a backticked command is part of it
        if !found.iter().any(|(s, e, _)| *s <= start && start <= *e) {
            found.push((start, end, body[start..end].replace("\\\"", "\"").replace("\\\\", "\\")));
        }
        from = end.max(start);
    }
    found.sort();
    found
}

/// Each quoted `sed -i` command `edit_guard::refusal` refuses, named, unless the clause that quotes it owns it: the text
/// from the command to the next quoted command holds "a multi-line edit outside the Edit tool" (Sprint 212: before,
/// one owning phrase anywhere in the entry passed every command in it). The classifier the hook would run judges the
/// record of what ran.
fn quoted_edits(n: u32, body: &str) -> Vec<String> {
    let commands = quoted_commands(body);
    commands
        .iter()
        .enumerate()
        .filter_map(|(i, (_, end, command))| {
            let why = edit_guard::refusal(command)?;
            let next = commands.get(i + 1).map_or(body.len(), |c| c.0);
            let clause = body.get(*end..next).unwrap_or("");
            (!clause.contains(OWNED_BREACH)).then(|| {
                format!(
                    "sprint {n:03}: its tail entry quotes `{command}`, a multi-line edit ({why}), and owns no breach"
                )
            })
        })
        .collect()
}

/// From Sprint 198: with `python=0` in the pass line's stages, "multi-vCPU: not run (Python gates not run)" and no
/// stage row in the run's window; with `python=1`, "multi-vCPU: k of 3 passed", exactly three rows in the window, k of
/// them passed. Any other pairing is named.
fn multi_vcpu_staged(root: &Path, n: u32, body: &str, pass: &str) -> Option<String> {
    let cols: Vec<&str> = pass.split('\t').collect();
    let Some(python) = cols
        .get(7)
        .and_then(|stages| stages.split(' ').find_map(|s| s.strip_prefix("python=")))
    else {
        return Some(format!("sprint {n:03}: its gates run names no stages (column 8)"));
    };
    let seconds: i64 = cols[0]
        .strip_prefix("exit 0 in ")
        .and_then(|s| s.strip_suffix(" s"))
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    let end = cols.get(2).and_then(|t| epoch_seconds(t)).unwrap_or(0);
    let text = fs::read_to_string(root.join(MULTI_VCPU)).unwrap_or_default();
    let sprint = format!("sprint {n:03}");
    let rows: Vec<Vec<&str>> = text
        .lines()
        .map(|l| l.split('\t').collect::<Vec<_>>())
        .filter(|c| c.get(2) == Some(&sprint.as_str()))
        .filter(|c| {
            c.first()
                .and_then(|t| epoch_seconds(t))
                .is_some_and(|t| end - seconds <= t && t <= end)
        })
        .collect();
    let passed = rows.iter().filter(|c| c.get(1) == Some(&"passed")).count();
    if python == "0" {
        if !body.contains("multi-vCPU: not run (Python gates not run)") {
            return Some(format!(
                "sprint {n:03}: its gates run did not run the Python gates, and its tail entry does not state \
                 \"multi-vCPU: not run (Python gates not run)\""
            ));
        }
        return (!rows.is_empty()).then(|| {
            format!(
                "sprint {n:03}: its gates run did not run the Python gates, and {MULTI_VCPU} holds {} rows in it",
                rows.len()
            )
        });
    }
    let stated = body
        .split("multi-vCPU: ")
        .nth(1)
        .and_then(|rest| rest.split_once(" of 3 passed"))
        .and_then(|(k, _)| k.parse::<usize>().ok());
    match stated {
        None => Some(format!(
            "sprint {n:03}: its gates run ran the multi-vCPU stage, and its tail entry states no \
             \"multi-vCPU: k of 3 passed\""
        )),
        Some(_) if rows.len() != 3 => Some(format!(
            "sprint {n:03}: its gates run ran the multi-vCPU stage, and {MULTI_VCPU} holds {} rows in it, not 3",
            rows.len()
        )),
        Some(k) if k != passed => Some(format!(
            "sprint {n:03}: its tail entry states multi-vCPU {k} of 3 passed, and {MULTI_VCPU} holds {passed} \
             passed in its gates run"
        )),
        Some(_) => None,
    }
}

/// Sprint `n`'s commits from its card's to the next card's (or HEAD), each named that is not its work ("Sprint N: "),
/// its close ("Sprint N closed:", bookkeeping only, held by `close_moved`) or the card itself (Sprint 195; reviewer-2's
/// Medium 4 of its review of 190-192: a commit under another subject between a pass and a close went unchecked).
fn commits_walked(root: &Path, n: u32) -> Result<Vec<String>, String> {
    let is_card =
        |s: &str, k: u32| s.starts_with(&format!("Sprint {k} card:")) || s.starts_with(&format!("Sprint {k}'s card:"));
    let log = git(root, &["log", "--reverse", "--format=%H%x09%s"])?;
    let commits: Vec<(&str, &str)> = log.lines().filter_map(|l| l.split_once('\t')).collect();
    let Some(start) = commits.iter().position(|(_, s)| is_card(s, n)) else {
        return Ok(vec![format!(
            "sprint {n:03}: no card commit (\"Sprint {n} card: ...\")"
        )]);
    };
    let work = format!("Sprint {n}: ");
    let mut wrong = Vec::new();
    for (hash, subject) in &commits[start + 1..] {
        if is_card(subject, n + 1) {
            break;
        }
        if !(subject.starts_with(&work) || is_close(subject, n)) {
            wrong.push(format!(
                "sprint {n:03}: commit {} \"{subject}\" stands between its card and the next card, and is neither its \
                 work nor its close",
                &hash[..9]
            ));
        }
    }
    Ok(wrong)
}

/// Whether the passes file holds a run of these seconds for any sprint.
fn kept_any(passes: &str, run: &str) -> bool {
    passes.lines().any(|l| l.split('\t').next() == Some(run))
}

/// The paths a close commit may change and the gates' tree leaves out (Sprint 192): the board, the diary, the agreement,
/// the cards, the reports, the docs and the passes themselves.
const BOOKKEEPING: [&str; 7] = [
    "BLACKBOARD.md",
    "KIT_DIARY.md",
    "WORKING_AGREEMENT.md",
    "sprints",
    "signal-reports",
    "docs",
    "store/gates",
];

/// The first tree a gates run tested that sprint `n`'s work commit (the newest commit, in `git log`'s order, whose
/// subject begins "Sprint NNN: ") does not hold, or None when every one matches (Sprint 187; reviewer-2's Medium 2 of
/// its review of 182-184: a pass line's HEAD and diff were written and never read). The pass line's seventh column is
/// `gates.sh`'s list: from Sprint 192 "root=tree", the whole work tree less `BOOKKEEPING` (reviewer-2's Medium 3 of its
/// review of 185-189), before it "dir=tree" for six directories; the commit's trees are taken the same way, from a
/// temporary index, and a root that differs is named by its first differing top-level path.
fn trees_moved(root: &Path, n: u32, line: &str) -> Result<Option<String>, String> {
    let Some(trees) = line.split('\t').nth(6).filter(|t| !t.is_empty()) else {
        return Ok(Some(format!(
            "sprint {n:03}: its gates run in {PASSES} names no trees (column 7)"
        )));
    };
    let subject = format!("Sprint {n}: ");
    let log = git(root, &["log", "--format=%H%x09%s"])?;
    let Some(commit) = log
        .lines()
        .filter_map(|l| l.split_once('\t'))
        .find(|(_, s)| s.starts_with(&subject))
        .map(|(h, _)| h.to_string())
    else {
        return Ok(Some(format!(
            "sprint {n:03}: no commit \"{subject}...\" to compare its gates run's trees with"
        )));
    };
    let index = std::env::temp_dir().join(format!("signal-report-index-{}-{n}", std::process::id()));
    let _ = fs::remove_file(&index);
    let with_index = |args: &[&str]| -> Result<String, String> {
        let out = Command::new("git")
            .arg("-C")
            .arg(root)
            .args(args)
            .env("GIT_INDEX_FILE", &index)
            .output()
            .map_err(|e| format!("git: {e}"))?;
        if !out.status.success() {
            return Err(format!(
                "git {}: {}",
                args.join(" "),
                String::from_utf8_lossy(&out.stderr).trim()
            ));
        }
        Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
    };
    with_index(&["read-tree", &commit])?;
    let mut remove = vec!["rm", "-r", "-q", "-f", "--cached", "--ignore-unmatch", "--"];
    remove.extend(BOOKKEEPING);
    // from Sprint 195 the documents design_check reads stay in the gated tree, as `scripts/gated-tree.sh` keeps them
    let kept: Vec<String> = DESIGN_DOCUMENTS.iter().map(|d| format!(":(exclude){d}")).collect();
    if n >= FIRST_WALKED_SPRINT {
        remove.extend(kept.iter().map(String::as_str));
    }
    with_index(&remove)?;
    let mut moved = None;
    for pair in trees.split(' ') {
        let Some((dir, ran)) = pair.split_once('=') else {
            continue;
        };
        if dir == "root" {
            let held = with_index(&["write-tree"])?;
            if held != ran {
                let first = git(root, &["diff-tree", "--name-only", ran, &held])
                    .ok()
                    .and_then(|d| d.lines().next().map(str::to_string))
                    .unwrap_or_else(|| "a path".into());
                moved = Some(format!(
                    "sprint {n:03}: its gates ran on a tree whose {first} its work commit {} does not hold",
                    &commit[..9]
                ));
                break;
            }
            continue;
        }
        let held = with_index(&["write-tree", &format!("--prefix={dir}/")])?;
        if held != ran {
            moved = Some(format!(
                "sprint {n:03}: its gates ran on {dir}/ {ran}, and its work commit {} holds {held}",
                &commit[..9]
            ));
            break;
        }
    }
    let _ = fs::remove_file(&index);
    Ok(moved)
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
        let blackboard = fs::read_to_string(root.join("BLACKBOARD.md")).map_err(|e| format!("BLACKBOARD.md: {e}"))?;
        let split = decisions_split(&blackboard);
        for w in &split {
            eprintln!("{w}");
        }
        if split.is_empty() {
            println!(
                "every agent-resolved Decision from {FIRST_CLAUSED_DECISION} holds one decision and one revisit clause"
            );
        }
        return Ok(missing.is_empty() && unquoted.is_empty() && early.is_empty() && split.is_empty());
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

/// The first date whose agent-resolved Decisions carry a decision clause and a revisit clause each (most before it
/// carry no decision clause).
const FIRST_CLAUSED_DECISION: &str = "2026-10-25";

/// Each "- **YYYY-MM-DD agent-resolved**" entry from `FIRST_CLAUSED_DECISION` holding other than one "Decision:" and
/// one "Revisit if:", named (Sprint 209; reviewer-2's Medium 4 of its review of 206-209: an insert split one Decision,
/// leaving its clauses trailing the next).
fn decisions_split(blackboard: &str) -> Vec<String> {
    blackboard
        .lines()
        .filter_map(|l| {
            let date = l.strip_prefix("- **")?.split(' ').next()?;
            (l.contains(" agent-resolved**") && date >= FIRST_CLAUSED_DECISION).then_some((date, l))
        })
        .filter_map(|(date, l)| {
            let (d, r) = (l.matches("Decision:").count(), l.matches("Revisit if:").count());
            (d != 1 || r != 1).then(|| {
                let title: String = l.split(" — ").nth(1).unwrap_or(l).chars().take(60).collect();
                format!("the Decision of {date} \"{title}\" holds {d} decision clauses and {r} revisit clauses, not one each")
            })
        })
        .collect()
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

    /// Sprint 209: a Decision split by an insert is named; one before 2026-10-25 is not held.
    #[test]
    fn a_decision_holds_one_decision_and_one_revisit_clause() {
        let whole = "- **2026-10-28 agent-resolved** — A (x). Decision: do it. Revisit if: it fails.";
        assert!(decisions_split(whole).is_empty());
        let split = "- **2026-10-28 agent-resolved** — A (x). The opening only.\n\
                     - **2026-10-28 agent-resolved** — B. Decision: b. Revisit if: b. Decision: a. Revisit if: a.";
        let named = decisions_split(split);
        assert_eq!(named.len(), 2, "{named:?}");
        assert!(
            named[0].contains("holds 0 decision clauses and 0 revisit clauses"),
            "{named:?}"
        );
        let old = "- **2026-10-09 agent-resolved** — Old. Revisit if: never.";
        assert!(decisions_split(old).is_empty());
    }

    /// Sprint 212 (reviewer-3's Medium 5 of its review of Sprint 211): a command quoted in double quotes is read as the
    /// shell ran it and classed; Sprint 211's owned breach, quoted so, is found and passes; an unowned one is named,
    /// and an owning phrase in another command's clause does not pass it.
    #[test]
    fn every_quoted_sed_is_classed_and_owned_where_it_is_quoted() {
        let breach = "on batch_record.rs, \"sed -i '' 's/^        executed: 3_541_969,$/        exits_from: \\\"the frozen \
                      host'\\\"'\\\"'s run to the same stop\\\",\\\\n        executed: Some(3_541_969),/' \
                      crates/w2m_vm/tests/batch_record.rs\", which made each of two lines two: a multi-line edit \
                      outside the Edit tool";
        let commands = quoted_commands(breach);
        assert_eq!(commands.len(), 1, "{commands:?}");
        assert!(
            commands[0].2.contains("exits_from: \"the frozen host'\"'\"'s run"),
            "{:?}",
            commands[0].2
        );
        assert!(
            edit_guard::refusal(&commands[0].2).is_some(),
            "the command makes one line two"
        );
        assert!(quoted_edits(211, breach).is_empty());
        // unowned, in double quotes
        let unowned = "\"sed -i '' '5a\\\\ x' f.rs\", one line added";
        assert_eq!(quoted_edits(212, unowned).len(), 1, "{:?}", quoted_edits(212, unowned));
        // owned in the first command's clause only: the second breach is named
        let two = "`sed -i '' '5a\\ x' f.rs`, a multi-line edit outside the Edit tool; then `sed -i '' '9a\\ y' g.rs`";
        let named = quoted_edits(212, two);
        assert_eq!(named.len(), 1, "{named:?}");
        assert!(named[0].contains("9a"), "{named:?}");
        // a one-line substitution passes unowned
        assert!(quoted_edits(212, "\"sed -i '' 's/a/b/' f.rs\"").is_empty());
    }

    /// Sprint 212 (reviewer-3's High 2 of its review of Sprint 211): the run a later entry cites for a close must have
    /// concluded success, or the close's own entry names it failed; a run not on file is named; no citation, pending.
    #[test]
    fn a_close_is_held_to_the_conclusion_of_its_ci_run() {
        let ci = "# header\n11\taaa\tsuccess\tt\n22\tbbb\tfailure\tt\n";
        let tail = "### Sprint 211\n- **Closed:** Sprint 210's CI run 11 (head aaa) passed.\n\
                    ### Sprint 212\n- **Closed:** Sprint 211's CI run (22, head bbb) failed.\n";
        assert!(ci_held(210, "an entry", tail, Some(ci)).is_empty());
        let named = ci_held(211, "an entry", tail, Some(ci));
        assert_eq!(named.len(), 1, "{named:?}");
        assert!(named[0].contains("its CI run 22 concluded failure"), "{named:?}");
        assert!(ci_held(211, "its run 22 failed, repaired later", tail, Some(ci)).is_empty());
        let unread = ci_held(210, "an entry", "Sprint 210's CI run 33 passed", Some(ci));
        assert!(unread[0].contains("33 is not in"), "{unread:?}");
        assert!(
            ci_held(212, "an entry", tail, Some(ci)).is_empty(),
            "no later entry cites 212's run: pending"
        );
    }

    /// A partial close is a close (Sprint 206: Sprint 205's "closed-partial:" commit failed the walk on CI).
    #[test]
    fn a_close_is_closed_or_closed_partial() {
        assert!(is_close(
            "Sprint 205 closed-partial: its Signal Report; BLACKBOARD",
            205
        ));
        assert!(is_close("Sprint 204 closed: its Signal Report; BLACKBOARD", 204));
        assert!(!is_close("Sprint 205 closed partial: its Signal Report", 205));
        assert!(!is_close("Sprint 2050 closed: x", 205));
    }

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
