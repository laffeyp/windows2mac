# Sprint 362 — The objects that cross processes behind their own lock, `kernel`

```yaml
---
id: 362
status: done
phase: 6
pass_kind: functional
---
```

## scope

Card 4f1 of `docs/design-a-process-that-starts-another-2026-11-26.md` ("The cards that follow", item 1). The design is
the spec. `kernel.rs`' `Objects` holds what processes share: named events, pipes, and a record per process. Today it
lives in the first process's `Guest` (`served.kernel`), and the turn form's `Supervisor` with it. `turn` moves both out
of the `Guest` and back in around each hand-over (`run.rs` 948-949, 969-972 at ae82b92ba). This card moves them into one
mutex beside `GuestLock`, the host design's Change 4 `kernel` lock: "`kernel`, for processes, cross-process names,
handles and pipes, one request at a time" (`docs/design-rust-host-2026-10-24.md` 289).

- **The lock.** A KERNEL 33 service takes `kernel` under its runtime word and the run's lock, as a leaf held for one
  request. `turn` no longer moves the objects out. A spawn is made under `kernel`, since the children's table is under
  it too.
- **The guard.** `kernel`'s guard asserts that its taker holds a runtime word, or is the server (none exists until card
  4f2), in release builds too, as `GuestLock::serve` asserts the word.
- **The class first.** List every reader and writer of `Objects` and the `Supervisor` today, in `host_request.rs`,
  `run.rs`, `turns.rs`, `kernel.rs` and the exit ledger, one line each with file:line and what it holds. Each takes
  `kernel`, or the list says why it need not. Report each defect found to the Supervisor at once, with its evidence.
- **The lock order**, as the design states it: in a vCPU thread, the runtime word, the run's lock, then `kernel`, a
  leaf. Nothing holding `kernel` waits for a word or the run's lock.
- **One vCPU and every run unchanged.** The turns stay, and `MACHINE_ON_ONE` stays. On two vCPUs the six held runs are
  still refused by name.
- **Not here:** the request server (card 4f2); any run of processes at once (4f3).

## prerequisites

- 361 (closed)

## context_files

- `docs/design-a-process-that-starts-another-2026-11-26.md`: in full.
- `crates/w2m_vm/src/kernel.rs`, `host_request.rs` 130-250 and 500-640, `turns.rs`, and `run.rs` 790-1000, 1500-1540,
  1800-1830 and 1930-1965.
- `crates/w2m_vm/src/guest.rs` (`GuestLock`, `served`) and `crates/w2m_vm/src/exit_ledger.rs` (any reader of the
  objects).
- `crates/w2m_vm/tests/family_runs.rs`, the `twoproc` guests' tests, and `store/family_runs/two-vcpus-refused-2026-11-26.tsv`.
- `grep -n "The form of card 4f on several vCPUs" BLACKBOARD.md`: the Decision.
- `WORKING_AGREEMENT.md` lines 137-266, "Build and verification commands".

## signal contract

### Emits

- None: where the shared objects live and which lock covers them; no emit site changes. Every run's records are
  unchanged.

### Invariants

- The batch run conforming, the 273 family runs on one vCPU and two as held (the six still refused by name), the
  `twoproc` guests and the exit ledger, all unchanged.

## artifact contract

### Files modified

