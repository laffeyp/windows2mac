# Sprint 346 — A new block's code is visible to every vCPU before its entry is published

```yaml
---
id: 346
status: done
phase: 6
pass_kind: functional
---
```

## scope

Sprint 345 closed partial at a fault older than its card (BLACKBOARD Deferred (Sprint 345, partial) (a); the card's
result, its diagnosis paragraph). On two vCPUs, `probes/thread_slots_race` stops at "guest runtime fault: ELR
0x7f00000680xx ESR 0x2000000 (EC 0x0)":
- 6 of 21 runs on the tree before card 345, 6 of 25 runs of its test after it, 0 of 42 runs on one vCPU;
- every fault lands on the first word of the newest host-compiled block, with no in-VM compile logged
  (`VMC_LOG_COUNT` 0);
- the block is the return from WaitForSingleObject that the 16 FLS threads reach once the main thread's SetEvent
  wakes them all;
- that word reads `f1000af7`, a valid `subs`, at the stop and 50 ms later, while the CPU took it as undecodable;
- the other vCPU was in the dispatcher's wait for the runtime word.

One reading of the code shows a window. It is a window to test, not a cause:
1. The host writes a block's words with plain stores. A grep of `crates/w2m_vm/src` for `sys_icache_invalidate`, IC
   IVAU and DC CVAU finds no maintenance in the host's own code.
2. `run::dispatch_insert` (`run.rs` 470-490) publishes the block's table entry by a release store, and the
   dispatcher reads the table by one `LDP` with no lock.
3. Only then does the requesting vCPU resume at `miss_va + 4` (`run.rs` 1976) and run `ic ialluis; tlbi vmalle1is;
   dsb ish; isb` (`monitor_code.rs` 816-824).

So a second vCPU can branch to the new block before any instruction-cache invalidation has run. 0x00000000, the word
the code cache held before the block was written, is permanently undefined on AArch64. An instruction fetch of a
stale line would fault this way, with this syndrome.

- **The rule, looked up first.** Quote the Arm Architecture Reference Manual (DDI 0487) on instruction-cache coherency
  and on concurrent modification and execution of instructions: what the writing agent must do, and what a PE that
  executes the new words must see or do, such as a context synchronization event. Also quote Apple's documentation of
  `sys_icache_invalidate` (`libkern/OSCacheControl.h`) on whether a host process's call reaches a guest's instruction
  fetch through another mapping of the same memory. Use the official texts, not forum posts. Where a text does not
  settle a point, say so.
- **The test of the reading, before any fix.** Run experiments whose outcome would refute it:
  - widen the window, for instance a delay between the publish and the host's return, on a temporary copy restored by
    `cp` and checked by `cmp`, and count the faults over the same number of runs;
  - narrow it by the order the rule gives.

  Quote each count. If the counts do not move as the reading predicts, stop, report, and claim no cause.
- **The fix, by the rule.** The block's words reach instruction fetch on every vCPU before its entry can be read:
  maintenance done before the publish, ordered before it, and whatever the executing vCPU needs, as the quoted rule
  gives. Keep the one-vCPU path's exits and the frozen host's code addresses unchanged, or state why they move.
- **The class.** List every path that writes code into the code cache and publishes it:
  - the host's compile (`run.rs` near 1940-1976);
  - the in-VM compiler's install (`monitor_code.rs` 640-700, whose `dc cvau`/`ic ivau`/`dsb ish`/`isb` runs on the
    installing vCPU);
  - a block split and its flush stub (`lib.rs` 70-75, `FLUSH_STUB`);
  - a written page's block compiled again (Sprint 250);
  - any other path found.

  For each, quote where its maintenance runs and where its entry is published, and fix each one that publishes
  first.
- **The judge.** Card 345's test, `run_loop::two_threads_taking_and_freeing_tls_and_fls_slots_at_once_never_share_one_on_two_vcpus`,
  loses its `#[ignore]`.
  - Run the probe on two vCPUs at least 100 times with no fault, and quote the count.
  - Answer Deferred (Sprint 345, partial) (c) by measurement: quote `tls.overlap` over those runs. If any run reads
    under the 1,000 floor, either ground a floor that holds or say why the loops did not overlap.

## prerequisites

- 345 (closed-partial)

## context_files

