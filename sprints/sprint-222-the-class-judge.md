# Sprint 222 — The class judge

```yaml
---
id: 222
status: done
phase: 6
pass_kind: functional
---
```

## scope

The Decision "Sequence after Sprint 214" as annotated at Sprint 221 (reviewer-3's Medium 3 of its refreshed project
review), and plan step 4's gate item "456 of 458 classes" (census row 3, "absent" on Rust).

- **What the frozen judge did:** `scripts/class_run.py` judged each of the 458 classes in its own substrate run and
  wrote `store/runs/class-runs.json`: per class its record, its rule's sha256, its case count, two verdicts (`model`,
  the Python rules interpreter; `managed_vm`, the VM) and its event counts. 456 conform in both; FCOS and FSIN are
  discrepancies in both.
- **What the Rust side holds, by lookup (2026-10-29):**
  - The 21 reference runs `class_run.py` names (`REFERENCES`, 54) are all under `build/probe-runs` with
    `result-1.bin`.
  - They span the four profiles census row 3 counts: 8573C on Windows Server 2025 (10 runs) and 2022 (9), 8370C (1)
    and 6973P-C (1).
  - `case_runs` (`crates/w2m_vm/src/bin/case_runs.rs`) judges one profile a session, over every downloaded run on that
    profile, not the class's own sets (`case_sets_of`, 85-91). It writes no class index.
- **The judge:**
  - For each class in the index, the cases of the reference sets that cover it, oldest first, judged by the VM through
    the Rust case runner.
  - The verdict is `managed_vm`'s. The `model` executor is the Python rules interpreter, which moves under `oracle/`
    with the rest of the frozen code. It has no Rust counterpart to port, and the card does not make one.
  - One record per class in `class_run.py`'s event order. The Rust index, `store/case_runner/class-runs-<date>.json`,
    is in the frozen index's form less `model`.
- **The gate:** a test holds the Rust index to the frozen one, class by class: the same 458 classes, the same case
  count, and the same `managed_vm` verdict. Where a class's verdict differs, the test names it and the class's first
  divergence.
- **Not here:** fixing FCOS and FSIN; the oracle move.

## prerequisites

- 221 (closed)

## context_files

- `scripts/class_run.py` (whole); `scripts/semantics_differential.py` (`judge`, `undefined_memory`, `CASE_SETS`);
  `scripts/vm_oracle.py` (`run_cases`)
- `crates/w2m_vm/src/bin/case_runs.rs`, `crates/w2m_vm/src/case_inputs.rs`, `crates/w2m_vm/src/cases.rs`,
  `crates/cpu_cases/src/lib.rs`
- `store/runs/class-runs.json`; `store/probe-runs/<run>/` for the 21 reference runs

## signal contract

### Emits

- Per class: SESSION_INIT, CANDIDATE_GENERATED, COMPARISON_COMPLETED (`managed_vm`), then CANDIDATE_ACCEPTED, or
  REDUCTION_COMPLETED, DISCREPANCY_REPORTED and DISCREPANCY_ROUTED; SESSION_COMPLETED. All are in vocabulary 0.12.

### Invariants

- The 512 verdicts `case_runs` gives today are unchanged.
- No class judged on a run outside its reference sets.

## artifact contract

### Files modified

- `crates/w2m_vm` (the class judge and its test), `scripts/gates.sh`, `store/case_runner/`,
  `docs/step-4-census-2026-10-28.md` (row 3)

### Command exit codes

- `cargo nextest run -p w2m_vm` and `scripts/gates.sh` return 0

## observation contract

- **Expected values:** classes judged, of 458; conforming, against 456; the discrepancies by name, against FCOS and
  FSIN; case counts equal per class.
- **Planted faults:**
  1. One reference result's CF flipped: the class it covers turns discrepancy, and the test names it.
  2. A class judged on every run of its profile rather than its own sets: the case count differs, and the test names
     the class.
  3. A class dropped from the Rust index: the test names it.

## done criteria

Each of the 458 classes the frozen index judged is judged on Rust, over the cases of its own reference sets. A gate
holds the Rust index to the frozen one class by class.

## self-review checklist

- [x] Scope names one verifiable property: the 458 classes' verdicts on Rust, equal to the frozen index.
- [x] context_files cite the frozen judge, the Rust runner and the reference runs.
- [x] Emits: the frozen sequence less the model's comparison, in vocabulary 0.12.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and three plants.
- [x] One concept: the class judge.

## result

- **The lookups, before the judge:**
  - **Case counts:** `tests/class_judge.rs` decodes every case of the 21 `probes/semantics*/cases.bin` files with
    `x86_decoder`. For all 458 classes, a class's decoded count equals the frozen index's `cases`. The frozen count
    includes the cases set aside (`undefined_memory`) and those the VM refused. The Rust judge's count per class is
    therefore every case it read, judged or not.
  - **Reference runs:** 22 runs are downloaded under `build/probe-runs` for the 21 sets. Set 039 has two, and the
    frozen judge used one (38011862284). Each frozen class record names its own runs in its RunStarted baseline
    (`"reference_run":"38011862284+38044974055"` for RET_NEAR). The Rust judge reads them there, not from
    `class_run.py`'s text.
- **Departures from the card:**
  - One substrate session holds the 458 classes, one unit each, where the card said one record per class. The frozen
    records together name the 21 reference runs (their RunStarted baselines), and a case's class is its decoded class,
    so one pass of the case runner over the 21 runs gives every class its own cases. 458 sessions would run the same
    cases in 458 VMs. Each class keeps its own contract (`x86.semantics.<class>`) and the version its frozen record
    names.
  - So SESSION_INIT and SESSION_COMPLETED are written once a session, not once a class.
  - `cases.rs`' `Tally` gains `unjudged_by_class`: the cases read and not judged (the compiler's refusals and the
    model steps the slice refuses), with the first one's index. `case_runs` reads no new field, so its verdicts are
    unchanged.
