// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! `blackboard_roll --surfaced KEEP ARCHIVE [ROOT]` (Sprint 232): moves the reviews in `BLACKBOARD.md`'s `## Surfaced
//! for review`, all but the newest KEEP, verbatim to ARCHIVE, newest first, and leaves one line in their place. A
//! review is an entry whose text after its date opens "reviewer-" or "The user's pasted code review". The kit's other
//! entries stay: comprehension affirmations, which a new session looks for (`sdd-kit-3/AGENTS.md` 27), user-only
//! questions and their answers, partial closes and sweeps. Reviewer-2's Medium 5 of its review of 178-181 ruled the
//! roll; the move is a program's so that 100 KB of text is not retyped through the Edit tool.
//!
//! `blackboard_roll --decisions ARCHIVE [ROOT]` and `--built THROUGH ARCHIVE [ROOT]` (Sprint 269, the Decision "Closed
//! work rolled out of the record"): the Decisions whose entry holds "*(Closed" or "*(Superseded", and the `## Built`
//! paragraphs whose first sprint named is at or below THROUGH, moved verbatim to ARCHIVE in the record's order, one
//! line left where the first of them stood.

use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

/// What a roll moves: from which section, under which pointer line, into an archive of which order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Surfaced,
    Decisions,
    Built,
}

impl Kind {
    fn section(self) -> &'static str {
        match self {
            Kind::Surfaced => "## Surfaced for review",
            Kind::Decisions => "## Decisions",
            Kind::Built => "## Built",
        }
    }

    /// The pointer line's opening, by which a later roll finds and replaces it.
    fn pointer(self) -> &'static str {
        match self {
            Kind::Surfaced => "- **Older reviews** — ",
            Kind::Decisions => "- **Closed Decisions** — ",
            Kind::Built => "- **Older Built paragraphs** — ",
        }
    }

    /// A new archive's opening text.
    fn heading(self) -> &'static str {
        match self {
            Kind::Surfaced => {
                "# Surfaced for review: archive\n\nReviews rolled out of `BLACKBOARD.md` `## Surfaced for review`, \
                 verbatim, newest first (Sprint 232).\n\n"
            }
            Kind::Decisions => {
                "# Decisions: archive\n\nDecisions closed or superseded, rolled out of `BLACKBOARD.md` `## Decisions` \
                 verbatim, in the record's order (Sprint 269).\n\n"
            }
            Kind::Built => {
                "# Built: archive\n\nParagraphs of `BLACKBOARD.md` `## Built` whose plan steps have closed, rolled out \
                 verbatim, in the record's order (Sprint 269).\n\n"
            }
        }
    }

    /// The reviews go newest first; the Decisions and the Built paragraphs keep the record's order, oldest first.
    fn newest_first(self) -> bool {
        self == Kind::Surfaced
    }

    /// The entries of the section that leave, by index: the reviews but the newest `number`; the Decisions marked
    /// closed or superseded (`number` unused); the Built paragraphs whose first sprint is at or below `number`.
    fn chosen(self, all: &[String], number: u32) -> Vec<usize> {
        let picked = |keep: &dyn Fn(&str) -> bool| (0..all.len()).filter(|i| keep(&all[*i])).collect::<Vec<usize>>();
        match self {
            Kind::Surfaced => {
                let reviews = picked(&is_review);
                reviews[..reviews.len().saturating_sub(number as usize)].to_vec()
            }
            Kind::Decisions => picked(&is_closed),
            Kind::Built => {
                picked(&|e| !e.starts_with(Kind::Built.pointer()) && first_sprint(e).is_some_and(|s| s <= number))
            }
        }
    }
}

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

/// A Decision marked closed or superseded in one of its annotations: `*(Closed ` or `*(Superseded ` and a date,
/// `YYYY-MM-DD`. A Decision that quotes the mark, as Sprint 269's names `"*(Closed"`, is not marked (Sprint 299).
fn is_closed(entry: &str) -> bool {
    ["*(Closed ", "*(Superseded "].iter().any(|mark| {
        entry.match_indices(mark).any(|(at, _)| {
            let date = &entry.as_bytes()[at + mark.len()..];
            date.len() >= 10
                && date[..10].iter().enumerate().all(|(i, b)| {
                    if i == 4 || i == 7 {
                        *b == b'-'
                    } else {
                        b.is_ascii_digit()
                    }
                })
        })
    })
}

