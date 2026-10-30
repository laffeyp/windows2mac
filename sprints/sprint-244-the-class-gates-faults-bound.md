# Sprint 244 — The class gate's faults bound

```yaml
---
id: 244
status: active
phase: 6
pass_kind: functional
---
```

## scope

BLACKBOARD Deferred 2026-10-30 (Sprint 235), the half card 243 left, and reviewer-3's APOSD point at 235: the class
gate's test gathers its faults over 25 lines and asserts once at the end, so a lost assertion, as from Sprint 222 to
235, leaves a judge that cannot fail; clippy's `collection_is_never_read` binds it meanwhile.

- **Where it stands, by lookup (2026-10-30):**
  - `crates/w2m_vm/tests/class_judge.rs` 86-167, `the_rust_index_is_the_frozen_one_or_held`: the faults gathered
    100-127 from the frozen index, the Rust index and the held file, asserted at 166; between them the ledger check
    (128-145) and a printed count (146-163), whose comment still names "the frozen Python lowering's" words, which
    Sprint 238 retired.
  - Deferred, searched for the file this card changes (KIT_DIARY #102): the entry of Sprint 235, which this card
    answers; and the entry of Sprint 222, two judges with two tally-to-verdict rules, whose re-visit ("when `case_runs`
    is next touched, or at the oracle move") came due unread at Sprint 226's removal and Sprints 238 and 241, which
    touched `case_runs`. That rule moves into `cases.rs`, a change to both judges: its own card, ordered next.
- **The readers:** the test alone; no program calls it.
- **The work:**
  - `class_faults(frozen, rust, held) -> Result<(), Vec<String>>`: the comparison as one function that returns its
    faults, the test asserting `Ok(())`.
  - A unit test over a small frozen and Rust index and a held file: a moved case count, a verdict unlike the frozen
    one, a held class that conforms and a class the frozen index lacks are each named; equal indexes are `Ok`.
  - The stale comment corrected.
  - Deferred's entry of Sprint 235 resolved; the entry of Sprint 222 annotated with its missed re-visit; the Decision
    "Sequence for step 5" annotated: 245 the verdict rule in one place, then the compile counts.
- **Not here:** the verdict rule's move (card 245).

## prerequisites

- 243 (closed)

## context_files

- `crates/w2m_vm/tests/class_judge.rs`; BLACKBOARD Deferred (Sprints 222, 235)

## signal contract

### Emits

- None: a test's shape, no run.

### Invariants

- The class gate passes on today's indexes, as before.

## artifact contract

### Files modified

- `crates/w2m_vm/tests/class_judge.rs`, `BLACKBOARD.md` (Deferred, the Decision)

### Command exit codes

- `scripts/gates.sh` returns 0

## observation contract

- **Expected values:** `class_faults` over today's indexes `Ok(())`; the unit test's four faults named.
- **Planted faults:**
  1. `class_faults` returning `Ok(())` whatever it found: the unit test names the faults it expected.
  2. The case-count comparison removed: the unit test names the moved count.
  3. A class in the held file that conforms: the class gate names it.

## done criteria

The class gate's comparison is a function that returns its faults, and a test shows each kind of fault named.

## self-review checklist

- [x] Scope names one verifiable property: the faults bound where they are found.
- [x] context_files cite the test and the Deferred entries the file's search found.
- [x] Emits: none, stated with its reason.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and three plants.
- [x] One concept: the class gate's faults bound.

## result
