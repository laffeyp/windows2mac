# Sprint 299 — The record's roll

```yaml
---
id: 299
status: active
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

- `BLACKBOARD.md`, `docs/decisions-archive-2026-10-30.md`

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

