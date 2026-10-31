# SIGNAL_REPORT — Sprint 265 — The cache key over its own inputs

Made by `signal_report 265` from 0 records under store/runs/ whose baseline names one of the sprint's 2 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 265.

## 1. Observed

### scope_confirmation

> The translation cache's key hashes the whole build's inputs (`built_from::crates_tree`: `crates/`, the manifests, the
> toolchain, `specs/`), so any edit under `crates/`, a test's included, names a cache no file holds: each sprint remakes
> it, and in Sprint 263 an edit to `run_loop.rs` while records ran voided a batch (Observation 4). This card makes the key
> hash what the cached words are made from and nothing else, held by a test against the crates' own manifests.

### work_performed

- 40026f382 Sprint 265: the cache key hashes what the cached words are made from
- 5e91de411 Sprint 265 card: the cache key over its own inputs

### signal_trace

```
(no record)
```

### signal_trace_tags


## 2. Expected

> - None; a key's inputs, held by a test.

## 3. Delta

### emits_check

- the card's Emits names no tag, and the sprint made no record

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 265: its Rubber Duck Pass and close.
