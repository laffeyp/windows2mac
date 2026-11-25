# Sprint 335 — The test build keeps no object files in its deps directory

```yaml
---
id: 335
status: active
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

GATES-LINE
