# Sprint 105 — the litmus tests as x86 code on several vCPUs

```yaml
---
id: 105
status: closed
phase: 5
pass_kind: functional
---
```

## scope

Design B's second step (docs/design-several-vcpus.md): the gate roadmap step 1 names. Five litmus tests written as
x86-64 code, compiled by w2m as any guest code is, each thread on its own vCPU (Sprint 104), many iterations, every
outcome counted, judged against the outcomes x86-TSO allows.

| Test | Threads | Outcome x86-TSO forbids | Allowed relaxation |
|---|---|---|---|
| MP | W: x=1; y=1. R: r1=y; r2=x | r1=1, r2=0 | none |
| SB | T0: x=1; r1=y. T1: y=1; r2=x | none | r1=0, r2=0 (the store buffer) |
| LB | T0: r1=x; y=1. T1: r2=y; x=1 | r1=1, r2=1 | none |
| 2+2W | T0: x=1; y=2. T1: y=1; x=2 | x=1, y=1 at the end | none |
| IRIW | W0: x=1. W1: y=1. R0: r1=x; r2=y. R1: r3=y; r4=x | r1=1, r2=0, r3=1, r4=0 | none |

(Owens, Sarkar and Sewell, "A Better x86 Memory Model: x86-TSO", TPHOLs 2009.)

No scheduler yet: the harness runs each thread once alone on its vCPU first, so every block is compiled before the
threads run together (two vCPUs compiling into the shared code cache at once is Sprint 106's to make safe), then runs
all of them at once through their channels; the threads meet at a start word each iteration and write their
registers to a results array.

## prerequisites

- 104.

## context_files

- docs/design-several-vcpus.md; src/w2m/vm_backend.py (the channels, the dispatcher's miss path); tests/test_vcpus.py;
  probes/mac_hv_litmus/mp_litmus.c (Sprint 009's harness).

## signal contract

### Emits

- None new.

### Invariants

- Vocabulary unchanged.

## artifact contract

- The five tests as x86 bytes with their harness (`tests/test_litmus.py`, hvf); `scripts/gates.sh` exit 0; CI
  `success` on the closing commit.

## observation contract

- **Judge:** over at least 100,000 iterations each, the forbidden outcome of MP, LB, 2+2W and IRIW appears 0 times;
  SB's relaxed outcome appears at least once (the test can see a reordering the hardware makes, so 0 elsewhere means
  something).
- **Plant:** ACTLR_EL1.EnTSO cleared on every vCPU: MP's forbidden outcome appears (Sprint 009 saw 2,245 in 5 million
  with it off on this Mac); if it does not appear in the run's iterations, the record says the plant was blind.

## done criteria

w2m-compiled x86 code on several vCPUs keeps x86-TSO's order on every litmus test; CI green.

## observed

- **Made:** tests/test_litmus.py: the five tests as x86 assembled by clang and compiled by w2m, one thread a vCPU,
  an arrival counter before each iteration; each thread runs once alone first, so its blocks exist before the
  threads run together.
- **Found on the way:** a PAUSE on a litmus vCPU yielded the slice every time: SLICE_UNTIL on its private page was
  copied from vCPU 0's as 0 (another thread ready), so each barrier spin left the VM, and 100,000 iterations ran for
  minutes; the harness sets it to none ready. Sprint 106's scheduler sets it per vCPU.
- **Judge,** 20 runs of 100,000 iterations each: MP, LB, 2+2W and IRIW's forbidden outcomes 0 in 2,000,000 each;
  SB's relaxed outcome 1 to 36 times a run. Both thread orders appear about half the time in every test (MP: (0,0)
  49,627, (1,1) 49,548, (0,1) 825), so the threads did run together.
- **Plant:** EnTSO clear on every vCPU: MP's forbidden outcome 4, 8, 8, 9 and 6 times in five runs of 100,000.
- **A flaky judge, measured:** one run of the test file failed once (its assertion not kept); SB's relaxed outcome was 0 in 3 runs of 100, so the SB test
  takes up to five runs and the plant test up to three.
