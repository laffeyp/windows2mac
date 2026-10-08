# Sprint {NNN} — {sprint title}

*A sprint card is a typed Command Message — the canonical handoff between the Architect's plain-English direction and the Agent's executable work. Copy this template to `sprints/sprint-NNN-{slug}.md` (NNN zero-padded; slug kebab-case). The Agent reads this card to know what to do.*

*Sprint sweet spot per AGENTS.md hard rule 6: ≤2 files / one concept. If a sprint requires more, split it into a chain of smaller sprints.*

---

## Frontmatter (REQUIRED)

```yaml
---
id: NNN
status: pending          # pending → active → closed (or closed-partial)
phase: 1                 # integer 0+
pass_kind: architecture  # architecture | functional | docs | bridge | observation
---
```

`pass_kind` options:

- `architecture` — establishes contracts, types, module boundaries. Gets the deepest self-review; errors here propagate to every later sprint.
- `functional` — fills in business logic against established contracts.
- `docs` — authors or revises documentation.
- `bridge` — integrates an external SDK or API. Requires a bridge-mapping in `WORKING_AGREEMENT.md` before authoring; if missing, the Agent writes it first (TECHNIQUES.md #46 — workers consistently misuse external SDK APIs without explicit bridge mapping).
- `observation` — sprint produces no new code but runs the project's observation contract to verify behavior of prior sprints.

(Frontmatter is documentation, not a code-enforced gate. The Agent's self-review against the checklist at the bottom of this card enforces it.)

---

## scope

*One paragraph naming what this sprint produces. Concrete and bounded. The Agent re-states it as `scope_confirmation` in the Signal Report and in one line in chat before executing; if the scope is fuzzy, the confirmation will be fuzzy — rewrite the scope until it names one verifiable property.*

*Good: "Author `src/cradle/operators/price_state_encoder.py` defining `PriceStateEncoder` per ADR-006. Module exports `encode_price_state(symbol, bar) -> PriceState`. File parses; `pytest tests/cradle/test_price_state_encoder.py` passes."*

*Bad: "Set up the encoder."*

{your one paragraph}

---

## prerequisites

*Bullet list of sprint ids that must close before this sprint dispatches. Discipline, not code-enforced.*

- {previous sprint id, or "none"}

---

## context_files

*Files the Agent must read to execute. The Agent reads these via its `Read` tool before authoring. Always include the kit's foundations + the project's vocabulary + the project's working agreement. For UI-touching sprints, MUST include the design canon (hard rule 8). For sprints that modify existing code, include the current source files so the Agent can preserve accreted detail (hard rule 7 — canonical home registry).*

- `sdd-kit/AGENTS.md`
- `signals/0.1.json` (the project's locked vocabulary)
- `BLACKBOARD.md` (current state, especially `## Decisions` for project scope)
- `WORKING_AGREEMENT.md` (project-specific overrides)
- {project-specific spec files, e.g., `context/technical_spec.md §6`}
- {existing source files this sprint will modify, e.g., `src/cradle/types.py`}
- *(If UI-touching:)* `context/design_handoff_bundle/components.md`, `context/design_handoff_bundle/copy.md`, `context/design_handoff_bundle/voice.md`, `context/design_handoff_bundle/tokens/tokens.json`

---

## signal contract

### Emits

*Every signal tag the sprint's code will fire during normal execution. MUST reference real tags from `signals/0.1.json` — never invent inline (per AGENTS.md hard rule 2). If a needed tag doesn't exist, propose it, ratify on the grounds, bump the vocabulary version, record, and then list it here.*

- `{TAG_NAME_1}` ({payload field 1}, {payload field 2})
- `{TAG_NAME_2}` (...)

### Consumes

*Bullet list of files / sections the Agent reads. Mirrors `context_files`.*

- ...

### Invariants

*Bullet list of structural invariants the sprint must preserve. Examples: "no force-unwrap operators introduced"; "no out-of-vocabulary tags emitted"; "the locked vocabulary file is not modified".*

- {invariant 1}

---

## artifact contract

### Files created

*Every file the sprint authors. Concrete paths (e.g., `src/cradle/operators/price_state_encoder.py`).*

- `{path/to/file.ext}`

### Files modified

*Every existing file the sprint modifies. The Agent uses Edit (SEARCH/REPLACE-style) on these — never full-file rewrite (per AGENTS.md hard rule against silently dropping accreted detail).*

- `{path/to/existing/file.ext}`

### Content assertions

*Concrete, verifiable per-file claims. Examples: "file contains `def encode_price_state(symbol: str, bar: Bar) -> PriceState`"; "JSON validates as JSON Schema Draft-07"; "file ≥ 200 bytes". If a grep or a verifier can't check it in under a minute, it's not gradable — rephrase.*

- ...

### Command exit codes

*Commands the Agent runs at grade time to verify the artifact (the project's CI may run them too). The Agent reports each exit code and output tail in the Signal Report. Each command's expected exit code is named.*

- `{command, e.g., "swift build"}` returns 0
- `{command, e.g., "pytest tests/cradle/test_price_state_encoder.py"}` returns 0

---

## observation contract (REQUIRED for `pass_kind: functional` or `observation` sprints; OPTIONAL for `architecture` / `docs`)

*Per AGENTS.md hard rule 9 and the soundfield round-23 lesson: behavior-touching sprints need an observation contract; content assertions don't cover product behavior.*

*Enumerate: UI driving steps (if applicable); expected log substrings; expected screenshot regions; expected runtime signals in the live trace.*

### UI driving steps

- {e.g., `ui_tap("chat-input")`, `ui_type("hello")`, `ui_tap("chat-send")`}
- *(omit if no UI)*

### Expected log substrings

*Substrings the Agent (or a tail-running script) verifies appear in the live log after running the UI driving steps.*

- {e.g., `"AVAudioWeatherEngine started"` count == 1}
- {e.g., `"Session auto-started successfully"` count == 1}

### Expected runtime signals

*Per-tag list of signals the project's signal capture should contain after running. Cross-reference to `signals/0.1.json`.*

- `{TAG_NAME_1}` with payload `{...}`

### Expected screenshot / visual state

*If UI-touching, name the visible elements that must be present in a post-driving screenshot.*

- {e.g., "field strip background == bone color (#FCFAF7), ΔE < 0.5"}

---

## done criteria

*One or two sentences. Human-readable summary of what "done" means. The dual contract (signal + artifact + observation) is the verification; this section is the human-readable summary.*

{your one-sentence summary}

---

## notes

*Free-form notes from the Architect or Agent at composition time. Caveats; edge cases; references to specific patterns in TECHNIQUES.md or examples in `example/`; surfaces the agent expects to discover during execution. Not graded; for the next reader's benefit.*

{any notes}

---

## self-review checklist

*(Every card. sdd-kit-2 called this the plan-mode review checklist and had the Architect run it before saying "go"; in sdd-kit-3 the Agent runs it and proceeds.)*

The Agent verifies these items before executing. A failed item is fixed in the card, not waited on:

- [ ] Scope is concrete and bounded (one paragraph; one verifiable property of the artifact).
- [ ] `context_files` covers everything the Agent needs to read; design canon included if UI-touching; SDK bridge mappings referenced if LLM-integration.
- [ ] Signal contract's `Emits` list references only tags in `signals/0.1.json`.
- [ ] Artifact contract is gradable: content assertions are concrete, command exit codes are runnable.
- [ ] If `pass_kind: functional` or `observation`: observation contract is present.
- [ ] Sprint sweet spot honored: ≤2 files modified, one concept. (If not, split into a chain.)

---

*SPRINT_CARD.md (sdd-kit-3) — template. Copy to `sprints/sprint-NNN-{slug}.md` and fill. The Agent self-reviews and executes; the Architect may read any card at any time. Dual contract (signal + artifact) is required; observation contract is required for behavior-touching sprints. ≤2 files / one concept.*
