# Sprint 227 — The Windows workflows without Python

```yaml
---
id: 227
status: done
phase: 6
pass_kind: functional
---
```

## scope

The Decision "No Python runs" (the user: "no python", "no depeding on pythong") and reviewer-3's Medium 1 and Low 1 of
its review at Sprint 226.

- **Where it stands, by lookup (2026-10-29):**
  - `.github/workflows/windows-probe.yml` 58-63 runs a probe's `generate` command as `uv run --no-dev python …`. All 21
    `probes/semantics*/probe.json` name `scripts/semantics_differential.py --cases …`, which left the tree in Sprint 226.
  - Since Sprint 225, each of the 21 sets is on a release: 8 as `<run>-cases.bin` assets, and 13 tracked in git.
  - `.github/workflows/windows-game.yml` 62-109, one bash step, runs Python six times:
    - the release's tag and its part names from `release.json`;
    - the free bytes on the game's drive;
    - each part's sha256 against `release.json`;
    - each unpacked file's sha256 against `files.json`, and no file unlisted.
  - `crates/signal_report/tests/no_python.rs` sees only tracked `.py` files.
- **The work:**
  - Each case-set probe names its input by release and asset in place of `generate`. `windows-probe.yml` downloads
    it with `gh release download` and checks it against `store/probe-runs/<run>/cases.sha256`. A tracked `cases.bin`
    needs no download. The `generate` step and its `uv` setup leave the workflow.
  - `windows-game.yml`'s six Python uses become `jq`, `sha256sum` and `df` in the same bash step. Whether the runner
    image ships `jq` is looked up first; if it does not, the step reads the JSON in PowerShell, which the workflow
    already uses.
  - `no_python.rs` also fails on `python` run in `.github/workflows/*.yml` and on a `.py` named in
    `probes/*/probe.json`.
- **Not here:** a Rust case-set generator, dispatcher and fetcher for a new reference (Deferred, Sprint 226).

## prerequisites

- 226 (closed)

## context_files

- `.github/workflows/windows-probe.yml`, `.github/workflows/windows-game.yml`, `probes/semantics*/probe.json`
- `crates/signal_report/tests/no_python.rs`; `store/probe-runs/<run>/cases.sha256`; `games/*/release.json`,
  `games/*/files.json`

## signal contract

### Emits

- None; workflows and probe configuration.

### Invariants

- No workflow runs Python; no probe names a `.py`.

## artifact contract

### Files modified

- the two workflows, the 21 `probe.json`, `no_python.rs`

### Command exit codes

- `scripts/gates.sh` returns 0; a dispatched `windows-probe` run of one case set and a dispatched `windows-game` run
  each conclude success

## observation contract

- **Expected values:** a `windows-probe` run on a case set reproducing its stored result's sha256; a `windows-game`
  run verifying every part and file; `no_python.rs` naming nothing.
- **Planted faults:**
  1. `python -c` added to a workflow step: `no_python.rs` names the file and line.
  2. A `.py` restored as one probe's `generate`: `no_python.rs` names the probe.
  3. A wrong sha256 for one game part, in a scratch copy of the step: the step stops, naming the part.

## done criteria

No workflow runs Python and no probe names a Python generator. Both Windows workflows run on a dispatch, and a test
holds the rule.

## self-review checklist

- [x] Scope names one verifiable property: no Python in the workflows or the probes.
- [x] context_files cite the workflows, the probes and the test.
- [x] Emits: none, with the reason.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and three plants.
- [x] One concept: the Windows workflows' Python.

## result

- **The runner's tools, looked up first:** the Windows 2025 and 2022 images list `jq` 1.8.2 and `zstd` 1.5.7
  (`actions/runner-images`, `images/windows/Windows2025-Readme.md` and `Windows2022-Readme.md`).
- **The probes:**
  - The 13 sets tracked in git lost their `generate` line.
  - The 8 asset sets name `cases_asset`, "<release>/<run>-cases.bin" (set 039 `semantics-039/38011862284-cases.bin`
    and so on).
  - All 33 `probe.json` parse, and none names a `.py`.
