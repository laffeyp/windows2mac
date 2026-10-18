# Several vCPUs: per-vCPU state, two designs (Sprint 103)

Roadmap step 1 (`docs/plan.md`): the in-VM scheduler runs guest threads on several vCPUs at once (M:N), gated by
the litmus tests MP, SB, LB, 2+2W and IRIW. Before any code, the review of 2026-10-17 asks for two designs (Ousterhout,
A Philosophy of Software Design, ch. 11). This file holds the inventory they rest on, both designs, and the choice.

## What is known first

- **One vCPU per host thread.** hv_vcpu.h: `hv_vcpu_create` "Creates a vCPU instance for the current thread"; "Each
  thread can only have one vCPU associated at a time"; the calls that read or write a vCPU "Must be called by the
  owning thread". `hv_vcpus_exit` may be called from any thread (the monitor's watchdog uses it today). So N vCPUs are N
  monitor threads, each owning one vCPU and serving its own channel.
- **The GIC first, the topology fixed once vCPUs run** (hv_gic.h; Sprint 097). The pool of vCPUs is created whole,
  each with its MPIDR_EL1 affinity, before the first runs.
- **Invalidation reaches one processor unless broadcast.** TLBI VMALLE1 invalidates on the executing PE; TLBI
  VMALLE1IS "applies to all PEs in the same Inner Shareable shareability domain" (Arm A-profile system register
  description, AArch64-tlbi-vmalle1is). IC IALLU and IC IALLUIS likewise. Today's dispatcher (vm_backend.py, the
  `hvc #1` path) and the code-write path (`_write_exception_entry`) use the local forms; with several vCPUs a page
  dropped on one vCPU must be dropped on all, so both become the IS forms.
- **Translation entries may differ between PEs only with CnP = 0.** TTBR0_EL1.CnP = 0: "The translation table
  entries pointed to by TTBR0_EL1, for the current translation regime and ASID, are permitted to differ from
  corresponding entries for TTBR0_EL1 for other PEs in the Inner Shareable domain"; with CnP = 1 and tables that
  differ, "the results of translations are CONSTRAINED UNPREDICTABLE" (Arm, TTBR0_EL1). w2m writes TTBR0_EL1 as the
  table's base address, bit 0 clear: CnP is 0.
- **x86-TSO allows only SB's relaxed outcome** among MP, SB, LB, 2+2W and IRIW (Owens, Sarkar and Sewell, "A Better
  x86 Memory Model: x86-TSO", TPHOLs 2009); the guest runs with ACTLR_EL1.EnTSO set, whose effect on MP Sprint 009
  observed on this Mac (probes/mac_hv_litmus).

## The inventory: state a running guest thread uses, by owner

Measured in the tree of 2026-10-17 (ca5cb64b).

| Where | What | Owner under M:N |
|---|---|---|
| monitor data page (x28) | lazy AF operands, PF_RESULT, DF, MXCSR, FCW, YMM high halves, the spill words | the running thread (saved and restored at a switch) |
| monitor data page | SAVE_AREA, SAVE_VECTORS, CODE_WRITE_SPILL, NATIVE_OPERANDS, SLICE_UNTIL, RUN_END, the count and limit words | the vCPU |
| monitor data page | CLOCK_MODE, CLOCK_RATE, CLOCK_WARP, CLOCK_EPOCH, RECON_COUNT, START_MODE, FOREGROUND_MODE | the process (one copy) |
| state block (PROCESS_VA + STATE_OFF) | GPRs at a call or fault, rflags, x87 status and registers, redirect_rip | the running thread, so the vCPU |
| runtime stack (MONITOR_VA + RUNTIME_STACK) | the runtime's C frames | the vCPU |
| run table (W2M_RUN_TABLE) | threads, states, deadlines | the process, shared, needing atomics |

How code reaches the vCPU's and thread's state: 28 loads and stores relative to x28 in vm_backend.py; absolute
addresses 8 times to the monitor data page, 9 to the state block, 9 to the runtime stack in vm_backend.py; in the C
runtime `W2M_MONITOR_DATA_VA` 12 times, `W2M_STATE_VA` 9, the save and exception areas 3, the run table 2 (shared under either design).

## Design A: each vCPU's state at its own address

Each vCPU gets its own monitor data page, state block and runtime stack at distinct addresses; x28 points at its page;
the 50 absolute references become relative (to x28 in generated code, to a per-vCPU base in TPIDR_EL1 for the C
runtime, as Linux arm64 keeps its per-CPU offset). Every lowering, stub and C function that names one of these
addresses changes, and each future one must know the rule.