/// The first sprint a Built paragraph names, "Sprint N" or "Sprints N-M".
fn first_sprint(entry: &str) -> Option<u32> {
    let at = entry
        .find("Sprint ")
        .map(|i| i + 7)
        .or_else(|| entry.find("Sprints ").map(|i| i + 8))?;
    let digits: String = entry[at..].chars().take_while(char::is_ascii_digit).collect();
    digits.parse().ok()
}

/// The archive with `moved` in it: before its older entries when newest first, else after its entries.
fn archived(kind: Kind, old: Option<&str>, moved: &[String]) -> String {
    let old = old.unwrap_or(kind.heading());
    if kind.newest_first() {
        let (head, older) = old.split_at(old.find("- **").unwrap_or(old.len()));
        format!("{head}{}{older}", moved.concat())
    } else {
        let mut text = old.to_string();
        if !text.ends_with("\n\n") && text.contains("- **") && !text.ends_with('\n') {
            text.push('\n');
        }
        text.push_str(&moved.concat());
        text
    }
}

/// The pointer line, from the archive as it stands (Sprint 243): its entries counted and their span, dates for reviews
/// and Decisions and sprints for Built paragraphs, so a second roll replaces the line rather than adding one beside it.
fn pointer(kind: Kind, archive_name: &str, archive: &str) -> String {
    let (_, all) = entries(&archive[archive.find("- **").unwrap_or(archive.len())..]);
    let span = |values: Vec<String>| {
        let oldest = values.iter().min().cloned().unwrap_or_default();
        let newest = values.iter().max().cloned().unwrap_or_default();
        (oldest, newest)
    };
    match kind {
        Kind::Surfaced => {
            let (oldest, newest) = span(all.iter().filter_map(|e| e.get(4..14)).map(str::to_string).collect());
            format!(
                "{}{} reviews, {oldest} to {newest}, moved verbatim to `{archive_name}`, newest first \
                 (reviewer-2's Medium 5 of its review of 178-181: the last three reviews stay here).\n\n",
                kind.pointer(),
                all.len()
            )
        }
        Kind::Decisions => {
            let (oldest, newest) = span(all.iter().filter_map(|e| e.get(4..14)).map(str::to_string).collect());
            format!(
                "{}{} Decisions marked closed or superseded, {oldest} to {newest}, moved verbatim to `{archive_name}` \
                 (Sprint 269, the Decision \"Closed work rolled out of the record\").\n",
                kind.pointer(),
                all.len()
            )
        }
        Kind::Built => {
            let sprints: Vec<u32> = all.iter().filter_map(|e| first_sprint(e)).collect();
            format!(
                "{}{} paragraphs, Sprints {:03} to {:03}, whose plan steps have closed, moved verbatim to \
                 `{archive_name}` (Sprint 269, the Decision \"Closed work rolled out of the record\").\n",
                kind.pointer(),
                all.len(),
                sprints.iter().min().copied().unwrap_or(0),
                sprints.iter().max().copied().unwrap_or(0)
            )
        }
    }
}

/// The record with the entries `Kind::chosen` picks out of `kind`'s section by `number` and one pointer in their
/// place, and the archive with them in: (record, archive, entries moved).
fn roll(
    blackboard: &str,
    kind: Kind,
    number: u32,
    archive_name: &str,
    archive_old: Option<&str>,
) -> Result<(String, String, usize), String> {
    let section = kind.section();
    let start = blackboard
        .find(&format!("{section}\n"))
        .ok_or(format!("no \"{section}\""))?
        + section.len()
        + 1;
    let end = blackboard[start..]
        .find("\n## ")
        .map_or(blackboard.len(), |i| start + i + 1);
    let (preamble, all) = entries(&blackboard[start..end]);
    let moved = kind.chosen(&all, number);
    if moved.is_empty() {
        return Ok((blackboard.to_string(), archive_old.unwrap_or_default().to_string(), 0));
    }
    let in_order: Vec<String> = moved.iter().map(|i| all[*i].clone()).collect();
    let in_order: Vec<String> = if kind.newest_first() {
        in_order.into_iter().rev().collect()
    } else {
        in_order
    };
    let archive = archived(kind, archive_old, &in_order);
    let line = pointer(kind, archive_name, &archive);
    let mut text_section = preamble;
    if kind == Kind::Surfaced {
        // the reviews' pointer stands last in the section (Sprint 232)
        for (i, e) in all.iter().enumerate() {
            if !moved.contains(&i) && !e.starts_with(kind.pointer()) {
                text_section.push_str(e);
            }
        }
        if !text_section.ends_with("\n\n") {
            text_section.push('\n');
        }
        text_section.push_str(&line);
    } else {
        // the pointer stands where the old one, or else the first entry moved, stood
        let at = all
            .iter()
            .position(|e| e.starts_with(kind.pointer()))
            .unwrap_or(moved[0]);
        for (i, e) in all.iter().enumerate() {
            if i == at {
                text_section.push_str(&line);
            }
            if !moved.contains(&i) && !e.starts_with(kind.pointer()) {
                text_section.push_str(e);
            }
        }
        // a blank line belongs to the entry before it, so moving the section's last entry took the section's closing
        // blank line with it; the section ends as it did
        if blackboard[start..end].ends_with("\n\n") && !text_section.ends_with("\n\n") {
            text_section.push('\n');
        }
    }
    let text = format!("{}{text_section}{}", &blackboard[..start], &blackboard[end..]);
    Ok((text, archive, moved.len()))
}

