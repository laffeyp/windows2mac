# SIGNAL_REPORT — Sprint 193 — The shared objects in one process

Made by `signal_report 193` from 0 records under store/runs/ whose baseline names one of the sprint's 2 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 193.

## 1. Observed

### scope_confirmation

> The Decision "Sequence after Sprint 190": one card per op family in the order the call stream meets them. Sprint
> 191's stream stops at row 4, UnityPlayer's CreateEventW, KERNEL 33's EVENT_OPEN. The family is `src/w2m/kernel.py`'s
> shared objects (rows 4-6 events, 7 CreatePipe, 16 CreateProcessW, 23 WaitForSingleObject on an event, 25-27 WriteFile
> into the pipe). The plan puts the crash handler's process in step 4 (its gate is the batch run, which always starts
> UnityCrashHandler64.exe; step 6 brings a process per guest process), but a second VM is a component of its own: this
> card serves the objects one process uses, and the second process is the next card's.

### work_performed

- d640b6d2f Sprint 193: the shared objects in one process
- c3b220559 Sprint 193's card: the shared objects in one process

### signal_trace

```
(no record)
```

### signal_trace_tags


## 2. Expected

> - None: no producer runs; the judges are `batch_run.rs` and unit tests.

## 3. Delta

### emits_check

- the card's Emits names no tag, and the sprint made no record

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 193: its Rubber Duck Pass and close.
