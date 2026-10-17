# Sprint 098 — the batch run's model-stepped classes made native

```yaml
---
id: 098
status: active
phase: 5
pass_kind: functional
---
```

## scope

The first of the five entries that were deferred and should not have been (user decisions 2026-10-11, 2026-10-16:
deferred work the project can do now is done now; a common instruction on a slow path is a defect). The batch run
(Sprint 097's, frontier-643a6283510c-1792212450643953000) leaves the VM for the host's model on these classes:

| class | exits | path |
|---|---|---|
| PACKSSDW | 9,681 | hvc 10 (SSE in the model) |
| PMULLD | 6,426 | hvc 10 |
| FLD, FSTP | 6,612 each | hvc 11 (x87 in the model) |
| FSQRT | 6,508 | hvc 11 |
| FNSTCW | 2,648 | hvc 11 |

and 545 more over nine classes (POPFQ 170, VCVTSS2SI and VCVTSI2SS 128 each, FSIN and FCOS 52 each, pinned since
065; XGETBV 11; MOVHPD 2; RCR 1). This sprint lowers PACKSSDW and PMULLD natively, and the x87 FLD, FSQRT, FSTP and
FNSTCW natively where the x87 state makes it exact.

**Known first.**

- PACKSSDW: "converts packed signed doubleword integers into packed signed word integers (PACKSSDW), using saturation
  to handle overflow conditions" (Intel SDM Vol 2B, PACKSSWB/PACKSSDW, as felixcloutier.com/x86 reproduces it, read
  2026-10-16); AArch64 SQXTN and SQXTN2 narrow with signed saturation.
- PMULLD: the low 32 bits of each 32-bit product (SDM Vol 2B); AArch64 MUL (vector) .4S keeps the low 32 bits.
- x87: a result is rounded to the precision the control word's PC field names. With PC 53, FSQRT's result rounds once to double precision, as AArch64 FSQRT (double)
  does; with PC 64 rounding twice can differ, so the native path checks PC and the rounding field and leaves other
  control words to the model. Windows' control word at a thread's start is 639 (0x27F, PC 53 bits), observed on the
  runners in Sprint 034 (specs/guest_state.json, x87_control_word).
- Judges exist: the class runs compare the model and the VM with the Xeon's results case by case.

## prerequisites

- 097.

## context_files

- src/w2m/vm_backend.py (the SSE and x87 lowerings, the slow paths); src/w2m/rules/ for these classes;
  scripts/class_run.py; the case sets that hold them.

## signal contract

### Emits

- None new; class item runs as before.

### Invariants

- Vocabulary unchanged; every class's verdict against the Xeon unchanged.

## artifact contract

- The lowerings; class runs of PACKSSDW, PMULLD, FLD, FSTP, FSQRT, FNSTCW accepted on the VM; all classes and
  families re-run; `scripts/gates.sh` exit 0; CI `success` on the closing commit.

## observation contract

- **Judges:** the six classes accept on the model and the VM against the Xeon's cases; 456 of 458 overall as before;
  the batch run conforms (168 of 168 lines, 0 divergences).
- **Plants:** SQXTN replaced by XTN (no saturation): the PACKSSDW cases that saturate must fail; the PC check
  removed with a PC-64 case: the x87 case set's 64-bit precision cases must fail.
- **Measure:** hvc 10 and 11 exits on the batch run fall from 39,000 to under 2,000.

## done criteria

The batch run's common model-stepped classes run natively, every judge holds, CI green.

## observed
