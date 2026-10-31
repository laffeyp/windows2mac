# Sprint 281 — The guest's exit-time services reach memory through the view

```yaml
---
id: 281
status: done
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

- **The access** (45382e739): 15 calls at exits through `Guest::memory_view`, not the card's 16:
  `place_cache_file` runs at load, before the run loop (`w2m-vm.rs` 853-856). The five at load (`Guest::new`,
  `with_trace` twice, `com_vtables`, `place_cache_file`) keep their slices, each commented as running before any
  vCPU. `called_service` checks its six bytes against the memory's end, as `mem.get(..)` did.
- **The translation tables:** `map_pages` changes a descriptor by one atomic load and a Release store;
  `page_descriptor` writes a split table's 512 descriptors, then stores the entry naming the table with Release.
- **The TLB question:** settled by lookup. The runtime broadcasts `TLBI VMALLE1IS; DSB ISH; ISB` after the requests
  that map or unmap pages (`runtime.h` 331-335, at `threads.c` 318 and `runtime.h` 342); a page made valid needs
  none, as a translation fault is never served from the TLB (the Armv8-A relaxed virtual memory test CoTfRpte). A
  fourth precondition of the second vCPU found instead: a block split changes the block size, which needs
  break-before-make while another vCPU runs (Will Deacon on LKML, quoted in the design's new row); this Mac's
  `sysctl` lists no FEAT_BBM. The design's table and the step 6 Decision say so.
- **Low 2:** the XMM capture is a `[u8; 256]` filled 16 bytes a register.
- **A commit whose lint failed:** the pre-commit run's lint refused `chunks_exact_mut` with a constant size in the
  capture; only the run's tail, the tests' line, was read before 45382e739 was committed and pushed and the gates
  started. The gates were stopped before they wrote a row, the capture fills its registers by `as_chunks_mut::<16>`
  (6972ba75c), and the lint, the Linux check, the module's and the run-loop tests ran again on it.
- **The count:** 91 slice calls in `crates/w2m_vm/src/`, from 106; `guest.rs` 5, all at load.
- **The gates:** `scripts/gates.sh` exit 0 in 674 s, kept in `store/gates/passes.tsv` as sprint 281 at 6972ba75c with root=147f6ba5e and "rust=crates python=0 samples=0", rdeps(w2m_vm), 156 tests, base=9ea9d40ee, Sprint 280's close; multi-vCPU: not run (Python gates not run)
- **Plant 1,** restored by `cp` and checked by `cmp`, `w2m-vm` rebuilt: `trace_count` reading its count one word off. The card named "a call-trace test"; `the_batch_runs_calls_are_the_python_runs_up_to_the_stop` passed under it in 0.08 s, as it compares the stored records and runs no guest, so the plant ran on the live game instead (`w2m-vm --game` as the gates run it, the stop in the scratchpad): the run reached the same `ReferenceEnd` and its stop file read "calls in flight 30557" where the held one reads "calls in flight 0", the line `the_batch_run_and_the_guests_stop_where_held` compares.
- **Plant 2,** restored by `cp` and checked by `cmp`: an `as_slice()` added to `clear_exception`'s doc; the count test failed, "guest.rs: 6 slice calls, census D7 allows 5".
