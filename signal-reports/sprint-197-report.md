# SIGNAL_REPORT — Sprint 197 — The second process

Made by `signal_report 197` from 0 records under store/runs/ whose baseline names one of the sprint's 2 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 197.

## 1. Observed

### scope_confirmation

> The Decision "Sequence after Sprint 196" (reviewer-2's Highs 1 and Mediums 1 of its review of Sprints 193-196):
> kernel.Machine in Rust. The batch run stops at row 16, UnityPlayer's CreateProcessW of UnityCrashHandler64.exe, and six
> guests stop at their PROCESS_CREATE (busychild, crosswake, handshake, pingpong, spinchild, twoproc). Hypervisor.framework
> allows one VM per process (kernel.py 5-6), so the child is a second `w2m-vm` process; the processes take turns, one at a
> time, on one machine clock, as the Python's do. No child that never runs is faked.

### work_performed

- 1646688eb Sprint 197: the second process
- 1fe3f982a Sprint 197 card: the second process

### signal_trace

```
(no record)
```

### signal_trace_tags


## 2. Expected

> - None: the judges are `batch_run.rs` and unit tests.

## 3. Delta

### emits_check

- the card's Emits names no tag, and the sprint made no record

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 197: its Rubber Duck Pass and close.
