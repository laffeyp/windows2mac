# Sprint 154 — A record kept, a judge on CI, and the ledger checked

```yaml
---
id: 154
status: active
phase: 6
pass_kind: functional
---
```

## scope

reviewer-2's review of Sprints 151-152 (2026-10-25): two medium findings, three conditions on the Rust ledger, and
lows.

- **Medium 1, a record deleted.** Sprint 152 removed Sprint 150's committed record
  (`store/runs/case-runner-1792916257296645000`) and its 22 verdict traces (`store/case_runner/traces`) with `git rm`;
  the kit forbids deleting files, and the record is the audit trail of the error the review found. They are restored
  from c1c8ee44f and moved with `git mv` to `store/superseded/` (outside the readers: `scripts/check_replay.py`, the two
  ledgers, the guest trace store), with a dated note saying its TRACE_INGESTED events name a validation that never
  happened and the Sprint 152 sessions supersede it.
- **Medium 2, a judge that skips on CI.** `crates/translator/tests/c_words.rs` finds clang only through `xcrun`, so on
  the Linux runner it prints "clang not found" and passes. It needs no VM and no untracked input (the C is portable,
  `store/stencil/record.txt.gz` is tracked), so it runs on CI: the compiler is `xcrun clang` on macOS, else `cc`, the
  library a `.dylib` or a `.so`, and a missing compiler fails.
- **The ledger's conditions.** (1) A gate: `case_runs --check-ledger` rebuilds the judges' ledger and fails when the
  file differs; `scripts/gates.sh` runs it. (2) WORKING_AGREEMENT says `store/runs/discrepancy-ledger.json` covers
  `differential` records only and is frozen with the Python. (3) The plan notes that at step 4, when the Python moves
  under `oracle/`, one Rust ledger reads both keys.
- **Lows.** Sprint 151's card becomes `closed-partial`, with a Deferred entry for its signal contract and that entry's
  resolution (Sprints 152 and 153 made the records). The 0.11 proposal list gains COMPARISON_COMPLETED's
  `reference_ref` and `target_execution_id`, which name no execution event on these records, beside T4's scope for
  judge-only sessions.
- **Group 2's counts.** The case runner's tally counts DIV and IDIV cases judged at a fault (Rust-compiled words on the
  fault path) and the cases each class sends to the model, reported per class.

## prerequisites

- 153 (closed)

## context_files

- reviewer-2's review of Sprints 151-152 (2026-10-25); `sdd-kit-3/AGENTS.md` 117 (partial pass)
- `crates/translator/tests/c_words.rs`, `crates/w2m_vm/src/bin/case_runs.rs`, `crates/w2m_vm/src/ledger.rs`,
  `crates/w2m_vm/src/cases.rs`, `scripts/gates.sh`, `scripts/check_replay.py` (read: which directories it walks)
- `docs/rust-port-plan-2026-10-24.md`

## signal contract

### Emits

None: this sprint changes no producer's events; its judges' records stand as Sprint 153 made them.

### Consumes

The stencil record (on CI too).

### Invariants

- No Python file added or changed.

## artifact contract

### Files created

- `store/superseded/` (the restored record and traces, by `git mv`), its note

### Files modified

- `crates/translator/tests/c_words.rs`, `crates/w2m_vm/` (`bin/case_runs.rs`, `cases.rs`, `tests/case_runner.rs`),
  `scripts/gates.sh`, `WORKING_AGREEMENT.md`, `docs/rust-port-plan-2026-10-24.md`, `BLACKBOARD.md`,
  `sprints/sprint-151-the-lifter-and-the-integer-core.md`

### Command exit codes

- `scripts/gates.sh` returns 0; the CI run of the closing commit runs `c_words` and concludes success

## observation contract

- **Expected values:** CI's log shows `c_words` comparing 1,096,154 calls; `case_runs --check-ledger` passes on the
  committed `{}`; the restored record replays at level 2 from its new place.
- **Planted faults:** a ledger file with one entry added fails `--check-ledger`; `c_words` with the compiler's name
  made wrong fails rather than skips.

## done criteria

No committed record is lost, the translator's cheapest judge runs on every push, and the judges' ledger cannot go stale
unnoticed.

## self-review checklist

- [x] Scope names verifiable properties, each from a review finding.
- [x] Emits: none, said so, with the reason.
- [x] Artifact contract gradable by command.
- [x] Observation contract: expected values and planted faults.
- [x] One concept: the review's findings.

## result
