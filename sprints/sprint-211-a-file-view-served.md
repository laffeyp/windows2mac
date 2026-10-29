# Sprint 211 — A file view served

```yaml
---
id: 211
status: done
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

## result

- **The request:** `host_request::map_view` serves HOST_MAP_VIEW 29 as vm_backend 3857-3864 does: `FileBridge::view`
  reads the descriptor's first bytes with one positional read at offset 0 (a descriptor not open stops by the frozen
  host's words), `Arena::map_view` takes the 64 KB-aligned MEM_MAPPED region, the bytes are copied in without the exit
  ledger's log (the frozen host's `self.mem.write` is the loader's memory, which its fixture does not wrap), and
  `serve` writes the answer (second 0, the base).
- **The run:** the stream equals the reference rows 0-1354, 430 rows past Sprint 210's 925, and stops between calls:
  HOST_THREAD 30's stack growth of thread 8, a created thread, "a created thread's growth, not carried yet", exits
  hvc 1 268, hvc 4 9, hvc 8 801, hvc 11 9, hvc 13 18; the held stop file's run rows rewritten with the Edit tool.
- **The record:** killed, with no SERVICE_UNIMPLEMENTED. `Guest::called_service` read the word at the guest's RSP and
  named KERNEL32.dll!RtlUnwindEx at a return address in mono; the reference's thread 8 never calls RtlUnwindEx (0 of
  its rows), so the word was older code's, left on the stack of a thread stopped by a fault. `RunEnd::of` names a
  service only when the call resolved there is also in flight in the call trace, else the run is `RunEnd::Unserved`;
  the frozen host reads the word for every refusal (vm_backend 3804-3805), which is not followed.
- **The judges:** `batch_run.rs` holds the stream and, when a call is in flight at the stop, that call; at a stop
  between calls it holds the rows alone. `batch_record.rs` holds the counts to the frozen host's fixture when the run
  stops at the fixture's request and to the held stop file past it, and holds that a stop with no call in flight names
  no service. `exit_ledger.rs` compares the game's rows to the fixture's stop, its stop row without the columns read
  at resume, and does not hold the rows past it (the Decision keeps the ledger at the first MAP_VIEW); its held file is
  unchanged.
- **Plant 1** (the bytes read from offset 1): the unit test failed, "the file's first byte at the base, left: 2 right:
  1". Restored byte for byte.
- **Plant 2** (the second word 1): "the second word, left: 1 right: 0". Restored byte for byte.
- **Plant 3** (the protection ignored, PAGE_READWRITE always): "left: (262144, 4, 4) right: (262144, 2, 2)", the
  query's kind, protection and allocation protection. Restored byte for byte.
- **Plant 4** (MAP_VIEW unserved again): `batch_run.rs` failed, "the batch run's first stop or a guest's stop moved
  from store/batch_run/first-stop-2026-10-28.tsv". Restored byte for byte.
- **Departures:** the card named census row 4; the stream's edge is row 5a's, which is where it was written. The card's
  files did not name `record.rs`, `bin/w2m-vm.rs`, `tests/batch_record.rs`, `tests/batch_run.rs` and
  `tests/exit_ledger.rs`: the new stop, between calls, showed that the record named a service from a stale stack word
  and that two judges assumed a stop inside a call or at the fixture's request. Those were changed here, each named
  above, rather than left wrong under a passing run. Sprint 210's CI run failed on Linux, `record.rs` naming the
  Mac-only `run` module: `record::StopKind` and `Stop::kind` now carry the stop's kind, and gates.sh type-checks the
  workspace for Linux (a plant, the Mac-only type again, failed it). reviewer-3's review of this sprint found a failure
  in serving a request still read as not served: `HostError` is now `NotServed` (the frozen host's `Unimplemented`
  cases and the requests the Rust host has not carried) and `Failed` (a fault); a read error in MAP_VIEW, an OSError
  and so a fault on the frozen host, is `Failed` here too. `RunEnd::of` trusts only the stopping thread's calls in
  flight. The sprint carried four concepts, the service, Sprint 210's CI repair, the end rule and three judges, against
  the one-concept rule; each is named here.
