# Sprint 351 — The 273 family runs on two vCPUs, held to Windows' rows

```yaml
---
id: 351
status: done
phase: 6
pass_kind: functional
---
```

## scope

Reviewer-4's twentieth review, High 1, recorded as the Decision "The family runs on two vCPUs before the lock cards".
G2 in `docs/step-6-census-2026-10-30.md` asks for the 68 Windows function families on two vCPUs, 273 runs, each held to
Windows' rows. Today `scripts/gates.sh`' `family_runs()` runs each run of `store/family_runs/known-2026-10-29.tsv`
through `w2m-vm` on one vCPU. `crates/w2m_vm/tests/family_runs.rs` then holds its printed rows to its contract's rows,
or to `store/family_runs/sweep-held-2026-10-29.tsv`. Neither takes a vCPU count.

The design's next four cards (`docs/design-handles-and-sync-objects-2026-11-26.md`) change the handle table, the waits,
the releases and CloseHandle. The threads, sync-object and handle families judge exactly those, so they must run on two
vCPUs before those cards do.

- **The change:**
  - `family_runs()` runs every run on one vCPU, as today, and on two, by `w2m-vm --vcpus 2`, writing each run's
    printed rows and stop beside the one-vCPU ones under a name that says two.
  - `family_runs.rs` holds each two-vCPU run to the same contract rows, or the same held entry, as its one-vCPU run.
  - A run that starts a child process stops on two vCPUs at the host's refusal, `MACHINE_ON_ONE`
    (`crates/w2m_vm/src/host_request.rs` 79, 531), until 4f. Hold those runs by name to that stop in a list the test
    reads, each with its family and run, and count them.
- **Every other two-vCPU failure is a finding.** Quote the run, its stop and the rows that differ. A run that stops,
  hangs past its budget or answers differently from Windows on two vCPUs is a defect. Under the user's Decision of
  2026-10-18 it is fixed in this card: report it to the Supervisor at once with its evidence, so the card can be
  amended for its fix. Do not hold a failing run to its failure.
- **The judge's reach.** Before the change, run the families on two vCPUs once by hand and quote the count that pass,
  the count refused for a child process, and each other failure.
- **Time.** The family runs take about 37 s on one vCPU. Quote the two-vCPU step's time from `store/gates/steps.tsv`.
- **Not here:**
  - the child-process runs on two vCPUs (4f);
  - the game's batch run on two vCPUs;
  - the lock cards.

## prerequisites

- 350 (closed)

## context_files

- `scripts/gates.sh` (`family_runs()`, lines 205-260) and `crates/w2m_vm/tests/family_runs.rs` whole.
- `store/family_runs/known-2026-10-29.tsv` and `store/family_runs/sweep-held-2026-10-29.tsv`.
- `crates/w2m_vm/src/bin/w2m-vm.rs` (`--vcpus`) and `crates/w2m_vm/src/host_request.rs` 70-90 and 520-540
  (`MACHINE_ON_ONE`).
- `docs/step-6-census-2026-10-30.md` (G2) and `docs/design-handles-and-sync-objects-2026-11-26.md` (the cards that
  follow).
- `WORKING_AGREEMENT.md` lines 137-266, "Build and verification commands".

## signal contract

### Emits

- None; the family runs' judge on two vCPUs.

### Invariants

- The one-vCPU family runs pass unchanged, every run held as before.

## artifact contract

### Files modified

- `scripts/gates.sh`, `crates/w2m_vm/tests/family_runs.rs`, and the list of runs refused for a child process (a new
  file under `store/family_runs/`, named with its date).

### Command exit codes

- `scripts/gates.sh` returns 0
- `target/release/signal_report --check` returns 0 on the tree

## observation contract

- **Expected values:**
  - The count of the 273 runs held to Windows' rows on two vCPUs, the count refused for a child process, by family.
  - Each other failure found and its fix, or the card's amendment.
  - The two-vCPU step's seconds.
- **Planted faults:**
  - (1) The two-vCPU runs skipped by `family_runs()`. The test fails, naming a missing two-vCPU output.
  - (2) One two-vCPU run's printed rows changed in its output file, on a scratch copy of `build/family_runs/`. The
    test fails, naming that run and its field.

  Before the change, read each judge's reach: the test failing with no two-vCPU outputs.

## done criteria

