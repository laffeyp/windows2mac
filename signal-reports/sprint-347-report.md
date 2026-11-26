# SIGNAL_REPORT — Sprint 347 — Another thread's TLS value after TlsFree and TlsAlloc, as Windows gives it

Made by `signal_report 347` from 0 records under store/runs/ whose baseline names one of the sprint's 2 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 347.

## 1. Observed

### scope_confirmation

> Reviewer-4's fifteenth review, Low 1, recorded as the Decision "TlsFree's reach settled by a Windows row". It found
> that w2m's TlsFree clears the freed index in the calling thread's TEB alone (`tools/guest_runtime/thread_storage.c`,
> `k32_TlsFree`). TlsAlloc does the same: it clears only the caller's slot (`k32_TlsAlloc`, `*slot = 0`). Microsoft
> Learn documents TlsAlloc as "When a TLS index is allocated, its storage slots are initialized to NULL" (TlsAlloc,
> Remarks), and the return-value section reads "The slots for the index are initialized to zero". The TlsFree page says
> nothing of what a free does to any thread's value. So a thread that set an index, which another thread then freed and
> allocated again, may read its old value in w2m where Windows reads 0.

### work_performed

- 003a0c1a9 Sprint 347 card amendment: the probe tls_expansion_first, for the first thread's use of an expansion index another thread allocated
- 1a9054451 Sprint 347 card: another thread's TLS value after TlsFree and TlsAlloc, as Windows gives it; the probe tls_free_reach

### signal_trace

```
(no record)
```

### signal_trace_tags


## 2. Expected

> - None; what two kernel32 services write.

## 3. Delta

### emits_check

- the card's Emits names no tag, and the sprint made no record

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 347: its Rubber Duck Pass and close.
