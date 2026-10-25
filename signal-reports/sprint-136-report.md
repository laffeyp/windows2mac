# SIGNAL_REPORT — Sprint 136 — The monitor code from Rust

Made by `signal_report 136` from 0 records under store/runs/ whose baseline names one of the sprint's 1 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 136.

## 1. Observed

### scope_confirmation

> The walking skeleton's fifth link. The monitor code the Python host writes at a VM's start, written from Rust with
> `a64_encoder` (Sprint 135) and an `Assembler` with labels (`vm_backend.py` 444-519): the 16 vectors (4209-4216: the
> synchronous current-EL slot branches to the exception entry, the others `hvc #2`), the dispatcher (4217-4258: the
> two-way table lookup on x17, the miss into the in-VM compiler under the runtime lock when `VMC_ENABLED`, else the
> host's `hvc #1` and the cache and TLB maintenance), the empty dispatch table (4264-4265), the exception entry
> (4343-4400: the code-write path to `hvc #13`, the save of x0-x30, ESR, FAR and ELR, `hvc #14`, the vectors' save, the
> ERET to EL1h, the call into `w2m_exception` under the lock, the return through the dispatcher), and the helpers it
> uses (`lock_runtime`, `unlock_runtime`, `count_from_budget`, `budget_from_count`, 521-557). Each Rust function cites
> the lines it carries. Its judge is behavior in a Rust VM, since the Python's words exist only when the Python runs: a
> dispatch-table hit in either of a pair's two slots reaches its code; a miss with the compiler off reaches `hvc #1`
> with x17 the target; a write fault on a read-only page with the code-write syndrome reaches `hvc #13` with x17 the
> faulting address; any other fault saves the registers, ESR, FAR and ELR to the save area and reaches `hvc #14`.

### work_performed

- 222990e5a Sprint 136: the monitor code from Rust

### signal_trace

```
(no record)
```

### signal_trace_tags


## 2. Expected

> None yet: the skeleton's record comes with the guest program.

## 3. Delta

### emits_check

- the card's Emits names no tag, and the sprint made no record

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 136: its Rubber Duck Pass and close.
