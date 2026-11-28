# Sprint 378 — A frozen family held to a contract re-taken on the laptop

```yaml
---
id: 378
status: active
phase: 6
pass_kind: functional
---
```

## scope

The Decision "Which Windows w2m answers as, the reference being the laptop" (Sprint 374) sets w2m to answer as the
laptop, and names tables still taken from GitHub's Server runner, `tools/guest_runtime/version.c` and `system_info.c`
first. Re-taking a table moves the families that read it (`version`, `system_info`, `files_more`): their judge,
`crates/w2m_vm/tests/family_runs.rs`, holds each run to the contract version in
`store/family_runs/known-2026-10-29.tsv`, and for a frozen family `crates/w2m_vm/src/bin/family_known.rs` (`contract`,
165-193) takes that version from the family record's first COMPARISON_COMPLETED, the runner's 0.1.0. A joined family
already moves by its row in `store/family_runs/joined-2026-11-27.tsv` (`named_sync_objects` at 0.2.0, Sprint 374); a
frozen family has no such route.

- **The route.** `store/family_runs/retaken-2026-11-27.tsv` (family, contract@version, the laptop run its rows come
  from), read by `family_known`: a family named there is held to that contract version, the rest as before.
  `known-2026-10-29.tsv` regenerated, its contract column alone changing, and only for families the new table names.
- **Its test.** A row in the new table moves that family's contract in `family_known`'s output and nothing else; a
  row naming a family the index lacks, or a contract file that does not exist, is refused naming it.
- **This card moves no family.** The new table ships empty of families, or with one only if a card's contract exists
  for it; the table re-takes are cards 379 (`version`: `version.c`), 380 (`system_info` and `files_more`:
  `system_info.c`, the system directory measured on the laptop), 381 (`system_messages.json`, `windows_exports.json`),
  382 (`recorded_folders.json`, the display calls). No API id is added.

## prerequisites

- 377 (closed)

## context_files

- `crates/w2m_vm/src/bin/family_known.rs`, `crates/w2m_vm/tests/family_runs.rs`.
- `store/family_runs/known-2026-10-29.tsv`, `store/family_runs/joined-2026-11-27.tsv` (their first lines).
- `grep -n "Which Windows w2m answers as\|The order after Sprint 377" BLACKBOARD.md`.
- `WORKING_AGREEMENT.md` lines 137-266.

## signal contract

### Emits

- None: the route changes which contract version a judge reads; no run is recorded.

### Invariants

- Every run as held; `known-2026-10-29.tsv` byte-equal to HEAD's while the new table names no family; G3 conforms;
  `crates/guest_runtime/src/api_ids.txt` unchanged.

## artifact contract

### Files created

- `store/family_runs/retaken-2026-11-27.tsv`

### Files modified

- `crates/w2m_vm/src/bin/family_known.rs`, with its tests and others only where the class requires, reported to main
  when found. One concept: the route by which a frozen family is held to a re-taken contract.

### Command exit codes

- `scripts/gates.sh` returns 0
- `target/release/signal_report --check` returns 0 on the tree

## observation contract

- **Expected values:**
  - `family_known`'s output byte-equal to `known-2026-10-29.tsv` with the new table empty.
  - A test fixture row moving one frozen family's contract column, and only that row.
- **Planted faults:**
  - (1) The route applied to every family: the known table's test fails naming a moved row.
  - (2) A table row naming a contract file that does not exist: `family_known` refuses it naming the file.

## done criteria

A frozen family can be held to a contract version re-taken on the laptop by one row of a table, and no family moves
until a card names it.

## self-review checklist

- [x] Scope names one verifiable property: the route, tested, moving nothing yet.
- [x] context_files cite the judge, the generator, the tables and the Decisions.
- [x] Emits: none, with the reason.
- [x] Artifact contract: one file created, one modified.
- [x] Observation contract: the unchanged output, the fixture; numbered plants.
- [x] One concept.

## result

GATES-LINE
