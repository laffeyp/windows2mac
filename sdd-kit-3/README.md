# sdd-kit-3

*sdd-kit-2 with the wait states removed. Same methodology, same files, same hard-rule count. Where sdd-kit-2 told the agent to halt, write to the BLACKBOARD, and wait for the Architect's "go", sdd-kit-3 tells it to settle the question on three stated grounds, record the decision, say it in chat, and keep working. Created 2026-09-27 in the `househunt` project; `sdd-kit-2/` is kept unchanged beside it as the audit trail.*

---

## What changed from sdd-kit-2/

One change, applied everywhere it appeared: **halt-and-articulate became resolve-and-record.**

- **Three grounds, most specific first.** The project (its Decisions, founding document, vocabulary, working agreement, code, tests), then SDD practice (this kit), then software-engineering practice with a named source. The agent's priors are not a ground. The full rule is in `AGENTS.md` "Resolve-and-record".
- **Agent-resolved decisions go into `## Decisions`.** `## Decisions` is no longer Architect-only. It holds user decisions and agent-resolved decisions, each labeled; an agent-resolved entry carries its grounds and a revisit condition; a user decision supersedes any agent-resolved one. Nothing is edited or removed.
- **Chat is the channel; the BLACKBOARD is bookkeeping.** Everything the agent writes to `## Surfaced for review` or `## Decisions` is also said to the user in chat in the same turn. The user is never expected to go read the BLACKBOARD to find a question.
- **A closed list of user-only questions.** Intent the artifacts do not contain; money, accounts, credentials; outward-facing or irreversible acts; risk only the user can accept; physical-world observation; contradictions in the user's own instructions. For these the agent asks in chat, once, with a recommendation, and keeps working on independent work.
- **Every sdd-kit-2 halt has a replacement action.** The table is in `AGENTS.md`. `vocabulary_change_required` becomes propose, ratify on the grounds, bump the version, record. `bridge_mapping_required` becomes reverse-engineer the SDK and write the mapping. `observation_contract_missing` becomes write the contract. `awaiting_architect_decision` becomes a partial-pass rule.
- **One cadence band.** Plan-mode-per-sprint and its "go or revise?" wait are gone. The agent self-reviews each card against the same checklist and proceeds. Plan mode is used only when the human turns it on.
- **The agent runs the build.** In sdd-kit-2 the human ran build commands and reported exit codes. In sdd-kit-3 the agent runs them and puts exit codes and output tails in the Signal Report.
- **Hand-author authorization became bypass logging.** Hard rule 10 still guards against bypassing the discipline, but a bypass is now logged instead of waiting for permission.
- **The Vocabulary Session runs without per-layer sign-offs.** Each "Architect: say 'Layer N ready'" became an agent self-review checklist carrying the same checks. `grammar/BOOTSTRAP.md` gives the procedure.

Not changed: `foundations/` (hard rule 1), `ADDENDUMS.md` (dated captures from other projects), `process-not-prompt-*.md` (research record), `lib/sdd.py`, and `example/`. The example's BLACKBOARD still shows the sdd-kit-2 halt-and-acknowledge exchange from 2026-05-20/22 as a dated record; `example/README.md` notes this.

Why the change: a wait for approval of a decision the grounds already settle costs the human a turn and adds no information. What halt-and-articulate protected — no silent decisions, a written reason for each, an audit trail — is kept by the record. The hypothesis is tracked in the adopting project's `KIT_DIARY.md`.

---

## What changed from sdd-kit/ (inherited from sdd-kit-2)

