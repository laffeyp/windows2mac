# Sprint 331 — Every other vCPU stopped for a locked model step

```yaml
---
id: 331
status: active
phase: 6
pass_kind: functional
---
```

## scope

The run-loop design's card 4d, split by rule 6: 4d1, here, the stop of every other vCPU for a model step of a form
x86 locks (the design's "A model step of a locked form", precondition 8); 4d2, the stop for a block split, a cancel
from another thread mid-split (Deferred, Sprint 305) and the guest Deferred (Sprint 330) names for the kick's wait.
Today a misaligned locked form runs in the model (`hvc #11`, `crates/translator/src/lower.rs` 1424-1432) as a load
and a store, under the runtime word and the lock (`run.rs` 1696-1716, Sprint 324), so two misaligned steps on two
vCPUs already run one at a time; an aligned locked form runs in the VM as an exclusive pair, which takes no word, so
an aligned add on another vCPU to a byte the step covers, made between the step's load and its store, is written
back over; x86 makes each access atomic at any alignment (Deferred, Sprint 272; the design's "A model step of a
locked form": "The word would not exclude the aligned pair").

- **The work:** `RunStop` stops every other vCPU before a model step of a locked form and waits until each is out of
  guest code (Sprint 330's in-guest flag) before the step's load; each resumes after the step's store, its cancel
  read as a kick. Which steps are locked: the lifted instruction's lock prefix or a memory XCHG, read from the lifter
  (`translator::lift`). A new probe, `probes/locked_race/`, built and run on the Windows runner
  (`.github/workflows/windows-probe.yml`), its binary and output kept by hash: in one 64-byte-aligned line, thread A runs
  `lock xadd` of 1 to the dword at offset 1, 100,000 times, and thread B `lock xadd` of 1 to the dword at offset 4
  until A is done, counting its adds; byte 4 is A's high byte and B's low byte. Rows: A's low 24 bits, 100,000, and
  B's adds lost, its count less its dword, 0.
- **Not here:** the split (4d2); the race test of the guest-memory design's choice 6 (4e).

## prerequisites

- 330 (closed)

## context_files

- `docs/design-run-loop-on-several-vcpus-2026-10-31.md` "A model step of a locked form" (58-68), precondition 8;
  `crates/w2m_vm/src/model_step.rs` `model_step`; `crates/w2m_vm/src/run.rs` `RunStop`, `kick_and_wait`, the
  `hvc #11` service; `crates/translator/src/lower.rs` 1424-1430; BLACKBOARD Deferred (Sprint 272)

## signal contract

### Emits

- None; when a locked model step stops the other vCPUs.

### Invariants

- Every run on one vCPU as held; on one vCPU there is no other vCPU to stop, and the step is as today.

## artifact contract

### Files modified

- `crates/w2m_vm/src/run.rs`, `crates/w2m_vm/src/model_step.rs`, `crates/w2m_vm/tests/run_loop.rs`,
  `probes/locked_race/` (new), `store/binaries/` and `store/family_runs/` (the probe's binary and Windows' output);
  the design's card 4 note, with the work

### Command exit codes

- `scripts/gates.sh` returns 0

## observation contract

- **Expected values:** a test running `locked_race` through `w2m-vm --vcpus 2`: A's sum 100,000 and B's adds lost
  0, every row equal to the Windows runner's output, held to those values; the same on one vCPU; every run on one
  vCPU as held.
- **Planted faults:** (1) the stop taken out of the locked step: B's adds lost above 0; (2) the wait for each vCPU
  out of guest code taken out, the cancels sent and not waited on: B's adds lost above 0. Each judge by lookup
  (WORKING_AGREEMENT, Sprint 284): A's misaligned operand reaches `hvc #11` (`lower.rs` 1424-1432, the `ubfx` and
  `cbz` on the address's low bits), the run loop's step under the word alone (`run.rs` 1696-1716), and B's aligned
  operand the exclusive pair, which takes no word; the test asserts B's adds lost, the value both plants change. That
  the race shows in a run is not read from the code: plant 1 is run first, as today's code, and if no run of 20 loses
  an add the result says the judge does not reach it and the window is measured before the stop is built; plant 2
  likewise against the built stop.

## done criteria

An aligned locked add on one vCPU loses no update to a misaligned locked add stepped in the model on another, as on
Windows.

## self-review checklist

- [x] Scope names one verifiable property: a locked model step is atomic on two vCPUs.
- [x] context_files cite the design, the step, the stop and the lowering.
- [x] Emits: none.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values held to Windows' output and the one-vCPU run, two plants, each judge by lookup,
  each plant's run before the result claims it.
- [x] One concept: the stop of every other vCPU for a locked step.

## result
