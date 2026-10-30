# Sprint 234 — No thread ready: the warp and the timed wait

```yaml
---
id: 234
status: done
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

- `src/w2m/vm_backend.py` at the tag (`_switch` 3521-3580, its no-ready branch 3565-3580, `warp_to_deadline` 3614-3628)
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

- **The switch:** with no thread ready, `switch` collects the threads blocked, unsuspended and waiting with a
  deadline, in rotation order. With none it refuses as a deadlock, as before. Otherwise `timed_waiter` readies one as
  the frozen `_switch` (3565-3580 at the tag; the card first cited 3570) and `warp_to_deadline` (3614-3628) do:
  - on the instruction clock, the earliest deadline, then the lowest id, the warp moving the clock to that deadline
    if it is still ahead;
  - on the host's clock, a sleep of 1 ms and the first timed waiter in rotation, which checks its own wait in guest
    code and blocks again if its time has not come.
  - A machine of several processes still returns Idle.
- **The runs:**
  - All 10 family runs end and conform: running_thread_context 1, slim_locks 4, threads 2, time 2, wait_handoff 1.
    Every family run now conforms, 273 of 273, and 68 of 68 families in every run.
  - `mutual` and `mutual_context` end and print "done", the frozen host's output: the guests are 16 of 16.
  - None of the 12 starts on the instruction clock, so the runs took the sleep. The warp is held by the unit test
    only.
- **Census rows 6 and 8** are met.
- **The unit test,** `a_timed_waiter_is_readied_as_the_frozen_host_readies_one`, uses a guest with two created
  threads. It covers the earliest deadline (warp 10,000,000 ticks), a tie to the lower id, a passed deadline with no
  warp, and the host's clock taking the first in rotation. Its first form wrote BLOCKED into the entries directly, and
  `set_status` overflowed the run table's count of blocked threads (threads.rs 509). It now blocks through
  `set_status`, as the host does.
- **My first rerun of the held runs** gave the two `threads` runs no beside DLL. They stopped at "import
  threads_dll.dll!threads_dll_count", a fault of the script, and ended once given their DLL as the gates give it.
- **Plant 1** (the warp one tick short): the unit test failed, "left: (9999999, 0, 1)".
- **Plant 2** (the last timed waiter readied, not the first): "the host's clock: the first in rotation", "left: 8, right: 4".
- **Plant 3** (the chosen thread not readied): the card expected the runs to stop again; they did not, since `switch` hands the vCPU to the chosen thread whatever its status, and both guests ended. The unit test named it: "left: (10000000, 1, 1)", the thread still blocked.
- Each plant was restored by `cp`, and `cmp` found `threads.rs` equal to its copy.
- **Reviewer-3's review at 234,** which came as the first gates run passed (457 s), on the tree before this change:
  - *Low 1* (the switch hands the vCPU to a thread without checking it is ready): `switch` now refuses a chosen
    thread whose status is not READY, on every path into it. Plant 3 run again, the waiter unreadied, stops both
    guests: "Handover: thread 4 given vCPU 0 while not ready (status 1)". Unplanted, both end with "done". The gates
    ran again on this tree.
  - Low 2 (the three argument forms neither host serves), Low 3 (the `sed` replacement with a newline) and the
    ledgers' join are dispositions in BLACKBOARD: a Deferred entry and census row 6's wording, a sentence in
    WORKING_AGREEMENT's edit rule, and the Decision's order.