`sdd-kit-2/` is the slim refactor. The original `sdd-kit/` remains in place as the audit trail (per the project's never-delete-originals discipline).

Cuts:

- **Three grammar files folded back**: `EMPIRICAL.md`, `PER_LAYER.md`, `EXAMPLES.md` not carried forward. The originals built vocabularies without a formal treatise; the two remaining files (`PRINCIPLES.md` + `BOOTSTRAP.md`) are the minimal explicit version of what worked.
- **Three procedure files folded inline**: `COMPREHENSION_AFFIRMATION.md`, `DUAL_CONTRACT.md`, `RUBBER_DUCK_PASS.md` not carried forward as separate files. Their procedures live inside `AGENTS.md` directly — that's how the originals practiced them (inline in the working agreement) and that's what worked.
- **TECHNIQUES.md application-gate ceremony dropped**: AGENTS.md's hard rule 13 ("cite techniques in every sprint card"), the sprint card's `## techniques applied` section, and the Rubber Duck Pass's technique-citation drift check — all removed. The catalog itself stays, but as a reference, not a gate. The prompt-factory's earlier TECHNIQUES.md had gates and didn't transmit; the simplest kit closes the failure mode by removing the ceremony and trusting the catalog to be consulted when relevant.

Additions:

- **`example/` folder** with a small worked CLI project: vocabulary, three sprint cards, BLACKBOARD with real entries, KIT_DIARY with real entries, source files. The original four projects' biggest legibility advantage was concrete artifacts to pattern-match against; `sdd-kit/` shipped only abstract templates and lost that.
- **TECHNIQUES.md expanded**: 53 universal techniques (up from 32) + 11 project-class subsections (up from 6). Made comprehensive so projects don't reinvent.
- **Hard rule 12 in AGENTS.md (no deletions)** — restructures always land in new files / folders / round-N versions. The audit trail is the work.

Net: 24 files → 17 files (8 fewer); plus an 8-file example project. The kit's claim of "no orchestration" is honest. The methodology is unchanged; the ceremony is gone.

---

## What this is

A folder of markdown documents + one ~150-line Python reference library. No orchestrator. No HTTP adapters. No dispatch logic. No build runners. No CLI. The kit is what the agent reads; the agent + human run the loop.

The methodology comes from four projects (Audio Object, Trading System, Make The Models Talk, Katybird) that practiced SDD without any orchestration layer — one demonstrated ship (Katybird) plus three formative/exploratory projects whose lessons inform the discipline. The kit additionally absorbs structural lessons from the soundfield project (a more recent project that ran with a heavy orchestrator and demonstrated what was over-engineering versus what was load-bearing).

The kit ships:

- `AGENTS.md` — the working agreement, tool-agnostic. The load-bearing read at session start. Procedures (comprehension affirmation, dual contract, Rubber Duck Pass) folded inline.
- `CLAUDE.md` — thin shim for Claude Code sessions; points at AGENTS.md.
- `TECHNIQUES.md` — comprehensive catalog of named development techniques. Reference, not gate. Universal (Section 1, 53 entries) + project-class-specific (Section 2: visual, audio, iOS, backend, LLM-integration, CLI, web, data science, game dev, documentation, embedded) + what's deliberately not in the kit (Section 3, orchestration-deferred).
- `ADDENDUMS.md` — techniques earned on real projects *after* the catalog stabilized, not yet back-propagated into `TECHNIQUES.md`. The holding pen between a project's `KIT_DIARY.md` and the numbered catalog. Top-level and visible on purpose: the newest lessons are the ones a fresh session is least likely to already know. On file: Addendum A — the visual/UI browser verification harness (pixel-anchor decode, three-lens grading, harness-is-code-too), from `substrate-ui`; Addendum B — the native-iOS simulator observation harness (XCUITest-as-driver, four-channel legibility, determinism-core diff-to-zero, feel-made-measurable), from `Cascade`; Addendum C — the native-iOS audio/looper harness (launch-arg signal scoping, XCUITest capture driver, on-device audio/MIDI two-ended debugging), from `Audio Object`; Addendum D — the discovery harness (a stochastic agent run wild in a disposable container to surface unknown-unknown crashes, feeding reproduce-then-kill; containerize the test not the product), from `substrate` (the runtime itself).
- `foundations/` — the four originals of the SDD canon, untouched. Read once per project; informs everything else.
- `grammar/` — vocabulary authoring discipline (two files: `PRINCIPLES.md`, `BOOTSTRAP.md`).
- `templates/` — the artifacts a project instantiates (six files: VOCABULARY.json, BLACKBOARD.md, SPRINT_CARD.md, SIGNAL_REPORT.md, WORKING_AGREEMENT.md, KIT_DIARY.md).
- `lib/sdd.py` — the reference library from foundation 02 (~150 lines). Opt-in for projects that want typed `emit_signal()` in Python.
- `example/` — a worked mini-project (a tiny CLI) showing the discipline applied end-to-end.

---

## How a project uses this kit

1. Copy `sdd-kit-3/` into the project. Pick a name for the project's own version (e.g., `_sdd-kit/`). Add a project-root `CLAUDE.md` (or the tool's equivalent) that points at the kit's `AGENTS.md`, so the agent loads it at session start.
2. Open a session with any LLM agent (Claude Code, Cowork, Cursor, Aider, Continue) and have it read `AGENTS.md` first.
3. The human states the project's intent in plain English: a founding document, a spec, or a paragraph in chat.
4. Agent copies `templates/BLACKBOARD.md` to project root and writes the first Decision (project scope) as a user decision, citing where the human stated it.
5. Agent runs the **Vocabulary Session** following `grammar/BOOTSTRAP.md`, settling each layer on the three grounds and asking in chat only the user-only questions. Output: `signals/0.1.json` + `signals/0.1-rationale.md`.
6. Agent drafts each sprint card using `templates/SPRINT_CARD.md`, self-reviews it, states its scope in one line in chat.
7. Agent reads AGENTS.md + the sprint card + the relevant foundations + the working agreement, writes a Comprehension Affirmation (first session), executes the sprint, runs the build/verification commands, returns a Signal Report per `templates/SIGNAL_REPORT.md`.
8. Agent runs the Rubber Duck Pass per the procedure in AGENTS.md, writes observations + dispositions to BLACKBOARD per single-writer discipline, and reports them in chat.
9. Agent writes a KIT_DIARY entry per `templates/KIT_DIARY.md` at phase boundaries and whenever the kit helped or got in the way.
10. Repeat from step 6. The human steers whenever they choose to; each steer is recorded as a user decision.

