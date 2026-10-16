# Sprint 084 — the reviewer's small review of 083: a block's pages protected in the VM, and the review's other asks

```yaml
---
id: 084
status: active
phase: 5
pass_kind: functional
---
```

## scope

The reviewer found Sprint 083 sound for one vCPU (no live state lost at the miss stub; replay order, the code-page
set and the cache sequence correct) and asked for five things:

- **(5) "Not a code page" is avoidable.** 3,930 of 4,106 host misses were blocks on pages no block covered yet: the C
  refused them because only a page the host had made read-only could be read safely. The C now protects them itself,
  as `_protect_code` does: it walks the stage-1 tables (`_page_descriptor`), splitting a 2 MB or 1 GB block descriptor
  into a table from the split-table area (its cursor becomes a monitor word the host and the C share), sets AP
  read-only, invalidates the page's TLB entry (TLBI VALE1IS, DSB ISH, ISB), adds the page to the code-page set and logs
  it; the host's replay creates its `code_pages` entry. Only a page below PROCESS_VA with a valid descriptor is
  protected, as on the host. Judged by the frontier run with the judge on: its host misses by cause, and the judge's
  counts.
- **The PROCESS_VA plant made observable.** My reason in 083 was half right: forms do depend on the address (a NOP has
  a form at 0x140001000 and 0x7FF712345000 and none at PROCESS_VA: the key holds the next address's value class, and
  there its second halfword is 0), but the MOV and CMP I placed have no form anywhere. A layout now jumps from below
  PROCESS_VA to PROCESS_VA + 0x12345000 onto a flag writer that has a form there; the host's scan stops at the
  jump, the planted C would read on.
- **The runtime stack's invariant written down.** The miss stub resets SP to the stack's top, which is safe only while
  no runtime frame is live when guest code runs (guest_calls.c's continuations); runtime.h says so beside
  `w2m_call_guest`, and a test checks that SP is the stack's top at every miss in a run whose services call guest
  code.
- **Layout constants in the table.** Beside the stack's top the table holds PROCESS_VA + 0x3000 (181 uses) and
  PROCESS_VA + 0x30b0 (12): the rule is that a layout change re-derives the table, and the instruction judges in CI
  are what enforce it (they compile against the current layout). The contract's next version says so.
- **Counts on the record.** The frontier record carries the VM's compile counts as MEASUREMENT_RECORDED (metric
  `vm_compile.<cause>`), so Decision 2026-10-15's measure reads from signals.

## prerequisites

- 083 (closed on CI)

## context_files

- `tools/guest_runtime/block_compiler.c`, `src/w2m/vm_backend.py` (`_page_descriptor`, `_protect_code`,
  `_vm_compiled`), `scripts/successor_scan_judge.py`, `scripts/run_frontier.py`, `tools/guest_runtime/runtime.h`

## signal contract

### Emits

- MEASUREMENT_RECORDED (metric `vm_compile.compiled` and one per refusal and cut cause; unit `blocks`) in the
  frontier record. No new tag.

### Invariants

- Vocabulary unchanged. A block the VM compiles has `assemble_block`'s words.

## artifact contract

- `tools/guest_runtime/block_compiler.c`, `src/w2m/vm_backend.py`, `scripts/successor_scan_judge.py`,
  `scripts/run_frontier.py`, `tools/guest_runtime/runtime.h`, tests
- `scripts/gates.sh` exit 0; classes and families re-run on the committed tree; CI `success`

## observation contract

- A judged frontier: "not a code page" near 0, 0 differing, 0 short; the same stop and calls as with the compiler off.
- The PROCESS_VA plant fails the spec judge.

## done criteria

The VM compiles blocks on pages no block covered yet, with the host's words; the review's other asks closed.
