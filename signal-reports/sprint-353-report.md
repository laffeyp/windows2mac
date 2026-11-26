# SIGNAL_REPORT — Sprint 353 — Every per-vCPU word the host writes, checked against run-wide state, and slicing past vCPU 0 judged

Made by `signal_report 353` from 0 records under store/runs/ whose baseline names one of the sprint's 1 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 353.

## 1. Observed

### scope_confirmation

> Reviewer-4's twenty-first review, Medium 1, recorded as the Decision "The served-vCPU words, enumerated in card 353".
> Sprint 351 found that a thread taken by vCPU 1 ran on the run's whole step budget and was never put down. `set_budget`
> (`crates/w2m_vm/src/threads.rs` 1328-1341) computes `shared` from run-wide state,
> `guest.served.threads.len() > 1 || guest.served.link.is_some()`, and writes the limit only at
> `guest.served.vcpu.monitor + 8`. Its callers are `threads.rs` 1322, `host_request.rs` 545 and `run.rs` 1358 and 1877.
> Sprint 351 added a call in `take_a_thread`. That defect names a class: a per-vCPU word whose right value depends on
> state another vCPU's service can change, written only for the vCPU served when that state changed.

### work_performed

- 97bb2578a Sprint 353 card: every per-vCPU word the host writes, checked against run-wide state, and slicing past vCPU 0 judged

### signal_trace

```
(no record)
```

### signal_trace_tags


## 2. Expected

> - None; which per-vCPU words the host writes, and when.

## 3. Delta

### emits_check

- the card's Emits names no tag, and the sprint made no record

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 353: its Rubber Duck Pass and close.
