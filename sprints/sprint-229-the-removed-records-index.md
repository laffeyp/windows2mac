# Sprint 229 — The removed records' index

```yaml
---
id: 229
status: active
phase: 6
pass_kind: functional
---
```

## scope

Reviewer-3's Medium 3 of its review at Sprint 226: the Python removed in Sprint 226, which the tree's records still
name by path.

- **Where it stands, by lookup (2026-10-30):**
  - The tag frozen-python-2026-10-29 holds 454 `.py` files outside `sdd-kit-3/`. A citation reaches one only by
    `git show frozen-python-2026-10-29:<path>`, and nothing in the tree says which paths the tag holds.
  - Tracked files under `store/` name 263 distinct repository-relative `.py` paths (`git grep -oIE
    '[A-Za-z0-9_./-]+\.py' -- store`), in 31,687 files, 31,681 of them under `store/runs/`. All 263 are at the tag.
    Seven more are bare names (`family_run.py`, `run_frontier.py` and five others), and one is an absolute path on
    this Mac.
  - Of the records, 7 are not code:
    - `store/candidates/<sha256>.py` (3), cited by `candidate_sha256` in `store/executions/` and `store/runs/`;
    - `docs/review-2026-10-11-sprint-054/` (3), a review's exhibits;
    - `probes/skeleton/candidates/loses_value.py` (1).
  - Records cite 13,916 candidates by `candidate_sha256`. Only 5 candidate bodies were ever committed: 2 `.c` in the
    tree and the 3 `.py` above. The rest name their source in `generator_ref` (for example `tools/guest_runtime/file.c`),
    so they are not this card's concern.
  - `store/stencil-inputs/record_insn_bytes.py` is on this Mac, under a gitignored directory that nothing reads.
  - Restoring the files is ruled out by the user's "the 22% of python into 0".
- **The work:**
  - `store/frozen_python/removed-2026-10-29.tsv`: one row per `.py` at the tag outside the kit, its blob id and its
    path, written by `git ls-tree -r frozen-python-2026-10-29`.
  - Two tests in `crates/signal_report/tests/no_python.rs`:
    - the index equals the tag's list, row for row;
    - every relative `.py` path in a tracked file under `store/` is a row of the index, else the test names the file
      and the path.
  - CI checks out the whole history, tags included (`fetch-depth: 0`), so both tests run there.
- **Not here:** the bare names and the absolute path, which name no repository path; the candidate bodies never
  committed.

## prerequisites

- 228 (closed)

## context_files

- `store/frozen_python/`, `store/runs/`, `store/executions/`, `crates/signal_report/tests/no_python.rs`,
  `.github/workflows/ci.yml` (its checkout)

## signal contract

### Emits

- None; an index of files.

### Invariants

- The index lists every `.py` at the tag outside the kit, with its blob id, and only those.
- Every relative `.py` path in a tracked record under `store/` is in the index.

## artifact contract

### Files modified

- `store/frozen_python/removed-2026-10-29.tsv` (new), `crates/signal_report/tests/no_python.rs`

### Command exit codes

- `scripts/gates.sh` returns 0

## observation contract

- **Expected values:** 454 index rows; 263 cited paths, each a row.
- **Planted faults:**
  1. A row dropped from the index: the tag test names the path.
  2. One row's blob id changed: the tag test names the path.
  3. A cited path's row dropped: the citation test names a record that cites it.

## done criteria

Every removed Python file is named with its blob at the tag, and every path a stored record names resolves through
that list.

## self-review checklist

- [x] Scope names one verifiable property: every removed `.py` indexed with its blob, every cited path in it.
- [x] context_files cite the records, the index's directory, the test and CI's checkout.
- [x] Emits: none, with the reason.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and three plants.
- [x] One concept: the removed records' index.

## result
