# Sprint 360 — The instruction clock on two vCPUs

```yaml
---
id: 360
status: active
phase: 6
pass_kind: functional
---
```

## scope

The Decision "Step 6's order weighed again at Sprint 360, G2 unmet" puts this card first. G2, the 68 families on two
vCPUs, stands at 267 of 273 runs conforming. Two of the six held runs, `processes` 38189162864 and 38189164581
(`store/family_runs/two-vcpus-refused-2026-11-26.tsv`), take the instruction clock. `vcpus_allowed` (`run.rs` 653-689)
refuses them: "the instruction clock reads the running vCPU's own count". `process.c` 41-46 computes the clock in clock
mode 2 as the instructions this process ran, at a set rate, plus the idle warp, from `monitor_word(0)`, the running
vCPU's own count. The per-vCPU pages design left it open: the runs that take it "stay on one vCPU until a card decides
how the counts combine" (`docs/design-each-vcpus-own-pages-2026-10-31.md` 106-109). This card decides that and builds
it.

- **The decision, first, written into the card's result before any code:**
  - **What the process's instruction count is on N vCPUs.** For instance, the sum of every vCPU's retired
    instructions: each vCPU publishes its count, and a reading adds the others' last published counts to its own.
  - **How a reading stays a clock.** Windows' QueryPerformanceCounter is monotonic and consistent across processors.
    Look it up: Microsoft Learn, "Acquiring high-resolution time stamps", and QueryPerformanceCounter's page; quote
    what they say. A reading on any vCPU must therefore never be below one any thread saw before. Say how the design
    ensures it, for instance a process-wide floor that each reading raises by an atomic max.
  - **The idle warp** (`W2M_CLOCK_WARP_VA`), and the deadline wakes that read the clock (`wake_expired`, the host's
    `warp_to_deadline` and `timed_waiter`): on two vCPUs, what each reads and writes.
  - **What one vCPU keeps:** the clock on one vCPU is bit for bit today's, so the batch run, the reference runs and every
    one-vCPU run are unchanged. The sum of one vCPU's count is its count.
  - Name each choice's grounds, and what Windows defines against what w2m chooses.
- **The build.**
  - The clock's C and the host's readers of the count as the decision needs.
  - `vcpus_allowed`'s instruction-clock refusal removed.
  - The two `processes` runs removed from `two-vcpus-refused-2026-11-26.tsv` and held to Windows' rows on two vCPUs in
    the gates' two-vCPU family runs (`scripts/gates.sh` `family_runs 2`), as Sprint 351 held the 267.
- **The class.** List every reader and writer of the instruction count and the warp, in the C and the host, one line
  each, with what it does on two vCPUs. That includes the slice and deadline arithmetic that turns a deadline into a
  count (`threads.c` `slice_until`, `threads.rs`' `slice_until`). Report each defect found to main at once, with its
  evidence.
- **Not here:** the batch run's stop on two vCPUs (its own card, later); a child process (card 4f).

## prerequisites

- 359 (closed)

## context_files

- `tools/guest_runtime/process.c` in full; `threads.c` (`slice_until`, `wake_expired`, the switch's warp); `runtime.h`
  (the clock's words).
- `crates/w2m_vm/src/run.rs` 640-690 (`vcpus_allowed`) and every host reader of the count and the warp (`grep -n
  'CLOCK_WARP\|CLOCK_RATE\|clock_mode\|instruction_clock\|warp_to_deadline' crates/w2m_vm/src/*.rs`).
- `docs/design-each-vcpus-own-pages-2026-10-31.md` 100-125.
- `crates/w2m_vm/tests/family_runs.rs` and `store/family_runs/two-vcpus-refused-2026-11-26.tsv`; `scripts/gates.sh`
  (`family_runs`).
- `grep -n "Step 6's order weighed again at Sprint 360" BLACKBOARD.md`: the Decision.
- `WORKING_AGREEMENT.md` lines 137-266, "Build and verification commands".

## signal contract

### Emits

- None: the clock's readings are the guest's own results, judged in the family runs' rows; no emit site changes.

### Invariants

- On one vCPU the clock is bit for bit today's: the batch run, the reference runs, the exit ledger and every family
  run pass unchanged.
- The 267 two-vCPU family runs that conform today still conform.

## artifact contract

### Files modified

- `process.c`, `threads.c` and the host's clock readers as the decision needs, `run.rs` (`vcpus_allowed`), the refused
  list, the tests, and `DEPARTURES`. One concept: the instruction clock on several vCPUs.

### Command exit codes

- `scripts/gates.sh` returns 0
- `target/release/signal_report --check` returns 0 on the tree

## observation contract

- **Expected values:**
  - The decision, with Microsoft Learn quoted.
  - The class list.
  - The two `processes` runs' rows on two vCPUs equal to Windows', in 20 runs each.
  - A two-vCPU judge of the clock's monotonicity: two threads, one on each vCPU, each reading QueryPerformanceCounter
    in a loop and recording the other's last reading. No reading is ever below one already seen, across 20 runs.
  - One vCPU's clock unchanged, by the batch run's held stops and the reference runs.
- **Planted faults:**
  - (1) A vCPU's reading from its own count alone, today's form. The monotonicity judge fails, a reading below one the
    other thread saw; or the `processes` rows differ.
  - (2) The floor (or whatever keeps readings monotone) removed. The monotonicity judge fails.
  - (3) The refusal restored. The two `processes` runs are refused again, and the gates' two-vCPU step names them.

## done criteria

The instruction clock on N vCPUs is one process-wide clock, monotonic across vCPUs as QueryPerformanceCounter is. The
two `processes` runs conform on two vCPUs, so 269 of 273 do, and one vCPU's clock is unchanged bit for bit.

## self-review checklist

- [x] Scope names one verifiable property: one monotonic process clock on N vCPUs; two held runs conforming.
- [x] context_files cite the clock's C, the host's readers, the design's open question, the family runs and the
      Decision.
- [x] Emits: none, with its reason.
- [x] Artifact contract gradable by command.
- [x] Observation contract: the decision, the class list, the rows, a monotonicity judge; numbered plants.
- [x] One concept: the instruction clock on several vCPUs.

## result

GATES-LINE