- `sprints/sprint-345-the-thread-slots-take-their-own-lock.md`: its amendment and its result's fault paragraph.
- BLACKBOARD's Deferred (Sprint 345, partial): `grep -n 'Sprint 345, partial' BLACKBOARD.md`.
- `crates/w2m_vm/src/run.rs`: `dispatch_insert` (470-490), the host compile path ending at line 1976, and
  `known_block`.
- `crates/w2m_vm/src/monitor_code.rs`: the dispatcher and its miss stub (800-830), and the in-VM install (600-700).
- `crates/w2m_vm/src/lib.rs` 60-80 (`DISPATCHER`, `FLUSH_STUB`).
- `crates/w2m_vm/tests/run_loop.rs`: card 345's test and `program_through_w2m_vm`.
- `WORKING_AGREEMENT.md` lines 137-266, "Build and verification commands".

## signal contract

### Emits

- None; the order of a block's maintenance and its publish.

### Invariants

- On one vCPU every run, the batch run, the exit ledger and the family runs pass unchanged.
- No block's entry is readable before its words are visible to instruction fetch on every vCPU.

## artifact contract

### Files modified

- `crates/w2m_vm/src/run.rs` or `crates/w2m_vm/src/monitor_code.rs`, as the fix needs; `crates/w2m_vm/tests/run_loop.rs`
  (the `#[ignore]` taken off, and the floor if (c) moves it).

### Command exit codes

- `scripts/gates.sh` returns 0
- `target/release/signal_report --check` returns 0 on the tree

## observation contract

- **Expected values:**
  - The rule quoted with its document and section.
  - The experiments' fault counts, before the fix and after.
  - The class's paths, each with its maintenance and its publish quoted.
  - The probe on two vCPUs at least 100 times with no fault, `tls.overlap` over those runs.
- **Planted faults:**
  - (1) The fix taken out, the publish back before the maintenance. Card 345's test fails on two vCPUs on the fault
    within the runs the builder names, the count quoted.
  - (2) In a second path of the class that the fix changes, its maintenance removed. Its judge fails. If no test
    reaches that path, say so, and name the judge it needs.

  Before the fix, read each judge's reach: count the faults over the same runs with the fault present.

## done criteria

On two vCPUs, no vCPU runs a new block's words before they are visible to its instruction fetch. Card 345's test is in
the gates, and the probe runs at least 100 times on two vCPUs with no fault.

## self-review checklist

- [x] Scope names one verifiable property: a block's code visible to every vCPU before its entry is readable.
- [x] context_files cite the fault's diagnosis, the publish, the maintenance, the class's paths and the judge.
- [x] Emits: none.
- [x] Artifact contract gradable by command.
- [x] Observation contract: the rule, the experiments, the class, the judge; two numbered plants, their reach read.
- [x] Sweet spot: the publish's order, its class, one test let into the gates.

## amendment (2026-11-25)

Reviewer-4's sixteenth review, Low 1, holds. The faulting vCPU's peer stayed in guest code from before the publish: it
took no exception entry or return and ran no ISB, then read the entry by one `LDP` and branched. So the rule's
reading side must be settled as well as the writer's:
- Quote what the Arm ARM asks of a PE that executes newly written words after another agent's invalidation, and
  whether a context synchronization event is needed on that PE.
- Read `CTR_EL0` in a vCPU on this machine and quote its DIC (bit 29) and IDC (bit 28): whether this core needs the
  data-cache clean and the instruction-cache invalidation at all for data-to-instruction coherence.
- Count two variants over the same runs, each on a temporary copy: (a) the writer's maintenance moved before the
  publish, the dispatcher unchanged; (b) an ISB on the dispatcher's path after it reads a new entry or takes the word,
  the writer unchanged.
- The fix takes what the quoted rule requires of each side, and the plants cover each side that changes.

## result