Every one of the 273 family runs runs on two vCPUs in the gates and is held to Windows' rows as on one vCPU, except the
runs that start a child process, which are held by name to the host's refusal until 4f.

## self-review checklist

- [x] Scope names one verifiable property: the family runs on two vCPUs held to Windows' rows.
- [x] context_files cite the gates' step, the test, its inputs, the vCPU flag and the refusal.
- [x] Emits: none.
- [x] Artifact contract gradable by command.
- [x] Observation contract: the counts, the findings, the time; two numbered plants, their reach read.
- [x] Sweet spot: `gates.sh` and `family_runs.rs`; one concept, G2's runnable part.

## amendment (2026-11-26)

Builder-351's run of the 273 on two vCPUs, before the change:
- 266 runs printed the same rows and stop as on one vCPU;
- 4 stopped at `MACHINE_ON_ONE`;
- 2 were refused by the instruction clock;
- 1 answered differently from Windows and hung.

- **A defect fixed in this card (the user's Decision of 2026-10-18).** `running_thread_context`
  (lap-20261013T234642777Z-5cb60b6d, `windows.thread.running_context@0.1.0`) conforms on one vCPU. On two, GetThreadContext
  of a thread suspended while it spins on the other vCPU answers none of the planted registers: `gpr_planted` 0
  (Windows 65519), `rip_in_loop` 0 (1), `xmm_planted` 0 (65535), `mxcsr` 8064 (40896). In 3 of 6 runs the run stopped at
  its time limit. SuspendThread of a thread on another vCPU is Sprint 330's kick.
  - Find by experiment, with no cause claimed beyond what is read, where the suspended thread's registers are taken
    from on two vCPUs, and when.
  - Fix it so GetThreadContext answers the suspended thread's own registers, and the run never hangs.
  - Run it 20 times on two vCPUs with every row Windows' and no time-limit stop, and quote the count.
  - Plant 3: the fix taken out; the family test fails on the run's fields or its stop.
- **The instruction clock's refusal.** `processes` 38189162864 and 38189164581 start with `{"instruction_clock":true}`,
  which `run::vcpus_allowed` refuses on two vCPUs (Sprint 315), before any VM. The instruction clock on two vCPUs is
  its own card in the order. Hold these two by name to that refusal, as the child-process runs are held to
  `MACHINE_ON_ONE`, each with its refusal written as the run's stop. Write the stop from the gates' capture of
  `w2m-vm`'s refusal line, or from `w2m-vm` itself, whichever changes less; record which.

## result

- **The judge's reach, before the change:** the 273 runs of `known-2026-10-29.tsv` run once by hand through
  `target/release/w2m-vm --vcpus 2`, otherwise as `family_runs()` ran them, 33 s. 266 printed the same rows and stop
  byte for byte as their one-vCPU outputs, each of which conforms. 4 stopped at `MACHINE_ON_ONE`
  ("HostRequest op 33: a process that starts another runs on one vCPU until the Machine has its own thread (the
  run-loop design's card 4f)"): process_parameters 38194049597 and 38194051809, time_and_error_mode 38206225386 and
  38206227300. Two failures were reported to the Supervisor and amended into the card: running_thread_context
  answered none of the planted registers and hung, and processes 38189162864 and 38189164581 were refused before any
  VM by the instruction clock ("w2m-vm: 2 vCPUs refused: the instruction clock reads the running vCPU's own count"),
  writing no stop. Three more sets by hand (93, 93 and 95 s) gave the same: 270 runs alike, the two refusals, and
  running_thread_context at "the run's time limit" each time. `family_runs.rs` read no two-vCPU output before the
  change, so it passed with none.
- **The change:** `scripts/gates.sh`' `family_runs VCPUS SUFFIX` runs every run on one vCPU as before, then again with
  `--vcpus 2`, writing `build/family_runs/<family>-<run>-two-vcpus.txt`, `.stop` and `.stderr`. The one-vCPU step alone
  clears the directory. The two steps are timed apart, `family-runs` and `family-runs-two-vcpus`.
  `crates/w2m_vm/tests/family_runs.rs` judges each run's two outputs by one `state_of`. A two-vCPU run must have the
  one-vCPU run's state, conforming or its held entry. A run in `store/family_runs/two-vcpus-refused-2026-11-26.tsv`
  must stop at the refusal listed there, and a refusal not listed fails the run as a changed state.
  `the_refused_runs_are_held_to_the_hosts_refusals` holds each listed stop to one of the host's two refusals, built
  from `MACHINE_ON_ONE` and `HOST_KERNEL` and from `run::vcpus_allowed`.
- **The instruction clock's refusal:** `w2m-vm` writes `vcpus_allowed`'s refusal of a start's vCPUs to the `--stop`
  file before it returns the error. `main` then stamps that file like any output. A stop the gates wrote from stderr
  would carry no stamp, and `built_from::fresh` refuses an unstamped stop.
- **The defect, found by experiment** (temporary `eprintln!`s in `kick_and_wait`, the cancelled exit, the turn's
  start, `threads::save` and `thread_context`, then removed). In five runs SuspendThread's kick wait ended with the run
  entry still naming vCPU owner 2 and that vCPU out of guest code (`in_guest false`). GetThreadContext then read the
  entry as the thread was created: RAX 0, x17 0x1000120000, MXCSR 0x1F80. No put-down of the suspended thread followed
  in any run. At the kick's turn start, vCPU index 1 read the current thread suspended, `slice_until` 0, and its slice
  limit 20,000,000,000, the run's whole budget. In the runs that hung, the wait went on with the vCPU in guest code
  until the run stopped at its time limit. Every vCPU's limit is set to the run's budget at the start (`run.rs` 983-993).
  `set_budget` shortens only the served vCPU's limit, at a thread's creation, so vCPU index 1's limit was never a slice.
- **The fix:** `take_a_thread` calls `threads::set_budget` for the vCPU that took a thread. `threads::thread_context`
  answers 0 while another vCPU has the thread loaded, and `k32_GetThreadContext` retries, as its comment says it
  does. After it, the kick's turn start read limit 20000, the host put the thread down (RAX 0x1010101010101010, x17
  0x140001177, MXCSR 0x9FC0), and GetThreadContext read those values. running_thread_context ran 20 times on two vCPUs:
  20 of 20 ended `ExitProcess { code: 0 }`, each with its rows byte for byte the one-vCPU run's, which conform.
- **The counts after**, from the test's output: "273 of 273 family runs conform, 68 of 68 families in every run"; "on
  two vCPUs: 267 of 273 held as on one vCPU, 267 of them conforming; 6 refused by name (process_parameters 2, a child
  process; processes 2, the instruction clock; time_and_error_mode 2, a child process)".
- **Plant 1** "no build/family_runs/code_pages-38044061149-two-vcpus.stop (scripts/gates.sh writes it), and
  W2M_REQUIRE_CASE_RUNS=1 requires it": `family_runs` given `[ "$vcpus" = 1 ] || return 0` after `suffix=$2` (one line
  for one, `sed`), its two steps run from that `gates.sh`, then the test, exit 100. `gates.sh` restored by `cp`, `cmp`
  0, and the steps run again.
