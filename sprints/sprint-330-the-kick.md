# Sprint 330 — The kick

```yaml
---
id: 330
status: active
phase: 6
pass_kind: functional
---
```

## scope

The run-loop design's card 4c3b3, "The kick" (the Python host's Sprint 119): SuspendThread of a thread another vCPU
holds stops that vCPU (`VcpuHandle::stop`) and waits, the word and the guard let go, until the thread's `vcpu` word
clears; the vCPU stopped leaves through the host, its `SLICE_UNTIL` 0 and the C's switch declining the suspended
thread. Today `threads::suspend` (`threads.rs`) raises the count alone, and the thread runs on to its vCPU's next
switch. Measured before this card, the threads family probe (`store/family_runs/known-2026-10-29.tsv`'s `threads`
row, through `w2m-vm` with its arguments) on one vCPU and on two: the spinner, on vCPU 1 while the main thread loops
on SwitchToThread, kept counting after SuspendThread, `suspend.frozen`'s `count_unchanged` 0 where one vCPU reads 1,
and GetThreadContext read it with `rip_in_image` 0 where one vCPU reads 1; every other row the same.

- **The work:** `suspend` of a thread whose run entry's `vcpu` word names another vCPU stops that vCPU through its
  handle and waits, holding neither the word nor the lock, until the word clears; the stopped vCPU, at its cancel
  with the run not stopped, leaves through the host to a switch, which puts the suspended thread down; the run's
  vCPU handles reachable from a host request (`RunStop`'s, by owner). A cancel that is not the run's stop is no
  longer read as the time limit. Card 4c3b3's run of guests on two vCPUs also takes Deferred (Sprint 329):
  `model_steps`, `mutual`, `mutual_context`, `qpc2` and `readied` join `through_w2m_vm`, each held to
  `store/batch_run/first-stop-2026-10-28.tsv`.
- **Not here:** every other vCPU stopped for a locked model step or a split (4d); the Machine's own thread (4f).

## prerequisites

- 329 (closed)

## context_files

- `docs/design-run-loop-on-several-vcpus-2026-10-31.md` "A vCPU with no thread", the kick (196-198);
  `crates/w2m_vm/src/threads.rs` `suspend`, `switch`, `E_VCPU`, `E_SUSPEND`; `crates/w2m_vm/src/run.rs` `RunStop`,
  `vcpu_loop`'s cancel; `probes/threads/threads.c` 164-194; BLACKBOARD Deferred (Sprint 329)

## signal contract

### Emits

- None; when a suspended thread stops.

### Invariants

- Every run on one vCPU as held; on one vCPU no thread is held elsewhere, and `suspend` is as today.

## artifact contract

### Files modified

- `crates/w2m_vm/src/threads.rs`, `crates/w2m_vm/src/run.rs`, `crates/w2m_vm/tests/run_loop.rs`; the design's card 4
  note, with the work

### Command exit codes

- `scripts/gates.sh` returns 0

## observation contract

- **Expected values:** a test running the threads family probe through `w2m-vm` on two vCPUs, with the arguments its
  `known` row gives: `suspend.frozen`'s `count_unchanged` 1 and `context`'s `rip_in_image` 1, held to those values,
  and every row equal to the one-vCPU run's; the five guests on two vCPUs ending with their held stop and output;
  every run on one vCPU as held.
- **Planted faults:** (1) the stop of the holding vCPU taken out of `suspend`: the test reads `count_unchanged` 0, as
  measured before this card; (2) the wait for the `vcpu` word taken out: the context read before the vCPU has put the
  thread down, `rip_in_image` 0. Each judge's reach read before this card's commit (WORKING_AGREEMENT, Sprint 284):
  the two rows were run on two vCPUs without the kick and read 0, where one vCPU reads 1.

## done criteria

A thread suspended while another vCPU runs it stops before SuspendThread returns, as on one vCPU.

## self-review checklist

- [x] Scope names one verifiable property: a suspended thread on another vCPU stops.
- [x] context_files cite the design, the suspend, the run's stop and the probe's rows.
- [x] Emits: none.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values held to known values and the one-vCPU run, two plants, each judge's reach run.
- [x] One concept: the kick.

## result

- **The kick:** `host_request::serve` answers a SuspendThread that succeeded on a thread another vCPU runs
  (`threads::held_elsewhere`: the run entry's `vcpu` word names an owner not the served vCPU's) with
  `Served::Kick { owner, entry, answer }`. `vcpu_loop` drops the lock and calls `kick_and_wait` (`run.rs`): the
  guest's word let go (`monitor_code::release_guests_word`, renamed from `release_ended_vcpus_word`, now answering
  whether it let go), the holder cancelled through `RunStop`'s handle by owner (`RunStop::kick`), a wait until the
  entry's `vcpu` word leaves that owner, the holder is out of guest code, or the run stops; then the word taken again
  for the guest (`take_word_for_guest`) and the answer written back into the transport page. The cancel, without the
  run's stop flag, is a kick: `vcpu_loop` goes back to the turn's start, which ends the suspended thread's slice
  (`slice_until`), and the C's switch puts it down. `counts.cancelled` no longer means the time limit, which stops
  the run through `RunStop` with the flag set first. `hvc #4` is served for every process.
