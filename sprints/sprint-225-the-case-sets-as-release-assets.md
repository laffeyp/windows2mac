# Sprint 225 — The case sets as release assets

```yaml
---
id: 225
status: active
phase: 6
pass_kind: functional
---
```

## scope

The Decision "No Python runs" as annotated in Sprint 224 (the user: "have we accounted for turing the the 22% of
python into 0 yet all tht way") and reviewer-3's Medium 1 of its review at Sprint 224.

- **Where it stands, by lookup (2026-10-29):**
  - 8 of the 21 reference case files are gitignored, 159 MB, built by `scripts/semantics_differential.py`. They are
    sets 039, 041, 049, 049b, 050, 065, 065b and 065c.
  - CI no longer builds them (Sprint 224), and no Python may run, so this Mac's `probes/` holds the only copies.
  - Each set has its release, holding its run's result files: `semantics-039` (run 38011862284), `semantics-041`
    (38029980267), `semantics-049` (38117654018), `semantics-049b` (38118493713), `semantics-050` (38122967607),
    `semantics-065` (38222873837), `semantics-065b` (38234338405), `semantics-065c` (38238041162).
  - Each run's `store/probe-runs/<run>/cases.sha256` records the case file's hash.
- **The work:**
  - Each `cases.bin` is checked against its recorded hash, then uploaded as an asset of its own release.
  - CI downloads the 8 with `gh release download`, checks each with `sha256sum -c` against `cases.sha256`, and runs
    the Rust tests with `W2M_REQUIRE_CASE_RUNS=1`, so a missing file fails there as it does in the gates.
- **Not here:** the Python leaving the tree (Sprint 226).

## prerequisites

- 224 (closed)

## context_files

- `.github/workflows/ci.yml`; `.gitignore` 30-39; `store/probe-runs/<run>/cases.sha256` for the 8 runs
- `crates/w2m_vm/tests/class_judge.rs` (`counts`), `crates/w2m_vm/tests/run_loop.rs` (the skip), `scripts/gates.sh`

## signal contract

### Emits

- None; a CI input.

### Invariants

- Each uploaded file's sha256 equals its run's `cases.sha256`.

## artifact contract

### Files modified

- `.github/workflows/ci.yml`; the 8 releases' assets

### Command exit codes

- `scripts/gates.sh` returns 0; the CI run on the work commit concludes success with the case-count test judging,
  not skipping

## observation contract

- **Expected values:** 8 assets uploaded, each hash equal to its record; CI's case-count test judging 458 classes.
- **Planted faults:**
  1. A wrong hash in one `cases.sha256` (a scratch copy): the check names the file.
  2. One download dropped from CI's step (a branch run): the required case runs fail, naming the set.
  3. A truncated asset (a scratch file checked against the recorded hash): `sha256sum -c` names it.

## done criteria

The 8 case sets are fetchable from their releases and checked by hash. CI judges what reads them, with no Python
run, and a missing set fails CI.

## self-review checklist

- [x] Scope names one verifiable property: the 8 case sets reproducible without Python.
- [x] context_files cite the workflow, the hashes and the tests that read the sets.
- [x] Emits: none, with the reason.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and three plants.
- [x] One concept: the case sets' source.

## result

- **Sprint 224's CI run failed** (41012631099, on 0a1c4003d). `cpu_cases`' `case_files_round_trip_and_match_their_runs`
  asserts 21 case files (round_trip.rs 54) and found 13. Sprint 224 had read that test as reading only the files
  present, and missed the count. This sprint is the fix.
- **The class of the error, checked whole:** every Rust file that names `probes/`, `cases.bin`, `third_party` or
  XED was listed and read for what CI no longer builds.
  - Only two tests read the 8 sets: `round_trip.rs`, which requires 21, and `class_judge.rs`, which skips without
    them.
  - Four other files matched on the word "fixed" or read tracked probe sources (`virtual_memory.rs`).
  - XED's build, the other thing CI made with Python, is read by no Rust test.
- **The assets:** each of the 8 `cases.bin` matched its run's recorded hash. Each was uploaded as `<run>-cases.bin`
  to its release:
  - `semantics-039`, 23,824,812 bytes;
  - `semantics-041`, 7,945,812;
  - `semantics-049`, 20,829,916;
  - `semantics-049b`, 40,206,332;
  - `semantics-050`, 7,699,812;
  - `semantics-065`, 38,056,596;
  - `semantics-065b`, 15,369,780;
  - `semantics-065c`, 5,257,596.
- **CI:** a step before the Rust job downloads each with `gh release download`. It checks each with `sha256sum -c`
  against `store/probe-runs/<run>/cases.sha256`, and requires 21 case files. Run in a scratch tree laid out as the
  repository, it left 21.
- **Departure:** the card set `W2M_REQUIRE_CASE_RUNS=1` in CI. That flag also requires the Mac's run outputs
  (`family_runs.rs`' stops, `run_loop.rs`' inputs), which CI cannot make. The step's own count of 21 holds the case
  sets instead, and `class_judge.rs` and `round_trip.rs` then judge all of them.
- **Plant 1** (set 041's recorded hash zeroed, in the scratch tree): "probes/semantics_041/cases.bin: FAILED".
- **Plant 2** (set 065c's download dropped): the step exited 1 with 20 files.
- **Plant 3** (a 1,000-byte truncation of set 041 checked against its hash): "t.bin: FAILED".
