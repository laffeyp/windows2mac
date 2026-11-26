# Sprint 353 — Every per-vCPU word the host writes, checked against run-wide state, and slicing past vCPU 0 judged

```yaml
---
id: 353
status: active
phase: 6
pass_kind: functional
---
```

## scope

Reviewer-4's twenty-first review, Medium 1, recorded as the Decision "The served-vCPU words, enumerated in card 353".
Sprint 351 found that a thread taken by vCPU 1 ran on the run's whole step budget and was never put down. `set_budget`
(`crates/w2m_vm/src/threads.rs` 1328-1341) computes `shared` from run-wide state,
`guest.served.threads.len() > 1 || guest.served.link.is_some()`, and writes the limit only at
`guest.served.vcpu.monitor + 8`. Its callers are `threads.rs` 1322, `host_request.rs` 545 and `run.rs` 1358 and 1877.
Sprint 351 added a call in `take_a_thread`. That defect names a class: a per-vCPU word whose right value depends on
state another vCPU's service can change, written only for the vCPU served when that state changed.

- **The enumeration.** List every word of a vCPU's monitor data page or state block that the host writes, one line a
  word: its writer and line, which vCPU it writes, and whether its right value depends on run-wide state. The words
  include at least:
  - at the run's start, for every vCPU: the limit `md + 8`, `MXCSR_OFF`, `LOCK_OWNER`, `FCW_OFF` (`run.rs` 983-998);
  - at each turn's start, for the turning vCPU: `SLICE_UNTIL`, `RUN_END` (`run.rs` near 1464);
  - for the served vCPU: `CURRENT`, `CURRENT_ENTRY`, `SLICE_COUNT`, `SLICE_PAUSES`, the limit (`threads.rs` 175-176,
    1033-1034, 1067-1068, 1075-1076, 1341);
  - any other found by grep of the monitor-data offsets in `crates/w2m_vm/src`.
- **Each word whose value can go stale on another vCPU is a defect.** It can leave a thread past its slice, so fix it
  in this card under the user's Decision of 2026-10-18, and report each one to the Supervisor at once with its
  evidence. Two cases to check first:
  - `SLICE_UNTIL`, computed from a run-wide count of ready threads that another vCPU's service changes between this
    vCPU's turns;
  - `set_budget` when `shared` turns true while another vCPU is mid-slice on the whole budget, for instance a second
    thread made on vCPU 0 while vCPU 1 runs the first.
- **The judge.** A guest built as `tests/data/timed_waits`' guests are: three CPU-bound threads on two vCPUs, each
  counting in a loop, the second made while the first already runs on vCPU 1. It is held to every thread's count
  being above 0 by the run's end, and the run ending inside its limit. Run it 20 times on two vCPUs and quote the
  counts. Give it its held stop line in `store/batch_run/first-stop-2026-10-28.tsv` if the batch run reads it (Sprint
  350's `shared_close`).
- **Not here:** the woken mark; any lock.

## prerequisites

- 352 (closed)

## context_files

- `crates/w2m_vm/src/threads.rs` (`set_budget`, `take_a_thread`, `slice_until`, every write to a monitor-data offset)
  and `crates/w2m_vm/src/run.rs` (the run's start near 983-998, the turn's start near 1464, 1358, 1877).
- `crates/w2m_vm/src/host_request.rs` 540-550 and the monitor-data offsets' definitions
  (`grep -n 'SLICE_UNTIL\|RUN_END\|CURRENT_ENTRY\|SLICE_COUNT\|SLICE_PAUSES\|LOCK_OWNER\|MXCSR_OFF\|FCW_OFF' crates/w2m_vm/src/*.rs crates/guest_runtime/src/*.rs`).
- Sprint 351's result (`sprints/sprint-351-the-family-runs-on-two-vcpus.md`), its fix and its diagnosis.
- `tests/data/timed_waits/shared_close.c` and its test in `crates/w2m_vm/tests/run_loop.rs`, for a w2m-only guest's
  form and its held stop.
- `WORKING_AGREEMENT.md` lines 137-266, "Build and verification commands".

## signal contract

### Emits

- None; which per-vCPU words the host writes, and when.

### Invariants

- On one vCPU every run, the batch run, the exit ledger and the family runs pass unchanged.
- The 273 family runs on two vCPUs pass as at Sprint 351.

## artifact contract

### Files modified

- `crates/w2m_vm/src/threads.rs` and `run.rs` as the fixes need, the new guest and its test, and any held line the
  gates name.

### Command exit codes

- `scripts/gates.sh` returns 0
- `target/release/signal_report --check` returns 0 on the tree

## observation contract

- **Expected values:**
  - The enumeration, one line a word.
  - Each stale word found, its evidence and its fix.
  - The guest's counts over 20 runs on two vCPUs, before any fix and after.
- **Planted faults:**
  - (1) `take_a_thread`'s `set_budget` call removed. The guest's test fails, a thread's count 0 or the run's limit.
  - (2) Each further fix this card makes taken out in turn. The guest's test, or a test the fix adds, fails.

  Before any fix, read the judge's reach: plant 1 on today's code.

## done criteria

Every per-vCPU word the host writes is listed with whether it depends on run-wide state; every one that could go
stale on another vCPU is fixed; and a guest of three busy threads on two vCPUs shows every thread sliced and the run
ending inside its limit.

## self-review checklist

- [x] Scope names one verifiable property: no per-vCPU word stale on another vCPU, slicing past vCPU 0 judged.
- [x] context_files cite the writers, Sprint 351's fix and the guest's form.
- [x] Emits: none.
- [x] Artifact contract gradable by command.
- [x] Observation contract: the enumeration, the fixes, the counts; numbered plants, their reach read.
- [x] Sweet spot: `threads.rs` and `run.rs` with one guest; one concept, the per-vCPU words.

## result

GATES-LINE
