# Sprint 278 — The scheduler reaches guest memory through the view

```yaml
---
id: 278
status: done
phase: 6
pass_kind: functional
---
```

## scope

Census row G4 (`docs/step-6-census-2026-10-30.md`) needs the run loop to drive a second vCPU of one process. The host
design (`docs/design-rust-host-2026-10-24.md` 12-23) gives each vCPU a native thread that runs it and serves its own
exits, with no lock across a service, so several exits are served at once while other vCPUs run. Sprint 276's design
(`docs/design-guest-memory-on-several-vcpus-2026-10-31.md`) allows a slice of guest memory only while every vCPU that
may write it is stopped; served exits at once leave no such moment, so the run loop's 174 slice calls in
`crates/w2m_vm/src/` (census D7) move to `ConcurrentGuestMemory` before a second vCPU runs. This card moves the
largest set, the scheduler's.

- **Where it stands, by lookup (2026-10-31):**
  - `crates/w2m_vm/src/threads.rs`: 45 slice calls, 31 outside its tests (before line 1084): `read_u64`, `write_u64` and `put` over
    `&[u8]`, `read_guest` and `write_guest`, and `GuestView { mem: guest.memory.as_mut_slice(), .. }` for the arena.
  - `guest::GuestView` (`guest.rs` 717-743): a `&mut [u8]` and the layout, behind the `GuestPages` trait (`zero`,
    `read`); 11 places build one.
  - `GuestMemory` gives its mapping only as `as_slice` and `as_mut_slice`. A view made from either would write
    through a pointer taken from a reference, so the view needs the mapping's own pointer.
- **The work:**
  - `hypervisor::GuestMemory::as_ptr`, the mapping's pointer; `guest::Guest::memory_view(&self)`, the view of the
    whole mapping for the life of the borrow.
  - `GuestView` holds the view; its 11 builders follow.
  - `threads.rs`'s helpers take the view, and its 31 calls outside its tests go through `memory_view`; its tests may
    keep slices, made while no vCPU runs.
  - A test, `crates/w2m_vm/tests/slice_calls.rs`, counts `as_slice()` and `as_mut_slice()` in each file of
    `crates/w2m_vm/src/` and fails when a file's count rises above the census's figure, the figures written in the
    test and in census row D7 together.
  - The step 6 sequence Decision annotated: the slice calls move module by module (threads, then host_request,
    guest, run, cases, compile and the rest), then the run loop drives a second vCPU, then G4.
- **Not here:** the other modules' calls; a second vCPU.

## prerequisites

- 277 (closed)

## context_files

- `crates/w2m_vm/src/threads.rs` 80-135 and its slice calls; `crates/w2m_vm/src/guest.rs` 717-743 (`GuestView`)
- `crates/hypervisor/src/lib.rs` 287-368 (`GuestMemory`); `crates/w2m_vm/src/concurrent_memory.rs`
- `docs/design-rust-host-2026-10-24.md` 12-23; `docs/design-guest-memory-on-several-vcpus-2026-10-31.md`
- `docs/step-6-census-2026-10-30.md` (D7)

## signal contract

### Emits

- None; an access path, held by tests and the one-vCPU runs.

### Invariants

- One-vCPU runs unchanged: the batch run's stop and stream, the 68 families, as held.

## artifact contract

### Files modified

- `crates/hypervisor/src/lib.rs`, `crates/w2m_vm/src/` (`guest.rs`, `threads.rs` and `GuestView`'s builders),
  `crates/w2m_vm/tests/slice_calls.rs` new, `docs/step-6-census-2026-10-30.md`, `BLACKBOARD.md`

### Command exit codes

- `scripts/gates.sh` returns 0

## observation contract

- **Expected values:** `threads.rs` 0 slice calls outside its tests; the run-loop, family and batch-run tests pass as
  before; the count test passes at the census's figures.
- **Planted faults:**
  - (1) `threads.rs`'s `write_u64` writing 8 bytes past its offset: a scheduler test names the word.
  - (2) one `as_slice()` added to `threads.rs` outside its tests: the count test names the file and its count.

## done criteria

The scheduler reaches guest memory only through the view, and a test holds every file's slice calls at or below the
census's count.

## self-review checklist

- [x] Scope names one verifiable property: the scheduler's guest memory access through the view.
- [x] context_files cite the module, the view type, the memory type, both designs and the census.
- [x] Emits: none.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and two plants.
- [x] One concept: one module's access moved, held by a count.

## result

- **The access** (01188b254): `GuestMemory::as_ptr`; `guest::memory_view(&GuestMemory)`, a free function so a
  service holds the view of the `memory` field beside a mutable borrow of another field (the arena's), and
  `Guest::memory_view` calling it.
- **Moved to the view:** `threads.rs`'s 31 slice calls outside its tests, its helpers `read_u64`, `write_u64`, `put`
  and a new `bytes`, `read_guest` and `write_guest`; `GuestView` and its 11 builders in four files; `CodeCache` and
  `traced_form` in `guest.rs`, which the substitution for `GuestView`'s builders also matched, writes at load before
  any vCPU, moved with it, and named here though the card did not name them. `Guest::new` keeps its slices: it runs
  before any vCPU exists.
- **The count:** 126 slice calls in `crates/w2m_vm/src/`, from 174; `threads.rs` 6, all in its tests.
  `crates/w2m_vm/tests/slice_calls.rs` holds each file at census row D7's figure.
- **Expected values:** the `w2m_vm` library tests, 81; before the commit, under WORKING_AGREEMENT's amended rule, the
  `hypervisor` and `w2m_vm` tests, 159 passed, 1 skipped, the only crates the change reaches (`cargo tree -i`).
- **The gates:** `scripts/gates.sh` exit 0 in 729 s, kept in `store/gates/passes.tsv` as sprint 278 at 01188b254 with root=e8c57ffb5 and "rust=crates python=0 samples=0", rdeps(hypervisor) and rdeps(w2m_vm), 159 tests, base=1fd21adc7, Sprint 277's close; multi-vCPU: not run (Python gates not run)
- **Plant 1,** restored by `cp` and checked by `cmp`: the scheduler's `write_u64` writing at its offset plus 8; three
  scheduler tests failed, the first "assertion `left == right` failed: the earliest deadline left: 4 right: 8".
- **Plant 2,** restored by `cp` and checked by `cmp`: an `as_slice()` added to `threads.rs` outside its tests; the
  count test failed, "threads.rs: 7 slice calls, census D7 allows 6".
