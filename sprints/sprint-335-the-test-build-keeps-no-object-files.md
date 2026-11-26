# Sprint 335 — The test build keeps no object files in its deps directory

```yaml
---
id: 335
status: done
phase: 6
pass_kind: functional
---
```

## scope

Sprint 334 placed the gates' slowdown (Deferred (Sprint 334), "The gates' time placed and not yet changed"). Its
findings:
- `target/debug/deps` holds 859,305 entries, and 854,915 of them are `.rcgu.o` object files. The dev and test
  profiles keep these as debug information under Cargo's documented macOS default, `split-debuginfo = "unpacked"`.
- One relink of three crates added 2,111 entries.
- `codesign -d` on a test binary takes 7.65-7.85 s in that directory and 0.00 s on a copy alone, reading the
  directory in 3,798 of 3,904 samples.
- A fresh copy's first launch takes 24.34 s there and 0.12 s alone.
- nextest spends 470 s of a 1,024 s run before its first test, and 708 s of a 1,159 s run that relinks three crates.

This card makes the test build stop keeping those object files, and measures the same figures before and after.

- **The work:**
  - **The setting.** Change one setting in the workspace `Cargo.toml`, under `[profile.dev]`, which the test profile
    inherits, so that a test build leaves no `.rcgu.o` in `target/debug/deps`. Choose between `split-debuginfo =
    "packed"` and a lower `debug` level from what the Cargo book's profiles page and the rustc book's
    `-C split-debuginfo` page say, read on this toolchain (quote both), and from the measurement below. Do not take
    either from memory.
  - **The one-time clearing.** Clear the existing debug build, `cargo clean --profile dev`. This removes build output,
    not project files.
  - **Keeping the debug information.** A failing test's panic must still name its file and line after the change.
    Check this with one test made to panic, run before and after, and quote both outputs.
  - **The record.** Add one dated note to WORKING_AGREEMENT's build section, naming the setting and why.
- **Before and after, each figure measured with no gates running, its command quoted:**
  - (1) `ls target/debug/deps | wc -l` and its `.rcgu.o` count.
  - (2) `codesign -d --entitlements -` on one test binary, five times.
  - (3) A fresh copy's first launch in `target/debug/deps`.
  - (4) `cargo nextest list` over the workspace.
  - (5) The entries one relink adds: touch the same three files Sprint 334 touched (`crates/w2m_vm/src/run.rs`,
    `crates/hypervisor/src/lib.rs`, `crates/signal_report/src/main.rs`) and rebuild the tests.
  - (6) One gates run on that relinked tree, `GATES_CHANGED` set to those three paths, with its step times. Sprint
    334's relinked run is the before figure: 1,159 s total, nextest 878 s.

  The clean build's own time is reported as a figure of its own, not folded into another.
