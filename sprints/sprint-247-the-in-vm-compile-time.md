# Sprint 247 — The in-VM compile time

```yaml
---
id: 247
status: done
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

- **Departure, the design:** the card planned the bracket in `block_compiler.c`. Built so (c000817d3), it grew the
  runtime image by 80 bytes and moved every runtime address linked after it. The gates found it: `python_image.rs`
  had no frozen image under the new key, and, once its oracle was given a held row, a full no-fail-fast run found
  `batch_run.rs`' call stream differing at row 2535, 0x7f000418d31c for 0x7f000418d2cc. The review of 2026-10-27,
  item 15, had foreseen the first ("the first sprint that edits the C runtime replaces the test's oracle first"); the
  card's search of Deferred did not reach the Surfaced archive, where it stood. The C change was reverted, the held
  row with it, and the timing put where Sprint 164's is: in the monitor code Rust writes, off by default, so the gates'
  run is the frozen host's word for word.
- **The probe:** `monitor_code::Timing { runtime_word, compile }`; with `compile`, the miss stub reads `cntvct_el0`
  into x26 before the `blr` to `w2m_vm_compile` and, after it, adds the difference to `VM_COMPILER +
  VMC_COMPILE_TICKS` (0x1B8, before the save area at 0x200), 11 words (`the_compile_probe_is_off_by_default_and_its_
  size_known`). `w2m-vm --time-compile` asks it, refusing a timed run that compiled and summed no tick; the frequency
  is `host_state::counter_frequency`, the one reader `run_measurements` now shares;
  `scripts/batch-run-record.sh <root> --time-compile` passes it and writes `compile_timing` in the baseline, and
  `batch_record.rs` holds a timed record to one positive `compile_critical_path_ms` and any other to none.
- **A first figure that was no figure:** the C bracket's first run gave 762 trillion ms. A print of the raw sum,
  then a count of backward reads (1,091, the hvc 1 count), showed the counter never read: the asm said `"isb; mrs
  ..."`, and `;` begins a comment in Apple's assembler, as `process.c` 27 says beside its own read. The tree held the
  answer; it was not read before the asm was written.
- **The records:** three timed records from 4f6a736c2, each conforming, `check_records` 0 violations:
  `store/runs/w2m-vm-game-1793381341000000000`, `-1793381446000000000`, `-1793381554000000000`; the VM compiled
  274,834 blocks in 1,725, 1,800 and 1,626 ms, the host 62.8, 62.8 and 60.2 ms on hvc 1; compiling 5.33% to 5.49% of
  the run's seconds, at one-minute load 5.07 to 5.95, raised in part by this session's own builds and tests run
  beside them. The three C-bracket records (5.13% to 5.18% at quieter load) are in `store/superseded/`.
- **The census:** row 15d met from the timed records; `census.rs` now fails a cited record withdrawn to
  `store/superseded/` (reviewer-3's Low 2 of its review of the rework), and named row 15d's old citation before it
  was rewritten.
- **The order:** the Decision "Sequence for step 5" ruled before the timed records were read: the revisit clause's
  rate test withdrawn as a bar on the plan, step 5 next, the share and the hitch measured on the record. Deferred
  holds the shared counters for step 6.
- **The gates:** `scripts/gates.sh` exit 0 in 605 s on 2577a0557, copied from `store/gates/passes.tsv`; the timing stage
  677 ns an exit at load 3.27. Earlier runs, each row in `store/gates/`: on 3400f1e46 the commit walk named c2fe0bbc8
  (now in `signal_report`'s `BETWEEN_SPRINTS`); on ae0662289 one failed on the tail entry headed closed too soon and
  one passed in 493 s; on 35687d79b one failed on the card marked done too soon, one was stopped for reviewer-3's
  review at the close, and one failed in `family_runs.rs` on a drive name a killed process had left, fixed in
  2577a0557 (the tail entry's Observation 7).
- **Plants,** run on ae0662289, each restored by `cp` and checked by `cmp`.
- **Plant 1,** `a_timed_run_writes_its_compile_time` failed, "left: [] right: [vm, 1725.5, ms]" (abridged): the
  compile time left off the record, `record.rs` 266 reading `e.compile_ms` as `None`. A departure: the card named
  `batch_record.rs`, which reads only records already written; the unit test (ae0662289) holds the writer.
- **Plant 2,** `w2m-vm` exited 1, "--time-compile: 274834 blocks compiled in the VM and no tick summed": the probe's
  store a `nop` (`monitor_code.rs` 289, the probe still 11 words), a timed run of the game. A departure: the card's
  plant was the C design's frequency slot, which the rework removed; the frequency is now read on the host.
- **Plant 3,** `batch_record.rs` failed, "a measurement of frame_time_ms, which the reference does not measure": a
  second metric the reference lacks, the gates' record's first `load_average` row renamed `frame_time_ms`.
