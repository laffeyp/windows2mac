# Sprint 245 — One verdict rule

```yaml
---
id: 245
status: done
phase: 6
pass_kind: functional
---
```

## scope

BLACKBOARD Deferred 2026-10-29 (Sprint 222; reviewer-3's Low 2 of its review at 222), its re-visit missed at Sprints 226,
238 and 241 and found by card 244: two judges turn a case runner's tally into a class's verdict, each by its own rule,
and they can disagree on one class.

- **Where it stands, by lookup (2026-10-30):**
  - `class_judge.rs` 143-151, `Class::verdict`: a difference makes a discrepancy; else a case read and not judged
    (`unjudged`) makes the class inconclusive; else it conforms. An inconclusive class gets COMPARISON_COMPLETED and
    no acceptance or report (365-368).
  - `case_runs.rs` 287-291: a difference makes a discrepancy, else it conforms; its units (`Unit`, judged and
    differences) take no count of a class's cases not judged, so refused cases count nowhere and a class with some
    cases refused is accepted.
  - `cases.rs` `Tally::unjudged_by_class` (127, 139-145) holds the cases each class sent to no verdict; `case_runs`
    does not read it.
  - Deferred searched for `cases.rs`, `case_runs.rs` and `class_judge.rs` (KIT_DIARY #102): this entry; Sprint 235's
    and 237's, resolved; Sprint 173's names `case_runs` as a load beside a frozen test, not this code.
- **The readers:** the two binaries; the records they write (the class judge's sessions, `case_runs`' sessions and the
  gates' planted session); no other program.
- **The work:**
  - `cases::class_verdict(differences, unjudged) -> events::Verdict`, the rule both judges call, its unit test.
  - `class_judge`'s `Class::verdict` calls it.
  - `case_runs`' `Unit` takes each class's unjudged count from `unjudged_by_class`; its verdict is `class_verdict`'s;
    an inconclusive unit gets COMPARISON_COMPLETED with the first unjudged line and no acceptance, report or
    resolution, as `class_judge` writes one, and the session's summary counts it.
- **Not here:** any record's rewriting: today every case is compiled (Sprint 238), so no unit is inconclusive and no
  record's verdict moves; the gates' planted `case_runs` session shows the form.

## prerequisites

- 244 (closed)

## context_files

- `crates/w2m_vm/src/cases.rs`, `src/bin/case_runs.rs`, `src/bin/class_judge.rs`; BLACKBOARD Deferred (Sprint 222)

## signal contract

### Emits

- None new: the two judges' sequences as before; an inconclusive `case_runs` unit, when one comes, writes the class
  judge's form for it.

### Invariants

- The class gate's index and the case runner's counts unchanged; the gates' planted `case_runs` record passes
  `check_records` as before.

## artifact contract

### Files modified

- `crates/w2m_vm/src/cases.rs`, `src/bin/case_runs.rs`, `src/bin/class_judge.rs`, `BLACKBOARD.md` (Deferred)

### Command exit codes

- `scripts/gates.sh` returns 0

## observation contract

- **Expected values:** `class_verdict(0, 0)` conforms, `(0, n)` inconclusive, `(n, m)` a discrepancy; the gates'
  case runner and class gate unchanged.
- **Planted faults:**
  1. `class_verdict` conforming with cases unjudged: its unit test names it.
  2. `class_verdict` ignoring the differences: its unit test names it.
  3. `class_verdict` putting inconclusive before a discrepancy: its unit test names it.

## done criteria

One function decides a class's verdict from its tally, and both judges call it.

## self-review checklist

- [x] Scope names one verifiable property: one verdict rule, both judges.
- [x] context_files cite the three files and the Deferred entry.
- [x] Emits: none new, stated with its reason.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and three plants.
- [x] One concept: the verdict rule in one place.

## result

- **The rule:** `cases::class_verdict(differences, unjudged) -> events::Verdict`, beside `Tally`, with
  `a_class_is_a_discrepancy_then_inconclusive_then_conforming` holding its four cases.
- **The judges:** `class_judge`'s `Class::verdict` is one call to it. `case_runs`' `Unit` keeps `unjudged`, filled from
  `t.unjudged_by_class` with `class_judge`'s line ("{class}: {n} cases not judged; run {run}: case {first}"); its
  verdict is the rule's; an inconclusive unit gets COMPARISON_COMPLETED, its first divergence the first unjudged line,
  and no acceptance, report or resolution; the summary line counts inconclusive units. Its module comment says so.
- **Records:** none rewritten. Every case is compiled since Sprint 238, so no unit has unjudged cases and no verdict
  moves; the gates' planted `case_runs` session, all classes with differences or accepted, runs through the new code.
- **A helper written and taken out:** a `Tally::unjudged_of` was added, then removed before any build, since both
  judges read `unjudged_by_class` directly.
- **Deferred (Sprint 222):** resolved.
- **The gates:** `scripts/gates.sh` exit 0 in 474 s on e033e5774, "rust=crates python=0 samples=0", 109 tests, the
  case runner, the class gate and the planted `case_runs` session among them.
- **Plant 1** (`class_verdict` conforming with cases unjudged, `&& false` on its second branch): "a case refused leaves
  nothing compared", "left: Conforms right: Inconclusive".
- **Plant 2** (`class_verdict` ignoring the differences, `&& false` on its first branch): "left: Conforms right:
  Discrepancy".
- **Plant 3** (inconclusive before a discrepancy, `&& unjudged == 0` on its first branch): "a difference outweighs a
  refusal", "left: Inconclusive right: Discrepancy".
- Each plant's `git diff --stat` showed its one line first; each was restored by `cp`, and `cmp` found the file equal
  to its copy.
