# Sprint 375 — G3's reference set chosen by its rules, so laptop game runs can join it

```yaml
---
id: 375
status: done
phase: 6
pass_kind: functional
---
```

## scope

G3 holds the game's batch run to a set of eleven Windows runs: runner run 38216081440 and the laptop runs
`lap-20261014T032442034Z-c8814547-01` to `-10`. The set is fixed: `crates/call_compare/src/lib.rs` 1251-1253, "the
membership rules, which read laptop and runner run files and records, are not ported; the set is the frozen host's
last choice". So no laptop game run made since 2026-10-14 can join it (the postmortem's next steps; the Decision "The
order after the stop, the cards that take each deferred item", card 375).

- **The rules ported**, from the frozen `scripts/call_compare.py` at the tag `frozen-python-2026-10-29` (read in full;
  text to read, never to run): `game_build` (544), `outside_set` (550) and `reference_set` (597), with `laptop_run`
  (523), `capture_thread` (529) and `load_reference` (536) as they need, into `call_compare`, so the set is computed
  from `store/game-runs/`, each run kept or left out with its reason.
- **A game run on the record**, as `scripts/record_game_run.py` did at the same tag (read in full): EXECUTION_STARTED
  and EXECUTION_COMPLETED for a stored game run, `profile_kind` `windows_reference`, under `store/runs/game-run-<run>/`,
  since `outside_set` reads each run's record. The vocabulary already has both tags; no tag or field is added.
- **Held to the frozen host's choice.** On today's store the computed set is exactly the eleven runs the frozen host
  chose, each left-out run named with the frozen host's reason; the G3 batch runs judge against the computed set, and
  each record's `reference_ref` is the set's. The three game runs Sprint 372 made with Steam signed in stay in `build/`
  and are not candidates.
