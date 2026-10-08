# Sprint 010 — vocabulary 0.2

```yaml
---
id: 010
status: closed
phase: 2
pass_kind: architecture
---
```

## scope

Evolve the vocabulary 0.1 → 0.2 through the proposal taxonomy. Ratify P8 (`types.version` is major.minor[.patch]; harness-plane integers lie in ±(2^53−1)) and new proposal P9 (`MEASUREMENT_RECORDED`: metric value `ns_per_access`; optional payload field `variant` naming the code-generation or configuration variant measured). Write `signals/0.2.json` (0.1 stays on disk), `signals/0.2-rationale.md`, `signals/current.json` (`{"version": "0.2"}`). Add `guest_plane_compatible_versions: ["0.1", "0.2"]` because the guest plane is unchanged, so 0.1 traces stay ingestible. Point the generator and the gate at `current.json`; ingest accepts any compatible guest version and records the trace's own version in TRACE_INGESTED.

## prerequisites

- 009

## context_files

- `sdd-kit-3/grammar/PRINCIPLES.md` (proposal taxonomy; Layer 9 version metadata)
- `signals/0.1.json`, `signals/proposals.json` (P8), `signals/0.1-rationale.md` (Layer 10 override: an added enum value bumps the minor version)
- `scripts/gen_events.py`, `scripts/check_vocab.py`, `src/w2m/__init__.py`, `src/w2m/trace.py`

## signal contract

### Emits

None at runtime (vocabulary change).

### Invariants

- `signals/0.1.json` byte-identical to commit 2276dbb.
- Every tag in 0.1 exists in 0.2 with the same required payload (additive change only).
- The 11 committed reference traces (vocab 0.1) still ingest.

## artifact contract

### Files created / modified

- created: `signals/0.2.json`, `signals/0.2-rationale.md`, `signals/current.json`
- modified: `signals/proposals.json`, `scripts/gen_events.py`, `scripts/check_vocab.py`, `src/w2m/__init__.py`, `src/w2m/trace.py`, `src/w2m/events.py` (regenerated), tests

### Command exit codes

- `python3 scripts/check_vocab.py` returns 0 (reads current.json)
- `uv run python scripts/gen_events.py --check` returns 0
- `uv run pytest`, `uv run ruff check`, `uv run mypy` return 0
- every trace in `store/traces/` ingests: TRACE_INGESTED

## observation contract

Not applicable (architecture). The store re-ingest is the behavior check.

## done criteria

Vocabulary 0.2 is current, additive over 0.1, and nothing recorded under 0.1 became unreadable.

## notes

Many files for one concept: a version bump touches each place the version is wired. Recorded as a scope exception in the Sprint tail rather than split, because a half-bumped vocabulary would fail its own gate between sprints.

## self-review checklist

- [x] Scope concrete.
- [x] Proposal taxonomy followed (two typed proposals, ratified with grounds).
- [x] Assertions gradable.
- [x] Additive-only invariant checkable mechanically.
- [x] Scope exception to hard rule 6 stated.
