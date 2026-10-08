# KIT_DIARY — w2m

*What sdd-kit-3 does well on this project, what gets in the way, and what the next kit version should change. Template: `sdd-kit-3/templates/KIT_DIARY.md`.*

---

## Hypothesis tracking

| # | Hypothesis | Verdict | Evidence |
|---|---|---|---|
| H1 | The research direction's discrepancy report (§11) and the kit's Signal Report are the same artifact; one vocabulary serves both the w2m runtime trace and the sprint loop. | _pending_ | — |
| H2 | Resolve-and-record holds up on a project whose key facts (TSO access, oracle behavior) can only be settled by probes: the agent records the decision provisionally and a probe sprint confirms or supersedes it, with no wait state. | partially | 2026-10-03: four same-day agent decisions superseded cleanly by the v0.2 user decision; but two of them were scope (intent), not probe-settleable facts. OQ-1 is still the first probe test. |
| H3 | The ≤2-files sprint rule fits a compiler/runtime project if contracts are one file each. | _pending_ | — |
| H4 | A correctness check that passes on zero observed failures needs a same-run control that fails. | confirmed | Sprint 011: the native MP check passed with no power until the plain control was required to fail. |

---

## Entries

### 2026-10-03 — Adoption and review

**What happened:** The agent read the kit in full (44 text files), read substrate's tutorial, API reference, applications, and topology guide, checked the research direction's external claims against primary sources, and wrote the review, BLACKBOARD, WORKING_AGREEMENT, and the Sprint 0 card.

**What worked:**

- Resolve-and-record let five architecture decisions land in one turn with grounds, instead of five questions.
- The closed user-only list sorted the open items cleanly: game choice (intent), oracle hardware (money), Steam login (credentials), license (risk).

**What got in the way:**

- `templates/WORKING_AGREEMENT.md` assumes one primary language; this project has at least three components in different languages. The agreement has no per-component build-command table.
- The kit's grammar assumes signals are emitted by the product. Here a second signal source exists: probe programs on a Windows oracle, which cannot import the project's emitter. The vocabulary needs a transport rule for oracle-side traces.

**What this says about the next kit version:**

1. `templates/WORKING_AGREEMENT.md` could carry a per-component table (language, build, test, vocabulary gate).

### 2026-10-03 — User decision: v0.2 is the plan

**What happened:** The user supplied `windows_to_macos_research_direction_v0_2.md` and said "this is the plan." It overruled four agent-resolved decisions from the same day (VM-first backend, AOT as cache, the named protection exclusions, the D3D11 slice) and kept two (substrate as harness, evidence-built contracts).

**What worked:**

- H2 evidence, partial: resolve-and-record made the four calls visible enough that a revised founding document could name and reverse each one. The supersession took one appended user entry; no entry was edited.
- The review fed the revision. v0.2's "made explicit" list takes in the review's findings on code discovery, TSO, the Steam bridge, Metal Shader Converter, and the oracle, and rejects its scope narrowing.

**What got in the way:**

- The agent resolved a product-scope question (which backend and which protection families are in v0) on engineering grounds. The user treated it as intent. The kit's line between "intent the artifacts do not contain" (user-only) and "structure" (agent-settled) blurred: picking a starting architecture for a research program is closer to intent than to structure.

**What this says about the next kit version:**

2. AGENTS.md's user-only list could name "narrowing the scope of the founding document" as intent, even when engineering grounds favor a narrowing. The agent proposes the narrowing; the user decides it.

### 2026-10-05 — Sprint 000 (Vocabulary Session) closed

**What happened:** The agent ran BOOTSTRAP Steps 0-12 against v0.2 and locked `signals/0.1.json` (60 tags, 12 categories). It wrote `scripts/check_vocab.py` to run the self-review checks against the file on disk.

**What worked:**

- Reading substrate's sequencer before Layer 1 settled the transport in one line of code (`event_kind = type(obj).__name__`): tag name and event kind can be the same string. TECHNIQUES.md #46 (bridge mapping from installed source) paid off before any bridge sprint existed.
- The mechanical gate caught two of the four Rubber Duck observations' neighbours: an irregular verb and two foreign keys. A gate written as code beat the BOOTSTRAP anti-pattern "rubber-stamp self-review" without extra ceremony.
- v0.2's own sentences supplied most invariants (§11.3 → S7/S8, §14 → S2/T7, §3.1 → T3/E10). A founding document written as contract prose makes Layers 4-7 nearly transcription.

**What got in the way:**

- BOOTSTRAP assumes one transport. A system whose observed program cannot import the emitter (the Windows reference machine) needs a second plane, and the kit has no slot for it; the decision had to go in a project-specific `transport` block.
- The card's "≤2 files" self-review item conflicts with BOOTSTRAP's required three outputs (vocabulary, rationale, proposals) before any gate script exists.

**What this says about the next kit version:**

3. `templates/VOCABULARY.json` could carry an optional `transport` block (planes, ordering source) for projects whose observed program and harness run in different processes or machines.
4. Ship a structural vocabulary gate with the kit (`lib/check_vocab.py`), since every project runs the same Step 1-10 checks; `scripts/check_vocab.py` here is a candidate.

### 2026-10-05 — Sprints 001-003 closed (Phase 1 opening chain)

