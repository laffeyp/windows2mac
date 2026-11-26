# Sprint 346 — A new block's code is visible to every vCPU before its entry is published

```yaml
---
id: 346
status: active
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

## result

GATES-LINE
