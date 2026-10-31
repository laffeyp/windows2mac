# Sprint 285 — The case runner reaches guest memory through the view

```yaml
---
id: 285
status: active
phase: 6
pass_kind: functional
---
```

## scope

Census row D7's next module (the step 6 sequence Decision, annotated at 278-284). `crates/w2m_vm/src/cases.rs`, the
case runner that judges the CPU cases against Windows' results, holds 12 slice calls, none in its tests (which start
at line 762), and one view built from a slice (507). It runs one vCPU, but census D7 counts every slice in
`crates/w2m_vm/src/` until each module is moved or named as a one-vCPU tool, and the case runner's helpers (`windows`,
`vm_ymm_hi`, `vm_x87`) read the same areas the run loop's do.

- **The rows of the design's table they cite** (`docs/design-guest-memory-on-several-vcpus-2026-10-31.md`): a vCPU's
  own areas (the monitor data's YMM high halves and FCW, the state block's x87 state); the run table and the
  reconstruction table (each case's entry and marks); the dispatch table (`dispatch_insert`, a view from the mapping);
  the transport page (a case's host request); guest buffers (the data, stack and edge windows).
- **The work:**
  - `cases.rs`' helpers and `run_cases` reach guest memory through `Guest::memory_view` or
    `guest::memory_view(&guest.memory)`; the view at 507 comes from the mapping.
  - `tests/slice_calls.rs` and census D7 lowered together.
- **Not here:** the other modules.

## prerequisites

- 284 (closed)

## context_files

- `crates/w2m_vm/src/cases.rs`; `crates/w2m_vm/tests/case_runner.rs` 76-290
- `docs/design-guest-memory-on-several-vcpus-2026-10-31.md` (the table); `docs/step-6-census-2026-10-30.md` (D7)
- `crates/w2m_vm/tests/slice_calls.rs`; WORKING_AGREEMENT, "A plant's judge"

## signal contract

### Emits

- None; an access path, held by the case runner's test against Windows' records.

### Invariants

- The case runner's classes against Windows as held: `store/case_runner/expected-by-class.tsv` unchanged.

## artifact contract

### Files modified

- `crates/w2m_vm/src/cases.rs`, `crates/w2m_vm/tests/slice_calls.rs`, `docs/step-6-census-2026-10-30.md`

### Command exit codes

- `scripts/gates.sh` returns 0

## observation contract

- **Expected values:** `cases.rs` 0 slice calls and 0 views built from a slice; the case runner's classes as held.
- **Planted faults**, each judge found by lookup to reach its line and to assert what the plant breaks (WORKING_AGREEMENT,
  2026-10-31): `case_runner.rs`' `every_judged_case_agrees_with_windows_with_the_rust_translator` calls `run_cases`
  (96) and fails when any class's row differs from the held table (210-228).
  - (1) `windows` reading the data window one byte past its offset: every case with a data window differs from
    Windows, and the class rows move.
  - (2) `vm_x87` reading FCW two bytes past its offset: the x87 classes' control word differs, and their rows move.

## done criteria

The case runner reaches guest memory only through the view, and its classes against Windows stand as held.

## self-review checklist

- [x] Scope names one verifiable property: the case runner's access through the view.
- [x] context_files cite the module, its judge's lines, the table, the census and the rule.
- [x] Emits: none.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and two plants, each judge's reach and assertion looked up.
- [x] One concept: one module's access moved.

## result
