# Sprint 012 — 4 KB page protection: correctness and cost on both backends

```yaml
---
id: 012
status: closed
phase: 2
pass_kind: observation
---
```

## scope

Probe v0.2 §8's example (page A read/write, page A+4096 inaccessible) on this Mac: native `mprotect` at 4 KB granularity; a native software shadow check (1 byte per 4 KB page); guest 4 KB stage-1 pages in a Hypervisor.framework VM with one invalid page and a guest vector table that exits on the fault. Then measure per-access cost: native plain / check_each / check_page_cross, VM pages_4k / blocks_2m, L1- and DRAM-sized working sets. Ratify P10 into vocabulary 0.3 (three capability values) in the same sprint.

## files

- `probes/page_geometry/page_geometry.c`, `scripts/page_geometry.py` (created)
- `signals/0.3.json`, `signals/0.3-rationale.md` (created); `signals/current.json`, `signals/proposals.json`, `src/w2m/__init__.py`, `src/w2m/events.py` (modified)

## signal contract

Emits SESSION_INIT, CAPABILITY_PROBED x3, MEASUREMENT_RECORDED x10, SESSION_COMPLETED.

## command exit codes

- `uv run python scripts/page_geometry.py` → 0 (correctness precondition gates the measurements)
- `python3 scripts/check_vocab.py`, `scripts/gen_events.py --check`, `uv run pytest`, `ruff check`, `mypy` → 0

## observation contract

- native_mprotect_4k absent (EINVAL); native_page_check_4k present (fault at offset 4096, none on A); guest_page_protection_4k present (guest data abort EC 0x25, FAR 0x20001000, ELR +4; A read 0x11223344).
- 10 MEASUREMENT_RECORDED with variant names.

## self-review checklist

- [x] Correctness precondition before cost; the driver refuses to measure if any check fails.
- [x] Vocabulary change by proposal, ratified in the needing sprint (PRINCIPLES.md).
- [x] Symmetric setup: VM and native both have a warm pass (KIT_DIARY finding 7).
