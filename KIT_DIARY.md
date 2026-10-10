# KIT_DIARY — w2m

*What sdd-kit-3 does well on this project, what gets in the way, and what the next kit version should change. Template: `sdd-kit-3/templates/KIT_DIARY.md`.*

---

## Hypothesis tracking

| # | Hypothesis | Verdict | Evidence |
|---|---|---|---|
| H1 | The research direction's discrepancy report (§11) and the kit's Signal Report are the same artifact; one vocabulary serves both the w2m runtime trace and the sprint loop. | _pending_ | — |
| H2 | Resolve-and-record holds up on a project whose key facts (TSO access, oracle behavior) can only be settled by probes: the agent records the decision provisionally and a probe sprint confirms or supersedes it, with no wait state. | partially | 2026-10-03: four same-day agent decisions superseded cleanly by the v0.2 user decision; but two of them were scope (intent), not probe-settleable facts. OQ-1 is still the first probe test. |
| H3 | The ≤2-files sprint rule fits a compiler/runtime project if contracts are one file each. | refuted | Sprints 023-033: ten of eleven cards recorded an exception; Sprints 029-032 touched 27-55 files each. The checkable unit is one path from input to verdict. Project override recorded 2026-10-09. |
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

### 2026-10-08 — Project review: the decoder phase ran outside the kit

**What happened:** An external review (`docs/review-2026-10-08-project.md`) of commit 34b85f0 found that Sprints 013-022 kept their gates and observations but dropped the vocabulary (no signals from any differential), six required card sections, pre-declared observation contracts, and append-only record order. Ten sprints ran in one day with no exit criterion, each naming the next decoder sprint from the last one's residue. The plan's central loop (contract, generated implementation, verification) was never exercised in 23 sprints.

**Why it happened:** Each decoder sprint had a real external check (XED), so each felt verified. The check was real and the discipline around it decayed: a differential that prints a table looks like an observation, so the signal leg was graded "vacuous (library)" ten times without anyone asking whether a comparison that produces verdicts is a library. The residue of every differential is a ready-made next sprint, which makes the work self-perpetuating unless a finish line is written before it starts.

**What the review got right that the sprints missed:** one decoder defect class (prefix before VEX, 1,170 of 2,813 such generated cases wrongly accepted) sat outside the generator's variation space; the review found it by reading SDM §2.3.2, not by running more cases.

**What this says about the next kit version:**

10. A generated-input differential is only as wide as its generator. The card for a generator should name the fields it varies, derived from the format's own definition, and list what it does not vary.
11. A chain of sprints in one area needs an exit criterion written in the first card of the chain. "The residue becomes the next sprint" is the failure mode to name.
12. "Signal vacuous (library)" should be refused for any sprint whose code produces verdicts: a comparison is an operator, and its verdicts belong on the signal plane.

### 2026-10-08 — Two records of what was not looked at

**What happened:** In Sprint 024 the agent wrote a Signal Report trace from memory; it listed session tags the judge had never emitted. In Sprint 026 a web-fetch summary gave a wrong field order for a binary format. Both were caught only because the next step read the primary source: the run record, LLVM's struct and the bytes of a real file.

**What this says about the next kit version:**

13. A Signal Report's `signal_trace` should be produced by a command that reads the record (the kit's `lib/sdd.py` could ship one), never typed. Format facts for binary parsers should be checked against a second implementation and a real file, not a prose summary of the specification.

### 2026-10-08 — One program path under lockstep passed a planted compiler fault

**What happened:** Sprint 030 compared the VM back end with the model after every one of the skeleton's 5,153 instructions and found no difference. A planted fault (SUB's borrow compiled as a signed compare) also found no difference, because every SUB on that path had operands of equal sign. A random-state differential over the same 217 instructions found it at the second case. The same differential then missed a planted store-width fault twice: first because the compared stack was all zeros, then because the skeleton's 32-bit stores go to .data, outside the compared window.

