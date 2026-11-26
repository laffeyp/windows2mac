# SIGNAL_REPORT — Sprint 357 — Vocabulary 0.14: each lock's takings, hold and wait

Made by `signal_report 357` from 0 records under store/runs/ whose baseline names one of the sprint's 1 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 357.

## 1. Observed

### scope_confirmation

> The Decision "The vocabulary for each lock's hold and wait" (2026-11-25, reviewer-4's fourteenth review, Medium 1)
> puts a vocabulary card, 0.14, before G6 and before the first card that measures a lock other than the runtime word.
> Sprint 356's Decision "A card that moves a service class between locks measures the runtime word" makes it the next
> card. Change 1 has now built four leaf locks: the heap (343), the TLS and FLS slots (345), the handle table (355) and
> the sync objects (356). Vocabulary 0.13 (`signals/rust-current.json`) has no word for any of them. It names
> `runtime_word_hold_ns_*` and `runtime_word_wait_ns_*` alone, `holder` for the word alone, and E15 forbids `holder`
> elsewhere. Its records count a lock's takings only inside a `variant` string (Sprint 356's Rubber Duck Pass,
> observation 2).

### work_performed

- 2e27ecee0 Sprint 357 card: vocabulary 0.14, each lock's takings, hold and wait

### signal_trace

```
(no record)
```

### signal_trace_tags


## 2. Expected

> - None; the card adds no emit site. A vocabulary card's new words are carried by the samples record
>   (`harness/vocabulary-samples.toml`, run in the gates' samples stage), which is not kept under `store/runs/` (Sprint
>   336's annotation), so the Signal Report cannot read them. Their evidence is the observation contract's: the samples
>   stage's measurement lines, with each of the six new `metric` values once and each `lock` value once on a `lock_*`
>   metric.
> - *(Annotated 2026-11-26 at the close, by the Supervisor: the card first named the measurement tag here, from the
>   samples record. Since Sprint 340 the record check fails a declared tag the Signal Report shows NOT FIRED, and the
>   report reads `store/runs/` alone; a kept samples record breaks the records check (Sprint 336's annotation). The first
>   form was the card's error, not the build's; the samples' counts are in the result.)*

## 3. Delta

### emits_check

- the card's Emits names no tag, and the sprint made no record

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 357: its Rubber Duck Pass and close.
