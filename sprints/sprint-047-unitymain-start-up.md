# Sprint 047 — UnityMain's start-up

```yaml
---
id: 047
status: closed (CI run 38088432347 success on 0f889e5, 2026-10-10)
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

- **The frontier moved** from 403,044 instructions (`SHELL32.dll!CommandLineToArgvW`) to 2,522,889 (`store/runs/frontier-643a6283510c-1791667653042496000`). It stops at the game's first file call, `GetFileAttributesExW("C:\\w2m\\app\\Data\\...")`: the frontier gives the VM no file-system root, and the bridge refuses absolute paths. Every service before the stop has a family judged against Windows.
- **The look-ahead** (`scripts/frontier_lookahead.py`; `on_unimplemented` in `VM.run`) listed the stops ahead from the program's own path: the 4 GB arena, PCMPISTRI, UNPCKHPD, MUL r/m32, LFENCE, BTC, the full reconstruction table, then CommandLineToArgvW, BCryptGenRandom, GetLogicalProcessorInformationEx and GetFileAttributesExW.
- **Capacity:** the VirtualAlloc arena from 4 GB to 512 GB (1 GB level-1 blocks over a sparse file, 40-bit IPA); the code cache from 4 to 64 MB and the reconstruction table from 65,536 to 4 Mi entries.
- **Instruction classes, set 047** (3,104 cases): PCMPISTRI (constructed over all 128 control bytes), PCMPISTRI64, UNPCKHPD, MUL r/m8-32, BTC, LFENCE, MFENCE. The model equals Rosetta 2 on 153,600 string compares, three AMD runs, and the Intel Xeon 8573C (run 38084067002) on all 3,093 judged cases; the VM equals the 8573C on all; a planted equal-each fault named 170 cases and 170 differ. 237 classes accepted.
- **Families accepted, each on its own record:** command line arguments (SHELL32, 26 rows), generate random (bcrypt, the host's CSPRNG), processor topology (seven functions, four runners on two Intel and one AMD model), file attributes (two functions, a new bridge primitive). Windows against expectation: CommandLineToArgvW takes quotes in the program name literally and sets the last error to 0 on success; BCryptGenRandom never touches the last error and fails a NULL buffer even at length 0; relationship 7 exists on Server 2025 and fails with error 31 on 2022; a file named with a trailing backslash is ERROR_DIRECTORY on 2025 and ERROR_INVALID_NAME on 2022; a directory's size is 4096.
- **Defects found and fixed:** CI red on 40 runs from 3d091e3 (depot-reading loader tests; then a class-run test over 30 s on a cold runner; then gitignored case files); `class_run` counted a crashed VM judge as accepted and a single-class run truncated the index (KIT_DIARY #34); `family_run` returned 0 on discrepancies and on plants that missed their reach; a BTC case read memory outside its windows; the Mac's float birth time came out after the access time; record scans read every record whole (`read_first_envelope` now) and the causal test reloaded each record 308 times.
- **The repository** went public as `laffeyp/windows2mac` under substrate's licensing (user, 2026-10-10); Actions for public repositories cost nothing; Sprint 046 closed on CI run 38084406263.
- **Feasibility review:** verified; C2 and C3 rejected by the user; F1 and F3 are the next sprint's first work.
- Gates and CI: CI run 38087819989 on 5ac2a1e failed on seven class-index entries that named records by this Mac's absolute path; 0f889e5 fixed them and added a test; CI run 38088432347 on 0f889e5 succeeded.

