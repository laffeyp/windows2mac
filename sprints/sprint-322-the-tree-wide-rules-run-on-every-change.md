# Sprint 322 — The tree-wide rules run on every change

```yaml
---
id: 322
status: active
phase: 6
pass_kind: functional
---
```

## scope

Deferred (Sprint 321): the gates select the Rust tests by the crates a change reaches (`scripts/gates.sh` 196-197,
`rdeps()`), and three tests read source beyond their own crate: `hypervisor/tests/no_wall_clock_bound.rs` (every
crate), `w2m_vm/tests/layout_constants.rs` (every crate's `src`) and `signal_report/tests/no_python.rs` (the repo).
Each is skipped when no crate it depends on changes, and Sprint 320's two upper time bounds in `w2m_vm` passed its
gates and failed CI on its close (101ce0f55). With the card's commit, WORKING_AGREEMENT gains the rule the Drift
watchlist's "Command descriptions that misstate the command" set at a third and did not get: a description names what
the command changes, a command that edits never described as a test, a showing or a check (Sprints 276, 278, 304,
319, 321).

- **The work:** `scripts/tree-tests.txt`, the tests that read beyond their crate, one nextest filter expression a line
  (`binary_id(hypervisor::no_wall_clock_bound)`, `binary_id(w2m_vm::all) & test(/^layout_constants::/)`,
  `binary_id(signal_report::no_python)`); `scripts/test-filter.sh <crate>...`, the one place the gates' nextest filter
  is built, the changed crates' `rdeps()` and every line of the list, which `scripts/gates.sh` calls and names in its
  output; with no crate changed the gates run no Rust test, the rules' inputs those they passed at the last gates.
  A test, `signal_report`'s `every_tree_test_runs_on_every_change`, finds every test file under `crates/*/tests` that
  joins `..` onto `CARGO_MANIFEST_DIR`, and fails naming any the list does not select.
- **Not here:** a change to what the three rules hold.

## prerequisites

- 321 (closed)

## context_files

- Deferred (Sprint 321); `scripts/gates.sh` 50-100, 190-200; the three tests; nextest's filterset reference
  (`binary()`, `test()`)

## signal contract

### Emits

- None; the gates' selection.

### Invariants

- Every run as held.

## artifact contract

### Files modified

- `scripts/gates.sh`, `scripts/tree-tests.txt` new, `scripts/test-filter.sh` new, `crates/signal_report/tests/` (the
  list's test); `WORKING_AGREEMENT.md` with the card

### Command exit codes

- `scripts/gates.sh` returns 0

## observation contract

- **Expected values:** `cargo nextest list -E "$(scripts/test-filter.sh pe_image)"` listing the three tree tests, which
  `rdeps(pe_image)` alone does not; this card's gates naming them run; the list's test passing.
- **Planted faults:** (1) `layout_constants` taken off the list: the list's test fails naming its file; (2)
  `no_wall_clock_bound`'s line taken off: the listing for a `pe_image` change no longer holds it, which the expected
  value's check names.

## done criteria

A rule that reads the whole tree runs in every gates run that runs Rust tests, whatever crate the change touches.

## self-review checklist

- [x] Scope names one verifiable property: the tree-wide tests in every selection.
- [x] context_files cite the gap, the gates and the tests.
- [x] Emits: none.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and two plants, each changing what its judge reads.
- [x] One concept: one set of tests always selected.

## result

- **The list** (`scripts/tree-tests.txt`): four nextest filter expressions, `hypervisor::no_wall_clock_bound`,
  `w2m_vm::all`'s `layout_constants` module, `signal_report::no_python`, and `signal_report::tree_tests`, this card's
  test, itself a reader of every crate's tests; each line's selection checked by `cargo nextest list`: 3, 4 and 5
  tests for the first three.
- **The filter** (`scripts/test-filter.sh`, `dash -n` passing): each named crate's `rdeps()` and every list line, the
  one place the gates' filter is built; `scripts/gates.sh` calls it where it names the selection and where it runs it.
  For a change to `design_check` alone, `rdeps(design_check)` selects none of the tree tests and the filter all 12 of
  the first three; `design_check` one of five crates no tree test's crate depends on (`call_compare`, `design_check`,
  `session_events`, `vocabulary_generator`, `vocabulary_samples`, from `cargo tree`). `pe_image`, the card's example,
  was the wrong one: `w2m_vm` depends on it, so `rdeps(pe_image)` already reached `layout_constants`.
- **The test** `signal_report`'s `every_tree_test_runs_on_every_change`: every test file under `crates/*/tests` that
  joins `".."` onto its crate's directory or reads `git ls-files` must be selected by the list, its own binary or its
  module in a merged `tests/all.rs`; `the_forms_are_read_and_a_merged_module_is_selected` holds the two forms and the
  merged selection. A walk from the repository root into `store/` or `probes/` is data, whose changes the gates
  already widen on (`scripts/gates.sh` 66), and is not this class.
- **Plant 1,** restored by `cp` and checked by `cmp`: `layout_constants`'s line struck; the test failed, "tests that
  read beyond their crate and are not in scripts/tree-tests.txt: [\"crates/w2m_vm/tests/layout_constants.rs\"]". Its
  first run struck every line instead, the `grep` that gave the line's number matching nothing and `sed` with an empty
  address rewriting the whole file, and named all three; rerun with `grep -F` and the number checked non-empty first.
- **Plant 2,** restored likewise, its number checked the same way: `no_wall_clock_bound`'s line struck; the listing for
  a `design_check` change held 0 of it, `layout_constants` still 4.
- **The gates:** GATES-LINE