- `crates/w2m_vm/src/host_request.rs`, `run.rs` and the field in `guest.rs` (the design's three files for 4f1), with
  `turns.rs` or `kernel.rs` only where the class list requires. One concept: the shared objects behind `kernel`.

### Command exit codes

- `scripts/gates.sh` returns 0
- `target/release/signal_report --check` returns 0 on the tree

## observation contract

- **Expected values:**
  - The class list, one line for each reader and writer.
  - Every run as held on one vCPU and two.
  - The guard's assertion's text.
- **Planted faults:**
  - (1) A KERNEL 33 service that takes `kernel` without its runtime word. The guard's assertion names it, in a test
    that drives that service.
  - (2) `turn` moving the objects out again beside the lock, as today. A test that holds `kernel` across a hand-over
    fails, or the `twoproc` guest's run differs.

## done criteria

The objects that cross processes live behind one leaf lock, `kernel`, taken under the runtime word and the run's lock.
`turn` no longer moves them, and every run is unchanged on one vCPU and two.

## self-review checklist

- [x] Scope names one verifiable property: the shared objects behind `kernel`, every run unchanged.
- [x] context_files cite the design, the code that moves the objects, the guard's precedent and the judges.
- [x] Emits: none, with its reason.
- [x] Artifact contract gradable by command.
- [x] Observation contract: the class list, the runs, the assertion; numbered plants.
- [x] One concept: where the shared objects live.

## result

- **The class, at e0e8fd619: every reader and writer of `Objects` (`served.kernel`) and of the link that holds the
  `Supervisor` (`served.link`), and what each takes now.**
  - `guest.rs` 776, 778, 1061-1062: the two `Served` fields, made at load (`Objects::new()`, `None`). Now one field on
    `Guest`, `kernel: Arc<run::KernelLock>` (`guest.rs` 733), made at load with no vCPU and so no lock.
  - `host_request.rs` 523-526: the supervisor's own KERNEL 33 moved the objects out, served, put them back. Now
    `kernel_request` (256-307) takes `kernel` (259) under the guest's word and the run's lock for the one request.
  - `host_request.rs` 537-545: the link made and the child spawned. Now under the same take: the children's table is
    in it.
  - `host_request.rs` 547, 551-552: `set_budget`, then `wake_children` under the wake. `wake_children` under the take;
    `set_budget` and `wake_blocked` after `kernel` is let go (they read this process's own state, and `set_budget`
    reads `linked`, below).
  - `host_request.rs` 165: a child's request read `Link::Child` and waited for the turn back. Now under `kernel`
    (167), held for the request and let go before the wake.
  - `run.rs` 948-949, 960, 963-964, 971-972: `turn` moved the link and the objects out across the hand-over. Now
    `turn` takes `kernel` (1020) under the word and the lock, lets go the lock and the word, runs the hand-over from
    the guard, and lets `kernel` go (1040) before the word is taken again.
  - `turns.rs` 486, 568, 592, 608-638: `hand_over`, `serve`, `objects.request` and `objects.end`, all on the objects
    `turn` hands in; now the guard's, so they run under `kernel` with no change in `turns.rs`.
  - `run.rs` 1524-1528 (now 1595) and 1937 (now 2010): `others_due` at a turn's start and `others_ready` at a slice's
    end. Now under `kernel`, a leaf inside the word and the lock, when a link is made.
  - `threads.rs` 1430-1432 (now 1428): `slice_until`'s `others_due`. Now under `kernel`, when a link is made.
  - `run.rs` 1040 (now 1110), `threads.rs` 1035 and 1407: whether a link is made (the run's first budget, the
    switch's `Switch::Idle`, `set_budget`'s slices). These read `KernelLock::linked`, an `AtomicBool`, and take no
    lock: `set_budget` runs from `create_thread` at load and in tests, with no word. A link is installed once and kept
    for the run, so `linked` is set once at the two installs and never cleared: `KernelLock::install` (`run.rs` 983),
    the link put in under the spawn's take and true stored (Release) beside it (`host_request.rs` 278), and `set_link`
    (989) for a child before its run. (It was first a copy of `link.is_some()` stored at every guard's
    drop.)
  - `bin/w2m-vm.rs` 796: the child's link installed before its run. Now `KernelLock::set_link` (799) through
    `Arc::get_mut`, no vCPU yet.
  - `tests/run_loop.rs` 1015: the refusal leaves no link. Now `!guest.kernel.linked()`.
  - `kernel.rs`: the type itself; nothing outside it holds the objects. `exit_ledger.rs`: no reader of the objects;
    the kernel's writes reach it through `KernelMemory` (`host_request.rs` 266) and `child_request` (176-184), both
    unchanged.
  - `tests/guest_parts.rs` 41 holds `Served` to the run-loop design's table: its `kernel` and `link` rows now read part
    *Kernel* ("Served until Sprint 362"), the count 22 Served and 2 Kernel, and the test 22.
- **The lock.** `run.rs` 930-993, beside `GuestLock`: `Kernel { objects, link }` in a `Mutex`, its guard the
  `MutexGuard` (`KernelGuard`). The order: the runtime word, the run's lock, then `kernel`, a leaf. No site that holds
  `kernel` takes a word or the run's lock: `turn` drops it before its retake (1040), `kernel_request` before
  `set_budget` and `wake_blocked`, `child_request` before its wake. One vCPU takes it uncontended; no link exists on two
  vCPUs, since a spawn there is still refused (`MACHINE_ON_ONE`, unchanged).
- **The guard.** `KernelLock::take` (968-974) asserts, before the lock, with `assert!`, so in release builds too:
  `"the kernel lock taken without a runtime word"`, from `runtime_word_held` of the taking vCPU's owner, as
  `GuestLock::serve` asserts the word. Card 4f2's server, which holds no word, takes it its own way; none exists yet.
- **The judges.** `host_request.rs` 769, `a_kernel_request_takes_kernel_only_under_the_runtime_word`: PIPE_CREATE under
  vCPU 0's guest's word answers the two ends from `kernel`; with the word free the assertion's text; the lock left free.
  `host_request.rs` 818, `a_spawn_installs_the_link_and_sets_linked_beside_it`: no link and `linked` false before;
  after the supervisor's own PROCESS_CREATE, under one take of `kernel`, `linked` true and the link the supervisor's
  (the child is this test binary started `--child`, which refuses the argument and ends, so the spawn answers the
  child's id 4100 or its start line's write failure; the link is installed before either). Ten runs, all passed;
  `pgrep` found no `--child` process left.
  `run.rs` 2481, `a_turn_holds_kernel_across_its_hand_over`: `turn` gives the turn to a child (a `/bin/sleep 60`
  adopted through the test seam `Supervisor::adopt`, `turns.rs` 660, `#[cfg(test)]`, its `w2m-vm` a `ChildLink` on the
  page), and while it waits `kernel` is held (`TryLockError::WouldBlock`); the child's stop ends the turn with
  "child process 4100: the test's child stops"; after, the link and the objects are back. Ten runs each, all passed;
  `pgrep` found no `sleep 60` left.
- **Plant 1** "the kernel lock taken without a runtime word", `a_kernel_request_takes_kernel_only_under_the_runtime_word`
  exit 100; live 00:27:40-00:28:01 UTC:
  - `host_request.rs` copied to the scratchpad; by the Edit tool, `release_guests_word` of the serving vCPU's owner
    before `kernel_request`'s take, the guest's word let go; `grep -n "PLANT 1"` found it at 262.
  - The judge panicked at `run.rs` 990:9 with the guard's text, its first `kernel_request` failing.
  - Restored by `cp`; `cmp` silent; no `PLANT` left.
- **Plant 2** "kernel free while the turn waits in its hand-over", `a_turn_holds_kernel_across_its_hand_over` exit 100;
  live 00:28:09-00:28:31 UTC:
  - `run.rs` copied to the scratchpad; by the Edit tool, `turn` took `kernel`, moved the objects and the link out into
    a local, let it go, ran the hand-over on the local, and put them back after its retake; `grep -n "PLANT 2"` found
    it at 1037 and 1066.
  - The judge panicked at `run.rs` 2544:9 with its message. The `twoproc` guest's run would not differ: on one vCPU no
    other thread takes `kernel` during a hand-over, which is why the judge looks at the lock itself.
  - Restored by `cp`; `cmp` silent; no `PLANT` left.
- **Every run as held.** The gates' row below, which holds the batch run conforming (`batch_run.rs`), the 273 family
  runs on one vCPU and on two (`family_runs.rs` `every_family_run_conforms_or_is_held_by_what_it_waits_on`, the six
  in `two-vcpus-refused-2026-11-26.tsv` still refused by name at `MACHINE_ON_ONE`), the `twoproc` guests (on the wall
  clock and on the instruction clock) and the exit ledger. No plant was live while it ran.
- **Files:** `crates/w2m_vm/src/run.rs`, `host_request.rs`, `guest.rs` (the card's three); `threads.rs`,
  `bin/w2m-vm.rs`, `tests/run_loop.rs` (the link's readers the card's list missed, found by the class list);
  `turns.rs` (the test seam only); `tests/guest_parts.rs` and `docs/design-run-loop-on-several-vcpus-2026-10-31.md`
  (the Served table). The scope widened by the class list. No C changed, so the runtime image is
  HEAD's. The plants ran before `linked` moved to its two installs; neither judge's code changed after.

exit 0 in 420 s	default	2026-11-27T00:48:22Z	sprint 362	e0e8fd619071695b48e6b15530debd8ab5dc4dcc	e0d508aaf36470008556a2f83ad267ca6b018e914d6537dd747a0ca7fabb936f	root=72f6c7f42ad4b0f7de930fa271d0cba82a4fc6be	rust=crates python=0 samples=0	base=d06948a579feed427d6a8adf814acb4f1dab75ad
