# Sprint 237 — The approximations, and the Python words fixture retired

```yaml
---
id: 237
status: done
phase: 6
pass_kind: functional
---
```

## scope

Plan step 4's "456 of 458 classes", BLACKBOARD Deferred "(Sprint 183)", and reviewer-3's direction at 235: the last
two held classes, VRCPPS and VRSQRTPS, and then the fixture no class needs.

- **Where it stands, by lookup (2026-10-30):**
  - The Rust model refuses RCPPS, RSQRTPS, RCPSS and RSQRTSS by name (`crates/w2m_vm/src/model.rs`, "its model comes
    with a later slice"). Their VEX forms are what the class judge holds inconclusive: VRCPPS 648 cases and
    VRSQRTPS 610 in set 049b.
  - The frozen model ran them from `approximate.py` (70 lines at the tag frozen-python-2026-10-29;
    `runtime.py` 859-864): one CPU's tables in `specs/approximations.json` (148,510 bytes, in the tree), each normal
    input's result the entry for its mantissa's top 12 bits with the exponent moved; zeros, denormals, infinities,
    NaNs and negative RSQRT inputs by rule.
  - Since Sprint 236 every case code is compiled by the Rust translator; the Python words fixture serves only the
    words test (`case_runner.rs` `the_rust_words_equal_the_python_words_on_every_case_code`) and the hybrid's
    comparison.
- **The work:**
  - `approximate.py` ported into the model, reading the same table, and the four classes and their VEX forms served
    by it as `runtime.py` serves them.
  - The class judge run again; VRCPPS and VRSQRTPS judged on their cases.
  - The fixture retired: the hybrid compiles by the Rust translator alone, the words test and the fixture's loader,
    pin, release note and `scripts/fetch-python-words.sh` go, and WORKING_AGREEMENT says so. Its words stay at the
    release for reading.
- **Not here:** FCOS and FSIN, discrepancies in the frozen index too.

## prerequisites

- 236 (closed)

## context_files

- `src/w2m/approximate.py`, `src/w2m/runtime.py` 859-864 at the tag; `specs/approximations.json`
- `crates/w2m_vm/src/model.rs`, `crates/w2m_vm/src/case_inputs.rs`, `crates/w2m_vm/tests/case_runner.rs`
- `store/case_runner/`, `scripts/fetch-python-words.sh`, `WORKING_AGREEMENT.md`

## signal contract

### Emits

- The class judge's session, as in Sprints 235 and 236.

### Invariants

- Every class that conforms still conforms; FCOS and FSIN stay as in the frozen index.

## artifact contract

### Files modified

- `model.rs`, `case_inputs.rs`, `case_runner.rs`, the class judge's index and record, the held class file, census
  row 3, `WORKING_AGREEMENT.md`; `scripts/fetch-python-words.sh` and `store/case_runner/python-words.sha256` removed

### Command exit codes

- `scripts/gates.sh` returns 0; the class judge's session completes and `check_records` passes it

## observation contract

- **Expected values:** VRCPPS and VRSQRTPS judged, against 0; classes conforming of 458, against 454, and 456 if both
  conform.
- **Planted faults:**
  1. One table entry changed: the case runner names a VRCPPS or VRSQRTPS case.
  2. A denormal input given zero for infinity: the model's unit test names it.
  3. The held class file keeping VRCPPS: the class test names it.

## done criteria

Every class but FCOS and FSIN is judged on Rust-compiled words against Windows, and no part of the case runner reads
the Python words fixture.

## self-review checklist

- [x] Scope names one verifiable property: the last held classes judged, and the fixture unread.
- [x] context_files cite the frozen tables and code, the model, the hybrid and the fixture's files.
- [x] Emits: the class judge's session.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and three plants.
- [x] One concept: the case runner on Rust alone.

## result

- **The approximations:** `crates/w2m_vm/src/approximate.rs` is `approximate.py`'s `rcp`, `rsqrt` and `_special`,
  reading the three 4,096-entry tables of `specs/approximations.json`, compiled in, once. Python's floor `%` and `//`
  are `rem_euclid` and `div_euclid`. The model serves RCPPS, RSQRTPS, RCPSS and RSQRTSS from them as `runtime.py`
  859-864 does, and VRCPPS and VRSQRTPS reach them through their base classes.
  - Its unit test holds every special input the reference CPU was measured on (`specials`) to the CPU's RCP and
    RSQRT.
- **The cases:** set 049b is judged whole, 46,970 cases, where 1,258 went to the model unjudged; VRCPPS's 675 and
  VRSQRTPS's 639 agree with Windows. `expected.tsv`'s row by the Edit tool, the per-class counts by the writer.
- **The session:** record `store/runs/class-judge-1793359177000000000`, `check_records` 0 violations, 456
  CANDIDATE_ACCEPTED. The index was written from it: 456 of 458 conform, every one on Rust-compiled words, none held;
  FCOS and FSIN discrepancies as in the frozen index. **Step 4's class gate, "456 of 458", is met.** The held class
  file keeps its header alone, and the ledger was rebuilt, 4 defects.
- **A plant during a session:** plants 1 and 2 ran while the session ran. `class_judge` compiles the table in, and its
  start check covers `crates/` only, so a plant to `specs/` could have reached a session still building. The binary
  was built at 04:19:41, before plant 1, and the session accepted RCPPS, which plant 1 failed. So the session judged
  the true table.
- **Departure, the scope split:** the fixture's retirement is not this card's. The fixture supplies the block base,
  the exit, and the symbol and layout checks, and its sha256 is in every class-judge and case-runs record's
  baseline. Retiring it changes three binaries, their tests and their records' meaning: card 238.
- **Departures, files:** `crates/w2m_vm/src/lib.rs`, the expected counts, the ledger.
- **Plant 1** (the first RCP entry changed, 1065349120 to 1065349121, in `specs/approximations.json`): set 049b showed 29 difference classes, "RCPPS xmm5: case 24: [2e, 0f, 53, e8] vm 3d91c0002ffff001b65fb800affff001, cpu 3d91c0002ffff000b65fb800affff000".
- **Plant 2** (a zero or denormal input given a signed zero): "RCP of 00000000: 0x00000000, the CPU 0x7f800000".
- **Plant 3** (VRCPPS put back in the held file, by the Edit tool): "VRCPPS: conforms as the frozen index, and store/case_runner/class-held-2026-10-29.tsv holds it".
- Each plant was restored by `cp`, and `cmp` found the files equal to their copies.
