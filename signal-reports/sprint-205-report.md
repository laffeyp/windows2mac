# SIGNAL_REPORT — Sprint 205 — A DLL loaded after the start

Made by `signal_report 205` from 0 records under store/runs/ whose baseline names one of the sprint's 3 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 205.

## 1. Observed

### scope_confirmation

> The Decision "Sequence after Sprint 198", item 4: the batch run's stream equals the Python's rows 0-499 and stops at
> row 500, MODULE 15, UnityPlayer.dll's LoadLibrary of MonoBleedingEdge\EmbedRuntime\mono-2.0-bdwgc.dll (4,974,728
> bytes; no TLS directory, entry point RVA 0x31953c), a DLL file after the start. windows.module.search's two binaries stop
> at the same request for w2m_search\w2m_found.dll. `_load_dll` (`src/w2m/vm_backend.py` 3906-3951) maps such a file
> and `_init_stub` (3988-4023) runs its DllMain as the caller's next code.

### work_performed

- 193204f44 Sprint 205: the stream's row 924 traced to the clock warp; the load deferred
- f99c974b6 Sprint 205 card: a DLL loaded after the start
- ccac604c4 Sprint 205 card: a DLL loaded after the start

### signal_trace

```
(no record)
```

### signal_trace_tags


## 2. Expected

> - None: the judges are `batch_run.rs`, `family_runs.rs` and unit tests.

## 3. Delta

### emits_check

- the card's Emits names no tag, and the sprint made no record

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 205: its Rubber Duck Pass and close.

> status: failed; reason: the stream's row 924 differed (mono's /GS cookie), first read as the clock's warp, then found a stale reference stream; rescoped-as: 207

*(Annotated 2026-10-28, reviewer-2's Low of its corrections to the 205 dispositions: the card's artifact contract did not land in Sprint 205, so the template's "failed; rescoped-as" status, not a partial pass; the card stays `closed-partial` for `--check`'s Deferred rule. The load landed in Sprint 207, whose card names it.)*