- **Not here:** new laptop game runs with the Steam client not running (card 376); code-integrity state in
  `tools/windows_machine/profile.ps1`, the laptop's host key under `w2m-laptop` and the walk's exact profile pattern
  (card 377; corrected at the close, reviewer-4's forty-ninth review, Low 1).

## prerequisites

- 374 (closed)

## context_files

- `git show frozen-python-2026-10-29:scripts/call_compare.py` (lines 500-640 and its `main`) and
  `git show frozen-python-2026-10-29:scripts/record_game_run.py`, read in full (read only; no Python runs).
- `crates/call_compare/src/lib.rs` (the reference set's reader, 1240-1300) and its tests; `harness/w2m-vm-game.toml`;
  `scripts/batch-run-record.sh`.
- `store/game-runs/` (listing; one laptop and one runner run read in full); `store/runs/` (the frozen host's record
  `frontier-643a6283510c-1792623423346201000`, its `reference_set`).
- `signals/` (the vocabulary's EXECUTION_STARTED and EXECUTION_COMPLETED).
- `WORKING_AGREEMENT.md` lines 137-266.

## signal contract

### Emits

- None on today's store (amendment, builder-375's finding): every stored game run the rules read, 38 of the 44, is
  already on the record (`store/runs/game-run-<run>/`, written by the frozen `record_game_run.py`), and a record is
  never written over, so the record step writes none. Its execution start and completion payloads, built
  from each stored run, are held field for field to those 38 frozen records by a test; the step's first records are
  card 376's new laptop game runs. No other emit site changes.

### Invariants

- Every run as held; G3's batch runs conform as before, against the same eleven runs. No stored game run is rewritten.

## artifact contract

### Files created

- `store/runs/game-run-<run>/` for each stored game run the rules read; the port's tests.

### Files modified

- `crates/call_compare/src/lib.rs` and its tests, with others only where the class requires, reported to main when
  found. One concept: the reference set chosen by its rules.

### Command exit codes

- `scripts/gates.sh` returns 0
- `target/release/signal_report --check` returns 0 on the tree

## observation contract

- **Expected values:**
  - The frozen rules quoted beside the Rust that does each.
  - The computed set on today's store, and each run left out with its reason; the set equal to the eleven.
  - The game-run records' EXECUTION_STARTED and EXECUTION_COMPLETED fields for one laptop and one runner run.
  - G3's batch run's `reference_ref` from the computed set, its verdict as before.
- **Planted faults:**
  - (1) A stored laptop game run's `exec.json` with a game build other than the reference's: the rules leave it out,
    and the test naming the set fails if it stays in.
  - (2) The set's rule taking every stored game run: the set's test fails, naming the run that joined.

## done criteria

G3's reference set is computed from the stored game runs by the frozen host's rules, equal to its eleven runs today, so
a laptop game run that meets the rules joins it.

## self-review checklist

- [x] Scope names one verifiable property: the set computed by its rules, equal to the frozen choice.
- [x] context_files cite the frozen rules, the reader, the records and the vocabulary.
- [x] Emits: none on today's store, with its reason; the record step held to the 38 frozen records.
- [x] Artifact contract gradable by command.
- [x] Observation contract: the set and its reasons; numbered plants.
- [x] One concept: the reference set chosen by its rules.

## result

- **The rules ported** into `crates/call_compare/src/lib.rs` 1339-1649, each beside the frozen text it follows
  (`call_compare.py` at frozen-python-2026-10-29):
  - `laptop_run` 523, "its profile.txt starts "laptop-"": `laptop_run` 1362, which `load_reference` 1314 now calls for
    `capture_thread` 529 / `load_reference` 536 (the capture thread `min(calls)[1]`, 1321, as before).
  - `game_build` 544, `"sha256:" + next(f["sha256"] for f in listing if f["path"] == exe)`: `game_build` 1372.
  - `GAME_ARGUMENTS` 512 and `arguments` 515, `shlex.split(given, posix=False)` then `a.strip('"')`: 1354 and
    `arguments` 1428, with `split_command_line` 1390 for a command line given as one string.
  - `outside_set` 550-594, rule for rule at 1541: "no call trace with thread names" 1545; `thread_names` empty, "the
    capture names no threads (SetThreadDescription without names)" 1548; a laptop run's exec.json "arguments" missing
    1561, `!= GAME_ARGUMENTS` "other arguments ({recorded})" 1564, the reason showing the list as Python's f-string does
    (`python_text`, `python_repr` 1454); profile.json's `cpu.logical != PROCESSORS` 1576 (`w2m.vm_backend.PROCESSORS`
    4; the constant 1359 is `w2m_vm::process::PROCESSORS`, one definition, as `layout_constants.rs`'
    `every_fact_has_one_definition_or_a_named_reason` requires; the first gates run failed it on a second literal 4); a
    runner run the contract's `excluded_threads` does not name 1583; "not on the record (game-run-<run>)" 1589; no
    windows_reference EXECUTION_STARTED 1603; "another game build ({subject_sha256})" 1607. The record is read as
    substrate's `read_record` reads it (`read_record` 1495: sealed segments, then the hot one to its torn tail, a seq that
    does not follow refused), the frames' crc not checked.
  - `reference_set` 597-609, every directory of store/game-runs in name order: `reference_set` 1621 and
    `reference_set_of` 1647. `Windows::load` 1219 reads it in place of the frozen record's list; that list stays as
    `frozen_reference_set` 1260, which the set is held to.
