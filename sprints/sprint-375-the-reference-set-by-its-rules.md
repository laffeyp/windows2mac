# Sprint 375 — G3's reference set chosen by its rules, so laptop game runs can join it

```yaml
---
id: 375
status: active
phase: 6
pass_kind: functional
---
```

## scope

G3 holds the game's batch run to a set of eleven Windows runs: runner run 38216081440 and the laptop runs
`lap-20261014T032442034Z-c8814547-01` to `-10`. The set is fixed: `crates/call_compare/src/lib.rs` 1251-1253, "the
membership rules, which read laptop and runner run files and records, are not ported; the set is the frozen host's
last choice". So no laptop game run made since 2026-10-14 can join it (the postmortem's next steps; the Decision "The
order after the stop, the cards that take each deferred item", card 375).

- **The rules ported**, from the frozen `scripts/call_compare.py` at the tag `frozen-python-2026-10-29` (read in full;
  text to read, never to run): `game_build` (544), `outside_set` (550) and `reference_set` (597), with `laptop_run`
  (523), `capture_thread` (529) and `load_reference` (536) as they need, into `call_compare`, so the set is computed
  from `store/game-runs/`, each run kept or left out with its reason.
- **A game run on the record**, as `scripts/record_game_run.py` did at the same tag (read in full): EXECUTION_STARTED
  and EXECUTION_COMPLETED for a stored game run, `profile_kind` `windows_reference`, under `store/runs/game-run-<run>/`,
  since `outside_set` reads each run's record. The vocabulary already has both tags; no tag or field is added.
- **Held to the frozen host's choice.** On today's store the computed set is exactly the eleven runs the frozen host
  chose, each left-out run named with the frozen host's reason; the G3 batch runs judge against the computed set, and
  each record's `reference_ref` is the set's. The three game runs Sprint 372 made with Steam signed in stay in `build/`
  and are not candidates.
- **Not here:** new laptop game runs and the job's Steam check (the next card, which also adds code-integrity state to
  `tools/windows_machine/profile.ps1` and names the laptop's host key `w2m-laptop`).

## prerequisites

- 374 (closed)

## context_files

- `git show frozen-python-2026-10-29:scripts/call_compare.py` (lines 500-640 and its `main`) and
  `git show frozen-python-2026-10-29:scripts/record_game_run.py`, read in full (read only; no Python runs).
- `crates/call_compare/src/lib.rs` (the reference set's reader, 1240-1300) and its tests; `harness/w2m-vm-game.toml`;
  `scripts/batch-run-record.sh`.
- `store/game-runs/` (listing; one laptop and one runner run read in full); `store/runs/` (the frozen host's record
  `frontier-643a6283510c-1792623423346201000`, its `reference_set`).
- `signals/` (the vocabulary's EXECUTION_STARTED and EXECUTION_COMPLETED).
- `WORKING_AGREEMENT.md` lines 137-266.

## signal contract

### Emits

- None on today's store (amendment, builder-375's finding): every stored game run the rules read, 38 of the 44, is
  already on the record (`store/runs/game-run-<run>/`, written by the frozen `record_game_run.py`), and a record is
  never written over, so the record step writes none. Its EXECUTION_STARTED and EXECUTION_COMPLETED payloads, built
  from each stored run, are held field for field to those 38 frozen records by a test; the step's first records are
  card 376's new laptop game runs. No other emit site changes.

### Invariants

- Every run as held; G3's batch runs conform as before, against the same eleven runs. No stored game run is rewritten.

## artifact contract

### Files created

- `store/runs/game-run-<run>/` for each stored game run the rules read; the port's tests.

### Files modified

- `crates/call_compare/src/lib.rs` and its tests, with others only where the class requires, reported to main when
  found. One concept: the reference set chosen by its rules.

### Command exit codes

- `scripts/gates.sh` returns 0
- `target/release/signal_report --check` returns 0 on the tree

## observation contract

- **Expected values:**
  - The frozen rules quoted beside the Rust that does each.
  - The computed set on today's store, and each run left out with its reason; the set equal to the eleven.
  - The game-run records' EXECUTION_STARTED and EXECUTION_COMPLETED fields for one laptop and one runner run.
  - G3's batch run's `reference_ref` from the computed set, its verdict as before.
- **Planted faults:**
  - (1) A stored laptop game run's `exec.json` with a game build other than the reference's: the rules leave it out,
    and the test naming the set fails if it stays in.
  - (2) The set's rule taking every stored game run: the set's test fails, naming the run that joined.

## done criteria

G3's reference set is computed from the stored game runs by the frozen host's rules, equal to its eleven runs today, so
a laptop game run that meets the rules joins it.

## self-review checklist

- [x] Scope names one verifiable property: the set computed by its rules, equal to the frozen choice.
- [x] context_files cite the frozen rules, the reader, the records and the vocabulary.
- [x] Emits: none on today's store, with its reason; the record step held to the 38 frozen records.
- [x] Artifact contract gradable by command.
- [x] Observation contract: the set and its reasons; numbered plants.
- [x] One concept: the reference set chosen by its rules.

## result

GATES-LINE
