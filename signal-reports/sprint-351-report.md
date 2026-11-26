# SIGNAL_REPORT — Sprint 351 — The 273 family runs on two vCPUs, held to Windows' rows

Made by `signal_report 351` from 0 records under store/runs/ whose baseline names one of the sprint's 1 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 351.

## 1. Observed

### scope_confirmation

> Reviewer-4's twentieth review, High 1, recorded as the Decision "The family runs on two vCPUs before the lock cards".
> G2 in `docs/step-6-census-2026-10-30.md` asks for the 68 Windows function families on two vCPUs, 273 runs, each held to
> Windows' rows. Today `scripts/gates.sh`' `family_runs()` runs each run of `store/family_runs/known-2026-10-29.tsv`
> through `w2m-vm` on one vCPU. `crates/w2m_vm/tests/family_runs.rs` then holds its printed rows to its contract's rows,
> or to `store/family_runs/sweep-held-2026-10-29.tsv`. Neither takes a vCPU count.

### work_performed

- 71889e1d7 Sprint 351 card: the 273 family runs on two vCPUs, held to Windows' rows; README's Status brought to the record

### signal_trace

```
(no record)
```

### signal_trace_tags


## 2. Expected

> - None; the family runs' judge on two vCPUs.

## 3. Delta

### emits_check

- the card's Emits names no tag, and the sprint made no record

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 351: its Rubber Duck Pass and close.
