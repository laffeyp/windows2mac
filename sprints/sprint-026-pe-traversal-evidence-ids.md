# Sprint 026 — delay-load imports, relocation sites, non-returning calls, evidence execution ids

```yaml
---
id: 026
status: closed
phase: 3
pass_kind: functional
---
```

## scope

Repair the four smaller defects of review §4.9 that touch code. `pe.parse` reads the delay-load import directory (index 13; descriptor layout from LLVM `Object/COFF.h` `delay_import_directory_table_entry`: Attributes, Name, ModuleHandle, DelayImportAddressTable, DelayImportNameTable, BoundDelayImportTable, UnloadDelayImportTable, TimeStamp) into `delay_imports` and the IAT slot map, and keeps every base relocation site as (RVA, type). `x86ref.walk` stops fall-through after a call to a documented non-returning import and after `int 0x29` (`__fastfail`). Contract evidence of category observed carries `execution_ids`, checked against the store's traces when a store is given; contract `windows.event.auto_reset.single_signal@0.2.1` adds them (12 executions: runs 37738035459 and 37850017162) with acceptance unchanged.

## prerequisites

- 025

## context_files

- `docs/review-2026-10-08-project.md` §4.9; Microsoft "PE Format" (Delay-Load Directory Table; base relocation block); LLVM `llvm/include/llvm/Object/COFF.h`; Microsoft ExitProcess reference ("terminates the current process, including the calling thread"); Andriesse et al., USENIX Security 2016 (non-returning calls)
- `src/w2m/pe.py`, `src/w2m/x86ref.py`, `src/w2m/contract.py`, `contracts/windows.event.auto_reset.single_signal/0.2.0.json`

## signal contract

### Emits

- none from the library code. The new contract version is judged with `judge_traces.py store:<run>` (SESSION_INIT … COMPARISON_COMPLETED … SESSION_COMPLETED, unchanged tags).

## artifact contract

### Files modified

- `src/w2m/pe.py`, `src/w2m/x86ref.py`, `src/w2m/contract.py`, tests

### Files created

- `contracts/windows.event.auto_reset.single_signal/0.2.1.json`

Hard rule 6 exception: three small library repairs from one review section; recorded here.

### Content assertions

- `_wmi.pyd`: `delay_imports` names one DLL with at least one function; every delay IAT slot is in `iat`.
- `relocations` totals equal the count of (RVA, type) sites with type ≠ ABSOLUTE plus ABSOLUTE padding.
- A call through an IAT slot named `kernel32!ExitProcess` ends the traversal path.
- `load_contract` refuses observed evidence without `execution_ids`, and, with a store, refuses an id with no trace.

### Command exit codes

- `uv run pytest` → 0; ruff, mypy → 0

## observation contract

### Expected log substrings

- corpus differential (`decoder_differential.py corpus/python-3.14.8`): reachable instruction count changes (non-returning stops cut fall-through; delay slots resolve indirect calls); `ours length differs 0; ours iclass differs 0` still holds; `unresolved_indirect` falls by the delay-load calls in `_wmi.pyd`.
- `judge_traces.py store:37738035459 --contract …@0.2.1` and `store:37850017162`: 12 of 12 conforms.

## done criteria

The PE reader inventories every import the loader binds, the traversal stops where documented non-returning calls stop execution, and observed evidence names executions the loader can resolve.

## self-review checklist

- [x] Scope concrete. [x] Sources named. [x] Emits only 0.4 tags. [x] Gradable. [x] Observation declared before running. [ ] ≤2 files: exception recorded.

## observed

- `_wmi.pyd`: delay imports `ole32.dll` (CoInitializeEx, CoCreateInstance, CoSetProxyBlanket, CoInitializeSecurity, CoUninitialize); unresolved indirect transfers 43 → 38.
- Corpus traversal 2,391,671 → 2,391,551 instructions (120 fewer), unresolved indirect 13,264 → 13,259. The removed instructions are almost all `int3` padding after calls to `abort` (85 call sites), `exit` (36), `_exit` (5), `_CxxThrowException`, `ExitProcess`: the stops are right.
- libcrypto-3.dll lost one `int3`. Its strings decoded as code (Deferred 2026-10-08) are not caused by non-returning calls; the recorded cause (data embedded in hand-written assembly) stands. Review §4.9 asked this.
- Decoder differential on the new traversal: length and iclass equal XED everywhere; spec gaps 0.
- Relocation sites: every non-padding entry in both probe binaries is DIR64 (type 10).
- Contract 0.2.1 loads with the store check (22 execution-id citations, 12 distinct executions); 0.2.0 still loads on the legacy prose rule; judge 10/10 and 2/2 conforms under 0.2.1.
- Delay-load descriptors with Attributes bit 0 clear (pre-VC7, VA form) are refused with `PeError`, not guessed.
- Grading gap: the NORETURN test checks the set, not traversal behavior; the behavior evidence is the corpus measurement above.
