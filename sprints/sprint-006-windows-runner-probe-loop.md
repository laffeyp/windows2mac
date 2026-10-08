# Sprint 006 — Windows runner probe loop

```yaml
---
id: 006
status: closed
phase: 1
pass_kind: bridge
---
```

## scope

Put the project in a private GitHub repo (`laffeyp/w2m`, user decision 2026-10-07) and prove the tiebreak loop end to end with a minimal probe. Author `probes/hello/hello.c`, a Win32 C program that writes an 8-line guest trace (two threads, three QueryPerformanceCounter reads, an hb edge from the joined thread), and `.github/workflows/windows-probe.yml`, a `workflow_dispatch` job on `windows-2025` that compiles the probe with clang, runs it, and uploads the trace. From this Mac: dispatch, wait, download, then `w2m.trace.ingest` returns TRACE_INGESTED.

## prerequisites

- 003 (ingester)

## context_files

- `signals/0.1.json` (guest tags, T5, T6)
- `src/w2m/trace.py`
- github.com/actions/runner-images `images/windows/Windows2025-Readme.md` (read 2026-10-07: OS 10.0.26100, LLVM 20.1.8, Visual Studio 2022)
- docs.github.com Actions billing (read 2026-10-07: private repos use included minutes)

## bridge mapping (written before authoring, TECHNIQUES.md #46)

- **Runner:** `runs-on: windows-2025`; Windows Server 2025, x64, build 10.0.26100 (same kernel build as Windows 11 24H2).
- **Compiler:** `clang` (LLVM 20.1.8 on PATH); target `x86_64-pc-windows-msvc`, links against the MSVC/SDK libraries installed with Visual Studio 2022.
- **Dispatch from the Mac:** `gh workflow run windows-probe.yml -f probe=hello -f execution_id=<id>`; `gh run watch <run-id> --exit-status`; `gh run download <run-id> -n trace-<id>`.
- **APIs used by the probe:** `QueryPerformanceCounter`, `CreateThread`, `WaitForSingleObject`, `GetCurrentThreadId`, `RtlGetVersion` (via `GetProcAddress` on ntdll) for the build number in `profile_id`.

## signal contract

### Emits

- Guest plane (in the trace file): GUEST_TRACE_OPENED, GUEST_THREAD_STARTED x2, GUEST_CLOCK_READ x3, GUEST_THREAD_EXITED, GUEST_TRACE_CLOSED.
- Harness plane (from the ingest step on the Mac): TRACE_INGESTED.

### Invariants

- No out-of-vocabulary tags; the trace passes the strict decoder.
- `substrate/` and `substrate-bash-fix/` are not added to the w2m repo.

## artifact contract

### Files created

- `probes/hello/hello.c`
- `.github/workflows/windows-probe.yml`
- `.gitignore`

### Content assertions

- The workflow contains `runs-on: windows-2025` and `workflow_dispatch`.
- `hello.c` writes `"tag":"GUEST_TRACE_OPENED"` first and `"tag":"GUEST_TRACE_CLOSED"` last.

### Command exit codes

- `gh repo create laffeyp/w2m --private --source . --push` returns 0
- `gh run watch <run-id> --exit-status` returns 0
- `uv run python -c "...ingest(...)"` prints `TRACE_INGESTED`
- `uv run pytest` returns 0

## observation contract

### Driving steps

1. `gh workflow run windows-probe.yml -f probe=hello -f execution_id=hello-001`
2. Watch to completion; download artifact `trace-hello-001`.
3. Copy the trace into `store/traces/<sha256>.jsonl`; run `ingest`.

### Expected runtime signals

- TRACE_INGESTED with `line_count=8`, `thread_count=2`, `vocab_version=0.1`.
- The trace's GUEST_TRACE_OPENED `profile_id` names `gha-windows-2025-x64` and a build number of 26100.

## done criteria

A probe written on the Mac runs on real x64 Windows and its trace becomes evidence-grade on the Mac, with one command each way.

## self-review checklist

- [x] Scope concrete.
- [x] Bridge mapping written from the runner README and GitHub docs, not memory.
- [x] Emits are locked tags.
- [x] Assertions gradable.
- [x] Observation contract present.
- [x] Three files (probe, workflow, ignore file), one concept (the runner loop).
