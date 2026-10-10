# Sprint 047 — UnityMain's start-up

```yaml
---
id: 047
status: open
phase: 5
pass_kind: functional
---
```

## scope

Sprint 046 left the frontier inside UnityMain at `SHELL32.dll!CommandLineToArgvW` (403,044 instructions). A look-ahead run (`scripts/frontier_lookahead.py`: each missing import returns 0 and the run goes on; a map, never a judgment) showed the next two stops before any card was written: the ninth 0x1FFFF000-byte reservation filled the 4 GB VirtualAlloc arena (462,293 instructions), and, with the arena enlarged, PCMPISTRI at 0x18140b914 has no operand template (491,869). This sprint takes the frontier through UnityMain's start-up, stop by stop.

1. **The look-ahead,** `on_unimplemented` in `VM.run` and `scripts/frontier_lookahead.py`: lists the imports past the stop in first-call order, so families are planned from the program's path and not one stop at a time.
2. **Address space:** the VirtualAlloc arena from 4 GB to 512 GB, mapped in 1 GB level-1 blocks over a sparse file, with a 40-bit IPA (TCR.IPS 2; `hv_vm_config_set_ipa_size`). Windows gives a 64-bit process 128 TB; a reservation past 512 GB stops by name.
3. **SSE4.2 string instructions:** PCMPISTRI and the classes the frontier meets next, each a class run on an Intel case set.
4. **SHELL32's CommandLineToArgvW** as a family: a runner probe over the documented quoting and backslash rules and the program-name rule, a contract from its rows, guest C, an item run. SHELL32 gets a synthetic module like kernel32's.
5. **The families the frontier meets next,** by the 044 pipeline. The plan's 047 work (exception dispatch, threads) moves to the sprint where the frontier first needs it.

## prerequisites

- 046 (closing; CI on 7155a70 refused by GitHub for billing, not run)

## context_files

- `src/w2m/vm_backend.py`, `src/w2m/virtual_memory.py`, `tools/vm_monitor/vm_monitor.c`, `scripts/run_frontier.py`, `scripts/family_run.py`, `specs/x86_decode.json`, Intel SDM Vol. 2B PCMPISTRI and Vol. 1 §4.1 (packed string compare), Microsoft Learn: CommandLineToArgvW, "Parsing C++ command-line arguments"; Arm ARM D8.3 (level-1 blocks)

## signal contract

### Emits

- Per family: the 044 item run. Per class: the class run. Per frontier run: EXECUTION_STARTED, SERVICE_UNIMPLEMENTED at the stop. The look-ahead emits nothing and writes no record.

### Invariants

- Vocabulary 0.7; no new tags.

## artifact contract

- created: `scripts/frontier_lookahead.py`, `tests/test_virtual_arena.py`, a CommandLineToArgvW probe and contract, case sets for the SSE4.2 classes
- modified: `src/w2m/vm_backend.py`, `tools/vm_monitor/vm_monitor.c`, the decode spec and rules
- gates 0 and CI `success` on the closing commit

## observation contract

- **The arena:** sixty-four 0x1FFFF000-byte reservations succeed at successive 512 MB bases (test), and the frontier passes the reservation that failed at 462,293 instructions.
- **Each new instruction class** equals an Intel CPU, named by model, on its case set, with a planted fault of named reach.
- **CommandLineToArgvW equals Windows** on the probe's rows in the VM, on its own record, with planted faults of named reach.
- **The frontier moves past 491,869 instructions,** every service before the new stop judged on a family's record.

## done criteria

The frontier's stop lies past PCMPISTRI at 0x18140b914 and past CommandLineToArgvW, with every service and class on the way judged against Windows and an Intel CPU.

## observed
