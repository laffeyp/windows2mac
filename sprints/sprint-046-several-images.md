# Sprint 046 — several images

```yaml
---
id: 046
status: open
phase: 5
pass_kind: functional
---
```

## scope

The launcher runs to its call of `UnityPlayer.dll!UnityMain` (Sprint 044: 19,084 instructions) and stops, because w2m loads one image. UnityPlayer.dll is the launcher's only import outside KERNEL32: 490 functions from 18 DLLs (217 KERNEL32, 107 USER32, 48 WS2_32, 24 ADVAPI32, 23 WINMM, 14 HID, 11 ole32, the rest 1 to 8 each), 44,285 relocation sites, no TLS callbacks, no delay imports, one export. mono-2.0-bdwgc.dll is not a static import; UnityPlayer loads it at run time.

1. **The loader's behaviour, recorded on Windows.** A no-CRT probe executable that statically imports a probe DLL (which imports KERNEL32 and one more system DLL) records: the PEB loader lists (load, memory and initialization order: names and whether each base equals the module handle), each DllMain call (reason, the reserved argument NULL or not, the order among DLLs), what GetModuleHandleW and GetModuleFileNameW answer for the DLL, and where the DLL loaded against its preferred base.
2. **Loading a second image:** map UnityPlayer.dll, relocate it where its base is taken, bind its imports (KERNEL32 to the guest services, the launcher's import of `UnityMain` to the DLL's export), and give every other system DLL a synthetic module (as `src/w2m/modules.py` does for kernel32 and kernelbase) whose functions stop by name when called (SERVICE_UNIMPLEMENTED).
3. **The PEB loader list and DllMain:** the lists as the probe's rows say; UnityPlayer's DllMain (its C runtime's start-up, then its own) runs with DLL_PROCESS_ATTACH before the launcher's entry, as Windows orders it.
4. **The families the DLL's start-up reaches,** by the 044 pipeline, as the frontier meets them.

## prerequisites

- 044 (closed; CI 38047526561 success on bf80c42), the 045 deferral (BLACKBOARD 2026-10-10)

## context_files

- `src/w2m/loader.py`, `src/w2m/modules.py`, `src/w2m/pe.py`, `src/w2m/vm_backend.py`, `scripts/run_frontier.py`, `scripts/family_run.py`, Microsoft Learn: DllMain, Dynamic-Link Library Best Practices; the PE format page (base relocations)

## signal contract

### Emits

- Per family and the loader probe: the 044 item run. Per frontier run: EXECUTION_STARTED, SERVICE_UNIMPLEMENTED at the stop.

### Invariants

- Vocabulary 0.7; no new tags.

## artifact contract

- created: `probes/loader/` (an exe and a DLL), its contract, the loader family's item run; guest and host code for several images
- modified: `src/w2m/loader.py`, `src/w2m/modules.py`, `src/w2m/vm_backend.py`
- gates 0 and CI `success` on the closing commit

## observation contract

- **The loader equals Windows** on the probe's rows in the VM, on its own record, with planted faults of named reach (a list order swapped; DllMain's reserved argument NULL for a static load).
- **UnityPlayer's DllMain returns TRUE** in w2m, and the launcher enters `UnityMain`: the frontier's stop is inside UnityPlayer's code, past `UnityMain`'s first instruction.
- **Relocations:** every one of UnityPlayer's 44,285 sites is applied when the DLL loads away from its preferred base (a test loads it at two bases and compares).

## done criteria

The launcher calls `UnityMain` and UnityPlayer.dll's code runs in w2m, its DllMain and every service it called judged against Windows on a family's record.
