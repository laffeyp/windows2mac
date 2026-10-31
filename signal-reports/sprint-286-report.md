# SIGNAL_REPORT — Sprint 286 — The host's miss service under the runtime word

Made by `signal_report 286` from 0 records under store/runs/ whose baseline names one of the sprint's 2 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 286.

## 1. Observed

### scope_confirmation

> Reviewer-3's Medium 1 at 285's close (BLACKBOARD Surfaced, this card's commit). The code cursor, the reconstruction
> table and the dispatch table have writers inside the guest that hold the runtime word (the in-VM compiler,
> `block_compiler.c`, and the installer, `monitor_code.rs`) and writers in the host that do not: `HostCompiler::compile`
> and `place` (`compile.rs` 131, 164, 170), `install` (247) and `run::dispatch_insert` (395-406), which run while the host
> serves `hvc 1`, after the miss stub released the word (`monitor_code.rs` 450, `release_runtime` before `cbz x25, host`).
> On two vCPUs a host compile for one and an in-VM compile for the other can read the same cursor and write their code
> over each other. The design's row says "every append and every host drain takes" the word; the host's appends do not.
> On one vCPU nothing runs beside the host, and no run shows it.

### work_performed

- 98a25aa09 Sprint 286: the host's miss service under the runtime word
- 42a17c0b6 Sprint 286 card: the host's miss service under the runtime word; reviewer-3's review at 285's close dispositioned

### signal_trace

```
(no record)
```

### signal_trace_tags


## 2. Expected

> - None; a lock's coverage, held by tests and the one-vCPU runs.

## 3. Delta

### emits_check

- the card's Emits names no tag, and the sprint made no record

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 286: its Rubber Duck Pass and close.
