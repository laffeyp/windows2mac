# Sprint 226 — The Python out of the tree

```yaml
---
id: 226
status: active
phase: 6
pass_kind: functional
---
```

## scope

The Decision "No Python runs" as annotated in Sprint 224 (the user: "have we accounted for turing the the 22% of
python into 0 yet all tht way"), and the Decision "Sequence after Sprint 214" as annotated in Sprint 224.

- **Where it stands, by lookup (2026-10-29):**
  - 467 tracked `.py` files, about 50,700 lines. 297 are in `src/`, 83 in `tests/`, 65 in `scripts/`, 13 in
    `sdd-kit-3/`, and the rest in `store/` (3), `docs/` (3), `tools/` (2) and `probes/` (1).
  - The Python directories also hold files the Rust project uses: `tools/guest_runtime` (52, the runtime's C),
    `tests/data` (35, the guests the gates run), `tools/vm_compiler` (8), `tools/windows_machine` (9) and others. Only
    `.py` files go.
  - Two Rust files read Python source as text: `crates/w2m_vm/tests/host_requests.rs` (the frozen host's dispatch
    lines in `src/w2m/vm_backend.py`, `kernel.py` and `file_bridge.py`) and `crates/w2m_vm/src/bin/family_known.rs`
    (`scripts/family_run.py`'s `Run` lines, start options and derivations).
  - `pyproject.toml` builds `src/w2m` as the package `w2m` (hatchling, 25-26) and lists Python dev tools. `uv sync`
    must still install substrate, the harness, which the gates and CI run under `uv`.
  - `scripts/sign_and_run.sh`, `scripts/gates.sh`, and `harness/case-runs.toml`, `case-runs-rust.toml` and
    `w2m-vm-game.toml` mention `.py`. Each is read in this sprint for whether it runs or only cites the Python.
- **The work:**
  - A tag, `frozen-python-2026-10-29`, is pushed on the last commit that holds the Python, so it stays readable.
  - Each Rust reader of Python text gets a stored extract, written by Rust from the text before the text goes, and
    held by a test to what it replaces. `host_requests.rs` gets the frozen host's request names. `family_known.rs`
    gets each `Run`'s binary and DLL, the start options and the derivations it reads.
  - Every `.py` outside `sdd-kit-3/` is removed.
  - `pyproject.toml` stops declaring a package (`[tool.uv] package = false`), drops the Python dev tools, and keeps
    `substrate-kernel`. `uv.lock` follows.
  - `.gitattributes` marks `sdd-kit-3/**` as `linguist-vendored`. The kit is read-only, and its 13 example files
    leave GitHub's language count.
  - Docs and comments that point at a removed `.py` as a live file are left as citations of the tag.
- **Not here:** the Python words fixture, which the card for the 17 classes and the 6 retires.

## prerequisites

- 225 (closed)

## context_files

- `pyproject.toml`, `uv.lock`, `.gitattributes`; `crates/w2m_vm/tests/host_requests.rs`,
  `crates/w2m_vm/src/bin/family_known.rs`, `crates/w2m_vm/tests/family_runs.rs`
- `scripts/gates.sh`, `scripts/sign_and_run.sh`, `harness/*.toml`, `.github/workflows/ci.yml`

## signal contract

### Emits

- None; a change to the tree.

### Invariants

- Every Rust test and gate that passed before passes after, reading extracts where it read Python text.
- `git ls-files '*.py'` lists only `sdd-kit-3/` files.

## artifact contract

### Files modified

- the removed `.py` files; `pyproject.toml`, `uv.lock`, `.gitattributes`; the two readers and their extracts

### Command exit codes

- `scripts/gates.sh` returns 0; CI concludes success on the work commit

## observation contract

- **Expected values:** `.py` files tracked, from 467 to 13, all in `sdd-kit-3/`; GitHub's language bar without
  Python; `uv sync` installing substrate; the extracts equal to what the Python text gave.
- **Planted faults:**
  1. One extract row changed: the reader's test names it.
  2. A `.py` file left outside the kit: a test that lists tracked `.py` files names it.
  3. `substrate-kernel` dropped from `pyproject.toml`: `uv run substrate` fails in the gates, named.

## done criteria

No Python outside the read-only kit is in the tree, the frozen Python stays readable at its tag, and every gate passes
reading stored extracts.

## self-review checklist

- [x] Scope names one verifiable property: no `.py` outside the kit.
- [x] context_files cite the packaging, the readers and the gates.
- [x] Emits: none, with the reason.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and three plants.
- [x] One concept: the Python's removal.

## result

- **The tag:** `frozen-python-2026-10-29` (annotated) is pushed on 6a036a236, the last commit holding the Python.
- **The extracts** (`store/frozen_python/`):
  - `family-run-2026-10-29.tsv`, 86 families and 259 runs, written by `family_known`'s own text reader from
    `family_run.py`. `family_known` then read it, and `the_stored_lookup_is_family_knowns_output` passed on it, the
    273-row table unchanged. The text reader and its writing mode were removed.
  - `host-dispatch-2026-10-29.tsv`, 56 names (`vm_backend.py` 30, `kernel.py` 12, `file_bridge.py` 14), written by
    `host_requests.rs`' own `dispatched` reader through a temporary ignored test. The test then read the extract and
    still counted 56 requests, each with a Rust arm. The writer and `dispatched` were removed.
- **The removal:** 454 `.py` files and `src/w2m/py.typed` left the tree with `git rm`. `git ls-files '*.py'` lists 13,
  all in `sdd-kit-3/`. `crates/signal_report/tests/no_python.rs` fails on any `.py` tracked outside the kit.
- **A dependency the lookup missed:** `case_inputs.rs`' fixture check ran `git log -1 -- src/w2m` to hold the Python
  words fixture to the commit it was made at. Deleting `src/w2m` would have moved that commit and refused the fixture.
  It now reads the log at the tag. The fixture's `src` and the tag's last `src/w2m` commit are both 37784f1a7.
- **`pyproject.toml`:** `[tool.uv] package = false`, only `substrate-kernel` as a dependency. The build system, the
  dev tools, the `reference` extra and the pytest, ruff and mypy settings are gone, and `uv.lock` lost 548 lines.
  `uv sync` and `uv run substrate` work.
- **`.gitattributes`:** `sdd-kit-3/** linguist-vendored` (`git check-attr` reads "set" on `sdd-kit-3/lib/sdd.py`).
- **Kept, not Python:**
  - the 5 comments citing a `.py` in `scripts/` and `harness/`, as citations of the tag;
  - `tests/data/` (the guests the gates run);
  - `tests/fixtures/steamcmd-app-info-409720.txt`, a Steam record no Rust reads yet and data for milestone D.
- **WORKING_AGREEMENT** no longer lists Python commands or dependencies, and names the tag and `no_python.rs`.
- **Plant 1** (a run's binary changed in the family extract): `the_stored_lookup_is_family_knowns_output` failed,
  "store/binaries/0c5dc179…exe: No such file or directory". Restored byte for byte.
- **Plant 2** (`tools/plant.py` tracked): `no_python_is_tracked_outside_the_kit` failed, "Python tracked outside
  sdd-kit-3/: [\"tools/plant.py\"". Removed.
- **Plant 3** (`substrate-kernel` dropped from `pyproject.toml`, then `uv sync`): `uv run substrate` failed, "Failed to
  spawn: `substrate`". Restored byte for byte, and `uv sync` brought it back.
- **Departure:** `case_inputs.rs`, outside the card's file list (above).
