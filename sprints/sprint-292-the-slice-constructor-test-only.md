# Sprint 292 — The slice constructor test-only

```yaml
---
id: 292
status: active
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

- **The gates:** GATES-LINE
