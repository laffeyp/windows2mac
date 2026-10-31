# Sprint 291 — A loaded DLL's entry published last

```yaml
---
id: 291
status: done
phase: 6
pass_kind: functional
---
```

## scope

Reviewer-3's Low 1 at 290, as the lookup at 290 settled it: `process::write_loader_data` writes every entry's fields
and names in its first loop, then each list's links node by node from the head (`link`, `process.rs` 618-627). On a
LoadLibrary the head's `Blink` and the old tail's `Flink` name the new entry before the new entry's own `Flink` and
`Blink` are written, so a guest thread on another vCPU walking `PEB->Ldr` can follow an unwritten link. The guest
runtime's module calls run under the runtime word, which the requesting vCPU holds; application code walking the
lists does not. Windows inserts an entry filled and linked, then publishes it.

- **The work:**
  - `process.rs`: each list's links written so every state between two writes is a whole list: the `Flink`s from
    the tail back to the first entry, then the head's; the `Blink`s from the first entry to the tail, then the
    head's. Each node's link is written only after the node it names has its own; links rewritten with the value
    they hold are harmless. `PEB.Ldr` last. Each link and `PEB.Ldr` a publishing write, `ProcessBytes::publish`: a
    plain write in a slice, at load, no vCPU running; a Release store of the aligned word through the view.
  - `crates/w2m_vm/tests/loader_lists.rs` new: the writes of a LoadLibrary recorded, applied one at a time onto the
    lists as they stood, and after each the three lists walked both ways from their heads.
  - The design's table: a row for the process area (the synthetic pages, the loader data, `PEB.Ldr`), which Sprint
    290's card cited and the table lacked.
- **Not here:** the load-time path's order, which no running vCPU sees; the other process-area writers.

## prerequisites

- 290 (closed)

## context_files

- `crates/w2m_vm/src/process.rs` 155-215 (`ProcessBytes`), 545-635 (`write_loader_data`)
- `crates/w2m_vm/src/modules.rs` 230-242; `crates/w2m_vm/tests/virtual_memory.rs` 455-463 (an image from a stored
  binary)
- `docs/design-guest-memory-on-several-vcpus-2026-10-31.md` (the table); BLACKBOARD Surfaced, the review at 290

## signal contract

### Emits

- None; an order of writes, held by a test.

### Invariants

- The game's and the family runs as held; `PEB.Ldr`'s final bytes the same as before for every list written.

## artifact contract

### Files modified

- `crates/w2m_vm/src/process.rs`, `crates/w2m_vm/tests/loader_lists.rs` new,
  `docs/design-guest-memory-on-several-vcpus-2026-10-31.md`

### Command exit codes

- `scripts/gates.sh` returns 0

## observation contract

- **Expected values:** after every one of a LoadLibrary's writes, each of the three lists walked forward and back
  returns to its head through entries whose `DllBase` is a loaded image's; the lists' final bytes equal to the
  writes' in the old order; the game's and the family runs as held.
- **Planted faults**, each judge the new test, which walks after each write:
  - (1) the links written in the old order, head first. Judge: the test, a walk reaching a link not yet written.
  - (2) the head's `Flink` written before the entries'. Judge: the same.

## done criteria

No state between two of LoadLibrary's writes to `PEB.Ldr` shows a walker a broken list, and each publishing write is
a Release store when the run runs.

## self-review checklist

- [x] Scope names one verifiable property: every state between the writes is a whole list.
- [x] context_files cite the writers, the caller, a test's image, the table and the review.
- [x] Emits: none.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and two plants, their judge the new test.
- [x] One concept: the order a list is published in.

## result

- **The order** (8a436e769): the entries' fields and names, then the header's two words, then each list's `Flink`s
  from the tail back to the head and its `Blink`s from the first entry on to the head, then `PEB.Ldr`; each link and
  `PEB.Ldr` by `ProcessBytes::publish`, a Release store of the aligned word in `ProcessView`.
- **Expected values:** `every_state_of_a_loadlibrarys_lists_is_a_whole_list` passing: a third DLL loaded after two
  images, first in init order, so inserted mid-list there and at the tail of the other two; 121 writes, the three
  lists walked both ways after each; the lists after the last equal to lists written whole into empty memory, the new
  DLL in each. 173 tests in the pre-commit run, the batch-run step first.
- **The card's "final bytes equal to the writes' in the old order":** the test compares with the lists written whole,
  not with the old order's, the old order's code being gone; the game's and the family runs, which hold the lists'
  readers, stand as held.
- **A lint the first run met:** the pre-commit run stopped at clippy, "useless use of `vec!`" (`loader_lists.rs` 92);
  an array, and the whole list run again.
- **The gates:** `scripts/gates.sh` exit 0 in 847 s, kept in `store/gates/passes.tsv` as sprint 291 at 8a436e769 with root=5efc1d580 and "rust=crates python=0 samples=0", rdeps(w2m_vm), 173 tests, base=aee4798a0, Sprint 290's close; multi-vCPU: not run (Python gates not run)
- **Plant 1,** restored by `cp` and checked by `cmp`: both loops from the head, the old order; the test failed,
  "after write 85 of 121 (0x544d8): list 0x7ffe00054010 forward: a link to 0x0, outside the loader data".
- **Plant 2 as the card had it could not fire:** the head's `Flink` written first passed, since it always names the
  first node, the program in two lists and ntdll in the init list (`init` begins with ntdll, kernelbase, kernel32),
  and a load never changes it; rewriting it first rewrites the value it holds. Replaced by the head's `Blink` written
  first, which names the new tail: the test failed, "after write 87 of 121 (0x54018): list 0x7ffe00054010 back: a
  link to 0x0, outside the loader data". Restored by `cp` and checked by `cmp`.