- **A game run on the record**, `record_game_run.py`'s `work`: `game_run_execution` 1659 (the timeout `round` half to
  even, as Python's); `call_compare game-run-events` (`src/main.rs` 156), the Producer that adds `one_step_run`'s
  SESSION_INIT and SESSION_COMPLETED; and `call_compare record-game-runs ROOT` (`src/main.rs` 263), which records each run
  the rules leave out only as "not on the record", from a committed tree (`uncommitted` 216, the frozen check), through
  `uv run substrate run --strict` and the run file `game_run_file` 240 writes to `harness/.game-run-<pid>.toml`. On
  today's store it prints "0 stored game runs to put on the record; the rules read the record of 11, each on it" and
  writes nothing (38 runs are on the record; the rules leave 27 of them out before reading it).
  `the_record_step_writes_the_frozen_hosts_executions` holds `game_run_execution` to all 38 frozen records field for
  field. The run file's text (`a_game_run_is_recorded_from_this_run_file`) ran through substrate into
  `scratchpad/plants/375/record-verify-lap-20261014T032442034Z-c8814547-01`: run exit 0, `substrate replay --level 2`
  "[OK] Level 2 replay successful. Frames replayed: 10", its events SESSION_INIT, EXECUTION_STARTED,
  EXECUTION_COMPLETED, SESSION_COMPLETED; the two executions' values the stored record's (with `vcpus` and
  `guest_trace` null, fields the vocabulary gained after 2026-10-14), SESSION_INIT's harness version 0.1.0, the crate's,
  and vocabulary 0.14 where the frozen host wrote 0.0.1 and 0.7. `check_records` names T4 in it, "0 ingests of
  sha256:cc9e4a88...", as it names T4 in both frozen records it was run on (game-run-38216081440 and -c8814547-01): the
  call trace is not a vocabulary guest trace and no TRACE_INGESTED follows.
- **The set on today's store** (`call_compare reference-set .`, `the_rules_choose_the_frozen_hosts_reference_set`):
  38216081440 and lap-20261014T032442034Z-c8814547-01 to -10, the frozen host's eleven in its order; `reference_ref`
  "game-run-38216081440 + game-run-lap-20261014T032442034Z-c8814547-01 + ... -10". Left out, 33: "no call trace with
  thread names" 37982448071, 38006665419, 38006671518, 38113342549, 38113343972, lap-20261014T041748433Z-73c66e6a-01;
  "the capture names no threads (SetThreadDescription without names)" 38176899195, 38203364352, 38206263976; "the
  contract names no capture thread for it (another capture or arguments)" 38275716702; "other arguments
  (['-screen-fullscreen', '0', '-screen-width', '1280', '-screen-height', '720', '-logFile',
  '"C:\\w2m\\unity.log"'])" lap-20261014T011035282Z-e61fca06-01 to -03; "a machine of 12 processors, w2m reports 4"
  lap-20261014T012302244Z-cb542225-01 to -10 and lap-20261014T023701004Z-3f1cf08d-01 to -10.
- **The rules the store does not reach**, `each_rule_leaves_a_laptop_run_out_for_its_reason`, on copies of
  c8814547-01 under `CARGO_TARGET_TMPDIR`: as stored, and with its arguments as one command line, it joins; no
  arguments, no profile.json ("a machine of None processors"), no record, another profile kind and another game build
  each leave it out for its reason.
- **The records' fields**, laptop `game-run-lap-20261014T032442034Z-c8814547-01`: EXECUTION_STARTED subject_sha256
  sha256:643a6283...f974, profile_kind windows_reference, profile_id laptop-x64-10.0.26200.7840-Client,
  timeout_seconds 153; EXECUTION_COMPLETED exit_status timed_out, exit_code null, elapsed_seconds 153.038207,
  guest_line_count 3499, trace_sha256 sha256:cc9e4a88...8ce1. Runner `game-run-38216081440`: profile_id
  gha-windows-2025-x64-10.0.26100.33438-Server, timeout_seconds 180; timed_out, elapsed_seconds 180.490339,
  guest_line_count 3494, trace_sha256 sha256:d87830c1...f739.
- **Files beyond the card's list** (reported to main): `crates/call_compare/src/main.rs` (the three commands),
  `crates/call_compare/Cargo.toml` and `Cargo.lock` (`guest_runtime` for SHA-256, as `windows_machine` takes it, and
  `w2m_vm`, until now a dev-dependency, for `process::PROCESSORS`).