- **The rule, Arm ARM DDI 0487 M.d** (the PDF of `documentation-service.arm.com/documentation/ddi0487/latest`, read by
  `pdftotext`). B2.2.5, "Concurrent modification and execution of instructions (CMODX)": "1. No PE must be executing
  an instruction when another PE is modifying that instruction. 2. To ensure that the modified instructions are
  observable, a PE that is writing the instructions must issue the following sequence ...: `STR Wt, [Xn]`; `DC CVAU,
  Xn`; `DSB ISH`; `IC IVAU, Xn`; `DSB ISH`", its note "In a multiprocessor system, the DC CVAU and IC IVAU are
  broadcast to all PEs within the Inner Shareable domain of the PE running this sequence"; "3. When the modified
  instructions are observable, each PE that is executing the modified instructions must execute an ISB or perform a
  context synchronization event to ensure execution of the modified instructions". B2.7.4.2: "The PE might have
  fetched the instructions from memory at any time since the last Context Synchronization event on that PE", and for
  "CTR_EL0.{DIC, IDC} == {0, 1} The write is complete for the shareability domain. Subsequently the location has been
  invalidated to the Point of unification (PoU) from the instruction cache, and that invalidation is complete for the
  shareability domain." K11.5.2.2: "An ISB is not broadcast, and so does not affect other PEs. This means that any
  other PE must perform its own ISB synchronization after it knows that the update is visible", its example's P2
  "`WAIT ([X2] == 1)`; `ISB`; `BR X1`". Apple's `libkern/OSCacheControl.h` (the SDK's copy) says of
  `sys_icache_invalidate` only "Prepare memory for execution. This should be called after writing machine
  instructions to memory, before executing them. It syncs the dcache and icache."; nothing on another mapping of the
  memory or a guest's fetch, and the Hypervisor headers say nothing of cache maintenance, so the texts do not settle
  whether the host's own maintenance reaches a vCPU; the fix runs the maintenance in a vCPU.
- **CTR_EL0 in a vCPU** (a temporary test in `flush_stub.rs`, `mrs x0, ctr_el0; hvc #0x30`, restored by `cp`,
  checked by `cmp`): 0x99444c004, DIC 0, IDC 1, L1Ip 0b11 (PIPT), 64-byte instruction and data lines. So on this core
  the data-cache clean is not needed and the instruction-cache invalidation is.
- **The experiments**, `probes/thread_slots_race`'s stored binary through the dev `w2m-vm` on two vCPUs, 100 runs
  each, a fault the stop "guest runtime fault: ... ESR 0x2000000 (EC 0x0)": HEAD 30. A 500 µs sleep after the host let
  the runtime word go, after the publish and before the requester's resume (`run.rs` 1972 then): 0, against the
  reading, which predicted more. The same sleep before the publish, the words written, the word held (1968): 39.
  HEAD with a temporary record of each hvc 1's vCPU: 32, and in all 32 the vCPU that faulted is not the vCPU whose
  hvc 1 compiled x86 0x140001c91, published 20 to 33 µs before the fault's report. (a) The writer's code sync before
  the publish, the dispatcher unchanged: 0. (b) An ISB before each hit's `br x25`, the writer unchanged: 48. Both: 0.
  The count moves with the writer's order, not with the reading side's ISB alone. No cause is claimed for the
  delay-after-publish's 0 of 100.
