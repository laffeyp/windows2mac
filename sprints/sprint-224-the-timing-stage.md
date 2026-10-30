# Sprint 224 — The timing stage

```yaml
---
id: 224
status: done
phase: 6
pass_kind: functional
---
```

## scope

The Decision "Sequence after Sprint 214" as annotated at Sprint 222's close (reviewer-3's Medium 2 of its review at
it), and the Drift watchlist's "Absolute timings drift between runs" as annotated in Sprint 222.

- **Where it stands, by lookup (2026-10-29):**
  - `crates/hypervisor/tests/exit_time.rs` 40 asserts an exit and its resume under 5,000 ns. It runs inside the
    gates' nextest run (`scripts/gates.sh` 197 and 201), which stops at its first failure.
  - In Sprint 222's first gates run it measured 56,333 ns at load averages of 189, 276 and 255. Nextest stopped, and
    152 of 224 tests went unrun. Alone it then measured 4,062 ns, and in the fourth run it passed.
  - WORKING_AGREEMENT 115 records 832 to 873 ns from Rust and 813 to 852 ns from `vm_monitor`'s `spin`. The
    test's own message names the C monitor's as 720 ns, a third figure the card reconciles.
- **The work:**
  - The exit time leaves the correctness run. It is measured in its own stage after the correctness tests, its figure
    and the load averages kept in a tracked file beside `multi-vcpu.tsv`. A figure over the bound is reported, as the
    multi-vCPU stage reports, and does not stop the gates.
  - The bound is read against the run's own conditions: the test records the figure and the load averages, and the
    stage names a figure over the bound at a load under a stated level as a failure that stops the gates.
  - The correctness run holds no wall-clock bound: a search of the crates' tests for `elapsed` and `Instant` under an
    assertion names any other.
- **Not here:** the multi-vCPU stage's frozen tests, plan step 6's.

## prerequisites

- 223 (closed)

## context_files

- `crates/hypervisor/tests/exit_time.rs`, `scripts/gates.sh` (the nextest runs, the multi-vCPU stage)
- `store/gates/multi-vcpu.tsv`; `crates/signal_report/src/main.rs` (`multi_vcpu_staged`, the form a stage's record is
  checked in); WORKING_AGREEMENT 115; BLACKBOARD Drift watchlist, "Absolute timings drift between runs"

## signal contract

### Emits

- None; a gates stage, as the multi-vCPU stage.

### Invariants

- Every correctness test runs in every gates run whatever the machine's load.

## artifact contract

### Files modified

- `crates/hypervisor/tests/exit_time.rs`, `scripts/gates.sh`, `store/gates/` (the timing record),
  `crates/signal_report/src/main.rs` if the close checks the stage

### Command exit codes

- `cargo nextest run` and `scripts/gates.sh` return 0

## observation contract

- **Expected values:** the exit time on this machine and the load at which it was taken; the correctness run's test
  count with the timing test gone from it.
- **Planted faults:**
  1. The bound set to 1 ns: the stage reports the figure over it, the gates go on, and the record holds the failure.
  2. The bound set to 1 ns at a load under the stated level: the stage stops the gates, naming the figure.
  3. A second wall-clock assertion added to a correctness test: the search names it.

## done criteria

No wall-clock bound sits in the gates' correctness run. The exit time is measured in its own stage, its figure and
load kept, and a figure over the bound on a quiet machine still stops the gates.

## self-review checklist

- [x] Scope names one verifiable property: no wall-clock bound in the correctness run.
- [x] context_files cite the test, the gates and the stage's record.
- [x] Emits: none, with the reason.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and three plants.
- [x] One concept: where a timing bound is judged.

## result

