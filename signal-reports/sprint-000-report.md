# SIGNAL_REPORT — Sprint 000

## 1. Observed

### scope_confirmation

> Locked w2m's vocabulary v0.1 from `windows_to_macos_research_direction_v0_2.md`: six operator families (evidence, contract, probe and model, code validity, discrepancy and repair, product lifecycle) plus the guest-trace plane, with a signed rationale and a proposals file.

### work_performed

- Read substrate `kernel/sequencer.py` and `kernel/topology.py` to fix the harness-plane transport (event kind = Struct class name).
- Authored `signals/0.1.json`: 22 entities, 12 categories, 60 tags, 34 enums, 5 session strata, 7 temporal invariants, 14 transition rules, 16 operators, 13 evidence constraints.
- Authored `signals/proposals.json` (P1-P6 open, P7 rejected) and `signals/0.1-rationale.md` (per-layer decisions, dual-contract audit, ten agent-resolved decisions, sign-off).
- Authored `scripts/check_vocab.py` and registered it as the vocabulary gate in `WORKING_AGREEMENT.md`.

### signal_trace

Content sprint; no runtime emission.

### signal_trace_tags

- (none)

## 2. Expected

> The card declared no `Emits`. Nothing was expected to fire.

## 3. Delta

### dual_contract_self_grade

**signal contract:** vacuous (content sprint). *pass*

**artifact contract:**

- `signals/0.1.json` — exists (57,739 bytes); `json.load` exit 0 [pass]
- `signals/proposals.json` — exists (4,040 bytes); `json.load` exit 0 [pass]
- `signals/0.1-rationale.md` — exists; contains `## Dual-contract audit` and `## Sign-off` [pass]
- SESSION_INIT payload includes vocab_version, profile, contract_version [pass]
- DISCREPANCY_REPORTED is incident with contract_id, mismatch_class, minimized_program_sha256 [pass]
- mismatch_class enum equals the six v0.2 §3 relations [pass]
- evidence_category enum is documented / observed / hypothesis [pass]
- `python3 scripts/check_vocab.py` exit 0: "60 tags, 12 categories, 99 foreign keys (99 resolved), 16 operators" [pass]

**overall:** pass

### blackboard_append

Written to `BLACKBOARD.md` `## Built`, `## Sprint tail`, `## Decisions` (vocabulary decisions; one citation correction), `## Surfaced for review` (Step 0 orientation), `## Drift watchlist` (APPLICATION_LAUNCHED view pairing).

## 4. Hypothesis

### rubber_duck_observations

See `BLACKBOARD.md` `## Sprint tail`, Sprint 000: four observations — enum count in the rationale (resolved-here), gate script outside the card's file list (resolved-here), APPLICATION_LAUNCHED view gap (deferred, P2), wrong addendum citation in a Decision (recorded).

### status_and_blockers

> status: complete

### artifact_payloads

Files on disk, not repeated here: `signals/0.1.json`, `signals/proposals.json`, `signals/0.1-rationale.md`, `scripts/check_vocab.py`.
