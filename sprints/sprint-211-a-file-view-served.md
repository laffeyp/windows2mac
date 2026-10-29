# Sprint 211 — A file view served

```yaml
---
id: 211
status: active
phase: 6
pass_kind: functional
---
```

## scope

The Decision "Sequence after Sprint 210" (reviewer-3's High 2 of its review of Sprints 205-209): the Rust host stops
the batch run at row 925 of the 10,505 the call stream judges, on HOST_MAP_VIEW (op 29), "host request 29: not served
by the Rust host yet", arguments 0x3b9a00 0x7 0x2 (store/batch_run/first-stop-2026-10-28.tsv). The frozen host serves
it at vm_backend 3857-3864: a file descriptor not open stops by name ("a file view of descriptor N, which is not
open"); otherwise `virtual.map_view(a0, a2)` takes a 64 KB-aligned MEM_MAPPED region of the view's pages committed with
the protection, the file's first `a0` bytes are copied over it (`os.pread(fd, a0, 0)`), and the answer is `(0, base)`
at the transport's 48. `virtual_memory::Arena::map_view` is already the Rust port of `map_view` (lib.rs 477-480).

- **The request:** `host_request.rs` names HOST_MAP_VIEW 29 and serves it as the frozen host does, the bytes read
  through the file bridge's descriptor.
- **The run:** the stream equals the reference past row 925 to the Rust host's next stop, which the held stop file
  names; the exit ledger stays held at the first MAP_VIEW, as the Decision says.

## prerequisites

- 210 (closed)

## context_files

- `src/w2m/vm_backend.py` 282-289, 3857-3864; `src/w2m/virtual_memory.py` 256-266; `src/w2m/file_bridge.py` (the
  descriptors); `crates/w2m_vm/src/host_request.rs`, `crates/w2m_vm/src/file_bridge.rs`,
  `crates/virtual_memory/src/lib.rs` 477-500; `crates/w2m_vm/tests/batch_run.rs`

## signal contract

### Emits

- The batch run's record (Sprint 210) carries the new stop: SERVICE_UNIMPLEMENTED names the next unserved request, or
  none if the run ends otherwise; `vm_exits` moves with it. No new emit site.

### Invariants

- No Python file added or changed; the stream equals the reference through every row the Rust host reaches; the exit
  ledger's held file unchanged.

## artifact contract

### Files modified

- `crates/w2m_vm/src/host_request.rs`, `crates/w2m_vm/src/file_bridge.rs`, `store/batch_run/first-stop-2026-10-28.tsv`
  (its rows rewritten with this card), `docs/step-4-census-2026-10-28.md` (row 4)

### Command exit codes

- `batch_run.rs`, `batch_record.rs`, `exit_ledger.rs`, the request's unit test and `scripts/gates.sh` return 0

## observation contract

- **Expected values:** the stream passes row 925 and equals the reference to its next stop, named in the held stop
  file with its row count; a unit test: a view of a file's first 0x3b9a00 bytes answers second 0 and a 64 KB-aligned
  base holding the file's bytes, VirtualQuery there gives MEM_MAPPED with the protection, and a descriptor not open
  stops by the frozen host's words.
- **Planted faults:** (1) the bytes read from offset 1: the unit test names the base's first byte; (2) the answer's
  second word 1: the unit test names it; (3) the protection ignored (PAGE_READWRITE always): the unit test names the
  query's protection; (4) the request left unserved again: `batch_run.rs` names the held stop.

## done criteria

The Rust host serves HOST_MAP_VIEW as the frozen host does, and the batch run's stream equals the reference past row
925 to its next stop.

## self-review checklist

- [x] Scope names one verifiable property: the stream past row 925.
- [x] context_files cite the Python to read first.
- [x] Emits: the record's stop moves; no new site.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and four plants, each reaching a named judge.
- [x] One concept: one host request served.
