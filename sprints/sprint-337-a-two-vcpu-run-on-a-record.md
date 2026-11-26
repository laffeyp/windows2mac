# Sprint 337 — A two-vCPU run on a substrate record

```yaml
---
id: 337
status: active
phase: 6
pass_kind: functional
---
```

## scope

Plan step 6's evidence is meant to be "on a record" (`docs/rust-port-plan-2026-10-24.md` 58; reviewer-4's first review
at 331, High 1). Today the two-vCPU runs are judged only by what tests read from `w2m-vm`'s stderr and stop files. One
example is `a_locked_model_step_loses_no_add_of_another_vcpus_aligned_lock` in `crates/w2m_vm/tests/run_loop.rs`
(1201-1223), which reads "w2m-vm: 100000 model steps of a locked form". No substrate record carries a run's vCPU count,
its locked steps, its turn's start, or the capabilities `vcpus_allowed` (`run.rs` 605-633) checks. Vocabulary 0.13
(Sprint 336) gave those their words. This card puts them on a record, as the Decision "Step 6 restored to the user's fix
(b)" orders: "`w2m-vm --vcpus N` on a substrate record, CAPABILITY_PROBED emitted where `w2m-vm` makes its refusal
checks".

- **The work:**
  - **The two-vCPU measurements.** `run_measurements` (`crates/w2m_vm/src/bin/run_measurements.rs`, the Producer of
    `harness/w2m-vm-measurements.toml`) gains a two-vCPU part, run as its other measurements are, as a signed `w2m-vm`
    child. It runs `probes/locked_race`, the binary that `store/probe-runs/41326239822/probe.sha256` names under
    `store/binaries/`, with `--vcpus 2`, and records three things:
    - `locked_model_steps` (unit `count`);
    - `turn_start_ns_p50` and `turn_start_ns_p99` (unit `ns`, nearest rank over the run's `Outcome.turn_start_ns`);
    - each with `vcpus` 2.

    `w2m-vm` prints these for a run, on stdout or to a file named by a flag, as `--serve-times` does now. Choose the
    form on the grounds and record it.
  - **The capability probes.** Add `w2m-vm --capabilities`, printing what `vcpus_allowed` and the design read:
    - FEAT_LSE2, `hypervisor::feature("FEAT_LSE2")`;
    - FEAT_BBM's level, a vCPU's `ID_AA64MMFR2_EL1` bits 55:52, `hypervisor/src/lib.rs` 195-196;
    - whether a guest WFE exits, which the `--wfe-probe` run already shows.

    `run_measurements` then emits one CAPABILITY_PROBED for each:
    - `feat_lse2`: `present` or `absent`;
    - `feat_bbm_level2`: `present` when the level is 2 or more, else `absent`, the level read in `detail` (vocabulary
      0.13's definition);
    - `guest_wfe_exit`: `present` when the WFE run's exits are above 0.
  - **The count as a type.** The one emit site of `vcpus` takes the count as `std::num::NonZeroU64`, so a count of 0
    cannot be written (vocabulary 0.13's E14, the rationale's enforcement).
  - **A new record.** Make one with `uv run substrate run --file harness/w2m-vm-measurements.toml --root
    store/runs/w2m-vm-measurements-<ns> --strict`, then `uv run substrate replay <root> --level 2`, then
    `target/release/check_records --since 2026-10-17 <root>`, which must print no violation.
  - **A test that holds it.** A test holds the newest `store/runs/w2m-vm-measurements-*` record. It checks:
    - `locked_model_steps` is 100,000 at `vcpus` 2, Windows run 41326239822's adds by thread A;
    - `turn_start_ns_p50` is at most `turn_start_ns_p99`, both above 0;
    - each of the three capabilities appears once, `feat_lse2` `present` on this Mac (`hypervisor/src/lib.rs` 187).

    The locked_race test keeps its stderr reading.
- **Not here:**
  - The runtime word's hold by `holder`, which needs the run to measure by holder (the host design's G6). C1 comes
    first in the order.
  - The instruction clock and the batch stop on two vCPUs, each a card of its own.
  - README's Status and the census of 0.12's 21 unemitted tags.

## prerequisites

- 336 (closed)

## context_files

- `crates/w2m_vm/src/bin/run_measurements.rs`, all 257 lines: its children, `measurement_line` and its record's order.
- `crates/w2m_vm/src/bin/w2m-vm.rs`: the argument parsing (300-341), `--serve-times`, `--wfe-probe`, and the
  `vcpus_allowed` calls (318-320, 389).
- `crates/w2m_vm/src/run.rs`: `vcpus_allowed` (605-633) and `Outcome` (`locked_steps`, `turn_start_ns`).
- `crates/w2m_vm/src/record.rs`: `measurement_line`, `quantile`, and how CAPABILITY_PROBED is written, if it is.
- `crates/hypervisor/src/lib.rs` 180-200: `feature`, `ID_AA64MMFR2_EL1`.
- `crates/events/src/generated.rs`: `MeasurementRecorded` (its `vcpus` and `holder`) and `CapabilityProbed`.
- `signals/0.13.json`: MEASUREMENT_RECORDED, CAPABILITY_PROBED, `enums.capability`, `enums.capability_outcome`, E14
  and E15. Also `signals/0.13-rationale.md`.
- `harness/w2m-vm-measurements.toml`, and `crates/w2m_vm/tests/run_loop.rs` 1065-1107 and 1197-1223, the two-vCPU
  helpers and the locked_race test.
- `WORKING_AGREEMENT.md` lines 137-266, "Build and verification commands", with "A sprint closes in its work commit"
  and "no gate row is dropped".

## signal contract

### Emits

- `run_measurements` on its record:
  - SESSION_INIT;
  - MEASUREMENT_RECORDED with `metric` `locked_model_steps`, `turn_start_ns_p50` and `turn_start_ns_p99`, each with
    `vcpus` 2;
  - CAPABILITY_PROBED with `capability` `feat_lse2`, `feat_bbm_level2` and `guest_wfe_exit`, each with its
    `outcome` and `detail`;
  - and, as before, its one-vCPU measurements and SESSION_COMPLETED.

### Invariants

- `check_records` over the new record prints no violation, E14 and E15 included.
- Every gate runs as before, and the one-vCPU measurements keep their form.

## artifact contract

### Files created

- `store/runs/w2m-vm-measurements-<ns>/` (the new record)
- The test, in a test file of `crates/w2m_vm/tests/`

### Files modified

- `crates/w2m_vm/src/bin/run_measurements.rs`, `crates/w2m_vm/src/bin/w2m-vm.rs`

### Command exit codes

- `uv run substrate replay store/runs/w2m-vm-measurements-<ns> --level 2` returns 0
- `scripts/gates.sh` returns 0
- `target/release/signal_report --check` returns 0 on the tree

## observation contract

- **Expected values:**
  - Quote the record's two-vCPU rows and its three CAPABILITY_PROBED rows in the result.
  - `locked_model_steps` is 100,000, and `feat_lse2` is `present`.
  - Quote `feat_bbm_level2`'s `detail` with the level read. Sprint 303 read 0 on this Mac, so `absent` is expected,
    and the result says which.
- **Planted faults:**
  - (1) The vCPU count written as 1 at the two-vCPU emit site. The test fails on the `vcpus` it reads.
  - (2) `feat_bbm_level2` given `present` at any level. The test, holding the outcome to the level in `detail`, fails.

  Before the work, read each judge's reach: the test reads the record the planted run writes, so the plant is run on a
  scratch record (`--root` under the scratchpad) that the test is pointed at.

## done criteria

A two-vCPU run's locked steps and turn's start, and the three capabilities `vcpus_allowed` and the design read, are on
a substrate record. The record passes the records check and replays, and a test holds its values.

## self-review checklist

- [x] Scope names one verifiable property: the two-vCPU run's figures and three capabilities on a record a test holds.
- [x] context_files cite the producer, `w2m-vm`'s flags, the refusal checks, the writer, the vocabulary and the tests.
- [x] Emits: tags and values in 0.13 only.
- [x] Artifact contract gradable by command.
- [x] Observation contract: the rows quoted; two plants, their reach read.
- [x] Sweet spot: `run_measurements.rs` and `w2m-vm.rs` plus a test; one concept, the record.

## result

GATES-LINE
