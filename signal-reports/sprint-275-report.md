# SIGNAL_REPORT — Sprint 275 — The host's drain under the runtime word

Made by `signal_report 275` from 0 records under store/runs/ whose baseline names one of the sprint's 3 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 275.

## 1. Observed

### scope_confirmation

> Census row D6 (`docs/step-6-census-2026-10-30.md`; Deferred, Sprint 247, annotated at 248 and 261) names the in-VM
> compiler's 23 counts, the miss stub's compile-time slots and the installer's shared words (the code cursor, the
> reconstruction table's count, the compile log's count) as plain reads and writes that race on several vCPUs. This
> card's lookup finds every one of them written inside the VM under the runtime word: `call_path` takes it before the
> compiler or the installer and releases it after the probe (`crates/w2m_vm/src/monitor_code.rs` 356-381), so on two
> vCPUs they do not race while the runtime word is held across a compile. What the word does not cover is the host:
> `run::vm_compiled` reads the compile log and writes its count back to 0 at each exit (`run.rs` 426-475, called at
> 718) with no lock, while another vCPU may be appending under the word (`block_compiler.c` 237-314: read `n_log`, write
> the entry, write `n_log + 1`). On two vCPUs an append between the drain's read and its reset is lost. This card has
> the host take the runtime word around its drain.

### work_performed

- 13674dd49 Sprint 275: census row D6, the shared words under the runtime word and the drain with them
- c6602b6b2 Sprint 275: the host drains the compile log under the runtime word
- cbfd2b928 Sprint 275 card: the host's drain under the runtime word

### signal_trace

```
(no record)
```

### signal_trace_tags


## 2. Expected

> - None; a lock taken by the host, held by tests.

## 3. Delta

### emits_check

- the card's Emits names no tag, and the sprint made no record

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 275: its Rubber Duck Pass and close.
