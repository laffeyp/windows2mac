# Sprint 328 — The host reaches a vCPU's own pages by address

```yaml
---
id: 328
status: active
phase: 6
pass_kind: functional
---
```

## scope

Deferred (Sprint 327, partial) and reviewer-3's Low 1 at 327. Form B gives each vCPU past the first its own monitor
data page, state block and runtime stack at vCPU 0's guest addresses (the per-vCPU pages design; `Guest::private_chain`,
Sprint 315). The host reaches the first two through `VcpuPages` (Sprint 313), and a guest address through
`Layout::offset` (`guest.rs` 280), the layout's maps alone, which are vCPU 0's: 17 calls in 7 files. The C runtime
passes the host pointers into its own frames on the runtime stack: `GetFileAttributesExW`'s `u64 st[4]`
(`file_attributes.c` 35-58), the host writing the stat; `FindFirstFileExW`'s `u16 full[FULL]` (`file_search.c` 113), a
path the host may read. On vCPU 1 those land in vCPU 0's stack: `statsize` on two vCPUs prints "sizes differ".

- **The work:** `Layout::offset` takes the served vCPU's pages, and a guest address in a private range (the monitor
  data page, the state block, the runtime stack) resolves to that vCPU's page; every caller passes the pages, which
  the compiler then holds, a caller with none naming vCPU 0's (`VcpuPages::FIRST`) by name; `VcpuPages` gains the
  stack's base, where `private_chain` places it. The per-vCPU pages design's table gains the host as a writer and a
  reader of the runtime stack (with the card), and the run-loop design records Sprint 327's build (with the card).
- **Not here:** the C runtime's frames (unchanged); the word's take at a stop (card 329).

## prerequisites

- 327 (closed partial)

## context_files

- BLACKBOARD Deferred (Sprint 327, partial); reviewer-3's Low 1 at 327; `docs/design-each-vcpus-own-pages-2026-10-31.md`
  (the table, 26-40); `crates/w2m_vm/src/guest.rs` `Layout::offset` and `private_chain`; `crates/w2m_vm/src/lib.rs`
  `VcpuPages`; `tools/guest_runtime/file_attributes.c`, `file_search.c`; `tests/data/timed_waits/statsize.c`

## signal contract

### Emits

- None; where the host reaches a guest address.

### Invariants

- Every run on one vCPU as held: `VcpuPages::FIRST` resolves as the layout does today.

## artifact contract

### Files modified

- `crates/w2m_vm/src/guest.rs`, `crates/w2m_vm/src/lib.rs`, the seven files of `Layout::offset`'s callers,
  `crates/w2m_vm/tests/` (the translation's test, `statsize` in the two-vCPU test); the two designs, with the card

### Command exit codes

- `scripts/gates.sh` returns 0

## observation contract

- **Expected values:** `statsize` on 2 vCPUs printing "done", as on one, added to `two_vcpus_end_each_guest_as_one_does`
  (the host writing a stat into vCPU 1's stack); a test of the translation: `MONITOR_VA + RUNTIME_STACK + 0x10`,
  `MONITOR_VA + MONITOR_DATA` and `PROCESS_VA + STATE_OFF` with vCPU 1's pages resolving into vCPU 1's slot
  (`VCPU_AREA + VCPU_STRIDE`, its page offsets as `private_chain` lays them), with `VcpuPages::FIRST` as the layout
  resolves them, and a shared address (`PROCESS_VA`) the same for both; every run on one vCPU as held.
- **Planted faults:** (1) the private ranges not consulted, every address through the layout as today: `statsize` on 2
  vCPUs prints "sizes differ" and the translation test names vCPU 1's stack address resolved into vCPU 0's; (2) the
  stack's base one page off in `VcpuPages`: the translation test names the stack address's offset. Each judge's reach
  read before this card's commit (WORKING_AGREEMENT, Sprint 284): the translation test calls `Layout::offset` with the
  pages; `statsize`'s worker asks `GetFileAttributesExW` on vCPU 1, read at Sprint 327 (`runs_by_vcpu` and the stat
  in vCPU 0's stack).

## done criteria

A host request's guest address resolves to the requesting vCPU's own pages, and `statsize` ends on two vCPUs as on one.

## self-review checklist

- [x] Scope names one verifiable property: a guest address resolved by the served vCPU.
- [x] context_files cite the gap, the review, the designs, the translation and the runtime's frames.
- [x] Emits: none.
- [x] Artifact contract gradable by command; the file count over 10 is the 7 callers of one function, whose
  signature change the compiler makes one change.
- [x] Observation contract: values and two plants, each judge's reach read.
- [x] One concept: the host's translation of a guest address per vCPU.

## result

