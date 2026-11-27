# Unserved declared imports, 2026-11-27 (Sprint 368)

The class Sprint 368's run-stopping defect names. A two-vCPU trial run of the game stopped "import
KERNELBASE.dll!SleepConditionVariableCS" (`build/batch_run/record-2-vcpus-1795761590000000000`): a thread the second
vCPU ran reached a function the reference run never called and the runtime did not serve. The same can happen with
any function the game declares and the reference did not reach, since a two-vCPU schedule reaches waits the
reference's did not.

**The rule.** A KERNEL32.dll import that UnityPlayer.dll or mono-2.0-bdwgc.dll declares, that the Windows batch run
called 0 times, and that no service in `tools/guest_runtime` serves.

- The declarations and their calls: `store/game-runs/38006665419/jit_capture.bin.imports.tsv` (importer, DLL,
  function, the calls the capture counted), 803 rows, 434 of them KERNEL32.dll, 271 of those with 0 calls.
- The services: every `k32_<Name>` the runtime's C defines, `grep -ho "k32_[A-Za-z0-9_]*" tools/guest_runtime/*.c`,
  196 names. A name with no `k32_` service was also searched by its bare name in `tools/guest_runtime/*.c`,
  `tools/guest_runtime/*.h` and `crates/w2m_vm/src/*.rs` for the 13 waits checked by hand (SignalObjectAndWait,
  QueueUserAPC, RegisterWaitForSingleObject, UnregisterWait, UnregisterWaitEx, CreateIoCompletionPort,
  GetQueuedCompletionStatus, CreateToolhelp32Snapshot, QueryDepthSList, CreateMutexW, OpenEventW, SetWaitableTimer,
  GetNumberOfConsoleInputEvents): no match.

The rule gives 108 functions in 140 rows (a function two images import is a row for each). None is served in Sprint
368; the next cards serve them.

| kind | functions |
|---|---|
| waits and threads | 25 |
| files | 31 |
| other | 15 |
| memory | 10 |
| console | 9 |
| locale | 9 |
| unwind | 9 |

**Served first.** The 26 the next cards take first: the 25 waits and threads and GetNumberOfConsoleInputEvents, a
console function by kind, in the list with them. They are the 17 of
the review's list (mono-2.0-bdwgc.dll's CreateMutexW, OpenMutexW, OpenEventW, OpenSemaphoreW, SetThreadContext,
DisableThreadLibraryCalls, GetNumberOfConsoleInputEvents; UnityPlayer.dll's CreateTimerQueue, CreateTimerQueueTimer,
ChangeTimerQueueTimer, DeleteTimerQueueTimer, CreateWaitableTimerA, SetWaitableTimer, OpenEventA, Thread32First,
Thread32Next; FreeLibraryAndExitThread from both) and nine more: SignalObjectAndWait (both), QueueUserAPC
(mono-2.0-bdwgc.dll), RegisterWaitForSingleObject, UnregisterWait, UnregisterWaitEx, CreateIoCompletionPort,
GetQueuedCompletionStatus, CreateToolhelp32Snapshot and QueryDepthSList (UnityPlayer.dll).

