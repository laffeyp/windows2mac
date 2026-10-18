# Sprint 115 — the Vcpu object, passed

```yaml
---
id: 115
status: done
phase: 5
pass_kind: architecture
---
```

## scope

Step 1, the chain's second sprint (docs/design-several-vcpus.md, "The host for several vCPUs, designed twice"). One
concept: what belongs to one vCPU is a `Vcpu` object, and every handler that reads it takes it as an argument. Today
`VM.serving`, `VM._md`, `VM._st`, `VM._vswap` and `VM._channel` hold the served vCPU's index, the file offsets of its
monitor data page and state block, its pending vector swap and its channel; `VM.serve(i)` would switch them and has no
caller, and `VM.vcpu_channels` holds vCPUs 1.. apart from vCPU 0. 92 sites in 38 methods read them, and 11 more
methods reach them through a call (the call graph over `VM`'s methods: 49 in all).

- **`class Vcpu`** (vm_backend.py): `index`, `channel`, `md`, `st`, `vswap`; and the accessors of its own pages and
  channel, moved from `VM`: `counts`, `slice_start`, `vectors`, `set_vectors`, `flush_swap`, `ymm_hi`, `set_ymm_hi`,
  `x87`, `set_x87`, `mxcsr`, `set_mxcsr`, `af`, `df`, `pf`, `set_af_df`. vCPU 0 with no channel (between runs) speaks
  through the monitor's pipe, as `VM.vectors` does today.
- **`VM.vcpus`**: vCPU 0 from construction; vCPUs 1.. from `_start_vcpus`, each with its channel (in place of
  `vcpu_channels`). `serving`, `serve`, `_md`, `_st`, `_vswap`, `_channel` and `vcpu_channels` go.
- **Handlers take the `Vcpu`**: `serve_exit(run, vcpu, exit, cpsr)` and every method below it that reads a vCPU's
  state (`_schedule`, `_switch`, `_save_thread`, `_load_thread`, `_host_request`, `_thread_request`, `_create_thread`,
  `_stack_grow`, `_set_budget`, `_slow_sse`, `_raise_in_guest`, `_traced_proc`, ...). `current_tid` becomes
  `current_tid(vcpu)` and `set_current(vcpu, tid)`.
- **Process-wide readers name vCPU 0**: `instructions_executed`, `pauses_executed`, `thread_census`, `_init_process`
  and the hooks' readers in `scripts/vm_oracle.py` and the tests read `vm.vcpus[0]`, the one vCPU that runs the
  process's threads until chain step 5.

No behaviour changes: one vCPU serves every run, as before.

## prerequisites

- 114.

## context_files

- src/w2m/vm_backend.py (`VM.__init__`, `serve`, `_start_vcpus`, `steps`, `serve_exit`, the 49 methods);
  docs/design-several-vcpus.md; scripts/vm_oracle.py; scripts/run_frontier.py; tests/test_thread_switch_vectors.py,
  test_vm_state.py, test_litmus.py, test_vcpus.py, test_runtime_lock.py, test_call_trace.py, test_call_compare.py.

## signal contract

### Emits

- On the batch run's record, as on Sprint 114's (frontier-643a6283510c-1792273674526334000): SESSION_INIT
  (vocab_version 0.9, purpose application); EXECUTION_STARTED (timeout_seconds 10800, kill_grace_seconds 60);
  MEASUREMENT_RECORDED 107 times (23 `vm_compile_blocks`, 84 of P20's metrics); EXECUTION_COMPLETED (exit_status
  killed, exit_code absent); TRACE_INGESTED (thread_count 28); COMPARISON_COMPLETED (first_divergence none);
  SESSION_COMPLETED (outcome completed, discrepancy_count 0).
- Per class run (458): SESSION_INIT, CANDIDATE_GENERATED, COMPARISON_COMPLETED (2 each), then CANDIDATE_ACCEPTED
  (456) or, for FCOS and FSIN, REDUCTION_COMPLETED, DISCREPANCY_REPORTED and DISCREPANCY_ROUTED (8 each),
  SESSION_COMPLETED. Per family run (68): SESSION_INIT, CANDIDATE_GENERATED, COMPARISON_COMPLETED, CANDIDATE_ACCEPTED,
  SESSION_COMPLETED.

### Invariants

- Vocabulary 0.9 unchanged.
- No attribute of `VM` names a served vCPU: `grep -n 'self\.serving\|self\._md\b\|self\._st\b\|self\._vswap\|self\._channel\|vcpu_channels' src/w2m/vm_backend.py` finds nothing.

## artifact contract

- `class Vcpu` and `VM.vcpus` in vm_backend.py; the callers changed; `scripts/gates.sh` exit 0; CI `success` on the
  closing commit.

## observation contract

- **Judge:** the batch run conforms (168 lines, 0 divergences, its instruction count 80,821,897,382 as on Sprint 114's
  run) and `check_records.py` finds 0 violations on it; its `vm_exits` the same eight kinds; 456 of 458 classes and 68
  of 68 families; `tests/test_switch_in_vm.py`'s census equalities, the litmus tests on vCPUs 1-4 and the runtime-lock
  test on vCPUs 1 and 2 pass.
- **Plant:** none of its own: a refactor with no new behaviour is judged by the existing judges, each with its plant.

## done criteria

Every handler that reads a vCPU's state takes its `Vcpu`; no `VM` attribute names the served vCPU; behaviour
unchanged by every judge; CI green.

## observed
- **Made** (076b3225): `class Vcpu` and `VM.vcpus`; `serve_exit(run, vcpu, exit, cpsr)` and the methods below it
  take the `Vcpu`; `serve`, `serving`, `_md`, `_st`, `_vswap`, `_channel` and `vcpu_channels` gone (the invariant's
  grep finds nothing); the oracle's judges take a `Vcpu`; four patched methods in `tests/test_instruction_count.py`
  given the new signatures (found by the first test run, whose failure a pipe to `tail` reported as exit 0).
- **Judge:** the batch run (frontier-643a6283510c-1792295964556678000) conforms: the log's 168 lines, 0 divergences;
  `check_records.py` 0 violations; `run_numbers.py` 107 numbers, 0 that do not add up; its exits the same eight kinds
  (hvc 1 1,091, 4 8, 8 43,562, 10 258, 11 286, 12 2, 13 13,673, 14 167). 456 of 458 classes (65 s; FCOS and FSIN
  as before), 68 of 68 families. Before the commit, the 34 VM test files passed (185 tests, the census equalities,
  the litmus tests and the runtime-lock test among them).
- **The card's count, amended:** it expected Sprint 114's 80,821,897,382 instructions; this run stopped at
  82,507,774,218. The batch run stops when the log holds the reference's 168 lines, a point that moves between runs
  (Sprint 110's run: 81,743,163,703), so the count is not a judge of unchanged behaviour; the lines, the divergences
  and the exit kinds are.
