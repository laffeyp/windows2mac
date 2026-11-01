# Sprint 309 — How the vCPU threads share the guest

```yaml
---
id: 309
status: active
phase: 6
pass_kind: functional
---
```

## scope

The run-loop design's cards 4b and 4c (`docs/design-run-loop-on-several-vcpus-2026-10-31.md`, split at Sprint 307's
close) leave one question open, found before 4b's card was committed: in what form the vCPU threads share `Guest`. A
first draft of 4b made `served` a `Mutex<Served>` reached by `get_mut` at about 230 sites while one vCPU holds `&mut
Guest`; 4c would rewrite every one when the threads share `Guest`, so the form is decided first. Two forms:

- **`served` alone locked:** the 42 functions taking `&mut Guest` (20 in `threads.rs`) take the load part by `&` and
  `&mut Served` from the guard taken once per served exit, a signature change through each.
- **The whole `Guest` behind one lock taken per served exit, under the word:** the functions keep `&mut Guest`, from the
  guard; each vCPU thread keeps a copy of the load facts it reads outside a service and its own view of the memory. A
  served exit already holds the runtime word, so the lock adds no contention to the word's.

- **The work, a design:** every read and write of `Guest` the run loop makes outside a service, listed by field and
  site (`run.rs`' loop, `turn`, the exits' counts), to show which form holds; the design's "The guest's parts" and card
  4 amended with the decision, grounds and the cards it shapes; a BLACKBOARD Decision. No code.
- **Not here:** the lock itself (4b, rewritten by this), the threads (4c).

## prerequisites

- 308 (closed)

## context_files

- the design's "The guest's parts", "The threads" and "The cards" 4; `run.rs` (`run_compiling`, `turn`, `take_word`);
  `guest.rs` (`Guest`, `Served`, `memory_view`)
- the guest-memory design's choice 6 (the lock order: the word outermost)

## signal contract

### Emits

- None; a design.

### Invariants

- No code changes.

## artifact contract

### Files modified

- `docs/design-run-loop-on-several-vcpus-2026-10-31.md`; BLACKBOARD's Decision at the close

### Command exit codes

- `scripts/gates.sh` returns 0; `design_check` passes

## observation contract

- **Expected values:** the list of `Guest`'s reads and writes outside a service in the run loop, each by line, and
  the form it supports; `design_check` passing.
- **Planted faults:** (1) the design's decision paragraph struck: `design_check`, which holds a design to its goal and
  systems, must still pass, so the plant is the Decision's revisit clause struck instead: `signal_report --check` must
  name the Decision without one.

## done criteria

The form in which the vCPU threads share `Guest` is decided from the run loop's own reads, before any lock is built.

## self-review checklist

- [x] Scope names one verifiable property: the sharing form, decided by a listed reading.
- [x] context_files cite the design, the run loop and the lock order.
- [x] Emits: none.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and a plant changing what its judge reads.
- [x] One concept: one design decision.

## result

- **The reading** (WORK-COMMIT, the design): in `run.rs` `run` (693-1261), outside a service: the Load part
  (`code.dispatch_va`, `code.miss_va`, `ttbr`, `entry_va`, `layout`, `start.tid`, `time_runtime_word`), the mapping
  through the view (19 `memory_view` calls), the diagnostics (`exit_ledger`, `block_judge`, `batch_stop`, refused on
  two vCPUs by Sprint 308), and the turn's start's `served.link` and `served.threads` through `slice_until`
  (786-800, `threads.rs` 1099-1113), without the word; the model step (1074-1075) touches memory and the code's
  addresses alone. Inside a service, under the word: `take_word` at 928, 1026, 1113 and 1196 before `hvc #13`, `#12`,
  `#4` and `#1`, and `host_request::serve` (952) under the guest's own hold.
- **The decision:** the whole `Guest` behind one `Mutex`, taken once per served exit inside the word, the 42 functions
  keeping `&mut Guest`; each thread a copy of the Load facts and its view; the turn's start's two reads under the word
  or moved to monitor data, card 4c measuring which. Written into the design's "How the threads share `Guest`" and
  card 4's split (4b), and as a BLACKBOARD Decision with the close, which supersedes "Architecture: when `served`
  becomes a lock" (its `&mut Served` passed down).
- **Expected values:** `design_check` passing; the record check passing over both Decisions.
- **The gates:** GATES-LINE
- **Plant 1,** restored by `cp` and checked by `cmp`: the new Decision's revisit clause struck; `signal_report --check`
  failed, "holds 1 decision clauses and 0 revisit clauses, not one each".