const USAGE: &str = "usage: blackboard_roll --surfaced KEEP ARCHIVE [ROOT] | --decisions ARCHIVE [ROOT] | --built \
                     THROUGH ARCHIVE [ROOT]";

fn run() -> Result<(), String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (kind, number, archive, rest): (Kind, Option<u32>, &String, &[String]) = match args.as_slice() {
        [flag, keep, archive, rest @ ..] if flag == "--surfaced" => (
            Kind::Surfaced,
            Some(keep.parse().map_err(|e| format!("KEEP {keep}: {e}"))?),
            archive,
            rest,
        ),
        [flag, through, archive, rest @ ..] if flag == "--built" => (
            Kind::Built,
            Some(through.parse().map_err(|e| format!("THROUGH {through}: {e}"))?),
            archive,
            rest,
        ),
        [flag, archive, rest @ ..] if flag == "--decisions" => (Kind::Decisions, None, archive, rest),
        _ => return Err(USAGE.into()),
    };
    let root = PathBuf::from(rest.first().map_or(".", String::as_str));
    let path = root.join("BLACKBOARD.md");
    let blackboard = fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let into = root.join(archive);
    let old = fs::read_to_string(&into).ok();
    let (text, archived, moved) = roll(&blackboard, kind, number.unwrap_or(0), archive, old.as_deref())?;
    if moved == 0 {
        println!("nothing to move from {}", kind.section());
        return Ok(());
    }
    fs::write(&into, archived).map_err(|e| format!("{archive}: {e}"))?;
    fs::write(&path, text).map_err(|e| format!("{}: {e}", path.display()))?;
    println!("{moved} entries of {} moved to {archive}", kind.section());
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
        let (text, archive, moved) = roll(RECORD, Kind::Surfaced, 2, "docs/a.md", None).unwrap();
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
        let (text, _, moved) = roll(RECORD, Kind::Surfaced, 4, "docs/a.md", None).unwrap();
        assert_eq!(moved, 0);
        assert_eq!(text, RECORD);
    }

    /// A second roll over a record that holds a pointer (Sprint 243; Deferred, Sprint 235: the tool wrote a second
    /// line beside the first): one pointer stays, its count and dates the archive's, the first roll's and the second's.
    #[test]
    fn a_second_roll_leaves_one_pointer_counting_the_whole_archive() {
        let (once, archive, _) = roll(RECORD, Kind::Surfaced, 2, "docs/a.md", None).unwrap();
        let again = once.replace(
            "- **2026-10-30 agent** — reviewer-3's review d\n",
            "- **2026-10-30 agent** — reviewer-3's review d\n- **2026-10-31 agent** — reviewer-3's review e\n",
        );
        let (twice, archive, moved) = roll(&again, Kind::Surfaced, 2, "docs/a.md", Some(&archive)).unwrap();
        assert_eq!(moved, 1, "review c leaves");
        let pointers: Vec<&str> = twice
            .lines()
            .filter(|l| l.starts_with(Kind::Surfaced.pointer()))
            .collect();
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

    /// The record's pointers are the ones their archives give: one line each, the archive's count and span (Sprints 243,
    /// 269).
    #[test]
    fn the_records_pointers_are_their_archives() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let record = fs::read_to_string(root.join("BLACKBOARD.md")).unwrap();
        for (kind, name) in [
            (Kind::Surfaced, "docs/surfaced-archive-2026-10-30.md"),
            (Kind::Decisions, "docs/decisions-archive-2026-10-30.md"),
            (Kind::Built, "docs/built-archive-2026-10-30.md"),
        ] {
            let Ok(archive) = fs::read_to_string(root.join(name)) else {
                assert!(
                    !record.contains(kind.pointer()),
                    "a pointer to {name}, which does not exist"
                );
                continue;
            };
            let lines: Vec<&str> = record.lines().filter(|l| l.starts_with(kind.pointer())).collect();
            assert_eq!(
                lines,
                [pointer(kind, name, &archive).trim_end()],
                "BLACKBOARD.md's pointer to {name}"
            );
        }
    }

    const DECIDED: &str = "# B\n\n## Decisions\n\n- **2026-10-03 user** — scope\n\
                           - **2026-10-20 agent-resolved** — A. Decision: a. Revisit if: x. *(Closed 2026-10-30, Sprint 243: done.)*\n\
                           - **2026-10-21 agent-resolved** — B. Decision: b. Revisit if: y.\n\
                           - **2026-10-22 agent-resolved** — C. Decision: c. Revisit if: z. *(Superseded 2026-10-30 by D.)*\n\n\
                           ## Built\n\n- **2026-10-05** — Sprint 000 (zero) a.\n- **2026-10-06** — Sprint 241 (b) b.\n\
                           - **2026-10-07** — Sprint 242 (c) c.\n- **2026-10-08** — Sprint 243 (d) d.\n\n## Sprint tail\n";

    /// A mark is the annotation with its date; a Decision quoting the mark, as Sprint 269's does, stays (Sprint 299).
    #[test]
    fn a_quoted_mark_is_not_a_mark() {
        assert!(is_closed("A. *(Closed 2026-10-30, Sprint 243: done.)*"));
        assert!(is_closed(
            "C. *(Superseded 2026-10-31, Sprint 299: by the Decision \"D\")*"
        ));
        assert!(!is_closed(
            "the Decisions whose entry holds \"*(Closed\" or \"*(Superseded\" to the archive"
        ));
        assert!(!is_closed("*(Closed by hand)*"));
    }

    /// The closed and superseded Decisions leave in the record's order, the pointer where the first stood; the rest,
    /// the user's scope first, stay byte for byte (Sprint 269).
    #[test]
    fn the_closed_decisions_leave_and_the_rest_stay() {
        let (text, archive, moved) = roll(DECIDED, Kind::Decisions, 0, "docs/d.md", None).unwrap();
        assert_eq!(moved, 2);
        assert!(
            archive.find("— A.").unwrap() < archive.find("— C.").unwrap(),
            "the record's order"
        );
        let section = &text[text.find("## Decisions").unwrap()..text.find("## Built").unwrap()];
        assert_eq!(
            section,
            "## Decisions\n\n- **2026-10-03 user** — scope\n- **Closed Decisions** — 2 Decisions marked closed or \
             superseded, 2026-10-20 to 2026-10-22, moved verbatim to `docs/d.md` (Sprint 269, the Decision \"Closed \
             work rolled out of the record\").\n- **2026-10-21 agent-resolved** — B. Decision: b. Revisit if: y.\n\n"
        );
        assert!(
            text.ends_with(&DECIDED[DECIDED.find("## Built").unwrap()..]),
            "the next sections untouched"
        );
    }

    /// The Built paragraphs through Sprint 242 leave, 243 stays; a second roll adds to the archive's end and keeps
    /// one pointer (Sprint 269; plant 1 moves 243 by an off-by-one bound).
    #[test]
    fn the_built_paragraphs_through_the_bound_leave_and_a_second_roll_keeps_one_pointer() {
        let (text, archive, moved) = roll(DECIDED, Kind::Built, 241, "docs/b.md", None).unwrap();
        assert_eq!(moved, 2);
        let (again, archive, moved) = roll(&text, Kind::Built, 242, "docs/b.md", Some(&archive)).unwrap();
        assert_eq!(moved, 1);
        let built = &again[again.find("## Built").unwrap()..again.find("## Sprint tail").unwrap()];
        assert_eq!(
            built,
            "## Built\n\n- **Older Built paragraphs** — 3 paragraphs, Sprints 000 to 242, whose plan steps have \
             closed, moved verbatim to `docs/b.md` (Sprint 269, the Decision \"Closed work rolled out of the \
             record\").\n- **2026-10-08** — Sprint 243 (d) d.\n\n"
        );
        let order: Vec<usize> = ["Sprint 000", "Sprint 241", "Sprint 242"]
            .iter()
            .map(|s| archive.find(s).unwrap())
            .collect();
        assert!(order.windows(2).all(|w| w[0] < w[1]), "the record's order");
        assert!(!archive.contains("Sprint 243"), "Sprint 243's paragraph stays");
    }
}
