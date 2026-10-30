# Sprint 243 — One pointer to the older reviews

```yaml
---
id: 243
status: done
phase: 6
pass_kind: functional
---
```

## scope

BLACKBOARD Deferred 2026-10-30 (Sprint 235), as annotated at 242: `blackboard_roll --surfaced` writes a new "Older
reviews" pointer line beside the one it finds, and Sprints 234, 235, 237 and 240 each merged the two by the Edit tool.
The entry's other half, the class gate's `Result` helper, is card 244: another crate, another concept.

- **Where it stands, by lookup (2026-10-30):**
  - `crates/signal_report/src/bin/blackboard_roll.rs` 40-83: `roll` keeps every entry that is not a review; the
    pointer line ("- **Older reviews** — …") is not one, so it stays, and 76-80 append a new one after it, counting
    only the reviews moved that time, its date span theirs.
  - The line today reads "33 reviews, 2026-10-27 to 2026-10-30", with a per-sprint tally kept by hand.
  - The archive, `docs/surfaced-archive-2026-10-30.md`, holds every moved review, newest first.
- **The readers, by search (2026-10-30; WORKING_AGREEMENT, a card lists its readers first):** WORKING_AGREEMENT 248
  (the roll's rule); BLACKBOARD's pointer line and Deferred 609; the tool's two unit tests; no other program.
- **The work:**
  - `roll` drops any "Older reviews" line it finds and writes one, derived from the archive as it will stand: its
    review count and its oldest and newest dates. The tally by sprint goes; the archive holds the entries.
  - A unit test: a record with a pointer already, rolled again, holds one pointer whose count and span are the
    archive's.
  - The pointer line rewritten by the tool once, the Deferred entry resolved in part.
- **Not here:** the `Result` helper (card 244).

## prerequisites

- 242 (closed)

## context_files

- `crates/signal_report/src/bin/blackboard_roll.rs`, `docs/surfaced-archive-2026-10-30.md`, WORKING_AGREEMENT 248

## signal contract

### Emits

- None: a tool over the record, no run.

### Invariants

- Every entry the tool kept before stays, in its order; every moved review lands in the archive verbatim.

## artifact contract

### Files modified

- `crates/signal_report/src/bin/blackboard_roll.rs`, `BLACKBOARD.md` (the pointer line, Deferred)

### Command exit codes

- `scripts/gates.sh` returns 0

## observation contract

- **Expected values:** after a roll, one "Older reviews" line, its count the archive's 33 reviews and its span
  2026-10-27 to 2026-10-30.
- **Planted faults:**
  1. The old pointer kept: the unit test names two pointer lines.
  2. The count taken from the reviews moved that time, not the archive: the unit test names the count.
  3. The span's newest date taken as the oldest: the unit test names the span.

## done criteria

A roll leaves one pointer line, and its count and dates are the archive's.

## self-review checklist

- [x] Scope names one verifiable property: one pointer, the archive's count and span.
- [x] context_files cite the tool, the archive and the rule.
- [x] Emits: none, stated with its reason.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and three plants.
- [x] One concept: the pointer line.

## result

- **The tool:** `roll` takes the archive's text and returns the new archive with the record; `archived` puts the
  moved reviews, newest first, before the archive's older ones; `pointer` counts the archive's entries and takes their
  oldest and newest dates. `roll` drops every line beginning `POINTER` and writes one. `run` reads and writes both
  files around it.
- **The tests:** the two earlier tests take the new return; `a_second_roll_leaves_one_pointer_counting_the_whole_archive`
  rolls twice and finds one pointer, "3 reviews, 2026-10-27 to 2026-10-29"; `the_records_pointer_is_its_archives`
  holds BLACKBOARD's line to `pointer` over `docs/surfaced-archive-2026-10-30.md`. Its first run named the hand-kept
  line beside the derived one, "33 reviews, 2026-10-27 to 2026-10-30" both, the tally by sprint the difference.
- **Departure:** the card planned the tool to rewrite the live line once, but `roll` returns before writing when no
  review moves, as today; the line was set to the derived form by the Edit tool, and the new test holds it there.
- **Deferred (Sprint 235):** resolved in part; the `Result` helper is card 244's.
- **The gates:** `scripts/gates.sh` exit 0 in 219 s on 8cf428d3d, "rust=crates python=0 samples=0", 33 tests.
- **Plant 1** (`roll` keeping the old pointer, `&& !e.starts_with(POINTER)` dropped at 95): "assertion `left == right`
  failed: [\"- **Older reviews** — 2 reviews, 2026-10-27 to 2026-10-28, …\", \"- **…\"] left: 2 right: 1".
- **Plant 2** (`pointer`'s count `dates.len().min(2)`): "- **Older reviews** — 2 reviews, 2026-10-27 to 2026-10-29",
  and `the_records_pointer_is_its_archives` named the live line, 33 against 2.
- **Plant 3** (`pointer`'s newest date taken by `min`): "3 reviews, 2026-10-27 to 2026-10-27", and the first test's
  "2 reviews, 2026-10-27 to 2026-10-28" not found.
- Each plant's `git diff --stat` showed its one line first; each was restored by `cp`, and `cmp` found the file equal
  to its copy.
- **The tool on the record:** at this close, the review at 243 added, `blackboard_roll --surfaced 3` moved the review
  at 235 and left one line, "34 reviews, 2026-10-27 to 2026-10-30", with `the_records_pointer_is_its_archives` passing.
- **A `sed` by a line number that had moved:** the work commit's Deferred annotation was a `sed` at line 609, where
  the entry had stood at 242's close; 242's Built paragraph moved it to 610, the `sed` matched nothing, and its
  `grep -c` printed 0, unread. The annotation went in at the close by the Edit tool, anchored on its text.
