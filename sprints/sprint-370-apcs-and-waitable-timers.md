# Sprint 370 — APCs and waitable timers the game declares: QueueUserAPC, CreateWaitableTimerA, SetWaitableTimer

```yaml
---
id: 370
status: closed-partial
phase: 6
pass_kind: functional
---
```

## scope

The Decision "G3 met at Sprint 368, and the order after it" serves first the waits and threads of
`docs/unserved-declared-imports-2026-11-27.md`. Sprint 369 served the named sync objects (six of them). This card
serves the asynchronous procedure calls those waits deliver, and the waitable timer that queues one:
- mono-2.0-bdwgc.dll's QueueUserAPC;
- UnityPlayer.dll's CreateWaitableTimerA and SetWaitableTimer.

Today every alertable wait in the runtime discards its flag ("no APC is ever queued": `sync_objects.c` 1031-1032,
`processes.c` 198, `threads.c` 545, `k32_SignalObjectAndWait`). An APC queued to a thread runs only when that thread
makes an alertable wait, and the wait then returns WAIT_IO_COMPLETION; this card makes the flag real on SleepEx,
WaitForSingleObjectEx, WaitForMultipleObjectsEx and SignalObjectAndWait.

- **Windows' rows first.** A probe, `probes/apcs_and_waitable_timers`, in the form of `probes/named_sync_objects`,
  records on Windows: an APC queued to the caller and to another thread, run only at an alertable wait, in FIFO order,
  the wait returning WAIT_IO_COMPLETION (192); an unalertable wait not running it; a manual-reset and a
  synchronization waitable timer, named and unnamed, set with a relative and an absolute due time and a period,
  waited on, re-set and cancelled by a second SetWaitableTimer; SetWaitableTimer's completion routine queued as an
  APC to the setting thread with its argument and the two time words. Expected rows from Microsoft Learn's pages,
  quoted with their URLs; the Windows run settles them. Main commits the probe as a card amendment (`probes/` only)
  and runs the Windows workflow.
- **The id pin first** (the Decision "The runtime's API id pin covers every row"): at Sprint 369's grade ids 302 and
  303 swapped passed the runtime's 21 tests, the pin holding rows 1-299 by sha256 and of the rows above only 300, 301
  and 306. Before any id is added, the test in `services.rs` pins rows 1-306 by one sha256; this card's new rows then
  raise it to cover them.
- **The services**, beside the sync objects, each new service taking the next id in
  `crates/guest_runtime/src/api_ids.txt`, the timer's due time on the guest clock the waits already read.
- **The family run.** The probe joins `store/family_runs/joined-2026-11-27.tsv`, held to its contract on one vCPU
  and two.
- **Not here:** the timer queues and registered waits (thread-pool callbacks), the completion ports, the thread and
  module calls of the class (the cards after this one).

## prerequisites

- 369 (closed)

## context_files

- `docs/unserved-declared-imports-2026-11-27.md`.
- `tools/guest_runtime/sync_objects.c` (the waits, SignalObjectAndWait, the named objects), `tools/guest_runtime/threads.c`
  (SleepEx, the thread records), `tools/guest_runtime/processes.c` 180-260 (WaitForMultipleObjectsEx),
  `crates/guest_runtime/src/services.rs` (the lock classes, `api_ids.txt`'s reader), `crates/guest_runtime/src/api_ids.txt`.
- `probes/named_sync_objects/` (the probe's form), `contracts/windows.sync.named_objects/0.1.0.json` (the contract's form),
  `store/family_runs/joined-2026-11-27.tsv`, `crates/w2m_vm/tests/family_runs.rs` 1-120.
- `docs/design-handles-and-sync-objects-2026-11-26.md`: the lock classes.
- `grep -n "G3 met at Sprint 368\|The runtime's API ids fixed" BLACKBOARD.md`: the Decisions.
- `WORKING_AGREEMENT.md` lines 137-266, "Build and verification commands".

## signal contract

### Emits

- None: services and a family run; no emit site changes. The new family's run records are held to its contract.

### Invariants

- Every run as held: the batch run on one vCPU and two (G3), the 274 family runs on one vCPU and two, the guests, the
  exit ledger. No existing id in `api_ids.txt` moves. A guest that never queues an APC sees every wait answer as before.

## artifact contract

### Files created

- `probes/apcs_and_waitable_timers/` (source, expected rows) and its family contract under `contracts/`.

### Files modified

- `tools/guest_runtime/sync_objects.c` and `threads.c` (the APC queue, the timer, the alertable waits),
  `crates/guest_runtime/src/services.rs` and `api_ids.txt`, `store/family_runs/joined-2026-11-27.tsv`, with others only
  where the class requires, reported to main when found. One concept: APCs and the timer that queues them.

### Command exit codes

- `scripts/gates.sh` returns 0
- `target/release/signal_report --check` returns 0 on the tree

## observation contract

- **Expected values:**
  - The class first: each function's Learn page quoted, and the alertable-wait pages (SleepEx,
    WaitForSingleObjectEx, Asynchronous Procedure Calls), and the probe's rows.
  - The Windows run's rows, and w2m's run of the probe held to them on one vCPU and two.
  - Every existing run as held. The gates' two-vCPU batch run's seconds to the stop quoted beside the laptop's 90 to
    102 s and Sprint 369's, with its `load_at_start` (the Decision at Sprint 369's close, reviewer-4's forty-third
    review).
- **Planted faults:**
  - (1) An APC run at an unalertable wait: the probe's unalertable row differs from Windows'.
  - (2) A periodic timer not re-armed after its first expiry: the period row differs, or its run waits to its limit.

## done criteria

QueueUserAPC, CreateWaitableTimerA and SetWaitableTimer are served, the four alertable waits deliver APCs as
Windows does, and all are held to Windows' own rows on one vCPU and two; every other run is unchanged.

## self-review checklist

- [x] Scope names one verifiable property: APC delivery and the timer, held to Windows' rows.
- [x] context_files cite the class, the waits, the probe's and contract's forms and the family judge.
- [x] Emits: none, with its reason.
- [x] Artifact contract gradable by command.
- [x] Observation contract: the probe's rows on Windows and w2m; numbered plants; the speed figure.
- [x] One concept: APCs and the timer that queues them.

## result

Closed partial at the stop of 2026-11-27 (`docs/postmortem-windows-runs-off-the-laptop-2026-11-27.md`): no Windows run made for it on GitHub's runner; its probe ran on the laptop instead, `store/probe-runs/lap-20261127T210820791Z-b5a7cf3d/`. Its unfinished services (QueueUserAPC, CreateWaitableTimerA, SetWaitableTimer, ids 307-309, the id pin over 309 rows; 8 files) are kept in git stash@{0} and resume in a later card. No gates ran; nothing of it is committed but its card and probe.
