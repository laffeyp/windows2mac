# Sprint 282 — The run loop reaches guest memory through the view

```yaml
---
id: 282
status: active
phase: 6
pass_kind: functional
---
```

## scope

Census row D7's fourth module (the step 6 sequence Decision, annotated at 278 and 281). `crates/w2m_vm/src/run.rs`
holds 23 slice calls, none in its tests (which start at line 1108), and 7 views built from a slice, one in its tests.
It is the run loop: each exit's service, the count's give-back, the vector swap of a thread switch, the transport
page's answer, the dispatch insert, the reconstruction table's lookup and the stop's figures.

- **The rows of the design's table they cite** (`docs/design-guest-memory-on-several-vcpus-2026-10-31.md`): the run
  table's words and the reconstruction table (the count read Acquire, a precondition of the second vCPU, reviewer-3's
  Low 1 at 280); the transport page; the dispatch table (`store_pair_release`, Sprint 276); the in-VM compiler's log
  and counters (the runtime word); a vCPU's own areas, the save slots the vector swap reads and writes.
- **The work:**
  - `run.rs`'s helpers (`code_at`, `exit_count`, `swap_vectors`, `read_runtime_word`, `give_back`) and its own reads
    and writes go through `Guest::memory_view` or `guest::memory_view(&guest.memory)`; the take and release of the
    runtime word, `x86_address_at` and `dispatch_insert` get their view from the mapping, not from a slice.
  - `x86_address_at` reads the reconstruction table's count by `atomic_u64` with Acquire, and the table's row and the
    step 6 Decision mark that precondition met.
  - What stays a slice, named in the census: `vm_compiled`'s, which `HostCompiler::installed_in_vm` takes
    (`compile.rs`, its own card), and the exit ledger's process area (`exit_ledger::resume`), a diagnostic of one-vCPU
    runs whose copy through the view at every exit would cost a copy of the process area each time.
  - `tests/slice_calls.rs` and census D7 lowered together.
- **Not here:** `compile.rs`, `cases.rs` and the other modules.

## prerequisites

- 281 (closed)

## context_files

- `crates/w2m_vm/src/run.rs`; `crates/w2m_vm/src/exit_ledger.rs` 68-80; `crates/w2m_vm/src/compile.rs` (`installed_in_vm`)
- `docs/design-guest-memory-on-several-vcpus-2026-10-31.md` (the table); `docs/step-6-census-2026-10-30.md` (D7)
- `crates/w2m_vm/tests/slice_calls.rs`

## signal contract

### Emits

- None; an access path, held by tests and the one-vCPU runs.

### Invariants

- One-vCPU runs unchanged: the batch run's stop, stream and exit ledger, the 68 families, the litmus tests.

## artifact contract

### Files modified

- `crates/w2m_vm/src/run.rs`, `crates/w2m_vm/tests/slice_calls.rs`, `docs/step-6-census-2026-10-30.md`,
  `docs/design-guest-memory-on-several-vcpus-2026-10-31.md`, `BLACKBOARD.md`

### Command exit codes

- `scripts/gates.sh` returns 0

## observation contract

- **Expected values:** `run.rs` at most the two slices named; its views from the mapping; the run-loop, litmus,
  family and batch-run tests pass as before.
- **Planted faults:**
  - (1) `swap_vectors` saving the vectors 16 bytes past the save slot: `run_loop.rs`'
    `sleeper_creates_and_ends_its_thread_as_the_python_host`, which switches threads, names it.
  - (2) the reconstruction count read at its offset plus 8: `run_loop.rs`'
    `model_steps_runs_through_the_model_as_the_python_host`, whose model steps look the address up, names it; the
    lookup's own tests pass a count and do not reach the read.

## done criteria

The run loop reaches guest memory through the view but for the two slices named, and the reconstruction table's count
is read Acquire.

## self-review checklist

- [x] Scope names one verifiable property: the run loop's access through the view, the count Acquire.
- [x] context_files cite the module, the two that keep slices, the table, the census and the count test.
- [x] Emits: none.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and two plants.
- [x] One concept: one module's access moved.

## result
