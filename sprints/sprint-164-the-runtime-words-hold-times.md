# Sprint 164 — The runtime word's hold and wait times, on a record

```yaml
---
id: 164
status: done
phase: 6
pass_kind: functional
---
```

## scope

The third of step 3's measurements (`docs/rust-port-plan-2026-10-24.md` step 3, amended 2026-10-27; reviewer-2's
High 1): how long the runtime word (`RUN_TABLE + RUNTIME_LOCK`, `monitor_code.rs` 27; `vm_backend.py` 518) is held per
taking, and how long a taker waits for it, as `runtime_word_hold_ns_p50/p99` and `runtime_word_wait_ns_p50/p99`
(vocabulary 0.11, P24). The host design budgets 200 ns a hold and revisits at a p99 wait over 10 µs
(`docs/design-rust-host-2026-10-24.md` 131-136; Decision 2026-10-24).

The word is taken and released only inside guest code the monitor writes (`lock_runtime`, `unlock_runtime`,
`monitor_code.rs` 49-74), at four places: the switch stub (`guest.rs` 332, 368, 372), a bridge (435, 439), the
dispatcher's call into the VM compiler (`monitor_code.rs` 134, 149) and the exception entry (219, 225). No host code
sees a taking, so the guest timestamps it with the virtual counter (`mrs x, cntvct_el0`, d53be050 for x16, clang
2026-10-27):

- **Timed lock** (`lock_runtime` clobbers x16 and x18 already): the counter to a fixed slot before the first attempt
  (the wait's start) and again after the taking.
- **Timed unlock**, given two registers free at its site (the bridge: x1 and x2, reloaded after; the VM compiler's and
  the exception entry's calls: x18 and x1 or x18 and x25 by what each reloads; the switch stub: x18 and x25, taken
  before its registers are reloaded, a few loads before the release): the triple (wait start, taken, released) to a
  ring of 32-byte entries, with its count and an overflow count.
- **Where:** the monitor data page from 0x600 (unused: its highest use is the YMM upper halves at 0x400-0x4FF,
  `PER_THREAD_MONITOR`; the vector table begins at 0x1000): the index at 0x600, the two slots at 0x608 and 0x610, the
  overflow at 0x618, 48 entries from 0x620.
- **Off unless asked:** `Start::time_runtime_word` (default false) chooses the timed forms, so the monitor code a run
  writes is the Python's word for word (Sprint 136's judge) unless the measurement asks; the run loop drains the ring
  at each exit into `Outcome.runtime_word` and resets its index.
- **On the record:** `w2m-vm --serve-times` gains the takings; `run_measurements` adds the four metrics per guest, in
  ns from the counter's ticks at its frequency (`sysctl -n hw.tbfrequency`, read on the record's Mac), and the
  overflow count must be 0.

## prerequisites

- 163 (closed)

## context_files

- `crates/w2m_vm/src/monitor_code.rs`, `crates/w2m_vm/src/guest.rs` 300-445, `crates/w2m_vm/src/process.rs` (`Start`),
  `crates/w2m_vm/src/run.rs`, `crates/w2m_vm/src/bin/w2m-vm.rs`, `crates/w2m_vm/src/bin/run_measurements.rs`
