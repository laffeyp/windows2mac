# Sprint 337 — A two-vCPU run on a substrate record

```yaml
---
id: 337
status: done
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

- **The record:** `store/runs/w2m-vm-measurements-1795656377000000000`, made by `uv run substrate run --file
  harness/w2m-vm-measurements.toml --root store/runs/w2m-vm-measurements-1795656377000000000 --strict` (exit 0,
  "[finalised]"). `uv run substrate replay <root> --level 2`: exit 0, "[OK] Level 2 replay successful. Frames replayed:
  55". `target/release/check_records --since 2026-10-17 <root>`: exit 0, "1 records; 0 violations".
- **Its two-vCPU rows** (each `vcpus` 2, application `locked-race`): `locked_model_steps` 100000 `count`;
  `turn_start_ns_p50` 334 `ns` and `turn_start_ns_p99` 583 `ns`, variant "199964 turns". Every other
  MEASUREMENT_RECORDED carries `vcpus` null, the one-vCPU form.
- **Its CAPABILITY_PROBED rows** (host `mac-Mac17,7-macos-25F80`, as `scripts/litmus_mp.py` 59 at the tag
  frozen-python-2026-10-29 formed it): `feat_lse2` `present`, "hw.optional.arm.FEAT_LSE2 is 1 (sysctlbyname), which
  vcpus_allowed reads before it admits a second vCPU"; `feat_bbm_level2` `absent`, "FEAT_BBM level 0: ID_AA64MMFR2_EL1
  0x1001001102001011, bits 55:52, as a vCPU reads it; level 2 or more lets a change of block size alone go without a
  break", the level 0 Sprint 303 read, so `absent` as the card expected; `guest_wfe_exit` `absent`, "the WFE probe's
  run of 1,000 WFE ended at its hvc #0 with 0 WFx traps (ESR EC 0x01) to the host; its run of 1,000 WFI with 1000".
  The card's rule is the WFE run's exits above 0; a guest WFE does not exit on this Mac, a WFI does.
- **`w2m-vm`:** `--vcpu-figures <json>`, beside `--serve-times` and `--runtime-word`, writes `{"vcpus", "runs_by_vcpu",
  "locked_steps", "turn_start_ns"}` for a guest run; a file, since a guest run's stdout carries its record. `w2m-vm
  --capabilities` prints `{"feat_lse2", "id_aa64mmfr2_el1", "feat_bbm_level"}`, the register read from a fresh vCPU
  as `crates/hypervisor/tests/id_registers.rs` reads it; run by hand: `{"feat_bbm_level":0,"feat_lse2":1,
  "id_aa64mmfr2_el1":"0x1001001102001011"}`. The WFE capability is the existing `--wfe-probe` run's, not a second run.
- **`run_measurements`:** runs `store/binaries/<probe.sha256 of run 41326239822>.exe` with `--vcpus 2 --vcpu-figures`,
  fails by name when its rows are not `store/probe-runs/41326239822/body-1.jsonl` byte for byte, when `runs_by_vcpu`'s
  length is not the count or a vCPU ran 0 times, or when no turn was timed (by hand: runs by vCPU 99,961 and 100,009),
  and emits the three rows and the three CAPABILITY_PROBED before SESSION_COMPLETED.
- **The count as a type:** `record.rs` gains `measurement_line_on(vcpus: NonZeroU64, ...)`, beside `measurement_line`
  (which writes `vcpus` none), both through one private builder; it is the one site that writes `vcpus`, and
  `run_measurements` takes the count with `NonZeroU64::new`, failing on 0.
- **A check repaired:** the first record run failed, "sleeper: 22 served hvc exits, the run's tally 26". The tally
  holds, besides the run's last exit, each flush stub's `hvc #15` (Sprint 305), which `run.rs`'s turn start adds to
  `exits` and times as the split's service, never as its own kind: sleeper's three. `run_measurements` had not run since
  Sprint 305. The check now subtracts the `hvc #15` count; both members of the class (the last exit, the flushes)
  checked against sleeper (22 + 1 + 3 = 26) and qpc (7 + 1 = 8). The failed record, untracked, was moved to the
  scratchpad, not kept under `store/runs/`.
- **The test:** `crates/w2m_vm/tests/measurements_record.rs` (declared in `tests/all.rs`) reads the newest
  `store/runs/w2m-vm-measurements-*`, or `W2M_MEASUREMENTS_RECORD`: `the_two_vcpu_run_is_on_the_record` (locked steps
  equal Windows row `a`'s `sum`, 100000; 0 < p50 <= p99; `vcpus` 2 on the three, null on the rest) and
  `the_three_capabilities_are_on_the_record` (each once; `feat_lse2` present; `feat_bbm_level2`'s outcome held to the
  level parsed from `detail`; `guest_wfe_exit` held to the record's `wfe_exits` `wfe` row). Both pass on the record.
  `run_loop.rs`'s locked_race test keeps its stderr reading, unchanged.
- **The harness baseline:** `harness/w2m-vm-measurements.toml`'s `implementation` said "on one vCPU"; it now names the
  two-vCPU rows, and its comment names Sprint 337's additions.
- **Plant 1** (`record.rs` 513, the emit site, `HarnessU64::new(1)` in place of the count): scratch record
  `plant1-w2m-vm-measurements-1795656422000000000` under the scratchpad, the test pointed at it:
  `the_two_vcpu_run_is_on_the_record` FAIL, "\"locked_model_steps\"'s vCPUs left: Number(1) right: 2".
- **Plant 2** (`run_measurements.rs` 384, `if level < u64::MAX` in place of `if level >= 2`): scratch record
  `plant2-w2m-vm-measurements-1795656476000000000`: `the_three_capabilities_are_on_the_record` FAIL, "feat_bbm_level2
  at level 0 left: String(\"present\") right: \"absent\"". Each plant was made from a cp backup, restored by cp and
  checked by cmp ("cmp equal"), before the gates.
- **The gates:** one run, after the plants, on the restored tree; `signal_report --check` exit 0 before it.

exit 0 in 362 s	default	2026-11-26T01:38:47Z	sprint 337	b8e6bbc70b3a20ea6725a0d4eb8f4c1bcd4b94ee	9c5f8a7366844baec69a30dac1be5c6cbb89f24d71e9c1cfc5030c858d72682f	root=eb6885a158492f43e9ab5aa5ab9bafa1e4d26b77	rust=all python=0 samples=1	base=682b53718aaa86cab8cfd2ba15869dac940a7648
