# SIGNAL_REPORT — Sprint 294 — The scheduling exits under the runtime word

Made by `signal_report 294` from 0 records under store/runs/ whose baseline names one of the sprint's 3 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 294.

## 1. Observed

### scope_confirmation

> The first card of `docs/design-run-loop-on-several-vcpus-2026-10-31.md`. The switch stub releases the runtime word
> before `hvc #12` (`guest.rs` `switch_stub`: `unlock_runtime`, then `hvc(HVC_SCHEDULE)`), and a slice's `hvc #4` comes
> from a translated block, so both reach the host without the word; their services change `threads` and the turn
> (`threads::schedule`, `set_budget`, `wake_expired`, `hand_over`), which another vCPU's services change under it.
> `hvc #13`'s service changes the host compiler's tables, and whether its vCPU holds the word is not established. The
> miss service, `hvc #1`, takes the word itself (Sprint 286).

### work_performed

- 6458bbe12 Sprint 294: the scheduler checks it runs under the runtime word
- ca7f60bf2 Sprint 294: the scheduling exits under the runtime word
- baedc0ff9 Sprint 294 card: the scheduling exits under the runtime word

### signal_trace

```
(no record)
```

### signal_trace_tags


## 2. Expected

> - None; a lock's coverage, held by tests.

## 3. Delta

### emits_check

- the card's Emits names no tag, and the sprint made no record

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 294: its Rubber Duck Pass and close.
