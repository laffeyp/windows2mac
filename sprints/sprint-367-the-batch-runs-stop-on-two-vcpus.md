# Sprint 367 — The batch run's stop on two vCPUs, on the wall clock

```yaml
---
id: 367
status: active
phase: 6
pass_kind: functional
---
```

## scope

The Decision "Step 6's order after G2, weighed again at Sprint 366" puts the batch run's stop on two vCPUs first, so
that G3 (Sprint 368: the game's batch run on two vCPUs, several runs, conforming as on one) can judge C2, C5 and C3
before they are built. Today the batch run cannot run on two vCPUs: `w2m-vm --game` takes the instruction clock
(`crates/w2m_vm/src/bin/w2m-vm.rs` 345-348, 439-441: "The game's start keeps 1"), which the run loop refuses above one
vCPU since Sprint 365; and the batch run's stop (`crates/w2m_vm/src/batch_stop.rs` 1-17) is "points on the
instruction clock": it is asked after each host request, and its fallback `REACH_AFTER_LOG` counts instructions since
the game's log reached the reference's 168 lines.

- **The game on two vCPUs.** `w2m-vm --game ... --vcpus 2` runs the batch run on the wall clock, as a family's start
  does above one vCPU (Sprint 360), naming the fallback on stderr; on one vCPU it keeps the instruction clock and its
  stop exactly as today.
- **The stop by the reference's own rule.** On several vCPUs the stop keeps `run_frontier.py`'s `stop_when` (the log
  at the reference's line count and the main thread's calls), asked after each host request whichever vCPU serves it,
  its state read and written under the run's lock. State what `since_log` and `REACH_AFTER_LOG` count on several vCPUs
  (the run's instructions summed over its vCPUs, the main thread's alone, or another measure), with grounds from the
  frozen host's `stop_when` (`run_frontier.py` 284-310) and `batch_stop.rs`' own comment, and how a main thread that
  spins after the log fills is still bounded (reviewer-3's Low 3 of its review of Sprint 219, as `batch_stop.rs` 11-13
  quotes it).
- **The class first.** List every reader of `batch_stop`, of `start.instruction_clock` on the game's path, and of the
  game's single-vCPU assumptions (`w2m-vm.rs`' `game`, `batch_run.rs`, `batch_record.rs`, the exit ledger, the
  translation cache's setup, `call_compare`), with file:line and what each does on two vCPUs after this card. Report
  each defect found to main at once with its evidence; fix any that can stop a run or leave a thread past its deadline
  here.
- **Not here:** G3's several runs and their conformance (Sprint 368). One two-vCPU run reaching the stop is this
  card's; whether its call stream conforms is the next card's, read under the user's Decision of 2026-10-13.

## prerequisites

- 366 (closed)

## context_files

- `crates/w2m_vm/src/batch_stop.rs`: in full.
- `crates/w2m_vm/src/bin/w2m-vm.rs` 280-480 (the options, the family start's fallback, `game`), `crates/w2m_vm/src/run.rs`
  (`instruction_clock_allowed`, `run`, the stop's call at 2019-2022, the run's instruction count),
  `crates/w2m_vm/src/host_request.rs` (where a request's serve asks the stop).
- `crates/w2m_vm/tests/batch_run.rs`, `crates/w2m_vm/tests/batch_record.rs`, `store/batch_run/first-stop-2026-10-28.tsv`,
  `store/batch_run/main-thread-calls-2026-10-29.tsv`, and `scripts/gates.sh` 155-200 (the batch run's steps).
- The frozen host's `scripts/run_frontier.py` 60-80 and 284-310 (`REACH_AFTER_LOG`, `stop_when`), read as source.
- `grep -n "Step 6's order after G2, weighed again at Sprint 366\|The instruction clock on several vCPUs\|The child on the run's vCPUs" BLACKBOARD.md`:
  the Decisions.
- `WORKING_AGREEMENT.md` lines 137-266, "Build and verification commands".

## signal contract

### Emits

- None: where the batch run stops on two vCPUs; no emit site changes. The one-vCPU batch run's records are unchanged.

### Invariants

- One vCPU unchanged: the batch run stopping where `store/batch_run/first-stop-2026-10-28.tsv` holds it today, on the
  instruction clock; the 273 family runs on one vCPU and two as held; the two-process guests; the exit ledger.

## artifact contract

### Files modified

- `crates/w2m_vm/src/batch_stop.rs`, `crates/w2m_vm/src/bin/w2m-vm.rs`, with others only where the class list
  requires, reported to main when found. One concept: the batch run's stop on several vCPUs.

### Command exit codes

- `scripts/gates.sh` returns 0
- `target/release/signal_report --check` returns 0 on the tree

## observation contract

- **Expected values:**
  - The class list.
  - The batch run on one vCPU stopping at its held row, quoted.
  - The batch run on two vCPUs on the wall clock (`w2m-vm --game ... --vcpus 2`) reaching `Stop::ReferenceEnd` by the
    reference's rule in at least one run, its stop line and its time quoted, every process on two vCPUs.
  - The stop's decision unit-tested on several vCPUs' counts (the `decide` form), quoted.
- **Planted faults:**
  - (1) Today's form: the game's start refused above one vCPU, or kept on the instruction clock, so the two-vCPU run is
    refused by name, and the two-vCPU stop test fails.
  - (2) The stop asked on one vCPU's requests alone, or its fallback counting one vCPU's instructions, as the class
    list shows the fault the two-vCPU run would meet; its judge names it. If a judge catches a plant only sometimes,
    report that to main before going on.

## done criteria

The game's batch run runs on two vCPUs on the wall clock and stops by the reference's rule; on one vCPU it stops where
it stops today.

## self-review checklist

- [x] Scope names one verifiable property: the batch run's stop on two vCPUs, one vCPU unchanged.
- [x] context_files cite the stop, the game's start, the run loop, the batch tests and the frozen host's rule.
- [x] Emits: none, with its reason.
- [x] Artifact contract gradable by command.
- [x] Observation contract: the one-vCPU stop, a two-vCPU stop, the decision's test; numbered plants.
- [x] One concept: the batch run's stop on several vCPUs.

## result

GATES-LINE
