# SIGNAL_REPORT — Sprint 130 — The host redesigned: no lock across a service

Made by `signal_report 130` from 0 records under store/runs/ whose baseline names one of the sprint's 1 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 130.

## 1. Observed

### scope_confirmation

> Fix (b) of the direction of 2026-10-24 (BLACKBOARD Decision 2026-10-24 user), step 1 of
> `docs/rust-port-plan-2026-10-24.md`. `docs/design-rust-host-2026-10-24.md` replaces the design of 2026-10-23 and
> changes four things, each stated against the goal's figures and the systems that solved it: the guest's runtime word
> guards only the run table, and `w2m_host` gives it up around `hvc #8`; translation goes into a region per vCPU with a
> lock-free lookup and a lock per guest code page for invalidation; one process's objects and waits stay inside its VM,
> and the supervisor holds only what crosses processes; a vCPU spinning on a held word waits after a bounded spin, by
> the futex protocol's three states. Its acceptance is the goal's figures, measured at the plan's steps, not the count
> of the Python host's attributes.

### work_performed

- 3a3c7dff5 Sprint 130: the host redesigned, no lock across a service

### signal_trace

```
(no record)
```

### signal_trace_tags


## 2. Expected

> None. A document.

## 3. Delta

### emits_check

- the card's Emits names no tag, and the sprint made no record

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 130: its Rubber Duck Pass and close.
