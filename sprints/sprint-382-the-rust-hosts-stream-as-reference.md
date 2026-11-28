# Sprint 382 — The Rust host's batch stream stored as the reference past the frozen host's cut

```yaml
---
id: 382
status: done
phase: 6
pass_kind: functional
---
```

## scope

Sprint 381's profile re-take moved the game's batch run from row 29, so the frozen Python host's stream
(`store/frontier-calls/frontier-643a6283510c-1792623423346201000.tsv`) holds row for row only to row 28 of 10,505
(`LAST_HELD_ROW`, `crates/w2m_vm/tests/batch_run.rs`), and Windows' judge, G3, alone holds the rest. G3 tolerates
scheduling; the frozen stream held the exact order on the one-vCPU instruction clock, the project's determinism
anchor. The Decision "The runner's strings re-taken, and the frozen host's reference after them" (Sprint 381) fires
its revisit now: the Rust host's own stream becomes the reference past the cut.

- **The stream.** The one-vCPU batch run's call stream (`build/batch_run/first-stop.calls.tsv`, 10,515 rows at
  Sprint 381's close, two runs alike) made at the tree of 758f9f900 and stored under `store/batch_run/` with a header
  naming the commit, the run and the date; the frozen stream kept unchanged as the port's history.
- **The test.** The batch-run test holds the run's stream to the frozen stream to `LAST_HELD_ROW`, and to the stored
  Rust stream row for row in every row, cell for cell; a difference fails naming the row and the cell. The two-vCPU run
  stays with G3 alone (its order is the wall clock's).
- **The rule for later cards.** A card that changes the stream regenerates the stored file from its closing run, and
  its result states the rows and cells that changed and why (measured, as Sprint 379 measured); the file's header
  gains one line per such card, as `store/exit_ledger/differences-2026-10-29.tsv`'s does.
- **Not here:** any guest answer. No API id is added. No Windows run.

## prerequisites

- 381 (closed)

## context_files

- `crates/w2m_vm/tests/batch_run.rs` (`LAST_HELD_ROW`, the stream test, the provenance check `built_from`).
- `store/batch_run/` (its files' headers), `store/exit_ledger/differences-2026-10-29.tsv` (its header's form).
- `scripts/gates.sh` (the batch-run step that writes `build/batch_run/`).
- `grep -n "The runner's strings re-taken, and the frozen host's reference after them\|where the frozen stream's cut falls" BLACKBOARD.md`.
- `WORKING_AGREEMENT.md` lines 137-266 and 301.

## signal contract

### Emits

- None declared: the change is to a test and a stored reference.

### Invariants

- Every run as held; G3 conforms on one and two vCPUs; the frozen stream byte-equal to HEAD's;
  `crates/guest_runtime/src/api_ids.txt` unchanged; no Windows run.

## artifact contract

### Files created

- the stored Rust stream under `store/batch_run/`, its name dated.

### Files modified

- `crates/w2m_vm/tests/batch_run.rs`, with others only where the class requires; file count recorded.

### Command exit codes

- `scripts/gates.sh` returns 0
- `target/release/signal_report --check` returns 0 on the tree

## observation contract

- **Expected values:**
  - The stored stream's row count and sha256, the commit and run it came from, and that two runs at that tree give it
    byte for byte.
  - The test's held share: rows 0-28 against the frozen stream, all 10,515 against the stored stream.
- **Planted faults:**
  - (1) One cell past row 28 changed in the run's stream (in `build/`, data): the test fails naming the row and cell.
  - (2) One row removed from the stored stream: the test fails naming the first row that differs.

## done criteria

The one-vCPU batch run is held row for row and cell for cell to a stored stream of the Rust host's own run in every
row, and to the frozen Python host's to the cut.

## self-review checklist

- [x] Scope names one verifiable property: the whole one-vCPU stream held row for row again.
- [x] context_files cite the test, the store, the gates step and the Decisions.
- [x] Emits: none, with the reason.
- [x] Artifact contract gradable by command.
- [x] Observation contract: the stream's count and digest, the held share; numbered plants.
- [x] One concept.

## result

- **The two runs at 758f9f900's tree are not byte-equal.** Each is `scripts/gates.sh` 168-180, the batch-run step,
  copied verbatim into `scratchpad/382/batch-run-step.sh` and run from the root after `cargo build --release --bin
  w2m-vm --bin check_records --bin call_compare`, its stamp `0c859882b4fe 0676170e46a5` (crates/ tree 99369f1307af
  and tools/ tree c2bde70f39af, 758f9f900's and HEAD's). Run 1, record `build/batch_run/record-1795854811000000000`,
  08:33:31-08:35:04Z: sha256 8479bbd41a3c05d2174b884924510f3ae2f3b8ecfd17587dd19bc83a86ed203c. Run 2,
  `record-1795854911000000000`, 08:35:11-08:36:42Z: sha256
  df1a36abb4c7408ffb7e44b7ca49ed75a84323ad7259d48dc78c794c94f5be08. Both have 10,515 rows of 15 fields and stop at
  "ReferenceEnd: the log holds the reference's 168 lines and the main thread its calls", with 10515 calls traced, 0 in
  flight and 0 dropped. `cmp`: "differ: char 46501, line 427". Every differing cell, by awk: rows 426 and 438, columns
  9, 10 and 11 (GetFileAttributesExW's a4-a6, the FILETIME low dwords the call leaves on the stack, 0x26fafc2d against
  0x61c59246). All six are in `store/batch_run/stream-varying-2026-10-28.tsv`, where the frozen host's own runs differ
  (Sprint 194). With those eight cells masked, the two runs are byte-equal (`cmp`; sha256 bdbc83b0aa94 for each).
  Sprint 381's "two runs alike" was a statement about the stop file's counts. The stream itself had not been compared.
- **The stored stream**, `store/batch_run/first-stop-calls-2026-11-28.tsv`: two `#` header lines written by
  `printf`, then run 2's `build/batch_run/first-stop.calls.tsv` appended by `cat`. `grep -v '^#' | cmp -` against
  the run's file shows the rows byte-equal. The file has 10,515 rows of 15 fields; its rows' sha256 is df1a36ab…be08
  and the whole file's is 7d9c491e673d8cc58198c9ab0bb4b46b0b5f1a58f291e89c0c8a90ef820d708a. The first header line
  states what the file is, its judge, and the rule for later cards (regenerate from the closing run, state the rows
  and cells that changed and why, and add one line). The second is this card's line: the commit, the tree, the
  record, the date, the count, the digest and the second run's cells. The frozen stream
  `store/frontier-calls/frontier-643a6283510c-1792623423346201000.tsv` is unchanged (`git status` shows no change
  under `store/frontier-calls/`).
- **The test** (`crates/w2m_vm/tests/batch_run.rs`): `RUST_STREAM` names the file (66-70). A new test,
  `the_batch_runs_calls_are_the_stored_rust_runs_in_every_row` (342-388), holds the run's stream to it row for row
  over every row and cell for cell, skipping the `VARYING` cells. A row missing on either side fails with both row
  counts. A differing cell fails naming the row, the column, both values and both calls. Cells are compared as
  written, with no address naming, since the stored stream ran its own tree's image. The frozen-stream test still
  holds rows 0-28 and the calls at the cut, and its printed line now says the rows past the cut "are held to"
  `RUST_STREAM` "and judged against Windows". The doc comments of the module (17-19), `VARYING` (34-36) and
  `LAST_HELD_ROW` (64) state the change. The Rust runs vary in the same cells: runs 3-7 below differ from run 2 at
  rows 426 and 438, columns 9-12, all eight varying cells.
- **The regeneration rule held in code** (reviewer-4's fifty-seventh review, added partway through the sprint): the
  test `the_stored_streams_header_names_the_sprint_that_last_changed_it` (426-459, with `header_sprint`,
  `active_sprint` and `git` at 390-424). The last `# Sprint N` header line must name one of two sprints. While the
  file differs from HEAD's, or HEAD does not hold it (`git show HEAD:<file>`), that is the active card's sprint
  (`sprints/sprint-NNN-*.md` holding `status: active`; none or two fail). Otherwise it is the "Sprint N" that begins
  `git log -1 --format=%s -- <file>`'s subject. Now: "store/batch_run/first-stop-calls-2026-11-28.tsv's header names
  Sprint 382, the active card, the file differing from HEAD's or not in HEAD". The committed branch's parse was
  checked on `first-stop-2026-10-28.tsv`, whose last commit's subject begins "Sprint 381:". The header line stays as
  `printf` wrote it; no line was added for this change, the stream's rows being unchanged.
- **What the stored stream is:** a change detector, not a correctness check. It says that the one-vCPU run's calls
  moved from the Rust host's own run at 758f9f900, not that either run is right. G3, `call_compare` against the
  Windows runs, judges the batch run's correctness. The frozen Python host now checks the port's equivalence on rows
  0-28 alone.
- **The held share**, from the tests on the final tree's run (run 8, `record-1795855939000000000`, 08:52:19-08:53:52Z,
  sha256 a15551ea27e1, equal to run 2 but in the eight varying cells; run 7, `record-1795855650000000000`,
  08:47:30-08:49:04Z, sha256 35faf1af6235, gave the same lines before the header test was added): "the Rust stream equals the Python's rows 0-28 of 10505, to the start environment re-taken from
  the laptop (0.3% held); its 10486 rows past it are held to store/batch_run/first-stop-calls-2026-11-28.tsv and
  judged against Windows", and "the Rust stream equals store/batch_run/first-stop-calls-2026-11-28.tsv's 10515 rows,
  every cell but the 8 varying". So rows 0-28 are held to the frozen stream and all 10,515 to the stored one. Runs 3-6
  (sha256 212ca7ce7ffc, 7e437c8715a4, 99cb48921801, 9e2c9780913f) were taken after each edit of the test's comments.
  Each edit changed `crates/`' tree and so staled the outputs' stamps (`built_from::fresh`). Every run gave 10,515
  rows equal to run 2 outside the varying cells.
- **Decisions:**
  - The stored stream is held outside the varying cells, because two Rust runs at one tree differ there. Grounds:
    project: `stream-varying-2026-10-28.tsv` and Sprint 194's finding (a FILETIME the call leaves on the stack), and
    the seven runs above. SDD: AGENTS.md "Resolve-and-record". Practice: n/a. Revisit if a run differs from the
    stored stream outside those cells, or if the host's file times stop reaching the game.
  - Cells are compared as written, not by the runtime image's names, because the stored stream and the run share one
    image until a card moves it, and that card regenerates the file. Grounds: project: the card's "cell for cell"
    and its rule for later cards. SDD: "Resolve-and-record". Practice: n/a. Revisit if a card's regeneration shows
    that only address cells moved.
  - The check is a test of its own beside the frozen one, so the frozen test's in-flight logic and the cut's checks
    stand unchanged. Grounds: project: the card's "The test holds ... to the frozen stream to `LAST_HELD_ROW`, and to
    the stored Rust stream". SDD: TECHNIQUES #44. Practice: n/a. Revisit if the two are ever needed in one failure.
  - The file is named `first-stop-calls-2026-11-28.tsv`, after `build/batch_run/first-stop.calls.tsv` and beside
    `first-stop-2026-10-28.tsv`, and dated by the local day. Grounds: project: the card's "its name dated". SDD:
    "Resolve-and-record". Practice: n/a. Revisit if: none.
- **File count** (`WORKING_AGREEMENT.md` 301): 1 created (the stored stream) and 1 modified (`batch_run.rs`), plus
  this card and the gates' rows. One concept: the Rust host's own stream as the reference. No guest answer changed,
  `crates/guest_runtime/src/api_ids.txt` is unchanged, and no Windows run was made.
- **Plant 1** "row 5000 column 12 of the call stream differs from store/batch_run/first-stop-calls-2026-11-28.tsv's:
  the run's 1807b2d8a, the stored 1807b2d89 (the run's call LeaveCriticalSection, the stored LeaveCriticalSection)",
  `batch_run::the_batch_runs_calls_are_the_stored_rust_runs_in_every_row` exit 100 (nextest); live 08:54:05-08:54:06
  UTC:
  - backup `scratchpad/plants/382/plant-1c.backup` (`cmp` equal); in `build/batch_run/first-stop.calls.tsv` (run
    8's), line 5001 (row 5000, LeaveCriticalSection, past row 28) column 12 set from 1807b2d89 to 1807b2d8a by `sed`;
    `cmp -l` shows 1 byte changed, and awk shows 10,515 rows of 15 fields. The three stream tests were run under
    `W2M_REQUIRE_CASE_RUNS=1`. The frozen-stream test passed (rows 0-28 untouched). Log `plant-1c.log`. Restored by
    `cp`, `cmp` equal.
  - Earlier runs of the same plant, each with the same row and column: 08:41:01-08:41:02 UTC (`plant-1.log`, whose
    message printed `Some("…")`, after which the message was made to print bare values) and 08:49:15-08:49:16 UTC
    (`plant-1b.log`), both on trees before the header test.
- **Plant 2** "row 6000 column 0 of the call stream differs from store/batch_run/first-stop-calls-2026-11-28.tsv's:
  the run's 6219, the stored 6220 (the run's call HeapReAlloc, the stored HeapReAlloc)", the same test, exit 100;
  live 08:54:06-08:54:07 UTC:
  - backup `scratchpad/plants/382/plant-2c.backup` (`cmp` equal); in the stored stream, line 6003 (row 6000, call
    6219 HeapReAlloc) deleted by `sed`; awk shows 2 header lines and 10,514 rows of 15 fields. The test names row
    6000, the first row that differs. Log `plant-2c.log`. Restored by `cp`, `cmp` equal. An earlier run at
    08:49:25-08:49:26 UTC (`plant-2.log`) gave the same line.
