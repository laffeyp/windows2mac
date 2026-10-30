# Sprint 224 — The timing stage

```yaml
---
id: 224
status: active
phase: 6
pass_kind: functional
---
```

## scope

The Decision "Sequence after Sprint 214" as annotated at Sprint 222's close (reviewer-3's Medium 2 of its review at
it), and the Drift watchlist's "Absolute timings drift between runs" as annotated in Sprint 222.

- **Where it stands, by lookup (2026-10-29):**
  - `crates/hypervisor/tests/exit_time.rs` 40 asserts an exit and its resume under 5,000 ns. It runs inside the
    gates' nextest run (`scripts/gates.sh` 197 and 201), which stops at its first failure.
  - In Sprint 222's first gates run it measured 56,333 ns at load averages of 189, 276 and 255. Nextest stopped, and
    152 of 224 tests went unrun. Alone it then measured 4,062 ns, and in the fourth run it passed.
  - WORKING_AGREEMENT 115 records 832 to 873 ns from Rust and 813 to 852 ns from `vm_monitor`'s `spin`. The
    test's own message names the C monitor's as 720 ns, a third figure the card reconciles.
- **The work:**
  - The exit time leaves the correctness run. It is measured in its own stage after the correctness tests, its figure
    and the load averages kept in a tracked file beside `multi-vcpu.tsv`. A figure over the bound is reported, as the
    multi-vCPU stage reports, and does not stop the gates.
  - The bound is read against the run's own conditions: the test records the figure and the load averages, and the
    stage names a figure over the bound at a load under a stated level as a failure that stops the gates.
  - The correctness run holds no wall-clock bound: a search of the crates' tests for `elapsed` and `Instant` under an
    assertion names any other.
- **Not here:** the multi-vCPU stage's frozen tests, plan step 6's.

## prerequisites

- 223 (closed)

## context_files

- `crates/hypervisor/tests/exit_time.rs`, `scripts/gates.sh` (the nextest runs, the multi-vCPU stage)
- `store/gates/multi-vcpu.tsv`; `crates/signal_report/src/main.rs` (`multi_vcpu_staged`, the form a stage's record is
  checked in); WORKING_AGREEMENT 115; BLACKBOARD Drift watchlist, "Absolute timings drift between runs"

## signal contract

### Emits

- None; a gates stage, as the multi-vCPU stage.

### Invariants

- Every correctness test runs in every gates run whatever the machine's load.

## artifact contract

### Files modified

- `crates/hypervisor/tests/exit_time.rs`, `scripts/gates.sh`, `store/gates/` (the timing record),
  `crates/signal_report/src/main.rs` if the close checks the stage

### Command exit codes

- `cargo nextest run` and `scripts/gates.sh` return 0

## observation contract

- **Expected values:** the exit time on this machine and the load at which it was taken; the correctness run's test
  count with the timing test gone from it.
- **Planted faults:**
  1. The bound set to 1 ns: the stage reports the figure over it, the gates go on, and the record holds the failure.
  2. The bound set to 1 ns at a load under the stated level: the stage stops the gates, naming the figure.
  3. A second wall-clock assertion added to a correctness test: the search names it.

## done criteria

No wall-clock bound sits in the gates' correctness run. The exit time is measured in its own stage, its figure and
load kept, and a figure over the bound on a quiet machine still stops the gates.

## self-review checklist

- [x] Scope names one verifiable property: no wall-clock bound in the correctness run.
- [x] context_files cite the test, the gates and the stage's record.
- [x] Emits: none, with the reason.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and three plants.
- [x] One concept: where a timing bound is judged.

## result
