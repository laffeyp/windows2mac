# SIGNAL_REPORT — Sprint 146 — The virtual memory arena: VirtualAlloc, VirtualFree, VirtualProtect and VirtualQuery in Rust

Made by `signal_report 146` from 0 records under store/runs/ whose baseline names one of the sprint's 1 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 146.

## 1. Observed

### scope_confirmation

> The case runner's fault path (`docs/design-host-compile-path-2026-10-24.md`, link 3) needs the host to answer the
> guest runtime's exception walk. With no handler, the walk asks for the x86 stubs' page (`HOST_EXCEPTION`, `exceptions.c`
> `known_code` 142-148), and the Python host writes that page into the VirtualAlloc arena (`vm_backend.py` `_code_page`,
> `_guest_stubs`). The arena comes first, as a crate of its own: `src/w2m/virtual_memory.py` (352 lines, read in full)
> carried whole into `crates/virtual_memory`:

### work_performed

- 90d43728a Sprint 146: the virtual memory arena in Rust

### signal_trace

```
(no record)
```

### signal_trace_tags


## 2. Expected

> None yet: the record comes at the skeleton's gate.

## 3. Delta

### emits_check

- the card's Emits names no tag, and the sprint made no record

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 146: its Rubber Duck Pass and close.
