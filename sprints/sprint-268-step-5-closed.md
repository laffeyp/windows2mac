# Sprint 268 — Step 5 closed

```yaml
---
id: 268
status: active
phase: 6
pass_kind: functional
---
```

## scope

Plan step 5's gate (`docs/rust-port-plan-2026-10-24.md` 158-160) reads met at Sprint 266 on card 5's gate, the
Decision "The translation cache's shape" annotated so. Step 4 closed with a document holding its gate item by item and
its census rows not met carried to Deferred (Sprint 242, `docs/step-4-census-2026-10-28.md` "Step 4 closed"); step 5
has no such close. This card writes it, with reviewer-3's Lows 1 and 2 at 266: the gate's leaving installs out shown
fair by the time an install takes, and the frame's tie to the fastest rate stated as the conservative side.

- **Where it stands, by lookup (2026-10-30):**
  - The gate's items and their records: the batch run conforming with the cache (Sprints 257, 263, 266); "compiled in
    UnityPlayer.dll" 149,308 to none and "compiled" 274,837 to 125,515 (257); card 4's time, 3.0% faster ledger-free
    (262); card 5's compile time 678 to 694 ms against 1,351 to 1,362 (263); the most compiles in a frame of
    instructions 19,201 against 54,996 (266).
  - The Decision "Sequence for step 5" handed census rows 2, 11, 13 and 15c to "the step-5 card whose work reaches
    it"; Deferred (Sprint 242) carries each. Row 13's flag liveness was judged against the C compiler by the whole-block
    judge (252) and the cached blocks' successors' reads (254), not against Windows; row 15c moved to step 7 (Sprint
    256's annotation); rows 2 and 11 no step-5 card reached.
  - An install's time, by subtraction on Sprint 263's adjacent timed pairs: 0.63 to 0.70 µs, against 4.67 to 4.72 µs a
    compile, the cached run's compiles assumed at the uncached mean.
  - The design's window, 53,500,000 instructions, spans 16.7 ms at 3.21 billion instructions a second and 20.2 ms at
    2.646 billion; reviewer-3's Low 2 reads a slower run's frame as shorter, and it is longer, the conclusion the same.
- **The work:**
  - `docs/step-5-close-2026-10-30.md`: the gate item by item with its records and sprints; the install's time and the
    frame's work weighted by it (111 to 114 ms cached against 257 to 259 uncached, both peaks in one frame); each
    census row handed to step 5 with what reached it and what re-visits it.
  - The design's clause on the window's length (Low 2).
  - `crates/signal_report/tests/census.rs` checks the step-5 document's paths as it checks the census's (today it reads
    `CENSUS` alone, `docs/step-4-census-2026-10-28.md`).
  - Deferred (Sprint 242)'s rows 2, 11, 13 and 15c annotated with their step-5 reading and their re-visit; the Decision
    "Sequence for step 5" closed, pointing to the document.
- **Not here:** step 6's sequence (the next card).

## prerequisites

- 267 (closed)

## context_files

- `docs/step-4-census-2026-10-28.md` ("Step 4 closed"); `docs/design-translation-cache-2026-10-30.md` (card 5's gate)
- `BLACKBOARD.md`: Decisions "Sequence for step 5" and "The translation cache's shape"; Deferred (Sprint 242)
- `store/runs/w2m-vm-game-17934190{13,40,66,92}000000000`, `-1793419118000000000`, `-1793419145000000000`

## signal contract

### Emits

- None; a document of records already made.

### Invariants

- No record or product code changes; one test reads one more document.

## artifact contract

### Files modified

- `docs/step-5-close-2026-10-30.md` new, `docs/design-translation-cache-2026-10-30.md`,
  `crates/signal_report/tests/census.rs`, `BLACKBOARD.md`

### Command exit codes

- `scripts/gates.sh` returns 0; `design_check` passes

## observation contract

- **Expected values:** each gate figure in the document equal to its record's, read back from the record by command.
- **Planted faults:**
  - (1) a record path in the step-5 document misspelled: `census.rs`'s path check names it.

## done criteria

Step 5's close stands in one document, each gate item tied to its record, installs shown cheap, every census row it
was handed placed; the Decision closed.

## self-review checklist

- [x] Scope names one verifiable property: step 5's gate shown met item by item.
- [x] context_files cite the model close, the design, the Decisions and the records.
- [x] Emits: none.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and one plant.
- [x] One concept: step 5's close.

## result
