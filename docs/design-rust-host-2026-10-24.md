# The Rust host, redesigned: no lock across a service (Sprint 130)

Step 1 of `docs/rust-port-plan-2026-10-24.md`, fix (b) of BLACKBOARD Decision 2026-10-24 user. It replaces
`docs/design-rust-host-2026-10-23.md` (Sprint 124), history from today. That design promised what the plan asked, a
thread per vCPU and a lock per shared part, then kept the Python host's big lock in a new place: the guest's runtime
word, held "around a service" for "one runtime call or service", first in its lock order, until a step the plan had
moved (its lines 29, 31, 68, 82). It sized itself from a test guest's poll loop, not from the game. This document
keeps what that one measured and decided, changes five things, and states each against the goal. Its rule: **no
thread waits, on any service path, for a lock held by another thread's service**, and every lock is held for one
access.

## What carries over

From the design of 2026-10-23, unchanged: a `w2m-vm` process per guest process, owning its VM; one native thread per
vCPU, which creates the vCPU, runs it and serves its exits (hv_vcpu.h); guest memory as anonymous memory of that
process, so no VM outlives its process and no backing file is left (its requirement 2); per-vCPU counters with no
lock; set-once values as atomics; FAIRSHARE for each host mutex two vCPU threads take on their exit paths, with its
measurement (first-fit starved an occasional taker for a median 7.5 to 13.4 ms beside a thread holding 10 µs of
11 µs; FAIRSHARE kept the median at 5.1 to 6.4 µs); no lock across `hv_vcpu_run`; the six faults found in the Python
host, each made impossible by a type; and the layers cpu, memory, nt, host, diagnostics, supervisor. Its table of the
Python host's 202 attributes stays the record of where each piece of old state went, amended by the rows below; it is
no longer the acceptance. The acceptance is the goal's figures.

## Against the goal

**The load.** In play the laptop's game makes 37,600 system calls a second (314 NT and 18 win32k a frame at 113
frames a second; `docs/plan-2026-10-09.md` 23-24) and retires 41 million instructions a frame at 112 frames a
second, about 4.6 billion a second, over six threads, the busiest about 1.33 billion a second
(`docs/recentering-2026-10-14.md` 35-36, 46). The first scene is 325.4 billion instructions in, at 62 s on the
laptop. A frame lasts 8.9 ms.

**Two budgets.** The game reaches w2m two ways, and the design is held to both.

