# Guest memory on several vCPUs (Sprint 276)

The Rust host reaches guest memory through `GuestMemory::as_slice` and `as_mut_slice` (`crates/hypervisor/src/lib.rs`),
a `&[u8]` or `&mut [u8]` over the whole mapping. On one vCPU each such reference is made while the one vCPU is stopped
at its exit, so nothing else writes the memory while it lives. Since Sprint 274 the host runs several vCPUs of one VM
at once, and the vCPUs write the mapping outside any host lock. This note sets how the host reaches memory that a
running vCPU may write.

## The rules Rust sets

- "Data races are undefined behavior. A data race is defined as conflicting non-synchronized accesses where at least
  one of the accesses is non-atomic" (`std::sync::atomic`, https://doc.rust-lang.org/std/sync/atomic/index.html).
- Volatile is no remedy: "whether an operation is volatile has no bearing whatsoever on questions involving concurrent
  accesses from multiple threads. Volatile accesses behave exactly like non-atomic accesses in that regard"
  (https://doc.rust-lang.org/std/ptr/fn.read_volatile.html).
- A `&mut [u8]` promises that nothing else reaches the bytes it covers while it lives, and a `&[u8]` that nothing
  changes them. A `&mut [u8]` passed to a function holds that promise for the whole call, so a write by another thread
  during the call breaks it, even an atomic write.

## The choices

1. **A slice of guest memory only while every vCPU that may write it is stopped.** The one-vCPU run loop keeps this at
   each exit; its 174 slice calls in `crates/w2m_vm/src/` stand (step 6's census, row D7) until the run loop runs a
   second vCPU.
2. **Otherwise, a view that makes no reference:** `w2m_vm::concurrent_memory::ConcurrentGuestMemory`, a pointer and a
   length, `Copy`, `Send` and `Sync`. It copies bytes in and out through the pointer (`read`, `write`, `read_u64` and
   the rest) where the guest's protocol says no running vCPU writes them at the time: under the runtime word, or before
   they are published. A word that running guest code may write is reached as an atomic (`atomic_u32`,
   `atomic_u64`), the same physical word the guest reaches by `CASA`, `LDAR` and `STLR`.
3. **A pair the guest reads at once is published at once.** The dispatcher reads a dispatch slot, target and code
   address, by one `LDP` from a 16-byte-aligned slot (`monitor_code.rs`, `dispatcher`). With FEAT_LSE2 an aligned
   pair access of two 64-bit registers to Normal write-back memory is single-copy atomic (DDI 0487 B2.2.1.1); this M5
   Max reports `hw.optional.arm.FEAT_LSE2: 1`, and the guest maps its memory Normal, inner and outer write-back
   (`MAIR` 0xFF, `crates/w2m_vm/src/lib.rs`). So `dispatch_insert` stores each pair by one `STP` after a release fence
   (`store_pair_release`). Before, it wrote the code address and then the target by two plain stores, which the
   compiler and the host CPU may reorder: the host runs Arm's model, and TSO is set on the guest's vCPUs only. On two
   vCPUs a guest could have matched a new target and branched through an old address, or the reverse when a filled
   slot is rewritten.
4. **A type of w2m's own, not `vm-memory`.** `vm-memory` serves several regions, guest addresses, dirty bitmaps and
   volatile copies; w2m has one mapping, and its copies are plain because volatile changes nothing for concurrency.
   The type is 194 lines with its tests, and its interface is the four kinds of access the host makes: bytes, words,
   atomic words and the published pair.
5. **Waiting tests read no time.** The take of the runtime word calls a hook each time it finds the word held
   (`host_take_runtime_word_waiting`); a test frees the word from the hook, or from the holder's thread once the hook
   has run, and so tells a take that waited from one that did not (reviewer-3's Low 1 at 275).

## Against the goal

The goal is play at speed: about 37,600 system calls a second in play and 41 million instructions a frame over six
threads, to the first scene (`docs/rust-port-plan-2026-10-24.md` 32-40). Six threads on several vCPUs is plan step 6,
and the host serves each vCPU's exits while the others run. Every access it makes then either keeps the rule of choice
1 or goes through the view; a reference held across another vCPU's write is undefined behaviour, which the compiler may
turn into a wrong read of a lock word or a lost store to a dispatch slot. The view takes no lock, and reads the same
bytes the slices read; the pair store adds one fence to each dispatch insert. Its effect on the run's rate is not
measured here; the batch run in the gates holds its stream and stop.

## Against the systems that solved it

- **Firecracker** and **Cloud Hypervisor** reach guest memory through `vm-memory` 0.18.0 (Firecracker's
  `src/vmm/Cargo.toml`, https://github.com/firecracker-microvm/firecracker/blob/main/src/vmm/Cargo.toml; Cloud
  Hypervisor's `Cargo.toml`, https://github.com/cloud-hypervisor/cloud-hypervisor/blob/main/Cargo.toml), whose rules are
  "No references or slices to volatile memory (`&` or `&mut`)", because "having references of any kind to memory
  considered volatile would violate pointer aliasing"
  (https://github.com/rust-vmm/vm-memory/blob/main/src/volatile_memory.rs). It reaches shared words by atomics
  (`get_atomic_ref`, `load` and `store` with an `Ordering`). Taken: no reference, atomics for shared words; not taken:
  its volatile copies and its region map.
- **crosvm** states the same two rules in `base/src/volatile_memory.rs`
  (https://github.com/google/crosvm/blob/main/base/src/volatile_memory.rs), the code `vm-memory` grew from, and its
  `GuestMemory` copies through `read_at_addr` and `write_at_addr`. Taken: copies in and out of the host's own buffers.
- **QEMU** reads and writes 16 bytes at once on an Arm host by an aligned `LDP` or `STP` when FEAT_LSE2 is present
  ("With FEAT_LSE2, 16-byte aligned STP is atomic", `host/include/aarch64/host/atomic128-ldst.h` at commit 052af14ef,
  https://qemu.googlesource.com/qemu/+/052af14ef26631b5d7a5549797abeacb42b4db8d/host/include/aarch64/host/atomic128-ldst.h).
  Taken: the dispatch slot's pair store.
- **Wine** keeps a process's objects in its server ("the clients must send requests to the Wine server", Wine
  Developer's Guide, Architecture Overview,
  https://gitlab.winehq.org/wine/wine/-/wikis/Wine-Developer's-Guide/Architecture-Overview, as the plan quotes it).
  Nothing in that source covers a host reaching a running guest's memory; nothing taken.
- **Rosetta 2** ("If Rosetta newly encounters code that haven't been translated at install time, then we'll compile it
  on the fly", WWDC20 session 10686, https://developer.apple.com/videos/play/wwdc2020/10686/) and **FEX** (its JIT's
  code looked up "from that FOZ database first", FEX-2609, https://fex-emu.com/FEX-2609/), as the plan quotes them:
  both quotes concern translating and caching code, not memory shared with running vCPUs; nothing taken.

## Revisit if

- The run loop runs a second vCPU (census G2 to G4): its slice calls move to the view, or the loop stops every vCPU
  before it takes one.
- A host without FEAT_LSE2 is supported: the pair store becomes a sequence the dispatcher can check, or a lock.
