# Sprint 285 — The case runner reaches guest memory through the view

```yaml
---
id: 285
status: done
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
  at line 96 and fails when any class's row differs from the held table, lines 210-228.
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

- **The access** (bd86cb8a3): `cases.rs`' helpers take the view, with a new `bytes`; `windows`, `vm_ymm_hi`,
  `vm_x87` and `run_cases`' reads and writes go through `Guest::memory_view` or `guest::memory_view(&guest.memory)`;
  `dispatch_insert` takes the view from the mapping.
- **One slice kept, not the card's 0:** the case runner fills the 16 MiB dispatch table (2^20 entries of 16 bytes,
  `layout.rs` 59-60) before each case. Through the view its test took 145.88 s; the committed code took 68.66 s,
  timed under the same build with the committed `cases.rs` put back and the work restored by `cp` and checked by
  `cmp`; with the fill by one slice copy, 69.29 s. The view's relaxed atomic words are not vectorized. The runner's
  one vCPU is stopped between cases, so the slice is sound (the design's choice 1); the design's goal paragraph is
  amended with the figures, the game's large copies to be weighed with G5 and G6.
- **The count:** 44 slice calls in `crates/w2m_vm/src/`, from 55; `cases.rs` 1; its views built from a slice 0.
- **Expected values:** the case runner's classes against Windows as held, 69.29 s.
- **A guard that did not guard:** the first pre-commit run, in `( set -e; … )`, printed clippy's four "useless
  conversion" errors (`u64::from` on constants already `u64`, `cases.rs` 247, 254, 538, 543) and went on, "clippy-ok"
  through "pre-commit exit 0". Measured in this shell, zsh 5.9: `set -e`, `setopt errexit` and `set -o errexit` in a
  subshell each let `false`, `exit 3` and a failed `ls` pass, the subshell exiting 0; `/bin/sh -e -c` and an `&&`
  chain stop. No commit was made over it. The closes of 281 to 284, made in such blocks, pass the record check today
  with no placeholder left, and their pre-commit outputs show no error between steps. The conversions are removed and
  the checks run again under `/bin/sh -e -c`.
- **The gates:** `scripts/gates.sh` exit 0 in 711 s, kept in `store/gates/passes.tsv` as sprint 285 at bd86cb8a3 with root=e96c2503e and "rust=crates python=0 samples=0", rdeps(w2m_vm), 157 tests, base=e0e11c802, Sprint 284's close; multi-vCPU: not run (Python gates not run)
- **Plant 1,** restored by `cp` and checked by `cmp`: `windows` reading the data window one byte past its offset;
  `every_judged_case_agrees_with_windows_with_the_rust_translator` failed, "run 38011858047: 107 difference classes".
- **Plant 2,** restored by `cp` and checked by `cmp`: `vm_x87` reading FCW two bytes past its offset; the same test
  failed, "run 38222873837: 25 difference classes", "run 38234338405: 43 difference classes".
