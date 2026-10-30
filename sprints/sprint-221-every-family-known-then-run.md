# Sprint 221 — Every family, known then run

```yaml
---
id: 221
status: active
phase: 6
pass_kind: functional
---
```

## scope

The Decision "Sequence after Sprint 214" as annotated in Sprint 218 (reviewer-3's High 1 of its review of Sprint 218),
plan step 4's gate item "68 of 68 families", reviewer-3's High 1, Medium 3 and Low 1 of its review of Sprint 219, and
the user's rule of 2026-10-29: "no searching for already knowable answers that you can look up at once"
(KIT_DIARY #87).

- **Where the families stand:** 5 families run on the Rust host (census row 6), one card each. The gate's judge,
  `store/runs/family-runs.json`, holds 68 families and 273 runner runs. Each run is judged against the contract id
  and version its family record's COMPARISON_COMPLETED names (`windows.code_pages.ansi_1252` 0.2.0, for one). 18
  contracts have versions past 0.1.0. `scripts/gates.sh` reads `/0.1.0.json` only.
- **The population:** for each of the 68 families, each run its entry judged: 273 runs, 269 keyed by GitHub run id
  and 4 laptop runs keyed `lap-…` (thread_stacks, running_thread_context, code_rewrite, wait_handoff), each named in
  `family_run.py` with its binary, every binary stored under `store/binaries` (checked 2026-10-30). For each run,
  three things: the binary (`scripts/family_run.py`'s `Run` for that id); the file beside it (its `dll`); and the
  contract id and version from the family's record.
- **The lookup first, all at once,** into one table, `store/family_runs/known-2026-10-29.tsv`, for each run:
  - **Imports:** the binary's imports and its beside DLL's (`pe_image`) that no service of the guest runtime serves
    (`guest_runtime::services`, `modules`' models).
  - **GetProcAddress:** the names the binary passes to it that its import table cannot show, where its strings name
    them.
  - **Argument forms:** the services it imports that refuse some argument form (the 319 `w2m_unimplemented(` sites in
    `tools/guest_runtime/*.c`).
  - **Code:** its x86 code decoded (`x86_decoder`) and held against the translator's lift and lowering census
    (`store/census/c-words-unlifted-2026-10-28.tsv`); the classes not lifted are named.
  - **Rows:** the family's derivation and exclusions (`family_run.py` `derive`, `excluded`), read for all families.
- **Then the sweep:**
  - The gates' family loop and `family_runs.rs` take the 273 runs from the index, each with its binary, beside file
    and contract version.
  - Each run has an instruction budget, so a hang ends by count, not by `GUEST_SECONDS`, and is counted.
  - A run that ends is held to its contract version's rows. A run that stops is held to its stop in the held stops
    file.
  - Each stop is checked against the table: a predicted stop is expected; an unpredicted one is named, and the
    card's result lists it with the reason the lookup missed it.
- **The like-for-like figure:**
  - The frozen harness keeps compiled blocks between runs (`run_frontier.py` 226-235, `build/translation-cache`), and
    the Rust host compiles afresh.
  - The census gets a line for the batch run's instructions a second, measured cache against cache, and read against
    plan step 4's 2,354,295,320.
- **Not here:** serving what the sweep stops on, which comes in the next cards, ordered by how many runs each gap stops.

## prerequisites

- 220 (closed)

## context_files

- `store/runs/family-runs.json` and the family records it names; `scripts/family_run.py` (`Family`, `Run`, `derive`,
  `excluded`); `contracts/windows.*/` (each version)
- `crates/w2m_vm/tests/family_runs.rs`, `scripts/gates.sh` (the family loop), `store/family_runs/`
- `crates/pe_image`, `crates/x86_decoder`, `crates/guest_runtime/src/services.rs`, `tools/guest_runtime/*.c`,
  `store/census/c-words-unlifted-2026-10-28.tsv`; `scripts/run_frontier.py` 226-235

## signal contract

### Emits

- None from the families' runs, which write no record on the Rust host (Deferred, 2026-10-17, the family runs'
  stops); the census row and the held stops file are the artifacts, and that Deferred entry's revisit condition is met
  by this sweep.

### Invariants

- The 14 runs that conform today still conform.
- Every stop is held and either predicted by the table or listed as unpredicted with its reason.

## artifact contract

### Files modified

- `crates/w2m_vm/tests/family_runs.rs`, the code that writes and holds the table, `scripts/gates.sh`,
  `store/family_runs/` (the table, the held stops, the beside rows), `docs/step-4-census-2026-10-28.md` (rows 6 and
  the instructions a second)

### Command exit codes

- `cargo nextest run -p w2m_vm` and `scripts/gates.sh` return 0

## observation contract

- **Expected values:**
  - Of the 273 runs: how many end and conform, how many stop, and how many hang to the budget.
  - Families: how many of 68 conform in every run.
  - For each stop, its prediction, or its listing as unpredicted.
  - The batch run's instructions a second, cache against cache.
- **Planted faults:**
  1. A service removed from the runtime's list: the table names the import it no longer serves.
  2. A run's contract version taken as 0.1.0 where its record names 0.2.0: `family_runs.rs` names the row or the
     version.
  3. A held stop changed: `family_runs.rs` names the moved stop.

## done criteria

Every runner run the frozen family index judged has run on the Rust host in one gates pass, against the contract
version it was judged by. Each conforms, stops where held, or hangs to its budget. A lookup made before the run
predicted each stop, or the result lists the stop with the reason it was missed.

## self-review checklist

- [x] Scope names one verifiable property: the gate's 68 families and 273 runs, their stops known by lookup first.
- [x] context_files cite the index, the family definitions, the contracts and the code.
- [x] Emits: none, with the reason and the Deferred entry it meets.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and three plants.
- [x] One concept: the families' state on the Rust host, whole.
