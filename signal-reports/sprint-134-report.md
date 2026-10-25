# SIGNAL_REPORT — Sprint 134 — The runtime runs in a Rust VM

Made by `signal_report 134` from 0 records under store/runs/ whose baseline names one of the sprint's 1 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 134.

## 1. Observed

### scope_confirmation

> The walking skeleton's third link. A Rust VM runs a function of the C guest runtime where it was linked, with the
> MMU on: the monitor area (`MONITOR_SIZE` bytes) mapped at `IPA_BASE`; stage-1 tables written for
> `(MONITOR_VA, IPA_BASE, MONITOR_SIZE)` by a port of `vm_backend._write_translation_tables` (`src/w2m/vm_backend.py`
> 2204-2242), 4 levels, 2 MB blocks where aligned, the descriptor bits `PAGE_ATTRIBUTES` (352); the runtime image
> (Sprint 133) copied to `RUNTIME_IMAGE`; the system registers the Python host sets (4980-4981: CPACR_EL1 `CPACR_FPEN`,
> MAIR_EL1 0xFF, TCR_EL1 `TCR`, TTBR0_EL1 the root, SCTLR_EL1 `SCTLR`, ACTLR_EL1 `ACTLR_ENTSO`); vectors that hand any
> exception to the host by `hvc`, so a fault is seen, not lost. The vCPU calls `w2m_cpuid_query(leaf, sub, r)`
> (`tools/guest_runtime/runtime.c` 119) with the link register at a stub that executes `hvc`, once for each input
> `tests/test_vm_state.py` 203-205 lists: every leaf of `specs/guest_cpuid.json`, then (1, 5), (7, 1), (8, 0),
> (0x40000000, 0), (0x80000009, 0), (0xFFFFFFFF, 0). Each answer must equal `cpuid.lookup`'s (`src/w2m/cpuid.py`
> 30-41, carried as the judge), as that test asserted for the Python host's VM.

### work_performed

- 87719b0a2 Sprint 134: the runtime runs in a Rust VM

### signal_trace

```
(no record)
```

### signal_trace_tags


## 2. Expected

> None yet: the skeleton's record comes with the guest program (a later link).

## 3. Delta

### emits_check

- the card's Emits names no tag, and the sprint made no record

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 134: its Rubber Duck Pass and close.
