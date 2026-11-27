# Postmortem: Windows reference runs made on GitHub's runner, not the laptop (2026-11-27)

## What happened

The project decided that the Windows gaming laptop is its Windows reference, and that every new probe run and game run
goes to it. From 2026-10-30 to 2026-11-27 none did. Ten probe runs went to GitHub's `windows-2025` runner through
`.github/workflows/windows-probe.yml`, and two game runs went through `windows-game.yml`. Every Windows reference the
project made in those four weeks was made on the wrong machine. The user stopped the project on 2026-11-27: "there's
no reason for the behavior probes for each sprint to run on the github's windows runner ... we have a windows machine
here, this is a huge failure."

## The two machines

| | The laptop (the reference) | GitHub's runner |
|---|---|---|
| Profile | `laptop-x64-10.0.26200.7840-Client` | `gha-windows-2025-x64-10.0.26100.*-Server` |
| Windows | Windows 11, client, build 26200 | Windows Server 2025, build 26100 |
| CPU | 13th Gen Intel Core i5-13420H | AMD EPYC (9V74, 9V45) |
| GPU, audio, Steam | yes | none |
| Game run limit | none | 180 s in batch mode |

Sources: `store/probe-runs/lap-20261017T040446837Z-dc268176/profile.txt` and `cpu.txt`;
`store/probe-runs/46053401854/profile.txt` and `cpu.txt`; `sprints/sprint-331-*.md` line 126.

## Timeline

- **2026-10-11.** The user bought the laptop (user decision, BLACKBOARD).
- **2026-10-12.** `docs/windows-reference-machine-2026-10-12.md` designed the path: `scripts/windows_machine.py` on the
  Mac sends a job over ssh and scp, and `tools/windows_machine/run-job.ps1` on the laptop runs it from inputs named by
  hash. The same day the main session wrote a memory file saying the laptop "has not yet arrived".
- **2026-10-13.** User decision: "The laptop is set up as the Windows reference before development goes on." The laptop
  was set up and ran 198 probe runs that day (`store/probe-runs/lap-20261013*`). The memory file was not updated.
- **2026-10-13 to 2026-10-17.** 201 laptop runs in all, the last `lap-20261017T040446837Z-dc268176` (Sprint 095); the
  play-run traces in `store/laptop-traces/`.
- **2026-10-16.** Agent-resolved Decision (Sprint 067, step G): "the laptop, by default, for every probe and game run
  (the runners stay for CI and for rows already on record) ... Revisit if: the laptop is unavailable for a run a sprint
  needs." Never superseded.
- **2026-10-21.** User decision: no new Python.
- **2026-10-29, Sprint 226.** Every tracked `.py` left the tree (commit 72e0bab6a), among them the laptop's whole Mac
  side: `scripts/windows_machine.py` (349 lines, the dispatcher), `scripts/laptop_dtrace.py`,
  `scripts/laptop_calibration.py`, `tools/windows_machine/pmc_images.py` and `pmc_summary.py`. No Rust replacement was
  built. Its Deferred entry names the replacement as "a dispatcher through `gh workflow run`, and a fetcher through
  `gh run download`": the GitHub runner, against the Decision of 2026-10-16, with no Decision recording the change.
- **2026-10-29.** Reviewer-3's project review, recorded on BLACKBOARD: "The Windows laptop is expected, not yet here."
  False since 2026-10-13. Not caught.
- **2026-10-30, Sprint 227.** "The Windows workflows without Python" made `windows-probe.yml` and `windows-game.yml`
  run without Python. Its card does not mention the laptop. Runner probe run 41018967302 and game runs 41018970831 and
  41019280283 followed.
- **2026-11-01, Sprint 331.** Probe `locked_race` on the runner (run 41326239822). The card's own text names a
  different reference machine and moves past it: "The property is x86's, so the runner's AMD part, not the reference
  Intel, judges it as well."
- **2026-11-26 to 2026-11-27, Sprints 344 to 369.** Eight more runner probe runs: 45760720489 (344), 45763850660 (345),
  45780352907 and 45783238565 (347), 45797800625 (350), 45813040124 (352), 45871497648 (356), 46053401854 (369). The
  cards the supervisor composed say "runs the Windows workflow" (369's and 370's scope).
- **2026-11-27.** The user opened the laptop's lid and said it is ready to connect to. The supervisor's answer named
  the runner as the probes' proper place ("The behaviour probes ... run on GitHub's Windows runner"). The user stopped
  the project.

## What the wrong machine reached

- **Nine probe families' rows** are Windows Server 2025 on AMD, not the reference, each kept in
  `store/probe-runs/<run>/`: `locked_race` (41326239822), `thread_values_race` (45760720489), `thread_slots_race`
  (45763850660), `tls_free_reach` (45780352907), `tls_expansion_first` (45783238565), `wait_closed_handle`
  (45797800625), `wait_closed_kinds` (45813040124), `mutex_abandoned` (45871497648) and `named_sync_objects`
  (46053401854). Their contracts and family runs are held to those rows. A tenth run, `semantics_065c` (41018967302,
  Sprint 227), checked the case-set download and kept no files.
