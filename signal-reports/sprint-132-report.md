# SIGNAL_REPORT — Sprint 132 — A vCPU in Rust

Made by `signal_report 132` from 0 records under store/runs/ whose baseline names one of the sprint's 1 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 132.

## 1. Observed

### scope_confirmation

> The walking skeleton's first link (step 3 of `docs/rust-port-plan-2026-10-24.md`). A crate, `hypervisor`, binds the
> Hypervisor.framework calls a one-vCPU host needs, each declaration copied from the macOS 26.5 SDK's headers, and wraps
> them in two owned types: `Vm` (one per process, destroyed on drop) and `Vcpu` (created on the thread that runs it,
> not `Send`, destroyed on drop on that thread). A test maps anonymous memory into a VM, places a few arm64
> instructions in it, runs them on one vCPU at EL1 with the MMU off, and reads the exits: an `hvc #8` with x0 = 42 and
> the PC after the `hvc` (the bridge mapping's measured fact, WORKING_AGREEMENT), then a resumed run to a second `hvc`.
> It also measures the time of an exit and its resume on this Mac against the C monitor's 0.72 µs, and a run stopped
> from another thread by `hv_vcpus_exit` returns as canceled. Tests that create a VM need the
> `com.apple.security.hypervisor` entitlement: a cargo runner signs each test and binary ad hoc before running it, as
> `vm_backend.py` signs `vm_monitor`. On Linux (CI's runner) the crate compiles to nothing and its tests do not run.

### work_performed

- bd79d9ee5 Sprint 132: a vCPU in Rust

### signal_trace

```
(no record)
```

### signal_trace_tags


## 2. Expected

> None. The crate and its tests put nothing on a record; the skeleton's record comes with the guest (a later link).

## 3. Delta

### emits_check

- the card's Emits names no tag, and the sprint made no record

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 132: its Rubber Duck Pass and close.
