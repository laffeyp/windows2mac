# Sprint 276 — Guest memory on several vCPUs

```yaml
---
id: 276
status: done
phase: 6
pass_kind: functional
---
```

## scope

Reviewer-3's Medium 1 at 275: the host reaches guest memory through `GuestMemory::as_slice` and `as_mut_slice`
(`crates/hypervisor/src/lib.rs` 355-361), a `&[u8]` or `&mut [u8]` over the whole mapping, and since Sprint 274 other
vCPUs write that mapping while such a reference lives. Rust's rules make that undefined: "data races are undefined
behavior. A data race is defined as conflicting non-synchronized accesses where at least one of the accesses is
non-atomic" (`std::sync::atomic`), and volatile is no remedy, "whether an operation is volatile has no bearing
whatsoever on questions involving concurrent accesses from multiple threads" (`std::ptr::read_volatile`). The
virtual-machine monitors that share guest memory with running vCPUs keep two rules, which `vm-memory` (Firecracker's
`vmm` and Cloud Hypervisor, both 0.18.0) and crosvm's `base/src/volatile_memory.rs` state alike: "No references or
slices to volatile memory (`&` or `&mut`)", because "having references of any kind to memory considered volatile would
violate pointer aliasing", and words shared with running code are reached by atomics (`vm-memory`'s
`get_atomic_ref`, `load`, `store`).

