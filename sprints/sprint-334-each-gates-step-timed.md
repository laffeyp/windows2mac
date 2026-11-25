# Sprint 334 — Each gates step timed

```yaml
---
id: 334
status: done
phase: 6
pass_kind: functional
---
```

## scope

Every sprint waits on one `scripts/gates.sh` run, and the run has grown: 4,928 s at Sprint 331, 5,845 s at 332, 6,243 s
at 333. Sprint 332 placed part of it and left three figures unexplained (Deferred (Sprint 332), "The gates' time not
spent in `codesign -d`"): nextest's listing at about 22.5 s a test binary with one 9.85 s signature check each; about
50 minutes between nextest's end and the timing stage's output that the log does not place; and a test binary's first
launch in a series at 15 to 30 s with its signature stamp matching and no `codesign -d` run. The Decision "The gates'
stall, a card after 333" (2026-11-24) orders this card: put a time on each gates step's start and end, and measure each
of the three figures before any change. This card measures; it changes no step's work. The change the figures call for
is the next card's, written from this card's numbers.

- **The work:** a helper in `scripts/gates.sh` that stamps each step's start and end, prints "gates: step NAME N s",
  and appends one row a step to a new tracked file `store/gates/steps.tsv` (the run's UTC start, the sprint, the step's
  name, its seconds); every step of the script wrapped, from `cargo fmt --check` to the last `check_records`, the batch
  run, the guests, the family runs, nextest, the doc tests, the timing stage and each records check each a step of its
  own; at the end, one line "gates: steps N s of the run's M s, unplaced K s", printed before the pass row and kept in
  the same file as a row named `unplaced`. A step whose command exits non-zero still exits the gates as today (`set -e`);
  its end is not stamped, and the printed start names where the run stopped.
- **The three figures, each measured on this Mac with the gates not running and the result quoting the commands and
  their output:** (1) listing: one test binary's `--list` run five times through `scripts/sign_and_run.sh` with its stamp
  matching, and five times bare, each timed; then `cargo nextest list` over the workspace, timed whole, with its binary
  count. (2) the unplaced time: read from this sprint's own gates run, `store/gates/steps.tsv` naming the step or steps
  that hold it. (3) the first launch: a test binary copied fresh (new inode, signed, stamped) launched once and then
  four more times, each timed; and the same for a binary relaunched with no copy; `log show --last 5m --predicate
  'process == "syspolicyd" OR process == "XprotectService"'` read after each first launch, its count of lines quoted.
  Each figure is reported as measured, its step or cause named only where the measurement shows it ("No guesses").
- **Amended 2026-11-25 (reviewer-4's review at 334, High 1 and Medium 2):** (a) the step's time names the step, not
  what its process waits on, so each slow figure is also sampled while it is slow: `/usr/bin/sample <pid> 5` on a
  `codesign -d --entitlements - <binary>` while it runs, on a test binary in a slow launch if one is caught, and on
  `cargo-nextest` during a slow `cargo nextest list`; one `top -l 2 -o cpu -n 10` taken during the stall; each quoted
  by the frame or process it names. (b) the timing helper is one `timed NAME command...` that runs its command and
  stamps both ends, a stage of several commands a shell function run through it, so no step's end can be forgotten or
  skipped by an `exit` inside it; the `step`/`step_end` pair is not kept. The plant becomes a stage run outside
  `timed`, raising `unplaced` by that stage's seconds.
- **Amended 2026-11-25 (reviewer-4's eighth review at 334, High 1):** this card changes no Rust source, so its own
  gates runs relink no test binary (`runner-binaries-build` 0-5 s and nextest's tests 173.6 s in the --all run, against
  Sprint 333's 1,255 s after a Rust change), and every code card relinks. One more gates run, after the --all run, on a
  relinked tree: one source file `touch`ed, contents unchanged, in each of `crates/w2m_vm`, `crates/hypervisor` and
  `crates/signal_report`, then `GATES_CHANGED` set to those three files; the `sample` and `top -l 2` taken during its
  slow listings and first launches; `target/debug/deps`' file count read before and after it. Figures (2) and (3) and
  the "unplaced under 60 s" criterion are judged on that run too, and the result states both runs side by side.
- **Not here:** any change to a step's work, to the runner or to nextest's settings, the Developer Tools setting
  (`spctl developer-mode enable-terminal`) included, a candidate for the change card after this one if a sample names
  the launch assessment (reviewer-4's Medium 1); the vocabulary card (0.13), which follows this one.

## prerequisites

- 333 (closed)

## context_files

- `scripts/gates.sh` (all 310 lines: every step to be wrapped, the pass row at 303-310)
- `scripts/sign_and_run.sh` (the runner and its stamp, Sprint 332), `.cargo/config.toml` (the runner's setting)
- `scripts/gated-tree.sh` (`store/gates/` is kept out of the gated tree, so the new file is bookkeeping)
- `crates/signal_report/src/main.rs`: `is_bookkeeping` (`grep -n 'fn is_bookkeeping' crates/signal_report/src/main.rs`)
  and every reader of `store/gates/` (`grep -n 'store/gates' crates/signal_report/src/main.rs`)
- The Deferred entry: `grep -n "The gates' time not spent" BLACKBOARD.md`; the Decision: `grep -n "The gates' stall, a
  card after 333" BLACKBOARD.md`
- Sprint 332's card result, its measurements of `codesign -d` and the runner: `sprints/sprint-332-the-runner-checks-a-binary-once.md`
- `WORKING_AGREEMENT.md` lines 137-266, "Build and verification commands", and its line naming "A sprint closes in its
  work commit" (this sprint is the first to close in its work commit; the builder still stops at a gated tree)
- The last three runs' totals: `tail -3 store/gates/passes.tsv | cut -f1-4`

## signal contract

### Emits

- None; a build script's timing rows, no runtime behavior.

### Invariants

- Every gate the script runs today still runs, in the same order, with the same exit behavior.
- `target/release/signal_report --check` passes on the tree.

## artifact contract

### Files modified

- `scripts/gates.sh`; `store/gates/steps.tsv` (new, tracked); `WORKING_AGREEMENT.md` (one dated note naming the file
  and the printed lines)

### Command exit codes

- `scripts/gates.sh` returns 0
- `target/release/signal_report --check` returns 0 on the tree

## observation contract

- **Expected values:** this sprint's gates run (every stage, since `scripts/gates.sh` changes) prints a "gates: step"
  line for each step and the "steps ... unplaced K s" line, with K under 60; `store/gates/steps.tsv` holds the same rows,
  their seconds summing with `unplaced` to the pass row's "exit 0 in M s" within 2 s. The three figures as measured,
  each quoted with its command.
- **Planted fault:** (1) one step's helper call taken out (the family runs', the step Sprint 332 could not place by
  log): a gates run selected to that step's path (`GATES_CHANGED=crates/w2m_vm/src/run.rs scripts/gates.sh`, which runs
  the family runs) prints an `unplaced` figure larger by that step's seconds in the unplanted run, within 10%, and no
  "gates: step" line for it. Its reach read before the work: `GATES_CHANGED` sets the change (gates.sh 50-51), and a
  crate change sets `rust=crates`, which runs the family runs (gates.sh 140-190).

## done criteria

The time of every gates step is printed and kept, the run's unplaced time is under a minute, and each of Sprint 332's
three unexplained figures is measured and placed or stated as unplaced.

## self-review checklist

- [x] Scope names one verifiable property: every step's time is recorded and the unplaced time is under 60 s.
- [x] context_files cite the script, the runner, the bookkeeping rule, the Deferred entry, the Decision and Sprint 332's result.
- [x] Emits: none.
- [x] Artifact contract gradable by command.
- [x] Observation contract: the printed lines held to the kept rows and the pass row; one plant, its reach read.
- [x] One concept: the gates' time placed, before any change to it.

## result

- **The work** (`scripts/gates.sh`): `timed NAME COMMAND...` prints "gates: step NAME started", runs the command, then
  prints "gates: step NAME N s" and appends a row to `store/gates/steps.tsv` (the run's UTC start, "sprint N", the
  step, its seconds); a stage of several commands is a function run through it, 29 calls in all, from
  `windows-probes-compile` and `cargo-fmt` to `records-check`; no command's text, order or exit behaviour changed (the
  one rewrite, `[ -n "$docs" ] && cargo test ...` to `if [ -n "$docs" ]; then ...; fi`, keeps a function that ran no
  doc tests from returning 1). Before "gates: all passed": "gates: steps N s of the run's M s, unplaced K s", kept as
  the row `unplaced`. Under `/bin/sh` (bash 3.2) a failure inside a stage exits with its status, its "started" the
  last line and no row (scratchpad test: a `false` inside a stage exits 1, a command's exit 3 exits 3). The
  `step`/`step_end` pair of the first draft is gone (the amendment's (b)). `WORKING_AGREEMENT.md`: one dated note on
  the `scripts/gates.sh` bullet.
- **The four gates runs of this sprint, every row kept** (WORKING_AGREEMENT line 260); each run's `steps.tsv` rows,
  `unplaced` included, sum to its pass row's seconds exactly:

  | run (steps.tsv start) | selection | pass row | unplaced | nextest | tests' Summary |
  |---|---|---|---|---|---|
  | 22:17:36Z, planted | `GATES_CHANGED=crates/w2m_vm/src/run.rs` | exit 0 in 467 s, 22:25:23Z | 36 s | 230 s | — |
  | 22:25:55Z | the same | exit 0 in 379 s, 22:32:14Z | 8 s | 170 s | — |
  | 22:32:47Z, the GATES-LINE | all (gates.sh changed) | exit 0 in 1024 s, 22:49:51Z | 5 s | 644 s | 173.592 s, 362 tests, 60 binaries |
  | 22:52:14Z, relinked | three touched files | exit 0 in 1159 s, 23:11:33Z | 20 s | 878 s | 169.775 s, 257 tests, 25 binaries |

  The --all run's other steps: batch-run 101, doc-tests 107, timing-stage 51, planted-chain 48, family-runs 29,
  case-runs-ledger-check 15, windows-probes-compile 11 s, the rest 0-4 s. The relinked run (one file `touch`ed,
  contents unchanged, in `crates/w2m_vm/src/run.rs`, `crates/hypervisor/src/lib.rs`, `crates/signal_report/src/main.rs`):
  cargo-clippy 32, doc-tests 40 s.
- **Figure (2), the unplaced time:** 5 s of the --all run and 20 s of the relinked run, both under 60. Sprint 332's
  50 minutes between nextest's end and the timing stage's output are placed: here nextest's end and the timing stage are
  steps of their own, the doc tests 107 s (40 relinked) and the timing stage 51 s (0 relinked), with nothing between.
  The run's time is in the nextest step, 644 s (878 s relinked), against its tests' 173.6 s (169.8 s): 470 s (708 s)
  before the first test, the listing below.
- **Figure (1), listing**, no gates running: `scripts/sign_and_run.sh target/debug/deps/signal_report-e2b7d80e5e2d45f2
  --list --format terse` five times, stamp matching, 0.00 s each; bare, five times, 0.00 s each. `cargo nextest list
  --cargo-quiet --message-format json`, 60 binaries (`."rust-suites" | length`), 363 tests: 1,186.64 s real, 6.84 user,
  63.09 sys, during which 13 binaries were relinked (new inodes born 13:24:01-02 local) and re-signed (stamps
  13:24:40-13:36:17); again on the unchanged tree, no stamp rewritten: 634.41 s real, 0.69 user, 0.74 sys. The same 60
  binaries listed through the runner from the shell, one after another: 0.00 s each; all 60 launched at once: 0.01-0.02 s
  each, under 1 s whole. `cargo test --workspace --no-run`: 0.05 s. `cargo nextest list -p` package by package: image_walk
  157.42 s, x86_decoder 111.62, pe_image 27.80, w2m_vm 27.71, translator 27.68, the other fifteen 0.08-0.10 s; size does
  not order them (call_compare's 4.2 MB binary fast, x86_decoder's 3.1 MB slow).
- **The samples** (`/usr/bin/sample <pid> 5`): `unityplayer-68af847c69cd63fa --list`, parent cargo-nextest, launched
  14:08:49.850 local, sampled from 14:11:20: 1,059 of 1,062 main-thread samples at `_dyld_start (in dyld) + 0`, the
  process not yet in its own code after 150 s; in the relinked run `tree_tests-af16e1e0dec80331 --list --format terse
  --ignored`, launched 14:55:51.474, sampled from 14:56:13: 4,518 of 4,518 at `_dyld_start + 0`. `cargo-nextest`
  during a slow listing: 68,975 samples in `__psynch_cvwait`, 3,819 in `kevent`, waiting on its children. A test
  launch caught in the relinked run (`all-2b0df0a7fe93c1f7 --exact cache_file::...`) was in its own `main`, a long test
  at 67% CPU, not a held launch. `top -l 2 -o cpu -n 10` during the stall: amfid 21.2%, syspolicyd 13.1%, XprotectService
  7.7% "stuck" (at 22:10Z); syspolicyd 29.2% "stuck", amfid 22.0% (relinked run). `codesign -d --entitlements -` on
  `signal_report-e2b7d80e5e2d45f2`, 7.67 s: 3,798 of 3,904 samples in `__getdirentries64` under
  `SecStaticCodeCreateWithPathAndAttributes` → `DiskRep::bestGuess` → `_CFBundleCreate` → `_CFIterateDirectory` →
  `readdir` (2,755 under `_CFBundleGetBundleVersionForURL`, 1,149 under `CFBundleGetInfoDictionary`): codesign reading
  the directory the binary sits in. `target/debug/deps` holds 857,194 entries (859,305 after the relinked run, 2,111
  added); `codesign -d` on that binary there, five times, 7.85, 7.74, 7.79, 7.74, 7.66 s; on a copy alone in an empty
  directory, 0.00 s five times, both naming the entitlement.
- **Figure (3), the first launch**, no gates running: the binary above copied fresh (new inode 249570425), its stamp
  written, into `target/debug/deps`: 24.34 s, then 0.00 s four times; the same copy in an empty scratchpad directory
  (inode 249570439): 0.12 s, then 0.00 s four times; the binary relaunched with no copy: 0.00 s five times. `log show
  --last 5m --predicate 'process == "syspolicyd" OR process == "XprotectService"'` after each first launch: 448 and 470
  lines. syspolicyd's `[com.apple.syspolicy.exec:default] GK performScan` → `GK evaluateScanResult: 2` for the copy, its
  signing identifier `signal_report-e2b…`: 14:16:40.109 → 14:16:48.413 in deps (8.3 s of the 24.34 s), 14:16:49.285 →
  14:16:49.401 in the empty directory (0.12 s). During the listing, unityplayer's scan began 14:11:13.364, 143 s after
  its launch, and ended 14:11:21.893, the second its dyld was seen running; the scans' results came one at a time. Over
  the relinked run (14:52-15:11 local), 40 scans paired by path: 319.7 s in all, median 0.33 s, longest 20.55 s.
- **Placed by the measurements:** `codesign -d`'s 7.7 s is its read of the 857,194-entry directory; a launch under
  nextest waits at `_dyld_start + 0` for syspolicyd's Gatekeeper exec scan, and the same bytes' first launch costs 24.34 s
  in that directory and 0.12 s in an empty one. Cargo's documented default (the 1.99.1 Cargo book, profiles:
  "split-debuginfo ... The default value for this option is unpacked on macOS for profiles that have debug information
  otherwise enabled"; the rustc book: "unpacked ... On macOS this means the original object files will contain debug
  information") keeps each test build's `.rcgu.o` files (854,915 of the entries) as the debug information.
  **Not placed:** why nextest's launches of a binary are scanned again while the shell's launches of the same binary
  are not (0.00 s), and why five packages' listings hold the time and fifteen list in 0.1 s.
- **Plant 1,** `scripts/gates.sh` copied to the scratchpad, the line `timed family-runs family_runs` written
  `family_runs` by the Edit tool, run as `GATES_CHANGED=crates/w2m_vm/src/run.rs scripts/gates.sh`: the judge printed
  "gates: steps 431 s of the run's 467 s, unplaced 36 s" and no "gates: step family-runs" line (`grep -c family-runs`:
  0), while `build/family_runs/` held 1,365 files; the unplanted run on the same selection printed "gates: step
  family-runs 27 s" and "unplaced 8 s"; 36 − 8 = 28 s against 27 s, 3.7%, inside 10%. Restored by `cp`, `cmp` silent.
  That run's rows stay: pass row "exit 0 in 467 s" at 2026-11-25T22:25:23Z, `steps.tsv` rows of run start 22:17:36Z,
  its `timing.tsv` row.
- **The gates:** `exit 0 in 1024 s	all	2026-11-25T22:49:51Z	sprint 334	d5a265bd496f08815e0eff49783ba7d9c1afdaa3	5c9bed851ef979b543a6cf1370baef282747c01b35f85a1254e2257eaee252fb	root=9da55699358448bcebc6ce5bcbac2eee4f4b737b	rust=all python=0 samples=1	base=7490ba53d7918ec30d4452031c4e413b36c9faf8`
