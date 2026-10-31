# Sprint 290 — LoadLibrary reaches guest memory through the view

```yaml
---
id: 290
status: done
phase: 6
pass_kind: functional
---
```

## scope

Census row D7's next module, by run-time access: `crates/w2m_vm/src/modules.rs` serves the guest's LoadLibrary
(`host_request.rs` 428, `modules::module`) while the run runs, and its four slice calls write guest memory then: a
system DLL's synthetic page (146), a loaded DLL's image (189), its import slots (208), and PEB.Ldr's lists (237). With
a second vCPU, another thread may read those pages, the loader's lists above all, as the request is served. Of the 31
guest slices left, the others run at load, in tests, between cases, or in one-vCPU diagnostics; `turns.rs`' two serve
a child process the supervisor runs alone, and `monitor.rs`' two reads are test helpers' (`MonitorMemory::read_u*`,
called from `tests/` only).

- **The rows of the design's table they cite** (`docs/design-guest-memory-on-several-vcpus-2026-10-31.md`): guest
  buffers (a DLL's image and import slots, in the arena); the process area (the synthetic pages, the loader data and
  PEB's `Ldr`).
- **The work:**
  - `process.rs`: `ProcessBytes`, where the process area's writers put bytes, implemented by `[u8]` (the area as a
    slice, at load, no vCPU running) and by `ProcessView` (the view, `MONITOR_SIZE` past guest memory's start); its
    helpers (`put`, `q`, `d`, `h`, `unicode_string`, `synthetic_optional_header`), `synthetic_image` and
    `write_loader_data` generic over it, each writing the bytes it writes now and no others.
  - `modules.rs`: the image and the import slots written through `guest::memory_view`; `synthetic_image` and
    `write_loader_data` given a `ProcessView`.
  - `tests/slice_calls.rs` and census D7 lowered together: `modules.rs` 0 guest slices (its one `parts.as_slice()`, a
    `Vec`, stays counted as Sprint 287 named it).
- **Not here:** the load-time writers (`guest.rs`' 5, `process.rs`' callers at load), `monitor.rs`, `turns.rs`.

## prerequisites

- 289 (closed)

## context_files

- `crates/w2m_vm/src/modules.rs` 105-247; `crates/w2m_vm/src/process.rs` 150-210, 513-614
- `crates/w2m_vm/src/host_request.rs` 420-440; `crates/w2m_vm/tests/slice_calls.rs`
- `docs/design-guest-memory-on-several-vcpus-2026-10-31.md` (the table); `docs/step-6-census-2026-10-30.md` (D7)
- WORKING_AGREEMENT, "A plant's judge" and "Before a work commit"

## signal contract

### Emits

- None; an access path, held by the batch run, the family runs and the process area's tests.

### Invariants

- The batch run's first stop and call stream, and the family runs, as held: the game loads `mono-2.0-bdwgc.dll` by
  LoadLibrary after its start, and families load DLLs and system DLLs the same way.

## artifact contract

### Files modified

- `crates/w2m_vm/src/process.rs`, `crates/w2m_vm/src/modules.rs`, `crates/w2m_vm/tests/slice_calls.rs`,
  `docs/step-6-census-2026-10-30.md`

### Command exit codes

- `scripts/gates.sh` returns 0

## observation contract

- **Expected values:** `modules.rs` holds no guest-memory slice call, 27 left in `src/`; the batch run's stop and
  calls and the family runs as held; `process.rs`' tests unchanged and passing.
- **Planted faults**, each judge found by lookup to reach its line and to assert what the plant breaks:
  - (1) `ProcessView::put` writing one byte past its offset. Judge: the batch run's readers, `batch_run.rs`' stop
    and call stream, the game's LoadLibrary of Mono writing PEB.Ldr and the image through it; to be found by running.
  - (2) the import slots written to the slot's offset plus 8. Judge: the same, a loaded DLL's calls going astray.

## done criteria

LoadLibrary at run time writes guest memory only through the view, and the runs that load DLLs stand as held.

## self-review checklist

- [x] Scope names one verifiable property: LoadLibrary's writes through the view.
- [x] context_files cite the module, the process area's writers, the request, the count, the table and the rules.
- [x] Emits: none.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and two plants, their judges named and to be confirmed by running.
- [x] One concept: one service's access moved.

## result

- **The plants' judge, found by lookup before they run** (the card left it to running, against WORKING_AGREEMENT's
  rule of 2026-10-31): the `late_tls` family (`store/family_runs/known-2026-10-29.tsv`, contract
  `windows.loader.late_tls@0.1.0`) loads its beside DLL, `late_tls_dll.dll`, by LoadLibrary after the start, `load_dll`
  writing its image, its import slots and PEB.Ldr; `family_runs.rs`' `every_family_run_conforms_or_is_held_by_what_it_waits_on`
  holds its rows to the contract's. No unit test reaches `synthetic_image` or `write_loader_data`.
