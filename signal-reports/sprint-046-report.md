# SIGNAL_REPORT — Sprint 046

## 1. Observed

### scope_confirmation

> The launcher loads UnityPlayer.dll, runs its DllMain to its return and calls UnityMain; UnityPlayer's code runs to 403,044 instructions and stops at `SHELL32.dll!CommandLineToArgvW`. Every service called on the way is judged against Windows on a family's record.

### work_performed

- Loader: several images, relocation, import binding, DllMain stub, PEB.Ldr, synthetic ntdll, static TLS (`src/w2m/loader.py`, `src/w2m/modules.py`, `src/w2m/vm_backend.py`).
- Families: loader, static TLS, pointer encoding, thread queries, slim locks, system information, virtual memory (`src/w2m/virtual_memory.py`), sync objects, storage get2, time and error mode (parent, child).
- Instruction sets 046 to 046e; RCPPS and RSQRTPS from measured Intel tables (`src/w2m/approximate.py`, `specs/approximations.json`, `scripts/approximation_tables.py`).
- Tools: host plants and host files in `scripts/family_run.py`; `--cpu` and retried gh calls in `scripts/probe_on_intel.py`; the gate's compile defines the SDK's `small` and `hyper`.

### signal_trace

```
store/runs/frontier-643a6283510c-1791636246457146000: SESSION_INIT, EXECUTION_STARTED, SERVICE_UNIMPLEMENTED (SHELL32.dll!CommandLineToArgvW, 0x180527460, 403,044 instructions), SESSION_COMPLETED
store/runs/family-runs.json: 22 families, each SESSION_INIT, CANDIDATE_GENERATED, COMPARISON_COMPLETED x runner binaries, CANDIDATE_ACCEPTED, SESSION_COMPLETED; all on the final runtime
store/runs/class-runs.json: 231 classes, model and managed_vm conform (046e on Intel Xeon 8573C run 38053474903)
store/runs/semantics-046{,b,c,d,e}-*, vm-oracle-046*: every unit accepted
```

## 2. Expected

> The card expected the loader to equal Windows on its probe's rows with named-reach plants, UnityPlayer's DllMain to return TRUE and the launcher to enter UnityMain, relocations applied at any base, and the families the DLL's start-up reaches. All held; two named reaches were wrong and are recorded.

## 3. Delta

### dual_contract_self_grade

**Signal contract:** pass (vocabulary 0.7, no new tags). **Artifact contract:** gates 0 and CI on the closing commit. **Observation contract:** loader pass; DllMain and UnityMain pass; relocations pass (two bases, 44,285 sites).

**overall:** pass.

## 4. Hypothesis

### rubber_duck_observations

**Sequence narration:** the frontier stops at a service or an instruction; a scratch run with stubs lists what follows; a probe on the runners (an Intel one for instructions, a chosen model for approximations) gives the rows; the guest or the host answers them; plants check the judge; the item run or the case set accepts; the frontier moves.

- *Observation 1 (headers):* the local gate's mingw headers differ from the runners' Windows SDK; five runner rounds lost. Disposition: the gate defines `small` and `hyper`; the rest recorded; KIT_DIARY #31.
- *Observation 2 (CPU-specific results):* RCPPS and RSQRTPS differ between vendors; the reference is a named CPU model, not "Intel". Disposition: tables measured and checked across two Intel generations; KIT_DIARY #32.
- *Observation 3 (wrong reaches):* two. Disposition: recorded on the card.

### status_and_blockers

`status: complete`
