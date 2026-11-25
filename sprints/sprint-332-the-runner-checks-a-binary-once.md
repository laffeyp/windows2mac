# Sprint 332 — The test runner checks a binary's signature once

```yaml
---
id: 332
status: active
phase: 6
pass_kind: functional
---
```

## scope

Every process the gates start for a test binary, its two listings and one process a test, goes through
`scripts/sign_and_run.sh`, the cargo runner (`.cargo/config.toml`, Sprint 132), which runs
`codesign -d --entitlements -` on the binary before it `exec`s it, to sign it with the Hypervisor entitlement if it
lacks one. Measured on 2026-11-24 during Sprint 331's second gates run: that check took 26.27 s on an already-signed,
already-run binary (`target/debug/deps/signal_report-e2b7d80e5e2d45f2`), and launching the binary took 0.00 s; Sprint
331's two gates runs spent about 45 and 40 minutes before their first test (the Decision "The test runner's
entitlement check first", 2026-11-24). This card makes the runner check a binary once: after it signs a binary, or
finds it signed, it writes a stamp beside it, and a later launch whose binary is not newer than its stamp skips
`codesign -d` and `exec`s at once. A relinked binary, newer than its stamp, is checked and signed as today.

- **The work:** `scripts/sign_and_run.sh`: the stamp (`$binary.signed`, a file), written under the existing lock
  after the binary is found or made signed; the check skipped when the stamp exists and the binary is not newer
  (`[ "$binary" -nt "$stamp" ]` false); the lock's protocol kept (Sprint 168: `codesign -f` on a file another process
  is signing or running failed 11 of 20 runs). WORKING_AGREEMENT's note on the runner amended with this card.