- **The class.** Every path that writes A64 words into the code cache, at the one cursor (`VMC_CURSOR`):
  - the host's compile and its install from the translation cache (`compile.rs` 169, `place`'s `mem.write(cursor,
    &bytes)`), a written page's block compiled again (Sprint 250: `code_written` drops it, the next miss compiles it
    at the cursor), and a bridge made at run time (`guest.rs` 321, `CodeCache::put`, GetProcAddress's `make_thunk`):
    each published only by the hvc 1 path's `dispatch_insert` (`run.rs` 2035, a release store of the pair). It
    published first: its maintenance was the requester's `ic ialluis; tlbi vmalle1is; dsb ish; isb` after its resume at
    `miss_va + 4` (`monitor_code.rs` 834-838). Fixed: on several vCPUs `run.rs` 2022-2034 runs the requester through
    `CODE_SYNC_STUB` over [the synced mark, the cursor) before `dispatch_insert`.
  - the in-VM installer (`monitor_code.rs` 675-701, `dc cvau` ... `dsb ish`, `ic ivau` ... "`a64::DSB_ISH,
    a64::ISB`") before its `stp` of the pair (756, 759), and the C compiler (`block_compiler.c` 298 `sync_code` before
    322 `dispatch_insert`): maintenance first already; unchanged.
  - a block split's flush stub (`lib.rs` 75, `dsb ishst; tlbi vmalle1is; dsb ish; isb; hvc #15`) writes stage-1
    tables, not code; its TLB maintenance runs before the table is written (`guest.rs` 1180-1182). The code-write fault's
    `tlbi vmalle1is; dsb ish; isb` (`monitor_code.rs` 867) is the same. Neither publishes code; unchanged.
  - `drop_block` (`compile.rs` 438-439) unpublishes by two plain stores, the target then the code, never naming new
    code; the case runner's `dispatch_insert` (`cases.rs` 513) runs on one vCPU before it starts. Unchanged.
- **The fix.** Writer: `monitor_code::code_sync_stub` at `CODE_SYNC_STUB` (0x2030, `lib.rs`), B2.2.5's sequence by
  CTR_EL0's line sizes over [x0, x1), then `dsb ish; isb; hvc #16` (`HVC_CODE_SYNCED`); `run::sync_code_through_stub`
  runs the requesting vCPU through it, as the flush does (`run_through_stub`, the flush's loop made common), its
  `dsb ish` completing the invalidation on every vCPU before the host's publish; `Records::code_synced` keeps the
  mark, so blocks, installs and bridges below it are not synced twice; each sync counted under `hvc 16` in the exits.
  Reader: `Options::isb_before_entry`, set when the start names more than one vCPU, an `isb` before each hit's `br x25`
  (B2.2.5 item 3, K11.5.2.2), kept though (a) read 0 without it, since the rule asks it of the executing PE. On one
  vCPU neither runs: the dispatcher's words and miss stub address, the exits and the frozen host's code addresses are
  as before; the stub's 24 words are written at 0x2030 on every run, in the free area no run reads.
- **The judge.** `run_loop::two_threads_taking_and_freeing_tls_and_fls_slots_at_once_never_share_one_on_two_vcpus`
  off `#[ignore]`: 100 runs by `cargo nextest`, 100 passed. `tls.overlap` on two vCPUs 19,581 to 19,894 in those 200
  counts, and 18,726 to 19,900 in the fix's 100 probe runs; on one vCPU 191 in each. No run under the 1,000 floor, which
  stays; the doc comment gives the counts. Two tests added: `the_isb_before_entry_is_off_by_default_and_before_each_hits_branch`
  and `the_code_sync_stub_completes_the_clean_and_the_invalidation_before_its_exit` (`monitor_code.rs`), and
  `flush_stub::the_code_sync_gives_back_every_register_cpsr_and_the_pc`.
- **Plant 1** "assertion `left == right` failed: thread_slots_race on 2 vCPUs; its rows {"s":"tls.wait","ret":0} ...
  left: "HostFailure op 5: guest runtime fault: ELR 0x7f0000068090 ESR 0x2000000 (EC 0x0), not in translated code or
  not a data abort"": the writer's code sync taken out (`run.rs` 2022 `if false && place.vcpus > 1`), the dispatcher's
  ISB left in; 20 runs of the test, 8 failed on the fault (ELR 0x...68014, 0x...805c twice, 0x...802c, 0x...8090
  twice, 0x...8014, 0x...8120). Read first: a plant that moved the publish before a sync still run before the
  requester's resume failed none of 20. Restored by `cp`, checked by `cmp`.
- **Plant 2** "assertion `left == right` failed: the word before the br at 8 left: 1409286209 right: 3573759967"
  (0x54000041, the `b.ne`, where 0xd5033fdf, `isb`, should be): the dispatcher's two hit ISBs taken out
  (`monitor_code.rs` 786 and 793). No run test reaches this side: variant (a), the same dispatcher, read 0 faults in
  100; the judge is the dispatcher's words. Restored by `cp`, checked by `cmp`.
- **Files:** `crates/w2m_vm/src/run.rs`, `crates/w2m_vm/src/monitor_code.rs`, `crates/w2m_vm/src/guest.rs` (the
  option, one line), `crates/w2m_vm/src/lib.rs` (`CODE_SYNC_STUB`), `crates/w2m_vm/tests/run_loop.rs`,
  `crates/w2m_vm/tests/flush_stub.rs` (a test).

- **The gates:** the selection "Rust tests: rdeps(w2m_vm) | ..."; nextest "234 tests run: 234 passed (2 slow), 0 skipped"; the timing stage "passed, 1033 ns an exit at load 2.67"; "gates: all passed". `exit 0 in 736 s	default	2026-11-26T07:58:35Z	sprint 346	725c88fa90ee0dba40da1eb24da9b85f787b3925	847b9cc4a13a028ade8b44afcda3dd1d9a05f1763315b83d8ce1556972e306fe	root=8af2c49130255ccdc34dc5f2a76d294ab263922d	rust=crates python=0 samples=0	base=7fb0a268a04c6756cc024bfc325809144c05a807`
