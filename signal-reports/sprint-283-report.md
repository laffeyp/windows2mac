# SIGNAL_REPORT — Sprint 283 — The host compiler reaches guest memory through the view

Made by `signal_report 283` from 0 records under store/runs/ whose baseline names one of the sprint's 2 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 283.

## 1. Observed

### scope_confirmation

> Census row D7's fifth module (the step 6 sequence Decision, annotated at 278, 281 and 282). `crates/w2m_vm/src/compile.rs`
> holds 12 slice calls and no tests of its own, and three functions take a slice: `page_set_add`, `page_set_remove`, and
> `installed_in_vm`, which `run.rs`' `vm_compiled` feeds, so `vm_compiled` holds a slice and builds two views from it.

### work_performed

- b3162737f Sprint 283: the host compiler reaches guest memory through the view
- 07c662f9a Sprint 283 card: the host compiler reaches guest memory through the view

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

See the BLACKBOARD tail entry for Sprint 283: its Rubber Duck Pass and close.
