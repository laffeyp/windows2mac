# Sprint 247 — The in-VM compile time

```yaml
---
id: 247
status: active
phase: 6
pass_kind: functional
---
```

## scope

Reviewer-3's Medium 1 of its review at 246 and the Decision "Sequence for step 5" as annotated after it: the revisit
clause ("the run compiles too few blocks at run time for the cache to move the rate") turns on the share of the run's
time spent compiling; Sprint 246 measured the host's share, 53.2 to 80.7 ms of about 28 s, and not the in-VM
compiler's, which lies inside `run_seconds` "vm" (census row 15d, partial).

- **Where it stands, by lookup (2026-10-30):**
  - The in-VM compiler's one entry is `w2m_vm_compile(target)` (`tools/guest_runtime/block_compiler.c` 221-317), nine
    returns; it runs on the thread that missed, before the thread goes on, so its time is compile time on the
    critical path.
  - Its counts live at `W2M_VMC_VA + 0x100` (`VMC_COUNTS_OFF`), 23 u64s to 0x1B8; the log starts at 0x1000.
    `W2M_VMC_VA` is `MONITOR_VA + VM_COMPILER` (`crates/guest_runtime/src/layout.rs` 71, 190), inside the monitor
    region; the exit ledger hashes the process area after `MONITOR_SIZE` (`run.rs` 385-388), so a slot past the counts
    is in no hash the frozen host's is held to.
  - The guest reads the counter and its frequency (`process.c` 31-46, `mrs cntfrq_el0`; the monitor code's `isb; mrs
    cntvct_el0`, WORKING_AGREEMENT 80).
  - The vocabulary's metric for it is `compile_critical_path_ms` (`signals/0.12.json` 187; v0.2 §, "Measure
    compilation on the critical path", line 456); no Rust record writes it, and the reference measures none.
  - Deferred searched for `block_compiler.c`, `run.rs`, `record.rs`, `batch_record.rs` (KIT_DIARY #102): row 15d's
    line (annotated at 246), this card's; row 15b's names `run.rs` for string errors, not touched by this work.
- **The readers:** `block_compiler.c`'s callers in the guest (the runtime's dispatch); `run.rs`' read of the counts at
  the stop; `record.rs`, `w2m-vm.rs`; `batch_record.rs`, which refuses a metric the reference does not measure.
- **The work:**
  - `w2m_vm_compile` becomes a wrapper: the counter read before and after the compile, the ticks added to a slot at
    `VMC_COUNTS_OFF + 8 * 23`, the frequency written to the slot after it.
  - The run reads both at the stop; the record writes `compile_critical_path_ms` "vm", the ticks over the frequency.
  - `batch_record.rs` allows that one metric the reference lacks, and holds it present and positive.
  - Three cold runs by `scripts/batch-run-record.sh`, at the load of 246's third record or Sprint 240's three where the
    machine allows; the share, VM compile plus host hvc 1 over the run's seconds; census row 15d's state from it.
  - The revisit clause judged on the share in the Decision: if small, it fires, and step 5's order goes to the user's
    plan with the number; else the cache follows.
- **Not here:** the cache.

## prerequisites

- 246 (closed)

## context_files

- `tools/guest_runtime/block_compiler.c`, `tools/guest_runtime/process.c` 31-46, `crates/guest_runtime/src/layout.rs`
- `crates/w2m_vm/src/run.rs`, `src/record.rs`, `src/bin/w2m-vm.rs`, `tests/batch_record.rs`
- `signals/0.12.json`; `windows_to_macos_research_direction_v0_2.md` 456; census row 15d

## signal contract

### Emits

- Three batch-run records, each with `compile_critical_path_ms` "vm" and Sprint 246's measurements.

### Invariants

- Each record conforms; the 23 counts as Sprint 246's; the exit ledger and the case runner unchanged.

## artifact contract

### Files modified

- `tools/guest_runtime/block_compiler.c`, `crates/w2m_vm/src/run.rs`, `src/record.rs`, `src/bin/w2m-vm.rs`,
  `tests/batch_record.rs`, `docs/step-4-census-2026-10-28.md` (row 15d), `BLACKBOARD.md`; three records

### Command exit codes

- `scripts/gates.sh` returns 0; each record's `check_records` 0 violations

## observation contract

- **Expected values:** the VM's compile milliseconds on each record; the share of the run's seconds, VM and host
  compile together; 274,834 compiled as before.
- **Planted faults:**
  1. The compile time left off the record: `batch_record.rs` names it.
  2. The frequency slot unwritten (0): the run's read names a zero frequency, not a division by it.
  3. A second metric the reference lacks written: `batch_record.rs` names it.

## done criteria

The record carries the in-VM compile time, and the Decision's revisit clause is judged on the share it gives.

## self-review checklist

- [x] Scope names one verifiable property: the VM's compile time on the record.
- [x] context_files cite the C, the layout, the readers, the vocabulary and the plan line.
- [x] Emits: three records.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and three plants.
- [x] One concept: the in-VM compile time.

## result
