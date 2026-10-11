# Sprint 052 — COM and the audio device enumerator

```yaml
---
id: 052
status: closed (partial)
phase: 5
pass_kind: functional
---
```

## scope

Sprint 051 left the frontier at 20,590,160 instructions, at `ole32.dll!CoInitializeEx` on mono's first thread (trace seq 1279: `CoInitializeEx(NULL, COINIT_MULTITHREADED)`, S_OK). The work list's com family holds what the captures call: CoInitializeEx 1, CoInitialize 5 (S_OK once, then S_FALSE), CoCreateInstance 6, CoUninitialize 4, PropVariantClear 4, CoCreateGuid 1; the OLEAUT32 ordinals 2, 6 and 9 (SysAllocString, SysFreeString, VariantClear) and CoSetProxyBlanket are imported and never called. Every CoCreateInstance asks UnityPlayer's audio code's one class: `CLSID_MMDeviceEnumerator` {BCDE0395-E52F-467C-8E3D-C4579291692E}, `IID_IMMDeviceEnumerator` {A95664D2-9614-4F35-A746-DE8DB63617E6}, CLSCTX_ALL or CLSCTX_INPROC_SERVER, read from UnityPlayer.dll at the RVAs the trace's arguments give.

1. **The rows.** A com probe on Server 2025 and 2022: the apartment calls (first, repeated, mixed models, uninitialized), CoCreateInstance of the enumerator and an unknown class, the enumerator's methods as the runner answers them (EnumAudioEndpoints, the collection's GetCount, GetDefaultAudioEndpoint, and a device's property store if there is one), reference counts, QueryInterface, PropVariantClear, CoCreateGuid's version and variant bits, and the three OLEAUT32 calls.
2. **COM in the guest runtime:** apartments per thread, class creation by CLSID, objects whose vtables are service thunks taking `this`, and the enumerator answering as the runner does.
3. **Amended 2026-10-11, before the work (KIT_DIARY #40):** with COM served, the frontier stops at `WS2_32.dll!#115` (WSAStartup) after 20,705,122 instructions: mono's call at trace seq 1283 comes before UnityPlayer's CoCreateInstance calls (seq 1705), so the done criteria need the network family's WSAStartup too (mono: version 2.0, UnityPlayer: 2.2, both answering 0; if_nametoindex is never called). A network probe gives its rows.

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

- **Closed partial:** the done criterion (the frontier past UnityPlayer's six CoCreateInstance calls, trace seq 1705) is not met. The frontier passes CoInitializeEx and WSAStartup and stops at `KERNEL32.dll!RaiseException` after 20,785,191 instructions (`store/runs/frontier-643a6283510c-1791710059997513000`): mono's thread 8 raises 0x406D1388 with three arguments, MSVC's thread-naming convention, which a `__try`/`__except` in mono catches when no debugger runs. Dispatching it needs structured exception handling (unwinding by .pdata, the language-specific handler, RtlUnwindEx), a concept of its own (hard rule 6); nothing else depends on the criterion, which moves to Sprint 053's (BLACKBOARD Deferred).
- **The com family equals Windows** (contract windows.com.com 0.1.0, 29 rows, runs 38126872187 and 38126873913) with four plants of named reach: apartments, the audio device enumerator as one object with COM's own reference, no audio endpoint as on the runners, notification clients, PropVariantClear, CoCreateGuid, BSTRs, VariantClear.
- **The network family equals Windows** (contract windows.network.network 0.1.0, 13 rows, runs 38127406287 and 38127407661) with two plants: WSAStartup's versions and WSADATA, WSACleanup's balance and last errors.
- **Built:** COM vtables in the runtime image, filled with the services' thunks at the start (no guest allocation moves); imports by ordinal mapped for OLEAUT32 and WS2_32 from Wine's spec files; the service pattern reads a DLL prefix with an underscore (ws2_32).
- **w2m's choice, recorded:** the game sees no audio endpoint, as on the runners (contract open question); sound output is later work.

