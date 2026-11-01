# Sprint 314 — The entries made before a second vCPU

```yaml
---
id: 314
status: active
phase: 6
pass_kind: functional
---
```

## scope

Reviewer-3's Medium 1 at 312 (BLACKBOARD Surfaced) on the per-vCPU pages design
(`docs/design-each-vcpus-own-pages-2026-10-31.md`): a private chain's level-0, level-1 and level-2 tables are copies,
and an entry the shared tables gain after a vCPU is made never reaches its copy. The disposition chose the reviewer's
(a): every upper-table entry a run-time mapping can write under level-0 entries 0xFE and 0xFF is made at load, before
any second vCPU, and the descriptor writer refuses a new one after. Which entries those are is a fact of the code, read
before the build card (card 4c2b, Sprint 315) depends on it. Under 0xFF lie the system DLLs (`DLL_BASE`, level-1 0x1E0
on), the TLS slots (`TLS_VA`), the loader stub (`LOADER_STUB`) and the thunks with the process area (`THUNK_BASE` =
`PROCESS_VA`, 0x1F8); under 0xFE the monitor area.

- **The work, a design amendment:** each writer of a descriptor at run time (`guest.rs` `page_descriptor` and
  `map_pages`, the loader's late images, `compile.rs` `protect_code`, the block compiler's level-3 writes) read for the
  levels and ranges it can write under 0xFE and 0xFF; the ranges' extents from the layout and the loader; the list of
  upper-table entries made at load, by level, range and count of tables; the refusal's place in the writer; the design's
  "The forms" and card 2 amended. No code.
- **Not here:** the build (Sprint 315).

## prerequisites

- 313 (closed)

## context_files

- the per-vCPU pages design and its Decision; reviewer-3's Medium 1 at 312; `crates/w2m_vm/src/tables.rs`, `guest.rs`,
  `compile.rs`; `tools/guest_runtime/block_compiler.c` 85-89, 237; `pe_loader` (`DLL_BASE`, `TLS_VA`, `LOADER_STUB`,
  `THUNK_BASE`, `map_image`); `guest_layout`

## signal contract

### Emits

- None; a design amendment.

### Invariants

- No code changes.

## artifact contract

### Files modified

- `docs/design-each-vcpus-own-pages-2026-10-31.md`

### Command exit codes

- `scripts/gates.sh` returns 0; `design_check` passes

## observation contract

- **Expected values:** each run-time descriptor writer with the levels and ranges it writes under 0xFE and 0xFF, by
  file and line; the pre-made entries' count of tables, against the 32 pages of a `VCPU_STRIDE` slot and the shared
  table area; `design_check` passing.
- **Planted faults:** (1) the design's goal section struck of "six threads": `design_check` must name the figure.

## done criteria

The upper-table entries a run-time mapping can need under the private roots are listed from their writers, so the build
makes every private table once and refuses a later entry.

## self-review checklist

- [x] Scope names one verifiable property: the pre-made entries, found from every writer.
- [x] context_files cite the design, the review, the writers and the layout.
- [x] Emits: none.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and a plant changing what its judge reads.
- [x] One concept: one list of entries made at load.

## result

