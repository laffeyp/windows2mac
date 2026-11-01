# SIGNAL_REPORT — Sprint 306 — One VM at a time under cargo test

Made by `signal_report 306` from 0 records under store/runs/ whose baseline names one of the sprint's 2 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 306.

## 1. Observed

### scope_confirmation

> Reviewer-3's Low 1 at 303: on this Mac `cargo test -p w2m_vm --test all` ran 72 passed and 8 failed at `Vm::new`
> (Sprint 303), Hypervisor.framework allowing one VM a process and `cargo test` running a binary's tests as threads of
> one; WORKING_AGREEMENT 142 records the failure instead of removing it. Eight test files make a VM in-process:
> `run_loop.rs` and `litmus.rs` each hold a lock of their own (`ONE_VM`), which serialises their own tests and not one
> file's against another's; `bridges.rs`, `cpuid_in_vm.rs`, `dispatch_hash_planted.rs`, `first_decline.rs`,
> `monitor_code_in_vm.rs` and `unmapped_image_faults.rs` hold none.

### work_performed

- d0464e0c2 Sprint 306: one VM at a time under cargo test
- dc6dccbef Sprint 306 card: one VM at a time under cargo test

### signal_trace

```
(no record)
```

### signal_trace_tags


## 2. Expected

> - None; the tests' setup.

## 3. Delta

### emits_check

- the card's Emits names no tag, and the sprint made no record

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 306: its Rubber Duck Pass and close.
