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
   `atomic_u64`), the same physical word the guest reaches by `CASA`, `LDAR` and `STLR`. *(Amended 2026-10-31, Sprint
   279: the copies were plain, `copy_nonoverlapping`; a guest thread writing a buffer the host copies, as Windows
   allows, made that a data race, undefined in Rust's model whatever the protocol. The copies are now relaxed atomic
   loads and stores, a word where aligned and a byte at the ends, so a racing guest gives torn bytes, defined. RFC 3301,
   "AtomicPerByte (aka atomic memcpy)" (https://github.com/rust-lang/rfcs/pull/3301, open, not merged): "All of the data
   would have to be written and read through atomic operations to prevent a data race … it's fine if there's tearing";
   C++'s P1478 `atomic_load_per_byte_memcpy` is "effectively a series of `AtomicU8::load`s". The protocol still sets
   the ordering, by the table below; Rust's one remaining limit is that atomics of different sizes must not race on the
   same bytes from two host threads, so no bulk copy covers a word another host thread reaches by `atomic_u32` or
   `atomic_u64` at the time.)*
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
   volatile copies; w2m has one mapping, and its copies are relaxed atomics, not volatile, because volatile changes
   nothing for concurrency (plain until Sprint 279).
   The type is 282 lines with its tests (194 at Sprint 276), and its interface is the four kinds of access the host makes: bytes, words,
   atomic words and the published pair.
5. **Waiting tests read no time.** The take of the runtime word calls a hook each time it finds the word held
   (`host_take_runtime_word_waiting`); a test frees the word from the hook, or from the holder's thread once the hook
   has run, and so tells a take that waited from one that did not (reviewer-3's Low 1 at 275).
6. **The runtime word is the outermost lock** (Sprint 287's close, reviewer-3's Medium 1 at 287). A host thread waits
   for the word holding no host lock, and takes a host structure's own lock (`HostCompiler`, the arena, the file
   bridge) only inside it; no lock covers `Guest` whole, and each thread reaches guest memory through the view. A guest
   holds the word across `hvc #8`, and `host_take_runtime_word` spins while another vCPU holds it, so a host lock held
   over that spin would wait on the request's service while the service waits on the lock. The second vCPU's race
   test holds the word across a host request on one vCPU while the other misses, a bound on the run's steps failing a
   hang, and plants a lock over `Guest` held across the take.

## The structures the host reaches (Sprint 279, reviewer-3's Low 3 at 278)

Each card that moves a module to the view cites the row of each structure it touches. "Copy" is the view's tearing
copy; the ordering column says what makes the bytes the host reads the ones it needs.

| Structure | Where | Access | Ordering, and the fact that makes it so |
|---|---|---|---|
| the runtime word | `RUN_TABLE + RUNTIME_LOCK` (0x80) | `atomic_u32`: compare-and-swap Acquire, store Release | the lock itself; the guest takes it by `CASA` and frees it by `STLR` (Sprint 275) |
| the run table's other words: thread entries, the clock's warp, counts | `RUN_TABLE` | copy, `read_u64`, `write_u64` | the runtime word, which the guest holds while it writes them; on a second vCPU the host holds it across each serve, the frozen host's rule (Sprint 118, `vm_backend.py` 5011 and 5053 at tag python-final) |
| the reconstruction table | `RECON_TABLE`, its count at `RUN_TABLE + RECON_COUNT` (0xC0) | entries: copy; the count: `atomic_u64` Acquire | entries below the count are published (vm_backend 4855-4859); the guest appends under the runtime word on TSO, but the host runs Arm's model, so it reads the count Acquire before the entries; `x86_address_at` reads it by `atomic_u64` with Acquire (Sprint 282; plain before), and the host's own appender, `HostCompiler::place`, stores it Release after its entries (Sprint 283). Three writers append, each under the runtime word (the host's since Sprint 286): `HostCompiler::place`, the in-VM compiler (`block_compiler.c`, the entries then the count) and the installer (`monitor_code.rs`); the guest's two store plainly, ordered entries before count by EnTSO, which every vCPU has set (`ACTLR_ENTSO`); `litmus.rs`' plant shows MP's forbidden outcome 549 to 1,652 times a run with EnTSO cleared, so a guest vCPU without it would break this row (reviewer-3's Low 1 at 283) |
| the dispatch table | `DISPATCH_TABLE`, 16-byte slots | each pair stored whole by one `STP`: the host's `dispatch_insert` by `store_pair_release`, the in-VM compiler's `dispatch_insert` by `store_pair` (`block_compiler.c`), the installer by `stp` (`monitor_code.rs`); copy to read | the dispatcher's one `LDP` sees a whole pair (choice 3), so long as every writer stores whole pairs; three writers, not one (reviewer-3's Medium 1 at 283: until Sprint 284 the in-VM compiler and the installer stored a pair word by word, and a move of the first way to the second left a slot torn for a few stores); a moved pair is stored at the second way before the new pair at the first, the first way read whole by one `LDP` (`load_pair` in the host, inline assembly in the C, `ldp` in the installer; Sprint 286); all three writers hold the runtime word, so they exclude each other, and the dispatcher reads meanwhile |
| the in-VM compiler's log and counters, the code cursor | `VM_COMPILER + VMC_LOG_COUNT`, `VMC_CURSOR`, the log | copy | the runtime word, which every writer holds: the in-VM compiler and the installer, which take it in the miss stub; the host's drain (Sprint 275, census D6); and the host's miss service, its `install`, `compile` (`place`) and `dispatch_insert`, which take it after the stub released it before `hvc 1` (Sprint 286, reviewer-3's Medium 1 at 285's close: before, they ran outside it), or find the vCPU holding it already |
| the code area | `MONITOR_VA` code | copy | written before the dispatch slot that names it is published by `store_pair_release`, whose release fence orders it |
| the transport page | `PROCESS_VA + TRANSPORT_OFF` | copy | the requesting vCPU holds the runtime word across `hvc #8` (`design-rust-host-2026-10-24.md`, change 2, "Today"); the page goes per vCPU with change 2 |
| a guest buffer a host request names (a file read's destination, a path, a structure) | anywhere in the guest | copy | none, as on Windows: another guest thread may write it while the host copies, and gets torn bytes, defined; the requesting thread is stopped at its exit |
| a vCPU's own state block, save area and monitor data | `MONITOR_DATA` and each vCPU's area | copy | its vCPU is stopped at the exit its host thread serves; no other vCPU reaches it |
| the stage-1 translation tables (Sprint 281) | `TRANSLATION_TABLES`, `SPLIT_TABLES` | a descriptor by `atomic_u64`, Release; a split table's 512 descriptors by copy before the entry that names it, Release | only the host writes them, at exits (`Guest::page_descriptor`, `map_pages`, called by the scheduler's thread creation and stack growth); a vCPU's table walk reads them. A page made valid needs no TLB maintenance, as a translation fault is never served from the TLB ("since the fault is from a TLB-miss it reads from memory", Armv8-A relaxed virtual memory test CoTfRpte+dsb-isb, https://www.cl.cam.ac.uk/~pes20/RelaxedVM-Arm/tests/CoTfRpte+dsb-isb.html); a page made invalid is followed by the runtime's `TLBI VMALLE1IS; DSB ISH; ISB` on every vCPU (`runtime.h` 331-335, called at `threads.c` 318 and `runtime.h` 342). A block split into a table changes the block size, which needs break-before-make while another vCPU may hold the block in its TLB: a TLB conflict abort is allowed otherwise, even with FEAT_BBM level 2 (Will Deacon, 2024-12-10, quoted in Jonathan Cameron's reply of 2025-01-02 on LKML, "arm64: support FEAT_BBM level 2 and large block mapping": "I think you can still get TLB conflict aborts in this case, so this doesn't work", and Huawei's implementations "might report TLB conflict abort after changing block size directly", https://lkml.iu.edu/hypermail/linux/kernel/2501.0/00462.html), and this Mac's `sysctl` lists no FEAT_BBM; on one vCPU the only vCPU is stopped during the split, so a precondition of the second vCPU |
| the process area: a system DLL's synthetic page, the loader data, `PEB.Ldr` (Sprints 290, 291) | `PROCESS_VA` (`SYSTEM_IMAGES_OFF`, `LOADER_DATA_OFF`, `PEB_OFF + 0x18`) | at load a slice (`process::ProcessBytes` for `[u8]`), no vCPU running; at a run's LoadLibrary `process::ProcessView`, each write a copy bounded by `PROCESS_SIZE`, each list link and `PEB.Ldr` a Release store of the aligned word (`ProcessBytes::publish`) | the requesting vCPU holds the runtime word across `hvc #8`, which the guest runtime's module calls take; application code walking `PEB->Ldr` takes nothing, so each entry's fields and names are written first and each list's links so every state between two writes is a whole list, a node's link after the node it names has its own, the head's last, as Windows links an entry filled (`crates/w2m_vm/tests/loader_lists.rs` walks the three lists both ways after each write); the guest reads under EnTSO, so a Release store orders the writes before it |

## Against the goal

The goal is play at speed: about 37,600 system calls a second in play and 41 million instructions a frame over six
threads, to the first scene (`docs/rust-port-plan-2026-10-24.md` 32-40). Six threads on several vCPUs is plan step 6,
and the host serves each vCPU's exits while the others run. Every access it makes then either keeps the rule of choice
1 or goes through the view; a reference held across another vCPU's write is undefined behaviour, which the compiler may
turn into a wrong read of a lock word or a lost store to a dispatch slot. The view takes no lock, and reads the same
bytes the slices read; the pair store adds one fence to each dispatch insert. Its effect on the run's rate is not
measured here; the batch run in the gates holds its stream and stop. *(Amended 2026-10-31, Sprint 285: a large copy
through the view costs more than a slice's: its relaxed atomic words are not vectorized, where `copy_from_slice` is.
The case runner fills the 16 MiB dispatch table before each case; through the view its test took 145.88 s, by one
slice copy 69.29 s, the committed slice code 68.66 s. The runner, one vCPU stopped between cases, keeps that one
slice. The game's large copies, a host request's file reads, go through the view; their rate on several vCPUs is
measured with G5 and G6, and a faster whole-word copy, by pair or vector loads, is weighed then.)* *(Amended
2026-10-31, Sprint 286, reviewer-3's Low 1 at 285's close: on one vCPU the game pays nothing measurable for the view's
copies. `w2m-vm` built at 77d21ab2f, Sprint 278's close, before Sprint 279 made the copies atomic, against 91802f99a,
the game with no exit ledger and no block judge, runs alternated: 27.77 s and 25.21 s against 25.98 s and 25.56 s, at
loads 2.84 to 3.17, every stop "ReferenceEnd"; the two old runs differ by more than the means.)*

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
