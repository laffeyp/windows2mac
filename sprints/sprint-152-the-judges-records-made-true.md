# Sprint 152 — The judges' records made true: no false trace, a ledger, one profile a session

```yaml
---
id: 152
status: active
phase: 6
pass_kind: functional
---
```

## scope

reviewer-2's review of Sprints 149-150 (2026-10-25) found the record form Sprint 150 built false in one event and cut
off from the ledger; Sprint 151 withheld its record for that reason. This sprint fixes the form and makes both judges'
records again.

- **High 1, a validation that never happened.** TRACE_INGESTED says every line of a guest trace was validated against
  the vocabulary; the "traces" were verdict lines nothing validated, written to meet rule T4. The judge the case runner
  carries, `scripts/vm_oracle.py --record` through `differential.run`, wrote no EXECUTION_* or TRACE_* event at all: its
  sequence is SESSION_INIT, CANDIDATE_*, COMPARISON_COMPLETED, DISCREPANCY_* and SESSION_COMPLETED. `case_runs` does the
  same: EXECUTION_STARTED, EXECUTION_COMPLETED and TRACE_INGESTED go, and with them `store/case_runner/traces` (removed
  with `git rm`; the records that named them are replaced, never edited). Each run's counts move to the session's
  baseline, where vm_oracle put its own (`baseline={"run", "judged", "faults"}`).
- **High 2, the ledger.** `differential.rebuild_ledger` (105-134) reads records whose baseline names `differential`, and
  `tests/test_differential.py` requires `store/runs/discrepancy-ledger.json` to equal that rebuild, so the Rust judges'
  records (baseline `judge`) keep a ledger of their own: `store/case_runner/discrepancy-ledger.json`, rebuilt in Rust by
  the same rule (a REPORTED opens an entry, a ROUTED gives its repair path, a RESOLVED closes it, records read in
  run-start order), at `case_runs`' start, written at its end. A judge's accepted class resolves the entries the ledger
  holds open on it, as `differential.py` 219-226 does. The chain is tested on a planted record: the CF flip reports, the
  ledger opens; the plain run resolves, the ledger closes.
- **Medium 1, one profile a session.** The 22 runs came from three CPUs (19 Xeon Platinum 8573C, 2 Xeon Platinum 8370C,
  1 Xeon 6973P-C). `case_runs` judges the runs of one reference CPU a session, chosen by `W2M_CASE_CPU`; three records
  a judge. SESSION_INIT names that CPU and profile; each COMPARISON_COMPLETED's `reference_ref` names that session's
  runs, and `target_execution_id` names the session.
- **Medium 2, the race.** `test_timed_waits`'s retried context read failed once under the gates' load in Sprint 150's
  first gates run; its comment records 0 hits in 10 runs under the gates before. The hit goes in BLACKBOARD's Deferred
  with the load and the count; Sprint 150's second full gates run exited 0, which its tail entry did not say.
- **Low.** The fixture header's `src` is checked against `git log -1 --format=%H -- src/w2m`; the header gains a
  `layout` field, the sha256 of the guest layout constants the words may embed (`guest_runtime::layout::runtime_defines`
  as `name=0x<value>` lines), checked against the Rust guest's; the revisit reports the smallest per-module judged
  count beside the reach; `exit_count` gains the model exit's give-back (5203-5204: `_unrun_after(e.pc - 4)` for
  `hvc #10` and `#11`).
