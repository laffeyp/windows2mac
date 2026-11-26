# SIGNAL_REPORT — Sprint 337 — A two-vCPU run on a substrate record

Made by `signal_report 337` from 0 records under store/runs/ whose baseline names one of the sprint's 1 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 337.

## 1. Observed

### scope_confirmation

> Plan step 6's evidence is meant to be "on a record" (`docs/rust-port-plan-2026-10-24.md` 58; reviewer-4's first review
> at 331, High 1). Today the two-vCPU runs are judged only by what tests read from `w2m-vm`'s stderr and stop files. One
> example is `a_locked_model_step_loses_no_add_of_another_vcpus_aligned_lock` in `crates/w2m_vm/tests/run_loop.rs`
> (1201-1223), which reads "w2m-vm: 100000 model steps of a locked form". No substrate record carries a run's vCPU count,
> its locked steps, its turn's start, or the capabilities `vcpus_allowed` (`run.rs` 605-633) checks. Vocabulary 0.13
> (Sprint 336) gave those their words. This card puts them on a record, as the Decision "Step 6 restored to the user's fix
> (b)" orders: "`w2m-vm --vcpus N` on a substrate record, CAPABILITY_PROBED emitted where `w2m-vm` makes its refusal
> checks".

### work_performed

- b8e6bbc70 Sprint 337 card: a two-vCPU run on a substrate record

### signal_trace

```
(no record)
```

### signal_trace_tags


## 2. Expected

> - `run_measurements` on its record:
>   - SESSION_INIT;
>   - MEASUREMENT_RECORDED with `metric` `locked_model_steps`, `turn_start_ns_p50` and `turn_start_ns_p99`, each with
>     `vcpus` 2;
>   - CAPABILITY_PROBED with `capability` `feat_lse2`, `feat_bbm_level2` and `guest_wfe_exit`, each with its
>     `outcome` and `detail`;
>   - and, as before, its one-vCPU measurements and SESSION_COMPLETED.

## 3. Delta

### emits_check

- `CAPABILITY_PROBED`: NOT FIRED
- `MEASUREMENT_RECORDED`: NOT FIRED
- `SESSION_COMPLETED`: NOT FIRED
- `SESSION_INIT`: NOT FIRED

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 337: its Rubber Duck Pass and close.