**What this says about the next kit version:**

14. A comparison oracle is graded by the faults it finds, not by the runs it passes. A card that declares an equivalence check should also declare at least one planted fault per rule class it claims to cover (flag rule, width, memory), and the check counts only after each planted fault is found.

### 2026-10-08 — The first generated candidate showed the contract was the weak part

**What happened:** Sprint 031 ran a local code model against `windows.thread.last_error.set_get@0.1.0`. The loop worked: generate, admit, run in the VM, judge, accept; and on a planted wrong candidate, reject, report, route, repair, accept, resolve. Checking what else the guard would accept showed `return 1234` conforms. The contract had been accepted on documented evidence and three Windows runs, and nothing in its construction asked whether a trivial implementation would pass it.

**What this says about the next kit version:**

15. A contract that will judge generated code should be tried against the cheapest wrong implementations (a constant, a no-op, an echo of the input) before it is accepted. If one passes, the scenario is not discriminating, however well the evidence supports it.

### 2026-10-09 — Two harness bugs that only a new program could reach

**What happened:** Sprint 032 replaced the skeleton with a probe that loops three times over SetLastError and GetLastError. Lockstep, clean on the skeleton since Sprint 030, reported the VM one instruction ahead at step 4,404: after a kernel32 call the VM re-entered the guest, and a return address compiled on an earlier pass ran before the next check. The skeleton returns to each address once. The same sprint's test of a candidate that raises found the loop rejecting one candidate three times: the runner had filled the shared result list for all three failed runs before the trigger read the first event. Sprint 031 never had every run fail. Neither bug was in the code under test; both were in the code that judges it.

**What this says about the next kit version:**

16. A verification harness has control paths of its own, and a clean result covers only the paths the subject program drove. A card that relies on an oracle should name the oracle's branches (first visit and revisit; all runs pass, some fail, all fail) and give a fixture for each, the way #14 asks for a planted fault per rule class. A trigger predicate should read its event, not state another operator writes.

### 2026-10-09 — Two fixtures chosen from a description, not from the thing

**What happened:** The test game, BioShock 2 Remastered, was chosen on 2026-10-05 from its Steam store page ("64-bit only"). Its Windows depot, fetched on 2026-10-09, is 32-bit: `Bioshock2HD.exe` and six DLLs are PE machine 0x014c. The store line and Steam's `osarch 64` describe the required OS, not the binary. Separately, the decoder phase closed on 2026-10-08 on "spec gaps 0" measured on the Python 3.14.8 corpus. On the replacement game, Hail to the Rainbow, the decoder has no row at 8,935 reachable positions in 123 kinds, all SIMD (CMPPS 4,757; MOVMSKPS 1,442; RSQRTPS 432; RCPPS 423; MOVNTPS 357), and 142 of the 577 rows the game uses lack XED confirmation. Integer-heavy interpreter code was a sound fixture for integer decoding and the wrong one for a game engine.

