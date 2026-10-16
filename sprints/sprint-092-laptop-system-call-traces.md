# Sprint 092 — the laptop's system-call traces: DTrace installed, E1 (a process's start) and E3 (calls per frame)

```yaml
---
id: 092
status: closed
phase: 5
pass_kind: functional
---
```

## scope

Step 1 of the solution (docs/solution-2026-10-16.md, section 6), the two recordings on the laptop. Both questions
were searched first: the shape of process start and the tracer are published (solution 4.2, 4.3); what remains is
build 26200.7840's own call list and the game's rate.

- **DTrace on the laptop, as Microsoft documents it** (Microsoft Learn, "DTrace on Windows"): the MSI from the
  Download Center (id 100441), Complete install; BitLocker suspended for one restart (`Suspend-BitLocker
  -RebootCount 1`; the page warns a boot-value change under BitLocker can demand the recovery key); `bcdedit /set
  dtrace ON`; restart. Settled when `dtrace -l` lists syscall probes (not only BEGIN, END, ERROR). Recorded: how many
  syscall probes, and whether win32k's calls are among them (the page says "NTOS system calls"); if not, win32k's
  are counted by the pid provider on win32u's exports.
- **E1, a process's start.** `probes/hello` (built for Windows) run under DTrace with `-c`: every system call of that
  process, entry and return, in order, with its six first arguments and its result, from the first to exit; the
  probe's entry point marked by the pid provider. For the ALPC and LPC connects (NtAlpcConnectPort,
  NtAlpcConnectPortEx, NtConnectPort, NtSecureConnectPort) the port's name is read from its UNICODE_STRING. Settled
  by: the list before the entry point, the number of calls, the distinct calls, and the ports.
- **E3, calls per frame.** Hail to the Rainbow on the laptop as in run lap-20261014T041748433Z-73c66e6a-01, past
  the first scene; for 10 s: system calls by name, win32u calls by name, and frames (dxgi's Present calls). Settled
  by: calls per frame, each kind.

## prerequisites

- 091 (committed e1b41f1b)

## context_files

- docs/solution-2026-10-16.md; scripts/windows_machine.py, scripts/record_game_run.py (the laptop's runs);
  probes/hello/hello.c

## signal contract

### Emits

- None new (recordings on the laptop, kept as data).

### Invariants

- Vocabulary unchanged.

## artifact contract

- `scripts/laptop_dtrace.py`: installs nothing; runs a D script on the laptop over ssh and fetches its output
- `store/laptop-traces/e1-hello.tsv.gz` (E1's calls), `store/laptop-traces/e3-hail-to-the-rainbow.tsv` (E3's
  counts), each with the build and the D script's sha256
- `scripts/gates.sh` exit 0; CI `success`

## observation contract

- E1: the trace holds hello's own known calls (its WriteFile reaches NtWriteFile with the bytes' length) in order,
  and ends with NtTerminateProcess.
- E3: the frame count over 10 s agrees with the game's 112 frames a second within the scene's spread.

## done criteria

DTrace runs on the laptop; E1's list and E3's rates are committed with their builds, each passing its observation.

## observed
- **DTrace on the laptop.** Installed as Microsoft documents it (MSI exit 3010; `bcdedit /set dtrace ON`; restart
  2026-10-16 20:57:06Z with BitLocker suspended for that boot, protection On after). The syscall provider lists 486
  NTOS calls (972 probes) and no win32k call (Microsoft's issue #23: none planned). Against the copied ntdll.dll's 488
  system-call stubs it has no probe for three (NtAlertMultipleThreadByThreadId, NtAlertThreadByThreadIdEx,
  NtSetEventEx) and one probe (NtQuerySystemTime) whose ntdll export is a jump, not a stub. The pid provider lists
  every one of win32u's 1,486 stubs and 7 of its 48 other exports.
- **E1, a process's start** (`store/laptop-traces/process-start-hello.tsv.gz`, not `e1-hello.tsv.gz` as the
  contract named it). probes/hello on build 26200.7840: 483 system calls of 73 kinds; 342 (62 kinds) before main's
  first act, its NtCreateFile of `trace.txt`. Its one port is `\Windows\ApiPort` (NtConnectPort: csrss). Before main
  it opens `\KnownDlls` and the KnownDllPath link, maps the KERNEL32 and KERNELBASE sections and
  `\Windows\SharedSection`, opens `\Connect` (the console), `\BaseNamedObjects`, and the keys Nls CodePage, Session
  Manager, Segment Heap, Image File Execution Options\hello.exe, Srp, Safer and SafeBoot. Its two NtWriteFile calls
  carry 0x3b4 bytes (trace.txt is 948) and 0x41 (the console's 65). The first two calls, on the thread that builds
  the process, report pid 0 and kernel addresses. The trace does not end with NtTerminateProcess, as the contract
  said: that call comes 461st and 468th of 483, and the last 15 are token, process and key queries after it. Limits:
  arguments past arg9 are not recorded (issue #66; NtCreateFile has eleven); execname is cut to 14 characters, so
  process_start.d's exact match serves short image names only; the three unprobed stubs would not show.
- **E3, calls per frame** (`store/laptop-traces/frame-calls-hail-to-the-rainbow.tsv`, not `e3-...`). Hail to the
  Rainbow, run lap-20261016T222633417Z-019003e3-01, play mode, DTrace attached at the game's second 60 and counting
  10.012 s from its own second 20: 1,132 frames (NtGdiDdDDIPresent in the window; presents.tsv's DXGI Present starts
  at seconds 80-89: 1,132), 113 a second against the earlier runs' 112. Per frame: 314.1 NT system calls (355,523;
  4,114 of them NtAlertThreadByThreadIdEx, counted by the pid provider at ntdll's stub), led by NtDelayExecution
  (114.3), NtWaitForSingleObject (99.9) and NtReleaseSemaphore (64.0); 18.4 win32k system calls (20,836), led by
  NtGdiDdDDISubmitCommandToHwQueue (5.0), NtGdiDdDDISignalSynchronizationObjectFromGpu2 (3.0) and one each of
  NtGdiDdDDIPresent and NtNotifyPresentToCompositionSurface. ABI_Get_ForegroundWindow (1,129) is a win32u export
  that enters no kernel and is kept apart. Tracing changed the frame rate by no measured amount: the run gave 112.5
  frames a second before the attach (seconds 40-59) and 112.8 after (60-99), each second of 70-99 between 111 and 115.
- **Defects found on the way, by class.** E3 failed three times, one defect each: `strstr() != NULL` is false on a
  match in this DTrace (index() works); `60 * 1000000000` overflows 32 bits in D (-129,542,144); a new process's first
  two calls report pid 0. The user then asked for each error's class to be searched whole before another run; the
  search (Microsoft's page, the repository's 30 issues, short tests) found: the three unprobed stubs; a win32u export
  counted as a system call; frames taken from a second clock; a helper that dropped the laptop's stderr; PowerShell
  reading `$e:win32u` as a drive variable; a window start never set when FROM is 0; `-o` appending, not replacing
  (both scripts delete first); and in E1's summary, main's own NtCreateFile counted before main, because an entry
  and its name share a timestamp (343 became 342). Each is fixed or recorded above; the run now fails on no frames,
  any DTrace stderr line, a window off 10 s by 0.1 s, or the two frame counts apart by 1%.
- **Observation contract.** E1: hello's known calls are there in order with their lengths; the clause "ends with
  NtTerminateProcess" was wrong about Windows and is corrected above. E3: 1,132 frames in 10.012 s, 113 a second,
  within the scene's spread of the earlier 112.
