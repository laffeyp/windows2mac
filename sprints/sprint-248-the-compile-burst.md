# Sprint 248 — The compile burst

```yaml
---
id: 248
status: active
phase: 6
pass_kind: functional
---
```

## scope

The Decision "Sequence for step 5" as annotated at Sprint 247's close: before the translation cache, the largest
compile time in a short window, on timed cold records run alone, with the total re-measured on them (reviewer-3's
Direction and Low 2 at 247's close). Sprint 247 put the total on the record, 1,626 to 1,800 ms of 274,834 compiles,
measured beside the session's own builds; a total says nothing of how the compiles bunch.

- **Where it stands, by lookup (2026-10-30):**
  - The probe: with `Timing::compile` the miss stub reads `cntvct_el0` into x26 before `blr w2m_vm_compile` and after
    it adds the difference to `VM_COMPILER + VMC_COMPILE_TICKS` (0x1B8) (`monitor_code.rs` 274-289); x0-x15, x17
    and x30 are saved at `VMC_SAVE` (0x200) before the call and restored after the sum, so between the two they are
    free, and 0x1C0 to 0x1FF are unused.
  - The guest reads the counter's frequency as `cntfrq_el0`; Sprint 247's first design read it so and its figures,
    1,324 to 1,381 ms, agreed with the host's `hw.tbfrequency`, 24,000,000. clang gives `d53be000` for `mrs x0,
    cntfrq_el0` (2026-10-30); `a64_encoder` has `CNTVCT_EL0` and no `CNTFRQ_EL0`.
  - `run.rs` 985 reads the sum at the stop; `w2m-vm` turns it into ms by `host_state::counter_frequency`; `record.rs`
    writes `compile_critical_path_ms` "vm"; `batch_record.rs`' `compile_time_problems` holds a timed record to one
    value. The vocabulary's `variant` is free text naming what was measured (`signals/0.12.json` 1335), as
    `run_seconds` "vm" and "host hvc <k>" already do; no new metric is needed.
  - Searched for `monitor_code.rs`, `run.rs`, `record.rs`, `w2m-vm.rs`, `batch_record.rs`, `a64_encoder` in Deferred,
    Surfaced and `docs/surfaced-archive-2026-10-30.md`: Deferred (Sprint 247), the counters as plain adds, holds the
    new slots too, re-visited at step 6; census row 15b names `run.rs` for string errors, which this card does not
    change. The lookup found that 1df116e4b wrote that Deferred line into census row 15d's, leaving 15d's notes after
    it so that it read resolved; the notes go back to row 15d in this card's commit. `guest.rs` is not changed, so
    census row 15a's re-visit does not come due.
- **The readers:** the miss stub's callers (every block miss); `run.rs`' read at the stop; `w2m-vm.rs`, `record.rs`;
  `batch_record.rs` on the gates' record and on every stored record.
- **The work:**
  - `a64_encoder::CNTFRQ_EL0`, held to clang's word.
  - The probe, still only with `Timing::compile`, keeps three more figures in counter ticks: the longest one compile
    at 0x1C0; and frames of 1/60 s, a 60 Hz display's frame: a frame opens at a compile's start (0x1C8) and holds
    every compile that ends within 1/60 s of it, its compile ticks summed at 0x1D0, the most any closed frame held at
    0x1D8. The test of the probe's size is updated; with `Timing::compile` off the words are unchanged.
  - The run reads the three at the stop, the open frame's sum folded into the most.
  - The record writes `compile_critical_path_ms` "vm longest compile" and "vm most in a 60 Hz frame" beside "vm".
  - `batch_record.rs` holds a timed record's figures to their order: 0 < longest ≤ most in a frame ≤ total, and the
    most in a frame ≤ 1000/60 ms plus the longest, since a frame closes on the first compile ending past it. Sprint
    247's three timed records, made before the frames, are named and held to the total alone.
  - Three timed cold runs by `scripts/batch-run-record.sh --time-compile`, nothing else of this session running; the
    totals beside 247's.
- **What the figure decides:** the batch run has no frames, so the most in a 60 Hz frame sizes the worst burst of
  compiling a cache would remove from a frame's span; whether a frame is late is judged on step 7's play path.
- **Not here:** the cache; a frame-time measurement.

## prerequisites

- 247 (closed)

## context_files

- `crates/w2m_vm/src/monitor_code.rs` 228-300, `crates/a64_encoder/src/lib.rs` 940-960
- `crates/w2m_vm/src/run.rs` 281, 985; `src/record.rs` 266; `src/bin/w2m-vm.rs` 754-765; `tests/batch_record.rs` 476-500
- `signals/0.12.json` 187, 1335; `windows_to_macos_research_direction_v0_2.md` 456; the Decision "Sequence for step 5"

## signal contract

### Emits

- Three batch-run records, each with `compile_critical_path_ms` "vm", "vm longest compile" and "vm most in a 60 Hz
  frame", and Sprint 246's measurements.

### Invariants

- Each record conforms; the 23 counts as Sprint 246's; the untimed monitor code word for word the frozen host's.

## artifact contract

### Files modified

- `crates/a64_encoder/src/lib.rs`, `crates/w2m_vm/src/monitor_code.rs`, `src/run.rs`, `src/record.rs`,
  `src/bin/w2m-vm.rs`, `tests/batch_record.rs`, `BLACKBOARD.md`; three records

### Command exit codes

- `scripts/gates.sh` returns 0; each record's `check_records` 0 violations

## observation contract

- **Expected values:** each record's total, longest compile and most in a 60 Hz frame in ms, in that order of size;
  274,834 compiled as before; the load at start.
- **Planted faults:**
  - (1) The frame never closing (the probe's frame test inverted): `batch_record.rs` names a most in a frame above
    1000/60 ms plus the longest.
  - (2) The longest never stored: `batch_record.rs` names a longest of 0.
  - (3) The frame row left off the record: `record.rs`' test names it.

## done criteria

Three timed records carry the longest compile and the most compiling in a 60 Hz frame, measured alone, and the
Decision is annotated with them.

## self-review checklist

- [x] Scope names one verifiable property: the compile burst on the record.
- [x] context_files cite the probe, the encoder, the readers, the vocabulary and the plan line.
- [x] Emits: three records.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and three plants.
- [x] One concept: how the compiles bunch.

## result