## Design B: the same address, a different page on each vCPU

Each vCPU gets its own level-0 table (its own TTBR0_EL1), which shares every lower table with the others except the
chain down to the pages that hold vCPU state; there each vCPU's leaf entries point at its own physical pages. The
monitor data page, state block and runtime stack keep their addresses; no generated code, stub or C function changes.
The host writes a page-table change once into the shared tables and into each private copy of the few tables on the
vCPU-state chain. Requires CnP = 0 (held).

## Choice: B

- **Change:** B touches the page-table writer and the monitor; A rewrites 50 absolute references (26 in vm_backend.py, 24 in the C runtime; the 28 x28-relative ones follow x28) and binds every later lowering to a
  per-vCPU rule (change amplification, the review's §1).
- **What must still be shared is visible in one table:** the process-wide words (the clock, RECON_COUNT) move off the
  per-vCPU page to a process page, read and written with atomics; the run table takes LSE atomics. Both are needed in
  A too.
- **Risk B carries:** TLB entries for the private chain must never be shared (CnP = 0, checked by a test reading
  TTBR0_EL1 on each vCPU); a host page-table edit must reach every private copy (a test maps a page on vCPU 1 after
  vCPU 0 touched the region).
- Revisit if: a lowering needs another vCPU's state by address (B hides it; A exposes it).

## The order of the work

1. **Sprint 104:** the monitor creates N vCPUs, one thread each, each with its own channel; each vCPU's private
   translation chain; the IS invalidations. Judged by raw AArch64 on four vCPUs reading four values at one address.
2. **Sprint 105:** the litmus programs (MP, SB, LB, 2+2W, IRIW) as x86 guest code compiled by w2m, one thread a
   vCPU, no scheduler; judged against x86-TSO's allowed outcomes. Plant: EnTSO off on one vCPU, MP's forbidden outcome
   must appear (as Sprint 009 saw).
3. **Sprint 106:** the in-VM scheduler across vCPUs: the run table under LSE atomics, a ready thread taken by any idle
   vCPU, a wake to an idle vCPU by an SGI through the GIC, the TEB per thread (GS base), the process-wide words on a
   process page, SLICE_UNTIL set per vCPU (Sprint 105 found a PAUSE yields on a private page copied with 0); the
   compiler inside the VM made safe for two vCPUs missing at once (its register save area `VMC_SAVE`, its cursor and
   its log are one shared page at `MONITOR_VA + VM_COMPILER`); the 7 threading families and the batch run as judges.

## The runtime under several vCPUs (before Sprint 106)

Sprints 104 and 105 ran guest code on several vCPUs with no service, no fault and no switch on them. The guest
runtime was written for one vCPU, and so was the host:

- 28 of the runtime's 52 C files keep state in file-scope variables (the heap's lists, the objects and their waiters
  in sync_objects.c, the run table in threads.c, the registry, files, modules, windows), and nothing locks them.
- threads.c's run table has one `current`, one `current_entry`, one `slice_count`; a thread is READY, BLOCKED or
  ENDED, with no state that says another vCPU is running it.
- The host serves exits in one generator, `VM.steps`, for vCPU 0's channel.

