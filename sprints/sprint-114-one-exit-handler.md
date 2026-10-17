# Sprint 114 — one exit handler

```yaml
---
id: 114
status: done
phase: 5
pass_kind: architecture
---
```

## scope

Step 1, the chain's first sprint (docs/design-several-vcpus.md, "The host for several vCPUs, designed twice"). One
concept: the body of `VM.steps`'s loop, which serves one exit, becomes `VM.serve_exit(run, exit, cpsr) -> Resume |
Stop | Slice | Switched`; `steps` keeps the run's setup, the pre-run words, the monitor call and the yields to a
kernel.Machine (`Slice`: another VM's turn; `Switched` with IDLE: no thread of this VM can run). `RunState` holds what
the loop's exits read and change (the hooks, max_steps, forced_stop, last_kind). No behaviour changes: the body moves
as written, each `continue` a `Resume`, each `break` a `Stop`.

## prerequisites

- 113.

## context_files

- src/w2m/vm_backend.py (`steps`); docs/design-several-vcpus.md; tests/test_switch_in_vm.py (the census judges).

## signal contract

### Emits

- On the batch run's record, as on Sprint 110's (frontier-643a6283510c-1792269281973223000): SESSION_INIT,
  EXECUTION_STARTED, MEASUREMENT_RECORDED (the compile counts and P20's numbers, 107 on that run), EXECUTION_COMPLETED,
  TRACE_INGESTED, COMPARISON_COMPLETED (0 divergences), SESSION_COMPLETED. Per class and family: their item runs'
  tags as on Sprint 112's sweep.

### Invariants

- Vocabulary 0.9 unchanged.

## artifact contract

- `serve_exit`, `RunState` and the four outcomes in vm_backend.py; `scripts/gates.sh` exit 0; CI `success` on the
  closing commit.

## observation contract

- **Judge:** the batch run conforms (168 lines, 0 divergences) and `check_records.py` finds 0 violations on it; its
  `vm_exits` by kind are the same kinds as Sprint 110's run; 456 of 458 classes and 68 of 68 families;
  `tests/test_switch_in_vm.py`'s census equalities (the same rows and per-thread instruction and PAUSE counts with
  the switch in the VM and in the host, on the instruction clock) hold.
- **Plant:** none of its own: a refactor with no new behaviour is judged by the existing judges, each of which has
  its plant.

## done criteria

One exit handler serves every exit of a run; behaviour unchanged by every judge; CI green.

## observed
- **Made** (123de94c): `VM.serve_exit`, `RunState`, `Resume`, `Stop`, `Slice`, `Switched`; `steps`'s loop calls it.
- **Judge:** the batch run (frontier-643a6283510c-1792273674526334000) conforms: the log's 168 lines at
  80,821,897,382 instructions, 0 divergences; `check_records.py` 0 violations; `run_numbers.py` 107 numbers, 0 that do
  not add up; its exits the same eight kinds as Sprint 110's run (hvc 1 1,091, 4 8, 8 39,979, 10 258, 11 286, 12 2,
  13 13,646, 14 167). 456 of 458 classes (95 s), 68 of 68 families. Before the commit, the 65 tests of the VM's
  files passed, `tests/test_switch_in_vm.py`'s census equalities among them.
