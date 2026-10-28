# Sprint 163 — An exit's time by kind, and whether a guest WFE exits, on a record

```yaml
---
id: 163
status: active
phase: 6
pass_kind: functional
---
```

## scope

Two of step 3's three measurements (`docs/rust-port-plan-2026-10-24.md` step 3, amended 2026-10-27; reviewer-2's High
1; Decision 2026-10-27), on a substrate record as MEASUREMENT_RECORDED with vocabulary 0.11's metrics (P24). The third,
the runtime word's hold and wait times, needs timestamps inside the monitor code's lock and unlock and is Sprint 164.

- **An exit's time by kind.** The Rust run loop already times each exit's service, from `hv_vcpu_run`'s return to the
  next run (`Outcome.serve_ns`, `run.rs` 353-359), as Sprint 142 measured one at 3,667 ns; it does not keep the exit's
  kind with the time. The run loop keeps them together (`hvc <k>`, `vtimer`, or the exception class); `w2m-vm
  --serve-times <file>` writes them. Measured on `qpc` and `sleeper` (`tests/data/timed_waits`), each run to
  ExitProcess: per kind, `exit_ns_p50` and `exit_ns_p99` (nearest rank) and `vm_exits` (P20), in ns and exits. The C
  monitor's exit alone measured 813-852 ns (Sprint 132); the host design's budgets assume a served exit near that.
- **Whether a guest WFE exits.** No guest code w2m runs executes WFE or WFI (none in `tools/guest_runtime`,
  `tools/vm_compiler` or the translator); Apple's documentation does not say whether Hypervisor.framework traps them,
  and QEMU's HVF backend handles `EC_WFX_TRAP` (`target/arm/hvf/hvf.c`; `docs/design-rust-host-2026-10-24.md` 247-249).
  `w2m-vm --wfe-probe` runs a guest of WFE ×1,000, WFI ×1,000 and `hvc #0`, counting the exits by exception class:
  `wfe_exits` with variant `wfe` and `wfi`. WFE (d503205f), WFI (d503207f) and `mrs x16, cntvct_el0` (d53be050) were
  assembled by clang 2026-10-27.
- **The record.** `run_measurements`, a Producer (`harness/w2m-vm-measurements.toml`, purpose `measurement`), runs the
  three `w2m-vm` children through `scripts/sign_and_run.sh` (one VM a process) and writes SESSION_INIT,
  MEASUREMENT_RECORDED per number, SESSION_COMPLETED.

## prerequisites

- 162 (closed)

## context_files

- `crates/w2m_vm/src/run.rs` 296-380, `crates/w2m_vm/src/bin/w2m-vm.rs`, `crates/w2m_vm/src/record.rs`
- `crates/hypervisor/tests/vector_register.rs` (a raw guest), `crates/hypervisor/src/lib.rs` (`Exit`)
- `signals/0.11.json` MEASUREMENT_RECORDED; a Python batch run's record (`store/runs/frontier-*`: its payload's
  conventions, `destination_profile_id` `w2m-vm`, `backend` `managed_vm`, `phase` `discovery`)

## signal contract

### Emits

- On `store/runs/w2m-vm-measurements-<ns>`: SESSION_INIT (purpose `measurement`), MEASUREMENT_RECORDED (`exit_ns_p50`,
  `exit_ns_p99` and `vm_exits` per guest and exit kind; `wfe_exits` for `wfe` and `wfi`), SESSION_COMPLETED
  (completed).

### Consumes

`tests/data/timed_waits/qpc.exe`, `sleeper.exe`.

### Invariants

- No Python file added or changed; the skeleton's record (Sprint 158) unchanged: `w2m-vm` without the new flags writes
  what it wrote.

## artifact contract

### Files created

- `crates/w2m_vm/src/bin/run_measurements.rs`, `harness/w2m-vm-measurements.toml`, the record under `store/runs/`

### Files modified

- `crates/w2m_vm/src/run.rs`, `crates/w2m_vm/src/bin/w2m-vm.rs`, `crates/w2m_vm/src/record.rs`, `WORKING_AGREEMENT.md`

### Command exit codes

- `uv run substrate run --file harness/w2m-vm-measurements.toml --root store/runs/w2m-vm-measurements-<ns> --strict`
  returns 0
- `scripts/gates.sh` returns 0

## observation contract

- **Expected values:** each guest runs to ExitProcess; its exit kinds' counts equal its run's `exits` (the run loop's
  own tally); `exit_ns_p50` ≤ `exit_ns_p99` for every kind; `wfe_exits` for `wfe` is 0 or 1,000 and so for `wfi`,
  whichever the hardware gives, stated as found. The card's result compares the medians with 813-852 ns.
- **Planted fault:** the quantile taken at index n·q without the nearest-rank ceiling, on a 3-sample unit test of known
  answer: the test names p50.

## done criteria

Step 3's exit time by kind and the WFE answer are on a record, in vocabulary 0.11's metrics.

## self-review checklist

- [x] Scope names one verifiable property: two of the three measurements on a record.
- [x] context_files cite the code carried and the record conventions.
- [x] Emits: every tag the run fires, by name.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values with their bounds and a plant.
- [x] One concept: step 3's exit and WFE measurements.

## result

- **The record** (`store/runs/w2m-vm-measurements-1793169234000000000`, substrate exit 0): SESSION_INIT (purpose
  measurement, vocabulary 0.11), 17 MEASUREMENT_RECORDED, SESSION_COMPLETED completed in 0.918 s.
- **An exit's host time, by kind** (from `hv_vcpu_run`'s return to the next run; nearest rank):

  | guest | kind | exits served | p50 | p99 |
  |---|---|---|---|---|
  | qpc | hvc 1 | 6 | 708 ns | 199,000 ns |
  | qpc | hvc 8 | 1 | 1,042 ns | 1,042 ns |
  | sleeper | hvc 1 | 14 | 584 ns | 223,083 ns |
  | sleeper | hvc 8 | 7 | 708 ns | 34,625 ns |
  | sleeper | hvc 12 | 1 | 3,375 ns | 3,375 ns |

  The medians, 584 to 1,042 ns, stand beside the C monitor's 813-852 ns of Sprint 132; Sprint 142's one sample, 3,667
  ns, was not the common case. The samples are small: with n under 100 the p99 is the largest, and the hvc 1 p99s
  (about 200 µs) are the misses where the host compiled a block, not exits served alone. Each guest's served hvc exits
  equal its run's own tally less the last exit (the producer checks it). No vtimer or exception exit occurred. The
  distribution the host design's budgets need comes from step 4's batch run, which serves exits by the hundred
  thousand; these are the skeleton's.
- **Whether a guest WFE exits:** no. Of 1,000 WFEs the framework trapped none, and the run reached its `hvc #0` before
  the watchdog: a guest WFE neither exits nor stalls. Of 1,000 WFIs it trapped all 1,000 (ESR EC 0x01, ISS bit 0 clear).
  The host design (247-249) said "If WFE exits, the spin can end in WFE and the host answer with a yield; if not, the
  hvc above stands": the hvc stands.
- **The plant** (below, run after the gates): the card named a 3-sample test; of three, the median is the second by
  either rule, so the test uses four ([10, 20, 30, 40]: nearest rank 20, an index of n·q 30). Planted, the index of
  n·q: the test failed "p50 … left: 30, right: 20". Restored, it passes.