- **Where it stands, by lookup (2026-10-31):**
  - 218 calls of `as_slice` or `as_mut_slice` in 30 files outside the hypervisor crate. On one vCPU each is made while
    the one vCPU is stopped at its exit, so no other writer exists; on several vCPUs none holds.
  - The live instances on several vCPUs: the litmus harness (`crates/w2m_vm/tests/litmus.rs` 203-311), and Sprint
    275's take and drain (`monitor_code::runtime_word` derives its atomic from a `&mut [u8]` over the whole memory) and
    their tests (`run.rs` 1155-1177 makes two `&mut [u8]` over one buffer on two threads).
  - A defect, not only a rule: `run::dispatch_insert` (`run.rs` 388-400) publishes an entry by two plain stores, the
    value then the target, which the guest reads with no lock. The compiler may reorder plain stores, and so may the
    host's CPU, which runs Arm's model (TSO is set on the guest's vCPUs only), so another vCPU may match the new target
    and branch through the old value.
  - `GuestMemory`'s `Send` SAFETY comment (`lib.rs` 294-297) says several threads reach the memory only through a
    lock; the vCPUs of Sprint 274 write it outside any host lock.
- **The work:**
  - A design note, `docs/design-guest-memory-on-several-vcpus-2026-10-31.md`: the rule (a slice only while every vCPU
    that may write the memory is stopped), the two paths for the rest (bytes copied through a raw pointer; words
    shared with running code by atomics), the publish order, and why a type of w2m's own rather than `vm-memory`.
  - `w2m_vm::concurrent_memory::ConcurrentGuestMemory`: a pointer and a length, `Copy`, `Send` and `Sync`, which
    makes no reference into the memory: `read` and `write` of bytes, of `u32` and of `u64`, and `atomic_u32` and
    `atomic_u64` at an aligned offset.
  - Moved to it: the runtime word's take and release, `vm_compiled`'s drain, `dispatch_insert` (the value written,
    then the target by a `Release` store), and the litmus harness's own reads and writes.
  - Reviewer-3's Low 1 at 275: the waiting tests read no time; the take calls a hook each time it finds the word
    held, and the holder releases only after the hook has run.
  - `GuestMemory`'s SAFETY comment states what holds.
  - Step 6's census, row D7: the slice calls left, counted by file, sound on one vCPU, moved by the cards that put a
    second vCPU in the run loop (G2 to G4).
- **Not here:** the compile path's slices (`HostCompiler::compile`, `compile.rs`), which the litmus harness runs under
  its lock while other vCPUs run; they are among D7's count.

## prerequisites

- 275 (closed)

## context_files

- `crates/hypervisor/src/lib.rs` 287-368 (`GuestMemory`); `crates/w2m_vm/src/monitor_code.rs` 130-160 (the take)
- `crates/w2m_vm/src/run.rs` 388-475 (`dispatch_insert`, `vm_compiled`), 1144-1215 (the tests)
- `crates/w2m_vm/tests/litmus.rs`; `docs/step-6-census-2026-10-30.md`
- https://doc.rust-lang.org/std/sync/atomic/index.html; https://doc.rust-lang.org/std/ptr/fn.read_volatile.html
- https://github.com/rust-vmm/vm-memory/blob/main/src/volatile_memory.rs;
  https://github.com/google/crosvm/blob/main/base/src/volatile_memory.rs

## signal contract

### Emits

- None; an access path, held by tests.

### Invariants

- One-vCPU runs unchanged: the batch run's stop and stream as held.
- The litmus tests' counts as Sprint 274 held them: forbidden outcomes 0.

## artifact contract

### Files modified

- `crates/w2m_vm/src/` (`concurrent_memory.rs` new, `lib.rs`, `monitor_code.rs`, `run.rs`), `crates/w2m_vm/tests/litmus.rs`,
  `crates/hypervisor/src/lib.rs`, `docs/design-guest-memory-on-several-vcpus-2026-10-31.md` new,
  `docs/step-6-census-2026-10-30.md`, `BLACKBOARD.md`

### Command exit codes

- `scripts/gates.sh` returns 0
- `target/release/design_check` returns 0

## observation contract

- **Expected values:** a word written by `write_u64` reads back by `read_u64` and by `atomic_u64`; a misaligned atomic
  refused; the take's hook runs before the holder releases; `dispatch_insert`'s entry as before; the litmus counts as
  274's.
- **Planted faults:**
  - (1) the take made a plain store: the waiting test names a take whose hook never ran, with no time read.
  - (2) `ConcurrentGuestMemory::read_u64` reading at the offset plus 4: the round-trip test names the word read.

## done criteria

Every host access to guest memory made while another vCPU runs goes through a path that forms no reference into it,
the dispatch table's entries are published value first, and the census counts the slices left for the run loop's
second vCPU.

## self-review checklist

- [x] Scope names one verifiable property: guest memory reached with no reference while other vCPUs run.
- [x] context_files cite the memory type, the take, the drain, the dispatch insert, the harness and the sources.
- [x] Emits: none.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and two plants.
- [x] One concept: the access path for memory other vCPUs write.

## result

- **The view** (545d19c49): `w2m_vm::concurrent_memory::ConcurrentGuestMemory`, 194 lines with its tests: `read`,
  `write`, `read_u32`, `read_u64`, `write_u32`, `write_u64`, `atomic_u32` and `atomic_u64` (aligned, refused
  otherwise), and `store_pair_release`, one `STP` after a release fence, on aarch64.
- **Moved to it:** the runtime word's take and release (`monitor_code`), `dispatch_insert`, which now stores each pair
  whole, `x86_address_at` and `recon_lookup`, its bisection now reading entries through the view, and the litmus
  harness's dispatch inserts and code reads, through a view made once before its threads start. `cases.rs`'s one
  `dispatch_insert` call follows the new signature, a file the card's list did not name.
- **The waiting tests:** `host_take_runtime_word_waiting` calls a hook each time the word is held. The take's test
  frees the word on the holder's thread only after the hook has run; the drain's test (`vm_compiled_waiting`) frees it
  from the hook on one thread. Neither reads a time. The take's tests moved to `monitor_code`, which builds on Linux, so
  CI runs them; `run`, where 275 put them, is macOS-only, and 275's entry said CI ran them.
- **`GuestMemory`'s SAFETY comment** says `Send` covers ownership only, a slice is sound only while every vCPU that may
  write is stopped, and the view serves the rest.
- **The design note** `docs/design-guest-memory-on-several-vcpus-2026-10-31.md`, the Decision of 2026-10-31, and census
  row D7: 209 slice calls left in `crates/w2m_vm`, 174 in `src/`.
- **Expected values:** the `w2m_vm` library tests, 81 (76 at 275), including a word read back by every path, a
  misaligned atomic and an access past the end refused, and a pair read back low then high; the litmus tests: MP, LB,
  2+2W and IRIW's forbidden outcome 0 in 100,000 each, SB's relaxed outcome 7,332, EnTSO cleared MP's forbidden outcome
  2,007; the run-loop tests, 11.
- **The gates:** `scripts/gates.sh` exit 0 in 703 s, kept in `store/gates/passes.tsv` as sprint 276 at 545d19c49 with root=8a3e4053d and "rust=crates python=0 samples=0", rdeps(hypervisor) and rdeps(w2m_vm), 158 tests, base=c21789e7e, Sprint 275's close; multi-vCPU: not run (Python gates not run)
- **Plant 1,** restored by `cp` and checked by `cmp`: the take a plain store; the test failed, "the take did not wait
  for the other vCPU's hold".
- **Plant 2,** restored by `cp` and checked by `cmp`: `read_u64` reading at the offset plus 4; the test failed,
  "assertion `left == right` failed: the word read at 8 left: 12302652056939934532 right: 1234605616436508552".
