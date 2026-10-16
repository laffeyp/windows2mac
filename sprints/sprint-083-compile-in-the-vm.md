# Sprint 083 — a dispatch miss compiled inside the VM

```yaml
---
id: 083
status: closed
phase: 5
pass_kind: functional
---
```

## scope

The second link (recentering step 6; Decision 2026-10-15, the inferred compiler put in the VM). Today a dispatch miss
exits (`hvc #1`) and the host compiles the block in Python. After this sprint the dispatcher first calls the C
compiler on the runtime's stack; only what it refuses exits.

- **Layout.** The runtime image grows from 1 MB to 8 MB (the table is 2.65 MB of it) and moves above the runtime stack.
  The stack's top stays at MONITOR_VA + 0x4170000, which three helper calls' stencils hold as a constant (the table's
  pool); the old 1 MB below the stack stays unused. The image's far end stays within 128 MB of the code cache.
- **Build.** `guest_runtime` compiles `tools/vm_compiler/{vm_compiler,x86_decode,a64_encoder}.c` into the image, its
  headers part of the build's key.
- **`tools/guest_runtime/block_compiler.c`, `w2m_vm_compile(target)`.** Refuses unless the target's page is in the
  code-page set; reads the block from the run of code pages there (`vmc_block`), its `live_out` from `vmc_live_out`
  over the code-page set, writes the words at the shared code cursor, the marks to the reconstruction table, the
  dispatch pair, and one log entry (start, end, code address, marks, live flags, the pages scanned); cleans the new
  words to the point of unification and invalidates them from the instruction cache (DC CVAU, DSB ISH, IC IVAU, DSB
  ISH, ISB: the sequence Linux's `caches_clean_inval_pou` runs). Counts every refusal by cause and every cut block by
  its cut's status. The counts move last, so an abandoned compile leaves nothing.
- **Host.** Publishes the code-page set (an open-addressed table of page addresses in the monitor area) from
  `_protect_code` and `_code_written`; the code cursor becomes a monitor word; at every exit, before anything else,
  replays the log into `blocks`, `pc_map`, the tail notes, `addrs`, `translations` and the pages' owners. The miss
  stub saves x0-x15, x17 and x30, sets SP to the runtime stack, calls `w2m_vm_compile`, restores, and branches to the
  new code or falls to `hvc #1`. Off in lockstep, under BLOCK_CENSUS, with an `on_dispatch` hook, and when `VM_COMPILE`
  is false.
- **Judge (`VM_COMPILE_JUDGE`).** At replay the host lifts each VM-compiled block, assembles it with
  `assemble_block` at its address with the flags the C used, and compares words and marks with the code cache; it
  computes `_flags_live_out` too and counts the C's flags equal, wider, or short. Any differing word or short flag
  stops the run.

- **The reviewer's small review of 082** (sound; three asks): scans that fetch bytes the record never compiled
  (the judge's 0xCC fill) counted apart from `equal`; thunks in the spec layouts, as jump targets and where runs fall
  through, with a plant in the thunk lookup; the invariant behind KIT_DIARY #60 (every setter writes the flags its
  Jcc tests) as a test. LIVENESS_SCAN and the flag-liveness differential (tests/test_flag_liveness.py, the check
  that the host's scan is right, which the mirror judges cannot be) go into the contract's next version.

## prerequisites

- 082 (closed)

## context_files

- `src/w2m/vm_backend.py` (layout; `_write_monitor`; `_compile`; `_protect_code`; `_code_written`; the run loop's
  `imm == 1`; `flush_translations`), `src/w2m/guest_runtime.py`, `tools/guest_runtime/runtime.h`,
  `tools/vm_compiler/vm_compiler.h`, `scripts/gen_stencil_table.py`

## signal contract

### Emits

- MEASUREMENT_RECORDED for the VM's compiles and each refusal and cut cause, where a frontier run records its
  measurements (no new tag).

### Invariants

- Vocabulary unchanged. A block compiled in the VM has the words `assemble_block` gives for it.

## artifact contract

- `src/w2m/vm_backend.py`, `src/w2m/guest_runtime.py`, `tools/guest_runtime/block_compiler.c`, tests
- `scripts/gates.sh` exit 0; every family and class re-run on the committed tree, accepted as before; CI `success`

## observation contract

- A frontier run with `VM_COMPILE_JUDGE`: blocks compiled in the VM, 0 differing, 0 short; its stop, instruction count
  and calls as without the VM's compiles.
- The counts: VM compiles against host compiles, the host's by cause (not a code page, no stencil, guarded, other).

## done criteria

Dispatch misses compile in the VM where the table and the code-page set allow, with the host's words; the run's
outcome unchanged; the host-compile share measured.

## observed
- **Judged frontier** (`store/runs/frontier-643a6283510c-1792110029789531000` on 1db10a26, W2M_VM_COMPILE_JUDGE=1):
  stop max_steps at 1,000,000,002 instructions; 87,887 blocks, 84,046 compiled in the VM, each equal to
  `assemble_block`'s words and marks; the VM's scans 83,738 equal, 164 wider, 0 short (the rest of the 84,046 were cut
  or ended at their window, every flag live). To the host: 3,930 misses on a page not yet behind translated code, 127
  that do not decode, 46 with no stencil, 2 guarded, 1 the trace's first address; 122 blocks cut short (97 by an
  instruction that does not decode in the window, 23 by one with no stencil, 2 by a guard).
- **Unchanged outcome.** Scratch runs with W2M_VM_COMPILE=0 and =1 (same tree): the same stop, the same calls by
  service and host requests (their lines byte-equal), 76 divergences each; the guest traces differ only in the
  execution id on their first line.
- **Timing** (`frontier-643a6283510c-1792110254336698000`, no judge, load average 34 from other processes): 3.8 s,
  host misses 0.3 s, against 28.3 s and 19.4 s in misses with the compiler off. 3,841 of 87,887 blocks (4.4%) compiled
  on the host, most from its translation cache. This is the run's start, not a play stretch: Decision 2026-10-15's
  revisit measure is the next sprint's.
- **Found on the way.** The table's constant pool holds the runtime stack's top (MONITOR_VA + 0x4170000, three
  helper calls): growing the image in place would have moved it, so the image moved above the stack. The compiler's
  zeroed structures call `bzero` (runtime.c now has it and `memset`). The run's report read the counts after
  `vm.close()`. My judge first compared a cut block against the scan of its whole extent (the log now gives the scan
  only for an uncut block). Families now reach `block_compiler.c` (the host names `w2m_vm_compile`) and through it the
  compiler's files, which `guest_runtime.source_path` finds in tools/vm_compiler.
- **Review of 082.** Fill-byte scans counted apart (CI samples: 465 of 4,000, as the reviewer measured); thunks in
  the spec layouts and the plant "sees no thunk" caught; the setter invariant a test (over 1,000 setter-condition
  pairs). Not planted: the scan's PROCESS_VA check, whose removal no judge can see (no form decodes at or past
  PROCESS_VA, and the VM's code-page set never holds a process page).
- **Sweeps on 1db10a26:** 456 of 458 classes accepted (FCOS and FSIN not, as before); 68 families exit 0; 527 records
  stamped clean. Contract `w2m.vm_compiler.stencil_table` 0.83.0 (the VM judge, LIVENESS_SCAN, the liveness
  differential named).
- **Gates:** `scripts/gates.sh` exit 0, 2,444 passed, 28,448 records intact.
