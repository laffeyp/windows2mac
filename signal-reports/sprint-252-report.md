# SIGNAL_REPORT — Sprint 252 — The whole-block judge

Made by `signal_report 252` from 0 records under store/runs/ whose baseline names one of the sprint's 4 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 252.

## 1. Observed

### scope_confirmation

> Card 1 of `docs/design-translation-cache-2026-10-30.md` (choice F): the cache's words will be the Rust translator's,
> and no judge compares whole blocks of the two compilers; `crates/translator/tests/c_words.rs` holds them equal
> instruction by instruction over 1,186,511 calls. This card judges every block the batch run's in-VM compiler makes,
> compiled again by the Rust translator from the same bytes, at the same code address and with the same flags live.

### work_performed

- d1df97078 Sprint 252: every block of the batch run judged, 274,837 equal, in every gates run
- f4d0b5582 Sprint 252: the drive test's names removed on a panic too
- 0a5033ffc Sprint 252: the whole-block judge, w2m-vm --judge-blocks
- 005d4dcfb Sprint 252 card: the whole-block judge

### signal_trace

```
(no record)
```

### signal_trace_tags


## 2. Expected

> - None of its own; the gates' batch run, which must still conform, and the judge's file from one run.

## 3. Delta

### emits_check

- the card's Emits names no tag, and the sprint made no record

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 252: its Rubber Duck Pass and close.
