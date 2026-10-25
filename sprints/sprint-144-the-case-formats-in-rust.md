# Sprint 144 — The CPU cases and the Windows results, read in Rust

```yaml
---
id: 144
status: done
phase: 6
pass_kind: functional
---
```

## scope

The case runner's first half: link 3 of `docs/design-host-compile-path-2026-10-24.md`, split by technique #17 once
`scripts/vm_oracle.py` was read. The runner's judge is the CPU cases Sprint 039 and its successors ran on Windows:
21 case sets in `probes/semantics*/cases.bin`, with each run's results. This sprint carries their formats into a Rust
crate, `cpu_cases`, from `scripts/semantics_differential.py`:

- `Case` and `X87State` (103-135);
- the case file's formats 1 to 4 (`case_format`, `read_cases`, `read_cases_format1`, `write_cases`, 752-818);
- `Result`, `result_size`, `parse_result` and `read_results` (820-873);
- the address constants (65 and its neighbours);
- `undefined_memory` (917-945), which decides whether a case's memory is defined.

From `scripts/vm_oracle.py` it carries `read_fault_results` (70-83) and `result_file`'s SHA-256 check (55-67), without
the download. Sprint 145 runs the cases in a VM.

Amended before code: `undefined_memory` reads the instruction's operands, so `src/w2m/x86ops.py` (`operands`, `_mem`,
`_bits`, `_reg`, 17-155) joins `x86_decoder` in this sprint. Its judge is the C twin the VM runs, `x86_operands` in
`x86_decode.c`, over the same record and sweep as Sprint 143's differential, compared operand by operand.

## prerequisites

- 143 (closed)

## context_files

- `scripts/semantics_differential.py` 60-135, 557-600, 727-873, 917-945 (read in full at the sprint's start);
  `scripts/vm_oracle.py` (read in full)
- `probes/semantics*/cases.bin`, `store/probe-runs/*/cases.sha256`, `store/probe-runs/*/results.sha256`,
  `store/probe-runs/*/result-1.faults.bin`

## signal contract

### Emits

None. A reader; it puts nothing on a record.

### Consumes

Nothing at run time.

### Invariants

- No Python file added or changed. The frozen Python may be run to read, never written.

## artifact contract

### Files created

- `crates/cpu_cases/` (the formats and their tests)

### Files modified

- `Cargo.toml`, `Cargo.lock`, `WORKING_AGREEMENT.md`, `BLACKBOARD.md` (Sprint 143's CI run, in the tail entry)

### Content assertions

- Each item cites the `semantics_differential.py` or `vm_oracle.py` lines it carries.

### Command exit codes

- `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `cargo build --release` return 0
- `scripts/gates.sh` returns 0

## observation contract

- **Expected values:**
  - each of the 21 case files read and written back byte for byte, its format named, its case count reported;
  - each file's SHA-256 equal to the `cases.sha256` of every probe run that ran it;
  - each committed fault extract read and written back byte for byte;
  - every result file present under `build/probe-runs/` that matches its run's `results.sha256` read whole, with
    one result per case of its set;
  - `undefined_memory`'s count of refused cases per set equal to the frozen Python's, read by running it.
- **Planted fault:** one field of the format-3 record read 8 bytes late: the round trip names the first case that
  differs.

## done criteria

Every CPU case w2m ran on Windows, and every result Windows gave, is read from Rust exactly as it was written.

## self-review checklist

- [x] Scope names one verifiable property: the case and result files round-trip and match their recorded hashes.
- [x] context_files cite the Python carried.
- [x] Emits: none, said so.
- [x] Artifact contract gradable by command.
- [x] Observation contract: expected values and a planted fault.
- [x] One concept: the case and result formats.

## result

- `crates/cpu_cases`: `X87State`, `Case`, `CaseResult` (the Python's `Result`, renamed so it does not shadow Rust's),
  `read_cases`, `write_cases`, `case_size`, `result_size`, `parse_result`, `read_results`, `read_fault_results`,
  `recorded_sha256` and `undefined_memory`, each citing its lines. Two writers have no Python twin, `pack_result` and
  `write_fault_results`; they exist only for the round trip. The format-1 readers are not carried, because no
  committed file is format 1.
- `crates/x86_decoder/src/operands.rs`: `operands`, `_mem`, `_bits`, `_reg`, carried from `x86ops.py`.
  `tests/c_differential.rs` now compares operands too: the C's `x86_operands` output is rendered in x86ops' string
  format, and all 1,186,511 record instructions and 58,684 swept instructions agree operand by operand.
- Expected values (`tests/round_trip.rs`, 4 tests):
  - 21 case files, 250,840 cases, read and written back byte for byte, each matching the `cases.sha256` of at least
    one probe run;
  - both committed fault extracts round-trip, with 5,485 and 2,195 faults;
  - 22 downloaded result files match their runs' `results.sha256` and read whole, one result per case of their set;
  - `undefined_memory` refuses 43 cases in set 044 and 11 in set 047, as the frozen Python does (run on 2026-10-24).
- Planted faults:
  - the format-3 YMM block read 8 bytes late: "semantics_049/cases.bin: the round trip differs at byte Some(452)
    (case Some(0))";
  - REX.B dropped from the `Z` operand: the differential named `mov r8, imm64` (`49 b8 ...`), "R:RAX" against the C's
    "R:R8".

  Both were restored, and the tests pass.
- Tests: cpu_cases 4; x86_decoder 9.
