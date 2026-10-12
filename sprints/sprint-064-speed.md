# Sprint 064 — speed: the exits, the transport, the model steps

```yaml
---
id: 064
status: closed
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
5. **The frontier passes seq 3426** (from Sprint 063), its comparison with the reference game run conforming.
6. **Threads paired by their names** (amended 2026-10-12, before the work: the faster frontier schedules threads
   differently, and pairing near-identical workers by agreement mispaired them). The capture records
   SetThreadDescription's name, as it records other string arguments, in a new reference run. w2m's trace records the
   same, and the comparison pairs threads by name before it pairs the rest by agreement.

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

## observed

- **The census** (`vm.exits`, `vm.exit_sites`, printed by the frontier) found that of 8,747,587 exits in a
  billion-instruction run, 8,474,510 were dispatch misses. The run then executed 4.0 million instructions a second.
- **The dispatch table.** At 4,096 entries indexed by an address's low 12 bits, the game's ~62,000 blocks evicted
  each other. It now has 2^20 entries (16 MB, sparse, after the reconstruction table), and the index folds the
  address's higher bits in, since the modules sit at 1 MB-aligned bases. Misses fell to 163,508, the first visits and
  the resumes after thread switches, and the billion instructions to 20.8 s.
- **Native lowerings, each judged on the Xeon's cases with a plant caught:**
  - CVTDQ2PS and VCVTDQ2PS (SCVTF; 149 and 605 cases; UCVTF plant);
  - CVTPS2PD and VCVTPS2PD.128 (FCVTL; 163 and 687; FCVTL2 plant);
  - CVTPD2DQ and VCVTPD2DQ (FRINTX and FCVTZS per lane, both range bounds; 345 and 251). The Xeon's case 227
    showed the lower bound matters for PE; the CVT_INDEFINITE switch plant was caught;
  - CVTPD2PS (FCVTN; 313; FCVTN2 plant);
  - 32-bit IDIV's fast path (288; a range-check plant).
  All 436 classes are accepted after each change.
- **The speed:** from 4.0 to 442.28 million instructions a second over a 20-billion-instruction run, which takes
  45.2 s. The game's assembly reload, which unity.log times, falls from 113.912 s to 7.750 s (Windows: 0.154).
- **Found on the way, all fixed with tests that failed first:**
  - **The trace lost calls.** A retried wait never gave back its trace slot, so 744,597 calls went unrecorded;
    the slot now comes back. A comparison over a trace that left calls out is now `inconclusive`.
  - **The trace's thunk table** was widened to 16,384 entries.
  - **The comparator's failure rule** took any 4-byte 0 for failure and judged a wait's stale last error; it now
    takes only a BOOL's FALSE, a NULL or a bad handle.
  - **Thread pairing.** Pairing threads by agreement mispaired near-identical workers once the schedule changed.
    The capture now records SetThreadDescription's name (reference run 38216081440: Job.Worker 0–2, Background
    Job.Worker 0–15, Loading.AsyncRead and others), w2m's trace keeps it, and threads pair by name.
- **Not met: the frontier does not pass seq 3426.** A 200-billion-step run stops after 23,371,280,986 instructions at
  `LiftError: no operand template for FLD at 0x1180ccfd2f` (mono, `dd 44 24 f8`: FLD m64), its comparison
  conforming over 2,508 calls. The game has left the compute phase that held 063 and needs x87, plan row 065. The
  criterion moves there by the partial-pass rule.
- **The snapshot's question.** At 442 million instructions a second a rerun costs less still, so Decision
  2026-10-11's decline stands.
- **Closed partial on CI run 38219168662** (success on cc88048a).
