# Sprint 110 — the frontier's execution placed in time on its record

```yaml
---
id: 110
status: done
phase: 5
pass_kind: functional
---
```

## scope

H2 of the SDD hardening order, its second half (docs/plan.md; Decision 2026-10-17; Deferred 2026-10-17, Sprint 109).
One concept: the frontier's record says when its execution ran, how long, and within what bound.

- **The run off the event loop.** The runner (`scripts/run_frontier.py`) calls `machine.run` inside its async
  generator, which blocks asyncio's loop for the whole run. Substrate's `_submit_emission` only puts an event in the
  inbox (`substrate/src/substrate/kernel/runtime.py:780`) and `_writer_loop` stamps and writes it when the loop runs,
  so every event of the run is stamped at its end: on frontier-643a6283510c-1792267359516238000 the runner's
  ProducerStarted and EXECUTION_STARTED carry t = 1792267400.96, 41 s after the harness finished. The run moves to a
  worker thread (`asyncio.to_thread`), at user-interactive QoS as `main` sets for its own thread (Sprint 068 item F).
- **A wall-time bound, stated.** The frontier has none; T3 cannot apply to its 46 records with an EXECUTION_STARTED
  and no timeout_seconds, and 43 frontier records have no completion at all. `--timeout-seconds` (default 10,800:
  the longest frontier on record took 8,013 s, frontier-643a6283510c-1791802134771149000, and the median 95 s) stops
  the run through `machine.stop_when`, which the machine reads once a second (STOP_CHECK_SECONDS); kill_grace_seconds
  60. EXECUTION_STARTED carries both.
- **The completion's measures.** EXECUTION_COMPLETED's `elapsed_seconds` is the wall time around `machine.run`, and
  `exit_code` the process's when it exited; `unobserved` names only what is still not observed.

## prerequisites

- 109.

## context_files

- scripts/run_frontier.py (`runner`, `main`, `interactive_qos`, `HARNESS_STOPS`); src/w2m/kernel.py (`Machine.run`,
  `stop_when`); substrate/src/substrate/kernel/runtime.py (`_submit_emission`, `_writer_loop`); signals/0.9.json
  (EXECUTION_STARTED, EXECUTION_COMPLETED, T3); scripts/check_records.py.

## signal contract

### Emits

- On the batch run's record: EXECUTION_STARTED with `timeout_seconds` 10800 and `kill_grace_seconds` 60, its `t`
  within 5 s of the harness's SESSION_INIT; EXECUTION_COMPLETED with `elapsed_seconds` the run's wall time, its `t`
  that many seconds after EXECUTION_STARTED's, within 2 s; the MEASUREMENT_RECORDED of Sprint 108 unchanged in kind.

### Invariants

- Vocabulary 0.9 unchanged; the batch run conforms as before (168 lines, 0 divergences).

## artifact contract

- run_frontier.py's runner and `--timeout-seconds`; `scripts/gates.sh` exit 0 with `check_records.py`; CI `success`
  on the closing commit.

## observation contract

- **Judge:** `check_records.py` on the closing batch run's record: T3 applied and checked (not unchecked), 0
  violations; the `t` of EXECUTION_STARTED and EXECUTION_COMPLETED as the Emits state; Sprint 108's card's unmet
  check now made: `run_seconds` `vm` plus the `host` values within 1% of `elapsed_seconds`.
- **Plant:** a batch run with `--timeout-seconds 5`: it stops as `killed` with the stop "timeout", EXECUTION_COMPLETED
  within 5 + 60 s of EXECUTION_STARTED, and T3 holds.

## done criteria

The frontier's record places its execution in time and within a stated bound, checked by T3; CI green.

## observed
- **Made** (ff2fe2b4): the run in a worker thread (`asyncio.to_thread`, the worker at user-interactive QoS);
  `--timeout-seconds` (default 10,800) and a 60 s kill grace on EXECUTION_STARTED, the bound stopping the run as
  `timed_out`; `elapsed_seconds` on EXECUTION_COMPLETED; the completion emitted after SERVICE_UNIMPLEMENTED too, and
  the trace ingested on the completion alone (with both kinds on the record the ingest would have fired twice).
- **Judge** (frontier-643a6283510c-1792269281973223000; conforms: 168 lines, 0 divergences, 81,743,163,703
  instructions): `check_records.py` T1 to T4 applied, T3 checked, 0 violations. EXECUTION_STARTED at 0.207 s after
  SESSION_INIT (before: 41 s, at the run's end); EXECUTION_COMPLETED at 33.965 s with `elapsed_seconds` 33.546, its
  `t` 0.212 s after start plus elapsed. Sprint 108's unmade check, made: `run_seconds` sum to 33.29 s, 0.76% under
  `elapsed_seconds`.
- **Plant** (`--timeout-seconds 5 --scratch`, runs/frontier-643a6283510c-1792269317173502000): stopped "timed out
  after 5 s" at 10,326,297,451 instructions; EXECUTION_COMPLETED `timed_out`, `elapsed_seconds` 5.164, 6.647 s after
  EXECUTION_STARTED (within 5 + 60 s); T3 holds.
- **Found:** 41 of the 43 frontier records with no completion had stopped at SERVICE_UNIMPLEMENTED, which ended the
  runner before EXECUTION_COMPLETED; the runner now emits both. `exit_code` stays absent on the batch run: the run
  is stopped by the harness (`killed`), and the game never exits.
- **The card's plant, amended:** it expected the bound's stop as `killed`; the vocabulary's own `timed_out` ("the
  harness killed it after timeout_seconds", EXECUTION_COMPLETED's note) is the one emitted.
- **A gate that failed by chance:** the first gate run failed `test_planted_entso_off_shows_mp_forbidden_outcome`.
  Measured: 3 of 25 runs show no forbidden MP outcome with EnTSO off, the others 1 to 11 in 100,000, so Sprint 105's
  three runs missed about one time in 600 (its "near one in a billion" took the count for Poisson). Now up to ten
  runs (about 6 in 10 billion); `tests/test_litmus.py`.