- **Plant 2** "sync_objects run 38051398628 on two vCPUs: ended; row \"semaphore.wait_available\": ret reference 0,
  target 258 (windows.sync.objects@0.1.0); on one vCPU: conforms": `"ret":0,` made `"ret":258,` in the second row of
  `build/family_runs/sync_objects-38051398628-two-vcpus.txt`, the directory first copied to the scratchpad. Exit 100.
  Restored by `cp`, then `cmp` 0 and `diff -r` against the copy empty.
- **Plant 3** "running_thread_context run lap-20261013T234642777Z-5cb60b6d on two vCPUs: stop: the run's time limit; on
  one vCPU: conforms": `run.rs` and `threads.rs` copied back to their state before the fix, built, the family steps
  run (two vCPUs 94 s), then the test, exit 100. The fixed files restored by `cp`, `cmp` 0, built, and the steps and
  the test run again: pass.
- **The gates:**
  - One run, passed: `exit 0 in 408 s	all	2026-11-26T11:29:18Z	sprint 351	71889e1d70b014cf2320127767b0da62559b5f2d	e8edee28abf8356b3ad1812a0b1d30dd97a34cadf50fc1a3d6b27695a9510141	root=51fc9bf8f125648216b9530c46d7ce8d38f9a238	rust=all python=0 samples=1	base=b34e598ea332adf2f41a385ccc432b8438585938`. Its steps `family-runs` 27 s and `family-runs-two-vcpus` 34 s (`store/gates/steps.tsv`, 2026-11-26T11:22:30Z); the family runs took about 37 s on one vCPU before.
