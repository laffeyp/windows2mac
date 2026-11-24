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
