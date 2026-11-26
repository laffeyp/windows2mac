# SIGNAL_REPORT — Sprint 354 — The woken mark: a thread readied before it blocks stays ready

Made by `signal_report 354` from 0 records under store/runs/ whose baseline names one of the sprint's 1 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 354.

## 1. Observed

### scope_confirmation

> Card 3 of `docs/design-handles-and-sync-objects-2026-11-26.md` ("The wait across two locks", 268-281; "The cards that
> follow", 354-357). Card 5 will take waits off the runtime word. A wait will then check its object under the sync
> objects' lock, let that lock go, and only then take the word to park in `w2m_block`. A readying that falls between the
> two is lost today. `w2m_ready_thread` (`tools/guest_runtime/threads.c` 150-159) readies only a BLOCKED thread, so it
> does nothing in that window, and `w2m_block` (91-116) then blocks the thread, which wakes only at its deadline, or never
> for INFINITE. The woken mark closes the window, under the word:
> - `w2m_ready_thread` sets the mark on a thread that is not BLOCKED;
> - `w2m_block` finds it set, clears it, and leaves the thread READY, so the switch retries the call at once.

### work_performed

- a85737f80 Sprint 354 card: the woken mark, a thread readied before it blocks stays ready, every readying path listed

### signal_trace

```
(no record)
```

### signal_trace_tags


## 2. Expected

> - None; a thread's state between a readying and its block.

## 3. Delta

### emits_check

- the card's Emits names no tag, and the sprint made no record

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 354: its Rubber Duck Pass and close.
