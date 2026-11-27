# Sprint 368 — G3's runnable part: the game's batch run on two vCPUs, conforming as on one

```yaml
---
id: 368
status: active
phase: 6
pass_kind: functional
---
```

## scope

G3 (`docs/step-6-census-2026-10-30.md` 20): "the batch run on the wall clock on two vCPUs conforming, in several runs".
The Decision "Step 6's order after G2, weighed again at Sprint 366" puts it here, before C2's build cards, so that the
game itself judges every change after it; G3 by Sprint 370. Sprint 367 made the batch run run on two vCPUs on the wall
clock and stop by the reference's rule, in two runs.

- **The judge.** "Conforming as on one vCPU" is `call_compare`'s verdict (`harness/w2m-vm-game.toml`, the game's
  calls against the Windows reference's) under the user's Decision of 2026-10-13: a VALUE divergence (the same call on
  the paired thread with the same inputs answered differently) is a defect; a presence or count divergence on a
  thread whose earlier calls all conform is SCHEDULING, recorded and not chased, unless an earlier value divergence
  explains it; the contract's `undetermined` entries (`contracts/windows.game.calls/0.1.0.json`) stand as they are. Any
  new `undetermined` entry a two-vCPU run calls for names its determinant with its evidence and is a contract change,
  recorded as one, never a widening of the judge to let a run pass.
- **Several runs.** Five batch runs on two vCPUs through the harness (`scripts/batch-run-record.sh` with the run's vCPU
  count), each record held by `batch_record.rs` and `check_records`, each stop at the reference's row, and each
  `call_compare` verdict read under the rule above; the verdicts quoted, every divergence sorted as value or
  scheduling with its thread and call.
- **G3 in the gates.** `scripts/gates.sh` gains one batch run on two vCPUs beside today's one-vCPU run, its record and
  `call_compare` verdict held the same way, so every later card's gates judge the game on two vCPUs.
- **Sprint 367's open item.** After a run, `bin/w2m-vm.rs` 1247 `current_tid` and 1262 `called_service` read
  `guest.served.vcpu`, the last vCPU served, not the stopper's, so an Unserved stop on two vCPUs may name another
  thread's call: they read the stopping vCPU's.
- **Every failure found is fixed here**, under the user's Decision of 2026-10-18 where it can stop a run or leave a
  thread past its deadline; report each to main at once with its evidence.
- **The class first.** List every reader of the batch run's record, verdict and stop that assumes one vCPU (the
  harness file, `batch-run-record.sh`, `batch_record.rs`, `check_records`, `call_compare`, the gates' batch step), with
  file:line, and what each does on two vCPUs after this card.

## prerequisites

- 367 (closed)

## context_files

- `docs/step-6-census-2026-10-30.md` (G3's row), `harness/w2m-vm-game.toml`, `scripts/batch-run-record.sh`,
  `scripts/gates.sh` 155-220.
- `crates/w2m_vm/tests/batch_run.rs`, `crates/w2m_vm/tests/batch_record.rs`, the `call_compare` and `check_records`
  binaries' sources, `contracts/windows.game.calls/0.1.0.json`.
- `crates/w2m_vm/src/bin/w2m-vm.rs` 1080-1280 (the game's run and its stop file), `crates/w2m_vm/src/batch_stop.rs`.
- `grep -n "2026-10-13 user\|Step 6's order after G2, weighed again at Sprint 366\|The batch run's stop on several vCPUs" BLACKBOARD.md`:
  the Decisions.
- `WORKING_AGREEMENT.md` lines 137-266, "Build and verification commands".

## signal contract

### Emits

- None: the game's batch run on two vCPUs through the existing harness and its Producers; no emit site changes. The
  one-vCPU run's records are unchanged; a two-vCPU run's record carries `vcpus` 2 (Sprint 367).

### Invariants

- One vCPU unchanged: the batch run stopping at its held row and its `call_compare` verdict as today; the 273 family
  runs on one vCPU and two as held; the two-process guests; the exit ledger.

## artifact contract

### Files modified

- `scripts/gates.sh` (the two-vCPU batch run), `crates/w2m_vm/src/bin/w2m-vm.rs` (the stopper's thread), and the
  harness or tests only where the class list requires, reported to main when found. One concept: the game judged on
  two vCPUs.

### Command exit codes

- `scripts/gates.sh` returns 0
- `target/release/signal_report --check` returns 0 on the tree

## observation contract

- **Expected values:**
  - The class list.
  - Five two-vCPU batch runs: each stop line and time, each record's `vcpus`, each `call_compare` verdict quoted, and
    each divergence sorted as value or scheduling under the user's Decision of 2026-10-13, with its thread and call.
  - The gates' two-vCPU batch run and its held verdict.
  - The one-vCPU batch run unchanged.
- **Planted faults:**
  - (1) A value fault the game meets on two vCPUs: one service answering a value that differs from the reference's
    (named in the result, chosen from the calls the runs make); the two-vCPU run's `call_compare` verdict names it as
    a value divergence, and the gates fail.
  - (2) The stopper's thread read from the last vCPU served, today's form; a test of an Unserved stop on two vCPUs
    names the wrong thread and fails.

## done criteria

The game's batch run conforms on two vCPUs in five runs, by `call_compare` under the user's Decision of 2026-10-13,
and the gates judge it on two vCPUs at every card from here; one vCPU is unchanged.

## self-review checklist

- [x] Scope names one verifiable property: the game conforming on two vCPUs in several runs.
- [x] context_files cite G3's row, the harness, the judge, the contract and the Decisions.
- [x] Emits: none, with its reason.
- [x] Artifact contract gradable by command.
- [x] Observation contract: five runs sorted under the rule, the gates' run; numbered plants.
- [x] One concept: the game judged on two vCPUs.

## result

GATES-LINE
