# SIGNAL_REPORT — Sprint 278 — The scheduler reaches guest memory through the view

Made by `signal_report 278` from 0 records under store/runs/ whose baseline names one of the sprint's 2 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 278.

## 1. Observed

### scope_confirmation

> Census row G4 (`docs/step-6-census-2026-10-30.md`) needs the run loop to drive a second vCPU of one process. The host
> design (`docs/design-rust-host-2026-10-24.md` 12-23) gives each vCPU a native thread that runs it and serves its own
> exits, with no lock across a service, so several exits are served at once while other vCPUs run. Sprint 276's design
> (`docs/design-guest-memory-on-several-vcpus-2026-10-31.md`) allows a slice of guest memory only while every vCPU that
> may write it is stopped; served exits at once leave no such moment, so the run loop's 174 slice calls in
> `crates/w2m_vm/src/` (census D7) move to `ConcurrentGuestMemory` before a second vCPU runs. This card moves the
> largest set, the scheduler's.

### work_performed

- 01188b254 Sprint 278: the scheduler reaches guest memory through the view
- 3c71dedf8 Sprint 278 card: the scheduler reaches guest memory through the view

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

See the BLACKBOARD tail entry for Sprint 278: its Rubber Duck Pass and close.
