# SIGNAL_REPORT — Sprint 314 — The entries made before a second vCPU

Made by `signal_report 314` from 0 records under store/runs/ whose baseline names one of the sprint's 2 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 314.

## 1. Observed

### scope_confirmation

> Reviewer-3's Medium 1 at 312 (BLACKBOARD Surfaced) on the per-vCPU pages design
> (`docs/design-each-vcpus-own-pages-2026-10-31.md`): a private chain's level-0, level-1 and level-2 tables are copies,
> and an entry the shared tables gain after a vCPU is made never reaches its copy. The disposition chose the reviewer's
> (a): every upper-table entry a run-time mapping can write under level-0 entries 0xFE and 0xFF is made at load, before
> any second vCPU, and the descriptor writer refuses a new one after. Which entries those are is a fact of the code, read
> before the build card (card 4c2b, Sprint 315) depends on it. Under 0xFF lie the system DLLs (`DLL_BASE`, level-1 0x1E0
> on), the TLS slots (`TLS_VA`), the loader stub (`LOADER_STUB`) and the thunks with the process area (`THUNK_BASE` =
> `PROCESS_VA`, 0x1F8); under 0xFE the monitor area.

### work_performed

- f0268fe9d Sprint 314: the copied tables after load, read from their writers
- b4341601d Sprint 314 card: the entries made before a second vCPU

### signal_trace

```
(no record)
```

### signal_trace_tags


## 2. Expected

> - None; a design amendment.

## 3. Delta

### emits_check

- the card's Emits names no tag, and the sprint made no record

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 314: its Rubber Duck Pass and close.
