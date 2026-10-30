# Sprint 243 — One pointer to the older reviews

```yaml
---
id: 243
status: active
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
