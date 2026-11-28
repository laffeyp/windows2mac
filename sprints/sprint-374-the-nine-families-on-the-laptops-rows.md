# Sprint 374 — The nine runner-made families held to the laptop's rows

```yaml
---
id: 374
status: done
phase: 6
pass_kind: functional
---
```

## scope

From 2026-11-01 to 2026-11-27 nine probe families' Windows rows were made on GitHub's runner, not the laptop
(`docs/postmortem-windows-runs-off-the-laptop-2026-11-27.md`). Sprint 372 ran their binaries on the laptop
(`store/probe-runs/lap-20261127T221622950Z-62b931c2-*`, and the rebuild `lap-20261127T225150596Z-62b931c2`). This card
compares the two machines' rows and moves each family's reference to the laptop's run (the user's decision of
2026-11-27: every probe run on the laptop; the Decision "The order after the stop, the cards that take each deferred
item").

- **The comparison**, held by a test: for each of the nine runner runs (41326239822, 45760720489, 45763850660,
  45780352907, 45783238565, 45797800625, 45813040124, 45871497648, 46053401854), its rows against the laptop run of the
  same family, by `cmp` of each body, with one of four outcomes the test names: equal on the same binary; equal on a
  laptop rebuild of one source (`wait_closed_handle`, 21c64b30 against f0ae78e5); equal but for a row the user's rule
  of 2026-10-13 calls scheduling (`thread_values_race`'s `overlap`, which `run_loop.rs`'s judge already leaves out);
  no laptop rows (`thread_slots_race`, both its binaries refused by Smart App Control, waiting on the user's answer).
  Any other difference fails the test and is a finding about the reference, judged before any card relies on it.
- **The references moved.** Each family whose laptop rows are on the record is held to them: `crates/w2m_vm/tests/run_loop.rs`'s
  judges read the laptop run in place of the runner run; `named_sync_objects`' row in `store/family_runs/joined-2026-11-27.tsv`
  names its laptop run, and `contracts/windows.sync.named_objects` takes a new minor version citing it (0.1.0 kept
  unchanged, as the Decision of 2026-10-07 requires). `thread_slots_race` keeps its runner run until the laptop can
  run it, its list row in `store/reference_machine/` saying so.
- **Every cited run held to the reference rule** (the Decision "A contract's evidence runs held to the reference
  machine too"): a test holding every run id a family table or a contract under `contracts/` cites to Sprint 373's
  rule, a cited run that is not stored passing only at or below the cut, 38311779649.
- **The Windows w2m answers as**, settled as a Decision at this card's close on the grounds (reviewer-4's forty-fifth
  and forty-sixth reviews): the agent-resolved Decision of 2026-10-11 has w2m answer as build 26100, Server 2025; the
  reference is Windows 11 client build 26200, now at UBR 9974, its October runs at 7840. The Decision says which build
  w2m answers as, whether the laptop's build is held fixed or each comparison names both builds, and which of w2m's
  build-specific tables (`specs/system_messages.json`, from 26100) a later card re-takes from the laptop.
- **Not here:** the reference set's membership rules and a game run on the record (card 375); turning Smart App
  Control off (the user's); re-taking build-specific tables.

## prerequisites

- 373 (closed)

## context_files

- `docs/postmortem-windows-runs-off-the-laptop-2026-11-27.md`; card 372's result
  (`sprints/sprint-372-batch-and-game-jobs-on-the-laptop.md`).
- `store/probe-runs/<the nine runner runs>/` and the laptop runs above; `store/reference_machine/runner-runs-after-the-laptop-2026-11-27.tsv`.
- `crates/w2m_vm/tests/run_loop.rs` (the judges that read the runner runs), `crates/windows_machine/tests/reference_machine.rs`.
- `store/family_runs/joined-2026-11-27.tsv`, `contracts/windows.sync.named_objects/0.1.0.json`, `contracts/` (the evidence fields).
- `grep -n "2026-10-11 agent-resolved\*\* — w2m's Windows profile\|2026-10-07\|The order after the stop" BLACKBOARD.md docs/decisions-archive-2026-10-30.md`.
- `WORKING_AGREEMENT.md` lines 137-266.

## signal contract

### Emits

- None: tests, the judges' inputs and a contract version; no emit site changes.

### Invariants

- Every w2m run as held: the family runs on one vCPU and two, G3's batch runs, the guests. No stored run is rewritten;
  `contracts/windows.sync.named_objects/0.1.0.json` unchanged.

## artifact contract

### Files created

- The comparison test and the cited-runs test; `contracts/windows.sync.named_objects/0.2.0.json`.

### Files modified

- `crates/w2m_vm/tests/run_loop.rs` (the laptop runs), `store/family_runs/joined-2026-11-27.tsv`,
  `store/reference_machine/runner-runs-after-the-laptop-2026-11-27.tsv` (thread_slots_race's reason), with others only
  where the class requires, reported to main when found. One concept: the nine families' references on the laptop.

### Command exit codes

- `scripts/gates.sh` returns 0
- `target/release/signal_report --check` returns 0 on the tree

## observation contract

- **Expected values:**
  - The comparison's outcome per family, quoted, with each pair's run ids and binaries.
  - Every `run_loop.rs` judge that moved, passing against the laptop run on one vCPU and two.
  - The cited-runs test's count by kind.
- **Planted faults:**
  - (1) A laptop body with one value changed in a row other than `overlap`: the comparison test fails, naming the
    family and the row.
  - (2) A contract citing a runner run after the cut that is not stored: the cited-runs test fails, naming the
    contract and the run.

## done criteria

The nine families' laptop rows are compared with their runner rows by a test, each family that has laptop rows is held
to them, every cited run meets the reference rule, and the Windows w2m answers as is a recorded Decision.

## self-review checklist

- [x] Scope names one verifiable property: the nine families held to the laptop's rows.
- [x] context_files cite the postmortem, the runs, the judges and the contracts.
- [x] Emits: none, with its reason.
- [x] Artifact contract gradable by command.
- [x] Observation contract: the comparison per family; numbered plants.
- [x] One concept: the nine families' references on the laptop.

## result

- **The comparison.** `crates/windows_machine/tests/runner_families_on_the_laptop.rs`,
  `each_runner_familys_rows_stand_to_its_laptop_rows_as_named`: the nine families are the list's nine runner runs, and
  each runner body (three a run) is compared by bytes with each laptop body (two a run), a difference named by family
  and row; the laptop run's family is read from its `job.json` or from its batch's item. Its output, quoted:
  - "locked_race: equal on the same binary d19052c8...; runner 41326239822 (3 bodies,
    gha-windows-2025-x64-10.0.26100.33674-Server) against laptop lap-20261127T221622950Z-62b931c2-001 (2 bodies,
    laptop-x64-10.0.26200.9974-Client)"
  - "thread_values_race: equal but for the scheduling row overlap, binary c9b9847f...; runner 45760720489 against
    laptop lap-20261127T221622950Z-62b931c2-002" (`overlap` 52,525 to 54,315 on the runner, 75,983 to 88,364 on the
    laptop; the test also holds that `run_loop.rs`'s judge leaves that row out)
  - "thread_slots_race: no laptop rows; runner run 45763850660 (binary 1c778788...) stays its reference" (the test
    holds that no stored laptop run is of the family)
  - "tls_free_reach: equal on the same binary 0dd9cc91...; runner 45780352907 against laptop
    lap-20261127T221622950Z-62b931c2-004"
  - "tls_expansion_first: equal on the same binary e7bc9985...; runner 45783238565 against laptop
    lap-20261127T221622950Z-62b931c2-005"
  - "wait_closed_handle: equal on a laptop rebuild of one source, 21c64b30 against f0ae78e5; runner 45797800625
    against laptop lap-20261127T225150596Z-62b931c2" (both binaries stored; `probes/wait_closed_handle` has one commit,
    1decadd51, up to the laptop's build at 62b931c25, read by `git log`)
  - "wait_closed_kinds: equal on the same binary af9e903d...; runner 45813040124 against laptop
    lap-20261127T221622950Z-62b931c2-007"
  - "mutex_abandoned: equal on the same binary 2e7af44b...; runner 45871497648 against laptop
    lap-20261127T221622950Z-62b931c2-008"
  - "named_sync_objects: equal on the same binary 446f77bc...; runner 46053401854 against laptop
    lap-20261127T221622950Z-62b931c2-009"
  - No other difference: Windows Server 2025 build 26100 (UBR 33674 and 33828; AMD EPYC 7763, 9V45 and 9V74, Intel Xeon
    Platinum 8370C and 8573C) and Windows 11 client build 26200.9974 on the i5-13420H gave the same rows in all eight
    families with laptop rows, scheduling aside.
- **The references moved.** `crates/w2m_vm/tests/run_loop.rs`'s seven judges of these families read the laptop run in
  place of the runner run, each body after body-1 held to it, each doc naming both runs: locked_race, thread_values_race,
  tls_free_reach, tls_expansion_first, wait_closed_handle (w2m now runs the laptop's binary 21c64b30), wait_closed_kinds,
  mutex_abandoned. Each passed on one vCPU and two (`W2M_REQUIRE_CASE_RUNS=1 cargo nextest run -p w2m_vm --test all`,
  7 passed, 1.30 to 5.31 s). thread_slots_race's judge keeps 45763850660, its doc saying why. `named_sync_objects`'
  row in `store/family_runs/joined-2026-11-27.tsv` names `lap-20261127T221622950Z-62b931c2-009` and
  `windows.sync.named_objects@0.2.0`, and `store/family_runs/known-2026-10-29.tsv`'s last row is `family_known`'s output
  of it (`the_stored_lookup_is_family_knowns_output` passed). `contracts/windows.sync.named_objects/0.2.0.json`: 0.1.0's
  rows, the laptop run as evidence (`laptop-rows`, its two executions), its applicability and start the laptop's, a
  `history`; 0.1.0 unchanged, held to 53449a78a by `the_named_objects_contract_0_1_0_is_unchanged`, and 0.2.0's rows to
  the laptop run's body by `named_sync_objects_is_held_to_its_laptop_run`. The gates' family-run step, run from
  `scripts/gates.sh` 242-261, gave `named_sync_objects-lap-20261127T221622950Z-62b931c2-009` ExitProcess 0 on one vCPU
  and two, each output equal by `cmp` to the laptop's body-1, and `every_family_run_conforms_or_is_held_by_what_it_waits_on`
  "274 of 274 family runs conform, 69 of 69 families in every run"; "on two vCPUs: 274 of 274 held as on one vCPU".
  `store/reference_machine/runner-runs-after-the-laptop-2026-11-27.tsv`: each reason names its family's laptop run and
  outcome, thread_slots_race's that Smart App Control refused both its binaries (Sprint 372) and that it keeps this run
  until the laptop runs the probe.
- **The class beyond the card's list** (reported to main): `crates/w2m_vm/src/bin/run_measurements.rs`'s
  `LOCKED_RACE_RUN`, whose locked_race judge held its two-vCPU run to 41326239822's rows, now the laptop run (the binary
  not run in this sprint: its record is made by hand, `harness/w2m-vm-measurements.toml`); `crates/w2m_vm/tests/measurements_record.rs`,
  which read thread A's sum from 41326239822's body; `crates/windows_machine/tests/windows_machine.rs`, two assertions
  that named_sync_objects' table row is 46053401854. `tools/guest_runtime/sync_objects.c` 686 and `thread_storage.c` 148
  cite runner runs as the evidence of a fix in comments and are left as they are.
- **Every cited run held to the reference rule.** `every_cited_run_is_held_to_the_reference_rule` in
  `crates/windows_machine/tests/reference_machine.rs`: each family table's run column and each contract's run ids
  (eleven digits standing alone, and laptop ids with a range "-01 to -10" read as each number,
  `the_citation_reader_reads_the_contracts_forms`) held to Sprint 373's `machine`, a run not stored passing only at or
  below 38311779649. "715 citations, 275 by the family tables and 440 by 105 contract files: {"Laptop": 51,
  "ListedRunner": 2, "RunnerBeforeTheLaptop": 648, "RunnerBeforeTheLaptopNotStored": 14}; failing 0". The two listed
  are 46053401854 in contract 0.1.0's evidence and 0.2.0's history; the 14 are nine run ids at or below the cut, in six
  contract files (`windows.event.auto_reset.single_signal` 0.2.0 to 0.3.1, `windows.environment.temp_path` 0.1.0,
  `windows.memory.thread_stack` 0.1.0).
- **The Windows w2m answers as**: drafted for the Decision recorded at this card's close (the report to main).
- **Plant 1** "thread_values_race: runner 45760720489/body-1.jsonl against laptop
  lap-20261127T221622950Z-62b931c2-002/body-2.jsonl: rows unlike ["b"]", `cargo nextest run -p windows_machine --test
  runner_families_on_the_laptop -E 'test(each_runner_familys_rows)'` exit 100; live 23:52:27-23:52:37 UTC:
  - backup `scratchpad/plants/374/thread_values_race-body-2.jsonl.backup`;
  - row "b"'s `tls_wrong` changed from 0 to 1 by Edit in the stored `store/probe-runs/lap-20261127T221622950Z-62b931c2-002/body-2.jsonl`;
    the grep found it at line 4;
  - the test failed naming the family and the row against each of the runner's three bodies (log
    `scratchpad/plants/374/plant1.log`);
  - restored by `cp`; `cmp` equal.
- **Plant 2** "contracts/windows.sync.named_objects/0.2.0.json cites run 46999999999: a GitHub run after 38311779649,
  not stored", `cargo nextest run -p windows_machine --test reference_machine -E 'test(every_cited_run)'` exit 100; live
  23:52:48-23:53:01 UTC:
  - backup `scratchpad/plants/374/named_objects-0.2.0.json.backup`;
  - an open question "A plant: runner run 46999999999." added by Edit to 0.2.0; the grep found it at line 181;
  - the test failed naming the contract and the run (log `scratchpad/plants/374/plant2b.log`). A first run, 23:52:42-23:52:47
    UTC, named the contract twice in its line (`plant2.log`); the message was made single and the plant run again;
  - restored by `cp`; `cmp` equal.
- **G3's two-vCPU batch run in these gates:** 129.37 s to the stop (`run_seconds` 129.374675881,
  `build/batch_run/record-2-vcpus-1795823822000000000`, `load_at_start` "load averages 2.50 2.15 2.24; busy: mds_stores
  81%; other w2m: none").
- **Gates:** `exit 0 in 608 s	default	2026-11-28T00:05:17Z	sprint 374	6002354e2fc09c2c65cd82f15f80aed788c8e7f0	2a3dbc24f15ea93ff7846da87bda316fb3acb985ac840dbaa2a8a50406333ed1	root=c58d7ed5fbdef4c5f2264e52db611f2f3be4d165	rust=all python=0 samples=0	base=91ec12caa3224c4e76b924ca686d6dfcfbf253ec` (copied from `tail -1 store/gates/passes.tsv`).