- **Two statements of the card that did not hold,** found in the check before the close: its scope cites the
  design's table for "the process area (the synthetic pages, the loader data and PEB's `Ldr`)", and the table has no
  such row (its rows end at the translation tables); its expected values name "`process.rs`' tests unchanged and
  passing", and `process.rs` has none (`#[test]` 0). The row lands with card 291, which settles the loader data's
  publish order on reviewer-3's Low 1 at 290; the process area's writers are held by the game's and the family runs
  alone, as the plants' judge says.
- **A bound kept:** the slice the process area's writers had at a run's LoadLibrary ended at `PROCESS_SIZE` and
  panicked past it; `ProcessView::put` asserts the same bound, found in the check before the commit. Its first form
  named `PROCESS_SIZE` from `guest.rs`, which builds on macOS alone, and the pre-commit run stopped at lint, "cannot
  find value `PROCESS_SIZE`", the lookup that took the constant for `process.rs`' having read `guest.rs`' line; the
  constant moved to `process.rs`, beside `TRACE_SIZE` it is made from, and `guest.rs` re-exports it, one definition.
- **The access** (d0e2fa622): `process::ProcessBytes`, implemented by `[u8]`, `Vec<u8>` (the parameters block) and
  `ProcessView`; the process area's helpers, `synthetic_image` and `write_loader_data` generic over it; `modules.rs`'
  four writes through the view. 36 slice calls in `src/`, 27 on guest memory; `modules.rs` 1, its `Vec`.
- **Expected values:** 172 tests in the pre-commit run, the batch-run step first, the game's and the family runs as
  held.
- **The gates:** `scripts/gates.sh` exit 0 in 824 s, kept in `store/gates/passes.tsv` as sprint 290 at d0e2fa622 with root=0d5d3950d and "rust=crates python=0 samples=0", rdeps(w2m_vm), 172 tests, base=6a2767ce2, Sprint 289's close; multi-vCPU: not run (Python gates not run)
- **Plant 1,** restored by `cp` and checked by `cmp`: `ProcessView::put` writing one byte past its offset; the game
  diverged, `batch_run.rs` 315 failing, "the batch run's unity.log lines left: 12 right: 168"; the planted game then
  handed its record a value outside JSON's safe integers ("int 18446603344900008000 outside JCS-safe range"),
  `batch-run-record.sh` exited 1 and the guests and families were not run again, so their readers refused the earlier
  build's outputs by their stamps, "made from the sources 99ca208a3… and the work tree's are 02860ad62…", Sprint
  288's check, not the plant's own effect on `late_tls`.
- **Plant 2,** restored by `cp` and checked by `cmp`: the import slots written 8 bytes past their offset; the batch-run
  step ran whole; `family_runs.rs` 1358 failed, "late_tls 38229484497 stop: Untranslated { target: 95512, why: \"no
  guest memory at 0x17518\" }", the DLL calling through a slot left unwritten; the game's stop moved to `import
  KERNEL32.dll!GetSystemTimes` (`batch_run.rs` 139) and its log to 0 lines.
