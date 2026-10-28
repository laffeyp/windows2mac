# Sprint 186 — The batch run on the Rust host, to its first stop

```yaml
---
id: 186
status: active
phase: 6
pass_kind: functional
---
```

## scope

reviewer-2's High 1 of its review of Sprints 182-184 (the Decision "Sequence after Sprint 185"): no card has run the
Rust host on the batch run, so the first stop it meets is unknown, and the order since Sprint 173 came from the Python
run's counts. This sprint runs it and names the stop.

- **The batch run's start:** `w2m-vm` gains `--game <directory> <exe>`: the program loaded with its DLLs supplied from
  the directory (`pe_loader::load`'s `DllSupply`, as `run_frontier.py` 268 supplies `here / n`), started as
  `run_frontier.py` 268-273 starts it: `gui`, `foreground_held`, `USER_ENVIRONMENT` (vm_backend 342), the command line
  `"C:\w2m\app\Hail to the Rainbow.exe" -batchmode -nographics -logFile "C:\w2m\unity.log"` (`BATCH_ARGUMENTS`, 64),
  and the instruction clock at 1,400,000,000 a second from FILETIME 134362624920000000 (68-69). The game is
  `~/w2m-depots/hail-to-the-rainbow` (Sprint 108's run).
- **The stop:** the run goes to whatever stops it, and the stop is written to `store/batch_run/first-stop-2026-10-28.tsv`
  by name: the `Stop` variant and its text, the instructions retired, the exits by number, the host requests by op,
  and the blocks compiled; a test holds the file.
- **The guests:** the 14 guests of `tests/data/timed_waits` (census row 8; 2 run by the Rust host today) each run by
  `w2m-vm` on one vCPU, each stop listed in the same file's second table against the Python host's stop and output.
- **The census:** rows 5, 5a, 12 and the counts' paragraph brought up to date (reviewer-2's Low 1), and rows 8 and 10
  given this sprint's stops.

## prerequisites

- 185 (closed)

## context_files

- `scripts/run_frontier.py` 60-75, 245-300; `src/w2m/vm_backend.py` 342-350; `crates/w2m_vm/src/bin/w2m-vm.rs`;
  `crates/w2m_vm/src/process.rs` (`Start`); `crates/pe_loader/src/lib.rs` (`load`, `DllSupply`);
  `docs/step-4-census-2026-10-28.md`; `build/sprint-108-run.log` 1-5, 120, 168-177

## signal contract

### Emits

- None new: `w2m-vm`'s record is written only for a run that ends at ExitProcess (`w2m-vm.rs` 8-10), which the batch run
  does not; the stop is held by the test.

### Invariants

- No Python file added or changed; the guests' Python stops are read from `tests/test_timed_waits.py`'s assertions.

## artifact contract

### Files modified

- `crates/w2m_vm/src/bin/w2m-vm.rs`, `store/batch_run/first-stop-2026-10-28.tsv` (new), a test holding it,
  `docs/step-4-census-2026-10-28.md`, `WORKING_AGREEMENT.md`

### Command exit codes

- the held-stop test and `scripts/gates.sh` return 0

## observation contract

- **Expected values:** the batch run stops at a named `Stop` short of ExitProcess, its exits and requests counted; each
  guest's stop listed, `qpc`, `sleeper` and `model_steps` at ExitProcess with their stored output.
- **Planted faults:** each run before this card's commit: (1) the DLL supply dropped: the run's stop moves into loading
  and the held-stop test names the new stop; (2) the command line without `-batchmode`: the held-stop test names
  whatever differs, or the plant is recorded as reaching no judge.

## done criteria

The batch run's first stop on the Rust host is named and held, and every tests/data guest's stop is listed.

## self-review checklist

- [x] Scope names one verifiable property: the first stop, held.
- [x] context_files cite the Python run the start copies.
- [x] Emits: none new, said why.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and two plants.
- [x] One concept: the batch run on the Rust host.
