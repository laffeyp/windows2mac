# SIGNAL_REPORT — Sprint 291 — A loaded DLL's entry published last

Made by `signal_report 291` from 0 records under store/runs/ whose baseline names one of the sprint's 2 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 291.

## 1. Observed

### scope_confirmation

> Reviewer-3's Low 1 at 290, as the lookup at 290 settled it: `process::write_loader_data` writes every entry's fields
> and names in its first loop, then each list's links node by node from the head (`link`, `process.rs` 618-627). On a
> LoadLibrary the head's `Blink` and the old tail's `Flink` name the new entry before the new entry's own `Flink` and
> `Blink` are written, so a guest thread on another vCPU walking `PEB->Ldr` can follow an unwritten link. The guest
> runtime's module calls run under the runtime word, which the requesting vCPU holds; application code walking the
> lists does not. Windows inserts an entry filled and linked, then publishes it.

### work_performed

- 8a436e769 Sprint 291: a loaded DLL's entry published last
- f706a3186 Sprint 291 card: a loaded DLL's entry published last; reviewer-3's Low 1 at 290

### signal_trace

```
(no record)
```

### signal_trace_tags


## 2. Expected

> - None; an order of writes, held by a test.

## 3. Delta

### emits_check

- the card's Emits names no tag, and the sprint made no record

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 291: its Rubber Duck Pass and close.
