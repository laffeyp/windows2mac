# Sprint 173 — The model's core and its integer classes, in Rust

```yaml
---
id: 173
status: done
phase: 6
pass_kind: functional
---
```

## scope

Step 4's first component, the model (S4; Decision 2026-10-27; census row 4, `docs/step-4-census-2026-10-28.md`). The
model is the frozen Python's interpreter of lifted operations: `src/w2m/runtime.py` `_exec` and `_exec_ops` (995-1161),
with `integer_model.py` (157 lines) for the `x86` steps, `x86fp.py` for SSE, `x87.py` and `_mmx` for the rest. The VM
reaches it through hvc 10 (slow SSE) and hvc 11 (slow step); the Rust run loop ends at both (`run.rs` 553-554).

The case runner counts, by class, the cases it sends to the model and does not judge (2026-10-28, its run with output:
25,077 cases in 96 of 289 classes): 2,680 in `integer_model`'s classes (RCR 784, SHLD 276, SHRD 264, DIV 237, IDIV 219,
CMC 132, CLC 116, …), 562 in the locked forms whose operand is misaligned (hvc 11 from `lower.rs` 1375-1381), and 21,835
in MMX and SSE classes (MOVQ 2,600, PSRAW 1,416, PSRAD 1,362, …). Every slice runs on the interpreter's core, so this
sprint ports it with the integer classes:

- **The core:** `_exec` (a fault restores every register and flag but REP's completed iterations; 995-1013) and
  `_exec_ops`' integer and control ops: `imm`, `addr` (a segment-relative address refused as the Python refuses it),
  `ld`, `st`, `alu` (its 22 operations), `flags`, `set`, `sel`, `cst`, `cstz`, `cmov` (a 32-bit destination
  zero-extended), `br`, `df`, `pause`, `fence`, `ud`, `div`, `cx16`, `rep`, `cpuid`, `xgetbv`, `mxcsr`, `fcw`, with the
  helpers they call (`_load`, `_store`, `_set_flags`, `_cond`, `_flag`, `_div`, `_cmpxchg16b`, `_rep`), cited by line.
  An `sse`, `vex`, `vexs`, `fma`, `mmx`, `vzeroupper` or `x87` op is refused by name: the next slices.
- **The integer classes:** `integer_model.execute`, all 21 classes, line for line, with its Xeon-measured cases (the
  16-bit SHLD with a count of 17 to 31; the 32-bit destination written at a count of 0).
- **Where it runs:** the case runner serves its model cases for these classes with the Rust model and judges them
  against Windows, as it judges the rest; the run loop serves hvc 11 when the instruction's ops are all the core's, and
  ends at a named stop otherwise.

The Python tests' values carried (plan 132-133; census row 1): no Python test names `integer_model` (searched
2026-10-28: `tests/`, `scripts/`); its judge was the class runs against Windows, which the case runner is; the core's
values come from `tests/test_runtime.py` (5 tests), each Rust test citing the file and line it carries.

## prerequisites

- 172 (closed)

## context_files

