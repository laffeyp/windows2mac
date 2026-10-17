# Sprint 103 — several vCPUs: the per-vCPU state and two designs

```yaml
---
id: 103
status: closed
phase: 5
pass_kind: architecture
---
```

## scope

Roadmap step 1 (docs/plan.md, Decision 2026-10-17) starts with its design, as the review of 2026-10-17 asks
(design it twice, before the first line). `docs/design-several-vcpus.md`: what Apple and Arm document that binds
the design (one vCPU per host thread; the GIC and the topology fixed before vCPUs run; the IS forms of TLBI and IC
for invalidation on every vCPU; CnP = 0 for translation entries that differ between vCPUs; x86-TSO's allowed
litmus outcomes), the inventory of per-thread, per-vCPU and per-process state measured in the tree, design A (each
vCPU's state at its own address) against design B (one address, a private page on each vCPU through its own
translation root), the choice and the order of the sprints after.

## prerequisites

- 102.

## context_files

- tools/vm_monitor/vm_monitor.c; src/w2m/vm_backend.py (the monitor data page's layout, the page-table writer, the
  exception entry, the dispatcher); tools/guest_runtime/runtime.h; the SDK's hv_vcpu.h and hv_gic.h;
  probes/mac_hv_litmus/mp_litmus.c.

## signal contract

### Emits

- None: a design sprint; no run.

### Invariants

- Vocabulary unchanged.

## artifact contract

- docs/design-several-vcpus.md with each documented fact quoted and sourced, the inventory's counts reproducible by
  the commands in this card's observed section, both designs and a choice with grounds; a BLACKBOARD Decision.

## observation contract

- None: no product behavior changes. Sprint 104's judges (the litmus outcomes) test the choice.

## done criteria

The design is written, sourced and chosen; Sprints 104 and 105 are carded from it; CI green.

## observed

- **Made:** docs/design-several-vcpus.md; BLACKBOARD Decision 2026-10-17 (design B).
- **Inventory, reproducible at ca5cb64b:** `grep -cF "MONITOR_VA + MONITOR_DATA" src/w2m/vm_backend.py` 8;
  `"PROCESS_VA + STATE_OFF"` 9; `"MONITOR_VA + RUNTIME_STACK"` 9; x28-relative `a64.(ldr|str_|ldp|stp|ldr_v|str_v)(.., ..,
  28,` 28; in tools/guest_runtime/*.c and *.h, W2M_MONITOR_DATA_VA 12, W2M_STATE_VA 9, W2M_SAVE_VA 1,
  W2M_EXCEPTION_VA 2, W2M_RUN_TABLE 2.
- **Sources quoted:** hv_vcpu.h in the macOS SDK (lines 20-36); Arm's TTBR0_EL1 (CnP) and TLBI VMALLE1IS
  descriptions; Owens, Sarkar and Sewell 2009 for x86-TSO's outcomes.