- **First, before any change (reviewer-4's Medium 1 at its fifth review):** the 26.27 s was one sample taken while
  the machine stalled (plain `ps` calls hung 60 to 120 s in the same hour; `uptime` 25 days). On a machine running no
  gates, measure five `codesign -d --entitlements -` on an already-signed test binary, five on a freshly linked one,
  and five `ps -A`, each by `/usr/bin/time -p`, and write the fifteen figures in the result. The rule, against the
  gates' own time (reviewer-4 on the draft): a gates run makes about 480 runner calls (nextest's one process a test,
  360 tests, and two listings each of 60 binaries; the card counts them from its own gates log), run on nextest's test
  threads (the card reads the count nextest uses); the change is built only if the quiet median `codesign -d` times
  480, divided by those threads, exceeds 5% of Sprint 330's 1,418 s gates run, 71 s. Read before the card: the
  gates pass no `--test-threads` (`scripts/gates.sh` 196, 202) and the repo has no `.config/nextest.toml`, so nextest
  uses its default, the core count, 18 here (`sysctl -n hw.ncpu`); the check runs before any lock
  (`scripts/sign_and_run.sh` 13-14), so the checks overlap across processes; the break-even is about 2.7 s of quiet
  median (71 s × 18 / 480). Context, not premise: Sprint 331's second gates run spent 2,694 s in its tests alone
  ("Summary [2694.344s] 360 tests run: 360 passed (263 slow), 1 skipped"), 1.9 times Sprint 330's whole 1,418 s run
  on the same boot and OS build, and about 40 minutes listing before them. Below it the premise is false
  (the Decision's own revisit clause): the card closes on the measurement alone, partial on the grounds of
  `AGENTS.md`'s partial-pass rule with no other card depending on the runner change, the runner left as it is; what
  stalls the machine during a gates run is the next card, with its own scope and observation contract (the stall
  reproduced, its cause named by measurement).
- **Not here:** why `codesign -d` takes 26 s on this machine, if it does; the card measures it and leaves it.

## prerequisites

- 331 (closed)

## context_files

- `scripts/sign_and_run.sh` (the whole file, 23 lines)
- `.cargo/config.toml` (the runner's line)
- `WORKING_AGREEMENT.md` lines 137-266, "Build and verification commands" (every line naming the runner or Sprint 168)
- The Decision "The test runner's entitlement check first": `grep -n "entitlement check first" BLACKBOARD.md`
- `tools/vm_monitor/entitlements.plist`

## signal contract

### Emits

- None; a build script.

### Invariants

- A binary the runner starts holds the Hypervisor entitlement (`codesign -d --entitlements -` names
  `com.apple.security.hypervisor`), whether it was signed before, just now, or relinked since.
- No Python runs.

## artifact contract

### Files modified

- `scripts/sign_and_run.sh`, `WORKING_AGREEMENT.md` (the runner's note)

### Command exit codes

- `scripts/gates.sh` returns 0

## observation contract

- **Expected values, measured before and after on one test binary already signed** (the same binary each way, the
  machine otherwise idle, ten launches of `scripts/sign_and_run.sh <binary> --list --format terse` each, wall time by
  `/usr/bin/time -p`): before, each launch pays the `codesign -d` check (26.27 s measured under the gates' load; the
  card measures it idle and writes the figure); after, each launch after the first under 1 s. And: a binary touched
  newer than its stamp is checked again (the stamp rewritten after the launch), and an unsigned copy of a test binary
  started through the runner comes out signed, `codesign -d` naming the entitlement.
- **The gates' own time:** the card's gates run's time before its first test, read from its log's timestamps, against
  Sprint 331's (about 40 minutes for the second run).
- **Planted faults:** (1) the stamp check taken out (`[ -f "$stamp" ]` made false): every launch pays `codesign -d`
  again, the ten launches' time back to the before figure; (2) the newer-than test taken out (the stamp trusted
  whatever the binary's age): a relinked, unsigned binary run through the runner is not signed, and `hv_vm_create`
  in it refuses (HV_DENIED), shown on a copy of a `hypervisor` test binary stripped of its signature with `codesign
  --remove-signature` and touched newer than a stale stamp. Each judge's reach read before the card's work: plant 1
  changes the one line every launch passes; plant 2's judge runs the runner on a binary whose stamp is older.

## done criteria

A test binary already signed starts through the runner in under a second, and a relinked binary is still signed.

## self-review checklist

- [x] Scope names one verifiable property: one signature check per binary build.
- [x] context_files cite the runner, its config, the working agreement's lines and the Decision.
- [x] Emits: none.
- [x] Artifact contract gradable by command.
- [x] Observation contract: times before and after, the relinked case, two plants with their reach read.
- [x] One concept: the runner's check of a binary's signature.

## result

- **The measurement first, no gates running** (2026-11-24 11:24-11:27 PST; `pgrep` found no gates, cargo, rustc or
  nextest process; load averages 2.15-2.38 on 18 cores), each figure `/usr/bin/time -p`'s `real`:
  five `codesign -d --entitlements -` on the already-signed `target/debug/deps/signal_report-e2b7d80e5e2d45f2`, the
  binary of the Decision's 26.27 s: 9.83, 9.85, 9.96, 9.87, 9.78 s, median 9.85; five on
  `target/debug/deps/no_python-0895bdd0b1f8eda8`, linked at 11:25 by `cargo test -p signal_report --test no_python
  --no-run` after its source's time was touched (content unchanged), holding the linker's signature and no
  entitlement: 9.70, 10.07, 10.12, 9.75, 10.49 s, median 10.07; five `ps -A`: 0.07 s each. The 26.27 s under the
  stalled machine was 2.7 times the quiet figure, and `ps` is not slow on a quiet one.
- **The rule:** 9.85 s × 480 calls / 18 threads = 263 s, against 5% of Sprint 330's 1,418 s, 71 s (break-even
  71 × 18 / 480 = 2.7 s). The premise holds, 3.7 times over the break-even, so the change is built.
- **The change** (`scripts/sign_and_run.sh`): `$binary.signed`, written under the existing lock once the binary is
  found or made signed, holds the binary's modification time to the nanosecond (`stat -f %Fm`); a launch whose binary
  has that time `exec`s without `codesign -d`. A launch without a matching stamp takes the lock, tests the stamp again
  (another process may have just written it), and checks and signs as before; the lock's protocol (Sprint 168) is
  unchanged, and the `codesign -d` that ran before the lock in every process now runs once, inside it.
- **A departure from the card's `-nt`, measured:** this Mac's `/bin/sh` is bash 3.2.57 (`/private/var/select/sh ->
  /bin/bash`), and its `[ a -nt b ]` compares whole seconds: of two files 0.92 s apart in the same second
  (1795548569.000000000 and 1795548569.922779000), the later read as not newer. Under the card's test, a binary
  relinked in the second its stamp was written would be trusted unsigned, against the invariant. The stamp's time is
  compared for equality instead, which also catches a binary replaced by an older file.
- **Before and after, one signed binary, ten launches each** (`scripts/sign_and_run.sh
  target/debug/deps/signal_report-e2b7d80e5e2d45f2 --list --format terse`, machine otherwise idle): before, 30.02,
  9.95, 10.41, 9.46, 9.80, 9.61, 9.83, 9.32, 9.58, 9.99 s; after, 29.74 s (no stamp yet: the check, the stamp
  written), then 0.01 and eight of 0.00 s. The listing's 13 lines byte for byte the same (`cmp`). The first launch of
  each series took about 30 s, three times a later one; what the other 20 s is was not measured.
- **The relinked case:** that binary `touch`ed (1795548675.924390000, stamp 1793438313.289035223): the next launch
  19.41 s, the stamp rewritten to 1795548675.924390000, the launch after 0.01 s. **The unsigned copy:**
  `no_python-0895bdd0b1f8eda8` copied to the scratchpad and stripped by `codesign --remove-signature` ("code object is
  not signed at all"); through the runner it listed its five tests, exit 0, and `codesign -d` then named
  `com.apple.security.hypervisor`, its stamp equal to its time after signing.
- **Plants,** the runner copied aside after the change, each plant one line written by the Edit tool, restored by
  `cp`, checked by `cmp` (silent, then "restored"):
- **Plant 1,** the stamp check made false (`stamped() { false && read -r at < "$stamp" && …`): ten launches of the
  signed `signal_report-e2b7d80e5e2d45f2`, its stamp matching its time (both 1795548675.924390000), took 9.86, 9.58,
  9.67, 9.76, 9.72, 9.61, 9.68, 9.66, 9.70, 9.73 s, the before figure every time. Restored: 0.22, 0.01, then eight
  of 0.00 s.
- **Plant 2,** the time test taken out (`stamped() { [ -f "$stamp" ]; }`, the stamp trusted whatever the binary's
  age), judged by `first_exit-6ff3ceeb3acb5b76` (`crates/hypervisor/tests/first_exit.rs`, `Vm::new` at line 26)
  copied to the scratchpad, with a stale stamp (1793438313.289035223, its file dated 09:00) and the copy touched
  newer. Control, the runner as changed: the copy signed, "test result: ok. 1 passed". As the card words it, the copy
  stripped by `codesign --remove-signature`: the planted runner left it unsigned and the kernel killed it at `exec`,
  runner exit 137, no output, so no `hv_vm_create` ran; a binary with no signature does not start on this Mac. A
  relinked binary instead carries the linker's ad hoc signature without the entitlement (the fresh `no_python` link
  above named none), so the judge ran again with the stripped copy signed `codesign -s - -f`, no entitlements: runner
  exit 101, "test an_hvc_exits_with_its_immediate_and_the_resumed_run_reaches_the_next ... FAILED", "panicked at
  crates/hypervisor/tests/first_exit.rs:26:26: called `Result::unwrap()` on an `Err` value: HvError { call:
  \"hv_vm_create\", code: -85377017 }", -85377017 being 0xFAE94007, which `crates/hypervisor/src` names HV_DENIED;
  `codesign -d` named no entitlement. Restored, the same judge: runner exit 0, "test result: ok. 1 passed".
- **`WORKING_AGREEMENT.md`:** the runner's note (Build and verification commands, `scripts/gates.sh`'s bullet)
  amended with Sprint 332: the check once a build, the stamp, the figures, and the whole-seconds `-nt`.
- **The gates:** `exit 0 in 5845 s	default	2026-11-24T21:13:20Z	sprint 332	574d356931154ce482d5f9c579f948bbfb0c8aa8	a09e0a8d6a6082ce4c4ebc99e42e80f18c4f936286ca9dd910e37b249bd9b75f	root=b41dd92f7eac4fa3c2087c83327cf28becd55710	rust=all python=0 samples=0	base=f9ebd009c058f1cef52cced5b991cb21e4a71d01`
  The selection "Rust tests: all (an input outside one crate changed)", the runner being one such input;
  "Starting 360 tests across 60 binaries (1 test skipped)", "Summary [1135.626s] 360 tests run: 360 passed (91 slow),
  1 skipped", "timing stage: passed, 781 ns an exit at load 1.64", "gates: all passed". Runner calls counted from the
  log: 360 tests and two listings each of 60 binaries, 480; nextest's threads the default, 18 (no `--test-threads`,
  no `.config/nextest.toml`).
- **The gates' time before the first test,** from the stamps the run wrote (no log line carries a time): the run
  started 19:35:55Z (`date -u` before it), the batch-run record is dated 19:36:52Z, and 59 test-binary stamps under
  `target/debug/deps/` were written between 19:41:13Z and 20:03:19Z, one a binary, so each binary's `codesign -d` ran
  once in the run (the 60th binary's stamp is not newer than the start; not read which binary that is). The first
  test started after 20:03:19Z: 27 minutes 24 s from the start, 22 minutes 6 s of it listing, against Sprint 331's
  third run's 35 minutes and its second's about 40. Listing still averages 22.5 s a binary with one check each
  (9.85 s quiet), so the check is no longer the whole of it; what the rest is was not measured, and it is the next
  card's (the stall). The tests took 1,135.6 s against 331's 2,420.9 s and 2,694.3 s. The run's whole 5,845 s exceeds
  331's 4,928 s: from the tests' end, about 20:22Z (20:03:19Z plus 1,135.6 s), to the timing stage's output file,
  21:12:28Z, about 50 minutes passed that the log does not place; the doc tests report 16.25 s and 16.48 s. Not
  measured, not attributed.
- **The supervisor's grade (2026-11-24):** the gated tree, `scripts/gated-tree.sh`, prints `b41dd92f…`, the pass row's
  `root=`; `scripts/sign_and_run.sh` compares equal by `cmp` to the builder's copy after the change, the plants
  restored. Ten launches of the stamped `signal_report-e2b7d80e5e2d45f2` through the runner, rerun by the supervisor:
  15.73 s, then 0.00 s nine times; the binary touched, 16.00 s with its stamp rewritten to the new time, then 0.00 s;
  `codesign -d` naming the entitlement after. The first launch of each series, 15-30 s with the stamp matching and no
  `codesign -d`, is the same unmeasured cost the builder saw, carried to the stall card. The departure from the card's
  `-nt` (equality on the nanosecond time) holds on the builder's measurement of bash 3.2's whole-second `-nt`.
