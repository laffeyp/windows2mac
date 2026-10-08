# SIGNAL_REPORT — Sprint 026

## 1. Observed

### scope_confirmation

> Made the PE reader inventory delay-load imports and keep relocation sites, stopped the traversal at documented non-returning calls, and made observed evidence name executions the loader resolves against the store.

### signal_trace

Library sprint; the judge runs under contract 0.2.1 read back from `runs/judge-1791496878573515000` and `runs/judge-1791496878655099000`: SESSION_INIT 1; EXECUTION_STARTED, EXECUTION_COMPLETED, TRACE_INGESTED, COMPARISON_COMPLETED 10 and 2 (all conforms); SESSION_COMPLETED 1 each.

## 3. Delta

### dual_contract_self_grade

**signal contract:** judge shape unchanged; 12 of 12 conforms. *pass*

**artifact contract:** content assertions hold (`_wmi.pyd` delay imports; relocation sites equal counts; contract id checks). The close claimed mypy 0 while mypy exited 1; corrected in commit 173e78a. *pass after correction*

## 4. Hypothesis

> The NORETURN list is name-based. A game that wraps ExitProcess in its own function will still be walked past; that needs inference of non-returning functions from their bodies (Andriesse et al. 2016), which waits until a game binary shows the case.
