# Sprint 269 — Closed work rolled out of the record

```yaml
---
id: 269
status: done
phase: 6
pass_kind: functional
---
```

## scope

`BLACKBOARD.md` is 510,721 bytes, which every session reads whole (`sdd-kit-3/AGENTS.md` 26); reviewer-3's reviews at
261, 263, 264 and 266 each named the roll. The kit rolls the Sprint tail into `## Built` (`templates/BLACKBOARD.md`
58) and sets no archive for `## Built` or `## Decisions`. This card rolls out the closed work, on a Decision recorded
with its grounds: the Decisions marked closed or superseded, and the Built paragraphs of the sprints through 242, whose
plan steps 1 to 4 closed there (`docs/step-4-census-2026-10-28.md` "Step 4 closed"), each moved verbatim to a dated
archive by `blackboard_roll`, as the Surfaced reviews are.

- **Where it stands, by lookup (2026-10-30):**
  - `## Decisions` 263,264 bytes, 210 entries; 2 marked "*(Closed" or superseded by name ("Sequence after Sprint 214",
    closed at 243; "Sequence for step 5", closed at 268), 13,861 bytes.
  - `## Built` 124,707 bytes, 257 paragraphs; those naming a sprint through 242 hold 119,807.
  - `blackboard_roll --surfaced KEEP ARCHIVE` (`crates/signal_report/src/bin/blackboard_roll.rs`, Sprint 232) moves
    reviews verbatim, newest first, and leaves one pointer line counting the archive.
  - `signal_report --check`'s `unbuilt` requires a `## Built` paragraph, "— Sprint N (", for each sprint from 183 in
    the tail archive; `decisions_split` reads the Decisions' lines in `BLACKBOARD.md`.
- **The work:**
  - A Decision, agent-resolved: closed work leaves the record for dated archives; what a session must read stays.
  - `blackboard_roll --decisions ARCHIVE`: every Decision whose line holds "*(Closed" or "*(Superseded", moved verbatim
    to `docs/decisions-archive-2026-10-30.md`, with one pointer line.
  - `blackboard_roll --built THROUGH ARCHIVE`: every Built paragraph whose first sprint named is at or below THROUGH,
    moved verbatim to `docs/built-archive-2026-10-30.md`, with one pointer line; run with 242.
  - `unbuilt` reads the Built archive beside `## Built`, so a rolled paragraph still counts.
  - Tests: each roll moves only its kind and keeps the rest byte for byte; a second roll keeps one pointer; joined
    back, record and archive hold every line once.
- **Not here:** the older ordering Decisions that later ones supersede without naming them (most of `## Decisions`'
  250 KB that stays), each needing its reader's mark first; plan step 6.

## prerequisites

- 268 (closed)

## context_files

- `crates/signal_report/src/bin/blackboard_roll.rs`; `crates/signal_report/src/main.rs` (`unbuilt`, `decisions_split`)
- `sdd-kit-3/templates/BLACKBOARD.md` 58, 76; `sdd-kit-3/AGENTS.md` 26, 92

## signal contract

### Emits

- None; a record's bytes moved, held by tests.

### Invariants

- Every line of `BLACKBOARD.md` before the roll stands once in the record or an archive after it; `signal_report
  --check` passes before and after.

## artifact contract

### Files modified

- `crates/signal_report/src/bin/blackboard_roll.rs`, `crates/signal_report/src/main.rs`, `BLACKBOARD.md`,
  `docs/decisions-archive-2026-10-30.md` new, `docs/built-archive-2026-10-30.md` new

### Command exit codes

- `scripts/gates.sh` returns 0

## observation contract

- **Expected values:** `BLACKBOARD.md`'s bytes before and after; the lines moved, counted, and every one found in its
  archive by command.
- **Planted faults:**
  - (1) the Built roll's bound off by one: the test names the paragraph of Sprint 243 moved.
  - (2) `unbuilt` not reading the archive: the check names the rolled sprints as without a paragraph.

## done criteria

The closed Decisions and the Built paragraphs through Sprint 242 stand verbatim in their dated archives, the record
points to each, and every check passes on the rolled record.

## self-review checklist

- [x] Scope names one verifiable property: the closed work out of the record, verbatim.
- [x] context_files cite the tool, the checks and the kit.
- [x] Emits: none.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and two plants.
- [x] One concept: closed work rolled out.

## result

- **The tool** (f61221cc5, a01c2ae3e): `blackboard_roll` takes `--decisions ARCHIVE` and `--built THROUGH ARCHIVE`
  beside `--surfaced`; one `roll` over a `Kind`, whose `chosen` picks the entries each moves and which `run` and the
  tests both call; the Decisions and Built paragraphs go in the record's order, the reviews newest first as before;
  each roll leaves one pointer line where its first entry stood, a second roll replacing it.
- **The check:** `unbuilt` reads `docs/built-archive-2026-10-30.md` beside `## Built`.
- **The roll:** `BLACKBOARD.md` 510,900 to 372,645 bytes; 2 Decisions, "Sequence after Sprint 214" and "Sequence for
  step 5", to `docs/decisions-archive-2026-10-30.md` (19,025 bytes); 241 Built paragraphs, Sprints 000 to 242, to
  `docs/built-archive-2026-10-30.md` (119,960 bytes). The old record's 771 lines, sorted, equal the new record's
  less its two pointer lines and the archives' entries, sorted: every line kept once. `signal_report --check` passes
  on the rolled record.
- **The Decision** "Closed work rolled out of the record", agent-resolved, its grounds and revisit clause.
- **Not as the card said:** the tests first carried their own copies of the selection rules beside `run`'s, so an
  off-by-one in `run` would have passed them; found while setting plant 1, and the rules moved into `Kind::chosen`
  (a01c2ae3e).
- **The gates:** `scripts/gates.sh` exit 0 in 262 s on a01c2ae3e, copied from `store/gates/passes.tsv`; the timing stage 690 ns an exit at load 10.00.
- **Plant 1,** restored by `cp` and checked by `cmp`: the Built bound `s <= number` made `s <= number + 1`; the test
  failed, "assertion `left == right` failed, left: 3, right: 2", Sprint 243's paragraph moved.
- **Plant 2,** restored by `cp` and checked by `cmp`: `unbuilt` reading `## Built` alone; the test failed, "left:
  [184] right: []", a rolled sprint named as without its paragraph.
