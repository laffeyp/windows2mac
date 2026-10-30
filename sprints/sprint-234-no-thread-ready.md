# Sprint 234 — No thread ready: the warp and the timed wait

```yaml
---
id: 234
status: active
phase: 6
pass_kind: functional
---
```

## scope

The Decision "Sequence after Sprint 214" as annotated at Sprint 233: the warp to a deadline, plan step 4's
"68 of 68 families" and its guests item.

- **Where it stands, by lookup (2026-10-30):**
  - Ten family runs and two guests stop at the same place: running_thread_context 1, slim_locks 4, threads 2,
    time 2, wait_handoff 1, and the guests `mutual` and `mutual_context`. The stop reads "no thread ready after thread
    4's hand-over and 1 waiting with a deadline: the warp to a deadline is not served by the Rust host yet"
    (`crates/w2m_vm/src/threads.rs` 724-728). On the frozen host the two guests end, "done"
    (`store/batch_run/first-stop-2026-10-28.tsv` 42-43).
  - The frozen `_switch` (`src/w2m/vm_backend.py` 3521-3580 at the tag) handles one process with no thread ready and
    a thread waiting with a deadline in two ways:
    - On the instruction clock, `warp_to_deadline` (3614-3628) moves the clock's warp to the earliest deadline,
      ordered by deadline then thread id, and readies that thread.
    - Otherwise it sleeps 1 ms and readies the first timed waiter in rotation order, which checks its own deadline
      in guest code.
  - None of the 12 starts on the instruction clock, so all 12 take the second branch.
  - The Rust host already has `counter_now`, the frozen clock reading (threads.rs 534-550), and `wake_expired`.
- **The work:**
  - `switch` with no ready thread and a timed waiter takes the frozen branches. With the instruction clock it warps
    to the earliest deadline and readies that thread. Otherwise it sleeps 1 ms and readies the first timed waiter in
    rotation.
  - A unit test holds the warp's arithmetic and the order of the choice on a guest's run table.
- **Not here:** a machine of several processes, which returns Idle as before.

## prerequisites

- 233 (closed)

## context_files

- `src/w2m/vm_backend.py` at the tag (`_switch` 3521-3580, `warp_to_deadline` 3614-3628)
- `crates/w2m_vm/src/threads.rs` (`switch`, `counter_now`, `wake_expired`)
- `store/family_runs/sweep-held-2026-10-29.tsv`, `store/batch_run/first-stop-2026-10-28.tsv`

## signal contract

### Emits

- None: the families' and guests' runs write no record on the Rust host.

### Invariants

- The 263 conforming family runs still conform; the guests that end still print the frozen host's output.

## artifact contract

### Files modified

- `crates/w2m_vm/src/threads.rs`, `store/family_runs/sweep-held-2026-10-29.tsv`, `store/batch_run/first-stop-2026-10-28.tsv`,
  census rows 6 and 8

### Command exit codes

- `scripts/gates.sh` returns 0

## observation contract

- **Expected values:** the 10 family runs judged, against 263 of 273; `mutual` and `mutual_context` end with the
  frozen host's output, guests 16 of 16.
- **Planted faults:**
  1. The warp set to the deadline less one tick: the unit test names the clock.
  2. The latest timed waiter readied, not the first in rotation: the unit test names the thread.
  3. The 1 ms sleep's thread not readied: the runs stop again, named by the judge.

## done criteria

A process with no thread ready and a thread waiting with a deadline goes on as the frozen host's does, and every
family run and guest is judged.

## self-review checklist

- [x] Scope names one verifiable property: no thread ready with a timed waiter goes on as the frozen host does.
- [x] context_files cite the frozen switch and warp, the Rust switch, and the held files.
- [x] Emits: none, with the reason.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and three plants.
- [x] One concept: the switch with no thread ready.

## result
