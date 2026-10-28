# Sprint 169 — The records check in Rust

```yaml
---
id: 169
status: active
phase: 6
pass_kind: functional
---
```

## scope

Decision 2026-10-28 (P22 and P23 need the records check in Rust first): `scripts/check_records.py` (Sprint 109, 244
lines, frozen Python) checks every record against the vocabulary's rules, and P22 changes T4, which it holds in code
(`_t4`, 130-138), and it reads `signals/current.json` (0.9). The plan's rule: a judge moves to Rust in the sprint whose
code it judges, and vocabulary 0.12 (Sprint 170) is that code.

`target/release/check_records` (a new binary in `crates/w2m_vm`, beside `case_runs` and `signal_report`), the
Python's checks rule for rule:

- the reader: a record's frames in sequence order without `substrate.*` kinds (`w2m_vm::ledger::frames`, which
  `signal_report` copies; substrate's `read_record`);
- T1 (first SESSION_INIT), T2 (last SESSION_COMPLETED), T3 (each EXECUTION_STARTED closed once, within
  `timeout_seconds` plus `kill_grace_seconds` by `t`; unchecked without a timeout), T4 (each EXECUTION_COMPLETED
  followed by one TRACE_INGESTED or TRACE_REJECTED of its `trace_sha256` before SESSION_COMPLETED), T7 (CONTRACT_REVISED
  followed by DEPENDENTS_INVALIDATED with its `contract_id` and `to_version`);
- S1-S13 from `state_transitions` (not `GUEST_*`), by kind: `pairing_ordering` and `allowed_set`, `forbidden_after`,
  `forced_next`, matched on the plain fields of `payload_match` and its `field=value` filters, prose checked for
  order alone and counted unchecked;
- E1 (every sha256-typed field of the form `sha256:` and 64 lowercase hex digits, and not the zero placeholder), E6 (a
  discrepancy comparison names its `first_divergence`);
- the known violations (`KNOWN`, with the commit time that fixed each), `--since`, `--show`, and its printed lines.

The vocabulary: the three rule sections (`temporal_invariants`, `state_transitions`, `evidence_constraints`) are equal
in 0.9 and 0.11 (compared 2026-10-28), so the Rust reads `signals/rust-current.json`.

## prerequisites

- 168 (closed)

## context_files

- `scripts/check_records.py` (read in full 2026-10-28); `crates/w2m_vm/src/ledger.rs` 17-62; `signals/0.11.json`'s
  rule sections

## signal contract

### Emits

None: the check writes no record.

### Invariants

- No Python file added or changed; the Python check stays in `--all`'s gates until the Rust one is held equal there.

## artifact contract

### Files created

- `crates/w2m_vm/src/bin/check_records.rs`

### Files modified

- `scripts/gates.sh` (the Rust check beside the Python one), `WORKING_AGREEMENT.md`

### Command exit codes

- `target/release/check_records --since 2026-10-17 --show 0` returns 0; `scripts/gates.sh` returns 0

## observation contract

- **Expected values:** over every record in `store/runs/`, the Rust check's printed lines equal the Python's
  (`check_records.py --since 2026-10-17 --show 3`), byte for byte: per rule applied, violations and unchecked; the
  totals (38,601 records and 2,017 violations at Sprint 167's gates, plus the records since); 0 held; the known lines.
- **Planted faults:** (1) a record copied outside `store/runs/` with its SESSION_COMPLETED's line removed: both checks
  name T2 for it; (2) T4's ingest test reversed in the Rust (`!= 1` as `== 1`): the comparison with the Python's lines
  names T4's count.

## done criteria

The Rust check gives the Python's verdicts on every record, so the next vocabulary can change a rule in Rust.

## self-review checklist

- [x] Scope names one verifiable property: the Python's verdicts from Rust.
- [x] context_files cite the Python ported and the reader reused.
- [x] Emits: none.
- [x] Artifact contract gradable by command.
- [x] Observation contract: byte-equal output on every record, and two plants.
- [x] One concept: the records check.