For a worked example of all this end-to-end on a tiny CLI project, see `example/` (written under sdd-kit-2's halt protocol; the discipline is otherwise the same).

No `python -m anything` runs. The kit is read; the agent acts and records; the human steers.

---

## What this kit does NOT do

Stated explicitly so the gaps are clear and project teams can fill them with their own tooling.

- **No orchestrator.** No process spawns workers, parses Signal Reports, writes artifacts to disk, or merges BLACKBOARD entries automatically. The agent does all of this through its own tool calls (Read, Write, Edit) inside the LLM session it runs in.
- **No build runner.** Dual contract requires running build commands (`swift build`, `pytest`, `cargo test`, `npm run build`). The agent runs them through its own shell tool. If the project wants more automation, wrap the kit in a `make` target or a CI step — outside the kit's scope.
- **No HTTP adapters.** No Ollama, OpenAI, Anthropic, or Bedrock integration. The kit assumes the agent runs in whatever LLM session the human chose.
- **No vocabulary parity enforcement beyond Python.** `lib/sdd.py` validates at the speaker's mouth for Python projects. For Swift, TypeScript, Rust, anything else: discipline-enforced (the agent honors the vocabulary; the Rubber Duck Pass records violations; the human catches what slips through). Projects that want hard parity ship a language-specific checker.
- **No artifact content-assertion runner.** Sprint cards declare assertions like "file contains X"; the agent greps or runs a project-side checker.
- **No best-of-N parallel dispatch.** No correction loops. No patch applier. If the agent's first output is wrong, the agent iterates in its own session; no Python loop manages it.
- **No comprehension-affirmation gate.** The procedure asks the agent to write the affirmation at session start and check it against the three-facts rule; no code refuses to dispatch sprints if the affirmation is missing.
- **No sprint card parser.** Sprint cards are plain markdown files in `sprints/`. The agent reads the next one and works on it; the human may review any card at any time.
- **No signal trace persistence across sessions.** `lib/sdd.py` keeps signals in an in-memory deque; the process dies, the trace is gone. If the project wants cross-session traces, wire `lib/sdd.py` to dump JSONL at session end and read on next start.

The bet: discipline + procedures + clear failure modes documented in advance get a Tier-1 LLM agent (Claude Code, Cowork, Cursor) most of the way without any of these. The soundfield build's heavy orchestrator generated more bugs and more meta-work than it solved.

---

## Lessons the kit absorbs vs lessons it leaves behind

**Absorbed (structural):**

- Sprint sweet spot is ≤2 files / one concept. → `AGENTS.md` hard rule 6.
- Canonical home registry — name which file owns which type. → `templates/WORKING_AGREEMENT.md` has a section for this; `AGENTS.md` hard rule 7.
- The dual contract works and is required. → `AGENTS.md` "The dual contract" section.
- Observation contract beats content assertion for behavior-touching work. → `AGENTS.md` dual-contract section + hard rule 9.
- Diary discipline (what worked / what got in the way) is what makes lessons compoundable. → `templates/KIT_DIARY.md` is a first-class artifact.
- Comprehension affirmation as session-start ritual. → folded into `AGENTS.md`.
- Originals over summaries (the v1 → v1.2 transmission lesson). → `AGENTS.md` hard rule 11; `foundations/` kept verbatim.
- Discipline bypass must be visible (sdd-kit-2: hand-author requires explicit human authorization; sdd-kit-3: bypass is logged). → `AGENTS.md` hard rule 10.
- Sprint-0 vocabulary materialization (don't backfill at sprint-60). → `grammar/BOOTSTRAP.md` is the explicit founding act; `AGENTS.md` hard rule 12.
- External-package bridge mapping before any code authoring. → `grammar/BOOTSTRAP.md` + `templates/WORKING_AGREEMENT.md`.
- Design-bundle context inclusion for UI-touching sprints. → `templates/SPRINT_CARD.md` notes; `AGENTS.md` hard rule 8.
- The rubber-duck-pass-after-sprint-close discipline. → folded into `AGENTS.md`.
- No deletions — restructures land in new files / round-N versions. → `AGENTS.md` hard rule 12 (in the "should never do" list).

**Not absorbed (kit-operational):**

- Best-of-N tuning, build-validation scoring weights, halt-on-all-fail discipline. → No best-of-N here.
- `think: false` in the Ollama adapter, HTTPStatusError retry classification. → No adapter here.
- `_emit_log` locking under ThreadPoolExecutor concurrency. → No emitter loop here.
- patch_applier.py CRLF round-trip discipline, multi-divider SR-block rejection, atomic temp+rename. → No patch applier here.
- preflight ripple detection, drafter public-surface enumeration, codebase_grepper brace-walking. → If a project wants these, they're available in the soundfield build's `_factory/prompt-factory/orchestrate/` as reference; this kit doesn't ship them.
- Foreman lock POSIX vs Windows fallback. → No foreman here.
- TECHNIQUES.md application gates (re-skim before drafting, cite in every sprint card, plan-mode-checklist verification). → The catalog stays; the gates don't. The prompt-factory had these gates and they didn't transmit; trusting the catalog to be consulted when relevant is the simpler bet.

These are real engineering accomplishments; they belong in an orchestration framework, not in a methodology kit.

---

## When to outgrow this kit

This kit is for projects in the bottom 80% of operational complexity:

- Single human + single LLM agent
- Sprint cadence is sequential (one sprint closes, the next dispatches)
- Build runs locally on the machine the agent session runs on
- The project is one repo, one team, one product

When a project crosses into the top 20%:

- Multiple agents running in parallel
- CI dispatching sprints automatically
- Dispatch decisions need to be made by a scheduler process, with no agent session behind them
- The project is being run as a service for other teams

… then the kit alone is no longer enough. Two paths from here:

- **Wrap the kit in a thin orchestrator.** A Python script that walks `sprints/`, dispatches Workers to an LLM, parses Signal Reports, writes artifacts. The discipline (this kit) stays the same; the orchestrator handles the operations.
- **Use a public orchestration framework.** Aider, Cline, Cursor agent, Continue, AutoGen, LangGraph all handle dispatch + tool use + patch application well. Bring this kit's discipline as the prompt + the working agreement + the templates. Let the framework handle orchestration.

The kit does not ship either path. The kit is the methodology. Orchestration is a separate concern.

---

## Reading order for a new agent on first session

1. `AGENTS.md` — load the working agreement into context.
2. `foundations/01-signal-driven-development.md` — the theory.
3. `foundations/04-sdd-claude-design.md` — the founding-act framing (sets up the Vocabulary Session).
4. `grammar/README.md` — points at the two grammar files.
5. `grammar/PRINCIPLES.md` — the 11-layer stack + non-negotiable commitments.
6. `grammar/BOOTSTRAP.md` — the Vocabulary Session procedure.
7. `example/README.md` — the worked example, useful for pattern-matching at first sprint.

`TECHNIQUES.md` is a reference, not first-session reading. Skim once to know what's there; dip in when relevant.

Other foundations (`02-sdd-practice.md`, `03-sdd-team-model.md`) are read as the work demands.

---

## Project layout after adoption

After the kit is copied into a project (say, named `myproject/`) and work starts:

```
myproject/
├── CLAUDE.md                               (points the agent at sdd-kit-3/AGENTS.md)
├── sdd-kit-3/                              (this folder; never edit; changes land as sdd-kit-4/)
├── BLACKBOARD.md                           (instantiated from sdd-kit-3/templates/BLACKBOARD.md)
├── KIT_DIARY.md                            (instantiated from sdd-kit-3/templates/KIT_DIARY.md)
├── WORKING_AGREEMENT.md                    (instantiated; project-specific overrides on top of AGENTS.md)
├── signals/
│   └── 0.1.json                            (the project's locked vocabulary, produced by the Vocabulary Session)
├── sprints/
│   ├── sprint-001-<slug>.md
│   ├── sprint-002-<slug>.md
│   └── ...
├── src/                                    (the project's actual code)
└── tests/
```

`sdd-kit-3/` is read-only — the kit's discipline. The project's own files (BLACKBOARD, KIT_DIARY, WORKING_AGREEMENT, signals/0.1.json, sprints/) are the project's evolving state.

---

*sdd-kit-3. sdd-kit-2 with the wait states replaced by resolve-and-record. Text and convention. No orchestration. No deletions. The agent runs the loop and records each decision; the human steers; the kit is what they read.*