- **Plant 3** "store/batch_run/first-stop-calls-2026-11-28.tsv's header's last \"# Sprint N\" line names None;
  expected Sprint 382, the active card, the file differing from HEAD's or not in HEAD",
  `batch_run::the_stored_streams_header_names_the_sprint_that_last_changed_it` exit 100; live 08:54:07-08:54:08 UTC:
  - backup `scratchpad/plants/382/plant-3.backup` (`cmp` equal); the stored stream's line 2, its `# Sprint 382:`
    line, deleted by `sed`, which gives a regenerated stream without its new header line. awk shows 1 header line and
    10,515 rows of 15 fields, and `grep -c '^# Sprint '` shows 0. The two row tests passed. Log `plant-3.log`.
    Restored by `cp`, `cmp` equal, sha256 7d9c491e…708a again.
- **The first gates run** was started at 08:51:02Z and ended during its batch-run step, with no pass row and its four
  step rows kept in `store/gates/steps.tsv`. It was stopped from outside when the header assertion was asked for. The
  gates below ran once, on the tree with it.

- **The judges in the gates run**, read again from its outputs after it (`cargo nextest run -p call_compare -p w2m_vm`
  on `binary_id(call_compare::call_compare)` and `batch_run::`, `--no-capture`, 18 passed, no file changed): G3, "the
  Rust host's 10515 calls: conforms, 0 defects, 0 past the cut (0 for the frozen host's same rows), 0 scheduling, 25
  values Windows varies" and "the Rust host's 10463 calls on two vCPUs: conforms, 0 defects, 0 past the cut, 0
  scheduling, 25 values Windows varies"; the three stream tests' lines as in the held share above, and
  "store/batch_run/first-stop-calls-2026-11-28.tsv's header names Sprint 382, the active card, the file differing from
  HEAD's or not in HEAD".
- **Gates:** `exit 0 in 584 s	default	2026-11-28T09:04:50Z	sprint 382	3d48e90e0e3f38453c901f0f75023d6f345207f5	1046d7ce1fae773bbf4cae46b09c432f639b0ab33cc2fdff9177cf6f15d49944	root=0a6291ec218cdfe74623439a2b91df0c8e9921ff	rust=crates python=0 samples=0	base=758f9f900e666a306dafc30e98a86fe47feb2643`
  (`rust=crates`: the change's crates and their dependants, `rdeps(signal_report) | rdeps(w2m_vm)` and the tree
  tests, 339 tests run, 339 passed; the batch runs ran, one vCPU 93 s and two vCPUs 174 s, with their records' checks
  and replays, the guests and the family runs).
- **G3's two-vCPU batch run in these gates:** `run_seconds` 132.819424748 (variant vm,
  `build/batch_run/record-2-vcpus-1795856207000000000`, verdict conforms); `load_at_start` "load averages 1.49 1.98
  2.17; busy: mds_stores 96%; other w2m: none".