- **`windows-probe.yml`:** the `uv` setup, the substrate clone and the `generate` step are gone. A `case file` step
  downloads the asset with `gh release download` and checks it with `Get-FileHash` against the run's `cases.sha256`.
  A tracked `cases.bin` needs no step.
- **`windows-game.yml`:** its six Python uses are `jq` (the tag, the part names, the listings), `df -B1 --output=avail`
  (free bytes) and `sha256sum -c` (each part, each file). A `find` against the listing, with `sort` and `comm`,
  requires that no unlisted file is present.
- **`no_python.rs`:** `no_workflow_runs_python_and_no_probe_names_it` fails on a Python command in a workflow line,
  and on a `probe.json` naming a `.py`. A command is a token `python`, `python3`, `python.exe`, `py`, `pip` or `pip3`
  outside a `#` comment. Its unit test holds the reader: `corpus/python-3.14.8` is a path, and a comment's "python
  -c" is not run.
- **Plant 1** (`tag=$(python -c "print(1)")` in `windows-game.yml`): the test failed, "windows-game.yml:71: python".
  Restored byte for byte.
- **Plant 2** (a probe's `source` set to `scripts/gen.py`): the test failed, "probes/semantics_042/probe.json: names
  a .py". Restored byte for byte.
- **Plant 3** (part `p2`'s sha256 zeroed in a scratch `release.json`, the step's own `jq` and `sha256sum` lines run
  over it): "p2: FAILED", exit 1.
- **A fault from Sprint 221, met in this sprint's gates** (the first run exited 100 in 530 s):
  - `family_runs.rs`' `steady` masked a temporary name only as `w2m` and four hex digits. This run's windows family
    drew `w2m417.tmp`, and the held line no longer matched.
  - GetTempFileNameW writes its number "in upper-case hex without leading zeros" (`tools/guest_runtime/file.c`
    475-479), so the mask now takes one to four.
  - `a_moving_stop_is_written_steadily` gains three cases: `w2m417.tmp` and `w2mA.tmp` masked, `w2m12345.tmp` not.
  - Departure: `crates/w2m_vm/tests/family_runs.rs`, outside the card's file list.
  - The class, checked whole:
    - `steady` is the only Rust code that reads a temporary name.
    - The runs' printed rows hold three: `files_more`'s `w2m12AB.tmp`, `w2m2345.tmp` and `w2m3.tmp`. Each is a
      number the guest passes, not one drawn from the clock, so the held rows' sha256 stays fixed.
    - Sprint 221's check of the rows searched for four digits and missed `w2m3.tmp`, the same width assumption.
- **The dispatched runs:**
  - `windows-probe` on set 065c, run 41018967302, concluded success: "probes/semantics_065c/cases.bin matches run
    38238041162's sha256".
  - `windows-game`, run 41018970831, failed in "fetch and unpack": "no assets match the file pattern". The native
    `jq.exe` writes CRLF under Git Bash, so each part name read from `release.json` ended in `\r`. jq's manual names
    `--binary` (`-b`) for this case.
  - Every `jq` call in the step now passes `-b`. The fix was gated, committed and pushed, and the game run dispatched
    again: run 41019280283 succeeded on ef9cb19fd, "parts verified" and "297 files verified against files.json; no other file present".
- **Reviewer-3's Low 1 of its review at 227:** `no_python.rs`' readers missed a versioned interpreter, the
  `setup-python` action, and a `.py` at the end of a probe value. The workflow reader now takes `python` with digits
  and dots and `setup-python`. Probes are parsed as JSON, and every string value with a word ending in `.py` is named.
  Three cases were added to the reader's test.
- **An expected value corrected:** the card expected a probe run to reproduce its stored result's sha256. The stored
  runs were on an Intel Xeon 8573C, and a runner's CPU is assigned per job, AMD or Intel, so a result can differ for
  that reason alone. The test is the input: the run fetches the case file and its sha256 equals the recorded one.