- **The game's call comparison is mostly the laptop's, and its contract's evidence is the runner's.** G3's
  `call_compare` holds w2m to a reference set of eleven Windows runs: runner run 38216081440 and ten laptop runs,
  `lap-20261014T032442034Z-c8814547-01` to `-10` (the frozen host's record
  `store/runs/frontier-643a6283510c-1792623423346201000`, its `reference_set`; each G3 record's `reference_ref`). The
  set is the frozen host's last choice: its membership rules, which read laptop and runner runs, were not ported
  (`crates/call_compare/src/lib.rs` 1251-1253), so no laptop game run since 2026-10-14 can join it. The contract
  `contracts/windows.game.calls/0.2.0.json` cites runner runs only as its entries' evidence (38176899195, 38203364352,
  38206263976, 38216081440, from 2026-10-11 and 2026-10-12).
- **What the store already holds from the laptop:** 201 probe runs (`store/probe-runs/lap-*`), 34 game runs
  (`store/game-runs/lap-*`) and the play-run traces (`store/laptop-traces/`).
- **What was lost with the Mac side:** per-thread instruction counts from the CPU's counters (`pmc_*`), the dtrace and
  calibration tools, and the only path that makes a new laptop reference. No laptop run has been made since 2026-10-17.
- **What the runner cannot give at all:** a game run past 180 s, any GPU or audio behavior, the Steam client (step 10's
  reference, `docs/rust-port-plan-2026-10-24.md` line 216).

No measured w2m behavior is known to be wrong because of this: no runner row has yet been compared with a laptop row.
That comparison is what settles it.

## Why it happened

1. **A working path was deleted and nothing replaced it.** Sprint 226 removed the laptop's dispatcher to meet the
   no-Python rule. Removing a capability the project depends on needed the replacement built first, or the
   capability's loss recorded as a Decision. Neither happened.
2. **The replacement was named against a standing Decision, silently.** The Deferred entry chose `gh workflow run`.
   AGENTS.md hard rule 4: "Never decide silently." The Decision of 2026-10-16 had a revisit clause ("if the laptop is
   unavailable") that nobody invoked; the laptop was available.
3. **A false fact was loaded into every session.** The main session's memory said the laptop had not arrived, from
   2026-10-12 until 2026-11-27, against the BLACKBOARD user decision of 2026-10-13 and 201 stored laptop runs. Each
   session read it, and none checked it against the record. The memory index also carried other event narratives,
   not standing rules.
4. **The reviews repeated the false fact or missed it.** Reviewer-3 wrote it into BLACKBOARD on 2026-10-29. Forty-four
   reviews by reviewer-4 tailed BLACKBOARD and read cards that said "the Windows workflow" and never raised it.
5. **A visible mismatch was argued away.** Sprint 331 named "the reference Intel" and used the AMD runner, on the
   ground that the property was x86's.
6. **No check holds where a reference was made.** No gate, test or card field asks which machine a probe's rows came
   from. `family_runs.rs` checks rows against rows, never the profile against the reference's.
7. **The supervisor composed the wrong path into cards.** Cards 344 to 370 name the runner workflow as the Windows run,
   and the supervisor dispatched those runs itself.

## What stops, and what comes next

The project stops. No card proceeds and no Windows run is made until the following is done, in order:

1. **This postmortem.**
2. **An investigation of how to send work to the laptop, done properly:** the laptop's current state over its existing
   ssh path; `tools/windows_machine/` and `docs/windows-reference-machine-2026-10-12.md` read in full; the deleted
   dispatcher read at the tag `frozen-python-2026-10-29`; and the methodology settled on the three grounds (project,
   kit, named practice), as Rust with no Python.
3. **Then a card**, built from that investigation.
4. **Then the work.**
5. **Then the project continues.** That includes re-running the nine runner-made probe families and the game's batch
   run on the laptop, and comparing them with the runner's rows. Any difference is a defect in the reference, judged
   before any card relies on it.

Three items join that order (reviewer-4's forty-fifth review, each checked against the record):

- **A check in code for where a reference was made** (cause 6), in the card that builds the laptop's path or the one
  after it: every probe run in `store/probe-runs/` carries its `profile.txt`; a test holds every family's and
  contract's evidence runs to the reference profile, `laptop-x64-10.0.26200.*-Client`; a run off that profile passes
  only when a list names it by run id with its Decision, and the runner rows made before 2026-10-30 go on that list;
  the plant is a family held to a runner run not on the list.
- **Which Windows w2m answers as, settled in the investigation.** The agent-resolved Decision of 2026-10-11 (BLACKBOARD
  line 145) reads "w2m's Windows profile is build 26100 (Server 2025; the Windows 11 24H2 client shares it)"; the
  reference is Windows 11 client build 26200. The game contract already holds one difference between them as
  undetermined: FlsGetValue's error, 87 on Server (run 38216081440) and 0 on the laptop (runs
  `lap-20261014T032442034Z-c8814547-01` to `-10`). The re-runs will show more. A Decision on which build w2m answers
  as, on the grounds that the reference is the laptop and the game's players run client Windows, sorts each
  difference into a w2m defect or a profile choice.
- **The reference set's membership rules ported**, read at the tag `frozen-python-2026-10-29`, in the card that first
  makes a laptop game run, so new laptop game runs can join G3's comparison (`crates/call_compare/src/lib.rs`
  1251-1253: "the membership rules ... are not ported; the set is the frozen host's last choice").

Sprint 370 is paused. Its builder is stopped, its uncommitted work stays in the tree, and its probe (8bd3e3269) has
not been run anywhere.