- **Plant 1** "runs the frozen host chose that the rules leave out, with the rules' reason: [ (
  "lap-20261014T032442034Z-c8814547-01", Some( "another game build
  (sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa)", ), ), ]", `cargo nextest run -p
  call_compare -E 'test(the_rules_choose_the_frozen_hosts_reference_set)'` exit 100; live 00:24:42-00:24:54 UTC:
  - the game build is read from the run's record, not its exec.json, which holds none (arguments, exited_by_itself,
    exit_code, elapsed_seconds): the plant is on the record's EXECUTION_STARTED;
  - backup `scratchpad/plants/375/plant1-events-000001.open.jsonl.bak`;
  - `subject_sha256` changed to sha256:aaaa... and the frame's crc to 494339a7 (`cksum -o 3` of the frame without its
    crc, the zlib CRC-32 substrate uses, which gave the stored 12f4f6ed for the original), by Edit in
    `store/runs/game-run-lap-20261014T032442034Z-c8814547-01/events-000001.open.jsonl`; the grep found it once;
  - the test failed naming the run and its reason (log `scratchpad/plants/375/plant1.log`); a first run, 00:23:29-00:24:00
    UTC, failed the same before the test's message was changed;
  - restored by `cp`; `cmp` equal.
- **Plant 2** "runs the rules let in that the frozen host left out: [ "37982448071", "38006665419", "38006671518",
  "38113342549", "38113343972", ...", `cargo nextest run -p call_compare -E
  'test(the_rules_choose_the_frozen_hosts_reference_set)'` exit 100; live 00:25:02-00:25:11 UTC:
  - backup `scratchpad/plants/375/plant2-lib.rs.bak`;
  - `reference_set`'s `Some(reason)` arm made `Some(_) => set.members.push(name_of(&run))` by Edit in
    `crates/call_compare/src/lib.rs`; the grep found it once;
  - the test failed naming all 33 runs that joined (log `scratchpad/plants/375/plant2.log`); a first run,
    00:24:07-00:24:24 UTC, named each with a `None` beside it, which the message dropped;
  - restored by `cp`; `cmp` equal.
- **G3 against the computed set**, the gates' batch runs: `build/batch_run/record-1795826339000000000` (one vCPU) and
  `build/batch_run/record-2-vcpus-1795826439000000000` (two), each COMPARISON_COMPLETED `verdict` "conforms", no
  `first_divergence`, `reference_ref` "game-run-38216081440 + game-run-lap-20261014T032442034Z-c8814547-01 + ... +
  game-run-lap-20261014T032442034Z-c8814547-10", the eleven of the set computed by the rules, as before.
- **What went wrong.** The first gates run (rows of 2026-11-28T00:28:24Z, no pass row) failed
  `layout_constants::every_fact_has_one_definition_or_a_named_reason`: "defined in two files, neither derived from the
  other nor held nor allowed: PROCESSORS in crates/w2m_vm/src/process.rs, crates/call_compare/src/lib.rs", call_compare
  stating the processor count as a second literal 4. The class was checked (call_compare's other `pub const`s,
  CONTRACT, GAME_RUNS and RECORDS, are defined nowhere else) and the constant derived from `w2m_vm::process::PROCESSORS`;
  the test, the crate's 22 tests and the gates passed after.
- **Gates**: exit 0 in 613 s	default	2026-11-28T00:48:58Z	sprint 375	75bcffba3fadc154fe4cb029c128b37afecd26f9	ca127d46acf67157e8123b119090c76ea22ab9184762970f6858ca5adaa381ec	root=03a97e72b2b27b3a9f376227c2cc8f077a0fb59b	rust=all python=0 samples=0	base=bb96338363e80768598d296997e38ebd2c0b6214
  (the two-vCPU batch run: run_seconds 175, its execution 174.94 s, load_at_start "load averages 2.27 2.47 2.46; busy:
  mds_stores 128%; other w2m: none"; the one-vCPU run 99 s, "load averages 2.35 2.39 2.44; busy: Visual 79%; other w2m:
  none").
