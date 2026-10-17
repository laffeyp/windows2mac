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
