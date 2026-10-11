# Sprint 052 — COM and the audio device enumerator

```yaml
---
id: 052
status: open
phase: 5
pass_kind: functional
---
```

## scope

Sprint 051 left the frontier at 20,590,160 instructions, at `ole32.dll!CoInitializeEx` on mono's first thread (trace seq 1279: `CoInitializeEx(NULL, COINIT_MULTITHREADED)`, S_OK). The work list's com family holds what the captures call: CoInitializeEx 1, CoInitialize 5 (S_OK once, then S_FALSE), CoCreateInstance 6, CoUninitialize 4, PropVariantClear 4, CoCreateGuid 1; the OLEAUT32 ordinals 2, 6 and 9 (SysAllocString, SysFreeString, VariantClear) and CoSetProxyBlanket are imported and never called. Every CoCreateInstance asks UnityPlayer's audio code's one class: `CLSID_MMDeviceEnumerator` {BCDE0395-E52F-467C-8E3D-C4579291692E}, `IID_IMMDeviceEnumerator` {A95664D2-9614-4F35-A746-DE8DB63617E6}, CLSCTX_ALL or CLSCTX_INPROC_SERVER, read from UnityPlayer.dll at the RVAs the trace's arguments give.

1. **The rows.** A com probe on Server 2025 and 2022: the apartment calls (first, repeated, mixed models, uninitialized), CoCreateInstance of the enumerator and an unknown class, the enumerator's methods as the runner answers them (EnumAudioEndpoints, the collection's GetCount, GetDefaultAudioEndpoint, and a device's property store if there is one), reference counts, QueryInterface, PropVariantClear, CoCreateGuid's version and variant bits, and the three OLEAUT32 calls.
2. **COM in the guest runtime:** apartments per thread, class creation by CLSID, objects whose vtables are service thunks taking `this`, and the enumerator answering as the runner does.

## prerequisites

- 051 (closed: CI run on d73e4ade; main 17b3e9d8)

## context_files

- `specs/work_list.json`, `store/game-runs/38113342549/jit_capture.bin.trace.tsv`, `src/w2m/vm_backend.py` (thunks, DLL_SERVICES), `src/w2m/modules.py`, `tools/guest_runtime/runtime.h`; Microsoft Learn: CoInitializeEx, CoCreateInstance, IMMDeviceEnumerator, PropVariantClear, CoCreateGuid, SysAllocString

## signal contract

### Emits

- Per family: the item run. Per frontier run: EXECUTION_STARTED, then SERVICE_UNIMPLEMENTED or EXECUTION_COMPLETED.

### Invariants

- Vocabulary 0.7; no new tags.

## artifact contract

- created: `probes/com/`, the com contract, the guest runtime's COM services
- modified: `src/w2m/vm_backend.py` or `src/w2m/modules.py` (a vtable's thunks), `tools/guest_runtime/runtime.h`
- gates 0 and CI `success` on the closing commit

## observation contract

- **The com family equals Windows** on its probe's rows on Server 2025 and 2022, with plants of named reach.
- **The frontier passes CoInitializeEx and UnityPlayer's audio enumeration**; the next stop is recorded.

## done criteria

The com family accepted; the frontier runs past the six CoCreateInstance calls.

## observed
