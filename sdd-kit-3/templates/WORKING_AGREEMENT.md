# WORKING_AGREEMENT.md — {Project Name}

*Per-project overrides and additions on top of `sdd-kit/AGENTS.md`. The Agent reads AGENTS.md first (the methodology) and then this file (the project specifics). When the two conflict, AGENTS.md wins — this file augments, it doesn't override the methodology's hard rules. Copy this template to project root as `WORKING_AGREEMENT.md`; do not edit this template.*

---

## Project identity

- **Project name:** {fill}
- **Project type:** {e.g., iOS app, Python backend, CLI tool, Swift package, browser extension}
- **Primary language(s):** {fill}
- **Primary build commands:** {e.g., `swift build && swift test`; `pytest`; `npm run build && npm test`; `cargo build && cargo test`}
- **Adopted SDD kit version:** {fill — e.g., `sdd-kit v0.1`}

---

## Project scope (verbatim from BLACKBOARD ## Decisions)

> {restate the first BLACKBOARD ## Decisions entry here; the Agent reads this to ground its understanding of the project}

---

## Canonical home registry

*Per AGENTS.md hard rule 7. Name which file owns which public type. The Agent consults this before authoring; without it, multi-sprint sequences thrash by silently re-declaring types across files.*

| Type / module | Canonical home |
|---|---|
| `{TypeName1}` | `{path/to/file.ext}` |
| `{TypeName2}` | `{path/to/file.ext}` |
| ... | ... |

Add rows as new types stabilize. When a sprint raises a "where does this type live" question, the Agent settles it on the grounds (the module that already owns the type's closest collaborators wins), adds the row here, and records an agent-resolved decision.

---

## External SDK bridge mappings

*Per TECHNIQUES.md #46. When the project uses an external SDK (an LLM library, an audio framework, an HTTP client, etc.), the Agent reverse-engineers the SDK's actual public API surface — from installed source or official documentation, never from memory — and documents it here BEFORE any sprint authoring code that imports the SDK. Later sprints read this to honor the actual API rather than guessing from spec prose.*

*Examples from the soundfield project — what would have prevented many sprints of guess-and-iterate:*

### {SDK name 1, e.g., "MLX Swift LM"}

- **Package URL:** `https://github.com/ml-explore/mlx-swift-lm`
- **Required imports for usage:** `MLXLLM`, `MLXLMCommon`, `MLXHuggingFace`, `HuggingFace`, `Tokenizers`
- **Critical API surface (verbatim from upstream source):**
  - `GenerateParameters(maxTokens: Int?, temperature: Float = 0.6, topP: Float = 1.0, topK: Int = 0, ...)` — note `Float` not `Double`; no `seed:` parameter.
  - `ModelContainer.prepare(input:) async throws → LMInput` — required before `.generate()`.
  - `ModelContainer.generate(input:parameters:) async throws → AsyncStream<Generation>` — `Generation` cases: `.chunk(String)`, `.info(GenerateCompletionInfo)`, `.toolCall(ToolCall)`.
- **Bridge mapping (our type ↔ their type):** {table per project; e.g., our `ModelInput.prompt` → their `UserInput(prompt: String, ...)`.}
- **Substrate gaps from spec (what the SDK doesn't provide):** {e.g., MLX high-level API doesn't expose per-token top-K or entropy; spec'd events for these are dropped in v0.}

### {SDK name 2, e.g., "AVFoundation / AVAudioEngine"}

- **Critical API gotchas:**
  - `engine.attach(engine.mainMixerNode)` CRASHES — it's auto-attached.
  - `AVAudioUnitEQ.bands[0].bypass` defaults to TRUE → silent filter trap.
  - `AVAudioSourceNode` render block runs on real-time audio thread → MUST NOT capture `self` (actor), MUST NOT allocate, MUST NOT acquire blocking locks.
- ...

Add a section per external SDK as the project encounters it. The first sprint that imports an SDK without a bridge mapping in this file writes the mapping before any other authoring (AGENTS.md "What replaces each sdd-kit-2 halt").

---

## Vocabulary discipline overrides

*Per `sdd-kit/grammar/PRINCIPLES.md`, projects may declare overrides on the default vocabulary discipline.*

- **Validator-extras posture:** `strict` (default; payload fields not in schema are rejected) | `documentation-only` (Trading System pattern; extras appear in emitted payloads but the validator does not enforce them).
- **View-payload-universal convention:** Apply the four-field convention (`frame, visible, scene_id, layer`) to every view-category tag? Yes/no/customized — if customized, list the project's universal view fields.
- **Vocabulary location:** `signals/0.1.json` (default) or per-project alternative.
- **Vocabulary CI gate command:** {e.g., `scripts/check_vocab.sh`} — the command the Agent runs at every sprint close to verify no emit site references out-of-vocab tags.

---

## Build and verification commands

*Per AGENTS.md "The dual contract": the Agent runs these at every sprint close and reports exit codes; it diagnoses failures instead of silently retrying.*

- **Primary build:** `{command}` — expected exit code 0
- **Test suite:** `{command}` — expected exit code 0
- **Linter:** `{command}` — expected exit code 0 (optional)
- **Format checker:** `{command}` — expected exit code 0 (optional)

---

## Observation contract environment (for UI-touching projects)

*Per `sdd-kit/AGENTS.md` hard rule 9 and `sdd-kit/AGENTS.md` § "The dual contract (and observation contract)". The Agent names the tools used to verify product behavior, preferring ones it can drive itself (a headless browser, a simulator harness); only physical-world observation falls to the human.*

- **Simulator / runtime environment:** {e.g., iPhone 17 Pro simulator, UDID `<UDID>`}
- **Build-and-install command:** {e.g., `xcodebuild build ... && xcrun simctl install booted ...`}
- **Launch command:** {e.g., `xcrun simctl launch booted app.example.MyApp`}
- **Log tail command:** {e.g., `xcrun simctl spawn booted log stream --predicate 'subsystem == "app.example.MyApp"'`}
- **Screenshot command:** {e.g., `xcrun simctl io booted screenshot /tmp/shot.png`}
- **UI driving tools available:** {e.g., `mcp__ios-simulator__ui_tap`, `mcp__ios-simulator__ui_describe_all`, `mcp__ios-simulator__ui_find_element`}

If no UI, omit this section.

---

## Hand-author log

*Per AGENTS.md hard rule 10 (discipline bypass is logged, never silent). When the Agent authors an artifact outside the normal card-and-contract flow, the bypass is logged here in the same turn and stated in chat.*

*(empty until the first bypass)*

- **{YYYY-MM-DD}** — Sprint {NNN}: hand-authored {file or change} because {what was attempted and why each attempt failed}. Bypassed: {what was bypassed, e.g., observation contract, vocabulary check}. Accepted drift: {what risk remains}. Contracts re-run on the result: {outcome}.

---

## Tone canon (for UI-touching or content-producing projects)

*If the project has player-facing or user-facing strings, name the voice/tone canon here. The Agent reads this when populating Layer 7 (Evidence) tonal constraints during the Vocabulary Session and when authoring strings during regular sprints.*

- **Voice register:** {e.g., "lowercase, terse, no exclamation marks, no marketing language"}
- **Forbidden constructions:** {e.g., "no second-person address in narration"; "no emojis"}
- **Reference:** {path to the voice canon document if separate}

---

## Drift surface log

*Patterns the project has identified as worth watching across sprints. Migrates here from `BLACKBOARD.md ## Drift watchlist` when the pattern becomes stable enough to name as a project invariant.*

*(empty on project start)*

- **DS-1.** {pattern description}; mitigation: {what to do when the pattern surfaces}.

---

## Sprint cadence policy

*Per AGENTS.md "The cadence": one continuous band. Every card is self-reviewed and executed without a go/revise wait. The project lists its phases so phase-boundary chat summaries have names.*

- **Phase 0 (Vocabulary Session):** {one line}
- **Phase 1 ({phase name}):** {one line}
- **Phase 2 ({phase name}):** {fill}
- ...

---

## Project-specific user-only questions

*AGENTS.md lists the closed set of questions only the user can answer. A project may name concrete instances of them (e.g., "enabling connector X accepts its terms-of-service risk"), so a later session recognizes them without re-deriving. It may not add categories: anything outside the AGENTS.md list is settled on the grounds.*

- `{question}` — category: {AGENTS.md category}. Ask when: {condition}. Work that can continue meanwhile: {what}.

---

## Custom techniques

*Project-specific techniques that aren't part of the kit's universal catalog but the project uses repeatedly. Document here so the Agent applies them by default.*

- **{Technique name}** — {one-paragraph description, including when to apply and what success looks like}.

---

*WORKING_AGREEMENT.md (sdd-kit-3) — template. Copy to project root and fill. The Agent reads AGENTS.md (the methodology) then this file (the project specifics). Augments, never overrides, AGENTS.md hard rules.*
