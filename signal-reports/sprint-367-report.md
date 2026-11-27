# SIGNAL_REPORT — Sprint 367 — The batch run's stop on two vCPUs, on the wall clock

Made by `signal_report 367` from 0 records under store/runs/ whose baseline names one of the sprint's 2 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 367.

## 1. Observed

### scope_confirmation

> The Decision "Step 6's order after G2, weighed again at Sprint 366" puts the batch run's stop on two vCPUs first, so
> that G3 (Sprint 368: the game's batch run on two vCPUs, several runs, conforming as on one) can judge C2, C5 and C3
> before they are built. Today the batch run cannot run on two vCPUs: `w2m-vm --game` takes the instruction clock
> (`crates/w2m_vm/src/bin/w2m-vm.rs` 345-348, 439-441: "The game's start keeps 1"), which the run loop refuses above one
> vCPU since Sprint 365; and the batch run's stop (`crates/w2m_vm/src/batch_stop.rs` 1-17) is "points on the
> instruction clock": it is asked after each host request, and its fallback `REACH_AFTER_LOG` counts instructions since
> the game's log reached the reference's 168 lines.

### work_performed

- b358c5588 Sprint 367 card amendment: every C2 card built, the measurement setting only their order (the user's fix (b)); the frozen host's run_frontier.py read from git
- 5ea26ac89 Sprint 367 card: the batch run's stop on two vCPUs, on the wall clock

### signal_trace

```
(no record)
```

### signal_trace_tags


## 2. Expected

> - None: where the batch run stops on two vCPUs; no emit site changes. The one-vCPU batch run's records are unchanged.

## 3. Delta

### emits_check

- the card's Emits names no tag, and the sprint made no record

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 367: its Rubber Duck Pass and close.
