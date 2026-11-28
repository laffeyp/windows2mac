# SIGNAL_REPORT — Sprint 376 — Laptop game runs with the Steam client not running, joining G3's reference set

Made by `signal_report 376` from 3 records under store/runs/ whose baseline names one of the sprint's 3 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 376.

## 1. Observed

### scope_confirmation

> Sprint 372's three game executions ran with the Steam client signed in on the laptop, unlike the ten reference runs
> of 2026-10-14, and are not comparable (their `unity.log` replaces "[Steamworks.NET] SteamAPI_Init() failed" with the
> game's Steam lines and the account's player name; kept in `build/game-runs-steam-on/`). The Decision "A laptop game
> run's initial state includes the Steam client" requires the next game job to state the Steam client's state and check
> it before the game starts. This card makes those runs, the first new laptop runs Sprint 375's ported rules must
> admit to G3's reference set (the Decision "The order after Sprint 375").

### work_performed

- 33b6423b3 Sprint 376 card amendment: the game's TEMP the user's own, the path the reference runs and w2m give it, with the game's own TEMP files removed before each execution; a run whose game read another TEMP is not stored
- 24284d8f1 Sprint 376 card amendment: the Steam client stopped for the game job and checked absent before each execution, started again after; an execution whose unity.log shows the Steam client running is not stored
- b8b382218 Sprint 376 card: laptop game runs with the Steam client not running, under the reference runs' initial state, admitted to G3's reference set by its rules

### signal_trace

```
game-run-lap-20261128T014720644Z-33b6423b-01: SESSION_INIT 1, EXECUTION_STARTED 1, EXECUTION_COMPLETED 1, SESSION_COMPLETED 1; producers failed 0; not finalised 0
  t=    0.007  game_run -> SESSION_INIT (contract_version=0.14, harness_version=0.1.0, profile=windows_reference, purpose=application, vocab_version=0.14)
  t=    0.008  game_run -> EXECUTION_STARTED (execution_id=game-run-lap-20261128T014720644Z-33b6423b-01, initial_state_sha256=sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4..., injected_inputs_sha256=sha256:e3b0c44298fc1c149afbf4c8996
  t=    0.008  game_run -> EXECUTION_COMPLETED (elapsed_seconds=153.0254793, execution_id=game-run-lap-20261128T014720644Z-33b6423b-01, exit_code=None, exit_status=timed_out, guest_line_count=3498, guest_trace=None, trace_sha256=sha256
  t=    0.008  game_run -> SESSION_COMPLETED (accepted_count=0, discrepancy_count=0, elapsed_seconds=0.001, outcome=completed)
game-run-lap-20261128T014720644Z-33b6423b-02: SESSION_INIT 1, EXECUTION_STARTED 1, EXECUTION_COMPLETED 1, SESSION_COMPLETED 1; producers failed 0; not finalised 0
  t=    0.007  game_run -> SESSION_INIT (contract_version=0.14, harness_version=0.1.0, profile=windows_reference, purpose=application, vocab_version=0.14)
  t=    0.007  game_run -> EXECUTION_STARTED (execution_id=game-run-lap-20261128T014720644Z-33b6423b-02, initial_state_sha256=sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4..., injected_inputs_sha256=sha256:e3b0c44298fc1c149afbf4c8996
  t=    0.007  game_run -> EXECUTION_COMPLETED (elapsed_seconds=153.0389971, execution_id=game-run-lap-20261128T014720644Z-33b6423b-02, exit_code=None, exit_status=timed_out, guest_line_count=3495, guest_trace=None, trace_sha256=sha256
  t=    0.007  game_run -> SESSION_COMPLETED (accepted_count=0, discrepancy_count=0, elapsed_seconds=0.001, outcome=completed)
game-run-lap-20261128T014720644Z-33b6423b-03: SESSION_INIT 1, EXECUTION_STARTED 1, EXECUTION_COMPLETED 1, SESSION_COMPLETED 1; producers failed 0; not finalised 0
  t=    0.007  game_run -> SESSION_INIT (contract_version=0.14, harness_version=0.1.0, profile=windows_reference, purpose=application, vocab_version=0.14)
  t=    0.007  game_run -> EXECUTION_STARTED (execution_id=game-run-lap-20261128T014720644Z-33b6423b-03, initial_state_sha256=sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4..., injected_inputs_sha256=sha256:e3b0c44298fc1c149afbf4c8996
  t=    0.007  game_run -> EXECUTION_COMPLETED (elapsed_seconds=153.0256377, execution_id=game-run-lap-20261128T014720644Z-33b6423b-03, exit_code=None, exit_status=timed_out, guest_line_count=3499, guest_trace=None, trace_sha256=sha256
  t=    0.007  game_run -> SESSION_COMPLETED (accepted_count=0, discrepancy_count=0, elapsed_seconds=0.001, outcome=completed)
```

### signal_trace_tags

- `EXECUTION_COMPLETED`
- `EXECUTION_STARTED`
- `SESSION_COMPLETED`
- `SESSION_INIT`

## 2. Expected

> - `EXECUTION_STARTED` and `EXECUTION_COMPLETED` for each of the three stored game executions, through Sprint 375's
>   record step.

## 3. Delta

### emits_check

- `EXECUTION_COMPLETED`: fired 3 times
- `EXECUTION_STARTED`: fired 3 times
- fired, not named in Emits: `SESSION_COMPLETED` (3), `SESSION_INIT` (3)

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 376: its Rubber Duck Pass and close.
