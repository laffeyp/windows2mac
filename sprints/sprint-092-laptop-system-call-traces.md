# Sprint 092 — the laptop's system-call traces: DTrace installed, E1 (a process's start) and E3 (calls per frame)

```yaml
---
id: 092
status: active
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