- **Beyond the card, from reviewer-3's Medium 1 at 330** (Surfaced): two vCPUs each suspending the other's thread
  waited on each other for ever, a vCPU in a host service never taking its pending cancel. `RunStop` keeps an
  in-guest flag for each vCPU, set around `vcpu.run()`, and the wait ends when the holder is out of guest code; the
  cancel stays pending, so the holder's next run returns at once. Measured with `w2m-vm --vcpus 2` on
  `tests/data/timed_waits/mutual_context.exe`: the wait on the word alone stopped at the run's 60-s time limit with
  no output; with the fix, 20 runs of 20 ended `ExitProcess { code: 0 }`, output "done", in 3 s. A loop of 50 runs
  each way was stopped once the hang showed, before it printed its tally. Its checks at 14:43:45 (a run 15 s in) and
  14:48:24 (a run 54 s in) found no stop file, which first read as a run that ended writing none (reviewer-3's Low 1 at
  330's result); each check fell inside a run in flight, and `w2m-vm` clears an earlier run's stop file and output at
  its start (`run_outputs::clear`, `w2m-vm.rs` 193; Sprint 289), so the file was the running run's to write. A solo
  run of the same build, its stderr kept, wrote "the run's time limit".
- **Beyond the card, from reviewer-3's Low 1 at 330:** `w2m_host` reads the answer from the transport page after its
  `hvc #8` (`runtime.c` 26-36), and another vCPU's request while the word was let go could overwrite it; `Served::Kick`
  carries the answer and `kick_and_wait` writes it back once the word is the guest's again.
- **The tests:** `run_loop::a_thread_suspended_on_another_vcpu_stops_before_suspend_returns` runs the threads probe
  through `w2m-vm --vcpus 2` with its `known` row's arguments: `suspend.frozen`'s `count_unchanged` 1, `context`'s
  `rip_in_image` 1, every row equal to the one-vCPU run's. `run_loop::the_other_single_process_guests_end_on_two_vcpus_as_held`
  runs `model_steps`, `mutual`, `mutual_context`, `qpc2` and `readied` on two vCPUs, each held to
  `store/batch_run/first-stop-2026-10-28.tsv`'s stop and output (Deferred (Sprint 329) met).
  `run::tests::a_kick_gives_the_guest_back_its_word_and_its_answer`: after `kick_and_wait` the word is `owner_of(0)`
  and the transport holds the answer, 7.
- **Plant 1,** `run.rs` copied aside, restored by `cp`, checked by `cmp`: `if false && place.stopping.kick(owner)`; the
  probe test failed, "the suspended thread went on: {"s":"suspend.frozen","count_unchanged":0}".
- **Plant 2, not caught:** the wait made `while false && mem.read_u64(entry + …E_VCPU) == …`; the probe test passed,
  nextest exit 0. The card named this judge from a run without the whole kick, not a run with the wait alone taken
  out, so its reach was reasoned, not read: a breach of WORKING_AGREEMENT's plant-judge rule (Sprint 284). The probe
  takes the spinner's count after a second SuspendThread, itself a host request, and compares it after a `Sleep(20)`;
  why the spinner did not count on in the planted run, one run, is not measured. No test reaches the wait; Deferred
  (Sprint 330) names a guest that reads the suspended thread straight after SuspendThread, at card 4d.
- **Plant 3,** restored likewise: the out-of-guest exit made `&& true`; the five-guest test failed, `mutual` on two
  vCPUs ending "the run's time limit". The card expected `mutual_context`; `mutual` came first in the test's order.
- **Plant 4,** restored likewise: the answer's write-back made `let _ = answer;`; the unit test failed, "assertion
  `left == right` failed: the kick's answer in the transport", left 2989, right 7.
- **A slip in the audit:** a transcript search at the result's settling ran `python3 -c 1`, a no-op left in a shell
  loop, against the project's rule that no Python runs; stopped by `TaskStop`, nothing read from it.
- **One class closed:** three test compiles failed this stretch on a layout name outside `run.rs`' test module
  (`RUN_TABLE` at 329, `MONITOR_SIZE` and `TRANSPORT_OFF` here), each mended by a full path; the module now imports
  the three once and the paths are gone, `cargo check -p w2m_vm --tests` exit 0. The import went in by a `sed` whose
  replacement held a newline, against the rule that a multi-line edit goes by the Edit tool; a slip, recorded.

