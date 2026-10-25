# SIGNAL_REPORT — Sprint 143 — The x86 decoder in Rust

Made by `signal_report 143` from 0 records under store/runs/ whose baseline names one of the sprint's 1 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 143.

## 1. Observed

### scope_confirmation

> Link 2 of `docs/design-host-compile-path-2026-10-24.md`: the translator's first part. A crate, `x86_decoder`, carries
> `src/w2m/x86dec.py` (287 lines: `Fields`, `Decoded`, the decode errors, `_pick`, `decode`) on tables generated in Rust
> from `specs/x86_decode.json`. A generator binary in the style of Sprint 126's `vocabulary_generator` writes them, with
> a `--check` mode for the gates, carrying `scripts/gen_decoder.py`'s rules (272 lines). The Python generator stays as
> it is, frozen.

### work_performed

- 08da0af14 Sprint 143: the x86 decoder in Rust

### signal_trace

```
(no record)
```

### signal_trace_tags


## 2. Expected

> None. A decoder; it puts nothing on a record.

## 3. Delta

### emits_check

- the card's Emits names no tag, and the sprint made no record

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 143: its Rubber Duck Pass and close.
