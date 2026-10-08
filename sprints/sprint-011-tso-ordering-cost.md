# Sprint 011 — execution time of x86-TSO ordering lowerings, native process and managed VM

```yaml
---
id: 011
status: closed
phase: 2
pass_kind: observation
---
```

## scope

Author `probes/ordering_cost/ordering_cost.c` and `scripts/ordering_cost.py`. One kernel, `a[i] += b[i]` (2 loads + 1 store per element, explicit address adds so every variant runs the same instruction count apart from ordering), in six variants: native `plain`, `ldapr_stlr`, `ldar_stlr`, `dmb_fences`; managed VM `entso_off`, `entso_on` (4 KB granule) and `entso_on_16k` (default 16 KB granule, control) (guest MMU on, Normal WB, 4 KB granule). Two working sets: `l1` (2 × 4 KiB arrays, repeated) and `dram` (2 × 64 MiB arrays). Each cell runs 5 times interleaved; the median is recorded as MEASUREMENT_RECORDED(metric `ns_per_access`, `variant`, `backend`). A native MP litmus (plain vs ldapr/stlr) runs first; if ldapr/stlr shows any forbidden outcome the driver exits 1 and records no cost.

## prerequisites

- 009 (VM guest setup), 010 (vocabulary 0.2: `ns_per_access`, `variant`)

## context_files

- `probes/mac_hv_litmus/mp_litmus.c`
- `signals/0.2.json` (MEASUREMENT_RECORDED)
- v0.2 §8.2, §9, §13
- Arm ARM: LDAPR (FEAT_LRCPC, RCpc acquire), LDAR (RCsc acquire), STLR (release), DMB ISHLD / ISHST

## signal contract

### Emits

- `SESSION_INIT`, `MEASUREMENT_RECORDED` x14 (7 variants × 2 working sets), `SESSION_COMPLETED`.

## artifact contract

### Files created

- `probes/ordering_cost/ordering_cost.c`
- `scripts/ordering_cost.py`

### Command exit codes

- `uv run python scripts/ordering_cost.py` returns 0
- `uv run pytest`, `uv run ruff check`, `uv run mypy` return 0

## observation contract

### Expected runtime signals

- Native MP check: plain > 0 forbidden outcomes; ldapr_stlr 0 (precondition for recording).
- 12 MEASUREMENT_RECORDED with `application_profile_id = synthetic.ordering_kernel`, `phase = sustained`, `artifact_state = warm`.

## done criteria

The execution time of each ordering lowering on this kernel is measured on this host, with the native lowering shown correct first.

## notes

This measures one ingredient of v0.2 §8.2, not a backend decision: v0.2 forbids choosing a backend from a single measurement. VM numbers exclude exit costs by design (one entry per timed run, milliseconds long); exit-heavy workloads are a separate measurement.

## self-review checklist

- [x] Scope concrete.
- [x] Correctness precondition before timing.
- [x] Emits use 0.2 tags/fields.
- [x] Assertions gradable.
- [x] Observation contract present.
- [x] Two files, one concept.
