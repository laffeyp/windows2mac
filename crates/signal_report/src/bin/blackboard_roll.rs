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

/// A pointer to the archive: the line `roll` leaves in the section.
const POINTER: &str = "- **Older reviews** — ";

/// The archive with `moved` (newest first) put before its older entries, or a new archive holding them.
fn archived(old: Option<&str>, moved: &[String]) -> String {
    let old = old.unwrap_or(
        "# Surfaced for review: archive\n\nReviews rolled out of `BLACKBOARD.md` `## Surfaced for review`, verbatim, \
         newest first (Sprint 232).\n\n",
    );
    let (head, older) = old.split_at(old.find("- **").unwrap_or(old.len()));
    format!("{head}{}{older}", moved.concat())
}

/// The pointer line, from the archive as it stands (Sprint 243): its reviews counted and their oldest and newest
/// dates, so a second roll replaces the line rather than adding one beside it.
fn pointer(archive_name: &str, archive: &str) -> String {
    let dates: Vec<&str> = archive
        .lines()
        .filter_map(|l| l.strip_prefix("- **"))
        .filter_map(|l| l.get(..10))
        .collect();
    let oldest = dates.iter().min().copied().unwrap_or("");
    let newest = dates.iter().max().copied().unwrap_or("");
    format!(
        "{POINTER}{} reviews, {oldest} to {newest}, moved verbatim to `{archive_name}`, newest first \
         (reviewer-2's Medium 5 of its review of 178-181: the last three reviews stay here).\n\n",
        dates.len()
    )
}

/// The record with the old reviews out and one pointer in their place, and the archive with them in: (record,
/// archive, reviews moved).
fn roll(
    blackboard: &str,
    keep: usize,
    archive_name: &str,
    archive_old: Option<&str>,
) -> Result<(String, String, usize), String> {
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
        return Ok((blackboard.to_string(), archive_old.unwrap_or_default().to_string(), 0));
    }
    let newest_first: Vec<String> = moved.iter().rev().map(|i| all[*i].clone()).collect();
    let archive = archived(archive_old, &newest_first);
    let mut section = preamble;
    for (i, e) in all.iter().enumerate() {
        if !moved.contains(&i) && !e.starts_with(POINTER) {
            section.push_str(e);
        }
    }
    if !section.ends_with("\n\n") {
        section.push('\n');
    }
    section.push_str(&pointer(archive_name, &archive));
    let text = format!("{}{section}{}", &blackboard[..start], &blackboard[end..]);
    Ok((text, archive, moved.len()))
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
    let into = root.join(archive);
    let old = fs::read_to_string(&into).ok();
    let (text, archived, moved) = roll(&blackboard, keep, archive, old.as_deref())?;
    if moved == 0 {
        println!("no review older than the newest {keep}");
        return Ok(());
    }
    fs::write(&into, archived).map_err(|e| format!("{archive}: {e}"))?;
    fs::write(&path, text).map_err(|e| format!("{}: {e}", path.display()))?;
    println!("{moved} reviews moved to {archive}");
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
        let (text, archive, moved) = roll(RECORD, 2, "docs/a.md", None).unwrap();
        assert_eq!(moved, 2);
        let entries: Vec<&str> = archive.split("\n- **").skip(1).collect();
        assert!(entries[0].contains("pasted code review b") && entries[1].contains("review a\n  - *Medium 1:* y\n\n"));
        assert!(text.contains("COMPREHENSION_AFFIRMATION") && text.contains("Sprint 178 closes partial"));
        assert!(text.contains("review c") && text.contains("review d") && !text.contains("review a"));
        assert!(text.contains("- **Older reviews** — 2 reviews, 2026-10-27 to 2026-10-28, moved verbatim"));
        assert!(text.ends_with("## Decisions\n- d\n"));
        let mut bytes: Vec<char> = text
            .lines()
            .filter(|l| !l.contains("Older reviews"))
            .flat_map(str::chars)
            .collect();
        let moved_text = &archive[archive.find("- **").unwrap()..];
        bytes.extend(moved_text.lines().flat_map(str::chars));
        let mut original: Vec<char> = RECORD.lines().flat_map(str::chars).collect();
        bytes.sort_unstable();
        original.sort_unstable();
        assert_eq!(bytes, original, "every character of the record is kept or moved");
    }

    #[test]
    fn nothing_moves_when_no_review_is_older_than_the_kept() {
        let (text, _, moved) = roll(RECORD, 4, "docs/a.md", None).unwrap();
        assert_eq!(moved, 0);
        assert_eq!(text, RECORD);
    }

    /// A second roll over a record that holds a pointer (Sprint 243; Deferred, Sprint 235: the tool wrote a second
    /// line beside the first): one pointer stays, its count and dates the archive's, the first roll's and the second's.
    #[test]
    fn a_second_roll_leaves_one_pointer_counting_the_whole_archive() {
        let (once, archive, _) = roll(RECORD, 2, "docs/a.md", None).unwrap();
        let again = once.replace(
            "- **2026-10-30 agent** — reviewer-3's review d\n",
            "- **2026-10-30 agent** — reviewer-3's review d\n- **2026-10-31 agent** — reviewer-3's review e\n",
        );
        let (twice, archive, moved) = roll(&again, 2, "docs/a.md", Some(&archive)).unwrap();
        assert_eq!(moved, 1, "review c leaves");
        let pointers: Vec<&str> = twice.lines().filter(|l| l.starts_with(POINTER)).collect();
        assert_eq!(pointers.len(), 1, "{pointers:?}");
        assert!(
            pointers[0].starts_with("- **Older reviews** — 3 reviews, 2026-10-27 to 2026-10-29, moved verbatim"),
            "{}",
            pointers[0]
        );
        assert!(
            archive.find("review c").unwrap() < archive.find("code review b").unwrap(),
            "newest first"
        );
    }

    /// The record's pointer is the one its archive gives: one line, the archive's count and span (Sprint 243).
    #[test]
    fn the_records_pointer_is_its_archives() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let name = "docs/surfaced-archive-2026-10-30.md";
        let archive = fs::read_to_string(root.join(name)).unwrap();
        let record = fs::read_to_string(root.join("BLACKBOARD.md")).unwrap();
        let lines: Vec<&str> = record.lines().filter(|l| l.starts_with(POINTER)).collect();
        assert_eq!(
            lines,
            [pointer(name, &archive).trim_end()],
            "BLACKBOARD.md's pointer to {name}"
        );
    }
}
