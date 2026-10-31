# SIGNAL_REPORT — Sprint 274 — Two vCPUs judged by the litmus tests

Made by `signal_report 274` from 0 records under store/runs/ whose baseline names one of the sprint's 3 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 274.

## 1. Observed

### scope_confirmation

> The Decision "Sequence for step 6" (as annotated at 271): after the XCHG (272) and the gates' base (273), the Rust
> host runs several vCPUs at once for the first time, judged first by the litmus tests MP, SB, LB, 2+2W and IRIW and
> their EnTSO plant (census row G1), before any lock is split. The frozen host's judge, `tests/test_litmus.py` at
> `frozen-python-2026-10-29` (Sprint 105), needs no guest process: x86 assembled by clang, compiled by w2m, each thread on
> a vCPU of one VM against shared words. This card ports it to the Rust host, with reviewer-3's Low 1 at 272: a
> misaligned memory XCHG raced on two vCPUs.

### work_performed

- f4e7affe2 Sprint 274: census row G1 met, the litmus tests on the Rust host's vCPUs
- a892fabe6 Sprint 274: the litmus tests on several vCPUs of the Rust host
- 0f8b8a7a8 Sprint 274 card: two vCPUs judged by the litmus tests

### signal_trace

```
(no record)
```

### signal_trace_tags


## 2. Expected

> - None; the memory model on several vCPUs, held by tests.

## 3. Delta

### emits_check

- the card's Emits names no tag, and the sprint made no record

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 274: its Rubber Duck Pass and close.