- `src/w2m/runtime.py` 995-1161 and the helpers it calls; `src/w2m/integer_model.py` (read in full 2026-10-28)
- `crates/w2m_vm/src/cases.rs` (the model's cases, `Tally::model_by_class`), `crates/w2m_vm/src/run.rs` 437-554
- `tests/test_runtime.py`

## signal contract

### Emits

- On four `case-runner-rust` records: SESSION_INIT, CANDIDATE_GENERATED, COMPARISON_COMPLETED, CANDIDATE_ACCEPTED,
  SESSION_COMPLETED, as before, now judging the integer classes' model cases.

### Invariants

- No Python file added or changed; every case judged today is judged as before.

## artifact contract

### Files created

- `crates/w2m_vm/src/model.rs` (or a `model` crate if the translator's `Op` is the shared type), its tests

### Files modified

- `crates/w2m_vm/src/cases.rs`, `crates/w2m_vm/src/run.rs`, `WORKING_AGREEMENT.md`; four records under `store/runs/`

### Command exit codes

- the case runner and `scripts/gates.sh` return 0

## observation contract

- **Expected values:** the 3,242 integer and misaligned-lock model cases judged against Windows with 0 differences;
  the case runner's "to the model" count falls by them; the MMX and SSE model cases stay counted, not judged.
- **Planted faults:** (1) RCR's count taken mod 8 for 8 bits where the Python takes mod 9: the case runner names RCR;
  (2) a fault that does not restore the flags: a case faulting mid-instruction names its flags.

## done criteria

The integer and misaligned-lock model cases are judged against Windows through the Rust model.

## self-review checklist

- [x] Scope names one verifiable property: the integer model cases judged through Rust.
- [x] context_files cite the Python read in full and the Rust it enters.
- [x] Emits: the case runner's records.
- [x] Artifact contract gradable by command.
- [x] Observation contract: counts and two plants.
- [x] One concept: the model's core and its integer classes.

## result

- **The model:** `crates/w2m_vm/src/model.rs`, `runtime.py` 193-388 and 995-1161 and `integer_model.py` whole, each
  function citing its lines. The case runner's hvc 10 and 11 call it as `_slow_sse` does (`cases.rs` `model_step`):
  the registers, flags (C Z S O from their registers, P and A lazy), DF, MXCSR and FCW in; one step over guest memory;
  the result out, x17 the next RIP and the dispatcher next; a guest fault raised through the runtime's exception path
  with the exit's registers, as `_raise_in_guest` raises it. An `sse` or `cpuid` operation is refused, the case
  counted to the model as before.
- **Judged against Windows:** 3,521 cases the case runner had counted and not judged, with 0 difference classes in
  both tests: the misaligned locked forms (533 in set 041: ADD_LOCK 91, XOR_LOCK 85, AND_LOCK 83, SUB_LOCK 83, OR_LOCK
  77, INC_LOCK 31, DEC_LOCK 30, CMPXCHG_LOCK 28, XADD_LOCK 25; NEG_LOCK 29 in 042), DIV 237 (044), IDIV 219 (046b), LDMXCSR 52
  (044d), VLDMXCSR 227 (049b) and 2,224 in set 050 (RCR 784, SHLD 276, SHRD 264, CMC 132, CLC 116, PUSHFQ 116, SAHF
  116, the string and loop classes 24 each, and the rest). `store/case_runner/expected.tsv`'s seven changed runs moved
  judged up and "to the model" down by exactly those counts.
- **A fault the judge found:** the first run named "2 IDIV r2: case 344: [2e, 4c, f7, 3f] vm 0xe5fc5d3b249a4f53, cpu
  0xe5fc5d3b249a4f54": the signed 128-bit dividend was built with `1 << 128`, a shift past the type that release
  builds wrap. Fixed; every other wide shift in the file read and found within its type.
- **Plant 1** (RCR's 8-bit count taken mod 8): the case runner failed, "88 RCR CF: case 206: [66, c0, 9c, 75, d6, 28,
  f7, 52, 4b] vm 1, cpu 0", with "120 RCR data window" and "7 RCR r0". Restored.
- **Plant 2** as the card wrote it (a fault that keeps the model's flags) reached no judge: the case runner passed.
  On a fault the case runner raises with the exit's own registers, as the Python's `_raise_in_guest` takes `x`, so
  the model's restored state is never read on that path. Replaced by a plant on the fault path the judge reads, the
  exception RIP off by one: the case runner failed, "91 ADD_LOCK fault rip: case 19: [f0, 66, 01, 42, d0] vm
  0x20000001, cpu 0x20000000", and so AND_LOCK, CMPXCHG_LOCK, DEC_LOCK. Restored.
- **The Python tests' values:** no Python test names `integer_model`; `tests/test_runtime.py` runs the whole Python
  runtime with its kernel32 over `skeleton.exe`, and its one model fact, `mul_hi`'s high half (59-61), is a Rust unit
  test, with DIV's fault and RCR's count mod 9.
- **Not done:** the Rust run loop (`run.rs` 553-554) still ends at hvc 10 and 11. No guest the Rust host runs reaches
  them (`qpc` and `sleeper` run to ExitProcess without), so a serving path there would have no judge; it lands with
  the batch run's first model exit.
