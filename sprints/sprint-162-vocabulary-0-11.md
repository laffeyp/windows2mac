# Sprint 162 — Vocabulary 0.11: the metrics step 3's measurements need

```yaml
---
id: 162
status: active
phase: 6
pass_kind: architecture
---
```

## scope

Decision 2026-10-27 (agent-resolved; reviewer-2's review of Sprints 155-161, High 1) closes step 3 of
`docs/rust-port-plan-2026-10-24.md` with a measurement sprint: an exit's time by kind, the runtime word's hold and
wait times, and whether a guest WFE exits, each on a record as MEASUREMENT_RECORDED. Vocabulary 0.10's `metric` enum
has no value for any of them (`signals/0.10.json` 181-203: launch and frame times, `dispatch_ns`, `ns_per_access`,
`vm_compile_blocks` and P20's batch-run numbers). This sprint adds them by one typed proposal, P24, of the kind P19 and
P20 used (ENUM_EXTENDED, layer 2), modeled on `frame_time_ms_p50` and `frame_time_ms_p99`:

- `exit_ns_p50`, `exit_ns_p99`: one VM exit's time from the vCPU's exit to its next run, in nanoseconds, the median and
  the 99th percentile over a run; variant the exit kind (`hvc <k>`, or the exception class).
- `runtime_word_hold_ns_p50`, `runtime_word_hold_ns_p99`: how long the runtime word (`RUN_TABLE + RUNTIME_LOCK`,
  `vm_backend.py` 518) is held per taking.
- `runtime_word_wait_ns_p50`, `runtime_word_wait_ns_p99`: how long a taker waits for it.
- `wfe_exits`: the WFE or WFI traps a run took (ESR EC 0x01); 0 says a guest WFE does not exit.

Vocabulary 0.11 is `signals/0.10.json` with these seven values appended and its version, lock date and prior version
set; `signals/rust-current.json` names 0.11; the Rust types and substrate's schema are generated again; the harness
baselines name 0.11. `signals/current.json` keeps naming 0.9, the frozen Python's (Decision 2026-10-24, two pointers).
A comprehension affirmation follows the bump (hard rule 5).

## prerequisites

- 161 (closed)

## context_files

- `signals/0.10.json`, `signals/0.10-rationale.md`, `signals/proposals.json` (P19, P20)
- `crates/vocabulary_generator/`, `crates/events/`, `crates/vocabulary_samples/`; `harness/*.toml`
- `sprints/sprint-131-vocabulary-0-10.md` (the last bump's procedure)

## signal contract

### Emits

- The vocabulary samples run (`harness/vocabulary-samples.toml`, Sprint 126) fires MEASUREMENT_RECORDED with each of
  the seven new metric values through its Rust type, on a record that replays; SESSION_INIT names `vocab_version`
  0.11.

### Consumes

`signals/0.11.json`.

### Invariants

- No tag added, removed or changed but the `metric` enum; no record under `store/runs/` changes; every record replays.
- No Python file added or changed; `signals/current.json` unchanged.

## artifact contract

### Files created

- `signals/0.11.json`, `signals/0.11-rationale.md`; one samples record under `store/runs/`

### Files modified

- `signals/rust-current.json`, `signals/proposals.json` (P24), `crates/events/src/generated.rs`,
  `harness/schemas/harness-events.json`, `harness/*.toml` (vocabulary 0.11), `crates/vocabulary_samples/` (the new
  values sampled), `WORKING_AGREEMENT.md`

### Content assertions

- `signals/0.11.json` differs from `signals/0.10.json` in its version fields and the seven `metric` values alone.

### Command exit codes

- `target/release/vocabulary_generator --check` returns 0
- `uv run substrate run --file harness/vocabulary-samples.toml --root store/runs/vocabulary-samples-<ns> --strict`
  returns 0
- `scripts/gates.sh` returns 0

## observation contract

- **Expected values:** the samples record holds a MEASUREMENT_RECORDED for each new metric value, and replays.
- **Planted fault:** a sample with metric `exit_ns_p95` (not in 0.11): the Rust type refuses it by name, and substrate
  refuses the line.

## done criteria

Vocabulary 0.11 names the seven metrics step 3's measurements need, generated into the Rust types and substrate's
schema, ratified by P24.

## self-review checklist

- [x] Scope names one verifiable property: 0.11 adds the seven metric values, nothing else.
- [x] context_files name the last bump's procedure and the precedent proposals.
- [x] Emits: MEASUREMENT_RECORDED and SESSION_INIT on the samples record.
- [x] Artifact contract gradable by command and by a diff.
- [x] Observation contract: the samples and a planted value outside the enum.
- [x] One concept: the metric enum's extension.
