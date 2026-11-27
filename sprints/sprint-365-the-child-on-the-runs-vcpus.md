# Sprint 365 — The child on the run's vCPUs; the run loop's own refusal of the instruction clock above one vCPU

```yaml
---
id: 365
status: active
phase: 6
pass_kind: functional
---
```

## scope

Card 4f4 of `docs/design-a-process-that-starts-another-2026-11-26.md` ("The cards that follow", item 4). The design is
the spec. Since Sprint 364 a process that starts another runs at once with its child on several vCPUs, but the child
runs on one: `child_start` gives it no vCPU count and `vcpus_allowed` refuses a child above one ("a child process runs
on one", `run.rs` 658-690). This card runs the child on the run's vCPUs. G3's batch run needs it, since the game starts
UnityCrashHandler64.exe as a child.

- **The child's count.** `child_start` (`host_request.rs` 228) carries the run's vCPU count, and `w2m-vm --child` runs
  the several-vCPU `run` with it. `vcpus_allowed`'s refusal of a child goes for a child started by a several-vCPU run;
  a child of a one-vCPU run stays on one.
- **The run loop's own refusal of the instruction clock** (reviewer-4's thirty-third review, Low 1, taken into this
  card): today only `w2m-vm` (347, 432) calls `vcpus_allowed`, and `run_compiling` (`run.rs`) does not, so any other
  caller of the run loop can start the instruction clock on two vCPUs, which the Decision "The instruction clock on
  several vCPUs" rules out. The run loop refuses it by name for every caller; `w2m-vm`'s fallback to the wall clock for
  a family's start stays, applied before it calls the run loop.
- **The class first.** List every caller of `vcpus_allowed`, of `run_compiling` and of the child's start, and every
  place that assumes a child runs on one vCPU (its clock, its budget, `on_one_vcpu`, the link's slot), with file:line
  and what each does after this card. Report each defect found to main at once with its evidence; fix any that can stop
  a run or leave a thread past its deadline here.
- **Not here:** C2 (host requests off the runtime word), the batch run on two vCPUs (G3).

## prerequisites

- 364 (closed)

## context_files

- `docs/design-a-process-that-starts-another-2026-11-26.md`: in full.
- `crates/w2m_vm/src/host_request.rs` 220-320, `crates/w2m_vm/src/run.rs` 640-700 and 1060-1300 (`vcpus_allowed`,
  `run_compiling`, `run`), `crates/w2m_vm/src/bin/w2m-vm.rs` 300-480 and 700-860 (the options, the family start's
  fallback, the child), `crates/w2m_vm/src/cross_process.rs` (the client side), `crates/w2m_vm/src/threads.rs` 660-830
  (the wake from another process, the park post).
- `crates/w2m_vm/tests/run_loop.rs` (the two-vCPU tests and `two_process_deadlock`'s), `crates/w2m_vm/tests/family_runs.rs`,
  `tests/data/timed_waits/two_process_deadlock.c` (the form for this card's ping-pong guest), and
  `tools/guest_runtime/threads.c` 97-129 (`w2m_block`, the woken mark).
- `grep -n "The instruction clock on several vCPUs\|Processes at once on several vCPUs, card 4f3\|G2 met at Sprint 364\|thirty-third review" BLACKBOARD.md`:
  the Decisions and the review.
- `WORKING_AGREEMENT.md` lines 137-266, "Build and verification commands".

## signal contract

### Emits

- None: how many vCPUs a child runs on, and where the clock's refusal lives; no emit site changes. The family runs'
  records are unchanged.

### Invariants

- One vCPU unchanged: the batch run conforming, the 273 family runs on one vCPU as held, the `twoproc` guests and the
  exit ledger.
- On two vCPUs: the 273 family runs held as on one, as at Sprint 364 (G2).

## artifact contract

### Files created

- A two-process guest under `tests/data/timed_waits/` (source and built image): a parent and a child pass one byte
  back and forth over two pipes, 10,000 rounds, every read INFINITE.

### Files modified

- `crates/w2m_vm/src/host_request.rs`, `crates/w2m_vm/src/bin/w2m-vm.rs` (the design's two files for 4f4), and
  `crates/w2m_vm/src/run.rs` (the clock's refusal), with others only where the class list requires, reported to main
  when found. One concept: the vCPU count a run's processes get, and the refusal that bounds it.

### Command exit codes

- `scripts/gates.sh` returns 0
- `target/release/signal_report --check` returns 0 on the tree

## observation contract

- **Expected values:**
  - The class list.
  - The six runs that start a child (`process_parameters` 38194049597 and 38194051809, `processes` 38189162864 and
    38189164581, `time_and_error_mode` 38206225386 and 38206227300) on two vCPUs, children on two, 20 runs each, all
    conforming, judged by `family_runs.rs`, quoted.
  - The ping-pong guest on two vCPUs ending at its own exit in 20 of 20 runs, with 10,000 rounds counted, quoted.
  - A `run_compiling` call with two vCPUs and the instruction clock refused by name, quoted.
- **Planted faults:**
  - (1) The link thread's wake without the woken mark: a wake from another process readies BLOCKED threads only, so a
    thread past its check and not yet parked misses it. The ping-pong guest hangs to its limit in at least one of 20
    runs on two vCPUs; quote how many of the 20 hang. If none of 20 hangs, report that to main before going on: the
    judge does not see the fault.
  - (2) Today's form of the clock's refusal (in `w2m-vm` alone): the `run_compiling` test runs the instruction clock on
    two vCPUs and its refusal check fails.

## done criteria

A child started by a several-vCPU run runs on the run's vCPUs, the six runs conform with their children on two vCPUs,
a ping-pong over pipes across two processes ends in every run, and the run loop itself refuses the instruction clock
above one vCPU for every caller; one vCPU is unchanged.

## self-review checklist

- [x] Scope names one verifiable property: a run's processes on its vCPUs, the clock's refusal bounding it.
- [x] context_files cite the design, the child's start, the refusal, the wake and the guests.
- [x] Emits: none, with its reason.
- [x] Artifact contract gradable by command.
- [x] Observation contract: the six runs, the ping-pong guest, the refusal; numbered plants, plant 1's count of 20.
- [x] One concept: the vCPU count a run's processes get, and its refusal.

## result

GATES-LINE
