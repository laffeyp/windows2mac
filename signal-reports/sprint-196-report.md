# SIGNAL_REPORT — Sprint 196 — The module families' judge

Made by `signal_report 196` from 0 records under store/runs/ whose baseline names one of the sprint's 2 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 196.

## 1. Observed

### scope_confirmation

> The Decision "Sequence after Sprint 193", reviewer-2's Medium 5 of its review of Sprints 190-192 (nothing before
> UnityMain is judged) and Medium 1 of its review of 185-189 (`in_app_directory`, 0 for a name found nowhere, `load_dll`).
> Census row 6, the family judge, is absent in Rust. On 2026-10-28 `w2m-vm` ran one runner binary of each module family:
> `windows.module.system` ends and its 14 rows equal the contract's; `windows.module.names_ansi` ends and its 10 rows
> equal the contract's once its derivation drops `w_len` and `ret`; `windows.module.queries` stops at its first
> LoadLibrary, `w2m-no-such-module.dll`, "a DLL file after the start, not served"; `windows.module.search` stops at
> FILE_MKDIR 25.

### work_performed

- c30a250b0 Sprint 196: the module families' judge
- 8f2c5e598 Sprint 196 card: the module families' judge

### signal_trace

```
(no record)
```

### signal_trace_tags


## 2. Expected

> - None: the judge is a test over `w2m-vm`'s output.

## 3. Delta

### emits_check

- the card's Emits names no tag, and the sprint made no record

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 196: its Rubber Duck Pass and close.
