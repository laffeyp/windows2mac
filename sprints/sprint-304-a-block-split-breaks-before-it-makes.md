# Sprint 304 — A block split breaks before it makes

```yaml
---
id: 304
status: done
phase: 6
pass_kind: functional
---
```

## scope

The second vCPU's precondition 4 (`docs/design-run-loop-on-several-vcpus-2026-10-31.md`), sized by Sprint 303: a vCPU
reads `ID_AA64MMFR2_EL1.BBM` 0, so a valid block made a table while another vCPU may hold it in its TLB needs the
block invalid, a broadcast invalidation, then the table (the Arm ARM's break-before-make; without it the result is
UNPREDICTABLE, a TLB conflict among the outcomes). `Guest::page_descriptor` (`guest.rs` 1064-1096) splits a block in
one store today. Its callers, the two requests that can split:

- a stack mapped or unmapped, `map_pages` from `threads.rs` 919 and 997 (THREAD_CREATE, THREAD_STACK_GROW), after which
  the runtime invalidates every vCPU's TLB (`threads.c` 318, `w2m_tlb_flush`, `tlbi vmalle1is`);
- a code page guarded at a host compile, `compile.rs` 212, 303, 399 and 443, after which the C invalidates that page on
  every vCPU (`block_compiler.c` 115, `tlbi vaae1is`).

- **The work:** a split in two steps, as the design states: `page_descriptor` finding a valid block makes it invalid
  and answers "split pending", the request it serves returning to the guest marked to retry; the guest's runtime
  invalidates every vCPU's TLB (`tlbi vmalle1is`, the stronger of the two forms, since a block covers many pages) and
  issues the request again; on the retry the block is invalid and the table is written whole before its descriptor, a
  Release store. Each of the two requests' paths in the C retries on that answer. A count of valid blocks split, on the
  batch run's record.
- **Not here:** the second vCPU; splitting at load, which the design rejected for its size (1 GiB of tables).

## prerequisites

- 303 (closed)

## context_files

- the design's precondition 4 and card 3; `guest.rs` (`page_descriptor`, `map_pages`); `threads.rs` 900-1000;
  `compile.rs` 195-450; `tools/guest_runtime/threads.c` 300-330; `tools/guest_runtime/block_compiler.c` 100-120;
  `runtime.h` 328-335
- the Arm ARM (DDI 0487) on break-before-make, cited by section in the result

## signal contract

### Emits

- None; an order of writes, held by a test and the runs.

### Invariants

- Every run as held; one vCPU; the runtime image's departures from the frozen C named in `python_image.rs`.

## artifact contract

### Files modified

- `crates/w2m_vm/src/` (`guest.rs`, `threads.rs`, `compile.rs`, the record's count), the C's two request paths, a
  test, `python_image.rs`' departures

### Command exit codes

- `scripts/gates.sh` returns 0

## observation contract

- **Expected values:** the count of valid blocks split on the batch run; each split's writes in order: the block
  invalid, the request's retry, the table's entries, the table's descriptor.
- **Planted faults:** (1) the split made in one step again (the table written at the first call): the recording test
  must name the write that came before the retry.

## done criteria

No valid block becomes a table without first being invalid with every vCPU's TLB invalidated.

## self-review checklist

- [x] Scope names one verifiable property: break-before-make for every split.
- [x] context_files cite the design, the callers, the C and the Arm ARM.
- [x] Emits: none.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and a plant changing what its judge reads.
- [x] One concept: a split's order, across its two request paths.

## result

- **A departure from the card's form, found before the work:** the card's two steps leave the block invalid between
  the first answer and the retry; on two vCPUs another touching any page of that 2 MB block takes a translation fault,
  which the runtime's fault path delivers as an access violation. The form built makes the split with every other
  vCPU stopped (the design's precondition 8, which card 4 builds for locked model steps; on one vCPU it holds), writes
  the table whole before its descriptor, and resumes the requesting vCPU through a flush stub in the monitor area,
  `dsb ishst; tlbi vmalle1is; dsb ish; isb; hvc #15`, before it touches memory: every vCPU's TLB loses the block
  entry, and no page is ever unmapped. The C runtime is unchanged, against the card's files; the design's
  precondition 4 says so.
- **The change** (07e802849): `a64_encoder::DSB_ISHST`; `w2m_vm::FLUSH_STUB` (0x2010, in the free area past the
  exception entry, beside `RETURN_STUB`) and `FLUSH_STUB_END`; `monitor_code::HVC_TLB_FLUSHED` (15) and `flush_stub`,
  written by `write`; `Served.splits`, counted in `page_descriptor`, its row in the design's table (23 Served) and
  `the_served_part_is_the_designs`' count; in `run`, a vCPU whose `splits` is ahead of `out.flushes` resumed at the
  stub, its PC kept, and given it back at `hvc #15`, `Outcome.flushes` counting; the stub's range, its end the PC
  after its `hvc`, kept out of the two places the loop sets and reads x23, which in the runtime's C is the C's own.
- **Expected values:** the sleeper run, 3 blocks split and 3 flushes, `hvc #15` 3 times among its exits, its stop and
  its comparisons with the Python host as before. The batch run: 35 blocks split, `hvc 15` 35 among its exits, every
  other count and its reference stop as before. That count, which the card asked for, was first written here as not
  recorded, `w2m-vm` reporting no counter of `Outcome`'s kind; the gates on 07e802849 showed otherwise, failing
  `batch_run.rs` and `batch_record.rs` on the batch run's exits, which the held stop file lists by `hvc` number and
  now holds `hvc 15\t35` (`store/batch_run/first-stop-2026-10-28.tsv`), no other figure moved.
- **The class the new exit moved, found by two gates runs and closed by one search:** the gates on bdea165fa failed
  `exit_ledger.rs`: the exit ledger compares the two hosts' exits row by row, and each `hvc #15`, which the frozen
  host never took, shifted every row after it, 29 held differences becoming 581. The flush stub's exit, the host's own
  TLB maintenance, has no ledger row now (`run.rs`, its two ledger sites). Each gates run had stopped at its first
  failures, so the batch-run step and every test depending on `w2m_vm` ran again with `--no-fail-fast`: 180 of 180
  passed, nothing else in the class moved.
- **A gap the form leaves, found by the audit after the commit:** the table is written before the flush, so the
  requesting vCPU's own TLB may hold the block for the stub's first two instructions while a speculative walk loads
  the table's entry, an UNPREDICTABLE pair; the Arm order needs the request's page writes after the flush. Stated in
  the design's precondition 4 and in BLACKBOARD Deferred (Sprint 304), with the TLB conflict abort's signature as a
  revisit; and card 4's stop of the other vCPUs now names the split, which the stub assumes (5e86ccbf1, the design
  alone).
- **The gates:** `scripts/gates.sh` exit 0 in 692 s, kept in `store/gates/passes.tsv` as sprint 304 at 36f751b13 with root=23e112bed and "rust=crates python=0 samples=0", rdeps(a64_encoder), rdeps(signal_report) and rdeps(w2m_vm), 237 tests, base=c54b9efed, Sprint 303's close; the runs on 07e802849 and bdea165fa stopped at the batch run's stop file and the exit ledger, no pass row; multi-vCPU: not run (Python gates not run)
- **Plant 1,** restored by `cp` and checked by `cmp`: the redirect switched off (`if false && …`); the sleeper test
  failed, "a split without its flush: the vCPU ran on with every TLB's block entry", left 0, right 3.

