# Sprint 176 — Checks that pass without seeing

```yaml
---
id: 176
status: done
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

## result

- **The C words judge's unlifted map:** `store/census/c-words-unlifted-2026-10-28.tsv` holds 105 classes and 1,933
  calls, the map `c_words` printed on 2026-10-28 (1,186,511 calls, 1,184,578 lifted); `c_words.rs` asserts
  `not_lifted` equal to it and names each class that moved. The review counted 116 classes; the map's entries are 105.
- **Plant 1** (MOVSS added to the lifter's `MMX_ONLY`, so `lift` refuses it): `c_words` failed, "the unlifted classes
  moved from the census: MOVSS: held None, now Some(15536)". Restored.
- **Per-class counts:** `store/case_runner/expected-by-class.tsv` holds 589 rows (run, class, judged, judged at a
  fault, to the model, stepped by the model, undefined memory; `Tally` gains `undefined_by_class`). Its columns sum to
  `expected.tsv`'s totals: 254,831 judged, 27,213 at a fault, 35,663 to the model, 54 undefined, and 81,055 stepped.
  Both judging tests of the case runner hold it.
- **Plant 2** (one judged case moved from AND_LOCK to ADD_LOCK in run 38029980267, totals unchanged): the case runner
  failed, "class rows differ from store/case_runner/expected-by-class.tsv: now 38029980267 ADD_LOCK 740 148 0 91 0;
  held 38029980267 ADD_LOCK 741 148 0 91 0; held 38029980267 AND_LOCK 705 138 0 83 0; now 38029980267 AND_LOCK 706 138
  0 83 0". Restored. A first draw, on ADC and ADD, changed nothing, since that run holds neither class; it passed and
  was redrawn.
- **Numbered plants:** `signal_report --check` reads a card's "(1)", "(2)" in its Planted faults bullet and requires a
  "**Plant k**" result bullet quoting a judge for each; a bullet that says its plant reached no judge counts only with
  a quote after that phrase. A card that numbers no plant (171) keeps Sprint 171's rule. Unit test
  `each_numbered_plant_needs_its_own_quote`.
- **Plant 3** (Sprint 173's card in a copy of the tree, plant 1's three quotes removed): `--check` failed, "sprint 173:
  its observation contract names a planted fault and its result quotes no judge's failure for a plant". Sprint 171's
  rule passed the same copy on plant 2's quote. A first draw removed one of plant 1's three quotes and passed, rightly,
  since the bullet still quoted two judge lines.
- **Tail entries closed early:** `--check` fails a BLACKBOARD tail entry headed "closed" whose card is not done or
  which holds GATES-LINE or CI-LINE; integration test `a_tail_entry_closed_before_its_card_or_its_gates_fails`, and
  the other two tests' roots gain a BLACKBOARD.
- **Plant 4** (Sprint 175's tail entry in a copy of the tree given GATES-LINE back): `--check` failed, "sprint 175: its
  tail entry is headed closed and holds GATES-LINE".
- **Overflow checks:** `overflow-checks = true` in `[profile.release]`. Release `case_runs` on the 8573C Server 2022
  profile took 41.11 s and 41.36 s before, 42.27 s and 42.74 s after.
- **Plant 5** (Sprint 173's IDIV path restored: the 64-bit dividend taken through `1i128 << (2 * bits)`): release
  `case_runs` panicked, "panicked at crates/w2m_vm/src/model.rs:417:36: attempt to shift left with overflow".
  Restored.
- **A failing pytest keeps its output:** `scripts/gates.sh` writes pytest's output to `build/gates-failures/` and
  keeps it on a failure (`build/` is git-ignored), printing its last 40 lines and its path.
- **Census row 4:** the C words census by slice: SSE 848 calls in 8 classes, VEX 719 in 71, MMX and integer SIMD 342
  in 21, x87 24 in 5, the rule for each slice stated in the row.
- **Departures from the card:** `crates/w2m_vm/src/cases.rs` (`undefined_by_class`) and
  `crates/signal_report/tests/check.rs` changed, files the card did not name; `check.rs` was edited by an inline Python
  rewrite, a multi-line edit outside the Edit tool.
- **Left:** reviewer-2's Medium 4(a), counted-not-judged classes on the record, needs a typed vocabulary proposal
  (Deferred); Medium 5's measured range of the frozen-host test runs alone and beside the case runner after this close.
