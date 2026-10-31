# Sprint 283 — The host compiler reaches guest memory through the view

```yaml
---
id: 283
status: done
phase: 6
pass_kind: functional
---
```

## scope

Census row D7's fifth module (the step 6 sequence Decision, annotated at 278, 281 and 282). `crates/w2m_vm/src/compile.rs`
holds 12 slice calls and no tests of its own, and three functions take a slice: `page_set_add`, `page_set_remove`, and
`installed_in_vm`, which `run.rs`' `vm_compiled` feeds, so `vm_compiled` holds a slice and builds two views from it.

- **The rows of the design's table they cite** (`docs/design-guest-memory-on-several-vcpus-2026-10-31.md`): the code
  area (`place` writes a block's words at the cursor, before the dispatch slot that names it is published); the
  reconstruction table, whose writer `place` is: it appends entries, then the count, both plain today, and the
  Acquire read Sprint 282 gave `x86_address_at` orders only the reader, so the count is stored Release after the
  entries; the in-VM compiler's words (the cursor, the install stop word, the code-page set); the translation tables
  (`protect_code` and `stop_cache` change a descriptor's access bits, by `atomic_u64` as `map_pages` does since 281).
- **The work:**
  - `compile.rs`'s helpers, `code_at`, `open_trace`, `compile`, `place`, `use_cache`, `install_block`,
    `protect_code`, `page_set_add`, `page_set_remove`, `installed_in_vm`, `code_written`, `drop_block` and
    `stop_cache` reach guest memory through the view; `place` stores the reconstruction count Release.
  - `run.rs`' `vm_compiled` takes the view, so its slice and its two views built from a slice go.
  - `tests/slice_calls.rs` and census D7 lowered together; the design's reconstruction row names the writer's Release.
- **Not here:** `cases.rs` and the other modules; the exit ledger's slice in `run.rs` (a one-vCPU diagnostic, census D7).

## prerequisites

- 282 (closed)

## context_files

- `crates/w2m_vm/src/compile.rs`; `crates/w2m_vm/src/run.rs` (`vm_compiled`)
- `docs/design-guest-memory-on-several-vcpus-2026-10-31.md` (the table); `docs/step-6-census-2026-10-30.md` (D7)
- `crates/w2m_vm/tests/run_loop.rs` 195-260, 547-580; `crates/w2m_vm/tests/slice_calls.rs`

## signal contract

### Emits

- None; an access path, held by tests and the one-vCPU runs.

### Invariants

- One-vCPU runs unchanged: the batch run's stop, stream and exit ledger, the cache's installs, the litmus tests.

## artifact contract

### Files modified

- `crates/w2m_vm/src/compile.rs`, `crates/w2m_vm/src/run.rs`, `crates/w2m_vm/tests/slice_calls.rs`,
  `docs/step-6-census-2026-10-30.md`, `docs/design-guest-memory-on-several-vcpus-2026-10-31.md`

### Command exit codes

- `scripts/gates.sh` returns 0

## observation contract

- **Expected values:** `compile.rs` 0 slice calls; `run.rs` 2 (the exit ledger's), its views built from a slice 1 (its
  tests); the run-loop, litmus and batch-run tests pass as before.
- **Planted faults**, each judge found by lookup to reach its line (the Drift watchlist's entry of 2026-10-31):
  - (1) `place` writing a block's words at the cursor plus 4: `run_loop.rs`'
    `qpc_runs_past_its_first_decline_compiling_as_the_python_host` runs `run_compiling` with a `HostCompiler`, whose
    `compile` calls `place`; it names the run's end.
  - (2) `stop_cache` writing the install stop word 8 bytes past `VMC_INSTALL_STOP`: `run_loop.rs`'
    `qpc_runs_with_its_blocks_installed_inside_the_vm` calls `code_written` on a cached block, which calls
    `stop_cache`, then asserts "the installer installed after the write".

## done criteria

The host compiler reaches guest memory only through the view, it publishes the reconstruction count Release after its
entries, and `vm_compiled` holds no slice.

## self-review checklist

- [x] Scope names one verifiable property: the host compiler's access through the view, the count's Release.
- [x] context_files cite the module, its caller in `run.rs`, the table, the census and the judges' lines.
- [x] Emits: none.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and two plants, each judge's reach looked up.
- [x] One concept: one module's access moved, with the writer half of the count's ordering.

## result

- **The access** (b3162737f): `compile.rs`'s helpers take the view; `code_at`, `open_trace`, `place` and the rest
  reach guest memory through `guest::memory_view(&guest.memory)`; `page_set_add`, `page_set_remove` and
  `installed_in_vm` take the view; a new `change_descriptor` changes a descriptor's access bits by one atomic load and
  a Release store, at its four sites (`use_cache`, `protect_code`, `code_written`, `stop_cache`).
- **The count's Release:** `place` stores the reconstruction count Release after its entries, the writer's half of
  282's Acquire; the design's row says so.
- **`vm_compiled`** and `vm_compiled_waiting` take the view; `run.rs`' slice helpers, left with no caller, removed.
- **The count:** 55 slice calls in `crates/w2m_vm/src/`, from 68; `compile.rs` 0, `run.rs` 2 (the exit ledger's).
  Views built from a slice in `run.rs` 2, both in its tests, not the card's 1: the drain's test and the lookup's.
- **The gates:** `scripts/gates.sh` exit 0 in 693 s, kept in `store/gates/passes.tsv` as sprint 283 at b3162737f with root=2eca8bd16 and "rust=crates python=0 samples=0", rdeps(w2m_vm), 156 tests, base=dbd0356d1, Sprint 282's close; multi-vCPU: not run (Python gates not run)
- **Plant 1,** restored by `cp` and checked by `cmp`: `place` writing a block's words at the cursor plus 4;
  `qpc_runs_past_its_first_decline_compiling_as_the_python_host` failed, "the translations in compile order (start,
  end, discovery)", three blocks where eight are held.
- **Plant 2,** restored by `cp` and checked by `cmp`: `stop_cache` writing the install stop word 8 bytes past
  `VMC_INSTALL_STOP`; `qpc_runs_with_its_blocks_installed_inside_the_vm` failed earlier than the card said, "the
  installer stopped otherwise: Exception { syndrome: 1509949454 … } left: Some(14) right: Some(85)": the word 8
  bytes on is `VMC_INSTALLER`, 0x308, the installer's address (`monitor_code.rs` 46-47), which the plant set to 1,
  so the install broke before the write rule's assertion was reached.
