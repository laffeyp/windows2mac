# Sprint 181 — The run loop serves the model

```yaml
---
id: 181
status: done
phase: 6
pass_kind: functional
---
```

## scope

Decision "Sequence after Sprint 178" (reviewer-2's Medium 2 of its review of 175-177): `model_step` (`crates/w2m_vm/
src/cases.rs`) is the only server of hvc 10 and 11, and the run loop stops there (`run.rs` 554, `Stop::Hvc`). Each model
slice since Sprint 173 has added state it carries in and out (x87 in 175, XMM back in 178). The move makes the case
runner, with its 86,129 stepped cases, judge the function the run loop runs.

- **The move:** `model_step`, `GuestModelMemory` and the state transfer go to `crates/w2m_vm/src/model.rs` (or a
  `model_step.rs` beside it), unchanged in behavior; `cases.rs` calls it from there.
- **The run loop:** `run.rs` serves hvc 10 and 11 through it, as `vm_backend.py`'s `_slow_sse` and `serve_exit` do
  (5203, 5270), resuming at the dispatcher; a refused or undefined step stops the run by name, as before.
- **The judge:** the case runner's counts, `expected-by-class.tsv`, the compiler-of-record files and the records stay
  as Sprint 180 left them (TECHNIQUES #43, a behavior-preserving move judged by unchanged counts).
- **A guest that reaches the model:** no run-loop judge's guest does (checked 2026-10-28: the words of qpc, sleeper and
  sleeper-end hold no `hvc #10` or `#11`). A new guest, `tests/data/timed_waits/model_steps.c`, built as its
  neighbours are (`x86_64-w64-mingw32-gcc -O1 ... -nostdlib -e start`, `tests/test_timed_waits.py` 4), runs CMPSS
  (`_sse.py` 425: always `hvc #10`) and RCR (a whole model step, `hvc #11`) in a loop and writes their results; the
  frozen Python host's run of it (`run_guest`, no Python edited) is its fixture, `build/run_loop/model-steps-python.json`
  pinned by a committed sha256, and the run-loop judge (`crates/w2m_vm/tests/run_loop.rs`) holds the Rust run's stop
  and output to it.

## prerequisites

- 180 (closed)

## context_files

- `crates/w2m_vm/src/cases.rs` (`model_step`), `crates/w2m_vm/src/run.rs` (439-444, 554), `src/w2m/vm_backend.py`
  4268-4345, 5195-5280

## signal contract

### Emits

- On four `case-runner-rust` records: the verdicts Sprint 180's records hold.

### Invariants

- No Python file added or changed; every count the case runner holds unchanged.

## artifact contract

### Files modified

- `tests/data/timed_waits/model_steps.c` and `.exe` (new), `store/run_loop/model-steps-python.sha256` (new), `crates/w2m_vm/tests/run_loop.rs`
- `crates/w2m_vm/src/model.rs` (or a new `model_step.rs`), `crates/w2m_vm/src/cases.rs`, `crates/w2m_vm/src/run.rs`,
  `WORKING_AGREEMENT.md`, `docs/step-4-census-2026-10-28.md` (row 4); four records

### Command exit codes

- the case runner, the run-loop judges and `scripts/gates.sh` return 0

## observation contract

- **Expected values:** the case runner's held files unchanged; a guest that reaches hvc 10 or 11 resumes after it.
- **Planted faults:** (1) MXCSR not written back after a model step: the case runner names an MXCSR difference;
  (2) the run loop resuming at the instruction instead of the next: the run-loop judge names the new guest's stop or
  output. Plant 2 reaches its judge only through the new guest, which is why the guest is in this card.

## done criteria

The run loop serves hvc 10 and 11 through the function the case runner judges.

## self-review checklist

- [x] Scope names one verifiable property: one model server, judged.
- [x] context_files cite the code to move and the Python it follows.
- [x] Emits: the case runner's records, unchanged.
- [x] Artifact contract gradable by command.
- [x] Observation contract: unchanged counts and two plants.
- [x] One concept: the run loop serves the model.

## result

- **The move:** `model_step`, `GuestModelMemory`, `ModelStep`, `mxcsr_flags_from_fpsr` and `FPSR_TO_MXCSR` left
  `cases.rs` for `crates/w2m_vm/src/model_step.rs` unchanged, `ModelStep` and `model_step` public; `cases.rs` calls them
  from there. The case runner's three tests pass with its held files unchanged (`expected.tsv`, `expected-by-class.tsv`,
  the two compiler-of-record files), and the four records hold Sprint 180's verdicts: 236, 177, 107 and 26,
  `check_records` 0 violations each; the ledger holds 10 open FCOS and FSIN entries.
- **The run loop:** `run.rs` serves hvc 10 and 11 before its `Stop::Hvc`: `x86_address_at` (vm_backend 4861-4863, over
  the reconstruction table in guest memory) gives the guest address of the hvc's site, `code_at` reads up to 15 bytes
  there with 16 zeros after, `model_step` steps it, and the run goes on at the PC the step leaves; a step the model
  cannot take stops the run as `Stop::Model { rip, why }`.
- **The guest:** `tests/data/timed_waits/model_steps.c`, built with GCC 16.2.0 as its neighbours, unmasks MXCSR's
  overflow exception (0x1b80) so ADDSS leaves its fast path for hvc #10, and runs RCR with STC before it, each a whole
  step by hvc #11, 100 rounds. The frozen Python host ran it to ExitProcess with "7a6a27c287bac48c" and 100 hvc #10, 200
  hvc #11 exits, twice byte for byte; `build/run_loop/model-steps-python.json`, pinned by
  `store/run_loop/model-steps-python.sha256`. The Rust run loop gives the same stop, output and exits, 9 dispatch
  misses too (`model_steps_runs_through_the_model_as_the_python_host`).
- **Plant 1** (MXCSR not written back after a step, `model_step.rs`): the case runner failed, "33 CVTPD2PS fault MXCSR:
  case 4137: [66, 0f, 5a, 20] vm 0xf300, cpu 0xf328", with ADDSD's. Restored byte for byte.
- **Plant 2** as the card wrote it (resuming at the instruction, not the next) would re-run the instruction without end
  and hang the judge rather than fail it; it was redrawn on the same path before it ran, the step's RIP one byte off: the
  run-loop judge failed, output "f55eb7fa87bac48c" against the Python's "7a6a27c287bac48c". Restored byte for byte.
- **Departures from the card:** the guest's hvc #10 class is ADDSS with an exception unmasked, not CMPSS, which the Rust
  does not lift (Sprint 178's Deferred classes): the first build stopped at "no lifting rule for CMPSS at 0x140001024";
  plant 2 redrawn as above. Every multi-line edit used the Edit tool; one-line `sed`s for the imports, the module line
  and the plants.
