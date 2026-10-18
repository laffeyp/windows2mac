# Sprint 117 — the kick

```yaml
---
id: 117
status: active
phase: 5
pass_kind: architecture
---
```

## scope

Step 1, the chain's fourth sprint (docs/design-several-vcpus.md, "The host for several vCPUs, designed twice": "a
kick, not a cut"). One concept: the host stops one running vCPU on demand. Today a vCPU leaves `hv_vcpu_run` at an
exit of its own or when its run's deadline passes (the monitor's watchdog, `hv_vcpus_exit`); a host thread that wants
another vCPU stopped (a stop request, chain step 6's SuspendThread on a thread another vCPU runs) has no way to ask,
and the parked Sprint 107 attempt cut every run at 50 ms instead.

- **The word:** each channel page gains a kick word at offset 2048 (vm_monitor.c `CH_KICK`, vm_backend.py
  `CH_KICK`), which the host bumps by one per kick with one 32-bit store and wakes (os_sync_wake_by_address_any).
- **The monitor:** each channel starts a kicker thread for its vCPU that waits on the word (os_sync_wait_on_address)
  and, on any change, marks the vCPU kicked and calls `hv_vcpus_exit` for it. hv_vcpu.h: hv_vcpus_exit names no owning
  thread (the watchdog already calls it from its own), and "If a vcpu is not running, the next time hv_vcpu_run is
  called ... it will return immediately", so a kick between runs is kept. The run loop reports a canceled exit of a
  kicked vCPU as kind `kick` (5 on the channel), ahead of the watchdog's `cancel`.
- **The host:** `Channel.kick()`, `Vcpu.kick()`; `serve_exit` answers a kick by resuming the vCPU where it stopped
  (its count kept from x23, as for every exit), and counts it (`VM.kicks`). What a kick is for (a stop, a preemption)
  is the kicker's to act on at the run loop's top, chain steps 5 and 6.

No behaviour changes for a run nobody kicks.

## prerequisites

- 116.

## context_files

- tools/vm_monitor/vm_monitor.c (`watchdog`, `run_vcpu`, `channel`, `wait_change`, `vcpu_thread`);
  src/w2m/vm_backend.py (`Channel`, `Vcpu`, `serve_exit`); tests/test_vcpus.py (the channel harness); the SDK's
  hv_vcpu.h (`hv_vcpus_exit`).

## signal contract

### Emits

- On the batch run's record, as on Sprint 116's (frontier-643a6283510c-1792299009808193000): SESSION_INIT
  (vocab_version 0.9); EXECUTION_STARTED (timeout_seconds 10800, kill_grace_seconds 60); MEASUREMENT_RECORDED 107
  times; EXECUTION_COMPLETED (exit_status killed); TRACE_INGESTED (thread_count 28); COMPARISON_COMPLETED
  (first_divergence none); SESSION_COMPLETED (outcome completed).
- Per class run (458): SESSION_INIT, CANDIDATE_GENERATED, COMPARISON_COMPLETED, then CANDIDATE_ACCEPTED (456) or, for
  FCOS and FSIN, REDUCTION_COMPLETED, DISCREPANCY_REPORTED and DISCREPANCY_ROUTED (8 each), SESSION_COMPLETED. Per
  family run (68): SESSION_INIT, CANDIDATE_GENERATED, COMPARISON_COMPLETED, CANDIDATE_ACCEPTED, SESSION_COMPLETED.

### Invariants

- Vocabulary 0.9 unchanged.

## artifact contract

- `CH_KICK`, the kicker thread and kind `kick` in vm_monitor.c; `Channel.kick`, `Vcpu.kick`, `VM.kicks` and
  serve_exit's answer in vm_backend.py; the tests below; `scripts/gates.sh` exit 0; CI `success`.

## observation contract

- **Judge 1 (a live kick):** vCPU 1 spins on `b .` in a run with a 5,000 ms deadline; a host thread kicks it 200 ms
  in: the run returns kind `kick` between 200 and 400 ms.
- **Judge 2 (a kick between runs is kept):** vCPU 1 kicked while stopped, then run on `b .` with a 5,000 ms deadline:
  the run returns kind `kick` within 100 ms.
- **Judge 3 (a kick loses no instruction):** vCPU 0 runs an x86 `jmp $` loop to max_steps through `steps`; a host
  thread kicks it 10 times: `VM.kicks` is 10 and the instruction count equals the unkicked run's.
- **Plant:** vCPU 1 spinning with a 300 ms deadline and no kick: the run returns kind `cancel` at about 300 ms, not
  `kick`.
- The batch run conforms (168 lines, 0 divergences, the same eight exit kinds, no kick); 456 of 458 classes, 68 of
  68 families; the litmus and runtime-lock tests.

## done criteria

A host thread stops any one vCPU on demand and the run goes on where it stopped; CI green.
