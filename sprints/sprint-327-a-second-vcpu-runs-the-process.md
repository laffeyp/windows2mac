# Sprint 327 — A second vCPU runs the process

```yaml
---
id: 327
status: closed-partial
phase: 6
pass_kind: functional
---
```

## scope

The run-loop design's card 4c3b2b, what Sprints 325 and 326 left of it: the Decision "Architecture: a vCPU with no
thread" (the park, the wait, the wake, the deadlock over every vCPU) and "The threads" (the stop, the merged outcome),
on a run of `n` vCPUs. Built before it: the lock held by `&` in `vcpu_loop` (324), the Machine refused above one vCPU
(325), the vCPU made on the run's own thread (326), `Sched` and `threads::take` (319-321), each vCPU's private chain
(315) and the switch on any vCPU (316). Today `vcpus_allowed` refuses any count above 1, "a second vCPU is not built",
and the loop stops with `PARKED_ALONE` where a second vCPU would park (`run.rs` 1281, 1422).

- **The work:**
  - `run` makes `n` vCPU threads in its scope; vCPU 0 as today; vCPU `k` from 1 on its pages and root table from
    `Guest::private_chain(k)`, taken before the threads start, with `k + 1` written at its `LOCK_OWNER` (Sprint 316's
    Observation 2: the chain copies vCPU 0's page, owner 1), its registers as vCPU 0's but its root table.
  - A vCPU with no thread: vCPU `k` starts with none, and a switch that returns `Switch::Parked` leaves it with none;
    either parks on `Sched` outside the lock and the word, with `park_deadline`'s deadline, and, woken, takes the word
    and the lock, wakes expired waits and calls `threads::take`, going on at the dispatcher with the thread taken, or
    parking again; `Woken::Deadlock` stops the run with the switch's deadlock error. `PARKED_ALONE` goes.
  - The stop: the first vCPU to stop records its `Stop` and sets the run's stop flag, then stops every other vCPU
    through its handle and notifies `Sched`; a vCPU that finds the flag set at a cancel, a park's wake or the word's
    spin ends recording nothing. The run's outcome is the first stop's.
  - The merged outcome: each vCPU's counts, times and exits summed into one `Outcome`; `executed` the sum of every
    vCPU's count; `runs_by_vcpu`, each vCPU's count of `vcpu.run` calls, kept apart.
  - `vcpus_allowed`'s last refusal, "a second vCPU is not built", goes; the rest stay.
- **Not here:** the kick (4c3b3), every other vCPU stopped for a locked model step or a split (4d), the race test
  (4e), the Machine's own thread (4f).

## prerequisites

- 326 (closed)

## context_files

- `docs/design-run-loop-on-several-vcpus-2026-10-31.md` "The threads" (154-165) and "A vCPU with no thread" (166-215);
  `crates/w2m_vm/src/run.rs` `run`, `run_on_vcpu`, `vcpu_loop`, `vcpus_allowed`, `take_word`, the two `PARKED_ALONE`
  stops; `crates/w2m_vm/src/threads.rs` `Sched`, `park`, `park_deadline`, `take`, `wake_expired`;
  `crates/w2m_vm/src/guest.rs` `private_chain`; BLACKBOARD Deferred (Sprint 325)

## signal contract

### Emits

- None; how many vCPUs run the process.

### Invariants

- Every run on one vCPU as held.

## artifact contract

### Files modified

- `crates/w2m_vm/src/run.rs`, `crates/w2m_vm/src/threads.rs` (the park's call, if its signature moves),
  `crates/w2m_vm/src/bin/w2m-vm.rs` (the count to the run), `crates/w2m_vm/tests/` (the two-vCPU runs)

### Command exit codes

- `scripts/gates.sh` returns 0

## observation contract

- **Expected values:** `qpc`, `sleeper` and `twoready` on 2 vCPUs ending as on one: the same stop,
  `ExitProcess { code: 0 }`, and the same output; `twoready`'s vCPU 1 running guest code, its count of runs in the
  merged outcome (`Outcome.runs_by_vcpu`, each vCPU's `vcpu.run` calls) above 0: the guest's main thread spins on its
  vCPU while it readies two workers each round, so a waiting vCPU must take one ("One waiting vCPU can take one
  readied worker", `twoready.c` 4-5); vCPU 1's `LOCK_OWNER`, read at its monitor page after the run, 2; `busychild`
  through `w2m-vm --vcpus 2` stopping with `MACHINE_ON_ONE` (Deferred, Sprint 325); every run on one vCPU as held.
- **Planted faults:** (1) vCPU `k`'s `LOCK_OWNER` left at 1: the test reading vCPU 1's word after the run reads 1 and
  fails; (2)
  the stop not passed to the other vCPUs: vCPU 1 stays parked after vCPU 0's stop and the run does not end, until the
  test's run limit of 60 s, as the gates' GUEST_SECONDS stops a guest, stops it as `Stop::TimeLimit` and the test
  fails naming that stop (a stop, not an assertion on elapsed time, which `no_wall_clock_bound` forbids); (3) `w2m-vm`'s `start.vcpus = vcpus` dropped: `busychild` on 2 vCPUs spawns, and its test fails naming the
  stop. Each judge's reach read before this card's commit (WORKING_AGREEMENT, Sprint 284): (1) reads the word the
  set-up writes, at the page `private_chain(1)` names, which the run leaves in guest memory; (2) runs the run's own
  two-vCPU path, which passes the stop; (3) runs `w2m-vm`, which sets the count on the guest path the test drives.
  Not a judge: whether the two-vCPU runs' stops differ under plant 1, which depends on which vCPU wins a race.

## done criteria

Two vCPUs run one process's threads and end it as one vCPU does.

## self-review checklist

- [x] Scope names one verifiable property: `qpc` and `sleeper` on two vCPUs ending as on one.
- [x] context_files cite the design's two sections, the loop, the scheduler and the chain.
- [x] Emits: none.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and three plants, each judge's reach read.
- [x] One concept: a second vCPU running the process.

## result

- **The run** (`crates/w2m_vm/src/run.rs`): `run` makes `guest.start.vcpus` vCPU threads in its scope, each through
  `run_on_vcpu`, which makes its vCPU, sets its registers and runs `vcpu_loop`. Before the threads: vCPU `k`'s root
  table and pages from `Guest::private_chain(k)`; each vCPU's budget, MXCSR, FCW and `LOCK_OWNER` `k + 1` written;
  the run table's `waiting` raised by the vCPUs past the first. vCPU 0 starts at the program's entry; vCPU `k` with no
  thread, taking one (`take_a_thread`).
- **A vCPU with no thread** (`take_a_thread`): under the word and the lock, `wake_expired` then `threads::take`; with a
  thread, its registers and its vectors (`load_vectors`) loaded and the vCPU on at the dispatcher; with none, a park
  on `Sched` with `park_deadline`'s deadline, holding neither, and `Woken::Deadlock` the run's stop with
  `threads::DEADLOCK`, the one-vCPU switch's words named once. The two `PARKED_ALONE` stops became the outgoing
  thread's vectors saved (`save_vectors`, `vswap`'s first half, at its slot, `vector_slot`) and a take.
- **The stop** (`RunStop`): the first `Stop` recorded; the flag set, then every vCPU stopped through its handle and the
  scheduler notified; a vCPU that finds the flag at a turn's start, a cancel or a take ends with `None`; `HaltAtEnd`
  halts the run when a vCPU's thread ends by any path, an error or a panic among them; the time limit's thread stops
  the run as `Stop::TimeLimit`. An ended vCPU's guest's hold of the runtime word is let go
  (`monitor_code::release_ended_vcpus_word`), in place of the card's stop flag in the word's spin: the waiter's take
  then succeeds, and its turn's start finds the flag.
- **The merged outcome:** `Outcome::empty` and `Outcome::add_vcpu`, each vCPU's times, exits and counts added;
  `runs_by_vcpu` each vCPU's `vcpu.run` count; `executed` the sum of every vCPU's count.
- **`vcpus_allowed`:** "a second vCPU is not built" gone; above `MAX_VCPUS`, 8, refused, `private_chain` making none
  past it. Four lines of `take_word`'s doc, left above `vcpus_allowed`'s, folded into `take_word`'s.
- **Found by the run, fixed:** `sleeper` on two vCPUs stopped at the 60-s limit; vCPU 1 had 0 runs and the run table
  `waiting` 1, the main thread ready and held by none, the spinner held by vCPU 0, no hand-over recorded. `add_thread`
  writes a new thread's `READY` with no notify, not through `set_status` (Sprint 319's "the one writer" did not hold
  for creation), so the parked vCPU slept through the spinner's creation and vCPU 0 switched to it inside the VM. It
  notifies now for every thread past the first; `sleeper` ended "woke\ndone\n", runs [13, 12]. Its class read whole:
  `threads::save`, the host's put-down, cleared a ready thread's `vcpu` word with no notify, and notifies now; the C
  runtime's `w2m_switch` does the same on a yield, carried to Deferred (Sprint 327) unseen in any run.
- **The tests** (`tests/run_loop.rs`): `two_vcpus_end_each_guest_as_one_does`, `qpc`, `sleeper` and `twoready` on 2
  vCPUs with a run limit of 60 s: vCPU 1's `LOCK_OWNER` 2, read first; the stop as on one vCPU; the output as on one,
  `qpc`'s by its form; `twoready`'s `runs_by_vcpu[1]` above 0. `busychild_through_w2m_vm_on_two_vcpus_is_refused_by_name`:
  "HostRequest op 33: " and `MACHINE_ON_ONE` (Deferred, Sprint 325, met). `run_loop_lock.rs` holds `run_on_vcpu`'s
  signature too, its `lock: &GuestLock<'_>` passed on.
- **Every guest on two vCPUs** (`w2m-vm --vcpus 2`, each against `store/batch_run/first-stop-2026-10-28.tsv`): seven
  that start a child stop with `MACHINE_ON_ONE`; `model_steps`, `mutual_context`, `mutual`, `qpc`, `qpc2`, `readied`,
  `sleeper`, `twoready` stop and print as held, `qpc`'s and `qpc2`'s readings by their form; `statsize` stops as held
  and prints "sizes differ" for "done": the host writes a stat into the requesting vCPU's runtime stack through vCPU
  0's mapping (`Layout::offset`). The done criterion does not hold for it: the close is partial, BLACKBOARD Deferred
  (Sprint 327, partial), card 328 its re-visit, before 4c3b3.
- **Every run on one vCPU as held:** `cargo nextest run -E 'rdeps(w2m_vm)'`, 198 tests passed in 415 s; `cargo
  clippy` with the gates' flags clean.
- **Plant 1,** `run.rs` copied aside and restored by `cp`, checked by `cmp`: the set-up's `LOCK_OWNER` `k + 1` made 1;
  the test failed, "assertion `left == right` failed: qpc: vCPU 1's LOCK_OWNER", left 1.
- **Plant 2,** restored likewise: the stop's hand-off (`place.stopping.stop(stop, …)`) and the `HaltAtEnd` guard both
  taken out, the guard alone hiding the hand-off's loss; the test failed at the run's 60-s limit, "assertion `left ==
  right` failed: qpc's stop on 2 vCPUs", left `TimeLimit`.
- **Plant 3,** `w2m-vm.rs` likewise: `start.vcpus = vcpus;` made `let _ = vcpus;`; the test failed, `busychild` ending
  "ExitProcess { code: 0 }".
- **Files, against the card's list:** `crates/w2m_vm/src/monitor_code.rs` (`release_ended_vcpus_word`) changed and is
  not listed; `crates/w2m_vm/src/bin/w2m-vm.rs`, listed for the count to the run, did not change: the run reads the
  count `w2m-vm` already writes on the guest's start (Sprint 325).
- **The step limit on several vCPUs:** each vCPU counts its own instructions against its own limit, `max_steps` at
  its monitor data's offset 8, so a run of `n` vCPUs may run up to `n` times `max_steps` before a `Stop::StepLimit`;
  `executed` sums the counts. One vCPU is as before. The family runs, which end a hang by count, run on one vCPU.
- **The gates:** `scripts/gates.sh` exit 0 in 1252 s, kept in `store/gates/passes.tsv` as sprint 327 at 411435de3 with root=0bc8a16f5 and "rust=crates python=0 samples=0", 211 tests, 651 ns an exit at load 3.13, base=6cdcc8e78, Sprint 326's close; multi-vCPU: not run (Python gates not run)

