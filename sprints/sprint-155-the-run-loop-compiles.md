# Sprint 155 — The run loop compiles: declined blocks translated in Rust, and `qpc` past its first decline

```yaml
---
id: 155
status: active
phase: 6
pass_kind: functional
---
```

## scope

Link 5's other judge (`docs/design-host-compile-path-2026-10-24.md` F.5: "each lets `qpc`, then `sleeper`, run
further"). The skeleton's run loop (`run_to_first_decline`, Sprint 142) stops at the first block the C compiler in the
VM declines and no bridge answers: `qpc` at 0x140001055 (`cmp r8, rax; jne`). The Rust translator (groups 1 and 2)
now compiles such a block, as the Python host's `_compile` does, and the run goes on to its next stop, which it names.

Carried from `src/w2m/vm_backend.py` (read in full: 4507-4651, 4778-4825, and `assemble_block` 922-970, read in Sprint
151):

- `_compile` (4588-4642) without the translation cache and the census: the block's instructions from `start` to the
  first branch, a thunk, a lift failure after the first, or 64; `_protect_code`; the flags live after the block
  (`_flags_live_out`, `_flags_live_in`, `LIVENESS_SCAN` 8); `assemble_block`; the words at the code cursor; the
  reconstruction marks, the tails, the blocks map;
- `_protect_code` (the page's stage-1 descriptor made read-only, a block over it split by `_page_descriptor`; the page
  in the code page set, `_page_set_add`), and `_code_written` on `hvc #13` (the blocks over a written page dropped,
  their dispatch slots emptied, the page writable again);
- `assemble_block` in `crates/translator`, with `insn_flags` and `nzcv_branch` as its flag arguments.

The run loop compiles on a decline and goes on; it stops by name at an instruction the Rust rules do not lift (the
class named), a CompileError, or an exit it does not serve.

## prerequisites

- 154 (closed)

## context_files

- `src/w2m/vm_backend.py` as above; `crates/w2m_vm/src/run.rs`, `guest.rs`, `tables.rs`; `crates/translator`;
  `crates/w2m_vm/tests/first_decline.rs`

## signal contract

### Emits

None this sprint: the skeleton's record comes at its gate (link 6), as the design orders; the run loop's stop is the
test's output.

### Consumes

`tests/data/timed_waits/qpc.exe`; the frozen Python host's compiles of `qpc` (a fixture made by running it, its sha256
committed).

### Invariants

- No Python file added or changed.

## artifact contract

### Files created

- `crates/w2m_vm/src/compile.rs` (the block compile, the page protection, the code-write exit), a test

### Files modified

- `crates/translator/src/lower.rs` (`assemble_block`), `crates/w2m_vm/src/run.rs`, `WORKING_AGREEMENT.md`,
  `BLACKBOARD.md` (Sprint 154's CI run, in the tail entry)

### Command exit codes

- `scripts/gates.sh` returns 0

## observation contract

- **Expected values, from the Python host:** the blocks the Rust host compiles for `qpc`, in order, (start, end), equal
  the frozen Python host's compiles of `qpc` in the same run, up to the Rust run's stop; the stop is named, and if it
  is a class not yet carried, the Python host compiled a block holding that class there.
- **Expected values, from the C emitter:** each Rust-compiled block's instructions, lifted alone at their addresses with
  the block's flag arguments, are covered by `c_words`' equality where the C stencils them (no new judge needed).
- **Planted fault:** `_flags_live_in`'s scan length set to 0 (every flag live out): the blocks' words change and the
  fixture comparison of words, where the fixture holds them, names a block.

## done criteria

The skeleton's run loop translates in Rust the blocks the VM's C declines, and `qpc` runs past its first decline to a
named stop, compiling the blocks the Python host compiles.

## self-review checklist

- [x] Scope names one verifiable property: `qpc` runs further, compiling as the Python host does.
- [x] context_files cite the Python carried.
- [x] Emits: none this sprint, with the design's reason.
- [x] Artifact contract gradable by command.
- [x] Observation contract: the Python host's compiles as reference, and a planted fault.
- [x] One concept: the run loop's compile.

## result
