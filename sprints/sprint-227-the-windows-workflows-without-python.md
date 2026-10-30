# Sprint 227 — The Windows workflows without Python

```yaml
---
id: 227
status: active
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
