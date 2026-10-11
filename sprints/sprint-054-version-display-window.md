# Sprint 054 — the version check, the display and the game window

```yaml
---
id: 054
status: closed
phase: 5
pass_kind: functional
---
```

## scope

Sprint 053 left the frontier at 41,315,682 instructions, at `KERNEL32.dll!VerSetConditionMask` (Windows trace seq 1524). The reference is the batch run, `store/game-runs/38113342549` (`-batchmode -nographics -logFile`, "Forcing GfxDevice: Null" in its unity.log), milestone A's mode. The other held run, 38113343972, is the play run (a window, Direct3D 11): the two agree call for call up to seq 1538 and part at window creation, where the play run registers one class, creates the main window (style 0xCA0000), calls IMM32 from its window procedure and loads dxgi and d3d11, and the batch run registers a second class with RegisterClassW and creates a hidden window from it (style 0, CW_USEDEFAULT).

**The frontier gave the game no arguments**, so past seq 1538 it would have taken the play path, not the batch run it is judged against. It now passes the runner's batch arguments: `-batchmode -nographics -logFile C:\w2m\unity.log` (C:\ is the file bridge's writable drive).

The unserved functions the batch trace calls between seq 1524 and the last of the six CoCreateInstance calls (seq 1712), listed by a query over the trace against the service table (KIT_DIARY #43), 21 in all:

| Family (specs/work_list.json) | Functions, with the trace's forms |
|---|---|
| version | VerSetConditionMask ×5 (MAJOR, MINOR, SERVICEPACKMAJOR, SERVICEPACKMINOR with VER_GREATER_EQUAL, then PLATFORMID with VER_EQUAL), ntdll RtlVerifyVersionInfo (mask 0x33, through GetProcAddress, STATUS_SUCCESS), SHELL32 DllGetVersion (through GetProcAddress, S_OK) |
| display | GetSystemMetrics 76-79 (the virtual screen: 0, 0, 1024, 768), EnumDisplayDevicesA ×3 (adapter 0, its monitor 0, adapter 1 absent), EnumDisplayMonitors (NULL DC and rectangle, one callback), GetMonitorInfoA, MonitorFromWindow (MONITOR_DEFAULTTONEAREST, 0x10001), shcore GetDpiForMonitor (MDT_EFFECTIVE_DPI, S_OK) |
| windows | LoadIconA (the exe, resource 0x67: NULL, last error 1812), RegisterClassExW, RegisterClassW, CreateWindowExW (atom, style 0, CW_USEDEFAULT ×4), DefWindowProcW (WM_GETMINMAXINFO, WM_NCCREATE, WM_NCCALCSIZE from inside CreateWindowExW), SetWindowLongPtrW (index 0, from the window procedure's WM_CREATE), GetClientRect, LoadCursorA (NULL, IDC_ARROW) |
| time | KERNELBASE GetSystemTimePreciseAsFileTime (through GetProcAddress), WINMM timeGetTime (seq 1707) |
| registry | RegOpenKeyExW (HKEY_CURRENT_USER, KEY_READ: ERROR_FILE_NOT_FOUND) |
| file_search | FindFirstFileExW (FindExInfoBasic: INVALID_HANDLE_VALUE, ERROR_PATH_NOT_FOUND) |

The game's exe carries a manifest that changes Windows's answers: supportedOS Windows 10, 8.1, 8 and 7, `dpiAwareness` PerMonitorV2, Common Controls 6. An unmanifested program is told 6.2 build 9200 and sees scaled metrics; the game is told 10.0 and sees the display unscaled. The probes embed the game's manifest (`-Wl,/manifest:embed -Wl,/manifestinput:games/hail-to-the-rainbow/manifest.xml`, the exe's RT_MANIFEST 1, 1,729 bytes, read by `src/w2m/pe.py`'s new resource table), and w2m reads the manifest of the exe it runs.

**The reference run did not run the game's exe.** `.github/workflows/windows-game.yml` puts jit_capture.exe in its place (built with no manifest and no resources), so the held traces answer as an unmanifested program and LoadIconA(exe, 103) failed there with 1812 (ERROR_RESOURCE_DATA_NOT_FOUND): the game's exe holds icon group 103 (RT_GROUP_ICON, 9 icons), which w2m runs. Where the trace and the game's exe disagree, the probes under the game's manifest and resources judge; jit_capture is now linked with the game's manifest for later captures.

1. **The frontier's command line** is the batch run's; the run keeps the guest's unity.log in its record.
2. **The manifest.** w2m reads the exe's RT_MANIFEST: supportedOS Windows 10 gives version 10.0 (GetVersionExW, VerifyVersionInfoW; the system_info family's unmanifested 6.2 stays), and the DPI awareness the display answers in.
3. **Calling the guest from a service.** Sprint 053's call stub becomes a general mechanism: a service keeps its state in a frame on the guest's stack, calls x86 code, resumes when it returns, and at the end returns to its own caller. The exception dispatcher, the window procedure and EnumDisplayMonitors's callback use it.
4. **The families, each with a probe on Server 2025 and 2022 and plants of named reach:**
   - **version:** the four functions of the work list's family (VerSetConditionMask, RtlVerifyVersionInfo, VerifyVersionInfoW, DllGetVersion) and GetVersionExW, under the game's manifest; conditions true and false, both runners' builds as relations.
   - **display:** GetSystemMetrics over every index 0-99, EnumDisplayDevicesA, EnumDisplayMonitors, GetMonitorInfoA and W, MonitorFromWindow, MonitorFromRect, MonitorFromPoint, GetDpiForMonitor, EnumDisplaySettingsW (ENUM_CURRENT_SETTINGS), DPI-aware as the game is.
   - **windows:** RegisterClassExW and RegisterClassW (atoms, a class registered twice), CreateWindowExW in the batch run's form and the play run's (style 0xCA0000), each message the window procedure receives before CreateWindowExW returns with its parameters, DefWindowProcW's answers to them, SetWindowLongPtrW and GetWindowLongPtrW (extra bytes, GWLP_USERDATA), GetClientRect and GetWindowRect, LoadIconA and LoadCursorA in the game's forms.
   - **time:** the work list's family: GetSystemTimePreciseAsFileTime, timeGetTime, timeBeginPeriod, GetTickCount64, as relations to the clocks w2m serves.
   - **registry:** RegOpenKeyExW and RegCloseKey: a key under HKEY_CURRENT_USER\Software that does not exist, and the argument errors. The registry's writes (RegCreateKeyW, RegSetValueExA, RegQueryValueExA, RegDeleteValueA, first at seq 2661) are not in this sprint.
   - **file_search:** the work list's family: FindFirstFileExW, FindNextFileW and FindClose over a directory the probe makes, a missing directory and a missing file.

5. **Amended 2026-10-11, before the work (KIT_DIARY #44):** the first frontier run with the batch arguments stopped at `KERNEL32.dll!SetConsoleCtrlHandler` after 2,576,452 instructions (`store/runs/frontier-643a6283510c-1791750587644112000`), and the listing over the whole batch trace (seq 1 to 1712), not only from 1524, shows w2m off the Windows path from its first traced calls: UnityPlayer's `VirtualAlloc(NULL, 0x3000, MEM_COMMIT, PAGE_NOACCESS)` (seq 2) reserves and commits on Windows (0x2bac2000000) and fails in w2m with ERROR_INVALID_ADDRESS (`src/w2m/virtual_memory.py` takes a commit without MEM_RESERVE as a commit into an existing reservation, also at a NULL address), so Unity skips the crash handler's start. Fixed here with a virtual_memory row (MEM_COMMIT alone at NULL) and a plant. The crash handler's start that follows (seq 6-18: CreateEventW ×3, CreatePipe, SetHandleInformation, CreateProcessW of UnityCrashHandler64.exe, then pipe writes and zero-timeout waits on the process), with RoInitialize (seq 75) and SetConsoleCtrlHandler (seq 419), is a second process: a concept of its own (hard rule 6), Sprint 055. This sprint's frontier criterion becomes: in batch mode the frontier passes seq 2 as Windows does and stops at the crash handler's start. Measured after the fix: it stops at seq 6, `CreateEventW` with a name (the first of the three events the handler inherits), after 421,498 instructions (`store/runs/frontier-643a6283510c-1791750833656913000`); named events go to Sprint 055 with the rest of the handler's start. The families of item 4 are judged by their rows, which do not depend on the frontier's order.

w2m's display is the runners' (one 1024×768 monitor, 96 DPI) until the game's window is a Mac window (phase 6); the contract records it as an open question, as the com family records the audio endpoint.

## prerequisites

- 053 (closed partial: CI run 38166790366 on 539c238c; main 8e65ea76)

## context_files

- `store/game-runs/38113342549/jit_capture.bin.trace.tsv`, `store/game-runs/38113343972/` (the play run), `specs/work_list.json`, `scripts/run_frontier.py`, `.github/workflows/windows-game.yml` (the batch arguments), `tools/guest_runtime/exceptions.c` (the call stub), `system_info.c`, `src/w2m/file_bridge.py`, `src/w2m/vm_backend.py`; the game's manifest (`Hail to the Rainbow.exe`, RT_MANIFEST 1); Microsoft Learn: VerifyVersionInfoW, VerSetConditionMask, GetSystemMetrics, EnumDisplayDevicesA, EnumDisplayMonitors, GetMonitorInfoA, GetDpiForMonitor, RegisterClassExW, CreateWindowExW, DefWindowProcW, SetWindowLongPtrW, FindFirstFileExW, RegOpenKeyExW, timeGetTime

## signal contract

### Emits

- Per family: the item run. Per frontier run: EXECUTION_STARTED, then SERVICE_UNIMPLEMENTED or EXECUTION_COMPLETED.

### Invariants

- Vocabulary 0.7; no new tags.

## artifact contract

- created: `probes/version/`, `probes/display/`, `probes/windows/`, `probes/time/`, `probes/registry/`, `probes/file_search/`; their contracts; the guest runtime's services for them; the guest-call mechanism
- modified: `scripts/run_frontier.py` (the batch arguments, the log kept), `tools/guest_runtime/exceptions.c` (the shared call stub), `system_info.c` (GetVersionExW under a manifest), `src/w2m/vm_backend.py` and the loader (the manifest), `scripts/family_run.py`
- gates 0 and CI `success` on the closing commit

## observation contract

- **Each of the six families equals Windows** on its probe's rows on Server 2025 and 2022, with plants of named reach; the exceptions family stays equal on the shared call stub.
- **The frontier, in batch mode, passes seq 2** (VirtualAlloc with MEM_COMMIT alone) and stops at the crash handler's start (seq 6, a named `CreateEventW`); the virtual_memory family stays equal with its new row. (Amended: seq 1712 moves to Sprint 055.)

## done criteria

The six families accepted; virtual_memory re-accepted with the commit-at-NULL row; the batch frontier at the crash handler's start (seq 6).

## observed

- **The six families equal Windows** on Server 2025 and 2022, each with plants caught by exactly the rows named:
  version (98 rows, 5 plants; runs 38173733146, 38173735116), display (147 rows, 6; 38174316653, 38174318347), windows
  (36 rows, 7; 38174244880, 38174246839), time (12 rows, 4; 38173127764, 38173129974), registry (16 rows, 4;
  38173241012, 38173242936), file_search (26 rows, 5; 38173241103, 38173242874). virtual_memory 0.2.0 equals Windows
  with the commit-at-NULL rows (38172734444, 38172736319; plant commit_null_refused, 8 rows).
- **The frontier, given the batch run's arguments, passes seq 2 and stops at the crash handler's start**: seq 6,
  `CreateEventW` with a name, after 421,498 instructions (`store/runs/frontier-643a6283510c-1791752756583708000`), as
  the amended criterion says. The 21 functions of the scope come after it on the batch path; they are served and
  judged by their rows.
- **Windows facts the rows fixed:** under the game's manifest GetVersionExW answers 10.0 (6.2 without one) and suite
  mask 0x190; VerSetConditionMask always sets bit 63 and answers 0 for a type mask of 0; RtlVerifyVersionInfo compares
  major, minor and the service pack as one version and skips a type whose condition is 0; shell32 reports 10.0.10586
  on both runners. A window's creation sends exactly WM_GETMINMAXINFO, WM_NCCREATE, WM_NCCALCSIZE, WM_CREATE;
  CREATESTRUCT keeps cx and cy as given (CW_USEDEFAULT) with x and y resolved; CW_USEDEFAULT cascades in steps of 26
  from a start the desktop's earlier windows set (52, 78, 130 seen) and sizes 3/4 of the work area, less 21 in height
  (519 on 2025's 720, 525 on 2022's 728). A find lists "." and "..", then names compared without case, a folder with
  size 0 (GetFileAttributesExW says 4096). A NULL or empty registry subkey returns the root's predefined handle.
  LoadIconA of a missing id is 1813 when the module has resources and 1812 when it has none.
- **Defects found and fixed:** the frontier gave the game no arguments (the play path) while every criterion since
  052 was read from the batch run (KIT_DIARY #44); `VirtualAlloc(NULL, n, MEM_COMMIT)` failed in w2m and reserves and
  commits on Windows, which sent UnityPlayer around its crash handler; modules.py compared lowercased names with
  System32's spellings, so `LoadLibrary("shcore.dll")` would have answered ERROR_MOD_NOT_FOUND; a system DLL the
  program does not import could not be loaded at all; the reference captures ran jit_capture.exe in the game exe's
  place, unmanifested and without its icons (KIT_DIARY #45); link.exe's /manifestinput needs mt.exe, which the
  runners lack (manifests now go in as resources through llvm-rc).
- **Built:** `tools/guest_runtime/guest_calls.c` (a service calls the guest's code and resumes; exception dispatch moved
  onto it, its four families conforming with every plant as named), `windows.c`, `display.c`, `version.c`, `time.c`,
  `registry.c`, `file_search.c`, `resources.c`; `src/w2m/guest_stubs.py` (was exception_stubs.py); FILE_LIST (host
  request 32); PE resource parsing (`pe.py`); system DLLs loaded by name at run time; `scripts/store_probe_runs.py`;
  the probe workflow's `rc` (llvm-rc); `games/hail-to-the-rainbow/manifest.xml`.
- **Closed on CI run 38175121068** (success on 2b02046a).
