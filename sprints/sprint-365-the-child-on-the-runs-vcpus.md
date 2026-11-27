# Sprint 365 — The child on the run's vCPUs; the run loop's own refusal of the instruction clock above one vCPU

```yaml
---
id: 365
status: done
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

A child started by a several-vCPU run runs on the run's vCPUs: the six runs that start a child conform in 120 of 120
runs on two vCPUs with every child on two, the pipe ping-pong across two processes ends in 20 of 20, the run loop
itself refuses the instruction clock above one vCPU by name, and one vCPU is unchanged.

**The class** (file:line at 4ba3d0117; what each does after this card):
- `vcpus_allowed` (`run.rs` 658): called at `w2m-vm.rs` 347 (the options) and 432 (a family's start after the
  wall-clock fallback at 426), both with `child` false, and by its test (`run.rs` 2373-2388); its refusal of a child
  (668-670) was reached by no caller. Now (`run.rs` 661) it has no `child` parameter, and asks
  `instruction_clock_allowed` (690) for the clock; its callers `w2m-vm.rs` 349, 435 and 875 (a child's start).
- `run_compiling` (`run.rs` 1153) and `run_to_first_decline` (620), both into `run` (1172): callers `w2m-vm.rs` 655
  (`on_one_vcpu`, called at 474, 744, 781, 1089), `run_loop.rs` 154, 1400, 1910, 1991, `first_decline.rs` 52; none
  set the clock with two vCPUs. Now `run` refuses the clock above one vCPU before any thread (`run.rs` 1203), by
  `Stop::Exit` with `instruction_clock_allowed`'s words, for every caller.
- The child's start: `child_start` (`host_request.rs` 228, called at 271) carried no vCPU count; now it carries
  `"vcpus"` (242). Read by `w2m-vm.rs` `child_guest` (797), which left `Start::vcpus` at `Start::console`'s 1
  (`process.rs` 134) and now sets it before `guest_of` (873; `Guest::new` reads it at `guest.rs` 961,
  `isb_before_entry`) and checks it by `vcpus_allowed` (875); `child` (719, the turn form) refuses a count other
  than one by name (750); `child_of_the_request_server` (761) runs on it. `turns.rs` 447 (`Supervisor::spawn`, one
  vCPU) and `cross_process.rs` 376 (`Server::spawn`) pass the start on unchanged.
- What assumed a child on one vCPU: `on_one_vcpu` (`w2m-vm.rs` 641), which ran `start.vcpus`, renamed
  `run_on_its_vcpus` (645) with its doc, and it prints "w2m-vm: process <pid> on <n> vCPUs, runs by vCPU [..]"
  (681), so a child's count shows on the stderr it shares with its parent; comments `w2m-vm.rs` 325-326, 716-718,
  756-760, `cross_process.rs` 20-21 and 372-375, `family_runs.rs` 23-26 and 1357-1358, `run_loop.rs` 1247-1249. The
  clock: a several-vCPU parent's start has none (`w2m-vm.rs` 426 takes it before `guest_of`), so `child_start` gives
  none, and `catch_up` is the turn form's alone (744). The budget: `run.rs` 1212 takes `max_steps` off turns. The
  park: `threads.rs` 789 counts parked against the run's vCPUs, the post per process. The link thread: `run.rs` 1255
  starts it for any several-vCPU run. The slot: `cross_process.rs` 377, one per child, one request in flight per
  process since the runtime word covers `hvc #8`; unchanged. No defect that stops a run or leaves a thread past its
  deadline was found.

**What changed.** `crates/w2m_vm/src/run.rs`, `host_request.rs`, `bin/w2m-vm.rs` as above; `cross_process.rs` and
`tests/family_runs.rs` comments; `tests/run_loop.rs` the two tests below (1324, 1369); `tests/batch_run.rs` 333 skips
the frozen-host output comparison for a guest the frozen host never ran ("-" in both columns), since the new guest
ends; `store/batch_run/first-stop-2026-10-28.tsv` 62 the new guest's one-vCPU stop, `ExitProcess { code: 0 }`.
Created: `tests/data/timed_waits/two_process_pipe_pingpong.c` and `.exe` (`x86_64-w64-mingw32-gcc` 16.2.0, `-O2
-nostdlib -e start -s -lkernel32`), the pipes and the child's start as `probes/processes/processes.c` makes them; it
exits 0 only when its 10,000 rounds and the child's exit code 7 are right.

**Observed.**
- The six runs on two vCPUs, children on two: every family run made on one and two vCPUs as `scripts/gates.sh` makes
  them, then the six made again on two vCPUs 19 more times, `family_runs.rs`'
  `every_family_run_conforms_or_is_held_by_what_it_waits_on` run after each of the 20 (03:45:08-03:48:27 UTC): 120
  of 120 stops `ExitProcess { code: 0 }`; every process "on 2 vCPUs" in every run ("w2m-vm: process 4100 on 2
  vCPUs;w2m-vm: process 4096 on 2 vCPUs" for `process_parameters` and `time_and_error_mode`, 4100, 4104 and 4096 for
  `processes`); 20 of 20 judge exits 0, each "273 of 273 family runs conform, 68 of 68 families in every run" and "on
  two vCPUs: 273 of 273 held as on one vCPU, 273 of them conforming; 0 refused by name ()";
  `{"s":"pipe.capacity","written_before_read":4096,"read":65536,"writer_done":0}` and
  `{"s":"process.exited","wait":1,"wait_0":0,"exit_code":7}` in 40 of 40 `processes` runs; `pgrep -fl w2m-vm` found
  none after each repetition.
