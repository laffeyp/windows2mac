# Sprint 364 — Processes at once on several vCPUs: the child on one, the request server answering it, the deadlock verdict across processes

```yaml
---
id: 364
status: active
phase: 6
pass_kind: functional
---
```

## scope

Card 4f3 of `docs/design-a-process-that-starts-another-2026-11-26.md` ("The cards that follow", item 3; "The form on
several vCPUs", "Which lock or request each crossing takes", "The deadlock rule", "The stop"). The design is the spec.
On several vCPUs a process that starts another is refused today at `MACHINE_ON_ONE` (`host_request.rs`). This card lifts
that refusal: the parent runs on the run's vCPUs, its child on one, the two at once, with no Machine and no turns, and
what crosses between them goes through Sprint 363's request server (`cross_process.rs`).

- **The spawn.** On several vCPUs a spawn makes the `Server` link under `kernel` and adopts the child into the server's
  table; the child, still on one vCPU, makes the `Client` link. One vCPU keeps the turn form unchanged.
- **Wakes across processes.** Each process gets its link thread and wake count, as the design states: the server
  raises every live process's wake count after a waking answer; a link thread takes its process's runtime word, then
  the run's lock, and runs `wake_blocked`, never `kernel`. The first process's own wake is in-process (the SDK header:
  SHARED "should not be used when synchronizing among multiple threads of a single process").
- **No turns on several vCPUs.** A link makes no `Switch::Idle` and no turn there; `others_due` and `others_ready` are
  false. A process with a link parks with no deadlock verdict of its own.
- **The deadlock verdict across processes**, as the design's "The deadlock rule" states: each process posts in its slot
  when every vCPU is parked and no blocked thread has a deadline, and clears it when a vCPU takes a thread; the server
  stops the run with `DEADLOCK`'s words when every live process has posted and a whole second passes with no request,
  no wake count raised and no post cleared.
- **The stop.** The server, on a child's stop or a child gone, stops the first process's run through its `RunStop`. A
  run that stops for any reason ends every child, including one waiting on a slot the server never answered (Sprint
  363's refused request leaves its slot unanswered): no child is left past the run, under the user's Decision of
  2026-10-18. A test holds it.
- **One home for two constants.** `cross_process.rs` takes `SHARED` and `MACH_ABSOLUTE_TIME` from `shared.rs`
  (declared in both today: `shared.rs` 33, 40; `cross_process.rs` 50, 52; the thirty-fifth review's disposition).
- **The class first.** List every reader of `MACHINE_ON_ONE`, of `Link`, of `others_due`/`others_ready` and of the turn
  form's `Switch::Idle`, with file:line, and what each does on several vCPUs after this card. Report each defect found
  to main at once, with its evidence, and fix any that can stop a run or leave a thread past its deadline here.
- **Not here:** the child on the run's vCPUs (card 4f4: `child_start` still gives a child one vCPU, and `vcpus_allowed`
  still refuses a child above one).

## prerequisites

- 363 (closed)

## context_files

- `docs/design-a-process-that-starts-another-2026-11-26.md`: in full.
- `crates/w2m_vm/src/cross_process.rs`: in full.
- `crates/w2m_vm/src/host_request.rs` 130-320 and 500-640, `crates/w2m_vm/src/threads.rs` 600-700 and 1000-1060 and
  1390-1440, `crates/w2m_vm/src/turns.rs` 1-60 and 160-200 and 430-650, `crates/w2m_vm/src/run.rs` 925-1060 and
  1580-1620 and 2000-2060 (the lock, the turn, `others_due`, `others_ready`), `crates/w2m_vm/src/bin/w2m-vm.rs` 700-820
  (the child), `crates/w2m_vm/src/shared.rs` 1-60.
- `probes/processes/processes.c` (the six runs' guest), `crates/w2m_vm/tests/family_runs.rs` 1300-1400 (the refused
  list's test), `store/family_runs/two-vcpus-refused-2026-11-26.tsv`, and one guest under `tests/data/timed_waits/` as
  the form for this card's two-process guest.
- `grep -n "The form of card 4f on several vCPUs\|The .kernel. lock, card 4f1\|The request server, card 4f2" BLACKBOARD.md`: the
  Decisions; `grep -n "thirty-fifth review" BLACKBOARD.md`: the constants.
- `WORKING_AGREEMENT.md` lines 137-266, "Build and verification commands".

## signal contract

### Emits

- None: how processes run on several vCPUs; no emit site changes. The six runs' records, which stopped at the refusal,
  become runs held to Windows' rows; every other run's records are unchanged.

### Invariants

- One vCPU unchanged: the batch run conforming, the 273 family runs on one vCPU as held, the `twoproc` guests (on the
  wall clock and the instruction clock) and the exit ledger.
- On two vCPUs: the 267 runs held as on one stay so held.

## artifact contract

### Files created

- A two-process guest under `tests/data/timed_waits/` (source and built image), each process waiting INFINITE on a
  named event only the other sets.

### Files modified

- `crates/w2m_vm/src/host_request.rs`, `crates/w2m_vm/src/threads.rs`, `crates/w2m_vm/src/cross_process.rs` (the
  design's three files for 4f3), with `run.rs`, `turns.rs`, `bin/w2m-vm.rs` and the tests only where the class list
  requires, reported to main when found. One concept: processes at once on several vCPUs.
- `store/family_runs/two-vcpus-refused-2026-11-26.tsv` and its test: the six runs leave it.

### Command exit codes

- `scripts/gates.sh` returns 0
- `target/release/signal_report --check` returns 0 on the tree

## observation contract

- **Expected values:**
  - The class list.
  - The six runs (`process_parameters` 38194049597 and 38194051809, `processes` 38189162864 and 38189164581,
    `time_and_error_mode` 38206225386 and 38206227300) on two vCPUs, parent on two and children on one, held to their
    contracts' `exact_rows`, pipe.capacity at 4,096; each run 10 times, all conforming, quoted.
  - The refused list holding none, and its test saying so.
  - The two-process deadlock guest on two vCPUs stopping within two seconds as a deadlock, by `DEADLOCK`'s words,
    quoted with its time.
  - A run stopped while a child waits on an unanswered slot: the child ends, `pgrep` finds none left.
- **Planted faults:**
  - (1) The child's end raises no wake count: the `processes` parent's WaitForMultipleObjectsEx on its child
    (`processes.c` 297) runs to its 10,000 ms timeout, and the row `process.exited` differs.
  - (2) The server's verdict skipped: the deadlock guest runs to GUEST_SECONDS and stops by the time limit, not as a
    deadlock.

## done criteria

On two vCPUs a process that starts another runs at once with its child on one vCPU, the crossings answered by the
request server and the wakes applied by each process's link thread; the six held runs conform, a deadlock across
processes stops as one, a stopped run leaves no child; one vCPU is unchanged.

## self-review checklist

- [x] Scope names one verifiable property: processes at once on several vCPUs, the six runs conforming.
- [x] context_files cite the design, the server, the turn form, the guest and the held list.
- [x] Emits: none, with its reason.
- [x] Artifact contract gradable by command.
- [x] Observation contract: the six runs, the deadlock guest, the stop; numbered plants.
- [x] One concept: processes at once. The design names three files; the deadlock verdict and the stop belong here,
  since without them this card's form would leave a run hanging to its limit (the user's Decision of 2026-10-18).

## result

GATES-LINE
