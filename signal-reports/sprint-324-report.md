# SIGNAL_REPORT — Sprint 324 — The run loop takes the lock as a thread would

Made by `signal_report 324` from 0 records under store/runs/ whose baseline names one of the sprint's 2 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 324.

## 1. Observed

### scope_confirmation

> The run-loop design's card 4c3b2 runs a host thread per vCPU over one `&GuestLock`. Its run loop (`run.rs` `run`)
> reaches `Guest` five times through `GuestLock::get_mut` (`run.rs` 844, 961, 1143, 1262, 1370), which needs the lock
> borrowed `&mut`, which two threads cannot hold; the borrow at 844 serves the rest of the body too: the exit ledger
> (897), `time_runtime_word` (906), the block judge (924), the model step (`model_step(guest, …)`, 1175) and the slice
> end's test of `served.link` (1194). The design decided how a thread reaches `Guest` outside a service ("How the
> threads share `Guest`", Sprint 309): each vCPU thread keeps a copy of the Load facts it reads, and the turn's start
> reads `served.link` and `served.threads` either under the word and the lock or from memory the services write, card 4c
> to measure the first against the second. No card has built it; card 4c3b2 as the design states it would build it with
> the threads. This card builds it on one vCPU first, so 4c3b2 changes how many threads run the loop and not how the
> loop reaches `Guest`.

### work_performed

- 38a90b136 Sprint 324: the run loop takes the lock as a thread would
- 0249c0a15 Sprint 324 card: the run loop takes the lock as a thread would; the design's card 4c3b2 split

### signal_trace

```
(no record)
```

### signal_trace_tags


## 2. Expected

> - None; how the run loop reaches its state.

## 3. Delta

### emits_check

- the card's Emits names no tag, and the sprint made no record

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 324: its Rubber Duck Pass and close.