- *Calls answered inside the VM.* w2m's runtime answers the hot kernel32 calls in the guest. In the Windows runner's
  180 s batch run the game called GetCurrentThreadId 27,773,497 times, TlsGetValue 23,291,261 and
  QueryPerformanceCounter 16,912,633: about 154,000, 129,000 and 94,000 a second; Mono called HeapAlloc 987,170
  times (BLACKBOARD Decision 2026-10-09, "Hot imports stay in the guest"; the feasibility review's F5). Today every
  one of them takes the runtime word for the whole call (`vm_backend.py` 3762-3765): with six threads at once, about
  377,000 takings a second of one word by the first three alone. That is the lock this design must take apart.
- *Crossings to the host.* If every system call left the VM, six threads would make 37,600 exits a second, one every
  160 µs on each. If the 278 scheduler calls a frame stay inside, about 6,100 leave (derived; the plan, "Against the
  goal"). Measured times on this Mac: an exit and back, 0.72 µs (`scripts/measure_exit.py`, 2026-10-14); an
  uncontended mutex, 4.9 ns; a request to another process and its answer, a median of 0.58 to 0.71 µs (the design of
  2026-10-23). At 2 µs a serve, 37,600 exits use 75 ms of each second, about 1.3% of six cores. The time is not the
  risk; a lock one serve holds while another waits is.

**Instructions.** 4.6 billion a second over six threads asks several vCPUs translating and running at once. The
Python host's one vCPU ran 2,095,145,119 a second on the batch run (Sprint 108's card, 89). A lock across
translation makes every thread's first entry into new code wait on every other's, and the game enters new code
throughout its first 325.4 billion instructions. Change 3 takes translation off the shared path, and the plan's step
5 takes most of it off the running path.

**What judges it.** Each figure is measured on a record at the plan's step that can run it: an exit's time, the
runtime word's hold times and whether a guest WFE exits at the walking skeleton (step 3); exits a second by kind and
instructions a second on one vCPU at the batch run (step 4); compiles at run time against the cache's share (step 5);
the hot imports' rate on six threads at once against one, every lock's hold time and waits, and duplicate compiles at
several vCPUs (step 6); system calls and frames a second in play, and the supervisor's requests a second (step 9).

## Against the systems that solved it

- **QEMU** took its translation lock out in 3.0 (2018): "In !user-mode, each vCPU has its own TCG state, so no locks
  needed. Per-page locks are used to protect the page descriptors. Per-TB locks are used in both modes to protect TB
  jumps" (Emilio G. Cota, https://github.com/qemu/qemu/commit/0ac20318ce16f4de288969b2007ef5a654176058). "Each vCPU
  has its own TCG context and associated TCG region, thereby requiring no locking during translation"; lookups use
  QHT, "designed for lockless lookups", and "QHT is used at insertion time as the synchronization point across
  threads" (https://raw.githubusercontent.com/qemu/qemu/master/docs/devel/multi-thread-tcg.rst). Change 3 is this.
- **crosvm** locks each device, not the machine: "Each `BusDevice` is reference counted and wrapped in a mutex"
  (https://github.com/google/crosvm/blob/main/ARCHITECTURE.md), one thread per vCPU (`src/crosvm/sys/linux/vcpu.rs`
  542). The host's locks per part (files, modules, the address map) are this.
- **Firecracker**: "one or more vCPU threads (one per guest CPU core)", which "execute synchronous I/O and
  memory-mapped I/O operations on devices models"
  (https://github.com/firecracker-microvm/firecracker/blob/main/docs/design.md). The exit served on its own thread
  is this.
- **Wine**: its server holds every object, and a wait is a request the server leaves unanswered "before the wait
  condition has been satisfied" (Wine Developer's Guide, Architecture Overview,
  https://gitlab.winehq.org/wine/wine/-/wikis/Wine-Developer's-Guide/Architecture-Overview). esync (2018) moved waits
  out, "without going through wineserver" (https://github.com/zfigura/wine/blob/esync/README.esync); fsync (2019) to
  futexes (https://github.com/ValveSoftware/Proton/releases/tag/proton-4.11-1b); ntsync, usable from Linux 6.14,
  because user space "cannot match Windows performance while offering accurate semantics"
  (https://docs.kernel.org/userspace-api/ntsync.html); the server had added "an interprocess-communication overhead to
  every locking operation" (https://lwn.net/Articles/961884/). Change 4 is this lesson: one process's objects and
  waits stay inside its VM.
- **Rosetta 2** translates "right from the moment your application is being installed" and compiles code it has not
  seen "on the fly" (WWDC20 session 10686, https://developer.apple.com/videos/play/wwdc2020/10686/). **FEX** caches
  its JIT's code on disk and looks it up "from that FOZ database first" (https://fex-emu.com/FEX-2609/). The PE
  images' blocks translated ahead of time, and mapped read-only, are this (plan step 5); what remains for run time is
  Mono's generated code and the blocks the cache missed.
- **Lock-holder preemption**: "If a guest kernel spins on a preempted lock, it could live out the remainder of its
  time slice spinning and accomplishing no work" (Uhlig, LeVasseur, Skoglund and Dannowski, VM '04,
  https://www.usenix.org/legacy/events/vm04/tech/full_papers/uhlig/uhlig.pdf). A vCPU thread is a macOS thread and
  is preempted like one. Change 5 is the answer: a bounded spin, then a wait the holder's release ends, the futex
  protocol's three states (Drepper, "Futexes Are Tricky", version 1.6, https://www.akkadia.org/drepper/futex.pdf).


## Change 1: a runtime call takes the lock of what it touches, or none

**Today.** Every service bridge takes the runtime word around the whole call into the runtime's C (`vm_backend.py`
3762-3765: `lock_runtime`, `blr`, `unlock_runtime`), so the word guards all of the runtime's state: the heap, the
handle table, TLS, the sync objects, the modules, the run table. One word serializes every kernel32 call the guest
answers, on every vCPU.

**The design.** Each service the runtime answers declares its lock class in the runtime's service table, and the
bridge takes that class's lock, not the word:

- **None**, for what one thread owns or what no one writes: GetCurrentThreadId, GetCurrentProcessId, TlsGetValue,
  TlsSetValue, FlsGetValue, FlsSetValue, GetLastError, SetLastError read and write the TEB or the thread's own slots;
  QueryPerformanceCounter reads the counter (`CNTVCT_EL0`). These are the three hottest calls above.
- **A lock per heap**, as Windows has one: "the heap serializes access within the calling process. Serialization
  ensures mutual exclusion when two or more threads attempt simultaneously to allocate or free blocks from the same
  heap" (Microsoft Learn, HeapCreate, https://learn.microsoft.com/en-us/windows/win32/api/heapapi/nf-heapapi-heapcreate).
  If step 6 measures contention on it, the low-fragmentation heap's buckets per size class are the known next form.
- **The TLS and FLS slots' lock** for TlsAlloc, TlsFree, FlsAlloc and FlsFree, which change the process's slot
  bitmap; only the Get and Set calls are thread-local.
- **The handle table's lock**, held to take, free or look up one slot.
- **A lock per sync object** (event, semaphore, mutant, critical section's slow path, timer), held to change its
  state and pick a waiter; readying the waiter then takes the run-table word.
- **The modules' lock** for LoadLibrary, GetProcAddress and the module list.
- **The call trace's own:** the traced wrapper (`w2m_traced`, `call_trace.c`) writes the process-wide trace area
  today under the word; its sequence number becomes an atomic increment and each thread writes its own records, since
  traced runs are the batch run's judge.
- **The run-table word** only for the run table: a thread's entry, `in_vm`, `waiting`, `count`.

Order inside the guest: modules → handle table → TLS slots → a sync object → a heap → the run-table word, which is a
leaf. No guest lock is held across `hvc #8` (change 2) or across a call into guest code.

**A wait across two locks.** A thread that blocks on an object registers as its waiter under the object's lock and
releases it, then takes the run-table word to park. A release in between must not be lost: the releaser, under the
object's lock, marks the waiter woken and readies it under the run-table word; the waiter, under the run-table word,
checks its woken mark before it parks, and does not park if it is set. Both take the object's lock before the word,
the order above.

**The run-table word's own load.** After this change the word is taken only for scheduling: a wait that parks, a
release that readies, a Sleep, a slice's end, a thread switch. In play that is at least the 278 scheduler calls a
frame, about 31,000 a second at 113 frames, plus the slice ends, from six threads. Held for a few loads and stores
(its hold times are measured at step 3; at 200 ns, 31,000 takings a second keep it busy 0.6% of the time), one word
is kept at step 6, deliberately: the C scheduler (`threads.c`) is one run table, and replacing it is part of its port
(plan step 10). Revisit if step 6 measures the word busy more than 5% of the time or a p99 wait above 10 µs: then
per-vCPU run queues with stealing, the form the recentering of 2026-10-14 named ("per-vCPU run queues", line 113)
and Go's runtime uses ("Queue of runnable goroutines. Accessed without lock", `runq` per P,
https://github.com/golang/go/blob/master/src/runtime/runtime2.go). The C runtime is changed only for
this (the bridge, the service table's class, these locks); it is ported to Rust after milestone B (plan step 10),
and any new code inside the VM before then is Rust. On one vCPU (plan steps 3 to 5) every lock is uncontended; the
split is built and judged at step 6.

*Measured 2026-10-28 (Sprint 164, `store/runs/w2m-vm-measurements-1793171817000000000`, one vCPU, today's single
word):* the switch stub's hold, the scheduling hold this paragraph prices at 200 ns, is 83 ns at p50 and at p99 over
`sleeper`'s 3,018 takings. The word's other holds are the ones changes 1 and 3 remove: a bridge across its C service
(2.1 to 4.3 µs at p50, to 57.5 µs when the service exits to the host) and the dispatcher across an in-VM compile
(1.3 to 3.4 µs at p50, to 246 µs). Every wait but each run's first is 4 ticks (167 ns) or less. The revisit
conditions stand as written; one vCPU cannot reach them.

**What it adds.** One uncontended three-state lock (change 5) a call for the classes that take one, and none for
the hottest three.

## Change 2: a host request from the guest holds no guest lock, on its own vCPU's page

**Today.** `w2m_host` (`tools/guest_runtime/runtime.c` 25) writes its request to `W2M_TRANSPORT_VA`, one page for
the whole process (`vm_backend.py` `TRANSPORT_OFF`, served at 3809), executes `hvc #8` and reads the result. Files,
modules, VirtualAlloc and its kin, the clock and random bytes go this way, from 65 call sites in 13 files (file.c 22,
sync_objects.c 8, module.c 7, processes.c 7, exceptions.c 5, virtual_memory.c 4, process.c 3, and others). The word
serializes the page's use.

**The design.**
- **A transport page per vCPU**, on the vCPU's private page beside its state block, and the host serves the exiting
  vCPU's page. Two vCPUs' requests can no longer overwrite each other.
- **No guest lock across `hvc #8`.** A service that needs both a host answer and a slot in a guest table takes the
  host's answer first and the slot after, or takes the slot, marks it pending, releases the lock, asks the host, and
  fills or frees the slot under the lock again. Each of the 65 call sites is read for state it holds across the
  request (a handle slot taken before a file open, a heap in the middle of a change, a module table during a load)
  in the sprint that splits the locks (plan step 6); at step 3 the transport page moves per vCPU and the rule is
  written into `w2m_host`'s comment.

## Change 3: translation in regions, looked up without a lock, retired safely

**Today.** The host's `code` lock (19 rows of the old table) is held for "one lookup, or one block's compile and
insert"; the in-VM compiler runs at a dispatch miss under the runtime word, with one save area (`VMC_SAVE`) and one
runtime stack for every vCPU (`vm_backend.py` 4240-4251), and bumps one code cursor; the in-guest dispatcher reads a
(guest address, host code) pair with two loads, and `block_compiler.c`'s `dispatch_insert` writes it with two
stores.

**The design.**
- **A region per vCPU.** Each vCPU compiles into its own region of the code area and owns its cursor, its compiler
  save area and its runtime stack for the miss path, all on its private page. No lock is held during a compile.
- **A lookup with no lock.** The dispatch table's slots are read and written as single 16-byte atomic pairs, or each
  slot carries a sequence word the writer bumps before and after and the reader checks (a seqlock), in both the
  in-guest dispatcher and the compiler's insert, and in the host's own lookup. A slot is filled with a
  compare-and-swap; two vCPUs that miss on one block both compile it, the first insert wins, and the other's copy
  stays unused in its own region (QEMU: "QHT is used at insertion time as the synchronization point across
  threads"). Removal writes a tombstone; a block translated again at the same address is inserted again.
- **A lock per page of guest code**, taken only to invalidate: a write to a page holding translated code (a code
  write fault, VirtualProtect, an image unmapped) tombstones that page's blocks and unlinks their chained branches
  under the page's lock. A block spanning two pages takes both pages' locks, lower address first. A chained branch is
  unlinked by a 4-byte write, followed by the architecture's sequence for code another core will run (DC CVAU, DSB
  ISH, IC IVAU, DSB ISH) before the guest page is made writable. A core executing the branch meanwhile needs no stop:
  replacing one B with another is among the cases the Arm ARM allows, where execution is "consistent with execution
  of either the instruction originally fetched or a fetch of the modified instruction" (Arm ARM, "Concurrent
  modification and execution of instructions").
- **Stale execution is x86's own rule.** A vCPU already inside a block of the changed page finishes it and stops at
  its next dispatch, which finds the tombstone. x86 asks no more of cross-modifying code: the executing processor
  must run a serializing instruction before it is sure to see the change (Intel SDM vol. 3, "Handling Self- and
  Cross-Modifying Code").
- **Retired code and old tables are freed only when no vCPU can be in them.** Each vCPU publishes an epoch, advanced
  at each dispatch; a retired block's space, or a replaced table, is reused only after every running vCPU's epoch has
  passed the epoch of its retirement (QEMU frees QHT's buckets under RCU for the same reason). A vCPU parked in the
  host counts as past every epoch.
- **The ahead-of-time cache** (plan step 5): the PE images' blocks, compiled before the run, mapped read-only and
  entered in the table at load. Nothing locks on that path.

**What it adds.** A duplicate compile when two vCPUs miss on one block at once, bounded by the vCPU count and counted
on the record (a field the step-6 sprint proposes).

## Change 4: one process's waits stay in its VM; the supervisor sees what crosses processes

**Today, in the design of 2026-10-23.** Named events, pipes and handles one process holds on another's objects live
in a supervisor process, reached by a request per operation.

**The design.**
- **Objects of one process live in its VM:** events, semaphores, mutants, timers, critical sections and the threads
  that wait on them, each under its own lock (change 1). The C runtime answers these inside the VM today
  (`sync_objects.c`, `threads.c`). A wait, a release and a wake between two threads of one process are loads, stores
  and the in-VM scheduler, with no exit, as on Windows.
- **A vCPU with no thread to run waits in its own host thread** on its condition (the `sched` lock), and a wake from
  another vCPU of the process comes as today: the in-VM readying, then W2M_THREAD_WAKE when a vCPU waits (Sprint 121),
  or `hv_vcpus_exit` to a vCPU running guest code.
- **The supervisor holds only what crosses processes:** the process table, a process's start and end, names one
  process opens in another, handles on another process's objects, pipes between processes. A request reaches it as
  in the old design (a request page per vCPU thread, `os_sync_wait_on_address`, a median 0.58 to 0.71 µs). An object
  two processes wait on lives there. The game always starts UnityCrashHandler64.exe; a process per guest process is
  what lets several vCPUs run while it exists (the Python host refused that, Sprint 120).
- **How often the game crosses** is not measured: no play run has counted it. Revisit if a play stretch sends the
  supervisor more than 376 requests a second (1% of 37,600): then cross-process objects move to shared pages both
  processes map, waited on with `os_sync_wait_on_address` (shared), ntsync's shape without its driver.

## Change 5: every lock word has three states, and a waiter stops spinning

**Today.** A vCPU that finds the runtime word held reads it in a loop until it is free (`lock_runtime`: `ldr`,
`cbnz`), with no bound, no yield and no exit. If the holder's host thread is preempted by macOS while its vCPU is in
the guest, every other vCPU entering the runtime spins for the rest of its time slice.

**The design.**
- Every lock word inside the guest (change 1's), and the host's takes of them, use Drepper's three states: 0 free,
  held with no waiters, held with waiters; the run-table word's holder's vCPU index stays in a second word, which
  `RuntimeLock` reads. Host threads follow the same protocol: a host release that stored 0 plainly would lose a wake.
- A taker spins a bounded number of reads, then marks the word "held with waiters" and exits to its host thread (a
  new hvc), which waits on the word's address with `os_sync_wait_on_address`: guest memory is the process's own
  memory, and the call compares the value in the kernel, as `futex_wait` does. A release that finds "held with
  waiters" exits to wake one waiter; a release with none stays one store.
- A guest taker that waits blocks its whole vCPU in the host, where under M:N another ready guest thread could have
  run on it. That is deliberate: the runtime's locks are held for nanoseconds, and a switch to another guest thread
  takes the run-table word itself.
- The bound is set from the hold times measured at step 3, first to the spin that covers their p99. Its judge at
  step 6 plants the case this change exists for: a holder whose vCPU thread is descheduled for 10 ms while in the
  guest; the other vCPUs must wait in the host, not spin.
- Whether Hypervisor.framework traps a guest WFE is not in Apple's documentation; QEMU's HVF backend handles a WFx
  exit (`EC_WFX_TRAP`, `target/arm/hvf/hvf.c`). Step 3 measures it. If WFE exits, the spin can end in WFE and the host
  answer with a yield; if not, the hvc above stands.

## The locks

Host, in the `w2m-vm` process:

| Lock | Guards | Held for | Order |
|---|---|---|---|
| `modules` | the loaded images, the module table, a DllMain stub due | one module's map or one lookup | 1 |
| `vm_map` | reservations, image protections, pristine pages, stacks | one reservation change and its page-table writes | 2 |
| `code_page` (one per guest page with code) | that page's blocks and chains | one invalidation | 3 |
| `files` | the descriptor table, read-only and recorded descriptors, checked copies | one lookup, insert or removal | 4 |
| `file` (one per open file) | that file's position | one read or write that uses the position | 4a |
| `output` | the guest's standard output | one append | 5 |
| `trace` | the call trace's bound images and thunks | one insert | 6 |
| `sched` | parked vCPUs, the change count, a pending wake, with its condition | one count change and notify | 7 |
| `vcpu_handle` | a vCPU's kick against its end | one flag read and `hv_vcpus_exit` | 8 |

Three of these are held across more than a load and a store, each for a stated reason:
- `files` is held only to find, add or drop a descriptor; the read or write itself runs with no table lock, at an
  explicit offset (`pread`, `pwrite`), or under that file's own lock when it moves the file's position. Six threads
  loading assets queue only on the same file, as crosvm's devices queue only on the same device.
- `modules` is held across one module's map, as Windows holds its loader lock: "DllMain is called while the
  loader-lock is held" (Microsoft Learn, Dynamic-Link Library Best Practices,
  https://learn.microsoft.com/en-us/windows/win32/dlls/dynamic-link-library-best-practices). Loads are rare after
  start; step 6 measures their waits.
- `vm_map` is held across one reservation's page-table writes, a few hundred stores at most for one VirtualAlloc;
  step 6 measures its hold times.

Guest, in the runtime (change 1): modules → handle table → TLS slots → a sync object → a heap → the run-table word (a
leaf).
No guest lock is held across `hvc #8`, so no host lock is ever taken under one.

Supervisor: `kernel`, for processes, cross-process names, handles and pipes, one request at a time.

No lock at all: the dispatch table (atomics), each vCPU's code region, cursor, save area, runtime stack and transport
page (owned), the counters (owned), the thread-local services. A thread holding a lock takes only those after it, or
the leaf. Each host lock is a Rust type reachable only through its guard, and debug builds check the order per thread.

Rows of the old table that change: `code_cursor` (runtime word → each vCPU's own); `addrs`, `blocks`, `bridges`,
`code_pages`, `pc_map`, `translations`, `tail_hosts`, `tail_left`, `flags_cache`, `lift_cache`, `lift_pages`
(`code` → the dispatch table, the page locks and each vCPU's region); `tcache`, `tcache_file`, `tcache_loaded` (→
the ahead-of-time cache, read-only after load); `_takers`, `threads`, `GuestThread.stack`, `GuestThread.exit_code`
(the run-table word, held only for their loads and stores).

## What is not decided here

- The spin bound, the dispatch table's size and hash, and the choice between 16-byte atomics and a sequence word
  per slot are set from measurements at steps 3 and 6 (whether this Mac's cores give single-copy-atomic 16-byte
  loads is read from its feature registers at step 3, not assumed).
- Microsoft's DLLs over a w2m kernel inside the VM (plan step 10) get a design of their own, through
  `design_check`.
