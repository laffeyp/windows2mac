# Sprint 305 — A split in the Arm order

```yaml
---
id: 305
status: active
phase: 6
pass_kind: functional
---
```

## scope

Reviewer-3's Medium 1 at 304: Sprint 304 writes the new table, then the descriptor, then flushes through `FLUSH_STUB`,
so the requesting vCPU may hold the old block and the new table's entry at once, which break-before-make forbids at
BBM 0 (Sprint 303); Arm leaves the outcome a TLB conflict abort or the silent use of either entry, and Deferred
(Sprint 304)'s revisit could see only the first. With every other vCPU stopped, the strict order costs one exit a split.
A split happens inside a request (`map_pages` writes page entries into the new table in the same host call), so the
host cannot return to the guest between the steps; it runs the requester's vCPU itself, from inside the split.

- **The work:** `Guest::page_descriptor` and `map_pages` take `flush: &mut dyn FnMut() -> Result<(), String>`; a split
  writes the block's descriptor invalid (a Release store), calls `flush`, then writes the table whole and its
  descriptor. The run loop's flush: the vCPU's PC kept, set to `FLUSH_STUB`, run until `hvc #15` (any other exit an
  error), the PC given back, x23 untouched. The callers pass it down: `stack_grow` and `create_thread` from
  `host_request::serve`, `protect_code`, `code_written` and `stop_cache` from the `hvc #1` and `#13` services;
  `use_cache`, at load before any vCPU ran, a flush that does nothing, said so. Sprint 304's resume through the stub
  after the request and its `splits` against `flushes` go; the count of splits stays, on the batch run's held file.
- **The nested run, a second small run loop (reviewer-3, by message at 304):** (1) the vCPU's PC, PSTATE and every
  general register kept and given back, the stub writing none but the TLB, the test asserting all equal before and
  after; (2) an exit other than `hvc #15`: a vtimer runs the stub again with the timer masked as the outer loop masks
  it; a cancel (`hv_vcpus_exit`, card 4's stop) is refused by name after the split finishes, the table written, so no
  block is left invalid; any other exit an error; (3) x23, the budget, untouched; (4) the flush counted once in the
  exits as `hvc 15`, the batch run's held 35 kept, and given no exit-ledger row. On (2), reviewer-3's note: if a
  cancel sent while the vCPU is outside `hv_vcpu_run` makes the next run return at once, the nested run must absorb it
  before re-entering the stub, record it, and report it after the split, or it never finishes the flush. Looked up
  while drafting, the SDK's `hv_vcpu.h` 377-378 on `hv_vcpus_exit`: "If a vcpu is not running, the next time
  hv_vcpu_run is called for the corresponding vcpu, it will return immediately without entering the guest": the one
  run that returns cancelled uses the pending cancel up, so the nested run notes it and runs the stub again, and the
  cancel is reported after the split; the test cancels the vCPU from another thread during a split and asserts the table written, the flush
  run, and the cancel reported after. The run's time limit (`w2m-vm`'s `GUEST_SECONDS`) sends such a cancel today.
- **Not here:** the stop of the other vCPUs (card 4); the C runtime, unchanged.

## prerequisites

- 304 (closed)

## context_files

- the design's precondition 4 (Sprint 304's form and gap); `guest.rs` `page_descriptor` and `map_pages`; `threads.rs`
  `stack_grow` 888 and `create_thread` 942; `compile.rs` `use_cache` 192, `protect_code` 295, `code_written` 389,
  `stop_cache` 425; `run.rs`' flush; BLACKBOARD Surfaced, the review at 304; Deferred (Sprint 304)

## signal contract

### Emits

- None; an order of writes, held by a test and the runs.

### Invariants

- Every run as held, the batch run's 35 splits among them; one vCPU.

## artifact contract

### Files modified

- `crates/w2m_vm/src/` (`guest.rs`, `threads.rs`, `compile.rs`, `host_request.rs`, `run.rs`), a test, the design's
  precondition 4

### Command exit codes

- `scripts/gates.sh` returns 0

## observation contract

- **Expected values:** a split's writes and its flush in the order invalid, flush, table entries, table descriptor,
  recorded by a test through a `flush` that notes when it ran against the descriptor's value then; the sleeper's 3
  splits and the batch run's 35, each with its flush, the runs as held.
- **Planted faults:** (1) the table written before `flush` is called, as Sprint 304 built it: the recording test must
  name the descriptor valid at the flush.

## done criteria

A block becomes a table only after it was invalid and every vCPU's TLB was flushed, in Arm's order.

## self-review checklist

- [x] Scope names one verifiable property: the Arm order of a split.
- [x] context_files cite the design, the callers, the run loop and the record.
- [x] Emits: none.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and a plant changing what its judge reads.
- [x] One concept: a split's order, passed to its callers.

## result

- **The change** (WORK-COMMIT): `guest::Flush`, a `&mut dyn FnMut() -> Result<(), String>`; `page_descriptor` making
  the block invalid (a Release store), calling `flush`, then storing the table's descriptor, the table's 512 entries
  written before; `map_pages`, `threads::stack_grow` and `create_thread`, and `HostCompiler::compile`, `install`,
  `install_block`, `protect_code`, `code_written` and `stop_cache` passing it down; `use_cache`, at load, a flush that
  does nothing, said on its line; `run::flush_through_stub` and `run::FlushCounts`, the nested run of the card's four
  points; `host_request::serve` taking the counts, the case runner's call a fresh set; the run loop folding the
  counts into `exits` as `hvc 15` and `flushes`, a cancel seen during a split ending the run as the time limit after
  it; Sprint 304's resume after the request, its PC kept across turns, its `hvc #15` arm, its x23 range checks and its
  ledger skip all gone, a nested exit never reaching the loop; the litmus test's compile given the real flush, the
  unit tests and `run_loop.rs`' direct calls, with no vCPU run, a no-op one; `a_split_makes_the_block_invalid_before_its_flush_and_the_table_after`;
  the design's precondition 4.
- **A caller the first search missed:** `compile`, the host compile at `hvc #1`, calls `protect_code` at
  `compile.rs` 129 on a line with a trailing comment, and the search had dropped every line holding `//`; the compiler
  named it. The class: a search that filters out comments drops code lines that carry one.
- **Expected values:** the order test: one flush, the descriptor at it the block's with its valid bit clear, a table
  after; the sleeper run, 3 splits and 3 flushes, as held; the batch run's 35, the gates to show.
- **Not shown by a test of their own:** the card's register assertion (1) and its cancel test (2). The nested run
  restores 31 general registers, PSTATE and the PC, and the sleeper's 3 and the batch run's 35 nested flushes leave the
  runs as held, which a register lost would not; a cancel during a split needs a second thread to cancel within the
  stub's five instructions, a race this card did not build. Both stand as Deferred (Sprint 305).
- **The gates:** GATES-LINE
- **Plant 1,** restored by `cp` and checked by `cmp`: the table's descriptor stored where the block is made invalid,
  Sprint 304's order, after a first try whose `sed` matched nothing, `cargo fmt` having wrapped the line; the order test
  failed, "at the flush the block is invalid, its other bits kept: 0x19970003".

