# Sprint 108 — vocabulary 0.9: the batch run's numbers on its record

```yaml
---
id: 108
status: done
phase: 5
pass_kind: functional
---
```

## scope

H1 of the SDD hardening order (docs/plan.md; Decision 2026-10-17; `docs/review-2026-10-17-sdd-hardening.md` item 1,
which takes in design sprint D1). One concept: the numbers the batch run is judged by reach its record. Today the
runner (`scripts/run_frontier.py`) gathers them into `state` and prints them to stderr; the record holds only the VM
compiler's counts (Sprint 101). Sprint 094's table was rebuilt from logs in `build/sprint-094/` for that reason.

Proposal P20 (ENUM_EXTENDED, layer 2), as P19 did: `metric` gains eight values, each a measure whose dimension is the
`variant` MEASUREMENT_RECORDED already carries:

| metric | unit | variant | source in the runner |
|---|---|---|---|
| `instructions_retired` | instructions | `process`, `thread <id>` | `r.instructions_executed`; `vm.thread_census()` |
| `pause_instructions` | instructions | `process`, `thread <id>` | `vm.thread_census()` |
| `instructions_per_second` | instructions/s | `process` | instructions over the run's wall seconds (in the VM plus in the host) |
| `run_seconds` | s | `vm`, `host hvc <k>` | `vm.run_ns`, `vm.exit_ns` |
| `vm_exits` | exits | `hvc <k>` | `vm.exits` |
| `thread_switches` | switches | `in the VM`, `host: <cause>` | `vm.switches_in_vm()`, `vm.handovers` |
| `thread_wakes` | threads | `readied`, `rewaits` | `vm.wake_census()` |
| `load_average` | load | `1 min`, `5 min`, `15 min` | `os.getloadavg()` at the runner's end (the start's is in the baseline) |

No value is added that this sprint does not emit (the review's finding 5: 27 of 62 tags never emitted). The system
call, frame and ambient-progress tags wait for the sprints that emit them (H5, step 6).

## prerequisites

- 106 (closed); 107 parked (Decision 2026-10-17).

## context_files

- signals/0.8.json, 0.8-rationale.md, proposals.json, current.json; scripts/gen_events.py; src/w2m/__init__.py;
  scripts/run_frontier.py (`runner`, `state`, `host_load`); src/w2m/vm_backend.py (`thread_census`, `wake_census`,
  `switches_in_vm`, `exits`, `exit_ns`, `run_ns`); sprints/sprint-101-vocabulary-0-8-compile-counts.md.

## signal contract

### Emits

- MEASUREMENT_RECORDED from the batch run's runner, after the compile counts, for each row above: phase `discovery`,
  backend `managed_vm`, artifact_state as Sprint 101's. Expected on the closing batch run:
  - `instructions_retired` `process` equal to the stop line's instruction count, and the `thread <id>` values summing
    to it;
  - `vm_exits` summing to the exits the progress line counts;
  - `run_seconds` `vm` plus the `host hvc` values within 1% of the runner's wall time around `machine.run`;
  - `thread_switches` `in the VM` equal to the printed "switches in the VM".

### Invariants

- Every 0.8 tag and enum value unchanged; guest-plane traces of 0.1-0.8 ingest under 0.9; records of 0.8 stay valid.

## artifact contract

- signals/0.9.json, 0.9-rationale.md, P20 ratified, current.json 0.9, VOCAB_VERSION 0.9, events.py regenerated
  (`--check` 0); `run_measurements(...)` in run_frontier.py, a function of the gathered numbers;
  `tests/test_run_measurements.py`; `scripts/gates.sh` exit 0; CI `success` on the closing commit.

## observation contract

- **Judge:** the batch run's record, read through substrate's `read_record`, holds the emits above with the expected
  values; the Sprint 094-style table (instructions, rate, host seconds by exit) is printed from the record alone.
- **Plant:** `run_measurements` given a thread census short of one thread: the test's sum check fails.

## done criteria

The batch run's deciding numbers are on its record under vocabulary 0.9; CI green.

## observed
- **Made** (018422051, then the reader): signals/0.9.json and its rationale, P20 ratified, current.json 0.9,
  VOCAB_VERSION 0.9, events.py regenerated (`--check` 0, check_vocab: 62 tags, 102 foreign keys resolved);
  `run_measurements`, `measurement_table`, `measurement_problems` in run_frontier.py; `VM.pauses_executed` (the
  monitor's PAUSE count, apart from the thread census, so the PAUSE sum is a check and not an identity);
  `scripts/run_numbers.py`; `tests/test_run_measurements.py`. 0.9's `rationale` names its own file; 0.6 to 0.8 had
  left it at 0.5's.
- **Judge** (frontier-643a6283510c-1792267359516238000, on 018422051; the batch run conforms: the log's 168 lines,
  0 divergences, 24 values Windows varies): `run_numbers.py` reads 107 numbers from the record through substrate and
  finds 0 that do not add up. The process retired 82,709,305,055 instructions, the sum of its 28 threads' (thread 4,
  the main thread, 56,199,454,901); 83,934,637 PAUSEs, 83,934,283 of them thread 4's; 31.159 s in the VM and 8.316 s
  in the host (hvc 8 7.305 s, hvc 13 0.458 s, hvc 1 0.402 s), 2,095,145,119 instructions a second over those
  39.48 s; exits hvc 8 44,010, hvc 13 13,673, hvc 1 1,091, hvc 11 286, hvc 10 258, hvc 14 167, hvc 4 8, hvc 12 2;
  762,122 switches in the VM and 2 host hand-overs; 181,421 threads readied and 53 rewaits; the load 4.13, 3.35,
  3.24 at the end. Each equals the run's printed line.
- **Not met as written:** the card expected the seconds within 1% of the runner's wall time around `machine.run`;
  the runner does not time `machine.run`, so the check was not made. The printed rate (1,978.30 M a second over
  41.8 s) divides by the session's time, set-up and comparison included; the record's by the run's 39.48 s.
- **Plant:** a thread census short of thread 8: `measurement_problems` reports the instructions (1000 against 700)
  and the PAUSEs (4 against 3); `tests/test_run_measurements.py`.
- **No change elsewhere** (e1cbc46f): 456 of 458 classes (FSIN and FCOS open, as before; 94 s), 68 of 68 families,
  every one of the 526 records stamped with the clean commit; `scripts/gates.sh` exit 0, 32,755 records intact.
