# Sprint 164 — The runtime word's hold and wait times, on a record

```yaml
---
id: 164
status: active
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
