# Sprint 000 — Vocabulary Session

```yaml
---
id: 000
status: closed
phase: 0
pass_kind: architecture
---
```

## scope

Run the Vocabulary Session per `sdd-kit-3/grammar/BOOTSTRAP.md` Steps 0-12 and produce `signals/0.1.json` and `signals/0.1-rationale.md` for w2m as specified by `windows_to_macos_research_direction_v0_2.md`. The vocabulary covers six operator families taken from v0.2: evidence (documented, observed, hypothesis; §11.1), contract (proposed, accepted, revised, with dependency invalidation; §11.2-11.3, §14), probe and model check (reference and destination profiles; §11.1, §9), code validity (target discovered, translation published, invalidated; AOT vs on-demand stage; §5.1, §6), discrepancy and repair (minimized report, mismatch class from §3, implementation repair vs contract revision as distinct tags; §11.3), and product lifecycle (acquire, session, run, close, relaunch, update, remove; §12). Done when the JSON parses and the rationale carries the Step 12 sign-off.

## prerequisites

- none

## context_files

- `sdd-kit-3/AGENTS.md`
- `sdd-kit-3/grammar/PRINCIPLES.md`
- `sdd-kit-3/grammar/BOOTSTRAP.md`
- `sdd-kit-3/templates/VOCABULARY.json`
- `windows_to_macos_research_direction_v0_2.md` (whole document; §3, §5.1, §6, §8, §11, §12, §14 most directly)
- `docs/review-2026-10-03-research-direction-v0_1.md` (input to v0.2; superseded where v0.2 differs)
- `BLACKBOARD.md` `## Decisions`
- `WORKING_AGREEMENT.md`
- substrate `docs/api.md` (Event, Subscription, assert_event: the transport the vocabulary rides on)

## signal contract

### Emits

None at runtime. Content sprint; `signal_trace` is narrated.

### Invariants

- Every tag has `name`, `category`, `stratum`, `payload`, `note`.
- Every category a tag names exists in `categories`.
- Every runtime-grammar operator (Layer 6) names the profile it ran on: Windows reference profile, VM backend, native-process backend, model, or service fixture (PRINCIPLES.md: a mock or fallback path must be able to say it is one; v0.2 §12.1 every result states its profile).
- Implementation repair and contract revision are separate tags; no tag lets a candidate change its own acceptance condition (v0.2 §11.3).
- Every incident tag has a diagnostic evidence constraint (BOOTSTRAP Step 8).

## artifact contract

### Files created

- `signals/0.1.json`
- `signals/0.1-rationale.md`
- `signals/proposals.json`

### Content assertions

- `signals/0.1.json` contains a tag with `SESSION_INIT` pattern whose payload includes `vocab_version`, `profile`, and `contract_version`.
- An evidence tag's payload includes `evidence_category` with enum values `documented`, `observed`, `hypothesis`.
- `signals/0.1.json` contains a discrepancy incident tag whose payload includes `contract_id`, `mismatch_class`, and `minimized_program_sha256`.
- `mismatch_class` is an enum whose values are the six relations in research direction §3 (exact, state relation, allowed outcome set, partial order, numerical envelope, progress obligation).
- `signals/0.1-rationale.md` contains a section headed `Dual-contract audit` and a section headed `Sign-off`.

### Command exit codes

- `python3 -c "import json; json.load(open('signals/0.1.json'))"` returns 0
- `python3 -c "import json; json.load(open('signals/proposals.json'))"` returns 0

## observation contract

Not applicable: content-only architecture sprint.

## done criteria

v0.1 is locked with a signed rationale; Phase 1 cards can name only tags that exist in it.

## notes

The Windows reference machine cannot import a Python emitter, so the session decides the reference-side transport (probe writes JSONL that a substrate Producer validates against the vocabulary on ingest). Record that as Layer 6 and as an agent-resolved decision. v0.2 fixes no game, backend, or graphics profile, so Layer 0 names `ApplicationProfile`, `ReferenceProfile`, `DestinationProfile`, and `Backend` as entities with enum-valued kinds, not a particular title or backend. The first-game answer (user-only Q1) adds profile instances later without a vocabulary bump.

Keep v0.2 §11.4 in view: the substrate record's total order is the order of orchestration work, not guest execution order. Guest-trace tags carry their own partial-order fields (thread id, per-thread sequence, happens-before references) rather than leaning on substrate `seq`.

## self-review checklist

- [x] Scope is concrete and bounded: two locked files plus proposals, one gate.
- [x] `context_files` covers the inputs BOOTSTRAP Step 0 names.
- [x] Signal contract emits nothing; consistent with a content sprint.
- [x] Artifact contract is gradable by `json.load` and grep.
- [x] Observation contract not required (`architecture`, no runtime behavior).
- [x] Three files, one concept (the vocabulary lock); the third is the proposals file BOOTSTRAP requires, accepted as part of the same artifact.