- **The third judge (the review's suggestion for link 5).** The Rust words equal the fixture's for every case code the
  Rust rules lift, all 25,919 codes walked with no VM.

Out of scope: a vocabulary proposal for judge-only executions (the review's typed-proposal route); this sprint emits no
execution event, so T4 does not apply.

## prerequisites

- 151 (closed)

## context_files

- reviewer-2's review of Sprints 149-150 (2026-10-25); `src/w2m/differential.py` (read in full); `scripts/vm_oracle.py`
  302-334; `tests/test_differential.py` 49-120
- `crates/w2m_vm/src/bin/case_runs.rs`, `case_inputs.rs`, `run.rs`; `crates/signal_report/src/main.rs` (the record
  frames reader); `src/w2m/vm_backend.py` 5195-5208

## signal contract

### Emits

- Per judge (`case-runner`, `case-runner-rust`) and per reference CPU (three): SESSION_INIT {profile the CPU and
  profile}, CANDIDATE_GENERATED, COMPARISON_COMPLETED {verdict conforms} and CANDIDATE_ACCEPTED per class judged in
  that CPU's runs, SESSION_COMPLETED {completed}. No EXECUTION_* or TRACE_* event.
- The planted CF flip (not committed): CANDIDATE_REJECTED, DISCREPANCY_REPORTED and DISCREPANCY_ROUTED; the plain run
  after it: DISCREPANCY_RESOLVED for each.

### Consumes

The case files, the downloaded Windows results, the fixture.

### Invariants

- No Python file added or changed.

## artifact contract

### Files created

- `store/case_runner/discrepancy-ledger.json`; six records under `store/runs/`

### Files modified

- `crates/w2m_vm/` (`bin/case_runs.rs`, `case_inputs.rs`, `run.rs`, `tests/case_runner.rs`), `harness/case-runs.toml`,
  `harness/case-runs-rust.toml`, `WORKING_AGREEMENT.md`, `BLACKBOARD.md`; the fixture regenerated with its `layout`
  field (`store/case_runner/python-words.sha256`)

### Files removed

- `store/case_runner/traces/` and the Sprint 150 record (`git rm`): the record states a validation that never happened

### Command exit codes

- `scripts/gates.sh` returns 0; `substrate replay --level 2` of each record returns 0

## observation contract

- **Expected values:** six records, each replaying at level 2 and held by no rule of the vocabulary; the classes
  accepted per CPU sum over the three sessions to every class judged; the ledger holds no open entry after the plain
  runs.
- **Planted faults:** the CF flip on the 8573C session opens ledger entries; the plain run of the same judge and CPU
  resolves them, each DISCREPANCY_RESOLVED naming the accepted candidate. A fixture header with a wrong `src` is refused.

## done criteria

The case runner's records say only what happened, each covers one reference CPU, and its discrepancies enter a ledger
that a later run resolves.

## self-review checklist

- [x] Scope names verifiable properties, each from a review finding.
- [x] context_files cite the Python carried.
- [x] Emits: each tag, per judge and CPU, and the plant's chain.
- [x] Artifact contract gradable by command.
- [x] Observation contract: expected values and planted faults.
- [x] One concept: the judges' records.

## result

- **High 1.** `case_runs` writes no EXECUTION_*, TRACE_* event, as `differential.run` wrote none; Sprint 150's record
  and its 22 verdict traces are removed (`git rm`). Amended from the card: the runs' counts do not go in the baseline,
  which a run file fixes before the Producer starts; they stay in `store/case_runner/expected.tsv`, asserted by the
  test, and each comparison's `rule_ref` names its class's judged count and the session's profile.
- **High 2.** `crates/w2m_vm/src/ledger.rs` carries `rebuild_ledger` and `open_on` for records whose baseline names a
  `judge`, and writes the ledger as `json.dumps(indent=1, sort_keys=True)` does (a unit test holds the text to the
  frozen Python's for one map). Amended from the card: a Producer cannot read its own record before substrate
  finalises it, so `case_runs` rebuilds the ledger at its start and `case_runs --write-ledger` writes the file after
  a run. The chain, run in the scratchpad: the CF flip on the 8573C runs reported 107 discrepancies, the ledger opened
  107 entries (the first "case-runner-…-ADD", "ADD: ADD CF: 1212 cases; run 38011862284 …", repair path
  `implementation_repair`); the plain session of the same judge and profile accepted 371 classes and resolved all 107
  (DISCREPANCY_RESOLVED 107, each resolving_ref the accepted candidate), and the ledger held 0 open. A unit test carries
  `test_differential.py`'s report-to-resolution check on two synthetic records.
- **Medium 1.** Measured, and the card was short: a reference profile is the CPU and the Windows image, and the 8573C
  runs span two images (9 on Server 2022, 10 on Server 2025). `W2M_CASE_PROFILE` selects `<cpu>; <profile>`; four
  profiles, eight sessions. Classes accepted per session, each judge: 205 (8573C, 2022), 174 (8573C, 2025), 107
  (8370C), 26 (6973P-C); together 387. Every session replays at level 2 and the vocabulary's rules hold none of it; the
  store's judges' ledger is `{}`.
- **Medium 2.** BLACKBOARD Deferred, 2026-10-25 (Sprint 152): the race hit, its load and its counts. Sprint 150's
  second full gates run exited 0, which its tail entry did not state.
- **Low.** The fixture's header gains `layout` (the sha256 of `runtime_defines()`, 049a5a85…, equal in the Rust guest)
  and its `src` is checked against `git log -1 -- src/w2m`; the words are unchanged, the fixture's sha256 is now
  a20dc6ef…. A header naming src 0000… was refused: "the fixture was made at src/w2m 0000…, and src/w2m is now at
  37784f1a…". The reach line gives the fewest cases a reached module has. `exit_count` takes a `GiveBack` (none, a
  fault's, a model step's `_unrun_after(pc - 4)`), used by `run_cases` and `run_to_first_decline`. The review's count of
  248 modules is not borne out: `store/rules/registry.tsv` holds 460 lines mapping to 249 distinct modules.
- **The third judge.** With no VM, the Rust words equal the Python words for all 4,103 of the 25,919 case codes the
  Rust rules lift (38 classes), in 0.06 s.
