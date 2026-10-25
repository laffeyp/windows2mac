# SIGNAL_REPORT — Sprint 141 — The bridges: an import's call reaches its service in the VM

Made by `signal_report 141` from 0 records under store/runs/ whose baseline names one of the sprint's 1 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 141.

## 1. Observed

### scope_confirmation

> The run loop's second piece (`docs/design-host-compile-path-2026-10-24.md`, link 1, split by technique #17 after
> Sprint 140; Sprint 142 runs `qpc` to its first declined block). The Rust host lays a program's guest memory out as
> `VM.__init__` does (`src/w2m/vm_backend.py` 2382-2424: the monitor area, the process area, each loaded region, the
> heap, the VirtualAlloc arena; one memory, one set of tables), writes the process area (Sprint 140) and the monitor
> code (Sprint 136), and writes one bridge per import thunk (`_bridge`, 3733-3794): save the guest's x86 registers in the
> state block, pass RCX, RDX, R8, R9 and RSP to the C service on the runtime stack under the runtime lock, put its result
> in RAX unless the service is void, restore the registers, do RET's pop, and branch to the dispatcher; an import with
> no service is `hvc #9`; a RETRY or a schedule bit goes to the switch stub (`_switch_stub`, 2650-) . Each bridge's
> address goes into the recovery table (`_recon_append`, 4850-). The service table is Sprint 133's
> (`guest_runtime::services`), and `service_for` (2302-2315) picks a thunk's service by DLL and name.

### work_performed

- 623133a4d Sprint 141: the bridges

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

See the BLACKBOARD tail entry for Sprint 141: its Rubber Duck Pass and close.
