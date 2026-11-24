# Sprint 331 — Every other vCPU stopped for a locked model step

```yaml
---
id: 331
status: done
phase: 6
pass_kind: functional
---
```

## scope

The run-loop design's card 4d, split by rule 6: 4d1, here, the stop of every other vCPU for a model step of a form
x86 locks (the design's "A model step of a locked form", precondition 8); 4d2, the stop for a block split, a cancel
from another thread mid-split (Deferred, Sprint 305) and the guest Deferred (Sprint 330) names for the kick's wait.
Today a misaligned locked form runs in the model (`hvc #11`, `crates/translator/src/lower.rs` 1424-1432) as a load
and a store, under the runtime word and the lock (`run.rs` 1696-1716, Sprint 324), so two misaligned steps on two
vCPUs already run one at a time; an aligned locked form runs in the VM as an exclusive pair, which takes no word, so
an aligned add on another vCPU to a byte the step covers, made between the step's load and its store, is written
back over; x86 makes each access atomic at any alignment (Deferred, Sprint 272; the design's "A model step of a
locked form": "The word would not exclude the aligned pair").

- **The work:** `RunStop` stops every other vCPU before a model step of a locked form and waits until each is out of
  guest code (Sprint 330's in-guest flag) before the step's load; each resumes after the step's store, its cancel
  read as a kick. Which steps are locked: the lifted instruction's lock prefix or a memory XCHG, read from the lifter
  (`translator::lift`). A new probe, `probes/locked_race/`, built and run on the Windows runner
  (`.github/workflows/windows-probe.yml`), its binary and output kept by hash: in one 64-byte-aligned line, thread A runs
  `lock xadd` of 1 to the dword at offset 1, 100,000 times, and thread B `lock xadd` of 1 to the dword at offset 4
  until A is done, counting its adds; byte 4 is A's high byte and B's low byte. Rows: A's low 24 bits, 100,000, and
  B's adds lost, its count less its dword, 0.
- **Not here:** the split (4d2); the race test of the guest-memory design's choice 6 (4e).

## prerequisites

- 330 (closed)

## context_files

- `docs/design-run-loop-on-several-vcpus-2026-10-31.md` "A model step of a locked form" (58-68), precondition 8;
  `crates/w2m_vm/src/model_step.rs` `model_step`; `crates/w2m_vm/src/run.rs` `RunStop`, `kick_and_wait`, the
  `hvc #11` service; `crates/translator/src/lower.rs` 1424-1430; BLACKBOARD Deferred (Sprint 272)

## signal contract

### Emits

- None; when a locked model step stops the other vCPUs.

### Invariants

- Every run on one vCPU as held; on one vCPU there is no other vCPU to stop, and the step is as today.

## artifact contract

### Files modified

- `crates/w2m_vm/src/run.rs`, `crates/w2m_vm/src/model_step.rs`, `crates/w2m_vm/tests/run_loop.rs`,
  `probes/locked_race/` (new), `store/binaries/` and `store/family_runs/` (the probe's binary and Windows' output);
  the design's card 4 note, with the work
- *Amended 2026-11-24 (reviewer-4's Low 3 at 331), to what the work changed:* `crates/w2m_vm/src/run.rs`,
  `crates/w2m_vm/src/bin/w2m-vm.rs`, `crates/translator/src/lower.rs`, `crates/w2m_vm/tests/run_loop.rs`,
  `store/batch_run/first-stop-2026-10-28.tsv`, `store/probe-runs/41326239822/`,
  `store/binaries/d19052c8d9eb….exe`, `probes/locked_race/` (34d6c9abc) and the design's 4d1 note; not
  `model_step.rs` or `store/family_runs/`. The wait for a vCPU already in guest code has no judge here or at 330's
  kick; 4d2's guest is to judge both.

### Command exit codes

- `scripts/gates.sh` returns 0

## observation contract

- **Expected values:** a test running `locked_race` through `w2m-vm --vcpus 2`: A's sum 100,000 and B's adds lost
  0, every row equal to the Windows runner's output, held to those values; the same on one vCPU; every run on one
  vCPU as held.
- **Planted faults:** (1) the stop taken out of the locked step: B's adds lost above 0; (2) the wait for each vCPU
  out of guest code taken out, the cancels sent and not waited on: B's adds lost above 0. Each judge by lookup
  (WORKING_AGREEMENT, Sprint 284): A's misaligned operand reaches `hvc #11` (`lower.rs` 1424-1432, the `ubfx` and
  `cbz` on the address's low bits), the run loop's step under the word alone (`run.rs` 1696-1716), and B's aligned
  operand the exclusive pair, which takes no word; the test asserts B's adds lost, the value both plants change. That
  the race shows in a run is not read from the code: plant 1 is run first, as today's code, and if no run of 20 loses
  an add the result says the judge does not reach it and the window is measured before the stop is built; plant 2
  likewise against the built stop.

## done criteria

An aligned locked add on one vCPU loses no update to a misaligned locked add stepped in the model on another, as on
Windows.

## self-review checklist

- [x] Scope names one verifiable property: a locked model step is atomic on two vCPUs.
- [x] context_files cite the design, the step, the stop and the lowering.
- [x] Emits: none.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values held to Windows' output and the one-vCPU run, two plants, each judge by lookup,
  each plant's run before the result claims it.
- [x] One concept: the stop of every other vCPU for a locked step.

## handoff to the builder (2026-11-24)

The work, its five plants and its measurements are done and recorded below; nothing is committed past 34d6c9abc.
What is left for the builder:

1. Run `scripts/gates.sh` on the working tree, in the background, and read its whole log. The gates judge the tree
   less the bookkeeping (`scripts/gated-tree.sh`), so they run before any commit. A failure is diagnosed, not rerun
   blindly; a held figure the change moves (as `hvc 15` moved at Sprint 304) is named with its evidence.
2. Add to `## result` a bullet "**Every run as held:**" with the tests the gates ran, and "**The gates:**" with the
   pass line copied from `store/gates/passes.tsv` (never typed).
3. In the final message: the tail entry for Sprint 331 in the form of Sprint 330's (below), its Rubber Duck Pass
   (sequence narration "no record"; observations from this result: plant 2 reaching no judge, the redundant gate,
   the count's held 0 and its judge, the slips), and a Deferred entry: the locked step's wait for a vCPU already in
   guest code has no judge, re-visit 4d2's guest, which is to judge both this wait and Sprint 330's kick.

Context for these steps (the supervisor's pointers; read these and the card's `context_files`):

- `WORKING_AGREEMENT.md` lines 137-266, "Build and verification commands" (the gates, plants, the pass line).
- The form of a tail entry: `sed -n '/^### Sprint 330 (/,/^### Sprint 329 (/p' BLACKBOARD.md` (read that range
  only).
- `scripts/gates.sh`, `scripts/gated-tree.sh`.

## result

- **The probe** (`probes/locked_race/`, committed 34d6c9abc before the work): built and run three times on the
  Windows runner, run 41326239822 (`windows-2025`, Server 2025 10.0.26100.33674, an AMD EPYC 9V74), its files kept in
  `store/probe-runs/41326239822/` and its binary in `store/binaries/d19052c8d9eb….exe`; the three runs alike:
  `{"s":"a","sum":100000,"expected":100000}` and `{"s":"b","lost":0,"counted_some":1}`. The property is x86's, so the
  runner's AMD part, not the reference Intel, judges it as well (reviewer-3 at 331).
- **Plant 1's reach, run before the stop was built:** `locked_race` through `w2m-vm` with no stop, 20 runs on one vCPU,
  every row Windows'; 20 on two, every one wrong in B's row, A's sum right. Eight read B's adds lost, 598 to 20,226;
  twelve read near 2^32, B's dword above its count: the step's stale byte 4, written back after B's adds had carried
  into byte 5, moved B's dword up by up to 255, and the probe prints the difference unsigned. The card's planted faults
  said "above 0"; the test holds the whole row to Windows' `"lost":0`, so either reads as a failure.
- **The stop** (`crates/w2m_vm/src/run.rs`): the `hvc #11` service, under the runtime word and the lock, lifts the
  instruction and, for a form x86 locks, calls `RunStop::stop_others` before the model's load: every other vCPU
  cancelled through its handle, then a wait until none is in guest code (Sprint 330's in-guest flag) or the run stops.
  A vCPU not running when cancelled returns from its next `hv_vcpu_run` "immediately without entering the guest"
  (`hv_vcpu.h` 377-378), and its way back in is its turn's start, which waits on the word the step holds, so no vCPU
  runs guest code until the step's store is done. `translator::lower::is_locked` is the one rule for a locked form,
  the lowering's inline rule moved there (`crates/translator/src/lower.rs`), the step and the lowering both reading it.
  The instruction is lifted twice, once for the stop and once in `model_step`; the batch run steps 286 model exits, so
  the second parse is left.
- **The count, from reviewer-3's Low 1 at 331** (the design's precondition 8: "the batch run counts such steps"):
  `Outcome::locked_steps`, summed over vCPUs, and the game's batch stop file's line "model steps of a locked form",
  held in `store/batch_run/first-stop-2026-10-28.tsv`: 0 on the batch run to its stop, of 286 `hvc #11` exits, the
  stop at ReferenceEnd as held. The game makes no misaligned locked access up to its first stop, so no step stops the
  other vCPUs there; the count is where a run further into play will show how often one does. A held 0 cannot fail
  on a count never raised (the Drift watchlist's comparison-judge entry), so `w2m-vm` also prints every run's count
  on stderr, "w2m-vm: N model steps of a locked form", and the `locked_race` test holds it to A's 100,000 adds on one
  vCPU and on two: three runs each way read 100000 before the test was written.
- **With the stop, the first build (the gate below still in):** 20 runs on one vCPU and 20 on two, every row
  Windows', in 19 s; **as committed, the gate gone:** the same, 20 and 20, every row Windows', in 19 s.
- **The test** `run_loop::a_locked_model_step_loses_no_add_of_another_vcpus_aligned_lock`: `locked_race` through
  `w2m-vm` on one vCPU and on two, `ExitProcess { code: 0 }`, every row equal to the Windows run's `body-1.jsonl`.
- **Plant 1,** `run.rs` copied aside and each plant written from the copy by a one-line `awk` substitution, restored by
  `cp`, checked by `cmp`: no form treated as locked (`.is_ok_and(|ins| false && …)`); the test failed, "locked_race's
  rows on 2 vCPUs as on Windows", left `{"s":"b","lost":35568,"counted_some":1}`.
- **Plant 2 reached no judge:** the wait taken out (`while false && self.in_guest(owner) && …`); the test passed, and
  20 runs on two vCPUs read every row Windows'. The cancel alone held the others out in every run; why the window
  between a cancel and the holder's exit never met the step's load is not measured. A replacement plant, the wait
  held until the run's stop (`while !self.stopped() {`), written and restored likewise: the test failed, "locked_race
  on 2 vCPUs", left "the run's time limit", right "ExitProcess { code: 0 }". The probe's run reaches the wait; what no
  test holds is that the step must wait for a vCPU already in guest code, the same gap as Sprint 330's kick (Deferred,
  Sprint 330; 4d2's guest).
- **Beyond the card, a gate found redundant:** the first build also held each vCPU at its entry to guest code while
  another's step was on (`enter_guest`, a `stopped_by` mark, a guard clearing it). Plant 3, the gate opened
  (`if true || by == 0 || …`), passed the test and 20 runs of 20 on two vCPUs. Read, not guessed: the header's
  lines 377-378 and the turn's start's take of the word already keep a cancelled vCPU out, so the gate, the mark and
  the guard are gone, `set_in_guest` as at 330.
- **Slips:** the rename of `set_in_guest` to `leave_guest`, later undone with the gate, first went in by a one-line
  `sed` on line 1110 without its text read first; the doc comment was on 1111, and a blank line was overwritten,
  found in the diff and put back. The plants were written by `awk`, one line for one line from the copy, not `sed`.
  A line of this result first said the stop "costs" the game nothing, against the rule that the word means money
  only; rewritten before the commit.
- **The files, against the card's list:** `crates/w2m_vm/src/run.rs`, `crates/w2m_vm/src/bin/w2m-vm.rs`,
  `crates/translator/src/lower.rs`, `crates/w2m_vm/tests/run_loop.rs`, `store/batch_run/first-stop-2026-10-28.tsv`,
  `store/probe-runs/41326239822/`, `store/binaries/d19052c8d9eb….exe`, `probes/locked_race/` (34d6c9abc) and the
  design's 4d1 note. The card named `model_step.rs`, which the work left alone, and `store/family_runs/`, where the
  probe's run is not kept; it did not name `lower.rs`, `w2m-vm.rs` or the held stop file.
- **Plant 5, the count's (reviewer-3's Low 1 at 331),** written and restored likewise: the increment made
  `out.locked_steps += 0;`; the test failed, "locked_race's locked steps on 1 vCPUs", stderr without "100000 model
  steps of a locked form". Format and lint clean after the restore.
- **Every run as held:** the gates' selection "Rust tests: all (an input outside one crate changed)", `probes/`
  and `store/` being outside one crate; the third run's nextest, "360 tests run: 360 passed (149 slow), 1 skipped",
  the skip `exit_time`, ignored there and run by the timing stage, "passed, 738 ns an exit at load 3.74". Among the
  360: `a_locked_model_step_loses_no_add_of_another_vcpus_aligned_lock`, the batch run held to the stop file with
  its new line, and every run on one vCPU. Two runs failed first. Run 1 stopped at
  `image_walk::fixtures every_fixture_image_walks_to_its_stored_counts`, "images walked against images stored",
  left 281, right 280: the probe's binary, new under `store/binaries/`, had no entry in
  `store/translation-cache/walk-fixtures-2026-10-30.json`, the first binary added there since Sprint 254 made the file.
  A held figure the change moves: the entry is the `image_walk` binary's output for the new image (302 instructions,
  all walked, 75 block starts, no table, none undecodable), put in by `jq` in the file's order (`.exe` keys sorted,
  then `.dll`), the order first checked to reproduce the stored file byte for byte, the diff that one entry; the
  test's doc comment's totals made 281 images and 641,343 instructions, summed by `jq`. The other readers that list
  `store/binaries/` (`llvm_objdump.rs`, `cache_file.rs`, `c_differential.rs`, `pe_loader`'s `images.rs`) hold
  properties of each file or a floor, no count of files; nextest's stop after the first failure left them to run 2.
  Run 2 passed all 360 and the timing stage (804 ns at load 4.28) and failed at `signal_report --check` on a Decision
  entry of BLACKBOARD written that day, which held no "Decision:" clause, a bookkeeping file outside the gated tree,
  mended by the Supervisor.
- **The gates:** `exit 0 in 4928 s	default	2026-11-24T17:42:57Z	sprint 331	34d6c9abc1c903da30708bec3cd69f9555549437	0a14cf3035bf986bfc58f88b406d013bea1fb9afe0c0fa3a941a1334ecf4e13f	root=f3409df38e02de59b63f2f0e8964c137d27a0b78	rust=all python=0 samples=0	base=52f94ad40df367569b3f66dff54e50c3352c2b04`
  The run's 4,928 s against Sprint 330's 1,418 s: 35 minutes of it the 60 test binaries listing before the first
  test, each launch behind `scripts/sign_and_run.sh`'s `codesign -d` check (26.27 s on one already-signed binary,
  measured during run 2 while the machine stalled; card 332 measures it quiet), and 2,421 s of tests.
- **The supervisor's grade (2026-11-24):** the gated tree, `scripts/gated-tree.sh`, prints `f3409df38…`, the pass
  row's `root=`; `crates/w2m_vm/src/run.rs` compares equal by `cmp` to the copy plants 1, 2, 2's replacement, 3 and 5
  were written from, those plants run by the supervisor on this code before the roles split, so they were not rerun
  (plant 3's gate is no longer in the code); the walk fixture's new entry rerun by the supervisor with
  `target/release/image_walk` on the binary, the same seven figures, 281 entries for 281 files under
  `store/binaries/`; the builder's two bullets read against `gates-331-3.log`. The card's file lists do not name
  `store/translation-cache/walk-fixtures-2026-10-30.json` or `crates/image_walk/tests/fixtures.rs`; the work changed
  both.
