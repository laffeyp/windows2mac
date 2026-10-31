# Sprint 287 — The model step reaches guest memory through the view

```yaml
---
id: 287
status: active
phase: 6
pass_kind: functional
---
```

## scope

Census row D7's next module (the step 6 sequence Decision, annotated at 278-286). `crates/w2m_vm/src/model_step.rs`,
which steps one instruction through the model at an hvc #10 or #11 exit, holds 4 slice calls, none in tests: the
model's own loads and stores (`GuestModelMemory`, lines 53 and 58), the monitor data and state block read before the
step (93) and written after it (158). The run loop (`run.rs` 954) and the case runner (`cases.rs` 588) call it while
the run runs; with a second vCPU its loads and stores are the guest's, made while the other vCPU writes the same pages.

- **The rows of the design's table they cite** (`docs/design-guest-memory-on-several-vcpus-2026-10-31.md`): a vCPU's
  own areas (the monitor data's lazy flags, DF, MXCSR, FCW and YMM high halves, the save area, the state block's x87
  state); guest buffers (the model's loads and stores).
- **The work:**
  - `GuestModelMemory` reads and writes through `guest::memory_view(&guest.memory)`, taking `&Guest`;
    `model_step`'s reads before the step and writes after it go through `Guest::memory_view`; `read_u64` and
    `write_u64` over a slice go.
  - `tests/slice_calls.rs` and census D7 lowered together.
- **Not here:** the other modules (`turns.rs`' child memory, `monitor.rs`, `modules.rs`, the binaries).

## prerequisites

- 286 (closed)

## context_files

- `crates/w2m_vm/src/model_step.rs`; `crates/w2m_vm/tests/run_loop.rs` 641-673; `crates/w2m_vm/tests/case_runner.rs`
  76-290
- `docs/design-guest-memory-on-several-vcpus-2026-10-31.md` (the table); `docs/step-6-census-2026-10-30.md` (D7)
- `crates/w2m_vm/tests/slice_calls.rs`; WORKING_AGREEMENT, "A plant's judge" and "Before a work commit"

## signal contract

### Emits

- None; an access path, held by the model-steps run against the Python host's and the case runner against Windows'.

### Invariants

- The case runner's classes against Windows as held: `store/case_runner/expected-by-class.tsv` unchanged.
- `model_steps.exe`'s stop, output and hvc #10 and #11 counts as the Python host's (`build/run_loop/model-steps-python.json`).

## artifact contract

### Files modified

- `crates/w2m_vm/src/model_step.rs`, `crates/w2m_vm/tests/slice_calls.rs`, `docs/step-6-census-2026-10-30.md`

### Command exit codes

- `scripts/gates.sh` returns 0

## observation contract

- **Expected values:** `model_step.rs` 0 slice calls; 40 in `crates/w2m_vm/src/`; the model-steps run and the case
  runner's classes as held.
- **Planted faults**, each judge found by lookup to reach its line and to assert what the plant breaks (WORKING_AGREEMENT,
  2026-10-31):
  - (1) `GuestModelMemory::read` reading one byte past its address. Judge: `run_loop.rs`'
    `model_steps_runs_through_the_model_as_the_python_host` runs the program, whose ADDSS and RCR steps read their
    memory operands through it, and asserts its stop, output and exit counts against the Python host's, lines 657-672;
    the case runner's test, whose cases with a memory operand step through the same read, asserts the class rows,
    lines 210-228.
  - (2) the YMM high halves written back one byte past their offset after a step. Judge: the case runner's test, whose
    AVX cases read the halves after the step, the class rows, lines 210-228.

## done criteria

The model step reaches guest memory only through the view, and the model-steps run and the case runner's classes stand
as held.

## self-review checklist

- [x] Scope names one verifiable property: the model step's access through the view.
- [x] context_files cite the module, its judges' lines, the table, the census and the rules.
- [x] Emits: none.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and two plants, each judge's reach and assertion looked up.
- [x] One concept: one module's access moved.

## result

- **The gates:** GATES-LINE
