# SIGNAL_REPORT — Sprint 047

## 1. Observed

### scope_confirmation

> UnityMain's start-up runs from 403,044 to 2,522,889 instructions and stops at the game's first file call, an absolute path the file bridge does not serve. Every service and instruction class on the way is judged against Windows and an Intel CPU.

### work_performed

- The look-ahead (`scripts/frontier_lookahead.py`, `VM.run(on_unimplemented=...)`); `scripts/vm_against_model.py` (the VM against the model with no runner).
- Capacity: a 512 GB arena (`src/w2m/vm_backend.py`, `tools/vm_monitor/vm_monitor.c`), a 64 MB code cache and a 4 Mi-entry reconstruction table.
- Classes: `src/w2m/string_compare.py`, rules pcmpistri, unpckhpd, lfence, mfence, BTC in bt, narrow MUL in `_int`; set 047 with constructed string compares.
- Families: command line arguments (`command_line_args.c`, SHELL32 through `DLL_SERVICES`), generate random (`bcrypt.c`, host request 23), processor topology (`topology.c`), file attributes (`file_attributes.c`, bridge `FILE_STAT`).
- Tools: `class_run` and `family_run` decide acceptance from the record and fail on a miss; the class index merges; class runs read the committed case files; record scans read one line per record.

### signal_trace

```
store/runs/frontier-643a6283510c-1791667653042496000: SESSION_INIT, EXECUTION_STARTED, EXECUTION_COMPLETED (faulted: a file call in a run with no file-system root; 2,522,889 instructions), SESSION_COMPLETED
store/runs/family-runs.json: 25 families, each SESSION_INIT, CANDIDATE_GENERATED, COMPARISON_COMPLETED x runner binaries, CANDIDATE_ACCEPTED, SESSION_COMPLETED
store/runs/class-runs.json: 237 classes, model and managed_vm conform (047 on Intel Xeon 8573C run 38084067002, release semantics-047)
```

## 2. Expected

> The card expected the arena to hold UnityPlayer's reservations, each new class to equal an Intel CPU with a planted fault of named reach, CommandLineToArgvW to equal Windows with plants of named reach, and the frontier to pass 491,869 instructions with every service judged. All held.

## 3. Delta

### dual_contract_self_grade

**Signal contract:** pass (vocabulary 0.7, no new tags). **Artifact contract:** gates 0; CI on the closing commit. **Observation contract:** arena pass (test and frontier); classes pass (8573C, plant 170 of 170); CommandLineToArgvW pass (plants 1, 2 and 24 as named); frontier pass (2,522,889).

**overall:** pass.

## 4. Hypothesis

### rubber_duck_observations

**Sequence narration:** look ahead past the stop; take each need in order (capacity, instruction, service family); probe, contract, guest code, item run, plants; run the frontier.

- *Observation 1 (a summary that counted what was not there):* class_run and family_run reported success the record did not hold. Disposition: both read the decision from the record; KIT_DIARY #34, #35.
- *Observation 2 (a host fact below the guest's resolution):* the Mac's birth time as a float was coarser than FILETIME. Disposition: read exactly from libc; KIT_DIARY #36.
- *Observation 3 (OS versions differ):* relationship 7 and a trailing backslash differ between Server 2025 and 2022. Disposition: excluded rows; w2m answers as 2025.

### status_and_blockers

`status: complete`