**What happened:** Three sprints built the harness package, generated typed events from the vocabulary, and the guest-trace ingester, each graded against a real substrate run record.

**What worked:**

- The observation contract caught nothing because the earlier probes had: two msgspec/substrate behaviors (no constraint checks at construction; RFC 8785's 2^53 integer limit) were found by five-line experiments before code depended on them. Running the transport's own encoder on boundary values belongs in every bridge mapping (TECHNIQUES.md #46).
- Generating the Structs from `signals/0.1.json` makes vocabulary drift a failing `--check`, which is the kit's "vocabulary parity validator beyond Python" (TECHNIQUES.md Section 3) arriving for free in a Python harness.

**What got in the way:**

- Hard rule 6 (≤2 files) counted a generator, its generated output, and its test as three files. The generated file is not authored; the rule's count could exclude generated artifacts.

**What this says about the next kit version:**

5. Hard rule 6 could exempt generated files whose generator is in the same sprint and whose `--check` is a graded command.

### 2026-10-07 — Sprints 004-011 (contracts, Windows reference loop, Mac capability and timing probes)

**What happened:** Eight sprints took the project from a vocabulary to evidence on both sides: an accepted Windows contract backed by 10 runs on a hosted `windows-2025` runner, public Hypervisor.framework controls for 4 KB guest pages and guest TSO, a litmus test showing EnTSO's effect (2245 vs 0 forbidden outcomes in 50M), and per-access costs of TSO on both backends. One vocabulary evolution (0.1 → 0.2) went through the proposal taxonomy.

**What worked:**

- The observation contract caught two false results in Sprint 011 that every content assertion passed: a 6.2x VM penalty that was a warm-up asymmetry, and a correctness precondition with no statistical power. Both would have shaped the backend decision.
- Reading the installed SDK headers before writing code (TECHNIQUES.md #46) made the Hypervisor probes work on the first run, and answered OQ-1 before any probe ran.
- Resolve-and-record absorbed four user redirections in one day (v0.2 plan, no physical machine, optional reference runs, private repo) as appended decisions with no edits to earlier ones.

**What got in the way:**

- The agent framed the Windows reference as a blocker and as a hardware purchase; the user had to ask twice. The kit's user-only list ("money, accounts") pulled a development aid into a purchasing question. The kit has no prompt to ask "is this on the product's critical path, or only on the evidence path?"
- Twice the agent's own diary edits landed in the wrong entry (text-anchor inserts). Mechanical: anchor on headings, verify order after each insert.

**What this says about the next kit version:**

6. The observation contract template could carry a "power" line for any check that passes on absence (zero violations, no errors): name the control that must fail in the same run.
7. Measurement sprints could require symmetric setup (warm passes, same allocation path) as a self-review checklist item.

## Phase boundary synthesis — Phase 2 (destination capability probes), 2026-10-08

**Delivered:** measured inputs for v0.2 §8.2 on both backends, on mac-Mac17,7-macos-25F80. Managed VM: 4 KB IPA granule present; guest TSO present and effective (MP 2245/50M off, 0/50M on); 4 KB guest page protection present in hardware; per-access cost of TSO and 4 KB pages within ~10% of native plain code. Native process: no public TSO control; LDAPR/STLR lowering correct and 2.6-2.7x per access; DMB fences 7.4x; no 4 KB mprotect (EINVAL); software page checks 2.1x per access, 1.04-1.12x when hoisted to page entry, valid only with an invalidation rule.

**Not delivered, by design:** a backend choice. v0.2 §8.2 requires guest/host interface costs (graphics, audio, input, file I/O across the VM boundary) and whole-application measurements, which need the CPU translator and a game.

**Through-line:** three of the four first-pass numbers in Phase 2 measurements were wrong for reasons the observation contract exposed and content assertions never would: warm-up asymmetry (6.2x), a litmus with no power, and branch layout (1.9x). H4 confirmed; the kit's measurement guidance needs symmetric setup, same-run controls, and a layout check for hot loops.

**What this says about the next kit version:**

8. Measurement sprints: treat each first-pass number as a hypothesis and name, before running, what would make it an artifact (warm-up, power, code layout, core placement).

### 2026-10-08 — User correction: no "cost" framing

**What happened:** The user rejected the word "cost" and the phrase "the native backend pays 2.7x" in the Phase 2 report: "There's no cost... It's going to poison your whole analysis."

**What was wrong:** The Phase 2 numbers are execution times of specific lowerings applied to every access of one synthetic loop. Reporting them as what a backend "pays" assigned a per-access penalty to the native backend in advance, which v0.2 §8.2 forbids ("Neither is assigned an assumed per-access or zero-crossing cost in advance") and §13 contradicts (enforcement is measured after optimization). A specializing compiler emits ordering only where the contract requires it and removes redundant checks; the microbenchmark measured neither.

**Correction:** Phase 2 delivered correctness evidence on both backends and the execution time of each enforcement variant on one kernel. It delivered no property of either backend. The phrases in this diary's Phase 2 synthesis ("per-access cost of TSO", "2.6-2.7x per access", "software page checks 2.1x per access") read as measurements of those variants on that kernel only.

**What this says about the next kit version:**

9. Report measurements as what was measured (variant, kernel, ratio, run). Value words (cost, pays, penalty, overhead, cheap) import a conclusion the data does not carry.
