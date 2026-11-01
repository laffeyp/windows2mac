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

