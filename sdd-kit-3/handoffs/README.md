# handoffs — session-close statements of what happened

*Instantiated 2026-08-17 in sdd-kit-2; carried into sdd-kit-3 on 2026-09-27. A project that adopts the kit writes its handoffs to `handoffs/` at the project root, following this README and the template, so the kit folder stays read-only and each project's session history stays with the project.*

---

## What a handoff is

A single markdown file that states, in full, what one session did. Written once, at the end of that session — either because the human is closing the session, or because the context window is nearly full and the next turn will be a compaction. One handoff per session, dated by the day the session ends.

Not the BLACKBOARD. The BLACKBOARD is per-project, single-writer-per-section, append-only, and it records artifacts + decisions + sprint tail. A handoff is per-session, whole-story, and it records what the session's operator + agent actually did — including the false starts, the reverts, the corrections, the moments the agent got redirected.

Not the KIT_DIARY either. KIT_DIARY captures what the kit did well and what got in the way, per increment. A handoff can cite a diary entry but is not a substitute for one.

## When to write one

Two triggers, either sufficient:

- The human declares the session is ending.
- The agent judges the context window is close to full and the next exchange will drop material — write the handoff before the compaction, not after.

Write the handoff when the session covered more ground than any single artifact captures. Short sessions with one clean sprint close do not need one; the sprint card and the BLACKBOARD entry are the record.

## What goes in it

Prose. Full sentences. Every claim carries a fact — a file path, a commit hash, a decision, a name, the actual error. See `TEMPLATE-handoff.md` for the shape. Sections are guidance, not gates; drop an empty section rather than write "N/A".

Load-bearing, in order:

- What the session was working on and why.
- What actually shipped — commits, files, artifacts, with paths.
- What got surfaced or reverted, and why.
- What is now open — user-only questions awaiting an answer, proposals left open, pending sprints — with pointers.
- What the next session needs to read first to pick up cleanly.

## What stays out

- Reassurances, celebrations, meta-commentary about the session going well or poorly.
- "Next steps" as hedged recommendations. State what the next session will do first and on which grounds (the pending sprint card, the open Decision); the human can redirect.
- Anything already recorded elsewhere (BLACKBOARD entries, diary entries, sprint cards, review files) — cite by path, do not restate.
- Emojis. Markers only.

## Naming and place

`handoffs/handoff-YYYY-MM-DD.md`. One per day at most; if a day has two sessions worth capturing, add `-am` / `-pm` or a short slug. Never overwrite an earlier handoff — new sessions get new files, per hard rule 12.

The folder is not indexed. Files sort by date on their own.

---

*README.md for handoffs. The convention: a whole-session statement written at session close or before context compaction, in prose, with facts. Not the BLACKBOARD, not the diary. One folder, this one, kit-only.*
