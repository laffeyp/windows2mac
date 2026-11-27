# Sprint 374 — The nine runner-made families held to the laptop's rows

```yaml
---
id: 374
status: active
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

GATES-LINE