**Finding (#17):** a fixture is chosen by reading the artifact the work will run on. A profile record states the PE machine field, read from the binary. An exit criterion names its fixture and says why that fixture stands for the target; when the target changes, the criterion is re-measured on it before it is cited as met.

**What worked:** the 2026-10-08 Decision kept criterion (d) open until the real game arrived. That one open item is how the gap surfaced on the first day the game's code was read.

**Kit change suggested:** the closed user-only list and the profile templates should carry "binary facts read from the artifact" as a required field, not a note.

### 2026-10-09 — The first independent check found what two agreeing checks could not

**What happened:** Sprint 037 ran 20,116 single instructions on the runner's CPU and compared them with the model. A 66-prefixed PUSH (`66 41 6a 48`) moved RSP by 8 in the model and by 2 on the CPU. The defect had been in `lift.py` since Sprint 029. Lockstep (VM against model, every instruction) and the state differential (16,400 random states) both passed it, because both sides executed `lift.py`'s operations. The same oracle's self-check found the lifter reading `gs:[0x1480]` as absolute address 0x1480, and three CPU models showed Intel and AMD disagreeing on `0F 0D` register forms.

**Finding (#18):** a check is independent only if its reference shares no code and no source with what it judges. Two implementations derived from one specification agree on that specification's errors (Knight and Leveson 1986, cited by this project since 2026-10-08 and not acted on for nine sprints). The cheapest independent reference here was the real CPU on a free runner; it took one sprint.

**Kit change suggested:** a technique entry for "external oracle": every verifier states what its reference is derived from, and a phase cannot close on verifiers that share a derivation.

### 2026-10-09 — Substrate used as a file format

**What happened:** The progress review counted nine of eleven substrate users computing everything in plain Python and then replaying the results through a one-producer topology. Their records show that a list was written, not that a comparison caused a discrepancy or a discrepancy a route. The repair loops that moved the project (the SDM row generator, the oracle's two lifter fixes) ran in the agent session, so their discrepancies live in commit messages. Only the candidate loop is a real topology. The architecture decision, which constrains everything, existed only as prose although the vocabulary has BACKEND_EVALUATED and BACKEND_SELECTED.

**Finding (#19):** a signal record is worth its causal chain. A producer that yields precomputed events is a log. Decisions with a tag belong on the signal plane, made inside a topology where the evidence causes the decision (now done for the backend: two evaluations trigger the selection). Repair loops belong in topologies too, triggered by DISCREPANCY_ROUTED.

**Kit change suggested:** a check that flags producers yielding more than N events with no trigger between them, and a review question: "which tagged decisions in this sprint exist only as prose?"

### 2026-10-09 — Three small errors of the same kind

**What happened:** In one evening: (1) the roadmap and an external review both planned to emit `BRIDGE_CALL_UNMAPPED` for a missing Windows service, a tag the vocabulary defines for Steam interfaces; reading its payload before writing the emitter caught it. (2) A depot resolver written as a regex returned shared redistributable depots instead of the game's own; a parser for the format fixed it. (3) A capture file stored as an asset of the game's release broke the game workflow, which downloaded every asset of that release. Each time, a name or a container was trusted for what it seemed to hold, not for what its definition says it holds.

**Finding (#20):** read the definition, not the name: a tag's note and payload, a file format's grammar, a container's full contents. A release, a directory or a tag that serves one purpose should hold only that purpose's members, or its consumers should read an explicit list (as `release.json` now does).

**Kit change suggested:** the signal-contract checklist asks "for each tag emitted, quote its note"; a sprint that adds an emitter cannot close without it.

### 2026-10-09 — A planted fault the judge could not see

**What happened:** Sprint 039's card planted "AF computed as undefined after ADD" as a fault the CPU oracle must catch. The judge compares only flags the model defines, so a model that wrongly calls a flag undefined passes every comparison. The plant was replaced with AF from the wrong bit, which the oracle caught. In the same sprint, the model's guesses about unmasked SIMD exceptions were marked "oracle" in the code; the CPU refuted one of them (UE suppressing PE) within the hour.

**Finding (#21):** a planted fault proves the check only if it lies inside what the check compares. Before choosing a plant, list what the judge skips (undefined flags, uncompared windows, unplaceable addresses); a fault there is a blind spot to record, not a test.

**Kit change suggested:** the observation contract's planted-fault line asks "which field differs when this fault is present, and does the judge compare that field?"


### 2026-10-09 — Sprint 040: two findings about references

**What happened:** Sprint 040 judged the VM's fault handling on Sprint 039's Windows records. The card planted "PUSH moves RSP before its store", and the judge found nothing: every PUSH fault in those records is a load of a memory source, because a stack store that faults cannot be dispatched on Windows and the generator never placed one. The same week's #21 had the same shape (AF undefined, outside what the judge compares). Separately, lifting one new instruction (CPUID) changed Sprint 039's case set, because the generator kept whatever the lifter could lift, and every judgment against runs already made on Windows broke at once.

**Finding (#22):** check a planted fault against the reference's inventory before writing it: list the reference cases where the planted defect changes a compared field. If the list is empty the plant is a blind spot to record, not a test. Twice in two sprints is a pattern; the card now carries the count with the plant.

**Finding (#23):** a reference's inputs must not depend on the code it judges. A case generator that consults the lifter to decide which cases exist changes the reference whenever the lifter changes. Freeze the inputs of every run made on the reference (here `CLASSES_039` and the seed, checked by the case file's sha256), and give new coverage a new run.

**Kit change suggested:** the observation contract's planted-fault line becomes "the fault, the field it changes, and how many reference cases change it"; the artifact contract of any reference generator names what its output depends on.

### 2026-10-10 — Sprint 041: a scratch register and two wordings

**What happened:** The VM's shift flag code computed the count in x16, then called `_mark_defined`, which also uses x16, then tested x16 as if it still held the count. Every shift defined OF as if by one bit. Lockstep and the state differential had not run the new classes. The CPU caught the defect in 625 cases on its first run of case set 041. In the same run, Intel showed that CMPXCHG leaves a 32-bit register destination unwritten when the comparison fails. The SDM's text ("always written") is about memory. And the SDM's flag text showed that the model had defined SHR's CF where the SDM leaves it undefined, since Sprint 029.

**Finding (#24):** an emitter helper's scratch registers are part of its signature. A helper that clobbers a register its caller reads later is a defect that type checks cannot see. Only a reference that exercises the path can.

**Finding (#25):** where the reference and the manual's wording disagree, the reference decides, and the rule cites the run (CMPXCHG, run 38029980267). Where the manual leaves a flag undefined and the CPU happens to produce a value, the manual decides what the model may promise (SHR CF).

**Kit change suggested:** generated-code helpers declare the registers they clobber; the compiler checks no live value sits in them across a call.

### 2026-10-10 — Sprint 042: a record that said "accepted" twice

**What happened:** Sprint 041's item runs fired the decision trigger on every COMPARISON_COMPLETED whose predicate held. Both judges had filled the shared state before the trigger was evaluated, so the predicate held for both comparisons and every one of the 139 class records carried two CANDIDATE_ACCEPTED events for one candidate. The index test checked each class's verdicts, its rule's sha256 and its causal chain, and passed all 139. The defect surfaced only when Sprint 042's CreateFileW run printed its event counts. The trigger is now `Once()` and every class was rerun: 141 records, one decision each.

**Finding (#26):** a test that reads a record must count the events the topology promises, not only find them. "Exactly one decision per candidate" is part of the signal contract; a check that looks for the event's presence passes a record that says it twice.

**Kit change suggested:** the signal contract states each event's multiplicity per run (one SESSION_INIT, one decision per candidate, one SESSION_COMPLETED), and the grading test asserts those counts.

### 2026-10-10 — Sprint 042: Windows's documentation and Windows

**What happened:** The CreateFileW probe ran on Server 2025 and Server 2022 and printed the same 188 rows. Three of them disagree with, or go beyond, Microsoft Learn: DeleteFileW of a file open with FILE_SHARE_DELETE removed the name at once (POSIX delete), where the page leads with "marks a file for deletion on close" and later opens failing with ERROR_ACCESS_DENIED; "name." and "name " opened "name", where Naming Files only says not to create such names; a seek before the start failed with 131, a code the SetFilePointerEx page never names. The sharing table, by contrast, matched the documented rule on all 144 pairs.

**Finding (#27):** for an operating-system function, the documentation is a hypothesis about the function, and the probe's rows are the reference, as the CPU is for an instruction (#25). The contract keeps both: documented evidence quoted, observed rows as acceptance, and each disagreement written into its open questions so it is not rediscovered.

**Kit change suggested:** a contract built from observation lists each documented statement the observation contradicts or extends.
