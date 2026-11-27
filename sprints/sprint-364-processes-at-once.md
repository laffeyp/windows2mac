# Sprint 364 — Processes at once on several vCPUs: the child on one, the request server answering it, the deadlock verdict across processes

```yaml
---
id: 364
status: done
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

On two vCPUs a process that starts another runs at once with its child on one vCPU: the six held runs end and conform
in 60 of 60 runs, a deadlock across two processes stops as one by `DEADLOCK`'s words, a run stopped while a child waits
on an unanswered slot leaves no process, and one vCPU is unchanged.

**The class** (file:line at 958177d36; what each does on several vCPUs now):
- `MACHINE_ON_ONE`: `host_request.rs` 79 (the constant) and 270 (the refusal): gone; a spawn there makes the request
  server's link (`host_request.rs` 276). Its readers rewritten: `run.rs` 810 (a comment), `family_runs.rs` 20, 24 and
  1359-1361 (`refusals()`, gone with its test, now `the_refused_list_holds_none`, 1360), `run_loop.rs` 1012 and
  1176-1183 (the two busychild tests, now 1004 and 1251).
- `Link`: `turns.rs` 319-354 gains `Server` and `Client` (325-326); `page_and_slot` answers None for them, so
  `others_ready` and `others_due` are false (336). `host_request.rs` 168 (`child_request`) adds the `Client` round trip
  (169); 274-288 (the spawn and `wake_children`) add the server's form (276, 290). `run.rs` 1039-1048 (`turn`'s match)
  answers the two variants as no link; no caller reaches it on several vCPUs. `w2m-vm.rs` 724 and 799: a start carrying
  `request_server` goes to `child_of_the_request_server` (761), the guest built by `child_guest` (797) for both forms.
- `linked()` readers: `threads.rs` 1035 (the switch's idle), 1407 (`set_budget`), 1428 (`slice_until`), `run.rs` 1119
  (the first budget), 1604 (the turn start's `others_due`) and 2019 (the slice end's `others_ready`) read the new
  `KernelLock::turns` (`run.rs` 1005), set once beside `linked` for the turn form's link only (now `threads.rs` 1126,
  1499, 1521; `run.rs` 1212, 1770, 2186). On several vCPUs none of them makes an idle, a turn or a time slice.
- `Switch::Idle`: made only at `threads.rs` 1126 (turns). With the request server's link the switch parks
  (`Switch::Parked`, `threads.rs` 1120) whether or not another vCPU holds a thread, one vCPU too. `run.rs` 1892, 1917,
  2049, 2067 (the loops into `turn`) are not reached on several vCPUs.
- Defects found and fixed here: `cross_process.rs` `Server::adopt` (265-270 at 958177d36) mapped a child's memory at the
  adopt from the file's length, 0 at a spawn, which the child sizes when it builds its guest; it maps at the first
  request now (`map_memory`). The server held `Arc<KernelLock>` and sits in that lock's link, an `Arc` cycle, so its
  `Drop`, which kills the children, would never run. Broken by construction: the server holds `Weak<KernelLock>`,
  upgraded once a pass and let go before the doorbell's wait, the thread ending when it cannot, and `Server::end` does
  not join its own thread when the lock's last drop lands there; its `Drop` is a working backstop
  (`the_request_servers_drop_ends_its_children_with_the_lock`, `run.rs`). The primary path is the run's explicit end
  by any path out (`EndRequestServerAtEnd`; `KernelLock::end_request_server`), as `Supervisor::end` is the turn form's.

**What changed.** `cross_process.rs`: `SHARED` and `MACH_ABSOLUTE_TIME` from `shared.rs` (33, 40, now `pub(crate)`),
one home each; `FirstProcess` (212), the first process's wake count, the server's stop and its park post, held in
`KernelLock::first` from the run's start; `Server::spawn` (376), the child started `--child` with `request_server` in
its start and adopted under `kernel`; `Server::wake_children` (417); the slot words `PARKED` (105) and `WOKE` (108);
the deadlock verdict (`weigh_deadlock`, 519, `QUIET` 67); `Client::side` (737) and `ChildSide` (837), a child's wake
count and post beside `kernel`. `run.rs`: the link thread (`apply_wakes`, 1540), started in `run` on several vCPUs and in
a child of the server's form (1255): it waits on its process's wake count and, when it moves, takes the runtime word
(the host's take) and the run's lock and runs `threads::wake_from_another_process` (679), never `kernel`; in the first
process it hands the server's stop to `RunStop`. `threads.rs`: `Sched::link` (761) and `ParkPost` (713); a linked
process's park posts at 796 and clears at 826, with no verdict of its own. `store/family_runs/two-vcpus-refused-2026-11-26.tsv`
holds no run. Created: `tests/data/timed_waits/two_process_deadlock.c` and `.exe` (`x86_64-w64-mingw32-gcc` 16.2.0,
`-O2 -nostdlib -e start -s -lkernel32`), and its one-vCPU stop in `store/batch_run/first-stop-2026-10-28.tsv` ("-" in
the frozen host's two columns, which never ran it).

**Observed.**
- The six runs on two vCPUs (parent on two, children on one), each run 10 times as `scripts/gates.sh` runs it, into
  `build/family_runs/<family>-<run>-two-vcpus.*`, and `family_runs.rs` run after each of the ten: 60 of 60 stops
  `ExitProcess { code: 0 }`; each of the ten judge runs exit 0, "on two vCPUs: 273 of 273 held as on one vCPU, 273 of
  them conforming; 0 refused by name ()". `pipe.capacity` read
  `{"s":"pipe.capacity","written_before_read":4096,"read":65536,"writer_done":0}` in 20 of 20 `processes` runs, and
  `{"s":"process.exited","wait":1,"wait_0":0,"exit_code":7}` in 20 of 20. `pgrep -fl "w2m-vm --child"` found none
  after each repetition.
- The refused list: `the_refused_list_holds_none` passes; `every_family_run_conforms_or_is_held_by_what_it_waits_on`
  prints "273 of 273 family runs conform, 68 of 68 families in every run".
- The deadlock guest on two vCPUs, `target/release/w2m-vm`, alone: "Handover: every thread waits with no timeout and
  nothing can wake one (a deadlock)", real 1.19 s (2026-11-26 18:35 PST); in nextest beside 43 tests, the same stop
  in 2.18 s; the verdict comes 1.0-1.1 s after both processes park (`QUIET`, 1 s, and the server's 100 ms look). On
  one vCPU: "Handover: every thread of every process waits with no timeout (a deadlock)", real 0.41 s. The card's
  "within two seconds" is quoted from these times, not held by the gate: a 2 s bound on the whole run failed at 2.18 s
  beside 43 tests, and a 10 s bound failed the gates' `crates/hypervisor/tests/no_wall_clock_bound.rs` (Sprint 224: no
  correctness test holds a wall-clock bound), exit 100 at the first gates run. The gate,
  `a_deadlock_across_two_processes_on_two_vcpus_stops_as_one`, holds the stop's own words and prints the time; a
  skipped verdict stops at "the run's time limit", GUEST_SECONDS 60 (plant 2).
- The stop: `--start {"arguments":"grandchild"}` on two vCPUs: "Handover: child process 4100: a grandchild process: not
  served yet", real 0.11 s; `a_stopped_run_ends_a_child_waiting_on_an_unanswered_slot` holds the stop and an empty
  `pgrep -l -g <the run's group>`, as `busychild_through_w2m_vm_on_two_vcpus_runs_its_child_at_once` and
  `a_deadlock_across_two_processes_on_two_vcpus_stops_as_one` do.
