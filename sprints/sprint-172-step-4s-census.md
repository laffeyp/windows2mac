# Sprint 172 — Step 4's census, sentence by sentence

```yaml
---
id: 172
status: active
phase: 6
pass_kind: functional
---
```

## scope

Step 4 of `docs/rust-port-plan-2026-10-24.md` opens with a census (Decision 2026-10-27), taken from plan lines 132-142
sentence by sentence (Decision 2026-10-28, after reviewer-2's High 2: the census planned at Sprint 165 left out about
half of the step's text; KIT_DIARY #69). The census is a committed table, `docs/step-4-census-2026-10-28.md`, one row per
sentence of the step and per item the reviews and the code review attach to it, each row with:

- **what the plan or review asks**, quoted with its line;
- **the source** the count comes from: a record under `store/runs/` (the batch run's, `frontier-643a6283510c-*`), a
  committed file, or a command, named exactly;
- **the count today** in the Python oracle and in Rust;
- **the Rust status:** done, partial (what is missing), or absent;
- **the judge that closes it**, and whether that judge exists in Rust.

The rows, at the least:

1. The skeleton grows by component, each with the Python tests' values carried into Rust tests by hand and cited by
   file and line (plan 132-133): per component, the Python tests and the Rust tests that carry their values.
2. Loading's remainder (S2).
3. The class judge over every class (S3), gate "456 of 458 classes": the Python class judge's record and a Rust class
   judge (absent).
4. The model and slow paths (S4): model-step classes, by count on the batch run.
5. Every exit kind: the batch run's exit kinds and host-request ops, by count, each with the Rust host's service.
6. The family judge, gate "68 of 68 families": the Python's family records and a Rust family judge (absent).
7. The batch run's call comparison, gate "168 lines, 0 divergences": the comparison's record and a Rust comparison.
8. The guests in `tests/data` with their stored output: by guest, the Rust status.
9. The Python host's code under `oracle/` (git mv) and out of CI; the two discrepancy ledgers merged (amendment
   139-142).
10. Instructions a second on one vCPU, against Sprint 108's 2,095,145,119.
11. The DLLs and imports the batch run loads and calls, each with its Rust status.
12. The C words judge's per-class counts held to the census, so a class at 0 shows (reviewer-2's Medium 1 of
    2026-10-27).
13. Flag liveness through each of the eight conditions, counted on the batch run (reviewer-2's High 1 of 2026-10-28).
14. The emits the batch run needs: EXECUTION_STARTED with `timeout_seconds`, EXECUTION_COMPLETED (and when
    `guest_trace` is false), SERVICE_UNIMPLEMENTED; which Rust producer writes each (none today).
15. The code review's rows: finding 5 (guest values that panic the host), 9 (errors as strings), 11 (a full code cache
    ends the run), 8 (a compile time per block against 200 µs), 14 (block composition judged against Windows).

## prerequisites

- 171 (closed)

## context_files

- `docs/rust-port-plan-2026-10-24.md` 100-186; BLACKBOARD `## Surfaced for review` (the reviews of 2026-10-27 and
  2026-10-28, the code review); `store/runs/frontier-643a6283510c-*`; `crates/translator/tests/c_words.rs`;
  `tests/data/`; `src/w2m/` (the Python host)

## signal contract

### Emits

None: the census is a document read from records and files.

### Invariants

- No code changes; no Python file added or changed.

## artifact contract

### Files created

- `docs/step-4-census-2026-10-28.md`

### Command exit codes

- `target/release/design_check` and `scripts/gates.sh` return 0

## observation contract

- **Expected values:** every sentence of plan 132-142 has a row; every row names a source a reader can run or open, and
  a count read from it, not written from memory; reviewer-2's items (a)-(j) of 2026-10-28 each map to a row.
- **Planted fault:** a row's count changed by one; reading its source back against the table names the row (the
  read-back the card requires before the close, every count quoted from its source's output in the result).

## done criteria

Step 4's work is a list of rows with sources, counts and judges, so each later sprint closes rows.

## self-review checklist

- [x] Scope names one verifiable property: a row per sentence, each with its source and count.
- [x] context_files cite the plan, the reviews and the sources.
- [x] Emits: none.
- [x] Artifact contract gradable by command.
- [x] Observation contract: coverage, sourced counts, a plant.
- [x] One concept: the census.
