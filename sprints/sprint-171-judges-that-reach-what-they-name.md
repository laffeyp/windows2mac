# Sprint 171 — Judges and plants that reach what they name

```yaml
---
id: 171
status: done
phase: 6
pass_kind: functional
---
```

## scope

Decision 2026-10-28 (sequence after Sprint 170), from reviewer-2's review of Sprints 165-170: KIT_DIARY's H5 and its
rule (738-740: "a second recurrence of a recorded class makes the check the next card"). The class: a judge or a plant
that cannot reach what it names. Each item is one instance of it, with a plant that must fail:

1. **`cond_flags`'s masks by value** (High 1): `crates/translator/src/lower.rs`'s test asserts each of the eight masks
   equal to `src/w2m/vm_backend.py` 852-853 (`o`, `b`, `z`, `s`, `p`, `be`, `l`, `le`), cited by line, where it held only
   `is_ok()`. Plant: `"l" => U_SF`; the test names `l`.
2. **The look-ahead refuses as the oracle does** (Low 2): `crates/w2m_vm/src/compile.rs` 190-192 counts every flag live
   on an unknown condition; the Python's look-ahead calls `_insn_flags` at vm_backend 4812 with no `try`, so its
   KeyError leaves the look-ahead, then `_compile` (4588; no `except` at 4617) and `serve_exit` (5190; `_compile` at
   5242, no `try`): the frozen Python ends the run with an uncaught KeyError, not a refused block (read 2026-10-28;
   reviewer-2's Low 2 assumed a refusal). The Rust look-ahead returns the error and the run stops at a named stop that
   names the condition, the oracle's outcome in the Rust run loop's form. Plant: a unit test feeding an
   instruction whose condition the table lacks; the refusal names it.
3. **The gates' Rust-test trigger** (Medium 1): `scripts/gates.sh` 56 adds `.cargo/`, `rust-toolchain.toml`,
   `scripts/sign_and_run.sh` and `corpus/` (read at `crates/pe_image/tests/llvm_objdump.rs` 266). Plant: a comment
   changed in `sign_and_run.sh`; the default run prints "Rust tests: all".
4. **`build/` inputs** (Medium 2): the gates hash `build/probe-runs/*/result-1.bin` and `build/case_runner/*` and
   compare them with committed manifests under `store/case_runner/`; a mismatch selects all Rust tests and names the
   file. Plant: a byte appended to a copy of one result, pointed at by the manifest check's input; the gates name it.
5. **Sprint 168's plant 2, placed** (Medium 3): an assertion made false in `a64_encoder`'s tests with only
   `design_check` changed; the default run's "Rust tests: rdeps(design_check)" and exit 0, and `--all`'s non-zero
   exit, each recorded.
6. **A string `guest_trace` refused by substrate** (Medium 4): `vocabulary_samples --plant-string-guest-trace` writes
   an EXECUTION_COMPLETED past the Rust type, as `--plant-unknown-metric` does (main.rs 35-52);
   `harness/vocabulary-samples-planted-guest-trace.toml`; a gates step greps its ProducerFailed event for
   `guest_trace`.
7. **The discrepancy chain in the gates** (Low 1): when `w2m_vm` is selected, a gates step runs `case_runs
   --plant-flip-cf` into a temporary root and runs `check_records` over it, requiring 32 DISCREPANCY_REPORTED and no S5
   or E1 violation.
8. **Named plants have their failure on the record** (Medium 4): `signal_report --check` fails a closed sprint from 171
   on whose card's observation contract names a planted fault and whose result section has no line quoting a judge's
   failing output for it.

## prerequisites

- 170 (closed)

## context_files

- `crates/translator/src/lower.rs` 575-640, 2700-2730; `src/w2m/vm_backend.py` 852-853, 4800-4820
- `crates/w2m_vm/src/compile.rs` 180-205; `scripts/gates.sh`; `crates/vocabulary_samples/src/main.rs`;
  `crates/w2m_vm/src/bin/case_runs.rs`; `crates/signal_report/src/main.rs`

## signal contract

### Emits

- The planted guest-trace run (a temporary root): SESSION_INIT, then ProducerFailed naming `guest_trace`.
- The planted discrepancy run (a temporary root): as Sprint 167's planted record.

### Invariants

- No Python file added or changed; every judged code word unchanged.

## artifact contract

### Files created

- `harness/vocabulary-samples-planted-guest-trace.toml`, the `store/case_runner/` manifests

### Files modified

- `crates/translator/src/lower.rs`, `crates/w2m_vm/src/compile.rs`, `scripts/gates.sh`,
  `crates/vocabulary_samples/src/main.rs`, `crates/signal_report/src/main.rs`, `WORKING_AGREEMENT.md`

### Command exit codes

- `scripts/gates.sh` and `scripts/gates.sh --all` return 0

## observation contract

- **Expected values:** each of the eight items' plant fails at the judge it names, quoted in the result.
- **Planted faults:** as listed with each item.

## done criteria

Every judge and plant reviewer-2 named reaches what it names, and the report check holds the next card to it.

## self-review checklist

- [x] Scope names one verifiable property: each named plant fails at its named judge.
- [x] context_files cite each file.
- [x] Emits: the two planted runs.
- [x] Artifact contract gradable by command.
- [x] Observation contract: a plant per item.
- [x] One concept: judges that reach what they name.

## result

1. **`cond_flags`'s masks by value:** the test holds each of `lift::cc`'s 16 conditions to `COND_FLAGS`'s mask
   (vm_backend 852-853), a negation as its condition. Plant `"l" => U_SF`: the test failed, "assertion `left == right`
   failed: l, left: Some(4), right: Some(12)". Restored.
2. **The look-ahead:** read first, the frozen Python's KeyError leaves `_flags_live_in` (4812), `_compile` (4588,
   no `except` at 4617) and `serve_exit` (5242) uncaught and ends the run; it does not refuse the block, as the review
   assumed. The Rust look-ahead now returns the error, and the run stops at `Stop::Untranslated` (`run.rs` 578) naming
   it. A test builds an instruction branching on "xx" and `insn_flags` refuses it by name. Not reached: no lifted
   instruction carries an unknown condition, so the look-ahead's `?` is judged by reading, not by a run.
