# Sprint 009 — EnTSO message-passing litmus on two vCPUs

```yaml
---
id: 009
status: closed
phase: 2
pass_kind: observation
---
```

## scope

Author `probes/mac_hv_litmus/mp_litmus.c`: a Hypervisor.framework VM (4 KB IPA granule) with two vCPUs on two host threads, guest MMU on (identity map, 1 GB level-1 block, Normal write-back inner-shareable memory) so weak ordering is possible. Writer guest: `x[i]=1; y[i]=1` for i < N. Reader guest: spin until `y[i]==1`, then read `x[i]`; count `x[i]==0` (the MP forbidden outcome under x86-TSO). Rounds alternate EnTSO off and on (ACTLR_EL1 bit 1 on both vCPUs), with ACTLR read back after every vCPU exit. Author `scripts/litmus_mp.py`: build, sign, run, and record CAPABILITY_PROBED(guest_tsoen) and CAPABILITY_PROBED(guest_tsoen_context_switch) with the counts, on a substrate record.

## prerequisites

- 008

## context_files

- `probes/mac_hv_capability/hv_capability.c` (VM bring-up that worked)
- SDK `hv_vcpu.h` (hv_vcpu_run exits; `hv_vcpu_set_vtimer_mask`), `hv_vcpu_types.h` (TTBR0_EL1, TCR_EL1, MAIR_EL1, SCTLR_EL1)
- Arm ARM: SCTLR_EL1.M=0 makes EL1 data accesses Device-nGnRnE; MP litmus (herdtools7 `MP` test) allowed on AArch64 without barriers, forbidden under x86-TSO
- v0.2 §9

## guest configuration (bridge mapping)

- TCR_EL1 = T0SZ 25 (39-bit VA, start level 1), IRGN0/ORGN0 = 01 (WB WA), SH0 = 11 (inner shareable), TG0 = 00 (4 KB), EPD1 = 1, IPS = 001 (36-bit).
- MAIR_EL1 attr0 = 0xFF (Normal WB). Level-1 entry 0: block 0..1 GB, AttrIndx 0, SH 11, AF 1 → 0x701.
- SCTLR_EL1 = 0x30D01805 (M, C, I, plus RES1 bits 11, 20, 22, 23, 28, 29).
- Code at IPA 0x10000000; table at 0x30000000; x at 0x20000000; y at 0x20800000.

## signal contract

### Emits

- `SESSION_INIT`, `CAPABILITY_PROBED` x2, `SESSION_COMPLETED`.

## artifact contract

### Files created

- `probes/mac_hv_litmus/mp_litmus.c`
- `scripts/litmus_mp.py`

### Command exit codes

- `uv run python scripts/litmus_mp.py` returns 0
- `uv run pytest`, `uv run ruff check`, `uv run mypy` return 0

## observation contract

### Expected runtime signals

- EnTSO on: forbidden outcome count 0 over every round.
- EnTSO off: forbidden outcome count recorded. If it is > 0, the effect of EnTSO is demonstrated. If it is 0, the test lacked power on this host and the EnTSO effect is recorded as not demonstrated (outcome stays `present` for the control, detail says "effect not demonstrated").
- ACTLR_EL1 reads back as configured after every exit.

## done criteria

The EnTSO bit is shown to change observable memory ordering for guest code on this Mac, or the record says plainly that this test could not show it.

## self-review checklist

- [x] Scope concrete.
- [x] Guest configuration written from the Arm architecture rules, not guessed; failures surface as exception exits with ESR.
- [x] Emits are locked tags.
- [x] Assertions gradable.
- [x] Observation contract present.
- [x] Two files, one concept.
