# SIGNAL_REPORT — Sprint 325 — A machine is refused on several vCPUs

Made by `signal_report 325` from 0 records under store/runs/ whose baseline names one of the sprint's 2 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 325.

## 1. Observed

### scope_confirmation

> The Decision "Architecture: a vCPU with no thread" (the run-loop design's "The Machine", Sprint 318): until the Machine
> has its own thread (card 4f, the Python host's Sprint 122), a process with a machine link is refused on more than one
> vCPU, by name, when its link is made: `host_request.rs` 524-530, where a `HOST_KERNEL` spawn makes the supervisor's
> link mid-run. `w2m-vm` reads `--vcpus` and hands the count to `vcpus_allowed` alone (`w2m-vm.rs` 300-320, 389); no
> part of the run knows it. A child process is refused at the start already (`vcpus_allowed`'s `child`, Sprint 308);
> a parent that spawns is not, and card 4c3b2b, the threads, lifts "a second vCPU is not built". This card comes
> before it.

### work_performed

- cdae07b69 Sprint 325: a machine is refused on several vCPUs
- 69dd8def0 Sprint 325 card: a machine is refused on several vCPUs

### signal_trace

```
(no record)
```

### signal_trace_tags


## 2. Expected

> - None; a refusal by name in the run's stop.

## 3. Delta

### emits_check

- the card's Emits names no tag, and the sprint made no record

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 325: its Rubber Duck Pass and close.
