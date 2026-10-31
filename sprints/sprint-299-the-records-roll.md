# Sprint 299 — The record's roll

```yaml
---
id: 299
status: done
phase: 6
pass_kind: functional
---
```

## scope

Deferred (Sprint 290): BLACKBOARD past 400 KB, to be rolled before it passes 420 KB; Sprint 269's Observation 3 named
the older ordering Decisions, superseded and not marked so, as the bulk that stays. At 298's work commit the record
held 212 Decisions in 254,665 bytes, 67 of them naming an order, a sequence or the roadmap, and one marked.

- **The work:** each Decision read against the Decisions after it and the plan (`docs/rust-port-plan-2026-10-24.md`);
  each that a later one replaced marked `*(Superseded <date>, Sprint 299: by the Decision "<its title>")*`, each whose
  work is done or whose question is settled marked `*(Closed <date>, Sprint 299: <what closed it>)*`, the mark naming
  its ground; then `target/release/blackboard_roll --decisions docs/decisions-archive-2026-10-30.md`, which moves the
  marked ones verbatim in the record's order and writes the pointer line from the archive.
- **Not here:** rewording any Decision that stays; the Built and Surfaced rolls, which run at each close.

## prerequisites

- 298 (closed)

## context_files

- BLACKBOARD `## Decisions`, Deferred (Sprint 290); `docs/decisions-archive-2026-10-30.md`
- `crates/signal_report/src/bin/blackboard_roll.rs` (`--decisions`, `the_records_pointers_are_their_archives`)
- `docs/rust-port-plan-2026-10-24.md`

## signal contract

### Emits

- None; the record's bookkeeping.

### Invariants

- Every Decision moved verbatim; every Decision that stays unchanged but for its mark.

## artifact contract

### Files modified

- `BLACKBOARD.md`, `docs/decisions-archive-2026-10-30.md`; `crates/signal_report/src/bin/blackboard_roll.rs` (`is_closed`, found before the roll)

### Command exit codes

- `scripts/gates.sh` returns 0; `signal_report 298 --check` and `design_check` pass.

## observation contract

- **Expected values:** the record under 400 KB; each moved Decision's text in the archive equal to the record's before
  the roll, by `diff`; the count of marks equal to the count moved.
- **Planted faults:** (1) the pointer line's count changed by one: `the_records_pointers_are_their_archives` must name
  "BLACKBOARD.md's pointer to docs/decisions-archive-2026-10-30.md".

## done criteria

The record holds the Decisions in force; the rest are in the archive, verbatim, each with the mark that says why.

## self-review checklist

- [x] Scope names one verifiable property: the Decisions in force alone.
- [x] context_files cite the section, the tool, its test and the plan.
- [x] Emits: none.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and a plant changing what its judge reads.
- [x] One concept: the Decisions' roll, two files.

## result

- **A selector that took a quotation for a mark, found before the roll:** `blackboard_roll`'s `is_closed` took any
  Decision holding `*(Closed` or `*(Superseded`, and Sprint 269's Decision "Closed work rolled out of the record"
  quotes both, so the roll would have archived a Decision in force; it takes the mark with its date, `YYYY-MM-DD`,
  `a_quoted_mark_is_not_a_mark` holding a quotation and an undated mark unmarked. That Decision stayed.
- **The reading:** an agent read the 212 Decisions and listed 76, 55 replaced by a later one and 21 whose work was
  done, with grounds; each replacing user Decision it cited was read here (73, 83, 87, 158, 161, 171, 186, 197, 199,
  209, 228, 247, 296 in the record then). One was kept: the 2026-10-13 user Decision that the laptop is the Windows
  reference, a standing fact plan step 7 relies on, which the 2026-10-16 user Decision on "earlier direction" did not
  replace. Its report that no entry had continuation lines was wrong for the record (the 2026-10-29 user Decision has
  them) and right for the 76, checked line by line.
- **The change** (68acf79b4): 75 marks, each the entry's last annotation, `*(Superseded 2026-10-31, Sprint 299: by
  the <date> <who> Decision "<its opening words> …")*` for 54 and `*(Closed 2026-10-31, Sprint 299: <the sprints that
  ran it>)*` for 21, the targets' heads and opening words checked against the list first; then `blackboard_roll
  --decisions`, 75 moved.
- **Expected values:** BLACKBOARD 411,959 bytes before, 320,904 after; 75 lines changed by the marks, each holding its
  mark, the line count 562 unchanged; after the roll 483 lines, the 75, the old pointer and 4 blank separators gone,
  one pointer line new, "77 Decisions marked closed or superseded, 2026-10-03 to 2026-10-30"; each of the 75 found
  whole in the archive, its earlier text unchanged byte for byte.
- **The gates:** `scripts/gates.sh` exit 0 in 312 s, kept in `store/gates/passes.tsv` as sprint 299 at 68acf79b4 with root=8e6c93cca and "rust=crates python=0 samples=0", rdeps(signal_report), 39 tests, base=eba6b6447, Sprint 298's close; multi-vCPU: not run (Python gates not run)
- **Plant 1,** restored by `cp` and checked by `cmp`: the pointer's count written 78; `the_records_pointers_are_their_archives`
  failed, "BLACKBOARD.md's pointer to docs/decisions-archive-2026-10-30.md".