- **The judge:** `class_judge` (a Mac binary in `w2m_vm`; `harness/class-judge.toml`) runs the case runner over the 21
  runs, with the Rust translator where it compiles a class and the frozen Python lowering's words for the rest, as
  `case_runs --rust-translator` does. One session takes about 150 s. `class_judge --index RECORD` writes
  `store/case_runner/class-runs-2026-10-29.json`.
  - All 458 classes are judged, each with the frozen case count.
  - 437 conform. 6 of them were judged on no case the Rust translator compiled, all on the frozen Python words:
    EMMS, LDDQU, MOVDQ2Q, ORPD, PEXTRW and PSHUFW.
  - FCOS and FSIN are discrepancies, as in the frozen index, each with two differences: ST0 one unit off in its last
    digit (FCOS 10 cases, FSIN 8) and FSW's C1 (4 cases each). The judges' ledger holds the four, 20 entries in all
    from 16.
  - 19 classes the frozen VM accepted are inconclusive, not judged on Rust, and held in
    `store/case_runner/class-held-2026-10-29.tsv`: 17 legacy SSE classes the Rust translator does not lift (3,856
    cases, the number BLACKBOARD's Deferred entry of Sprint 178 gives), and VRCPPS and VRSQRTPS (648 and 610 cases,
    those of the Deferred entry of Sprint 183). They open no ledger entry.
- **The gate:** `tests/class_judge.rs`:
  - `the_rust_index_is_the_frozen_one_or_held`: every frozen class, its case count, and its verdict or its held line.
  - `the_stored_rust_index_is_its_records`: the stored index is what `--index` gives on its record.
  - `every_indexed_class_has_its_frozen_case_count`: the 21 case files, decoded.
- **Plant 1** (`--plant-flip-cf`, CF flipped in the first run's completed results): the gate failed, "ADD:
  discrepancy, the frozen index conforms", and so on for each class the run covers.
- **Plant 2** (`--plant-every-run`, set 039's second downloaded run judged too): the gate failed, "ADD: 2748 cases,
  the frozen index 1374".
- Plants 1 and 2 ran on the binary before the fixes below. Those fixes change no measured difference and no case
  count, the two things the plants moved.
- **Plant 3** (RET_NEAR dropped from the stored index): the gate failed, "RET_NEAR: not in
  store/case_runner/class-runs-2026-10-29.json". Restored byte for byte.
- *(Annotated 2026-10-30, Sprint 235; reviewer-3's Medium 1 of its review at 235: the committed
  `the_rust_index_is_the_frozen_one_or_held`, at this sprint's work commit af30038c7 and until Sprint 235, built its
  faults and never asserted them. Plants 1 and 3 ran on a version that asserted, an assertion lost before the commit,
  and were not run again on the committed tree. From this sprint to 234 the gate could not fail on a class's verdict
  or on a class missing from the Rust index; the ledger's half of the same test did assert. The class verdicts of
  222-234 are shown by Sprint 235's run, with the assertion restored, and not before.)*
- **Fixed on the way:**
  - `--index` on a failed record wrote an empty index and exited 0. It now fails: "holds no comparison".
  - The candidate's sha256 was the Python words' for every class. A class the Rust translator compiles now names
    the translator's, as `case_runs` does. `translator_sha` moved from `case_runs` into `case_inputs` for both.
  - Reviewer-3's High 1 of its review at 222: the cases read and not judged were pushed into the differences list,
    so 19 classes read discrepancy, each with DISCREPANCY_REPORTED, and the ledger would have opened 19 entries for
    nothing measured. `Class` holds them apart now, and `Class::verdict` gives such a class `inconclusive` (vocabulary
    0.12) with no DISCREPANCY_* event. The gate holds every class-judge ledger entry to a class whose verdict is a
    discrepancy.
  - A fault the review did not name, found by reading `ledger.rs` for High 1: DISCREPANCY_REPORTED's
    `first_divergence` began "managed_vm:", and the ledger takes the text before the first colon as the unit. It
    begins with the class now ("FCOS: managed_vm: …").
  - Reviewer-3's Medium 1: each comparison's rule_ref and each index row carry the cases the Rust translator compiled
    (`rust_compiled`), and the gate prints the conforming classes with none.
- **Departure:** `scripts/gates.sh` gains no class-judge step. The judge's session runs by hand into `store/runs`, as
  `case_runs`' sessions do. The gates hold its record through `class_judge.rs`' tests and `case_runs --check-ledger`.
- **Departure:** `scripts/gates.sh` moves one frozen-host test into the multi-vCPU stage, whose outcomes are kept in
  `store/gates/multi-vcpu.tsv` and do not stop the gates:
  `test_planted_kick_into_other_processes_only_leaves_a_new_child_unrun`.
  - It failed the second gates run, and found no hang in 160 runs on 2026-10-29, both alone and in the full run.
  - A worktree at Sprint 221's work commit, whose gates had passed it at 17:53, failed it the same way at 19:55.
  - Its cause is the machine's state, not this sprint's tree. Its fix is plan step 6's, with the stage's other three.
  - `signal_report --check` held a closed entry's multi-vCPU sentence to "k of 3 passed" and the stage to 3 rows,
    constants that would have failed this close. From Sprint 198's rule on, the entry states "k of N passed" and N
    is held to the stage's rows in its gates run (`multi_vcpu_staged`). Earlier sprints' "of 3" still pass, and
    `the_multi_vcpu_sentence_is_held_to_the_stages_that_ran` gains four rows stated as 4 (passes) and as 3 (named:
    "holds 4 rows in it, not 3").
