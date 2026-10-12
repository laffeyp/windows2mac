# Sprint 064 — speed: the exits, the transport, the model steps

```yaml
---
id: 064
status: open
phase: 5
pass_kind: functional
---
```

## scope

The game under w2m is correct as far as it gets and slow. Its unity.log matches Windows' first 12 lines, and the
13th says "Completed reload, in 113.912 seconds" against Windows' 0.101. A frontier run of 4 billion instructions
(689 s) stays in UnityPlayer's deserialization loop (Sprint 063). Sprint 062's profile of a 33-second run puts:
- about 20 s in the monitor's text pipe: 1,659,004 commands, two an exit, so about 830,000 exits;
- 21 s in model steps (`_slow_sse`, 139,148);
- about 12 s in translation.
Of the exits, 1,088 are host requests and 61,498 are block compiles; the rest are not yet counted by kind. Decision
2026-10-11 (user) puts speed ahead of MMX and x87 and thread stacks.

1. **The census.** The frontier counts every exit by kind (hvc number and host op) and by the guest address that
   caused it, and it records instructions per second. Both go in the frontier's printout and in the card's
   observed section, before any change.
2. **The largest causes, removed or moved into the VM,** in the order the census ranks them. Each change carries a
   test that fails first and the frontier's census after it.
3. **The transport.** The monitor's text pipe (fgets, sscanf and readline: 10.3–10.5 µs an exit, Sprint 048) is
   replaced by a shared-memory exchange with the monitor process if the census shows exits still cost a large share
   after item 2.
4. **Native lowerings** for the model-step classes the census ranks highest, each judged against the Xeon as
   Sprint 050's classes were.
5. **The frontier passes seq 3426** (from Sprint 063), its comparison with game run 38206263976 conforming.

## prerequisites

- 063 (closed partial: CI run 38215049553; main 208e695b)

## context_files

- `src/w2m/vm_backend.py` (`steps`, `cmd`, the hvc numbers, `_slow_sse`), `tools/vm_monitor/vm_monitor.c`,
  `src/w2m/scheduler.py`, `scripts/run_frontier.py`, `scripts/measure_exit.py`; Sprint 062's profiles

## signal contract

### Emits

- The frontier's records (EXECUTION_STARTED, its stop, TRACE_INGESTED, COMPARISON_COMPLETED, SESSION_COMPLETED)
  before and after each change; each lowered class's item run as in Sprint 050.

### Invariants

- Vocabulary 0.7; no new tags without a proposal.

## artifact contract

- modified: `src/w2m/vm_backend.py`, `tools/vm_monitor/vm_monitor.c`, `scripts/run_frontier.py`, the lowered classes'
  files; tests that fail before each change
- gates 0 and CI `success` on the closing commit

## observation contract

- **The census before and after,** on the frontier's records: instructions per second and exits by kind.
- **The frontier passes seq 3426** with a conforming comparison and the same calls before it.

## done criteria

The frontier past seq 3426, conforming; its speed measured before and after each change.
