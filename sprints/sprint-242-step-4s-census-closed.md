# Sprint 242 — Step 4's census closed

```yaml
---
id: 242
status: active
phase: 6
pass_kind: functional
---
```

## scope

The Decision "Sequence after Sprint 214" as annotated at 241, and reviewer-3's direction at 240 ("a census close-out
of step 4, each gate item with its record, as the starting fact for step 5's card"): plan step 4's gate read row by row
against the records that meet it, and every other row of `docs/step-4-census-2026-10-28.md` given its state today.

- **Where it stands, by lookup (2026-10-30):**
  - The plan's gate (`docs/rust-port-plan-2026-10-24.md` 143-146, amended 151-160) is census rows 3, 6, 7, 8, 9 and
    10. Rows 3, 6, 8, 9 and 10 read met; row 7's cell reads "present over the whole reference" though the comparison is
    Rust and conforms on every record since Sprint 219, the three of Sprint 240 among them.
  - The other rows were last changed between Sprints 172 and 219. By lookup today:
    - row 4: every class the frozen index judged is judged on Rust, the model's slices included (Sprints 173-237);
      the batch run's hvc 10 (258) and hvc 11 (286) served on records that conform;
    - row 5: the record `store/runs/w2m-vm-game-1793364362000000000` holds hvc 1, 4, 8, 10, 11, 13 and 14, every
      kind the batch run makes, and conforms; `Stop::Hvc` (`run.rs` 930) is reached by none;
    - row 5a: every host request the batch run asks is served, the run conforming;
    - row 12: `store/census/c-words-unlifted-2026-10-28.tsv` holds its header alone (Sprint 236), held by
      `crates/translator/tests/c_words.rs` 253-257;
    - row 14: the batch run's record writes every event the row names (Sprints 210, 217);
    - row 1 (the per-component map of tests), row 2 (the loader's tests), row 11 (the imports' census), row 13 (flag
      liveness by condition) and rows 15a-15d (5 of 6 panic sites unnamed; 29 `Result<_, String>` in `threads.rs` 15,
      `kernel.rs` 5, `compile.rs` 5, `run.rs` 4; a full code cache, `compile.rs` 118, ends the run; compile time per
      block not measured) stay open.
  - "What the counts say" is Sprint 172's text and reads "2 of 14 guests".
- **The readers:** the census itself; BLACKBOARD's Decision and Deferred; no program reads the census.
- **The work:**
  - Rows 4, 5, 5a, 7, 12 and 14 given their state today, each with the record or test that shows it.
  - A close-out section: the gate's six items, each with its record or test and its sprint; the open rows, each with
    where it goes.
  - The open rows carried to one Deferred entry with a re-visit each, so the census closes with nothing unaccounted.
  - KIT_DIARY's entry for step 4, its lessons from Sprints 172-241.
  - A test, `crates/signal_report/tests/census.rs`: every `store/`, `crates/` or `docs/` path the census names
    exists, and every row whose status does not begin "met" is named in BLACKBOARD's Deferred as "census row N".
- **Not here:** any open row's work; step 5's card.

## prerequisites

- 241 (closed)

## context_files

- `docs/step-4-census-2026-10-28.md` whole; `docs/rust-port-plan-2026-10-24.md` step 4
- the records named in each row; `KIT_DIARY.md` (its last entries)

## signal contract

### Emits

- None: a census and its records read, no run.

### Invariants

- No row reads met without a record or test that shows it.

## artifact contract

### Files modified

- `docs/step-4-census-2026-10-28.md`, `BLACKBOARD.md` (Deferred), `KIT_DIARY.md`,
  `crates/signal_report/tests/census.rs` new

### Command exit codes

- `scripts/gates.sh` returns 0; `signal_report --check` and `design_check` return 0

## observation contract

- **Expected values:** six gate rows met, each naming its record or test; every other row met with its evidence or
  named in Deferred; the summary's counts today's.
- **Planted faults:**
  1. Row 7's evidence record name changed to one that does not exist: `census.rs` names the path.
  2. A cited test file's name changed (row 12's `c_words.rs`): `census.rs` names the path.
  3. An open row left out of the Deferred entry: `census.rs` names the row.

## done criteria

Step 4's census says, row by row, what is met and by which record, and where each open row goes.

## self-review checklist

- [x] Scope names one verifiable property: every census row's state backed by a record, a test or a Deferred entry.
- [x] context_files cite the census, the plan and the records.
- [x] Emits: none, stated with its reason.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and three plants.
- [x] One concept: step 4's census closed.

## result
