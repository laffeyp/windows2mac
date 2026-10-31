# Sprint 281 — The guest's exit-time services reach memory through the view

```yaml
---
id: 281
status: active
phase: 6
pass_kind: functional
---
```

## scope

Census row D7's third module (the step 6 sequence Decision, annotated at 278). `crates/w2m_vm/src/guest.rs` holds 20
slice calls, none in its tests (which start at line 1488). Four run at load, before any vCPU exists, which the design's
choice 1 allows to keep their slices: `Guest::new` (807), `with_trace` (969, 988) and `com_vtables` (1007). The other
16 run at the guest's exits, while on two vCPUs another may run: `place_cache_file`, `page_descriptor`, `map_pages`,
`called_service`, `trace_owner`, `trace_ranges` (three), `traced_proc` (two), `traced_strings`, `trace_count`,
`trace_record`, `init_stub`, `exception` and `clear_exception`.

- **The rows of the design's table they cite** (`docs/design-guest-memory-on-several-vcpus-2026-10-31.md`): the
  translation tables' descriptors (`page_descriptor`, `map_pages`: the host's own, written at exits; no vCPU writes
  them; a lookup on 2026-10-31 found the monitor code's broadcast invalidation, `TLBI VMALLE1IS; DSB ISH; ISB`, after
  a dispatch miss's `hvc 1` and after the code-write exit (`monitor_code.rs` 769-776, 804), while `map_pages` is
  called by the scheduler's stack growth (`threads.rs` 889, pages made valid) and stack release (963, pages made
  invalid); a page made valid needs no invalidation, as a translation fault is never served from the TLB ("since the
  fault is from a TLB-miss it reads from memory", the Armv8-A relaxed virtual memory test CoTfRpte+dsb-isb,
  https://www.cl.cam.ac.uk/~pes20/RelaxedVM-Arm/tests/CoTfRpte+dsb-isb.html, from arXiv 2203.00642; the Arm ARM's own
  sentence not found in the lookup), while a page made invalid needs one on every vCPU, and whether the release's exit
  path makes one is not yet read); the call
  trace's area (copy; the guest appends under the runtime word); a vCPU's own areas (`exception`, `clear_exception`:
  its vCPU is stopped at the exit served).
- **The work:**
  - The 16 calls at exits go through `Guest::memory_view`; the four at load stay, each with a comment that it runs
    before any vCPU.
  - `tests/slice_calls.rs` and census D7: `guest.rs` at 4.
  - The design's table gains a row for the translation tables, with the invalidation each change needs on several
    vCPUs and where the code makes it; if the stack release's path makes none, the row names it as a defect for two
    vCPUs and census D7 the card that adds it.
- **Reviewer-3's review at 280's close** (Surfaced, this card's commit):
  - Low 2, a length the slice enforced now held only by its caller: the sweep of 278's and 280's diffs finds 17
    copies made `write`s; 16 took the slice's length from their source (`bytes.len()`, `data.len()`, `raw.len()`,
    `n` with `&raw[..n]`, an 8-byte `to_le_bytes()`), so the slice checked nothing; one, the XMM capture's
    `at..at + 256` (`host_request.rs`), checked its 256 bytes. This card makes the capture a `[u8; 256]`, so the
    length is part of the type.
  - Low 1, the reconstruction table's Acquire read: the step 6 sequence Decision lists it as a precondition of the
    card that runs the second vCPU, beside the FEAT_LSE2 refusal and this card's TLB question.
- **Not here:** the translation tables' writer (`tables.rs`), which `page_descriptor` and `map_pages` call with a
  slice: census D7 counts it with the other modules.

## prerequisites

- 280 (closed)

## context_files

- `crates/w2m_vm/src/guest.rs`; `crates/w2m_vm/src/tables.rs`
- `docs/design-guest-memory-on-several-vcpus-2026-10-31.md` (the table); `docs/step-6-census-2026-10-30.md` (D7)
- `crates/w2m_vm/tests/slice_calls.rs`

## signal contract

### Emits

- None; an access path, held by tests and the one-vCPU runs.

### Invariants

- One-vCPU runs unchanged: the batch run's stop and stream, the call trace's records, the 68 families.

## artifact contract

### Files modified

- `crates/w2m_vm/src/guest.rs`, `crates/w2m_vm/src/host_request.rs` (the capture), `crates/w2m_vm/tests/slice_calls.rs`, `docs/step-6-census-2026-10-30.md`,
  `docs/design-guest-memory-on-several-vcpus-2026-10-31.md`, `BLACKBOARD.md`

### Command exit codes

- `scripts/gates.sh` returns 0

## observation contract

- **Expected values:** `guest.rs` 4 slice calls, all at load; the run-loop, batch-run and call-trace tests pass as
  before.
- **Planted faults:**
  - (1) `trace_count` reading its count one word off: a call-trace test names the count.
  - (2) one `as_slice()` added to an exit-time function: the count test names the file and its count.

## done criteria

The guest's services that run at exits reach guest memory only through the view, and the load-time slices are named
as such.

## self-review checklist

- [x] Scope names one verifiable property: the exit-time services' access through the view.
- [x] context_files cite the module, the tables' writer, the table, the census and the count test.
- [x] Emits: none.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and two plants.
- [x] One concept: one module's exit-time access moved.

## result
