# Sprint 244 — The class gate's faults bound

```yaml
---
id: 244
status: done
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

- **The helper:** `class_faults(frozen, rust, held) -> Result<(), Vec<String>>` is the comparison that stood in the
  test's body, unchanged rule for rule; `held` reads the held file; `the_rust_index_is_the_frozen_one_or_held` panics
  with the faults when it is `Err`, then checks the ledger and prints the count as before.
- **The test:** `each_kind_of_class_fault_is_named` passes equal indexes, then names four faults in order: "ADD: 11
  cases, the frozen index 10", "FCOS: conforms, the frozen index discrepancy", "MUL: conforms as the frozen index, and
  store/case_runner/class-held-2026-10-29.tsv holds it", "NEW: not in the frozen index".
- **The comment:** the printed list is named `uncompiled`, its comment no longer naming the Python words Sprint 238
  retired.
- **Deferred:** the entry of Sprint 235 resolved. The entry of Sprint 222 annotated: its re-visit, due at Sprint 226
  and Sprints 238 and 241, was missed each time and found by this card's search of Deferred for its file; card 245
  takes it, and the Decision "Sequence for step 5" is annotated with the new order, the compile counts at 246.
- **The gates:** `scripts/gates.sh` exit 0 in 474 s on b5c915fe7, the class tests among them.
- **Plant 1** (`class_faults` returning `Ok(())` whatever it found, line 121): `each_kind_of_class_fault_is_named`
  panicked at its `unwrap_err`, line 201, the faults it expected not returned.
- **Plant 2** (the case-count comparison off, line 99): "left: [\"FCOS: conforms, …\", \"MUL: …\", \"NEW: …\"] right:
  [\"ADD: 11 cases, the frozen index 10\", …]".
- **Plant 3** (the held file given "ADD inconclusive a plant" by the Edit tool): the class gate, "1 classes differ: ADD:
  conforms as the frozen index, and store/case_runner/class-held-2026-10-29.tsv holds it".
- Each plant's `git diff --stat` showed its one line first; each was restored by `cp`, and `cmp` found the file equal
  to its copy.
