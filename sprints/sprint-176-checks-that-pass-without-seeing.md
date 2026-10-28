# Sprint 176 — Checks that pass without seeing

```yaml
---
id: 176
status: active
phase: 6
pass_kind: functional
---
```

## scope

reviewer-2's review of Sprints 171-174 (BLACKBOARD Surfaced and the Decision of 2026-10-28): High 1, Mediums 3, 4(b), 5
and 6, Low 1, and High 2's census count. Each item is a check that passes while what it should hold has moved.

- **The C words judge's unlifted map** (High 1): `crates/translator/tests/c_words.rs` 218-229 print the classes the
  Rust lifter does not lift and assert nothing about them. The map of 2026-10-28 (105 classes, 1,933 calls) is
  committed as `store/census/c-words-unlifted-2026-10-28.tsv` and the test asserts `not_lifted` equal to it.
- **Per-class case counts** (Medium 4b): `store/case_runner/expected-by-class.tsv` (run, class, judged, faults
  judged, model, stepped by the model, undefined memory), asserted by `case_runner.rs` beside `expected.tsv`.
- **One quoted judge line per numbered plant** (Medium 3): `signal_report --check` counts the plants a card's
  observation contract numbers ("(1)", "(2)") and requires one result line per number that names the plant and quotes
  its judge; a line saying a plant reached no judge counts only if a replacement plant with a quote follows.
- **No tail entry closed before its card** (Medium 6): `signal_report --check` fails a BLACKBOARD tail entry headed
  "closed" whose card is not `status: done`, or which holds `GATES-LINE` or `CI-LINE`.
- **Overflow checks in release** (Low 1): `overflow-checks = true` in `[profile.release]`; the case runner's time
  measured before and after.
- **A failing pytest keeps its output** (Medium 5): `scripts/gates.sh` copies a failing Python gate's output to
  `build/gates-failures/` before it exits.
- **Census row 4's count** (High 2): the c_words census by slice (SSE, VEX, MMX and integer SIMD, x87).

## prerequisites

- 175 (closed)

## context_files

- `crates/translator/tests/c_words.rs`, `crates/w2m_vm/tests/case_runner.rs`, `crates/w2m_vm/src/cases.rs` (`Tally`)
- `crates/signal_report/src/main.rs` (`unquoted_plants`, `section`), `scripts/gates.sh`, `Cargo.toml`
- BLACKBOARD Surfaced (the review of Sprints 171-174), `docs/step-4-census-2026-10-28.md`

## signal contract

### Emits

- None: no producer runs; the judges are tests and checks.

### Invariants

- No Python file added or changed; every count the judges held before holds as before.

## artifact contract

### Files modified

- `crates/translator/tests/c_words.rs`, `store/census/c-words-unlifted-2026-10-28.tsv` (new)
- `crates/w2m_vm/tests/case_runner.rs`, `store/case_runner/expected-by-class.tsv` (new)
- `crates/signal_report/src/main.rs`, `scripts/gates.sh`, `Cargo.toml`
- `WORKING_AGREEMENT.md`, `docs/step-4-census-2026-10-28.md` (row 4)

### Command exit codes

- `c_words`, the case runner, `signal_report --check` and `scripts/gates.sh` return 0

## observation contract

- **Expected values:** `c_words` holds 1,933 unlifted calls in 105 classes; `expected-by-class.tsv` holds every
  class of every run; `signal_report --check` passes over Sprints 171-175 as committed.
- **Planted faults:** each on the path its judge reads, each quoted in the result: (1) the Rust lifter refusing MOVSS:
  `c_words` fails and names MOVSS; (2) one class's judged count moved to another run of equal total: the case runner
  fails and names the class; (3) Sprint 173's card with plant 1's quote removed: `signal_report --check` fails on it;
  (4) a tail entry headed "closed" with GATES-LINE: `signal_report --check` fails; (5) Sprint 173's IDIV shift
  (`1i128 << 128`) restored: the release case runner panics with overflow.

## done criteria

Each check the review named fails on its plant and passes on the tree.

## self-review checklist

- [x] Scope names one verifiable property: each check fails when what it holds moves.
- [x] context_files cite the code each check lives in.
- [x] Emits: none, said so.
- [x] Artifact contract gradable by command.
- [x] Observation contract: five plants, each on its judge's path.
- [x] One concept: checks that pass without seeing.
