# The run loop on several vCPUs (Sprint 293)

Step 6 of `docs/rust-port-plan-2026-10-24.md`, after census row D7 (`docs/step-6-census-2026-10-30.md`): every host
access to guest memory while a vCPU may run goes through `ConcurrentGuestMemory`
(`docs/design-guest-memory-on-several-vcpus-2026-10-31.md`). The run loop still holds the whole guest as one
`&mut Guest` (`crates/w2m_vm/src/run.rs`), which a second vCPU's thread cannot share. The host design
(`docs/design-rust-host-2026-10-24.md`) fixes the target: a thread per vCPU that creates it, runs it and serves its
exits; a host lock per part (`modules`, `vm_map`, `code_page`, `files`, `file`, `output`, `trace`, `sched`,
`vcpu_handle`); no lock across `hv_vcpu_run`. The census fixes the first move: a second vCPU under today's one runtime
word, the frozen host's shape, before the guest's locks are split ("What the counts say"). This document divides
today's `Guest` for that move, and orders the cards from here to gate items G2 to G4.

## The rule

**Every service that changes state two vCPUs share runs under the runtime word, and its host state is reached through
one host lock, `served`, taken only by a thread whose vCPU or whose host side holds the word.** The word is the
outermost lock (the guest-memory design's choice 6); `served` is inside it, so no thread holds `served` while it waits
for the word, and two threads never contend for `served`, the word having already chosen between them. What a vCPU
owns, and what is set once at load, needs no lock.

One host lock is the frozen host's shape, not the host design's table. It is the step the census names: the batch run,
the families and the guests are judged on two vCPUs with the locking they were judged with on one. The table's locks
replace `served` one part at a time after G4 (the host design's change 1 and its table), each measured by G6.

## The exits

Each exit the run loop serves (`run.rs` 796-1100), whether the exiting vCPU holds the word, and what its service
touches. The word is read where the guest leaves it: a bridge holds it across `hvc #8` (`guest.rs`, `_bridge`, "call
the C service on the runtime stack under the runtime lock"); the switch stub releases it before `hvc #12`
(`switch_stub`: `unlock_runtime`, then `hvc(HVC_SCHEDULE)`); a slice end's `hvc #4` comes from a translated block,
which holds nothing; the miss stub releases it before `hvc #1` (Sprint 286).

| Exit | The vCPU holds the word | Its service touches | The rule's form |
|---|---|---|---|
| `hvc #1`, a dispatch miss | no | the code cursor, the reconstruction and dispatch tables, `bridges` | the host takes the word (Sprint 286), then `served` |
| `hvc #4`, a slice's end | no | `threads` (`set_budget`, `wake_expired`, `schedule`), `link` | the host takes the word, then `served`; released before any turn wait |
| `hvc #8`, a host request | yes | `served`'s every part, by request | `served`, under the guest's hold; no service waits but a child's KERNEL 33 (below) |
| `hvc #9`, an import with no service | the bridge's | `thunks` (read), `unimplemented` | the stop's record (below) |
| `hvc #10`, `#11`, a model step | no | the vCPU's own areas, guest memory | the view and no word; a form x86 locks, every other vCPU stopped (precondition 8) |
| `hvc #12`, a schedule hand-over | no, released before it | `threads`, `link` | as `hvc #4` |
| `hvc #13`, a write to translated code | not established | the host compiler's tables, `code_page` | the host takes the word (`host_take_runtime_word` returns at once if this vCPU holds it), then `served` |
| `hvc #14`, a fault given back | no | the vCPU's count | none |
| the vtimer, a cancel | no | the vCPU's own | none |

A wait never holds the word or `served`: a process's turn (`hand_over`, `turns.rs`) and a parked vCPU's wait (the
`sched` condition, census D2) release both first and take them again after, as the host design's "no thread waits ...
for a lock held by another thread's service" asks.

**The host requests that can wait** (Sprint 295, reviewer-3's Low 2 at 294). A request's service runs while the guest
holds the word, and the host cannot let it go: the guest's C may be in the middle of its own state. Read from the
code: a pipe read on an empty pipe answers at once, its second word 1 or 2, "would block", and the guest waits in its
own code (`kernel.rs` 270-272); the file bridge's one loop names a drive at its creation (`file_bridge.rs` 161), no
wait. One service waits: a child process's KERNEL 33 request, which the supervisor serves in its turn
(`host_request.rs` `child_request`, `turns.rs` 264, `hand_back(Message::Request, …)`), the child's word held by its
guest throughout. So **a child process keeps one vCPU** until KERNEL 33 waits with no guest lock held, the host
design's change 2 ("no guest lock across `hvc #8`"); the supervisor's process may run on two, its own turn waits
letting the word go (`turn`, Sprint 294).

**A model step of a locked form** (Sprint 295, reviewer-3's Medium 1 at 294). x86 makes a `LOCK`-prefixed form and a
memory XCHG atomic at any alignment; the VM runs an aligned one as an exclusive pair, and the model steps a misaligned
one as a load and a store (`hvc #11`; Deferred, Sprint 272). The word would not exclude the aligned pair, which takes
none. So the step of such a form runs with every other vCPU stopped: the stepping thread calls `hv_vcpus_exit` on the
others and waits until each has left `hv_vcpu_run` and said so before the step's load, since `hv_vcpus_exit` only
asks; each resumes after the step's store. An exclusive pair interrupted by the stop must fail its store on resume:
guest memory is Inner Shareable (`PAGE_ATTRIBUTES`, `lib.rs` 75), so its exclusive accesses are checked against a
global monitor, and an exclusive store fails "if the check fails on any monitor" (Arm, "ARMv8-A Synchronization
primitives"); that a plain store from another core clears the mark is the race test's to show, a `clrex` on re-entry
after a stop the fallback.

## The guest's parts

Each of `Guest`'s 45 fields (`crates/w2m_vm/src/guest.rs`, `pub struct Guest`), by the writers found in the code after
load. *Load*: written while the guest is built, read after, by any thread, no lock. *Served*: behind `served`, under
the word. *View*: guest memory, through `ConcurrentGuestMemory`. *Diagnostic*: a one-vCPU tool; a run with it refuses
a second vCPU.

| Field | Part | Its writers after load |
|---|---|---|
| `memory` | View | the view's copies everywhere; slices at load, in tests and in the exit ledger (census D7) |
| `layout` | Load | none |
| `ttbr` | Load | none |
| `code` | Load | none |
| `process` | Served | `modules.rs` `module` (`process.system.insert`), a system DLL loaded |
| `switch_va` | Load | none |
| `switch_slice_va` | Load | none |
| `thunks` | Served | `guest.rs` `make_thunk`, from LoadLibrary, thread start and end, `guest_stubs` |
| `imports` | Served | `guest.rs` `import_thunk` |
| `proc_thunks` | Served | `modules.rs` (GetProcAddress's thunk) |
| `bridges` | Served | `guest.rs` `make_thunk`; read at `hvc #1` |
| `slice_va` | Load | none (`with_trace`, at load) |
| `entry_va` | Load | none (`with_trace`) |
| `services` | Load | none |
| `symbols` | Load | none |
| `arena` | Served | VirtualAlloc and its kin (`commit`, `protect`, `free`), `modules.rs` `load_dll` (`place_image`, `snapshot_image`), a thread's stack (`stacks.push`) |
| `stub_page` | Served | `guest.rs` `guest_stubs`, on first use |
| `stdout` | Served | `host_request.rs` (a guest's write to standard output) |
| `image_range` | Load | none |
| `dll_ranges` | Served | `modules.rs` `load_dll` |
| `next_table` | Served | `guest.rs` `page_descriptor`, a block split |
| `threads` | Served | `threads.rs` (a thread created, its exit code) |
| `time_runtime_word` | Load | none |
| `images` | Served | `modules.rs` `load_dll` |
| `init_order` | Load | none |
| `image_entry` | Load | none |
| `dependency_order` | Load | none |
| `dll_paths` | Served | `modules.rs` `load_dll` |
| `pending_init` | Served | `modules.rs` `load_dll`; taken by `host_request.rs` at the module request's second answer |
| `call_trace` | Load | none |
| `app_root` | Load | none |
| `trace_from` | Load | none (`with_trace`) |
| `trace_quiet` | Served | `guest.rs` `traced_proc` (GetProcAddress traced) |
| `trace_dynamic` | Served | `guest.rs` `traced_proc` |
| `trace_bound` | Served | `guest.rs` `trace_ranges`, from `load_dll` |
| `kernel` | Served | `host_request.rs` and `run.rs`, a request to the supervisor and the turn |
| `link` | Served | `host_request.rs` (a child started), `run.rs` (the turn) |
| `start` | Load | none |
| `random` | Served | `host_request.rs` (random bytes) |
| `files` | Served | `host_request.rs` (a file request) |
| `limited` | Load | none |
| `exit_ledger` | Diagnostic | `host_request.rs`, `run.rs`, at every exit |
| `block_judge` | Diagnostic | `run.rs`, at every exit |
| `batch_stop` | Diagnostic | `run.rs`, at an answered request |
| `unimplemented` | Served | `host_request.rs` and `run.rs`, at a stop; the first stop's |
| `splits` | Served | `guest.rs` `page_descriptor`, a valid block made a table (Sprint 304) |
| `vcpu` | Served | `run.rs` `GuestLock::serve`, the served exit's vCPU's pages (Sprint 313) |

19 Load, 24 Served (`splits` from Sprint 304, `vcpu` from Sprint 313), 1 View, 3 Diagnostic, counted from the
table. `memory` stays one mapping; `Guest` becomes three values the run loop's threads hold: the load part, shared by reference; `served`, a host mutex over the 22; and the
mapping, from which each thread takes its view.

**The run loop's own state** (`run.rs`, `run`): `HostCompiler`'s tables are Served, reached at `hvc #1` and `#13`
under the word; the exit counts, `run_ns`, `serve_ns`, `serve_by_kind` and the other fields of `Outcome` are each
vCPU's own, merged when every thread has ended; `out.bridged`, `blocks` and `translations` are Served.

**How the threads share `Guest`** (Sprint 309, read from `run.rs` `run`, 693-1261). Outside a service the run loop
touches the Load part (`code.dispatch_va`, `code.miss_va`, `ttbr`, `entry_va`, `layout`, `start.tid`,
`time_runtime_word`), the mapping through the view, the diagnostics (refused on two vCPUs, Sprint 308), and, at each
turn's start, `served.link` and, through `threads::slice_until`, `served.threads` (`run.rs` 786-800, `threads.rs`
1099-1113), without the word; the model step touches memory and the code's addresses alone. Every other touch of
`served` is inside a service under the word: `take_word` before `hvc #13`, `#12`, `#4` and `#1` (`run.rs` 928, 1026,
1113, 1196), `host_request::serve` under the word the guest holds. Two forms were weighed: `served` alone locked, the
42 functions that take `&mut Guest` (20 in `threads.rs`) taking the Load part by `&` and `&mut Served` from the guard,
each signature changed; or the whole `Guest` behind one lock, taken once per served exit under the word, the functions
keeping `&mut Guest` from the guard. A served exit already holds the word, so the second adds no contention to the
word's and changes no signature. **Decision:** the whole `Guest` behind one `Mutex`, taken once per served exit, inside
the word (the guest-memory design's choice 6: the word outermost); each vCPU thread keeps a copy of the Load facts it
reads outside a service and its own view of the mapping. The turn's start reads `served.link` and `served.threads`, so
it takes the word and the lock for those reads, or the services that change them write the slice's end and whether
another process is due into the vCPU's monitor data and the turn's start reads memory alone; card 4c measures the first
against the second. Card 4b builds the lock: `Guest` wrapped where the run starts, the run loop holding the guard across
each served exit, its acquire asserting the word in debug builds, card 2's plant.

## The threads

- The run creates a host thread per vCPU; each creates its vCPU, runs it, serves its exits, and ends when the run
  stops. `vcpu_handle` (the host design's lock 8) holds each vCPU's id for `hv_vcpus_exit`.
- **The stop.** The first thread to stop records its `Stop` under `served` and calls `hv_vcpus_exit` on every other
  vCPU; each other thread finds its run cancelled, records nothing, and ends. The run's outcome is the first stop's.
- **A parked vCPU** (no guest thread to run) waits on the `sched` condition holding neither the word nor `served`; a
  thread readied on another vCPU wakes it, as `W2M_THREAD_WAKE` does in the guest (census D2: on a busy second vCPU,
  the wake is the kick).
- **The diagnostics** (`exit_ledger`, `block_judge`, `batch_stop`) are refused on a run of more than one vCPU, by name:
  each reads every exit in order, which two threads do not have.

## The second vCPU's preconditions

The step 6 sequence Decision's eight, the eighth from Sprint 294's close, and a ninth from Sprint 311's, each with where it sits:

1. The reconstruction count read Acquire: met (Sprint 282).
2. **FEAT_LSE2**: the run refuses a second vCPU unless `hw.optional.arm.FEAT_LSE2` reads 1, the dispatch pair's single
   copy atomicity resting on it (the guest-memory design's choice 3); in the card that creates the threads.
3. TLB maintenance on every page made invalid: the runtime's broadcast after each request that maps or unmaps
   (`runtime.h` 331-335).
4. **Break-before-make for a block split** (`page_descriptor`): a valid block made a table while another vCPU may hold
   it in its TLB needs the block made invalid, a broadcast invalidation, then the table entry. Only the guest can
   invalidate its stage-1 TLB, so the split answers its request in two steps: the block invalid and the request
   marked to retry; the runtime's broadcast; the table on the retry. Splitting every block of the arena at load
   instead would cost 1 GiB of tables (`VIRTUAL_SIZE` 0x80_0000_0000, 512 GiB, in 2 MB blocks of one 4 KB table
   each), so the split stays at run time; its card counts how often a valid block is split in the batch run.
   *(Sprint 303: a vCPU reads `ID_AA64MMFR2_EL1` = 0x1001001102001011, BBM 0, so the guest is promised no FEAT_BBM
   and a change of block size needs the break; the two steps are card 304's.)* *(Sprint 304, the form built: not
   the two steps above, whose invalid block would fault another vCPU touching any page of it, an access violation in
   the runtime's fault path; the split is made with every other vCPU stopped, as a locked model step is (precondition
   8; on one vCPU it holds already), the table written whole before its descriptor, and the requesting vCPU resumed
   through `FLUSH_STUB` (`dsb ishst; tlbi vmalle1is; dsb ish; isb; hvc #15`), which invalidates every vCPU's TLB
   before any vCPU touches memory again; at `hvc #15` the host puts it back where it stopped. No window leaves a page
   unmapped, and the C runtime is unchanged. The sleeper run splits 3 blocks and flushes 3 times. Not the Arm order on
   the requester itself: the table is written before the flush, so in the stub's first two instructions its TLB may
   hold the block while a speculative walk, which the architecture allows at any time, loads the table's entry, an
   UNPREDICTABLE pair; the full order, the block invalid, the flush, then the table, needs the request's page writes
   after the flush (`map_pages` writes into the table in the same request). BLACKBOARD Deferred (Sprint 304).)*
   *(Sprint 305, the Arm order built: the block's descriptor invalid, then the flush, the host running the requesting
   vCPU from inside the split at `FLUSH_STUB` until `hvc #15` (`run::flush_through_stub`, its PC, PSTATE and general
   registers kept and given back, a vtimer or a cancel running the stub again, the cancel reported after the split),
   then the table and its descriptor; the flush passed down to the split's callers. The gap above is closed.)*
5. Every dispatch writer stores a pair whole: met (Sprint 284).
6. The host's miss service under the word: met (Sprint 286).
7. The lock order: the word outermost (the guest-memory design's choice 6); `served` inside it, here.
8. **A model step of a form x86 locks**, run with every other vCPU stopped (above, "A model step of a locked form"),
   the stop complete before the step's load; the batch run counts such steps.
9. **Each vCPU's own pages** (found at Sprint 311's close, Observation 5). The monitor code reaches its data at one
   address, `MONITOR_VA + MONITOR_DATA` (`monitor_code.rs` 116, 811-874: the count and limit, `LOCK_OWNER`, the save
   area, the spill words), the C runtime its state block and transport page at fixed process-area offsets, and the
   runtime's C frames one stack; `compile.rs` 6-7: "One vCPU: no private tables". Two vCPUs would share them. The
   Python host gave each vCPU its own level-0 table, every lower table shared but the chain to the per-vCPU pages,
   whose leaves point at each vCPU's own physical pages, and wrote every page-table change into each private copy
   (`docs/design-several-vcpus-2026-10-17.md`, Sprint 103, "Choice: B"; built at Sprint 104); the process-wide words
   on those pages moved to a process page (Sprint 106). The litmus test (Sprint 274) runs several vCPUs on one monitor
   data page because its threads make no service, fault or switch, Sprint 105's condition. Card 4c2 decides the
   Rust host's form from today's inventory before card 4c3 runs a second thread.

## The cards

1. **The scheduling exits under the word.** `hvc #4`, `#12` and `#13` take the word as `hvc #1` does; a turn's wait
   releases it first. Judge: a count of services that changed shared state outside the word, 0 on the qpc, sleeper
   and model-steps runs and the batch run; a plant serving `#12` without the take.
2. **`Guest` divided.** The load part, `served` and the mapping; `served`'s guard takeable only with the word held,
   checked by the guard in debug builds; one vCPU still. Judge: every run as held; a plant taking `served` without
   the word, which the guard's check names. *(Split by rule 6 at Sprint 302: 2a, Sprint 302, the 22 Served fields
   gathered into one value, `Guest.served`, held to this design's table by a test; 2b, `served` a lock whose guard
   checks the word, the run loop's serving paths taking it, with this card's judge and plant.)* *(2b folded into card
   4 at Sprint 302's close, BLACKBOARD Decision "Architecture: when `served` becomes a lock": on one vCPU `&mut Guest`
   is exclusive, and a lock taken per touch would lock twice on one thread; the lock is taken once per served exit
   when the threads share `Guest`, its guard passed down.)*
3. **Break-before-make** for a block split, its count on the batch run's record. Judge: a split's three writes in
   order, by a test that records them; the batch run as held. *(Sprint 303: first reads `ID_AA64MMFR2_EL1.BBM`; at
   FEAT_BBM level 2 a change of block size alone needs no break, the runtime's broadcast invalidation following.)*
4. **The threads and the second vCPU.** `served` a lock taken once per served exit (card 2b, folded here), its guard
   checking the word in debug builds and passed down as `&mut Served`, with card 2's plant; Deferred (Sprint 298)'s
   revisit clause checked at the card's start. A thread per vCPU, the stop, the kick, the merged outcome, FEAT_LSE2's
   refusal, the diagnostics refused by name, a child process held to one vCPU, the stop of every other vCPU for a model
   step of a locked form and for a block split (precondition 4, Sprint 304: the split's flush stub assumes it), and a
   stop flag the take's spin reads, so a thread waiting for a word a cancelled vCPU's guest
   held ends at the stop (reviewer-3's Low 1 at 294). Judge: `qpc` and `sleeper` on two vCPUs ending as on one; the race
   test of the guest-memory design's choice 6, a vCPU holding the word across a host request while the other misses, a
   step bound failing a hang, with its plant, `served` held across the take; a misaligned XCHG raced against an aligned
   one through the run loop, the sum of the exchanged values conserved, with its plant, a step without the stop; a stop
   while a guest holds the word, the run ending within its bound. *(Split by rule 6 at Sprint 307's close, each its
   own card, in this order, with the judges above shared out: 4a, the refusals on one vCPU: a vCPU count, 1 by default,
   refused above 1 without FEAT_LSE2, with a diagnostic, or in a child process, each by name; 4b, the whole `Guest`
   behind one lock taken once per served exit inside the word (Sprint 309, "How the threads share `Guest`"), its
   acquire checking the word, card 2's plant, Deferred (Sprint 298)'s clause checked; 4c, a
   thread per vCPU, the stop, the kick, the merged outcome and the stop flag, `qpc` and `sleeper` on two vCPUs ending as
   on one, a stop while a guest holds the word; 4d, every other vCPU stopped for a locked model step and for a block
   split, the misaligned XCHG race and its plant, a cancel from another thread mid-split (Deferred, Sprint 305); 4e, the
   race test of the guest-memory design's choice 6 and its step bound.)* *(4c split by rule 6 at Sprint 311's card:
   4c1, the run's Served state behind `GuestLock` with the guest, `HostCompiler` and the records a served exit or the
   in-VM compiler's drain writes into `Outcome` (the drain and the exit's count of `tails`, at every exit, missed by
   Sprint 309's reading), and `turn` letting the guard go, then the word, around the link's wait (Sprint 310's Observation 2,
   the inversion choice 6 forbids); 4c2, the threads inside `std::thread::scope`, so none outlives the borrows the lock
   holds (reviewer-3 at 310), the stop, the kick, the merged outcome, the stop flag, and the turn's start's reads
   measured, the link being out of `Guest` while another vCPU's turn waits.)* *(4c2 split at Sprint 312's card, for
   precondition 9: 4c2, each vCPU's own pages, a design: the per-vCPU state the Rust host reaches today, by word and
   address, the process-wide words among it, and the form, Sprint 103's B carried or replaced, with the cards it
   shapes; 4c3, the threads as 4c2 was written above.)* *(4c3 split at Sprint 316's card: 4c3a, the host's thread
   switch on any vCPU, the loaded thread's `vcpu` word the vCPU's own `LOCK_OWNER` and a thread another vCPU holds
   skipped, as the C runtime's `held_elsewhere` does (`threads.c` 65-68, 293-294), where the host's `switch` writes 1
   and stops at such a thread (`threads.rs` 859-878); 4c3b, the threads, the stop, the kick, the merged outcome and the
   stop flag, inside `std::thread::scope`.)*
5. **G2**: the 273 family runs on two vCPUs. **G4**: Sprint 122's guests ending on two vCPUs in many runs, the count
   in the card. **G3**: the batch run on the wall clock on two vCPUs, conforming in several runs.

## Against the goal

The game makes 37,600 system calls a second in play and retires 41 million instructions a frame at 112 frames a
second, over six threads (the host design, "Against the goal"); its first scene is 325.4 billion instructions in, at
62 s on the laptop. One vCPU cannot run six threads at once: Sprint 240 measured 2.87 to 2.93 billion instructions a
second on one, against the game's 4.6 billion. This design adds the second vCPU without changing the locking the runs
were judged with, so a difference between one vCPU and two is the concurrency's, not a new lock's. Its cost is one
uncontended host mutex a service, 4.9 ns on this Mac (the host design), against a 0.72 µs exit; at 37,600 services a
second over six threads, 0.18 ms of each second. The runtime word stays the bound on how many services run at once:
G6 measures the hot imports' rate on six threads against one under it, and the host design's change 1 takes it apart
after G4.

## Against the systems that solved it

- **crosvm** runs one thread per vCPU (`src/crosvm/sys/linux/vcpu.rs`) and locks each device, not the machine: "Each
  `BusDevice` is reference counted and wrapped in a mutex" (https://github.com/google/crosvm/blob/main/ARCHITECTURE.md).
  `served` is the one-device form of this, split into the host design's table after G4.
- **Firecracker** serves an exit on the vCPU's own thread: "one or more vCPU threads (one per guest CPU core)", which
  "execute synchronous I/O and memory-mapped I/O operations on devices models"
  (https://github.com/firecracker-microvm/firecracker/blob/main/docs/design.md). The threads above are this.
- **QEMU** ran every vCPU under one big lock before its multi-threaded TCG, and took the lock apart a part at a time
  after: "each vCPU has its own TCG context ... thereby requiring no locking during translation"
  (https://raw.githubusercontent.com/qemu/qemu/master/docs/devel/multi-thread-tcg.rst). One lock first, judged, then
  split, is this design's order.
- **Wine** kept one server serializing every object until esync and fsync moved waits out of it
  (https://github.com/zfigura/wine/blob/esync/README.esync); a wait that holds the lock others need is what this
  design's "a wait never holds the word or `served`" forbids.
- **Rosetta 2** translates x86 on arm64 and runs the translated threads natively, each its own thread
  (WWDC20 session 10686, https://developer.apple.com/videos/play/wwdc2020/10686/); **FEX** runs each guest thread on
  a host thread with its JIT's code shared (https://fex-emu.com/FEX-2609/). Both are why the guest's six threads need
  several vCPUs at all.

## Revisit if

- A service is found that changes shared host state from an exit not in the table above: it joins the table and takes
  the word before its card lands.
- The batch run splits a valid block more than once a second: the arena's reservations are split at reservation
  instead, before any block is valid.
- G4's guests do not end on two vCPUs in its card's count of runs with `served` uncontended: the cause is measured
  before any lock is split.
- The batch run makes more than one model step of a locked form a second: stopping every vCPU for each costs more
  than a lock of its own, and the misaligned locked forms get one, taken by the model step and by the VM's exclusive
  pair for the same address range.
- A host request other than KERNEL 33 is found to wait: it joins "The host requests that can wait", and its process
  keeps one vCPU until it does not.
