# Sprint 185 — The model's steps, widened

```yaml
---
id: 185
status: active
phase: 6
pass_kind: functional
---
```

## scope

reviewer-2's Medium 5 of its review of Sprints 178-181 (the Decision "Sequence after Sprint 182"): the run loop's
judge of `model_step` covers ADDSS (hvc #10) and RCR (hvc #11) only. The batch run's model exits are POPFQ 170, FCOS
52, FSIN 52, XGETBV 11 and RCR 1 by hvc #11 (`build/sprint-108-run.log` 170-177). This sprint widens the guest to them
and to a raised #XM.

- **The guest:** `tests/data/timed_waits/model_steps.c` adds, each round, POPFQ setting CF from bit 0 of the round
  (read back by SETC), FSIN and FCOS on x87, and XGETBV; after the rounds, one ADDSS that overflows with the overflow
  exception unmasked, raised as the guest's #XM and caught by a vectored handler that masks every exception in the
  context's MXCSR so ADDSS runs again. Built with GCC 16.2.0 by `tests/test_timed_waits.py`'s line.
- **The fixture:** the frozen Python host's run of the guest (`run_guest`, no Python edited): ExitProcess, output
  "48743d4d4fd79694" and "c00000917f800000" (STATUS_FLOAT_OVERFLOW, then infinity), and exits hvc #1 21, #8 4, #10
  301 (ADDSS 101, CVTSI2SD 100, MULSD 100), #11 701 (POPFQ 101; PUSHFQ, STC, RCR, FSIN, FCOS, XGETBV 100 each), as
  read on 2026-10-28 before this card's commit; `build/run_loop/model-steps-python.json`, pinned by
  `store/run_loop/model-steps-python.sha256`.

## prerequisites

- 184 (closed)

## context_files

- `tests/data/timed_waits/model_steps.c`, `crates/w2m_vm/tests/run_loop.rs` (`model_steps_runs_through_the_model_as_
  the_python_host`), `tests/test_timed_waits.py` (`run_guest`), `tools/guest_runtime/vectored_handlers.c`

## signal contract

### Emits

- None: no producer runs; the judge is the run-loop test.

### Invariants

- No Python file added or changed; the Python host's run is read, not edited.

## artifact contract

### Files modified

- `tests/data/timed_waits/model_steps.c` and `.exe`, `store/run_loop/model-steps-python.sha256`,
  `crates/w2m_vm/tests/run_loop.rs` (its doc names ADDSS, not CMPSS), `WORKING_AGREEMENT.md`,
  `docs/step-4-census-2026-10-28.md` (row 4)

### Command exit codes

- the run-loop judges and `scripts/gates.sh` return 0

## observation contract

- **Expected values:** the Rust run loop ends the guest as the Python host: ExitProcess 0, the same two lines, and the
  same hvc #10 and #11 counts, 301 and 701.
- **Planted faults:** each checked against the guest before this card's commit: (1) the model's POPFQ dropping CF (`c:
  Some(false)`): the run-loop judge names the first line, "6df648a15c8ca4a2" against "48743d4d4fd79694"; (2)
  `xm_status` giving the overflow 0xC000008F: the judge names the second line, "c000008f7f800000" against
  "c00000917f800000".

## done criteria

The run loop serves the batch run's every hvc #11 class, and a raised #XM, as the frozen Python host does.

## self-review checklist

- [x] Scope names one verifiable property: the widened guest's run equals the Python host's.
- [x] context_files cite the guest, the judge and the Python run.
- [x] Emits: none, said so.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and two plants, both run before the commit.
- [x] One concept: the model's steps on the run loop.
