# Sprint 184 — The judges' inputs, again

```yaml
---
id: 184
status: active
phase: 6
pass_kind: functional
---
```

## scope

reviewer-2's review of Sprints 178-181 (BLACKBOARD Surfaced and the Decision "Sequence after Sprint 182"): Mediums 1, 2,
3 and 4. Each item is a check that holds less than its name says.

- **The runtime image's symbols** (Medium 1): `c_words`'s zero-halfword test reads `head["symbols"]` from the stored
  record. It reads instead the symbol table of the runtime image the build makes (the image `guest_runtime::build`
  returns, as `case_runtime_calls` reads it), and holds the symbols with a zero halfword below bit 48 to the committed
  list, so a symbol that moves onto one fails.
- **Closed-partial** (Medium 2): `signal_report --check` accepts a tail entry headed "closed-partial" only over a card
  whose status is `closed-partial`, and requires that status of a sprint with a Deferred entry tagged "(Sprint N,
  partial)". KIT_DIARY records H5's recurrence (a coverage claim in a card's prose, Sprint 178's lift premise) with the
  rule that a card claiming the translator lifts N classes quotes the lift's per-class output before its commit.
- **The gates' run bound** (Medium 3): `scripts/gates.sh` appends the HEAD sha, the sha256 of `git diff HEAD`, the
  sprint (the newest card with `status: active`), the mode, the exit and the seconds; a closed tail entry from this
  sprint on quotes its line's sprint and seconds; `--check` matches both, fails locally when the file is absent, and
  under `CI=true` prints "gates-passes: skipped (CI)".
- **`x86_address_at` as bisect_right** (Medium 4): `partition_point` over the reconstruction table's first
  RECON_COUNT entries, minus one, with unit tests for tied hosts, a pc below the first entry (0) and a truncated count.

## prerequisites

- 183 (closed)

## context_files

- `crates/translator/tests/c_words.rs` (the zero-halfword test), `crates/w2m_vm/src/case_inputs.rs`
  (`case_runtime_calls`), `crates/signal_report/src/main.rs`, `scripts/gates.sh`, `crates/w2m_vm/src/run.rs`
  (`x86_address_at`), `src/w2m/vm_backend.py` 4861-4863, `sdd-kit-3/AGENTS.md` (partial pass), `KIT_DIARY.md`

## signal contract

### Emits

- None: no producer runs; the judges are tests and checks.

### Invariants

- No Python file added or changed; every count the judges held before holds as before.

## artifact contract

### Files modified

- `crates/translator/tests/c_words.rs`, `store/census/runtime-symbols-zero-halfword-2026-10-28.tsv`
- `crates/signal_report/src/main.rs`, `crates/signal_report/tests/check.rs`, `scripts/gates.sh`
- `crates/w2m_vm/src/run.rs`, `KIT_DIARY.md`, `WORKING_AGREEMENT.md`

### Command exit codes

- `c_words`, the run-loop judges, `signal_report --check` and `scripts/gates.sh` return 0

## observation contract

- **Expected values:** the runtime image's zero-halfword symbols read and held; `--check` passes over Sprints 171-183
  with Sprint 178 `closed-partial`; the run-loop judges pass with the bisect.
- **Planted faults:** each on its judge's path, quoted: (1) a held symbol list missing one entry the image has:
  `c_words` names the symbol; (2) a tail entry naming a gates run of another sprint: `--check` names the mismatch; (3)
  `x86_address_at` taking the first entry at or below pc instead of the last: its unit test on tied hosts fails.

## done criteria

Each check the review named holds what its name says, and each plant fails it.

## self-review checklist

- [x] Scope names one verifiable property: each check reads what it names.
- [x] context_files cite the code each check lives in.
- [x] Emits: none, said so.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and three plants.
- [x] One concept: the judges' inputs.