3. **The Rust-test trigger:** `.cargo/`, `rust-toolchain.toml`, `scripts/sign_and_run.sh` and `corpus/` select every
   Rust test. Plant (a change to the runner, by `GATES_CHANGED`): the plan printed "Rust tests: all (an input outside
   one crate changed)", where `design_check` alone gives "rdeps(design_check)".
4. **`build/` inputs:** `store/case_runner/build-inputs.sha256` holds 147 files. Plant (one hash changed): "a build/
   input differs from store/case_runner/build-inputs.sha256: build/case_runner/python-words.txt: FAILED"; plant (a new
   download): "build/probe-runs/plant-171/result-1.bin new". Each selected every Rust test; removed, the selection is
   `rdeps(design_check)` again.
5. **Sprint 168's plant 2, placed:** `a64_encoder`'s CNTVCT test made false, with only `design_check` changed: the
   default run ran 14 tests and exited 0; `--all` exited 101, "tests::the_virtual_counter_reads_as_clang_assembles_it
   --- FAILED". The first default run exited 1 with its 14 tests passed: since Sprint 168, `docs=$(for ...)` returned
   the last `[ -f ]`'s 1 under `set -e` whenever no changed crate has a library; the loop now ends with `true`.
6. **A string `guest_trace`:** substrate refused the planted line, "EXECUTION_COMPLETED: Expected `bool | null`, got
   `str` - at `$.guest_trace`", after the Rust type refused it; the gates read that from ProducerFailed.
7. **The discrepancy chain:** the gates run `harness/case-runs-planted.toml` whenever Rust tests run: 32
   DISCREPANCY_REPORTED, 0 violations. Plant (the report's initial-state hash set to the zero placeholder):
   `check_records` held 32, "DISCREPANCY_REPORTED.initial_state_sha256 is the zero placeholder", exit 1. Restored.
8. **Named plants quoted:** `signal_report --check` fails a sprint from 171 marked done whose contract names a planted
   fault and whose result quotes no judge's failure for one. Plant (this card marked done before this result existed):
   "sprint 171: its observation contract names a planted fault and its result quotes no judge's failure for a plant",
   exit 1. Restored.
- **`scripts/gates.sh --plan`** prints the selection and stops; the selection is decided before any gate runs.
