# Sprint 279 — The view's copies tear but never race

```yaml
---
id: 279
status: done
phase: 6
pass_kind: functional
---
```

## scope

Reviewer-3's Lows 1 and 3 at 278. `ConcurrentGuestMemory::read` and `write` (`crates/w2m_vm/src/concurrent_memory.rs`)
copy with `copy_nonoverlapping`, a plain access, which the design (`docs/design-guest-memory-on-several-vcpus-2026-10-31.md`
choice 2) allows only for bytes "no running vCPU writes at the time". The next module, `host_request.rs`, copies the
guest buffers a host request names (a file read's destination, a path), which the guest's other threads may write while
the host copies, as they may on Windows. Under Rust's model that race is undefined behaviour, not torn data: "data races
are undefined behavior … at least one of the accesses is non-atomic" (`std::sync::atomic`). RFC 3301, "AtomicPerByte
(aka atomic memcpy)" (https://github.com/rust-lang/rfcs/pull/3301, open, not merged), states the remedy: "All of the data
would have to be written and read through atomic operations to prevent a data race. We don't need the atomicity of the
data as a whole though; it's fine if there's tearing", and notes the `seqlock` crate's volatile copy "is technically
undefined behavior"; C++'s P1478 `atomic_load_per_byte_memcpy` is "effectively a series of `AtomicU8::load`s", in any
order and not necessarily a byte at a time.

- **The work:**
  - `read` and `write` copy by relaxed atomic loads and stores, `AtomicU64` over the 8-byte-aligned middle and `AtomicU8`
    at the ends: a racing guest gives torn bytes, defined, as on Windows. The module's head and the design's choice 2
    say so, with RFC 3301 and P1478 as grounds, and the one limit Rust keeps: atomics of different sizes must not race
    on the same bytes from two host threads (`std::sync::atomic`, mixed-size accesses), so no bulk copy covers a word
    another host thread reaches by `atomic_u32` or `atomic_u64` at the time.
  - Low 3: the design gains a table, per structure the host reaches (run table, runtime word, dispatch table, compile
    log, transport page, guest buffers a host request names, the code area), with its access (copy, atomic, pair), the
    ordering it needs and the fact that makes it so; each module card after cites its row.
  - Low 1: `tests/slice_calls.rs` also counts `ConcurrentGuestMemory::new(` in each file outside the module, at today's
    figures, so that path falls with the slices.
- **Not here:** `host_request.rs`'s calls (card 280); the slice constructor made test-only (the last module's card).

## prerequisites

- 278 (closed)

## context_files

- `crates/w2m_vm/src/concurrent_memory.rs`; `docs/design-guest-memory-on-several-vcpus-2026-10-31.md`
- `crates/w2m_vm/tests/slice_calls.rs`; BLACKBOARD Surfaced, reviewer-3's review at 278
- https://github.com/rust-lang/rfcs/pull/3301 and its text
  (https://github.com/m-ou-se/rfcs/blob/atomic-memcpy/text/3301-atomic-memcpy.md);
  https://doc.rust-lang.org/std/sync/atomic/index.html

## signal contract

### Emits

- None; an access path, held by tests.

### Invariants

- One-vCPU runs unchanged: the batch run's stop and stream, the 68 families, the litmus tests' counts.

## artifact contract

### Files modified

- `crates/w2m_vm/src/concurrent_memory.rs`, `crates/w2m_vm/tests/slice_calls.rs`,
  `docs/design-guest-memory-on-several-vcpus-2026-10-31.md`, `docs/step-6-census-2026-10-30.md`, `BLACKBOARD.md`

### Command exit codes

- `scripts/gates.sh` returns 0
- `target/release/design_check` returns 0

## observation contract

- **Expected values:** copies of every length 0 to 40 at every offset 0 to 15 read back what was written; two threads,
  one copying out of a buffer while another writes it word by word, finish with each byte one of the two values
  written; the count test passes at today's figures.
- **Planted faults:**
  - (1) the copy's middle loop stopping one word early: the length test names the length and offset.
  - (2) one `ConcurrentGuestMemory::new(` added outside the module: the count test names the file.

## done criteria

The view's copies are atomic per word and tear rather than race, the design says per structure which access the host
uses and why, and the slice-built views are counted.

## self-review checklist

- [x] Scope names one verifiable property: the view's copies made defined under a racing guest.
- [x] context_files cite the module, the design, the count test, the review and the sources.
- [x] Emits: none.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and two plants.
- [x] One concept: the view's copy semantics, with the table and count that rest on it.

## result

- **The copies** (54a6d4c01): `read` and `write` load and store `AtomicU8` up to 8-byte alignment, `AtomicU64` over
  the aligned words, and `AtomicU8` for the tail, all relaxed, in native byte order; the module's head and the Sync
  comment say so, with RFC 3301 and P1478.
- **The table** (Low 3): `docs/design-guest-memory-on-several-vcpus-2026-10-31.md`, "The structures the host reaches",
  nine rows. One row names a defect for two vCPUs: `x86_address_at` reads the reconstruction table's count plain, then
  its entries, and the host's Arm ordering lets those loads pass each other; the card that moves `run.rs` reads the
  count Acquire. Choice 2 amended, choice 4's "plain" corrected, the type's lines 282.
- **The count** (Low 1): `tests/slice_calls.rs` counts `ConcurrentGuestMemory::new(` outside its module: run.rs 7,
  cases.rs 1, guest.rs 1, monitor_code.rs 1; census D7 says so.
- **Expected values:** every length 0 to 40 at every offset 0 to 15 reads back, nothing written outside it; 20,000
  copies racing 20,000 writes read only bytes written; the count tests pass.
- **The gates:** `scripts/gates.sh` exit 0 in 670 s, kept in `store/gates/passes.tsv` as sprint 279 at 54a6d4c01 with root=358439357 and "rust=crates python=0 samples=0", rdeps(w2m_vm), 156 tests, base=77d21ab2f, Sprint 278's close; multi-vCPU: not run (Python gates not run)
- **Plant 1,** replaced: the card's plant, the word loop stopping a word early, cannot fire, as the byte loop after it
  copies what is left and the result is the same; the plant made instead puts a word's bytes in big-endian order, by a
  one-line `sed`, restored by `cp` and checked by `cmp`; the test failed, "length 1 at offset 0 wrote outside its
  bytes".
- **Plant 2,** restored by `cp` and checked by `cmp`: a second `ConcurrentGuestMemory::new(` in `cases.rs`; the count
  test failed, "cases.rs: 2 views built from a slice, census D7 allows 1".
