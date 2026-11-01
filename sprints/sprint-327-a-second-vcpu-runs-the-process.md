# Sprint 327 — A second vCPU runs the process

```yaml
---
id: 327
status: active
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

