# SIGNAL_REPORT — Sprint 331 — Every other vCPU stopped for a locked model step

Made by `signal_report 331` from 0 records under store/runs/ whose baseline names one of the sprint's 3 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 331.

## 1. Observed

### scope_confirmation

> The run-loop design's card 4d, split by rule 6: 4d1, here, the stop of every other vCPU for a model step of a form
> x86 locks (the design's "A model step of a locked form", precondition 8); 4d2, the stop for a block split, a cancel
> from another thread mid-split (Deferred, Sprint 305) and the guest Deferred (Sprint 330) names for the kick's wait.
> Today a misaligned locked form runs in the model (`hvc #11`, `crates/translator/src/lower.rs` 1424-1432) as a load
> and a store, under the runtime word and the lock (`run.rs` 1696-1716, Sprint 324), so two misaligned steps on two
> vCPUs already run one at a time; an aligned locked form runs in the VM as an exclusive pair, which takes no word, so
> an aligned add on another vCPU to a byte the step covers, made between the step's load and its store, is written
> back over; x86 makes each access atomic at any alignment (Deferred, Sprint 272; the design's "A model step of a
> locked form": "The word would not exclude the aligned pair").

### work_performed

- c303095ad Sprint 331: every other vCPU stopped for a locked model step
- 34d6c9abc Sprint 331: the locked_race probe, a misaligned locked add against an aligned one sharing a byte
- 558e05bc1 Sprint 331 card: every other vCPU stopped for a locked model step

### signal_trace

```
(no record)
```

### signal_trace_tags


## 2. Expected

> - None; when a locked model step stops the other vCPUs.

## 3. Delta

### emits_check

- the card's Emits names no tag, and the sprint made no record

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 331: its Rubber Duck Pass and close.
