# SIGNAL_REPORT — Sprint 260 — The in-VM install's design

Made by `signal_report 260` from 0 records under store/runs/ whose baseline names one of the sprint's 3 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 260.

## 1. Observed

### scope_confirmation

> Card 5 of `docs/design-translation-cache-2026-10-30.md`, the install inside the VM, cannot be written from the design
> as it stands: it names what the install does (the copy and the re-encoding in the miss stub's path, no exit; by range
> and by page, Sprint 259) and not where the cached words live in guest memory, what code copies them, how the host
> learns of an install, or how the run judges one. This card writes those choices into the design, `design_check`
> passing and a review before code, as Sprints 251 and 256 did for cards 1 to 4.

### work_performed

- d60e3250e Sprint 260: choice I sized against the guest's memory
- 3056a992c Sprint 260: card 5's build in the design
- 6ec632e34 Sprint 260 card: the in-VM install's design

### signal_trace

```
(no record)
```

### signal_trace_tags


## 2. Expected

> - None; a design.

## 3. Delta

### emits_check

- the card's Emits names no tag, and the sprint made no record

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 260: its Rubber Duck Pass and close.
