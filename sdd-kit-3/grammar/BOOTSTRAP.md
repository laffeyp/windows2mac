# BOOTSTRAP.md — the Vocabulary Session procedure

*The Vocabulary Session is Sprint 0 of any new SDD project. The agent walks through the steps below, producing `signals/0.1.json` (the locked vocabulary) and `signals/0.1-rationale.md` (the rationale doc). After this session, no implementation sprint dispatches until both files exist and the rationale doc carries the agent's sign-off (Step 12).*

*sdd-kit-3 change: in sdd-kit-2 each step ended with the Architect reviewing and saying "Layer N ready". Here each step ends with a **Self-review** block that carries the same checks. The agent runs them, fixes what fails, records the non-obvious choices as agent-resolved decisions (`AGENTS.md` "Resolve-and-record"), and moves on. The human is asked in chat only the user-only questions; the step that depends on the answer continues with the recommended answer marked provisional, and is revised if the answer differs.*

*Read `grammar/PRINCIPLES.md` before running this procedure. For deeper per-layer guidance, worked examples, and the empirical lessons from prior projects, see the predecessor kit's expanded grammar folder at `sdd-kit/grammar/` (the audit-trail-preserving original): `PER_LAYER.md`, `EXAMPLES.md`, `EMPIRICAL.md`. The two-file grammar here is the minimal version that supports running the procedure; the expanded versions are reference when needed and available.*

---

## Inputs

Before starting the session, the agent gathers:

- **A product spec.** What the system is, who it's for, why it exists. Operational language, not marketing. (A founding document serves.)
- **A technical spec** (if applicable). Architecture, components, data flows, runtime decisions, operator boundaries. (Existing code and its tests count as technical spec.)
- **A design canon** (if the project has UI). Component descriptions, voice canon, design tokens, copy.
- **Prior-project postmortems** (if applicable). Make The Models Talk's Round-2 postmortem is canonical; if the new project's domain overlaps with a prior project, that prior project's vocabulary is reference material.

If an input is thin, the agent does not invent entities or events to fill it. It strengthens the input from what the project already holds (the founding document, `## Decisions`, the code, the tests, conversation the human had with the agent), writes the strengthened text into the project's docs with a citation for each addition, and records the change as an agent-resolved decision. Only product intent that none of those sources contain is a user-only question.

---

## Outputs

At session close, two artifacts exist on disk:

- **`signals/0.1.json`** — the locked vocabulary. Eleven layers populated to the extent the docs support; gaps recorded as typed proposals in `signals/proposals.json` for future versions.
- **`signals/0.1-rationale.md`** — the rationale document. Per-layer section recording: what was considered, what was rejected, what failure modes were named, which prior-project postmortems informed which choice, which choices were agent-resolved and on what grounds. The dual-contract audit table appears here.

The agent signs off on both (Step 12). v0.1 is locked.

---

## The session structure

Eleven steps walking the layers in dependency order, plus a sign-off step.

The agent drives each step by reading the relevant docs, populating the relevant layer, and running the step's self-review. This is the one phase where natural language from the human is load-bearing: the human's stated intent is the product spec. Everything the human did not state is settled on the three grounds and recorded.

---

## Step 0 — Preflight

**Agent:**

