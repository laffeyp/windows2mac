# SIGNAL_REPORT — Sprint 194 — The stream judge's fixes

Made by `signal_report 194` from 0 records under store/runs/ whose baseline names one of the sprint's 2 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 194.

## 1. Observed

### scope_confirmation

> The Decision "Sequence after Sprint 193", reviewer-2's Mediums 2-3 and Lows 1 and 3 of its review of Sprints 190-192.
> `batch_run.rs` holds every cell of the Rust stream equal to the Python's, but three repeat Python runs of the same code
> differ from the reference at rows 426 and 438 (GetFileAttributesExW, columns a4-a7), so a correct Rust host would fail
> there; the call in flight at the stop is dropped and never compared; and the build digest's inputs miss
> `specs/x86_decode.json`, which `crates/x86_decoder/build.rs` 98 reads.

### work_performed

- 086fcb47d Sprint 194: the stream judge's fixes
- a553ed1fa Sprint 194 card: the stream judge's fixes

### signal_trace

```
(no record)
```

### signal_trace_tags


## 2. Expected

> - None: no producer runs; the judges are `batch_run.rs` and unit tests.

## 3. Delta

### emits_check

- the card's Emits names no tag, and the sprint made no record

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 194: its Rubber Duck Pass and close.
