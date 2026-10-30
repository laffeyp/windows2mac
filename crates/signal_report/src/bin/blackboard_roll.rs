// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! `blackboard_roll --surfaced KEEP ARCHIVE [ROOT]` (Sprint 232): moves the reviews in `BLACKBOARD.md`'s `## Surfaced
//! for review`, all but the newest KEEP, verbatim to ARCHIVE, newest first, and leaves one line in their place. A
//! review is an entry whose text after its date opens "reviewer-" or "The user's pasted code review". The kit's other
//! entries stay: comprehension affirmations, which a new session looks for (`sdd-kit-3/AGENTS.md` 27), user-only
//! questions and their answers, partial closes and sweeps. Reviewer-2's Medium 5 of its review of 178-181 ruled the
//! roll; the move is a program's so that 100 KB of text is not retyped through the Edit tool.

use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

const SURFACED: &str = "## Surfaced for review";

/// The section's entries: each from its "- **" line to the next one's, blank lines with the entry before them; the
/// text before the first entry is the preamble. Joined again, they are the section's text.
fn entries(section: &str) -> (String, Vec<String>) {
    let mut preamble = String::new();
    let mut out: Vec<String> = Vec::new();
    for line in section.split_inclusive('\n') {
        if line.starts_with("- **") {
            out.push(line.to_string());
        } else if let Some(last) = out.last_mut() {
            last.push_str(line);
        } else {
            preamble.push_str(line);
        }
    }
    (preamble, out)
}

fn is_review(entry: &str) -> bool {
    entry
        .split_once("** — ")
        .is_some_and(|(_, text)| text.starts_with("reviewer-") || text.starts_with("The user's pasted code review"))
}

/// The record with the old reviews out, the moved entries newest first, and the line left in their place.
fn roll(blackboard: &str, keep: usize, archive: &str) -> Result<(String, Vec<String>), String> {
    let start = blackboard
        .find(&format!("{SURFACED}\n"))
        .ok_or(format!("no \"{SURFACED}\""))?
        + SURFACED.len()
        + 1;
    let end = blackboard[start..]
        .find("\n## ")
        .map_or(blackboard.len(), |i| start + i + 1);
    let (preamble, all) = entries(&blackboard[start..end]);
    let reviews: Vec<usize> = (0..all.len()).filter(|i| is_review(&all[*i])).collect();
    let moved: Vec<usize> = reviews[..reviews.len().saturating_sub(keep)].to_vec();
    if moved.is_empty() {
        return Ok((blackboard.to_string(), Vec::new()));
    }
    let (first, last) = (
        all[moved[0]]
            .split(" agent** — ")
            .next()
            .unwrap_or("")
            .trim_start_matches("- **"),
        all[*moved.last().unwrap()]
            .split(" agent** — ")
            .next()
            .unwrap_or("")
            .trim_start_matches("- **"),
    );
    let mut section = preamble;
    for (i, e) in all.iter().enumerate() {
        if !moved.contains(&i) {
            section.push_str(e);
        }
    }
    if !section.ends_with("\n\n") {
        section.push('\n');
    }
    section.push_str(&format!(
        "- **Older reviews** — {} reviews, {first} to {last}, moved verbatim to `{archive}`, newest first (Sprint 232; \
         reviewer-2's Medium 5 of its review of 178-181: the last three reviews stay here).\n\n",
        moved.len()
    ));
    let text = format!("{}{section}{}", &blackboard[..start], &blackboard[end..]);
    Ok((text, moved.iter().rev().map(|i| all[*i].clone()).collect()))
}

fn run() -> Result<(), String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [flag, keep, archive, rest @ ..] = args.as_slice() else {
        return Err("usage: blackboard_roll --surfaced KEEP ARCHIVE [ROOT]".into());
    };
    if flag != "--surfaced" {
        return Err("usage: blackboard_roll --surfaced KEEP ARCHIVE [ROOT]".into());
    }
    let keep: usize = keep.parse().map_err(|e| format!("KEEP {keep}: {e}"))?;
    let root = PathBuf::from(rest.first().map_or(".", String::as_str));
    let path = root.join("BLACKBOARD.md");
    let blackboard = fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let (text, moved) = roll(&blackboard, keep, archive)?;
    if moved.is_empty() {
        println!("no review older than the newest {keep}");
        return Ok(());
    }
    let into = root.join(archive);
    let old = fs::read_to_string(&into).unwrap_or_else(|_| {
        "# Surfaced for review: archive\n\nReviews rolled out of `BLACKBOARD.md` `## Surfaced for review`, verbatim, \
         newest first (Sprint 232).\n\n"
            .to_string()
    });
    let (head, older) = old.split_at(old.find("- **").unwrap_or(old.len()));
    fs::write(&into, format!("{head}{}{older}", moved.concat())).map_err(|e| format!("{archive}: {e}"))?;
    fs::write(&path, text).map_err(|e| format!("{}: {e}", path.display()))?;
    println!("{} reviews moved to {archive}", moved.len());
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("blackboard_roll: {e}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RECORD: &str = "# B\n\n## Surfaced for review\n\n- **2026-10-03 agent** — COMPREHENSION_AFFIRMATION: x\n\
                          - **2026-10-27 agent** — reviewer-2's review a\n  - *Medium 1:* y\n\n\
                          - **2026-10-28 agent** — The user's pasted code review b\n\
                          - **2026-10-28 agent** — Sprint 178 closes partial\n\
                          - **2026-10-29 agent** — reviewer-3's review c\n- **2026-10-30 agent** — reviewer-3's review d\n\n\
                          ## Decisions\n- d\n";

    /// The two oldest of four reviews leave, newest first; everything else stays in its order; the kept and moved
    /// entries hold every byte of the section but the new line.
    #[test]
    fn the_oldest_reviews_leave_and_nothing_else_moves() {
        let (text, moved) = roll(RECORD, 2, "docs/a.md").unwrap();
        assert_eq!(moved.len(), 2);
        assert!(moved[0].contains("pasted code review b") && moved[1].contains("review a\n  - *Medium 1:* y\n\n"));
        assert!(text.contains("COMPREHENSION_AFFIRMATION") && text.contains("Sprint 178 closes partial"));
        assert!(text.contains("review c") && text.contains("review d") && !text.contains("review a"));
        assert!(text.contains("- **Older reviews** — 2 reviews, 2026-10-27 to 2026-10-28, moved verbatim"));
        assert!(text.ends_with("## Decisions\n- d\n"));
        let mut bytes: Vec<char> = text
            .lines()
            .filter(|l| !l.contains("Older reviews"))
            .flat_map(str::chars)
            .collect();
        bytes.extend(moved.concat().lines().flat_map(str::chars));
        let mut original: Vec<char> = RECORD.lines().flat_map(str::chars).collect();
        bytes.sort_unstable();
        original.sort_unstable();
        assert_eq!(bytes, original, "every character of the record is kept or moved");
    }

    #[test]
    fn nothing_moves_when_no_review_is_older_than_the_kept() {
        let (text, moved) = roll(RECORD, 4, "docs/a.md").unwrap();
        assert!(moved.is_empty());
        assert_eq!(text, RECORD);
    }
}
