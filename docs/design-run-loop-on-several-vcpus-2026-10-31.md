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
| `hvc #8`, a host request | yes | `served`'s every part, by request | `served`, under the guest's hold |
| `hvc #9`, an import with no service | the bridge's | `thunks` (read), `unimplemented` | the stop's record (below) |
| `hvc #10`, `#11`, a model step | no | the vCPU's own areas, guest memory | the view; no word |
| `hvc #12`, a schedule hand-over | no, released before it | `threads`, `link` | as `hvc #4` |
| `hvc #13`, a write to translated code | not established | the host compiler's tables, `code_page` | the host takes the word (`host_take_runtime_word` returns at once if this vCPU holds it), then `served` |
| `hvc #14`, a fault given back | no | the vCPU's count | none |
| the vtimer, a cancel | no | the vCPU's own | none |

A wait never holds the word or `served`: a process's turn (`hand_over`, `turns.rs`) and a parked vCPU's wait (the
`sched` condition, census D2) release both first and take them again after, as the host design's "no thread waits ...
for a lock held by another thread's service" asks.

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

19 Load, 22 Served, 1 View, 3 Diagnostic, counted from the table. `memory` stays one mapping; `Guest` becomes three
values the run loop's threads hold: the load part, shared by reference; `served`, a host mutex over the 22; and the
mapping, from which each thread takes its view.

**The run loop's own state** (`run.rs`, `run`): `HostCompiler`'s tables are Served, reached at `hvc #1` and `#13`
under the word; the exit counts, `run_ns`, `serve_ns`, `serve_by_kind` and the other fields of `Outcome` are each
vCPU's own, merged when every thread has ended; `out.bridged`, `blocks` and `translations` are Served.

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

The step 6 sequence Decision's seven, each with where it sits:

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
5. Every dispatch writer stores a pair whole: met (Sprint 284).
6. The host's miss service under the word: met (Sprint 286).
7. The lock order: the word outermost (the guest-memory design's choice 6); `served` inside it, here.

## The cards

1. **The scheduling exits under the word.** `hvc #4`, `#12` and `#13` take the word as `hvc #1` does; a turn's wait
   releases it first. Judge: a count of services that changed shared state outside the word, 0 on the qpc, sleeper
   and model-steps runs and the batch run; a plant serving `#12` without the take.
2. **`Guest` divided.** The load part, `served` and the mapping; `served`'s guard takeable only with the word held,
   checked by the guard in debug builds; one vCPU still. Judge: every run as held; a plant taking `served` without
   the word, which the guard's check names.
3. **Break-before-make** for a block split, its count on the batch run's record. Judge: a split's three writes in
   order, by a test that records them; the batch run as held.
4. **The threads and the second vCPU.** A thread per vCPU, the stop, the kick, the merged outcome, FEAT_LSE2's
   refusal, the diagnostics refused by name. Judge: `qpc` and `sleeper` on two vCPUs ending as on one; the race test
   of the guest-memory design's choice 6, a vCPU holding the word across a host request while the other misses, a step
   bound failing a hang, with its plant, `served` held across the take.
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
