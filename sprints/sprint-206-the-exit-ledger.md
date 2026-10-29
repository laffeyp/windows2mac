# Sprint 206 — The exit ledger

```yaml
---
id: 206
status: active
phase: 6
pass_kind: functional
---
```

## scope

The Decision "Sequence after Sprint 205" (reviewer-2's High 1 and Medium 1 of its review of Sprints 199-205). The Rust
and Python hosts ran the same 2,648,351 instructions before mono's load, yet the Rust game process carried a clock
warp of 609 ticks that the Python's did not, and no judge compares a clock value between the hosts. The call stream
holds arguments, returns and last errors only; what a host writes through pointers and the clock it keeps are outside
it. This sprint builds the judge that sees both, to the batch run's mono load.

- **The frozen host's ledger:** a fixture generator runs the frozen Python batch run (`scripts/run_frontier.py`'s
  start) under WORKING_AGREEMENT's frozen-host fixture rule, its exit handler and `_host_request` wrapped, to mono's
  load, and writes one row per host exit of every process: process id, exit kind (the hvc number), x86 site, monitor
  word 0, the warp, and for a host request its op, arguments, answer and the sha256 of each guest range the host
  wrote while serving it. `build/exit_ledger/python.tsv`, pinned by `store/exit_ledger/python.sha256`.
- **The Rust ledger:** `w2m-vm --exit-ledger <tsv>` writes the same rows from `run.rs`'s exit handling and
  `host_request.rs`.
- **The judge:** `crates/w2m_vm/tests/exit_ledger.rs` compares the two row by row and names the first row that
  differs, with its process, its column and both values.
- **The run:** the judge's first difference, named; it answers which exit the 609 ticks of warp come from.

## prerequisites

- 205 (closed-partial)

## context_files

- `src/w2m/vm_backend.py` 5200-5300 (the exit handler, `self.exits`), 3796-3880 (`_host_request`), 2721-2729
  (`_counter_now`), 3614-3688 (`warp_to_deadline`, `_catch_up`); `src/w2m/kernel.py` 328-356 (`Machine.run`);
  `scripts/run_frontier.py` 215-280; WORKING_AGREEMENT "The case runner's inputs" (the frozen-host fixtures);
  `crates/w2m_vm/src/{run.rs, host_request.rs, threads.rs, turns.rs}`; `crates/w2m_vm/tests/run_loop.rs` (the
  existing fixture readers)

## signal contract

### Emits

- None: the judge is `exit_ledger.rs`. The emits come with their own card (the Decision's order).

### Invariants

- No Python file added; the fixture generator runs the frozen Python inline as the existing fixtures' generators do,
  and its command is recorded in WORKING_AGREEMENT. The batch run, guests and module families as before.

## artifact contract

### Files modified

- `crates/w2m_vm/src/{run.rs, host_request.rs, bin/w2m-vm.rs}`, `crates/w2m_vm/tests/exit_ledger.rs` (new),
  `store/exit_ledger/python.sha256` (new), `scripts/gates.sh`, `WORKING_AGREEMENT.md`,
  `docs/step-4-census-2026-10-28.md` (rows 7 and 14a)

### Command exit codes

- `exit_ledger.rs` returns 0 with the first difference held by name in `store/exit_ledger/first-difference-*.tsv`;
  `scripts/gates.sh` returns 0

## observation contract

- **Expected values:** the two ledgers agree on every row before the game's CreateProcessW (row 16 of the stream);
  the first difference is a warp or a turn order after it, named with its process and exit; two frozen-host runs give
  the same fixture bytes.
- **Planted faults:** (1) the Rust warp's catch-up doubled: the judge names the first catch-up row; (2) Sprint 204's
  plant 2, READ's count one more: the judge names the FILE_READ row by its written-range hash; (3) one instruction
  added to the Rust count at a block entry: the judge names the first row past it by monitor word 0.

## done criteria

Both hosts write the exit ledger to mono's load, and a test names the first exit where they differ.

## self-review checklist

- [x] Scope names one verifiable property: the first differing exit, named.
- [x] context_files cite the Python to read first.
- [x] Emits: none, said so.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and three plants.
- [x] One concept: the exit ledger.