| importer | dll | function | reference calls | kind |
|---|---|---|---|---|
| UnityPlayer.dll | KERNEL32.dll | AttachConsole | 0 | console |
| mono-2.0-bdwgc.dll | KERNEL32.dll | CancelIo | 0 | files |
| UnityPlayer.dll | KERNEL32.dll | CancelIo | 0 | files |
| mono-2.0-bdwgc.dll | KERNEL32.dll | CancelIoEx | 0 | files |
| mono-2.0-bdwgc.dll | KERNEL32.dll | CancelSynchronousIo | 0 | files |
| UnityPlayer.dll | KERNEL32.dll | ChangeTimerQueueTimer | 0 | waits and threads |
| mono-2.0-bdwgc.dll | KERNEL32.dll | CompareStringW | 0 | locale |
| UnityPlayer.dll | KERNEL32.dll | CompareStringW | 0 | locale |
| mono-2.0-bdwgc.dll | KERNEL32.dll | CopyFileW | 0 | files |
| UnityPlayer.dll | KERNEL32.dll | CopyFileW | 0 | files |
| mono-2.0-bdwgc.dll | KERNEL32.dll | CreateFileA | 0 | files |
| UnityPlayer.dll | KERNEL32.dll | CreateFileA | 0 | files |
| UnityPlayer.dll | KERNEL32.dll | CreateIoCompletionPort | 0 | waits and threads |
| mono-2.0-bdwgc.dll | KERNEL32.dll | CreateMutexW | 0 | waits and threads |
| mono-2.0-bdwgc.dll | KERNEL32.dll | CreateProcessA | 0 | other |
| UnityPlayer.dll | KERNEL32.dll | CreateTimerQueue | 0 | waits and threads |
| UnityPlayer.dll | KERNEL32.dll | CreateTimerQueueTimer | 0 | waits and threads |
| UnityPlayer.dll | KERNEL32.dll | CreateToolhelp32Snapshot | 0 | waits and threads |
| UnityPlayer.dll | KERNEL32.dll | CreateWaitableTimerA | 0 | waits and threads |
| UnityPlayer.dll | KERNEL32.dll | DebugBreak | 0 | other |
| UnityPlayer.dll | KERNEL32.dll | DeleteTimerQueueTimer | 0 | waits and threads |
| mono-2.0-bdwgc.dll | KERNEL32.dll | DisableThreadLibraryCalls | 0 | waits and threads |
| mono-2.0-bdwgc.dll | KERNEL32.dll | EnumSystemLocalesW | 0 | locale |
| UnityPlayer.dll | KERNEL32.dll | EnumSystemLocalesW | 0 | locale |
| UnityPlayer.dll | KERNEL32.dll | ExpandEnvironmentStringsW | 0 | other |
| mono-2.0-bdwgc.dll | KERNEL32.dll | FindFirstFileExA | 0 | files |
| mono-2.0-bdwgc.dll | KERNEL32.dll | FindFirstFileW | 0 | files |
| UnityPlayer.dll | KERNEL32.dll | FindFirstFileW | 0 | files |
| mono-2.0-bdwgc.dll | KERNEL32.dll | FindNextFileA | 0 | files |
| UnityPlayer.dll | KERNEL32.dll | FlushConsoleInputBuffer | 0 | console |
| mono-2.0-bdwgc.dll | KERNEL32.dll | FlushViewOfFile | 0 | files |
| UnityPlayer.dll | KERNEL32.dll | FormatMessageA | 0 | other |
| mono-2.0-bdwgc.dll | KERNEL32.dll | FreeConsole | 0 | console |
| mono-2.0-bdwgc.dll | KERNEL32.dll | FreeLibraryAndExitThread | 0 | waits and threads |
| UnityPlayer.dll | KERNEL32.dll | FreeLibraryAndExitThread | 0 | waits and threads |
| mono-2.0-bdwgc.dll | KERNEL32.dll | GetComputerNameW | 0 | other |
| UnityPlayer.dll | KERNEL32.dll | GetComputerNameW | 0 | other |
| mono-2.0-bdwgc.dll | KERNEL32.dll | GetDateFormatW | 0 | locale |
| UnityPlayer.dll | KERNEL32.dll | GetDateFormatW | 0 | locale |
| mono-2.0-bdwgc.dll | KERNEL32.dll | GetDiskFreeSpaceExW | 0 | files |
| UnityPlayer.dll | KERNEL32.dll | GetDiskFreeSpaceExW | 0 | files |
| mono-2.0-bdwgc.dll | KERNEL32.dll | GetDriveTypeW | 0 | files |
| UnityPlayer.dll | KERNEL32.dll | GetDriveTypeW | 0 | files |
| mono-2.0-bdwgc.dll | KERNEL32.dll | GetEnvironmentVariableA | 0 | other |
| UnityPlayer.dll | KERNEL32.dll | GetEnvironmentVariableA | 0 | other |
| UnityPlayer.dll | KERNEL32.dll | GetFileAttributesA | 0 | files |
| UnityPlayer.dll | KERNEL32.dll | GetFileInformationByHandle | 0 | files |
| mono-2.0-bdwgc.dll | KERNEL32.dll | GetFileTime | 0 | files |
| mono-2.0-bdwgc.dll | KERNEL32.dll | GetFullPathNameA | 0 | files |
| mono-2.0-bdwgc.dll | KERNEL32.dll | GetLocaleInfoW | 0 | locale |
| UnityPlayer.dll | KERNEL32.dll | GetLocaleInfoW | 0 | locale |
| mono-2.0-bdwgc.dll | KERNEL32.dll | GetLogicalDriveStringsW | 0 | files |
| mono-2.0-bdwgc.dll | KERNEL32.dll | GetNumberOfConsoleInputEvents | 0 | console |
| UnityPlayer.dll | KERNEL32.dll | GetOverlappedResult | 0 | files |
| mono-2.0-bdwgc.dll | KERNEL32.dll | GetPriorityClass | 0 | other |
| mono-2.0-bdwgc.dll | KERNEL32.dll | GetProcessTimes | 0 | other |
| mono-2.0-bdwgc.dll | KERNEL32.dll | GetProcessWorkingSetSize | 0 | memory |
| UnityPlayer.dll | KERNEL32.dll | GetQueuedCompletionStatus | 0 | waits and threads |
| UnityPlayer.dll | KERNEL32.dll | GetStartupInfoA | 0 | other |
| UnityPlayer.dll | KERNEL32.dll | GetSystemDirectoryA | 0 | files |
| UnityPlayer.dll | KERNEL32.dll | GetSystemPowerStatus | 0 | other |
| mono-2.0-bdwgc.dll | KERNEL32.dll | GetSystemTimes | 0 | other |
| mono-2.0-bdwgc.dll | KERNEL32.dll | GetTimeFormatW | 0 | locale |
| UnityPlayer.dll | KERNEL32.dll | GetTimeFormatW | 0 | locale |
| mono-2.0-bdwgc.dll | KERNEL32.dll | GetUserDefaultLCID | 0 | locale |
| UnityPlayer.dll | KERNEL32.dll | GetUserDefaultLCID | 0 | locale |
| mono-2.0-bdwgc.dll | KERNEL32.dll | GetVolumeInformationW | 0 | files |
| mono-2.0-bdwgc.dll | KERNEL32.dll | GlobalAlloc | 0 | memory |
| UnityPlayer.dll | KERNEL32.dll | GlobalAlloc | 0 | memory |
| mono-2.0-bdwgc.dll | KERNEL32.dll | GlobalFree | 0 | memory |
| UnityPlayer.dll | KERNEL32.dll | GlobalLock | 0 | memory |
| UnityPlayer.dll | KERNEL32.dll | GlobalMemoryStatus | 0 | memory |
| mono-2.0-bdwgc.dll | KERNEL32.dll | GlobalReAlloc | 0 | memory |
| UnityPlayer.dll | KERNEL32.dll | GlobalUnlock | 0 | memory |
| UnityPlayer.dll | KERNEL32.dll | HeapQueryInformation | 0 | memory |
| mono-2.0-bdwgc.dll | KERNEL32.dll | IsValidLocale | 0 | locale |
| UnityPlayer.dll | KERNEL32.dll | IsValidLocale | 0 | locale |
| UnityPlayer.dll | KERNEL32.dll | LoadLibraryA | 0 | other |
| mono-2.0-bdwgc.dll | KERNEL32.dll | LockFile | 0 | files |
| mono-2.0-bdwgc.dll | KERNEL32.dll | MoveFileExW | 0 | files |
| UnityPlayer.dll | KERNEL32.dll | MoveFileExW | 0 | files |
| mono-2.0-bdwgc.dll | KERNEL32.dll | MoveFileW | 0 | files |
| UnityPlayer.dll | KERNEL32.dll | OpenEventA | 0 | waits and threads |
| mono-2.0-bdwgc.dll | KERNEL32.dll | OpenEventW | 0 | waits and threads |
| mono-2.0-bdwgc.dll | KERNEL32.dll | OpenFileMappingW | 0 | files |
| mono-2.0-bdwgc.dll | KERNEL32.dll | OpenMutexW | 0 | waits and threads |
| mono-2.0-bdwgc.dll | KERNEL32.dll | OpenSemaphoreW | 0 | waits and threads |
| mono-2.0-bdwgc.dll | KERNEL32.dll | PeekConsoleInputA | 0 | console |
| mono-2.0-bdwgc.dll | KERNEL32.dll | PeekNamedPipe | 0 | files |
| UnityPlayer.dll | KERNEL32.dll | PeekNamedPipe | 0 | files |
| UnityPlayer.dll | KERNEL32.dll | QueryDepthSList | 0 | waits and threads |
| mono-2.0-bdwgc.dll | KERNEL32.dll | QueueUserAPC | 0 | waits and threads |
| mono-2.0-bdwgc.dll | KERNEL32.dll | ReadConsoleInputA | 0 | console |
| UnityPlayer.dll | KERNEL32.dll | ReadConsoleInputW | 0 | console |
| mono-2.0-bdwgc.dll | KERNEL32.dll | ReadConsoleW | 0 | console |
| UnityPlayer.dll | KERNEL32.dll | ReadConsoleW | 0 | console |
| UnityPlayer.dll | KERNEL32.dll | ReadProcessMemory | 0 | memory |
| UnityPlayer.dll | KERNEL32.dll | RegisterWaitForSingleObject | 0 | waits and threads |
| mono-2.0-bdwgc.dll | KERNEL32.dll | ReplaceFileW | 0 | files |
| UnityPlayer.dll | KERNEL32.dll | ReplaceFileW | 0 | files |
| mono-2.0-bdwgc.dll | KERNEL32.dll | RtlCaptureContext | 0 | unwind |
| UnityPlayer.dll | KERNEL32.dll | RtlCaptureContext | 0 | unwind |
| mono-2.0-bdwgc.dll | KERNEL32.dll | RtlDeleteFunctionTable | 0 | unwind |
| mono-2.0-bdwgc.dll | KERNEL32.dll | RtlInstallFunctionTableCallback | 0 | unwind |
| mono-2.0-bdwgc.dll | KERNEL32.dll | RtlLookupFunctionEntry | 0 | unwind |
| UnityPlayer.dll | KERNEL32.dll | RtlLookupFunctionEntry | 0 | unwind |
| UnityPlayer.dll | KERNEL32.dll | RtlPcToFileHeader | 0 | unwind |
| UnityPlayer.dll | KERNEL32.dll | RtlUnwind | 0 | unwind |
| UnityPlayer.dll | KERNEL32.dll | RtlUnwindEx | 0 | unwind |
| mono-2.0-bdwgc.dll | KERNEL32.dll | RtlVirtualUnwind | 0 | unwind |
| UnityPlayer.dll | KERNEL32.dll | RtlVirtualUnwind | 0 | unwind |
| mono-2.0-bdwgc.dll | KERNEL32.dll | SetConsoleMode | 0 | console |
| UnityPlayer.dll | KERNEL32.dll | SetConsoleMode | 0 | console |
| mono-2.0-bdwgc.dll | KERNEL32.dll | SetEndOfFile | 0 | files |
| UnityPlayer.dll | KERNEL32.dll | SetEndOfFile | 0 | files |
| mono-2.0-bdwgc.dll | KERNEL32.dll | SetEnvironmentVariableA | 0 | other |
| mono-2.0-bdwgc.dll | KERNEL32.dll | SetFileAttributesW | 0 | files |
| UnityPlayer.dll | KERNEL32.dll | SetFileAttributesW | 0 | files |
| mono-2.0-bdwgc.dll | KERNEL32.dll | SetFilePointer | 0 | files |
| UnityPlayer.dll | KERNEL32.dll | SetFilePointer | 0 | files |
| mono-2.0-bdwgc.dll | KERNEL32.dll | SetFileTime | 0 | files |
| UnityPlayer.dll | KERNEL32.dll | SetFileTime | 0 | files |
| mono-2.0-bdwgc.dll | KERNEL32.dll | SetPriorityClass | 0 | other |
| mono-2.0-bdwgc.dll | KERNEL32.dll | SetProcessWorkingSetSize | 0 | memory |
| mono-2.0-bdwgc.dll | KERNEL32.dll | SetThreadContext | 0 | waits and threads |
| UnityPlayer.dll | KERNEL32.dll | SetWaitableTimer | 0 | waits and threads |
| mono-2.0-bdwgc.dll | KERNEL32.dll | SignalObjectAndWait | 0 | waits and threads |
| UnityPlayer.dll | KERNEL32.dll | SignalObjectAndWait | 0 | waits and threads |
| mono-2.0-bdwgc.dll | KERNEL32.dll | TerminateProcess | 0 | other |
| UnityPlayer.dll | KERNEL32.dll | TerminateProcess | 0 | other |
| UnityPlayer.dll | KERNEL32.dll | Thread32First | 0 | waits and threads |
| UnityPlayer.dll | KERNEL32.dll | Thread32Next | 0 | waits and threads |
| mono-2.0-bdwgc.dll | KERNEL32.dll | TzSpecificLocalTimeToSystemTime | 0 | locale |
| mono-2.0-bdwgc.dll | KERNEL32.dll | UnhandledExceptionFilter | 0 | unwind |
| UnityPlayer.dll | KERNEL32.dll | UnhandledExceptionFilter | 0 | unwind |
| mono-2.0-bdwgc.dll | KERNEL32.dll | UnlockFile | 0 | files |
| mono-2.0-bdwgc.dll | KERNEL32.dll | UnmapViewOfFile | 0 | files |
| UnityPlayer.dll | KERNEL32.dll | UnregisterWait | 0 | waits and threads |
| UnityPlayer.dll | KERNEL32.dll | UnregisterWaitEx | 0 | waits and threads |
| mono-2.0-bdwgc.dll | KERNEL32.dll | VerLanguageNameW | 0 | locale |
