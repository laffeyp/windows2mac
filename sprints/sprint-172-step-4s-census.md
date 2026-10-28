# Sprint 172 — Step 4's census, sentence by sentence

```yaml
---
id: 172
status: done
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

## result

- **The census:** `docs/step-4-census-2026-10-28.md`, 19 rows: every sentence of plan 132-142, the code review's
  findings 5, 8, 9 and 11, reviewer-2's items (a)-(j) of 2026-10-28 and its Medium 1 and 2 of 2026-10-27. Each count
  was read from its source this sprint: `class-runs.json` (458 classes, 456 conform, FCOS and FSIN not),
  `family-runs.json` (68 of 68 conform, 3 with notes of unimplemented argument forms), the batch run's record
  (`frontier-643a6283510c-1792267359516238000`: 2,095,145,118.8 instructions a second; exits hvc 8 44,010, hvc 13
  13,673, hvc 1 1,091, hvc 11 286, hvc 10 258, hvc 14 167, hvc 4 8, hvc 12 2, equal to Sprint 108's card),
  `run.rs` 446-586 (Rust serves hvc 1, 8, 12, 13; 4, 10, 11, 14 end at `Stop::Hvc`), `host_request.rs` (6 ops),
  `tests/data/timed_waits` (14 guests, 2 run by the Rust host), the C words judge (1,933 unlifted).
- **What it shows:** the Rust side holds none of the gate's three judges (classes, families, the call comparison) and
  2 of 14 guests; hvc 4, 10, 11 and 14 and the unserved host requests end a Rust run before the batch run could.
- **Not on any record:** host requests by op, the DLLs and imports as counts, flag liveness by condition, compile time
  per block on the batch run; each row says which measurement closes it.
- **Three cells corrected before the commit** to what their sources say: the loader's row, "512 class verdicts over
  four profiles" (not 512 classes), and the 116,718 cases as Decision 2026-10-27 words them.
- **The plant** (row 5's hvc 8 count changed to 44,011): the read-back of row 5 against the record printed "row 5
  differs from the record: hvc 8 44,010"; restored, it reads back clean.
