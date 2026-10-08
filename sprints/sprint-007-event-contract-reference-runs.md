# Sprint 007 — reference runs for the auto-reset event contract

```yaml
---
id: 007
status: active
phase: 1
pass_kind: functional
---
```

## scope

Author `probes/event_auto_reset/event_auto_reset.c`, the probe for `windows.event.auto_reset.single_signal@0.1.0`: main creates an auto-reset event non-signaled, starts waiters a and b (each `WaitForSingleObject(event, 2000)`), calls `SetEvent` once after both are about to wait, joins them, then calls `WaitForSingleObject(event, 0)`. It writes a guest trace (GUEST_CALL_RETURNED lines with SHA-256 `args_sha256` via BCrypt; hb edge SetEvent → released waiter; hb edges waiter exit → join) and a `roles.json` role map. The profile_id records build, UBR, and installation type from the registry. Extend `.github/workflows/windows-probe.yml` with a `repeat` input. Author `scripts/judge_traces.py`, which runs one substrate topology over downloaded traces: EXECUTION_COMPLETED per trace → TRACE_INGESTED → COMPARISON_COMPLETED.

## prerequisites

- 005, 006

## context_files

- `contracts/windows.event.auto_reset.single_signal/0.1.0.json`
- `src/w2m/trace.py`, `src/w2m/compare.py`, `probes/hello/hello.c`
- Microsoft Learn: BCryptHash, RegGetValueW (registry value `UBR`, `InstallationType` under `HKLM\SOFTWARE\Microsoft\Windows NT\CurrentVersion`)

## signal contract

### Emits

- Guest plane: GUEST_TRACE_OPENED, GUEST_THREAD_STARTED x3, GUEST_CALL_RETURNED x6, GUEST_THREAD_EXITED x2, GUEST_TRACE_CLOSED — 13 lines per execution.
- Harness plane (judge run): EXECUTION_COMPLETED, TRACE_INGESTED, COMPARISON_COMPLETED per trace.

### Invariants

- Contract 0.1.0 is not edited; observed evidence lands in a new version file.

## artifact contract

### Files created / modified

- `probes/event_auto_reset/event_auto_reset.c` (created)
- `scripts/judge_traces.py` (created)
- `.github/workflows/windows-probe.yml` (modified: `repeat`, extra libs)

### Command exit codes

- `gh run watch <run-id> --exit-status` returns 0
- `uv run python scripts/judge_traces.py <dir>` returns 0 and prints one verdict per trace
- `uv run pytest`, `uv run ruff check`, `uv run mypy` return 0

## observation contract

### Driving steps

1. `gh workflow run windows-probe.yml -f probe=event_auto_reset -f execution_id=evt -f repeat=10`
2. Download artifact; run `scripts/judge_traces.py` on it.

### Expected runtime signals

- 10 TRACE_INGESTED; 10 COMPARISON_COMPLETED with verdict `conforms`; no ProducerFailed.
- Outcome split recorded (how many times a vs b was released); either split conforms.
- profile_id names build 26100, a UBR, and `Server`.

## done criteria

The event contract has observed evidence from real x64 Windows, judged on a substrate record.

## self-review checklist

- [x] Scope concrete.
- [x] Bridge APIs named with their source pages.
- [x] Emits are locked tags.
- [x] Assertions gradable.
- [x] Observation contract present.
- [x] One concept (reference evidence for one contract); three files, one of them a small workflow edit.