1. Read `foundations/01-signal-driven-development.md`, `foundations/04-sdd-claude-design.md`, and `grammar/PRINCIPLES.md`. (If not already loaded.)
2. Optional reference (if the project's vocabulary is non-trivial and the predecessor kit is available): `sdd-kit/grammar/PER_LAYER.md` and `sdd-kit/grammar/EXAMPLES.md`. Skim during Steps 1–10 as appropriate.
3. Read the product spec, technical spec, design canon, prior-project postmortems.
4. Write a one-paragraph orientation entry to `BLACKBOARD.md ## Surfaced for review` and say the same in chat: "I have read [list of kit files]. I have read the project's inputs at the following paths: [list]. Beginning the Vocabulary Session."

**Self-review:**

1. The orientation entry names the actual paths read, not a generic list.
2. Every input in the Inputs section is either read or recorded as absent with the reason.
3. Proceed.

---

## Step 1 — Layer 0 (Ontology) — extract entities from the docs

**Agent:**

1. Walk through the docs. For each kind of thing the system tracks state for, propose an entity. Examples: from a checkout spec, propose `Cart`, `Order`, `Payment`, `Inventory`, `User`. From a game spec, propose `Player`, `Region`, `Item`, `Beat`.
2. For each entity: name (PascalCase), one-sentence definition drawn from the docs, source citation (which document, which section, which sentence).
3. Resolve per-entity problems on the grounds:
   - The docs don't name the entity but the code or the docs' logic requires it (`ENTITY_IMPLIED_BUT_UNNAMED`). Include it if a cited sentence or a code type depends on it; mark it `framing_inferred` in the rationale doc with the citation. If nothing cites it, leave it out and record a proposal in `signals/proposals.json`.
   - Cross-document polysemy: the same entity appears under different surface forms across docs (`ENTITY_MERGE_PROPOSED`). Merge when the definitions and the code types coincide; keep separate when they carry different state. Record the choice as an agent-resolved decision.
4. Write the entity list to `signals/0.1.json` under a Layer-0 / `ontology` section.

**Self-review:**

1. Under-extraction check: walk the code's public types and the docs' nouns once more. Agents under-extract more often than they over-extract. Every type in the canonical home registry maps to an entity or is recorded as a non-entity (a value object, a config) with a reason.
2. Over-extraction check: every entity has a citation and tracks state.
3. Every polysemy case has a recorded merge-or-split decision.

---

## Step 2 — Layer 1 (Lexical) — name the tags per entity

**Agent:**

1. For each entity from Layer 0, walk through the docs again. For each observable event involving that entity (each verb the system can do or that can happen to the entity), propose a tag.
2. Tag form: `{ENTITY_SNAKE_UPPER}_{VERB_PAST_PARTICIPLE_UPPER}`. Past tense. Irregular forms handled explicitly (see `BIRD_FLIT` example in EXAMPLES.md).
3. Recognize chain decomposition (see Audio Object's `REC_QUEUED → REC_FIRE → REC_START_NOW` in EXAMPLES). When a domain interaction produces multiple lifecycle events at different observability points, propose multiple tags.
4. For each tag: category (architectural boundary), stratum (event / ambient / summary / incident), one-sentence note describing what the tag means and when it fires.
5. Resolve on the grounds:
   - A Layer-0 entity that is load-bearing but produces zero tags. Check whether it is passive (no observable lifecycle). If passive, record it as such in the rationale doc. If not, the missing tags are an under-extraction; add them.
   - A single entity-verb pair that decomposes into more than five tags. Over-decomposition is likely. Merge until each remaining tag marks a distinct observability point, and record the merge.
6. Write tags to `signals/0.1.json` under `tags[]`.

**Self-review:**

1. Every tag name is past tense and follows the form; rename any that aren't.
2. Chains: each tag in a chain marks a point a consumer could act on differently. Collapse tags that don't.
3. Strata: `event` fires per occurrence; `summary` fires once per boundary with aggregates; `ambient` samples on a cadence; `incident` fires only when something needs attention. Re-check each tag against these definitions.

---

## Step 3 — Layer 2 (Payload) — type the fields per tag

**Agent:**

1. For each tag from Layer 1, walk the docs again. What data does the program need at this emission point? List candidate fields.
2. Per field: name (snake_case), type (string / int / float / bool / enum-with-values / entity-reference), required vs optional. Enum-typed fields list their values explicitly.
3. Apply the universal view-payload convention: every tag with `category: view` automatically gets `payload: [frame, visible, scene_id, layer]`. Add project-specific structural fields per-view-tag as the dual-contract pairings require (see `SCENE_RENDERED.visible_paths` example).
4. Foreign-key resolution: for each `_id` field, identify the Layer-0 entity it references. If no match: either add the entity to Layer 0 (with a citation, per Step 1's rule) or rename the field.
5. Resolve on the grounds:
   - A tag whose payload is entirely ambiguous (every field `requiredness: ambiguous`). Read the code path that will emit it, or the doc sentence that motivates it, and type the fields from that. If neither exists, the tag has no support; move it to `signals/proposals.json`.
   - A foreign-key resolution rate below 70%. Return to Step 1; entities are missing.
6. Write payloads to `signals/0.1.json` per tag.

**Self-review:**

1. Every field is typed and marked required or optional; no `ambiguous` remains.
2. Every enum lists its values.
3. Validator-extras posture decided (default: strict; the Trading System documentation-only-extras pattern only when the project has a stated reason). Recorded in the rationale doc and as an agent-resolved decision.
4. Semantic boundary check (from the sdd-kit-2 `example/` KIT_DIARY suggestion): for each field, is its meaning obvious from name and type alone? If not (e.g., is a count recursive or not?), write the boundary into the tag's note now.

---

## Step 4 — Layer 3 (Session) — frame the event streams

**Agent:**

1. Walk the existing tags. Identify boundary candidates (tags whose verb roots are in `{start, init, begin, open, end, close, stop, resume, reset}`). For each, ask: does this open or close a stream of other tags?
2. Per session stratum: name, boundary-open tag, boundary-close tag (or "inferred from timeout / app-exit"), re-entry tag (if applicable), list of framed tags, persistent attributes (from the boundary-open tag's payload).
3. Recognize nesting: a `game_session` may contain many `gameplay_sessions`. Declare the parent-child relationship explicitly.
4. If the docs do not name "session" as a concept and you must construct the abstraction from boundary tags, mark `framing_inferred` in the rationale doc.
5. Resolve on the grounds:
   - No boundary-open candidates. Add a `SESSION_INIT`-pattern tag (TECHNIQUES.md #9); every capture needs a baseline. Record it.
   - A boundary-open candidate frames no tags. It is not a boundary; reclassify it.
6. Write session strata to `signals/0.1.json` under a `sessions[]` array.

**Self-review:**

1. Every session has an open tag, a close tag or stated inference, and a list of framed tags.
2. Nesting is explicit.
3. Persistent attributes appear in the open tag's payload and nowhere else redundantly.

---

## Step 5 — Layer 4 (Temporal) — name timing invariants

**Agent:**

1. Walk the docs. Find sentences containing temporal keywords: `within`, `after`, `before`, `every`, `Hz`, `cooldown`, `timeout`, `eventually`, `never`, `at most`, `at least`.
2. For each candidate: kind (window / cadence / cooldown / pairing / forbidden_after / eventually_must), tag(s) involved, duration (if any), payload constraint (if any), source citation.
3. Cross-check ambient-stratum tags from Layer 1: every ambient tag MUST have a cadence invariant.
4. Resolve on the grounds:
   - An ambient tag with no cadence. Set one from the code's actual cadence or the doc's stated rate; if neither exists, the tag is not ambient — reclassify it.
   - Conflicting invariants. The one with the more specific citation wins; record the conflict and the choice.
   - Vague durations ("within a few seconds"). Pick a number from the code or from practice for that operation, cite the source, and record it as an agent-resolved decision.
5. Write invariants to `signals/0.1.json` under a `temporal_invariants[]` array.

**Self-review:**

1. Every ambient tag has a cadence.
2. Every duration is a number with a unit.
3. Walk the docs once more for timing stated in prose the keyword filter missed.

---

## Step 6 — Layer 5 (State-Transition) — graph the allowed orderings

**Agent:**

1. Walk the docs for ordering language: `after`, `before`, `precedes`, `follows`, `requires`, `terminal`, `cannot follow`, `must precede`, `next`.
2. For each candidate: kind (allowed_set / forbidden_after / forced_next / terminal / pairing_ordering), from_tag, to_tags_allowed (or to_tags_forbidden), payload_match (if any).
3. Build the transition graph. Check: cycles outside named loop constructs (usually a bug), unreachable tags (one or both endpoints not appearing in any allowed edge), sessions with no reachable terminal.
4. Cross-check with Layer 3 (session boundaries) and Layer 4 (window invariants imply ordered transitions).
5. Resolve on the grounds: an unexpected cycle is either a named loop (name it) or a wrong edge (remove it); an unreachable tag gets an edge from the code path that emits it or moves to proposals; a session with no terminal gets its close tag marked terminal; conflicting rules resolve by the more specific citation. Record each.
6. Write transition rules to `signals/0.1.json` under a `state_transitions[]` array.

**Self-review:**

1. Terminals are correct: each session's close tag is terminal.
2. Allowed-sets are not over-restrictive: every edge the code can take is allowed.
3. Sparseness is fine: not every tag pair needs a rule. Most Layer-5 rules cover the critical paths (pairing, terminal, forbidden_after); the rest is implicit.

---

## Step 7 — Layer 6 (Runtime / Operator) — name the operators

**Agent:**

1. Read the technical spec (and the code). Identify named subsystems via section headings, class/struct/type definitions, architecture tables.
2. Per operator: name, responsibilities (one paragraph), which tags it emits (cross-reference to Layer 1), which tags it consumes, model route (if LLM-based), fallback chain (if applicable), performance constraints.
3. Recognize dual classification: an entity at Layer 0 may also be an operator at Layer 6 (Katybird's `Bird` is both). Declare both classifications.
4. Resolve on the grounds: more operators than tags means over-decomposition — merge operators that share a boundary; a tag with no emitting operator gets one assigned from the code path, or moves to proposals.
5. Write operators to `signals/0.1.json` under an `operators[]` array.

**Self-review:**

1. Operator boundaries match the technical spec's architecture and the code's module boundaries.
2. Every dual-classification case is declared and explained in the rationale doc.
3. Runtime grammar is present, not only domain grammar (PRINCIPLES.md: a mock or fallback path must be able to say it is one).

---

## Step 8 — Layer 7 (Evidence) — declare payload constraints

**Agent:**

1. Read the design canon's voice/copy guides (if applicable). Extract `TonalRule` records — what tonal constraints apply to user-facing strings? Bind them to relevant payload fields (those of type `string` likely to contain user-visible content).
2. Walk the product and technical specs for numeric ranges and cross-reference assertions. Per constraint: kind, target tag, target field, constraint spec, source citation, enforceability (`mechanical | llm_check | manual_review`).
3. Apply stratum-specific evidence requirements: incident-stratum tags need diagnostic-required Evidence; summary-stratum tags need outcome-required Evidence; view-category tags need structural Evidence (covered by Layer 2's universal view payload).
4. Resolve on the grounds: an incident tag with no diagnostic constraint gets one (what field says why it fired); a tonal rule with no bound field is bound or dropped; a cross-reference to an unresolved foreign key sends you back to Step 3.
5. Write constraints to `signals/0.1.json` under an `evidence_constraints[]` array.

**Self-review:**

1. Enforceability is assigned per constraint. Prefer `mechanical` wherever a check can be written; `manual_review` only for what a human must perceive.
2. Every incident tag has a diagnostic constraint.

---

## Step 9 — Dual-Contract Audit

**Agent:**

1. Walk every behavior-category tag from Layer 1. For each, pair it with one of:
   - A view-category tag (e.g., `WORLD_PATH_UNLOCKED` ↔ `SCENE_RENDERED.visible_paths`).
   - A structural payload field on an existing view tag (e.g., `APOLOGY_LAYER_ADDED` ↔ `HUD_RENDERED.apology_layers_present`).
   - A typed gap-proposal naming what the artifact-side substrate should be (e.g., `DUAL_CONTRACT_PAIRING_GAP: KATY_GLIMPSED needs dedicated view tag because narrative weight is high`).
2. Write the audit table to the rationale doc as a section with three columns: behavior tag, view-side counterpart (tag or field), notes.
3. For each gap: decide on the grounds — add a dedicated view tag, add a structural payload field to an existing view tag, or accept the gap (the behavior tag has no view-side counterpart; rare; write why). Record each decision. If new view tags or fields were added, re-run the audit.

**Self-review:**

1. Every behavior tag has a row.
2. Every accepted gap has a written reason.
3. For a project with no UI yet: pair behavior tags with the artifact that proves them (a stored record, a file, a report) and note that view pairings are deferred to the first UI sprint.

---

## Step 10 — Synthesize Layers 8–10

These layers are mostly inherited; the agent emits them deterministically.

**Agent:**

1. Layer 8 (Report): bind the project's tag set into the Signal Report template per `templates/SIGNAL_REPORT.md`. Behavior tags → OBSERVED/EXPECTED; view tags → RENDERED; incident tags → NOTES; proposals → GRAMMAR_DRIFT. Write the binding to `signals/0.1.json` under a `report_binding` object.
2. Layer 9 (Version): emit lock metadata. `{version_id: "0.1", locked_at: $NOW, locked_by: "agent via Vocabulary Session, $DATE, per sdd-kit-3 resolve-and-record", prior_version: null, migration_proposals: []}`.
3. Layer 10 (Grammar-Growth): reference the proposal taxonomy version and declare any project-specific overrides (e.g., validator-extras posture). Default: `{grammar_growth_taxonomy_version: "0.1", project_overrides: []}`.

**Self-review:**

1. Version metadata parses and names its lock date.
2. Every override named here also appears in the rationale doc.

---

## Step 11 — Write the rationale doc

**Agent:**

1. Compose `signals/0.1-rationale.md`. Sections, in order:
   - **Intent and scope.** What this vocabulary is for; what project it belongs to; what its bounds are.
   - **Per-layer decisions.** One section per Layer 0–7. For each layer: what was populated, what was considered and rejected, what gaps were left as proposals, which source documents informed which choices, which prior-project postmortems were consulted.
   - **Dual-contract audit table.** The table produced in Step 9, with rationale for each gap-resolution choice.
   - **Project-specific overrides.** Validator-extras posture, view-payload convention overrides (if any), category-set deviations from common patterns.
   - **Agent-resolved decisions.** Every choice the session settled on the grounds, with the grounds, mirrored from `## Decisions`.
   - **Open proposals for v0.2.** Typed proposals (per the eight evolution kinds) that the docs forced but v0.1 did not adopt.
   - **Sign-off.** Written in Step 12.

**Self-review:**

1. Defensibility test: if someone six months from now asks "why is `BIRD_FLIT` separate from `BIRD_HOPPED`?", the rationale answers it. Pick three tags at random and check that the rationale explains each one's existence and boundaries.

---

## Step 12 — Sign-off

**Agent:**

1. Verify both files exist on disk.
2. Verify `signals/0.1.json` parses as JSON (run the parse; record the exit code).
3. Confirm every step's self-review passed.
4. Write the sign-off at the bottom of the rationale doc: date, agent identifier, "self-review checks for Steps 0–11 passed", the count of agent-resolved decisions, and the count of open proposals.
5. Write the close entry to BLACKBOARD `## Built`: "Sprint 0 (Vocabulary Session) closed. signals/0.1.json + signals/0.1-rationale.md authored. v0.1 locked. N agent-resolved decisions recorded. Ready for Sprint 1."
6. Tell the human in chat, in a short paragraph: tag count, category count, the agent-resolved decisions most likely to matter to them, and any user-only question still open. Their answer, if it changes something, arrives as a v0.2 proposal like any other change.
7. The next sprint is Sprint 1 of regular implementation work.

---

## Anti-patterns to watch for

The empirical baseline and the soundfield experience surface these patterns. The agent watches for them throughout the session.

- **Fabricating to avoid a gap.** When the docs underspecify a layer, the temptation is to invent. Don't. Settle it from what the project holds (code, tests, Decisions, the human's stated intent) with a citation, or leave it as a proposal. An uncited entity or tag is a fabrication, whether or not a halt exists to catch it.
- **Deciding silently.** Resolve-and-record replaces halting; it does not replace recording. A choice that shaped the vocabulary and appears nowhere in `## Decisions` or the rationale doc is the failure mode halt-and-articulate existed to prevent.
- **Skipping the rationale doc.** Without it, the vocabulary is undefendable six months later. Audio Object proves the methodology can work without one only because the project IS where the methodology emerged; every other project must produce one.
- **Compressing the session for speed.** The Vocabulary Session is the founding act. A defendable v0.1 saves dozens of sprints of downstream rework; a vague one costs them.
- **Rubber-stamp self-review.** The self-review blocks are the Architect's old checks, now run by the agent. Run each check against the file on disk, not against memory of what was written.
- **Treating v0.1 as a draft.** v0.1 is locked. Changes go through the proposal taxonomy and bump the version (v0.2). "Just edit the JSON" is the silent-drift failure mode that breaks every project's vocabulary stability.
- **Vocabulary materialized after implementation starts.** The soundfield project did this; the 60 sprints prior to vocabulary materialization carried Logger-instead-of-signal drift that took 6 cleanup sprints to repair. Vocabulary is Sprint 0; no implementation sprint dispatches until the lock is signed. A project that already has code (because it predates adopting the kit) runs the session next and retrofits emit calls in the first implementation sprints.

---

## Estimated time

For a project with moderate-complexity specs (~50-100 pages total across product + technical + design), sdd-kit-2 estimated 2.5–4 hours of Architect-and-agent collaborative work. Without per-layer review waits, the agent's time is dominated by reading and the self-review passes; the human's time drops to answering user-only questions. The output size is unchanged: `signals/0.1.json` (50-100 tags, 9-15 categories, 4 strata, 5-15 temporal invariants, 10-30 state-transition rules, 5-12 operators, 10-25 evidence constraints) + a 2,000-5,000-word rationale doc. Smaller projects produce proportionally smaller outputs; the procedure is the same.

The session is one of the most leveraged work blocks in the project's lifetime. Treat it accordingly.

---

*BOOTSTRAP.md (sdd-kit-3). The 12-step Vocabulary Session procedure. Sprint 0 of any new SDD project. Each step ends in an agent self-review instead of an Architect sign-off. Outputs: signals/0.1.json + signals/0.1-rationale.md. No implementation sprint dispatches until the lock is signed.*
