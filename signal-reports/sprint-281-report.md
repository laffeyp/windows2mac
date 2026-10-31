# SIGNAL_REPORT — Sprint 281 — The guest's exit-time services reach memory through the view

Made by `signal_report 281` from 0 records under store/runs/ whose baseline names one of the sprint's 3 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 281.

## 1. Observed

### scope_confirmation

> Census row D7's third module (the step 6 sequence Decision, annotated at 278). `crates/w2m_vm/src/guest.rs` holds 20
> slice calls, none in its tests (which start at line 1488). Four run at load, before any vCPU exists, which the design's
> choice 1 allows to keep their slices: `Guest::new` (807), `with_trace` (969, 988) and `com_vtables` (1007). The other
> 16 run at the guest's exits, while on two vCPUs another may run: `place_cache_file`, `page_descriptor`, `map_pages`,
> `called_service`, `trace_owner`, `trace_ranges` (three), `traced_proc` (two), `traced_strings`, `trace_count`,
> `trace_record`, `init_stub`, `exception` and `clear_exception`.

### work_performed

- 6972ba75c Sprint 281: the XMM capture fills its registers by fixed-size chunks, as the lint asks
- 45382e739 Sprint 281: the guest's exit-time services reach memory through the view
- 9f78f6b5a Sprint 281 card: the guest's exit-time services reach memory through the view; reviewer-3's review at 280 dispositioned

### signal_trace

```
(no record)
```

### signal_trace_tags


## 2. Expected

> - None; an access path, held by tests and the one-vCPU runs.

## 3. Delta

### emits_check

- the card's Emits names no tag, and the sprint made no record

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 281: its Rubber Duck Pass and close.
