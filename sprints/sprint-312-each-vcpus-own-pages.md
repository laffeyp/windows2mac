# Sprint 312 — Each vCPU's own pages

```yaml
---
id: 312
status: done
phase: 6
pass_kind: functional
---
```

## scope

The run-loop design's precondition 9 and card 4c2 (`docs/design-run-loop-on-several-vcpus-2026-10-31.md`, split at
this card). The Rust host reaches per-vCPU state at fixed addresses: the monitor code its data at `MONITOR_VA +
MONITOR_DATA`, the C runtime its state block and transport page at process-area offsets, the runtime's C frames one
stack; `compile.rs` 6-7 writes a descriptor once, "One vCPU: no private tables". A second vCPU would share all of it.
The Python host's answer, each vCPU's own level-0 table with a private chain to its own pages (Sprint 103's design B,
built at 104, the process words moved at 106), was not carried to Rust. Building before the form is decided would
repeat 309's first draft, so the form is decided first.

- **The work, a design** (`docs/design-each-vcpus-own-pages-2026-10-31.md`): the per-vCPU state the Rust host and the C
  runtime reach today, by word, address and reader (`monitor_code.rs`, the translator's lowering, `tools/guest_runtime`,
  `guest_layout`), each marked the running thread's, the vCPU's or the process's; the process-wide words on the
  per-vCPU pages and where they go; the form, design B carried (each vCPU's level-0 table and private chain, every
  descriptor write copied) against design A (addresses relative to a per-vCPU base) and any third the reading finds,
  decided with grounds from the Rust code as it is; the cards it shapes, before 4c3's threads. A BLACKBOARD Decision.
  No code.
- **Not here:** the pages built (the cards the design names); the threads (4c3).

## prerequisites

- 311 (closed)

## context_files

- the run-loop design's precondition 9 and card 4; `docs/design-several-vcpus-2026-10-17.md` (designs A and B, the
  inventory of 2026-10-17); `crates/guest_layout/src/lib.rs` (`MONITOR_DATA`, `VCPU_AREA`, `VCPU_STRIDE`);
  `crates/w2m_vm/src/monitor_code.rs`, `guest.rs` (`page_descriptor`, `ttbr`), `compile.rs`; `tools/guest_runtime/runtime.h`;
  `crates/w2m_vm/tests/litmus.rs` (several vCPUs on one monitor data page)

## signal contract

### Emits

- None; a design.

### Invariants

- No code changes.

## artifact contract

### Files modified

- `docs/design-each-vcpus-own-pages-2026-10-31.md` new; BLACKBOARD's Decision at the close

### Command exit codes

- `scripts/gates.sh` returns 0; `design_check` passes

## observation contract

- **Expected values:** the inventory, each word or page with its address, its readers by file and line and its owner;
  the count of absolute references design A would rewrite against the tables design B would copy, counted in today's
  Rust and C; `design_check` passing.
- **Planted faults:** (1) the design's systems section struck: `design_check`, which holds a design to its goal and the
  systems that solved the problem, must fail naming it.

## done criteria

The form in which each vCPU has its own state is decided from the Rust host's own references, before any second
thread runs.

## self-review checklist

- [x] Scope names one verifiable property: the per-vCPU form, decided by a counted inventory.
- [x] context_files cite the design that needs it, the Python-era design, the layout and the readers.
- [x] Emits: none.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and a plant changing what its judge reads.
- [x] One concept: one design decision.

## result

- **The inventory** (`docs/design-each-vcpus-own-pages-2026-10-31.md`): the monitor data page, its set-once words,
  the state block, the runtime stack, the transport page, the exception area and the shared monitor-area tables, each
  with its address, owner and readers. The guest's absolute references: the C runtime 22 (12 `W2M_MONITOR_DATA_VA`, 9
  `W2M_STATE_VA`, 1 `W2M_SAVE_VA`), the monitor code 9, the lowering 7. The host's reads by offset: 97 of the monitor
  data page in 7 files, 26 of the state block and transport page in 10. The transport page and the exception area are
  written only inside the runtime word (`runtime.c` 25-36; Sprint 106), so they stay shared; the private set is the
  Python host's, 18 pages.
- **The decision:** B, Sprint 103's form: each vCPU past the first its own level-0 table and 8 tables down to its 18
  pages, all in its `VCPU_STRIDE` slot (26 of 32 pages), `TTBR0_EL1` per vCPU with `CnP` 0, the set-once words copied
  at its creation, no guest code changed; A would change 38 guest references and the runtime image. Both need the
  host's 123 reads to take the exiting vCPU's base. A run-time descriptor write into a private chain's range refused by
  an assertion.
- **Found:** the C runtime's instruction clock reads the running vCPU's own count (`process.c` 45), so a run on the
  instruction clock stays on one vCPU until a card decides how counts combine, refused by name in card 4c2b.
- **The cards it shapes:** 4c2a, the host's per-vCPU base; 4c2b, each vCPU's private chain, with Sprint 104's judge
  (two vCPUs reading one address, each its own word); then 4c3, the threads.
- **Sources looked up for the systems section:** Linux arm64 `set_my_cpu_offset` writing `tpidr_el1` (`percpu.h`
  18-21, through `gh api`); Wine's `NtCurrentTeb`, `%gs:0x30` on x86-64 and `x18` on aarch64 (`include/winnt.h` 2723,
  2737); QEMU's multi-thread-tcg.rst, "Structures held on a per-vCPU basis won't need locking unless other vCPUs will
  need to modify them". Microsoft's `NtCurrentTeb` page names no register, so the design cites Wine's header for GS.
- **Expected values:** `design_check` passing, "7 plans and designs state the goal and the systems".
- **Plant 1,** restored by `cp` and checked by `cmp`: the systems heading renamed; `design_check` exit 1,
  "docs/design-each-vcpus-own-pages-2026-10-31.md: no "## Against the systems that solved it" section".
- **The gates:** `scripts/gates.sh` exit 0 in 15 s, kept in `store/gates/passes.tsv` as sprint 312 at 614e6fb63 with root=2ce2e78dd and "rust=none python=0 samples=0", no crate changed, base=c1919f9be, Sprint 311's close; multi-vCPU: not run (Python gates not run)