- `src/w2m/vm_backend.py` 88-130 (the monitor data page's offsets)

## signal contract

### Emits

- On `store/runs/w2m-vm-measurements-<ns>`: SESSION_INIT (purpose `measurement`), MEASUREMENT_RECORDED (Sprint 163's
  metrics, and `runtime_word_hold_ns_p50`, `runtime_word_hold_ns_p99`, `runtime_word_wait_ns_p50`,
  `runtime_word_wait_ns_p99` per guest), SESSION_COMPLETED (completed).

### Consumes

`tests/data/timed_waits/qpc.exe`, `sleeper.exe`.

### Invariants

- No Python file added or changed; with `time_runtime_word` false the monitor code and every judge are as before
  (Sprint 136's, 155-158's).

## artifact contract

### Files modified

- `crates/w2m_vm/src/monitor_code.rs`, `guest.rs`, `process.rs`, `run.rs`, `bin/w2m-vm.rs`, `bin/run_measurements.rs`,
  `WORKING_AGREEMENT.md`; a record under `store/runs/`

### Command exit codes

- `uv run substrate run --file harness/w2m-vm-measurements.toml --root store/runs/w2m-vm-measurements-<ns> --strict`
  returns 0
- `scripts/gates.sh` returns 0

## observation contract

- **Expected values:** each guest runs to ExitProcess with its output unchanged by the timing; the takings counted
  equal the lock's takings (the drained entries plus the overflow); every entry's wait start ≤ taken ≤ released; with
  one vCPU no taker meets the word held, so each wait is the taking's own cost, a few ticks; the holds are stated as
  found against the design's 200 ns. The counter's tick (41.7 ns at 24 MHz) bounds the resolution, stated.
- **Planted fault:** the timed unlock writing its release time before reading the taking's: an entry with released <
  taken, refused by the drain by name.

## done criteria

Step 3's three measurements are on records; step 3 closes by its own text.

## self-review checklist

- [x] Scope names one verifiable property: the runtime word's hold and wait times on a record.
- [x] context_files cite the code the timing enters.
- [x] Emits: every tag the run fires, by name.
- [x] Artifact contract gradable by command.
- [x] Observation contract: bounds, a resolution, and a plant the drain refuses.
- [x] One concept: the runtime word's timing.

## result

- **The ring the card describes, then histograms, then an entry per taking.** A 48-entry ring drained at the stop lost
  2,914 of `sleeper`'s 3,002 takings to its overflow count. Its replacement, one pair of 128-bucket tick histograms for
  all four sites, met reviewer-2's review of 2026-10-28: the sites mixed, the last bucket (5.3 µs) below the design's
  10 µs revisit point, a lower bound carried in the variant's prose, and counter reads without an ISB. The code now
  writes every taking to a 64-byte entry (`monitor_code.rs` `record_taking`; `LOCK_TIMING`, the 1,331,200 bytes past
  the monitor region's eight vCPU areas, 16,384 entries): wait start, taken, released, its `Site`, and the host's exit
  count at each of the three (the run loop counts each exit there). Each counter read is `isb; mrs cntvct_el0`, as
  Linux reads it (`arch/arm64/include/asm/arch_timer.h`, read 2026-10-28; this Mac has FEAT_ECV, but whether the guest
  sees it is not known, and the ISB is right either way). The quantiles are nearest rank over exact ticks, so there is
  no bound; the resolution is the counter's tick, 41.7 ns (`sysctl -n hw.tbfrequency` 24000000).
- **The record** (`store/runs/w2m-vm-measurements-1793171817000000000`, substrate exit 0): SESSION_INIT (purpose
  measurement, vocabulary 0.11), 41 MEASUREMENT_RECORDED, SESSION_COMPLETED completed in 1.609 s.
- **The runtime word's times by site** (from a second, timed run of each guest; nearest rank; "across" holds an exit
  inside the hold, so the host's time is in it):

  | guest | site | takings | hold p50 | hold p99 | wait p50 | wait p99 |
  |---|---|---|---|---|---|---|
  | sleeper | switch | 3,018 | 83 ns | 83 ns | 0 ns | 42 ns |
  | sleeper | bridge | 5 | 2,125 ns | 6,583 ns | 42 ns | 42 ns |
  | sleeper | bridge, across | 7 | 2,458 ns | 57,500 ns | (with the 12 above) | |
  | sleeper | compile | 28 | 3,417 ns | 245,708 ns | 0 ns | 7,458 ns |
  | qpc | bridge | 2 | 4,292 ns | 5,625 ns | 42 ns | 42 ns |
  | qpc | bridge, across | 1 | 4,292 ns | 4,292 ns | (with the 3 above) | |
  | qpc | compile | 12 | 1,333 ns | 143,458 ns | 0 ns | 6,792 ns |

  No exception-site taking occurred. Below 100 takings the p99 is the largest value. The switch stub's hold, the one
  the design prices at 200 ns ("After this change the word is taken only for scheduling", design lines 131-134), is 83 ns at p50 and
  p99 over 3,018 takings. The compile site holds the word across an in-VM compile for up to 246 µs, and a bridge
  holds it across its C service; these are the holds the design's change 3 (no lock across a compile) and the split of
  the C runtime's lock remove at step 6. The design's revisit rule (the word busy above 5%, or a p99 wait above
  10 µs, at step 6) is not reached on one vCPU.
- **The waits.** Every wait is 4 ticks or less but each run's first taking, a compile-site taking with no exit in it:
  163 ticks on `qpc`, 179 on `sleeper` (6,792 ns and 7,458 ns, the compile site's p99s). The card's line "each wait is
  a few ticks" holds for every taking but that one. Writing the timing area's first page from the host before the run
  left it at 170 to 388 ticks, so host page allocation alone does not explain it; its cause is not measured.
- **The plant** (the entry's taken and released slots swapped): the Producer failed by name, "qpc: 15 takings, 15
  backward"; the lines restored, the record above was made from the restored code. The histograms' plant (the hold's
  `sub` reversed) failed by name too, "qpc: 15 takings, 14 backward", before they were replaced.
- **Finding 12 of the code review of 2026-10-27** (the serve timer timed its own `format!`): fixed here, the kind named
  before the timer starts; Sprint 163's exit medians include it and are biased upward.
- **Step 3** has its three measurements on records: an exit's time by kind, the WFx traps, and the runtime word's hold
  and wait.