**Design A: one runtime lock first.** A spinlock (LSE compare-and-swap) taken at every entry from guest code into
the runtime (the service bridges, the exception entry, the switch stub, the compiler's miss path) and released at
its return; guest code runs on every vCPU at once, the runtime on one at a time. The run table gains RUNNING and
a current thread per vCPU (on its private monitor page). The host serves each vCPU's channel on its own thread, the
handlers under one Python lock. The practice it follows: Linux's SMP kernel ran under one big kernel lock before
its locking was split up, and FreeBSD's under its Giant lock.

**Design B: a lock per object from the start.** Every one of the 28 files locks its own state.

**Choice: A,** then B where a measure shows the lock held: B touches every service before any measure says which
ones matter, and the measure that decides (guest seconds per wall second on the batch run, step 1's) can count the
lock's wait. Revisit if: a run spends a measured share of its vCPUs' time waiting on the lock.

**The sprints:** 106, the runtime lock and RUNNING in the run table, judged by two guest threads calling services
at once on two vCPUs (a family's threads under the litmus harness) with a plant (the lock removed: the heap's lists
corrupt); 107, the host serving every vCPU's channel; 108, an idle vCPU parked until a wake (an SGI through the
GIC); 109, the batch run on two to four vCPUs, judged as before and measured.

## The host for several vCPUs, designed twice (Sprint 113)

Sprint 107's first attempt (branch `wip/sprint-107-host`, 2d254cd4) ran its first judge into a hang and failed the
adherence check of 2026-10-17 (`docs/review-2026-10-17-sdd-hardening.md` §3): a second run loop beside `steps` with
about 100 lines of its exit handling repeated, a `VM.serving` attribute read at about 70 sites and right only under
the lock, the lock's protocol spread over six places each with a one-vCPU case, and a 50 ms cut of every run so a host
thread could see a stop. What both designs below share, from that attempt and the code at 4d09f7e7:

- **A `Vcpu` object, passed.** Each vCPU's channel, the file offsets of its private monitor page and state block,
  its pending vector swap, the thread it runs and whether its host thread holds the runtime lock. Each handler takes
  the `Vcpu` it serves as an argument (`_schedule(vcpu, x, schedule)`, `_host_request(vcpu, x)`, ...); `VM.serving`
  does not exist. The one-vCPU run is a `Vcpu` too, index 0.
- **One exit handler.** `serve_exit(vcpu, exit) -> Resume(pc, x) | Idle | Stop(reason)`, the body of today's `steps`
  loop; `steps` (one vCPU, with the oracle's hooks and a kernel.Machine's yields) and each host thread of several
  vCPUs both call it. A new hvc is written once.
- **A kick, not a cut.** A word per vCPU on its channel page; the host writes it and wakes the monitor, whose thread
  for that vCPU calls `hv_vcpus_exit`. The SDK's hv_vcpu.h: hv_vcpus_exit takes a set of vCPUs and, unlike the
  functions beside it, says nothing of an owning thread (the watchdog already calls it from its own thread); "If a
  vcpu is not running, the next time hv_vcpu_run is called ... it will return immediately", so a kick between runs
  is not lost. A stop and a preemption (SuspendThread on a thread another vCPU runs) are kicks.

**Design A: one lock, held by whoever serves.** The guest runtime lock (Sprint 106) is the only lock. A `RuntimeLock`
object owns the word and the C helper (`tools/runtime_lock/runtime_lock.c`, through ctypes CDLL, which releases the
GIL); `with lock.serving(vcpu):` takes it with the vCPU's value unless the vCPU holds it already (an exit made inside
the runtime's C), gives it back on leaving unless the vCPU resumes inside that C, and is a no-op object on one vCPU.
Each vCPU's host thread runs its channel and calls `serve_exit` inside the lock. Host state (threads, blocks, the
dispatch table, the transport page) is touched only there.

**Design B: a process server.** One host thread owns the VM's Python state and serves requests; each vCPU's host
thread runs its channel and sends each exit to the server through a queue, waiting for its answer (the shape of
Wine's wineserver, which owns the objects while each process's threads ask it). The server still takes the guest
runtime lock before it touches what the runtime's C reads (the run table, the reconstruction table), because guest C
on another vCPU reads them without the host.

**Choice: A.** Both serialise the handlers; B adds a queue crossing to every exit (two thread hand-offs) and still
needs A's lock against guest C, so it is A plus a queue. A's lock is already the serialisation point the guest uses,
and its one-vCPU case is a no-op object, not a branch at each site. Revisit if: a measure shows host threads waiting
on the lock for a share of the run worth a second design (the lock's wait is countable on the record, vocabulary 0.9's
`run_seconds`, a variant per wait).

**The chain, one concept each:** (1) `serve_exit` extracted from `steps`, one vCPU, no change in behaviour (the batch
run conforms and its numbers on the record are unchanged in kind); (2) the `Vcpu` object in place of `VM.serving`'s
role, one vCPU; (3) `RuntimeLock`; (4) the kick (vm_monitor.c, one thread per vCPU on its kick word); (5) the RUNNING
mark and the per-vCPU host threads, judged by the threads family on two vCPUs, its record naming the vCPU count;
(6) a thread suspended on another vCPU. Sprint 107's first part (4d09f7e7) stands under all of them.

**Step 5 as built (Sprint 118).** The RUNNING mark is a run entry's word (0x138), the index + 1 of the vCPU that has
the thread loaded. vCPUs past the first park when they have no thread; vCPU 0 keeps deadlines, the Machine's idle and
the deadlock check. Judged by the wait_handoff family on two vCPUs, not the threads family: its probe suspends a
spinning thread, which on two vCPUs another vCPU runs, so the threads family judges step 6. The runtime lock is taken
by compare-and-swap, guest and host: an exchange left the last waiter's value in the word, which the host reads as
the holder.
