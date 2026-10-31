# Sprint 292 — The slice constructor test-only

```yaml
---
id: 292
status: done
phase: 6
pass_kind: functional
---
```

## scope

Reviewer-3's Low 1 at 278, as Surfaced dispositioned it: "the card that moves the last module makes the slice
constructor test-only, leaving `memory_view` and `from_raw_parts`". Sprint 290 moved the last module that writes guest
memory while the run runs. `ConcurrentGuestMemory::new(&mut [u8])` builds a view from a slice; outside the view's own
tests it is called at `guest.rs` 853, the code cache at load, and in `monitor_code.rs`' and `run.rs`' tests. A view
built from a slice is sound only while no vCPU runs; built from the mapping (`guest::memory_view`) it needs no such
care.

- **The work:**
  - `guest.rs`: the code cache at load takes `memory_view(&memory)`; the slice made at line 808 is last used before
    it.
  - `concurrent_memory.rs`: `new` under `#[cfg(test)]`, its doc saying so; a `compile_fail` doc test, as Sprint
    157's holds `Vcpu` to `!Send`, holding it out of reach of code outside the crate's tests.
  - `tests/slice_calls.rs` and census D7: `guest.rs` 0 views built from a slice.
- **Not here:** the 27 guest slices at load, in tests and in one-vCPU diagnostics.

## prerequisites

- 291 (closed)

## context_files

- `crates/w2m_vm/src/concurrent_memory.rs` 30-60; `crates/w2m_vm/src/guest.rs` 800-880
- `crates/w2m_vm/tests/slice_calls.rs`; `docs/step-6-census-2026-10-30.md` (D7)
- the doc test of Sprint 157 (`crates/hypervisor`, `Vcpu` not `Send`); `scripts/gates.sh` 192-203 (doc tests)

## signal contract

### Emits

- None; a constructor's reach, held by a doc test and the type checker.

### Invariants

- The game's, the guests' and the family runs as held; the code cache's words unchanged.

## artifact contract

### Files modified

- `crates/w2m_vm/src/concurrent_memory.rs`, `crates/w2m_vm/src/guest.rs`, `crates/w2m_vm/tests/slice_calls.rs`,
  `docs/step-6-census-2026-10-30.md`

### Command exit codes

- `scripts/gates.sh` returns 0

## observation contract

- **Expected values:** `ConcurrentGuestMemory::new(` outside `concurrent_memory.rs` only in `#[cfg(test)]` modules
  (`monitor_code.rs` 1, `run.rs` 2); `guest.rs` 0; the doc test passing; the runs as held.
- **Planted faults**, each changing what the code compiles, each judge looked up:
  - (1) `#[cfg(test)]` taken off `new`. Judge: the `compile_fail` doc test, which then compiles and fails;
    `cargo test --doc -p w2m_vm`, which the gates run for a change to `w2m_vm` (`gates.sh` 198-200).
  - (2) `guest.rs` building the cache from the slice again. Judge: the crate's build outside tests, which then
    cannot find `new`; and `slice_calls.rs`' count, `guest.rs` 1 view against the census's 0.

## done criteria

No code outside the crate's tests builds a view from a slice, and the compiler refuses one that tries.

## self-review checklist

- [x] Scope names one verifiable property: the slice constructor's reach.
- [x] context_files cite the view, the load, the count, the precedent and the gates' doc tests.
- [x] Emits: none.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and two plants, each changing what compiles, each judge named.
- [x] One concept: one constructor's reach.

## result

- **A doc test that did not exist:** the first form put the `compile_fail` test on `new`'s own doc; rustdoc gathers
  doc tests from the build without `cfg(test)`, where `#[cfg(test)]` removes `new` and its doc with it, and the
  pre-commit run's doc tests read "0 passed; 0 failed". Moved to `ConcurrentGuestMemory`'s doc, which every build
  keeps: "1 passed". The same run piped the doc tests into `grep` under `/bin/sh -e`, where a pipe's status is its
  last command's, so a failing doc test would not have stopped it; the run again calls them plainly.
- **A guard that fails for its one thing alone** (reviewer-3's Low 1 at 292): a bare `compile_fail` passes on any
  error; stable rustdoc reads `compile_fail,E0599` as plain text (the rustdoc book, "Unstable features"), so the review's
  fix would remove the guard. Each `compile_fail` sample, here and `hypervisor`'s `Vcpu`, gains a twin that differs in
  the guarded thing alone and must compile: `from_raw_parts` for `new`, run on the vCPU's own thread (`no_run`) for the
  move. Listed by name: `ConcurrentGuestMemory (line 28)` ok, `(line 35) - compile fail` ok, `Vcpu (line 390) -
  compile` ok, `(line 396) - compile fail` ok. A departure: `crates/hypervisor/src/lib.rs` is changed, the same class.
- **The change** (364763e11): `ConcurrentGuestMemory::new` under `#[cfg(test)]`; the code cache at load from
  `memory_view(&memory)`; the twin-guarded doc tests in `concurrent_memory.rs` and `hypervisor`; `guest.rs` 0 views
  built from a slice, `monitor_code.rs` 1 and `run.rs` 2, all in `#[cfg(test)]` modules.
- **Expected values:** the four doc tests passing; 179 tests in the pre-commit run, the batch-run step first, the doc
  tests of both crates called plainly.
- **The gates:** `scripts/gates.sh` exit 0 in 886 s, kept in `store/gates/passes.tsv` as sprint 292 at 364763e11 with root=9aeb6f12c and "rust=crates python=0 samples=0", rdeps(hypervisor) and rdeps(w2m_vm), 179 tests and 4 doc tests, base=15160ecc8, Sprint 291's close; multi-vCPU: not run (Python gates not run)
- **Plant 1,** restored by `cp` and checked by `cmp`: `#[cfg(test)]` off `new`; `ConcurrentGuestMemory (line 35) -
  compile fail ... FAILED`, "Test compiled successfully, but it's marked `compile_fail`", its twin at line 28 passing.
- **Plant 2,** restored by `cp` and checked by `cmp`: the code cache built from the slice again; the build failed at
  `guest.rs` 853, "error[E0599]: no associated function or constant named `new` found for struct
  `ConcurrentGuestMemory<'a>`". The card's second judge, `slice_calls.rs`' count, did not run: the integration test
  links the library built without `cfg(test)`, which the same error stops; the compiler is the judge alone.
