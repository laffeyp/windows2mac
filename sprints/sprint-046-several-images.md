# Sprint 046 — several images

```yaml
---
id: 046
status: closed
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

## observed

- **The frontier moved** from 19,084 instructions (the call to `UnityPlayer.dll!UnityMain`, a stop) to 403,044: the loader maps UnityPlayer.dll, its DllMain runs its C runtime and its own start-up and returns, the launcher's WinMain calls UnityMain, and UnityPlayer's code stops at `SHELL32.dll!CommandLineToArgvW` (SERVICE_UNIMPLEMENTED, called from 0x180527460). Every one of the 54 services called on the way has rows in a family (checked against the frontier's call counts).
- **The loader:** a DLL from the program's directory at its preferred base when free, else relocated (44,285 DIR64 sites, tested at two bases); imports bound to exports, a missing one refusing the load; DllMain(DLL_PROCESS_ATTACH, non-NULL) from an x86 stub before the program's entry; PEB.Ldr's three lists and a synthetic ntdll; static TLS (index 0 the program's, the first DLL's 1).
- **Families accepted, each on its own record:** loader, static TLS (the DLL on the static C runtime, its start-up run to the end), pointer encoding, thread queries, slim locks, system information, virtual memory (an arena behind one host request per call), sync objects, storage get2 (Server 2025's rows), time and error mode (parent and child). Families may name host files and plant faults in host modules.
- **Windows against expectation:** apphelp.dll joins the loader lists on a binary's first execution only; an ASLR DLL loaded at its preferred base on every runner; an image's writable pages read PAGE_WRITECOPY until written; a failed VirtualProtect still writes the old protection; FlsGetValue on a freed index answers NULL with error 0; Server 2022 lacks FlsGetValue2 and TlsGetValue2; a thread's creation time can read later than the system time on Server 2022; GetVersionExW says 6.2 build 9200 and product type 3 (Server) without a manifest; GetTimeZoneInformation sets error 0; the error mode under cmd is 0x8003.
- **Instruction classes:** sets 046 (13 SSE classes), 046b (CQO, CWDE, IDIV; Windows's divide status rule holds for IDIV), 046c (CMPXCHG16B, constructed cases for the success path), 046d (26 SSE2 integer classes; a table-driven NEON lowering), 046e (RCPPS, RSQRTPS, RCPSS, RSQRTSS; 742 cases on the 8573C); 231 of 231 classes accepted. RCPPS and RSQRTPS: every CPU tabulated has the same shape (12 mantissa bits decide; exponents scale exactly), AMD EPYC 7763, 9V45 and 9V74 share one table set, Intel Xeon 8573C and 8370C another, bit for bit; w2m carries Intel's (`specs/approximations.json`) and runs these classes in the model (slow path, a performance debt). The Xeon 6973P was not drawn in 48 runners.
- **Planted faults:** two named reaches were wrong and are recorded: a static-TLS plant (indices from 0) named one row and faulted the run; a February-29 plant named February 31, which stays invalid, while later dates moved. A host plant's patch leaked into the next plant until importers were repointed.
- **Lost runner rounds and their causes:** the Windows SDK and mingw's headers differ (`small` twice, a union in IMAGE_TLS_DIRECTORY64, GetVersionExW's deprecation; the gate now defines `small` and `hyper`); `_fltused` without the C runtime; a hand-made TLS directory faulted on Windows while w2m ran it (replaced by the static C runtime's); an import used only through a function pointer is invisible to the reference traversal (probe revised, runs replaced); a shell chain committed a probe whose local compile had failed; a cancel loop killed another dispatcher's runs; one transient `gh` failure ended a search (gh calls now retried).
- Gates and CI: see the closing commit.
