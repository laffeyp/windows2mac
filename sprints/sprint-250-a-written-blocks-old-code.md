# Sprint 250 — A written block's old code

```yaml
---
id: 250
status: active
phase: 6
pass_kind: functional
---
```

## scope

A departure from the frozen host, found by the lookup for the translation cache's design (2026-10-30): after a guest
write to a page of translated code, the Rust host can hand back the in-VM compiler's old code for a block on that page.
The cache will put its blocks in the same maps, so the maps are made right first.

- **Where it stands, by lookup (2026-10-30):**
  - The frozen host keeps one map, `self.blocks`, of both compilers' blocks; `_vm_compiled` adds the in-VM
    compiler's, and `_code_written` pops every block over a written page from it (`vm_backend.py` at `python-final`,
    2559-2575, 4554-4578); an `hvc 1` reads `self.blocks.get(target) or self._compile(target)` (5242).
  - The Rust host keeps two: `Outcome::blocks`, the in-VM compiler's, which nothing removes from, and
    `HostCompiler::blocks`, both compilers' (`vm_compiled` inserts the VM's at `run.rs` 430), from which
    `code_written` drops a written page's blocks (`compile.rs` 304-330). The `hvc 1` lookup reads `out.blocks`
    first (`run.rs` 959-963), so a block dropped by a write and then declined by the C compiler gets its old code.
  - Whether the batch run reaches that path is not known: no count records it.
  - Searched for `run.rs` and `compile.rs` in Deferred, Surfaced and `docs/surfaced-archive-2026-10-30.md`: census
    row 15b names `run.rs` and `compile.rs` for string errors, which this card does not change; census row 15c names
    `compile.rs` 118, a full code cache, not touched.
- **The readers:** the `hvc 1` arm of the run loop; `out.blocks`' other reader, the exit ledger's blocks at the stop
  (`run.rs` 990), which wants every block either compiler made and is not changed.
- **The work:**
  - The batch run measured first: a count of `hvc 1` lookups that `out.blocks` answers and `h.blocks` does not, by a
    temporary count, one run, then removed; the number in the card's result.
  - The lookup as one function, `known_block`: with the host's compiler, its map alone; without it, the in-VM
    compiler's; a unit test holds a block dropped from the host's map unknown though the VM's map holds it.
- **Not here:** the cache's design, which follows.

## prerequisites

- 249 (closed)

## context_files

- `crates/w2m_vm/src/run.rs` 400-447, 950-975, 985-992; `src/compile.rs` 300-331
- `vm_backend.py` at `python-final` 2559-2575, 4554-4578, 5242

## signal contract

### Emits

- None of its own; the gates' batch run, which must still conform.

### Invariants

- The batch run conforms; its call stream, exit ledger and 274,834 compiles as before unless the count shows the
  path taken.

## artifact contract

### Files modified

- `crates/w2m_vm/src/run.rs`, `BLACKBOARD.md`

### Command exit codes

- `scripts/gates.sh` returns 0

## observation contract

- **Expected values:** the count of `hvc 1` lookups the old order answered from a dropped block, on one batch run.
- **Planted faults:**
  - (1) The lookup reading the in-VM compiler's map first again: the unit test names the old code handed back.
  - (2) The host's map ignored with the host present: the unit test names a host-compiled block unknown.

## done criteria

A block dropped by a write is unknown to the `hvc 1` lookup, as in the frozen host, and the record says whether the
batch run ever took the old path.

## self-review checklist

- [x] Scope names one verifiable property: a dropped block is not handed back.
- [x] context_files cite both hosts' maps, the lookup and the drop.
- [x] Emits: the gates' batch run.
- [x] Artifact contract gradable by command.
- [x] Observation contract: one value and two plants.
- [x] One concept: the blocks a write drops.

## result