- The ping-pong guest on two vCPUs, `a_pipe_pingpong_across_two_processes_on_two_vcpus_ends_in_every_run`: 20 of 20
  "ExitProcess { code: 0 }", output "rounds 10000, child 7\n", 346 to 700 ms each, both processes on 2 vCPUs (run 1:
  "w2m-vm: process 4100 on 2 vCPUs, runs by vCPU [24566, 23996]", "w2m-vm: process 4096 on 2 vCPUs, runs by vCPU
  [20771, 20690]"), none of the group left. On one vCPU through `w2m-vm`: "ExitProcess { code: 0 }", "rounds 10000,
  child 7".
- The refusal, `the_run_loop_refuses_the_instruction_clock_above_one_vcpu`: "qpc on the instruction clock, 1 vCPU(s):
  stop ExitProcess { code: 0 }, 149 instructions", "qpc on the instruction clock, 2 vCPU(s): stop Exit(\"2 vCPUs
  refused: the instruction clock reads the running vCPU's own count\"), 0 instructions".
- The card's plant 1 as written, the link thread's wake without the woken mark (`threads.rs` 691
  `mark_woken_for(guest, 0)` replaced by `let _ = mark_woken_for;`), live 03:48:48-03:50:28 UTC: the ping-pong judge
  passed, 0 of 20 hanging, "rounds 10000, child 7" in each; `pingpong` (named events across two processes) 20 runs
  at once on two vCPUs, 20 of 20 "ExitProcess { code: 0 } / done". The source says why: a wait across processes
  checks the host's answer and blocks inside one holding of the runtime word (`w2m_pipe_read`,
  `tools/guest_runtime/processes.c` 71-89, PIPE_READ by `hvc #8` then `w2m_block` on "would block"), and the link
  thread takes that word before it applies a wake (`run.rs` `apply_wakes`, `take_word_for_guest(HOST_OWNER)`), so the
  wake lands before the check or after `w2m_block` set BLOCKED, where it readies the thread; the window the mark
  covers is not reached by a wake from another process until C2 takes host requests off the word. Replaced by the plant below.

**Plants.**
- **Plant 1** "two_process_pipe_pingpong: 20 of 20 runs on two vCPUs failed", `run_loop.rs`
  `a_pipe_pingpong_across_two_processes_on_two_vcpus_ends_in_every_run` exit 100; live 03:51:51-03:52:48 UTC:
  - the replacement of the card's plant 1 (above): the link thread's wake readying no BLOCKED thread;
    `crates/w2m_vm/src/threads.rs` copied to `plants/365/threads.rs.orig-1b` in the scratchpad; the readying loop of
    `wake_from_another_process` replaced by `let _ = blocked; // PLANT 1 (replacement): no BLOCKED thread readied`,
    found by grep at 688;
  - the judge run through nextest, `target/debug/w2m-vm` rebuilt at 03:52:04 UTC: every run "Handover: every thread
    waits with no timeout and nothing can wake one (a deadlock)", output "", in 1.82 to 2.24 s, the request server's
    verdict once both processes parked;
  - restored by `cp`, `cmp` equal, `git diff` of `threads.rs` empty.
- **Plant 2** "assertion `left == right` failed: qpc on the instruction clock, two vCPUs: the run loop's own refusal
  / left: ExitProcess { code: 0 } / right: Exit(\"2 vCPUs refused: the instruction clock reads the running vCPU's own
  count\")", `run_loop.rs` `the_run_loop_refuses_the_instruction_clock_above_one_vcpu` exit 100; live
  03:51:04-03:51:25 UTC:
  - today's form of the refusal, in `w2m-vm` alone: `crates/w2m_vm/src/run.rs` copied to `plants/365/run.rs.orig`;
    the run loop's check made `if false && let Err(why) = instruction_clock_allowed(...)`, found by grep at 1203;
  - the judge run: "qpc on the instruction clock, 2 vCPU(s): stop ExitProcess { code: 0 }, 149 instructions", then
    the failure quoted;
  - restored by `cp`, `cmp` equal.

`scripts/gates.sh` on the finished tree, no plant live: exit 0 in 429 s	default	2026-11-27T04:00:53Z	sprint 365	4ba3d01174f258858c04dad7ad559533e227f9a4	3ee7ad3ad5a7675fc948acc4f052e63bcb26e70cbd55748eb7713c7dc4aa5777	root=8ede16ea4d263f2f9a723acd987bfa8445d8773a	rust=all python=0 samples=0	base=dec44f4d651586f685c686f108c7d520d9112d10 (429 tests run: 429 passed, 1 skipped).