- **Amended 2026-11-25 (reviewer-4's ninth review, Medium 1):**
  - **Why the check changed.** A panic message names its file and line from `std::panic::Location`, which is compiled
    into the binary and does not read the debug information. So the check above passes under any setting, `debug = 0`
    included.
  - **The new check.** Run one test made to panic with `RUST_BACKTRACE=1`, through nextest as the gates run it, and
    quote its backtrace before and after. It passes when a frame inside the test's crate names its `.rs` file and line
    in both.
  - **Its plant.** `debug = 0` for that one run: the frame loses its file and line, and the check fails.
  - **If it fails after the change.** If the "after" backtrace lost the file and line, the `.dSYM` is not read where
    nextest runs the binary. That is the result, and it decides the setting.
- **Not here:**
  - Why nextest's launches of a stamped binary are scanned again while the shell's are not, unless the after figures
    show it gone.
  - The five slow packages' listings, likewise.
  - Any security setting on this Mac.
  - The vocabulary card (0.13), which follows this one.

## prerequisites

- 334 (closed)

## context_files

- `Cargo.toml` (the workspace's profiles: only `[profile.release]` today, `debug = 1`, `overflow-checks = true`)
- `.cargo/config.toml` (the test runner `scripts/sign_and_run.sh`) and `scripts/sign_and_run.sh` (the signature stamp
  beside each binary, Sprint 332)
- `scripts/gates.sh`: its `timed` helper and the nextest step (`grep -n 'timed\|nextest' scripts/gates.sh`)
- Card 334's result, the before figures and their commands: `sprints/sprint-334-each-gates-step-timed.md`
- The Deferred entry: `grep -n "The gates' time placed and not yet changed" BLACKBOARD.md`
- `WORKING_AGREEMENT.md` lines 137-266, "Build and verification commands", and its line naming "A sprint closes in its
  work commit" (the builder still stops at a gated tree)
- The Cargo book, profiles, `split-debuginfo` and `debug`; the rustc book, `-C split-debuginfo` (the toolchain's own
  copies via `rustup doc`, or doc.rust-lang.org at the toolchain's version)

## signal contract

### Emits

- None; a build profile setting, no runtime behavior.

### Invariants

- Every gate runs as before, and `scripts/gates.sh` returns 0.
- A failing test's panic still names its file and line.
- `target/release/signal_report --check` passes on the tree.

## artifact contract

### Files modified

- `Cargo.toml` (one profile setting); `WORKING_AGREEMENT.md` (one dated note); `store/gates/` (the runs' rows)

### Command exit codes

- `scripts/gates.sh` returns 0 (every stage, since `Cargo.toml` changes)
- `target/release/signal_report --check` returns 0 on the tree

## observation contract

- **Expected values:**
  - After the change and the clean build, `target/debug/deps` holds no `.rcgu.o`, and a relink of the three files
    adds none.
  - `codesign -d` and a fresh copy's first launch there are measured against Sprint 334's 7.65-7.85 s and 24.34 s.
  - The relinked gates run's total and its nextest step are measured against 1,159 s and 878 s.
  - Each figure is stated before and after, side by side, as measured, whichever way it falls.
  - If the relinked run comes in under 1,418 s (Sprint 330's run), the result says so. That answers the Low 1 of
    reviewer-4's seventh review, a date beside step 6's "Sprint 360".
- **Planted fault:** (1) The setting taken out. After a rebuild of one touched test crate, `target/debug/deps` gains
  `.rcgu.o` entries again, its count quoted against zero. Read the judge's reach before the work: the count is taken
  by `ls` on the directory the test binaries are linked into.

## done criteria

The test build keeps no object files in `target/debug/deps`. Each of Sprint 334's figures is measured before and after
the change. Test panics still name their file and line.

## self-review checklist

- [x] Scope names one verifiable property: no `.rcgu.o` in `target/debug/deps` after a test build.
- [x] context_files cite the profiles, the runner, the gates' timing, card 334's figures, the Deferred entry and the documentation to read.
- [x] Emits: none.
- [x] Artifact contract gradable by command.
- [x] Observation contract: six figures before and after; one plant, its reach read.
- [x] Sweet spot: two files, `Cargo.toml` and one note; one concept.

## result

- **The documentation, read on this toolchain** (rustc 1.99.1, cargo 1.99.1; `rustup`'s local copies under
  `share/doc/rust/html`). The Cargo book, profiles, `split-debuginfo`: "The default value for this option is unpacked on
  macOS for profiles that have debug information otherwise enabled"; `debug`: "\"line-tables-only\": line tables only.
  Generates the minimal amount of debug info for backtraces with filename/line number info"; `test`: "inherits =
  \"dev\"". The rustc book, `-C split-debuginfo`: "packed - This is the default for Windows MSVC and macOS ... all the
  debug information is packed into a separate file from the main executable ... on macOS this is a *.dSYM folder";
  "unpacked ... On macOS this means the original object files will contain debug information".
- **The choice, measured.** A scratch crate in the scratchpad with one test made to panic, built four ways, each in its
  own target directory (`CARGO_PROFILE_DEV_*` set for the build): the default, 37 entries in `deps`, 30 `.rcgu.o`;
  `split-debuginfo = "packed"`, 9 entries, 0 `.rcgu.o`, 2 `.dSYM`; `debug = "line-tables-only"`, 37 entries, 30
  `.rcgu.o`; `debug = "none"`, 7 entries, 0 `.rcgu.o`. The backtrace names `./tests/made_to_panic.rs:3:5` under the
  first three and no file under `none`. A lower `debug` level keeps the object files until it keeps no debug
  information at all, so the setting is `[profile.dev] split-debuginfo = "packed"` in `Cargo.toml`, with a comment.
- **The one-time clearing:** `cargo clean --profile dev`, "Removed 945992 files, 58.1GiB total", 99.78 s. **The clean
  build:** `cargo test --workspace --no-run`, 8.79 s real (55.23 user), exit 0 confirmed by a no-op rerun.
- **The six figures**, none measured beside a gates run:

  | figure | command | before | after |
  |---|---|---|---|
  | (1) | `ls target/debug/deps \| wc -l`, `grep -c '\.rcgu\.o$'` | 859,305 entries, 857,026 `.rcgu.o` | 300 entries, 0 `.rcgu.o`, 74 `.dSYM` |
  | (2) | `codesign -d --entitlements -` on `signal_report-*` (unit tests), five times | 7.91, 8.12, 8.04, 8.05, 7.94 s | 0.00 s five times |
  | (3) | a fresh copy, stamp written, launched through `scripts/sign_and_run.sh ... --list --format terse` in deps, five times | 25.24, 0.01, 0.00, 0.00, 0.00 s | 0.14, 0.00, 0.00, 0.00, 0.00 s |
  | (4) | `cargo nextest list --cargo-quiet --message-format json`, 60 binaries, 363 tests | 1,062.89 s, no binary relinked or stamp rewritten | 10.45 s signing 60 new binaries; 0.20 s unchanged |
  | (5) | `touch` run.rs, hypervisor lib.rs, signal_report main.rs; `cargo test --workspace --no-run` | 2,111 entries added, all `.rcgu.o`; 32.07 s | 0 added (360 before and after); 2.60 s |
  | (6) | the same touch, `GATES_CHANGED` the three paths, `scripts/gates.sh` | Sprint 334: 1,159 s, nextest 878 s | 375 s, nextest 177 s (257 passed), unplaced 7 s |

  The relinked run comes in under Sprint 330's 1,418 s. `deps` went from 300 to 548 entries over it, 0 `.rcgu.o`.
  Before figure (4) was 428 s slower than Sprint 334's 634.41 s on an unchanged tree. My copies for (3) were removed
  after each measurement.
- **The debug information, as amended:** `layout.rs` (translator) backed up, one test `made_to_panic_sprint_335` added
  by Edit, run as `RUST_BACKTRACE=1 cargo nextest run --cargo-quiet --status-level fail -p translator --test layout -E
  'test(made_to_panic_sprint_335)'`. Before, taken in a scratch target dir with `CARGO_PROFILE_DEV_SPLIT_DEBUGINFO=
  unpacked` for that build (368 `.rcgu.o` in its deps): frame "4: layout::made_to_panic_sprint_335 / at
  ./tests/layout.rs:20:5". After, in `target/`: the same frame, "at ./tests/layout.rs:20:5". The plant, `debug = 0`
  (`CARGO_PROFILE_DEV_DEBUG=0`, a scratch target dir): frames 4-6 name no file or line, the panic line alone keeping
  `layout.rs:20:5` from `std::panic::Location`. Restored by `cp`, `cmp` silent. The first form of the check, `cargo
  test` before the change and after, also gave the same frame both times.
- **Plant 1,** `Cargo.toml` backed up and its `[profile.dev]` setting removed by Edit, `touch
  crates/signal_report/src/main.rs`, `cargo test -q -p signal_report --no-run`: the judge, `ls target/debug/deps`,
  printed "entries 1266, rcgu.o 872" against 0 before it (e.g.
  `signal_report-0ed0c4da88bb3c12.02wc1ghyso4spw6b8f0tm0lbl.18qqo8p.rcgu.o`). Restored by `cp`, `cmp` silent; the planted
  output cleared by `cargo clean --profile dev` (11,336 files, 0.55 s) and the tests rebuilt in 9.05 s, 0 `.rcgu.o`.
  Both gates runs came after.
- **The record:** WORKING_AGREEMENT, one dated note heading "Build and verification commands".
- **The gates:** `exit 0 in 387 s	all	2026-11-26T00:19:21Z	sprint 335	98917f6664f2e315b0e9843dd990149efb1c29fc	fa462dbbf50fa4fc8c86219299818fa4e0beb954dd3ee637f629f478412ea031	root=6b041bd81c1191a4d2342f5b782f34bb0fa0276b	rust=all python=0 samples=1	base=2b77c630c49cf07daeead6364451f267ed1339de`