- One vCPU unchanged: the gates' batch run, the 273 one-vCPU family runs, `twoproc` and `twoproc-ic` and the exit
  ledger pass as held.

**Plants.**
- **Plant 1** "processes run 38189162864 on two vCPUs: stop: Handover: every thread waits with no timeout and nothing
  can wake one (a deadlock); on one vCPU: conforms", `family_runs.rs` `every_family_run_conforms_or_is_held_by_what_it_waits_on`
  exit 100; live 02:56:19-03:01:34 UTC:
  - `crates/w2m_vm/src/cross_process.rs` copied to the scratchpad; the ENDED answer's wake made false, the end kept
    (`Ok((0, 0, { kernel.objects.end(c.pid, code); false }))`), checked by grep at 580-583;
  - `w2m-vm` built, every family run made on one and two vCPUs as the gates make them, the judge run. The run gave:
    both `processes` runs on two vCPUs stop as a deadlock (quoted above), and both `time_and_error_mode` runs "stop:
    the run's time limit" (their WaitForSingleObject(child, 60000) outlasts GUEST_SECONDS). The card forecast the
    `processes` parent's WaitForMultipleObjectsEx (`processes.c` 297) running to its 10,000 ms timeout and the row
    `process.exited` differing; that row is never printed, since after the 10 s wait the parent's INFINITE read of the
    writer child's pipe has no wake when the writer's end breaks the pipe, and the server's verdict stops the run
    before the parent flushes its rows;
  - restored by `cp`, `cmp` clean; the backup and the judge's log in
    `/private/tmp/claude-501/-Users-peterlaffey-port/18efb1a6-9978-416c-904b-05e124030429/scratchpad/plants/364/`
    (`builder-plant1-cross_process.rs`, `builder-plant1-judge.txt`).
- **Plant 2** "left: \"the run's time limit\" right: \"Handover: every thread waits with no timeout and nothing can wake
  one (a deadlock)\"", `run_loop.rs` `a_deadlock_across_two_processes_on_two_vcpus_stops_as_one` exit 100, the run
  60.73 s; live 03:01:47-03:03:10 UTC:
  - `crates/w2m_vm/src/cross_process.rs` copied to the scratchpad; the verdict skipped (`if false {
    weigh_deadlock(..) }`), checked by grep at 491-493;
  - the judge run under nextest;
  - restored by `cp`, `cmp` clean; the backup and the judge's log in
    `/private/tmp/claude-501/-Users-peterlaffey-port/18efb1a6-9978-416c-904b-05e124030429/scratchpad/plants/364/`
    (`builder-plant2-cross_process.rs`, `builder-plant2-judge.txt`).
- Both plants were made before the server held `kernel` weakly; that change, after them, leaves the ENDED answer and
  the verdict's call as planted.

exit 0 in 419 s	default	2026-11-27T03:26:27Z	sprint 364	958177d36ae8f9c38a7420f40b258d583e703a57	10b07a91f36f7edbf58c3e65f9bce3e3387931e922b2cb3b27b91bbbe8f6dd86	root=42dce982e1afa83b7a17888a5f1059a238363787	rust=all python=0 samples=0	base=c4666a34856a3b52c9a80682f9ea03cf34edfe99
