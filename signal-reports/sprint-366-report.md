# SIGNAL_REPORT — Sprint 366 — Host requests off the runtime word (C2): the design

Made by `signal_report 366` from 0 records under store/runs/ whose baseline names one of the sprint's 2 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 366.

## 1. Observed

### scope_confirmation

> The Decision "Step 6's order weighed again at Sprint 360" puts C2 first after G2, and G2 is met (the Decision "G2 met
> at Sprint 364"). C2 is the host design's Change 2 (`docs/design-rust-host-2026-10-24.md` 155-170): "a host request
> from the guest holds no guest lock, on its own vCPU's page". Today `w2m_host` (`tools/guest_runtime/runtime.c` 25-35)
> writes one transport page for the whole process and executes `hvc #8`, and "the transport page is every vCPU's"
> (`runtime.c` 60), so every host request is made with the runtime word held: 67 call sites in 14 files at 7ca67b25a
> (file.c 22, sync_objects.c 8, processes.c 7, module.c 7, exceptions.c 5, virtual_memory.c 4, process.c 3, and seven
> files with one or two). This card is the reading Change 2 asks for: "Each of the 65 call sites is read for state it
> holds across the request". It is written as a design, `docs/design-host-requests-off-the-runtime-word-2026-11-26.md`,
> that passes `design_check`.

### work_performed

- 5fd4d381d Sprint 366 card amendment: the guest layout's path, crates/guest_layout/src/lib.rs
- 620bdba44 Sprint 366 card: host requests off the runtime word (C2), the design

### signal_trace

```
(no record)
```

### signal_trace_tags


## 2. Expected

> - None: a design card with no code.

## 3. Delta

### emits_check

- the card's Emits names no tag, and the sprint made no record

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 366: its Rubber Duck Pass and close.
