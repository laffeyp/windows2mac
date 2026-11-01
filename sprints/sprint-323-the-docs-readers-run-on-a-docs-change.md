# Sprint 323 — The docs readers run on a docs change

```yaml
---
id: 323
status: done
phase: 6
pass_kind: functional
---
```

## scope

Reviewer-3's Low 1 at 322 (BLACKBOARD Surfaced), widened by the audit after it: three test files in three crates read
the repository's own `docs/` or `BLACKBOARD.md` from its root (`join("../..")`): `hypervisor`'s `id_registers.rs` and
`w2m_vm`'s `guest_parts.rs` the run-loop design, `signal_report`'s `census.rs` two step documents and `BLACKBOARD.md`.
`scripts/gates.sh` maps neither path to a test: a sprint that changes only a design runs no Rust test (`rust=none` at
Sprints 309, 312, 314 and 318), and CI alone runs them after. `design_check`'s `planted.rs` and `signal_report`'s
`check.rs` quote the same paths but write them under `temp_dir()`, and are not this class.

- **The work:** the three join `scripts/tree-tests.txt`, each as its binary or its merged module; `tree_tests.rs`'
  criterion reads a third form, a root join with a quoted `"docs/` or `"BLACKBOARD.md"`; `scripts/gates.sh` runs the
  list's tests when a change touches `docs/` or `BLACKBOARD.md` and no crate, through `scripts/test-filter.sh` with no
  crate named, and names it.
- **Not here:** what the three tests hold.

## prerequisites

- 322 (closed)

## context_files

- reviewer-3's Low 1 at 322 and its disposition; `scripts/gates.sh` 55-100, 190-202; `scripts/test-filter.sh`;
  `scripts/tree-tests.txt`; `crates/signal_report/tests/tree_tests.rs`; the three tests and the two fixture-builders

## signal contract

### Emits

- None; the gates' selection.

### Invariants

- Every run as held.

## artifact contract

### Files modified

- `scripts/gates.sh`, `scripts/tree-tests.txt`, `crates/signal_report/tests/tree_tests.rs`

### Command exit codes

- `scripts/gates.sh` returns 0

## observation contract

- **Expected values:** `scripts/gates.sh --plan` on a change to `docs/` alone naming the list's tests, where it named
  "Rust tests: none" before; `cargo nextest list -E "$(scripts/test-filter.sh)"` listing the three files' tests; the
  list's test passing.
- **Planted faults:** (1) `guest_parts` taken off the list: the list's test fails naming its file; (2) the `docs/` rule
  taken out of `gates.sh`: the plan for a change to `docs/` alone reads "Rust tests: none" again.

## done criteria

A change to a document a test reads runs that test in the gates.

## self-review checklist

- [x] Scope names one verifiable property: a docs change selects the tests that read docs.
- [x] context_files cite the review, the gates and the tests.
- [x] Emits: none.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and two plants, each changing what its judge reads.
- [x] One concept: the tests a docs change reaches.

## result

- **The list** (`scripts/tree-tests.txt`): three lines more, `hypervisor::id_registers`, `w2m_vm::all`'s `guest_parts`
  module and `signal_report::census`; `cargo nextest list -E "$(scripts/test-filter.sh)"` lists their four tests,
  `the_vcpu_reports_its_bbm_level`, `the_served_part_is_the_designs` and `census`' two.
- **The test** `tree_tests.rs`: `reaches_beyond` reads a third form, `join("../..")` with a quoted `"docs/` or
  `"BLACKBOARD.md"`; `the_forms_are_read_and_a_merged_module_is_selected` holds both documents matched and a
  `temp_dir()` fixture not. Before the list's three lines, `every_tree_test_runs_on_every_change` failed naming exactly
  the three, `id_registers.rs`, `census.rs` and `guest_parts.rs`; `planted.rs` and `check.rs` not.
- **The gates** (`scripts/gates.sh` 74-76, `dash -n` passing): a change to `docs/` or `BLACKBOARD.md` takes the
  `crates` branch with no crate, so `scripts/test-filter.sh` gives the list alone and no doc test runs. With
  `GATES_CHANGED` the run-loop design alone, `--plan` printed "Rust tests: (binary_id(hypervisor::no_wall_clock_bound))
  | …", and the same for `BLACKBOARD.md` alone.
- **Plant 1,** restored by `cp` and checked by `cmp`, the line's number checked non-empty first: `guest_parts`' line
  struck; the test failed, "tests that read beyond their crate and are not in scripts/tree-tests.txt:
  [\"crates/w2m_vm/tests/guest_parts.rs\"]".
- **Plant 2,** restored likewise: the clause `|| touched '^(docs/|BLACKBOARD\.md$)'` taken out; `--plan` for the
  run-loop design alone printed "Rust tests: none (no crate changed)".
- **The gates:** `scripts/gates.sh` exit 0 in 1383 s, kept in `store/gates/passes.tsv` as sprint 323 at d0556beb7 with root=a57230b2e and "rust=all python=0 samples=1", every stage for the change to `scripts/gates.sh` (its line 57), 346 tests, 650 ns an exit at load 3.09, base=15bcba220, Sprint 322's close; multi-vCPU: not run (Python gates not run)
