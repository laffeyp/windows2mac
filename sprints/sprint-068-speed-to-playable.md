# Sprint 068 — speed toward the playable slice

```yaml
---
id: 068
status: open
phase: 5
pass_kind: functional
---
```

## scope

Milestone P (`docs/milestone-playable-slice.md`) needs the game's first scene in under 5 minutes on the Mac; the
laptop reaches it in about 60 s. The time census (commit d91a9e21) of frontier 1791942295792627000 split its host
time by exit kind: 2,366 s in the vCPU, 2,129 s in hvc 10 (the slow-SSE path), about 555 s in hvc 12 (the scheduler),
about 323 s in hvc 11. Nearly all of hvc 10 was one instruction, UnityPlayer's strstr at 0x18140b914 (PCMPISTRI
imm 0x0C, MXCSR 0x3f). This sprint removes host time in the order the census ranks it, one cause per item, each
measured by the census before and after and each leaving the frontier's comparison conforming.

- **A. PCMPISTRI natively** (done before this card; logged in `WORKING_AGREEMENT.md ## Hand-author log`):
  `tools/guest_runtime/string_compare.c` (d31f35e6, 78768984), index 1ecc22b2. Frontier on 1ecc22b2: hvc 10 down
  from 2,129 s to 0.5 s; 41.7 billion instructions in 186.8 s inside the vCPU.
- **B. Thread switches.** After A, hvc 12 is the largest host share (61.6 s of about 300 s at 41.7 billion), about
  55 µs an exit, and hvc 4 time-slice exits switch threads through the same `_save_thread`/`_load_thread`. A profile
  splits the 55 µs between the text pipe (`vget`, `vset`, `set`, `run`) and Python; the change follows the split.
- **C. Native lowerings for the model's integer SSE classes** (added 2026-10-13 from frontier 1ecc22b2's census past
  41.7 billion instructions, where hvc 10 rose again to 168.5 s): PMADDUBSW (344,613 steps), PSADBW (152,246), PEXTRW
  (150,337), CMPPS (112,370), MOVD (67,206), POPCNT (50,260), CVTPS2DQ (21,393), RSQRTPS (18,965), all with every MXCSR
  exception masked. Each lowering is judged on the Xeon's cases with a plant caught, as Sprint 064's were.
- **D. hvc 4** (budget exits) by the census after B and C.
- **E. The transport** (Sprint 064 item 3, not done there): the text pipe replaced by shared memory if exits still
  hold a large share after B and C.

## prerequisites

- 066 (closed: CI run 38506623160)

## context_files

- `src/w2m/vm_backend.py` (`steps`, `_schedule`, `_switch`, `_save_thread`, `_load_thread`, `vectors`), `src/w2m/kernel.py`
  (`Machine._next`), `tools/vm_monitor/vm_monitor.c`, `scripts/run_frontier.py`, `docs/milestone-playable-slice.md`

## signal contract

### Emits

- The frontier's records (EXECUTION_STARTED, its stop, TRACE_INGESTED, COMPARISON_COMPLETED, SESSION_COMPLETED)
  after each change; the family sweep's records (CANDIDATE_ACCEPTED for all 68 families) on each runtime change.

### Invariants

- Vocabulary unchanged; no new tags without a proposal.
- The frontier's comparison against runner run 38216081440 stays conforming (0 defects by `call_compare.by_cause`).

## artifact contract

- modified: `src/w2m/vm_backend.py`, `tools/vm_monitor/vm_monitor.c` where an item needs it; a test that fails before
  each change
- `uv run pytest` exit 0; the family sweep 68 of 68 accepted; CI `success` on the closing commit

## observation contract

- **The census before and after each item,** from the frontier's `time:` and `progress:` lines: seconds by exit kind
  per billion instructions, and instructions per second of wall time.
- **The frontier conforms** with the same calls before its stop.

## done criteria

Host time outside the vCPU below the vCPU's own time on the frontier, each item measured, the comparison conforming.

## observed

- **A.** Family sweep 68 of 68 accepted on the native runtime (index 1ecc22b2). Frontier on 1ecc22b2 at 41.7 billion
  instructions: in run 186.8 s; host by exit hvc 12 61.6 s, hvc 4 33.8 s, hvc 1 10.4 s, hvc 8 7.3 s, hvc 10 0.5 s.
- **B, the profile.** A cProfile of 400 million instructions (scratch) made 205,614 switches: `_schedule` 10.3 s
  cumulative, about 50 µs a switch, of which `vectors()` (a `vget` round trip and its hex parse) 4.2 s and the
  `vset` about as much. Each pipe command waits about 12.5 µs in `readline` (1,028,724 commands, 12.8 s).
- **B, the change.** `vm_monitor.c` `vswap` copies Q0-Q15, FPCR and FPSR to the outgoing thread's slot
  (`VECTOR_SLOTS`, in the monitor area's padding) and loads the incoming one's, sent in the same write as the next
  command. `tests/test_thread_switch_vectors.py`: the threads and running_thread_context probes keep Windows' rows
  with `vget`/`vset` only for model steps; a direct swap inside a running VM; a dropped load and a dropped save are
  each caught. The two probes alone did not catch the dropped load: neither holds a vector across a switch.