- **Departure, CI runs no Python** (BLACKBOARD Decision 2026-10-29, "No Python runs", and the user the same day: "no
  depeding on pythong"):
  - `.github/workflows/ci.yml` lost:
    - XED's build (`python3 mfile.py`), which no Rust test reads;
    - the eight case sets built with `semantics_differential.py`;
    - `check_vocab.py` and `gen_events.py`;
    - `gen_decoder.py`, ruff, mypy and `gen_stencil_table.py`;
    - pytest and `check_replay.py`.
  - CI keeps:
    - the Rust job;
    - substrate's session and vocabulary samples;
    - llvm-objdump;
    - the Python 3.14.8 distribution, downloaded as a corpus of Windows PE files that `pe_image`'s test reads, not
      run.
  - `class_judge.rs`' case-count test skips where the 8 built sets are absent, as `run_loop.rs` does. The gates
    require them (`W2M_REQUIRE_CASE_RUNS=1`). With set 065c's file moved away, the test skipped, "20 of the 21
    reference sets' case files", and failed under the flag. The file was restored at its 5,257,596 bytes.
  - Two Rust files read Python source as text, which the Decision's "a reference read as stored files" allows:
    - `host_requests.rs` reads the frozen host's dispatch;
    - `family_known.rs` reads `family_run.py`.
  - The card's line "Not here: the multi-vCPU stage's frozen tests" is moot. That stage left the gates in Sprint 223.
- **Departure, all work on `main`** (the user's Decision of 2026-10-29): `main` was fast-forwarded from bb7c899be to
  5f4452b14, 586 commits, and pushed. The checkout moved to `main`, carrying this sprint's uncommitted work.
  `scripts/gates.sh` (lines 9, 44 and 46) and WORKING_AGREEMENT (lines 13 and 253) measure the change against
  `origin/main`, not `origin/sprint-066`.
- **The timing stage:**
  - `exit_time.rs`' test is `#[ignore = "a wall-clock bound: the gates' timing stage runs it (Sprint 224)"]`, so
    the correctness run (nextest, and CI's `cargo test`) skips it. Its bound stays 5,000 ns.
  - `scripts/gates.sh` runs it after the Rust tests (`--run-ignored only`), reads the one-minute load average
    (`sysctl vm.loadavg`), and appends date, outcome, sprint, figure and load to `store/gates/timing.tsv`.
  - Over the bound with a load under 6, the performance cores of this Mac (6 of 18), it stops the gates. Under load
    it reports the figure and the gates go on.
  - The three figures reconcile: 720 ns is `measure_exit.py`'s of 2026-10-14, and 813 to 852 ns the monitor's
    `spin` on another day, both the C monitor's. The 832 to 873 ns are Rust's on an idle machine, and 4,062 ns
    Rust's under a load near 170.
- **The search:** `crates/hypervisor/tests/no_wall_clock_bound.rs` reads every crate's Rust files. It names any
  `assert!` that compares `elapsed()`, or a variable bound from it, with `<` or `<=`, and skips the file holding the
  ignored bound. Its first form skipped itself, because it quotes the ignore marker in a string; the skip now
  matches only an attribute line. The one other wall-clock assertion, `stop_from_another_thread.rs`' "at least
  50 ms", is a lower bound, which load cannot fail, and is not named.
- **Plant 1** (the bound 1 ns, at load 17.62): the stage reported "failed, 1395 ns an exit at load 17.62" and
  "reported, not stopping", and exited 0.
- **Plant 2** (the bound 1 ns, the quiet level 1,000 so this load counts as quiet): the stage stopped, exit 1, "the
  exit time is over its bound on a quiet machine (load 17.71)".
- Plants 1 and 2 ran the stage's lines (`gates.sh` 202-218) extracted to a scratch script. The first two
  extractions were empty: a pattern matched two lines, then a `$` was read as an anchor. Nothing ran on them, and
  `exit_time.rs` was restored after each. The plant rows were removed from `timing.tsv`, which did not exist
  before. `exit_time.rs` was restored byte for byte.
- **Departure, the README and the About line** (the user, 2026-10-29, ad hoc): GitHub's About line and the README's
  first line now read "w2m will one day (hopefully) run unmodified Windows x86-64 programs on an Apple silicon Mac."
  The README states that w2m is an experiment and not a product, lists its methods (a Hypervisor.framework guest per
  program, the hardware's TSO bit, which `monitor.rs` 137 sets, translation block by block, a Windows layer inside
  the guest, every part judged against Windows), and places it among related work: Wine and Proton, Rosetta 2,
  FEX-Emu and Box64. It names no competitor and makes no comparison of merit. Its Python badge became a Rust one,
  its plan the Rust port plan, and its status that of 2026-10-29. The README is gated, so this sprint's gates run
  was stopped before its timing stage, with no pass line, and run again with it.
- **Reviewer-3's review at 224** (no High), with this sprint's gates running, which were stopped with no pass line:
  - **Low 1:** the stage read "did not run" as "slow". It now has three outcomes: passed; over-bound, which
    depends on load; and no-figure, where no figure was read, which stops the gates at any load.
  - **Low 2:** the search exempted whole files and missed reversed comparisons and `duration_since`. Only the
    function under the ignore attribute is exempt now. The condition is split at its comparison: time on the left
    of `<`/`<=` or the right of `>`/`>=` is an upper bound. `duration_since` counts as a time source. The search's
    own fixture file is excluded by name. A second bound planted in `exit_time.rs` (`assert!(started.elapsed()
    .as_nanos() < 10)`) was named, and the file restored byte for byte.
  - **Low 3:** `ci.yml`'s header now says no project Python runs, and that substrate runs under `uv`.
  - Its Mediums go to Sprint 225 (the 8 case sets as release assets) and to the Deferred speed item, reworded to
    the stored figure.
- **Plant 3** (`assert!(started.elapsed() < Duration::from_secs(1), "slow")` added to
  `stop_from_another_thread.rs`): the search failed, naming the file and the line. Restored byte for byte.
