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
| H5 | A lesson reaches new code only as a check in code; a lesson held in prose recurs. | supported | 2026-10-28: the Emits rule (126, 131, 162), multi-line edits outside the Edit tool (128-158), a skipping judge (149, 154) recurred in prose; Signal Reports (a gate since 127) and planted samples (read from 162) have not. |

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

### 2026-10-10 — Six red pushes behind green gates

**What happened:** Sprint 042 made substrate an editable path source pointing at `./substrate`, a directory `.gitignore` excludes. Every local gate passed; CI, on a fresh checkout without the clone, failed `uv sync` on six consecutive pushes (38033959901 through 38036178791), and an earlier push (a7061c1) had failed a test the next commit fixed. Sprints 042 closed and 043 opened on "gates 0". The deep-dive review found it by reading `gh run list`.

**Finding (#28):** a gate that runs only where it was written checks the writer's machine, not the project. The closing evidence is the CI run on the closing commit; the local gate is the fast loop before it. Any input the local run has and a fresh checkout lacks (a clone, a download, a cache) is a pin the CI must reproduce, checked by both.

**Kit change suggested:** the dual contract's artifact pass names the CI run id of the closing commit and its conclusion.

### 2026-10-10 — Sprint 044: the model passed the rows, and only the rows

**What happened:** kimi-k2.7-code, in substrate's session topology, wrote `environment.c` for the two environment families. Both item runs accepted it after two judgments. Its file and the agent's agree on every row and differ everywhere the rows say nothing: for an empty block it allocates Windows's 4 bytes but writes one NUL, so the second unit is whatever the heap held; it answers a full arena with NULL and no last error; FreeEnvironmentStringsW(NULL) succeeds and lstrlenW(NULL) reads address 0. The agent's file stops the run on each of these, because no row says what Windows does.

**Finding (#29):** an exact-rows contract specifies the rows and nothing between them. A candidate written to pass the judge fills the silent places with whatever passes; a candidate written to the contract's spirit refuses them. Both are accepted. The difference is visible only by reading the files side by side, so each family's silent places must be either probed (a row) or declared (the guest refuses, and a test shows the refusal).

**Kit change suggested:** a family's contract lists its known silent places (inputs no row covers); the item run plants each and asserts the candidate refuses it rather than answering.

### 2026-10-10 — Sprint 044: bounds that skip quietly

**What happened:** the guest runtime counts each service call by API id, `if (api < W2M_RING_APIS) per_api[api]++`. The bound was 64. Sprint 044 added ids 64 to 68 (LCMapStringEx, the environment functions, SetUnhandledExceptionFilter); their calls ran, were served, and were never counted. The frontier's call counts looked complete. A coverage check that listed every launcher call against the probes found four services missing from the counts, though the launcher had called them. In the same sprint, a refused argument form ended the frontier as a "faulted" execution, because only the import path set the field that emits SERVICE_UNIMPLEMENTED.

**Finding (#30):** a guard that skips silently when its bound is passed turns a growth into missing data with no error. The bound must be checked where the ids are made (a test: every id below it), and a stop must report what stopped it on every path that can stop, not on the first path written.

**Kit change suggested:** every fixed-size table indexed by a growing enumeration has a test holding the enumeration under the bound; every exit path of a run is enumerated in one place that maps it to its signal.

### 2026-10-10 — Sprint 046: the gate's headers are not the runner's

**What happened:** the local gate compiles every probe against mingw-w64's Windows headers; the runners build with clang against Microsoft's Windows SDK. Five runner rounds failed on differences the gate could not see: rpcndr.h's `#define small char` (twice, in two probes), IMAGE_TLS_DIRECTORY64's Characteristics inside a union (-Wmissing-braces), GetVersionExW's deprecation (-Wdeprecated-declarations), and `_fltused`, which a no-CRT program using floats must define for MSVC's linker. Each round costs a dispatch and minutes; none of the five was a question about Windows.

**Finding (#31):** a gate that stands in for a remote build is only as good as its likeness to that build. Where the two toolchains differ, the gate must either reproduce the remote's definitions (the gate now defines `small` and `hyper` as rpcndr.h does) or the remote must be the gate.

**Kit change suggested:** for each class of remote-only failure, add the cheapest local reproduction to the gate the first time it costs a round, and record the class in the diary.

### 2026-10-10 — Sprint 046: "Intel" is not a reference

**What happened:** RCPPS and RSQRTPS are approximations the SDM bounds and leaves to the implementation. Measured exhaustively, every CPU's results have the same shape (the mantissa's top 12 bits decide; exponents scale exactly), but AMD's EPYC 7763, 9V45 and 9V74 share one table set and Intel's Xeon 8573C and 8370C another. A case set judged on "an Intel runner" would have been judged against whichever Intel model the draw gave; the dispatcher now waits for a named model (`--cpu`).

**Finding (#32):** where the architecture leaves a result to the implementation, the reference is a CPU model, not a vendor; the evidence must name the model and show which other models agree before the rows stand for more than one.

**Kit change suggested:** every contract whose rows come from implementation-defined behavior names the models it was observed on and the ones it was not.

### 2026-10-10 — Sprint 046: closed on red CI, again

**What happened:** Review 2026-10-10 D1 found sprints closed on local gates while CI was red, and the rule "close on green CI of the closing commit" was adopted. Sprint 046 then broke it: three loader tests read the game's depot, which exists only on the developer's machine, and CI failed on 40 runs in 80 minutes. Nobody read a CI conclusion until the closing run, which GitHub refused for billing, and that refusal was reported as the only obstacle.

**Finding (#33):** a rule that is checked only at the close is checked too late, and a blocked check hides the failures behind it. A test that reads an input CI cannot have is a local test whatever its marker says; the inputs belong in the repository (here, the probe binaries already in `store/binaries`).

**Kit change suggested:** read the CI conclusion of each push before the next push, and before reporting a blocked run as the only obstacle, read the last run that did start.

### 2026-10-10 — a summary that counted what was not there

**What happened:** A full rerun of the class runs printed "231 of 231 classes accepted". ADD_LOCK's record held one COMPARISON_COMPLETED (the model's), a ProducerFailed from the VM judge (`vm_monitor: vset -> []`, the monitor process gone), and no CANDIDATE_ACCEPTED. The summary counted a class accepted when every verdict present conformed, so a missing verdict passed; the index test caught it. A rerun of that one class then rewrote the whole index with one entry. The monitor's death recurred in none of 1,155 further runs.

**Finding (#34):** a summary must read the decision from the record, not infer it from what is present; absence is a result. A tool that writes a shared index from a partial run must merge, not replace.

**Kit change suggested:** every tool that prints an acceptance count derives it from the record's decision event, and its test plants a judge failure.

### 2026-10-10 — Sprint 047: an exit status that said nothing

**What happened:** `family_run.py --run` and `--plants` returned 0 whatever the verdict and whatever a plant changed. Loops written as `family_run ... || echo FAIL` reported nothing on eleven reruns; the index test caught what mattered. The processor-topology family's first run, a build error, printed empty verdicts and exited 0.

**Finding (#35):** a tool's exit status is part of its contract; a tool that prints a verdict must exit by it, or every script and loop above it inherits a false success.

**Kit change suggested:** a test for each run tool that plants a failure and checks the exit status.

### 2026-10-10 — Sprint 047: a host fact coarser than the guest's unit

**What happened:** the file-attributes family failed one row on one run: creation time after access time. Python 3.12 gives a Mac file's birth time only as float seconds, about 120 ns off at this date; FILETIME counts 100 ns. The bridge now reads st_birthtimespec from libc and checks the layout against os.stat.

**Finding (#36):** a value carried from the host to the guest must keep the guest's resolution end to end; a relation the guest documents (creation no later than access) breaks when one side is rounded.

**Kit change suggested:** for each host value a contract compares, name its host resolution beside the guest's.

### 2026-10-10 — Sprint 048: a probe's own fault read as Windows's

**What happened:** the temp-path probe died with 0xC0000005 before printing a row. Writing each row as it ended showed the last row before the fault, `clear`, and a commit blamed GetEnvironmentStringsW on an emptied environment and changed the probe around that guess. The fault stayed. The cause was the probe's: it deleted at most 64 variables in one pass, a runner holds more, and the next row printed the rest into a 4 KB buffer. Four runner runs went to it (38098431870, 38098511094, 38098565802, 38098627044).

**Finding (#37):** when a probe faults, the probe is the first suspect; a row writer bounded by assumption about the runner's state is a probe defect waiting for a runner. A cause goes into a commit message only once a run has confirmed it.

**Kit change suggested:** probes write each row as it ends and bound every buffer they fill from the machine's state.

### 2026-10-10 — Sprint 048: a plant whose reach the row cannot see

**What happened:** the plant that tries TEMP before TMP named ten rows; nine changed. `temp.size_zero` prints only the size needed, and `C:\t\` is as long as `C:\m\`, so the fault cannot show there. `family_run` failed the plant, as it should.

**Finding (#38):** a plant's named rows are a prediction about what each row prints, not about what the fault touches; a row that prints a length or a flag is blind to a fault that keeps the length or the flag.

**Kit change suggested:** when naming a plant's rows, read each row's printed fields, not only its call.

### 2026-10-11 — Sprint 049: a citation written from memory

**What happened:** a commit named run 38113744018 for a probe run whose id was 38113744326, and two documented() entries carried Microsoft Learn page dates (2023-02-08, 2021-01-07) that the pages do not show (2018-12-05, 2025-07-14). All three were written from recall, read like facts, and were corrected the same day. Every page date since came from fetching the page.

**Finding (#39):** an identifier or a date in a record is evidence only when copied from its source in the same turn; written from memory it is a guess dressed as a citation.

**Kit change suggested:** a record field that cites a run or a page is filled by a tool call's output, never typed.

### 2026-10-11 — Sprint 049: a sprint that outgrew its card, and a decision left unwritten

**What happened:** the user decided mid-sprint that the guest CPU reports AVX. The work followed at once (two case sets, 131 classes), but BLACKBOARD kept the earlier v2 entry as the last word on the CPU until the sprint's end, and the card was never amended: three families and the AVX sets ran past the card's done criteria under a card that did not name them.

**Finding (#40):** a user decision changes the record in the turn it is made, before the work it starts; a sprint whose done criteria are met closes, and the work the frontier shows next opens a new card.

**Kit change suggested:** at each frontier run, compare the stop with the open card's done criteria; when they are met, close before continuing.

### 2026-10-11 — Sprint 050: undefined in the manual, fixed on the CPU

**What happened:** the SDM leaves a 16-bit SHLD or SHRD with a count over 16 undefined. The model first wrote 0 there; the Xeon 8573C gave the same non-zero value in all three runs, and a rule fitted from six cases (shift the 48 bits dst:src:dst) then held for every SHLD and SHRD case, register and memory. A count of 0, which the SDM says changes nothing, still zero-extends a 32-bit register on the Xeon.

**Finding (#41):** "undefined" in a manual means the vendor promises nothing, not that the part varies; a program that ran on the part depends on what it does. The reference CPU's repeated answer is the specification to model, and the fit is written down with the cases that fixed it.

**Kit change suggested:** a semantics judge reports undefined-per-manual differences apart from defined ones, so the fit is a decision, not an accident.

### 2026-10-11 — Sprint 051: a call with no visible effect, missing for five sprints

**What happened:** w2m's loader has called each DLL's DllMain at process start since Sprint 046 and never its TLS callbacks. Every family that loaded a DLL passed: the static_tls probe checked TLS blocks and indices, and the C runtime's own TLS callback (`__dyn_tls_init`, in UnityPlayer.dll and mono too) has no effect a row printed. The threads probe's DLL logged each TLS callback and DllMain call, and its first row in w2m lacked the callback.

**Finding (#42):** a loader is judged by the calls it makes, not only by the state they leave; a call whose effect no row reads is invisible to every state check. A probe that logs the calls themselves (kind, reason, thread) catches the missing one at once.

**Kit change suggested:** for each documented callback sequence (loader, thread start, exit), one probe that records the sequence of calls, not their results.


### 2026-10-11 — Sprint 053: a done criterion set at a trace position without reading the trace up to it

**What happened:** Sprint 052 closed partial and moved its criterion (the frontier past UnityPlayer's CoCreateInstance calls, Windows trace seq 1705) to Sprint 053, on the reading that mono's RaiseException was the one thing in the way. 053 caught the exception and the frontier stopped at seq 1524, on the first of 20 unserved functions (the version check, the display, the game window) that the held trace had listed all along. One query over the trace, run at 052's close, would have shown them.

**Finding (#43):** a criterion stated as a position in a held trace ("past seq N") is a claim about every call before N; it is checked by listing the unserved calls between the current stop and N before the criterion is written, not by running to the next stop.

**Kit change suggested:** a frontier criterion carries the list of unserved functions between the current stop and its target, produced by a script over the reference trace, in the card.

### 2026-10-11 — Sprint 054: a reference run in another mode than the run it judged

**What happened:** two game runs were captured on the runner in Sprint 049, one in batch mode (`-batchmode -nographics`) and one in play mode. Every frontier criterion since Sprint 052 was read from the batch run's trace, while `scripts/run_frontier.py` started the game with no arguments, which is play mode. The two traces agree call for call up to seq 1538, so nothing showed until the window: the play run creates its main window and loads Direct3D; the batch run creates a hidden window and never does. Reading both unity.log files at the start of 054 found it (KIT_DIARY #43's listing used the batch trace only).

**Finding (#44):** a reference trace judges a run only when both were given the same inputs: command line, working directory, environment. The inputs belong in the record beside the trace, and the run that is judged reads its inputs from there.

**Kit change suggested:** a reference capture stores its process inputs (arguments, directory, environment) as a file the local run loads, never retypes.

### 2026-10-11 — Sprint 054: the reference process ran another exe

**What happened:** the game captures of Sprint 049 replace the game's exe with jit_capture.exe, which loads UnityPlayer and calls UnityMain itself. The exe a process runs decides some of Windows's answers: its manifest (the version Windows reports, DPI awareness, Common Controls 6) and its resources (the icon UnityPlayer loads, id 103). jit_capture.exe had neither, so the reference trace shows LoadIconA failing with 1812 and an unmanifested process, where the real game gets an icon and version 10.0. The game's exe was in the depot all along; reading its resources at the start of 054 showed the difference.

**Finding (#45):** a capture tool that stands in for a program changes the program's inputs; every property Windows reads from the image (manifest, resources, subsystem, PE flags) must be carried over, or the capture's answers for them are the tool's.

**Kit change suggested:** a stand-in launcher copies the original image's manifest and resources at build time, and the capture records a hash of what it carried.

### 2026-10-11 — Sprint 055: a planted fault that reached half the system

**What happened:** the processes probe starts itself as a child, and its rows hold both processes' answers. The first plant run caught the parent's plants and changed no child row at all: the family runner built the parent's VM from the planted runtime, while the child's VM, built by the kernel's spawner, used the runtime as committed. A plant on child-side code would have passed with the fault in place.

**Finding (#46):** a planted fault is judged only where the system under test runs the planted build; when the system starts more of itself (a child process, a worker, a plugin host), each part must be built from the same planted source, or the plant tests nothing there.

**Kit change suggested:** a plant run asserts that every process or module it observed was built from the planted source (a hash in each part's record), and fails otherwise.

### 2026-10-11 — Sprint 054, reviewed: rows that covered the game's inputs, code that took wider ones

**What happened:** an outside review of Sprint 054 found seven defects, all reproduced: a stack overflow when a short search pattern matches a long name, a directory listing parsed after a transfer that did not complete, unchecked heap results, registry names narrowed from UTF-16 to char so other names opened Software, the app mount missing from its parent's listing, class names cut at 63 units, and MonitorFromWindow ignoring the window's position. Searching for the class-name pattern found an eighth (window titles cut at 127). Every probe row passed, because the probes fed the game's own inputs: short ASCII names, an unchanging directory, windows on the screen. The project's rule is that an input the rows do not cover stops the run by name; these paths accepted such inputs and answered wrongly instead.

**Finding (#47):** a probe that copies the program's inputs proves the implementation on those inputs only; every input the code accepts without a row must either get a row or stop the run, and fixed buffers, narrowing conversions and unchecked host answers are where silent acceptance hides.

**Kit change suggested:** each family's probe carries boundary rows (the longest name, a non-ASCII name, a changing input, an out-of-range geometry) for every input the implementation accepts, and a gate test rejects the patterns that accept silently.

### 2026-10-11 — Sprint 057: planted faults whose text had gone

**What happened:** each plant names a piece of source text and its replacement, and a plant run asserts the text is present before it runs. Only a local plant run checked that, and a sprint ran the plants of the families it worked on. Running every family's plants in one sweep stopped on six whose text had been rewritten (a ternary for a constant, a helper for a TEB offset, a refactored wait): those plants had tested nothing since their files changed, while every gate passed.

**Finding (#48):** a check that runs only when someone asks for it decays silently; the cheap part of it (the plant's text still present once) belongs in the gate that runs on every push, and the expensive part (the plant run) in a sweep whenever the candidate changes.

**Kit change suggested:** the gate checks every planted fault's anchor on each push, and a change that makes a family's candidate stale reruns that family's plants with its item run.

### 2026-10-11 — Sprint 058: a planted fault in a module the run had copied names from

**What happened:** a host plant replaces a Python module's source, runs, and restores it. It repoints the names other w2m modules imported from the patched one, and only those. The family runner imports `Machine` and `spawner` from the kernel itself, so the two plants on the kernel changed a module the run never called: both reported 0 unequal rows where they named 1 and 2. The plant run said "NOT as named", which is how it was found; a plant whose named reach had been empty would have passed.

**Finding (#49):** a plant reaches the code under test only through the references the running program holds; patching a module is not patching the copies of its names that callers took at import. Every holder of a reference, the runner included, must see the planted object, or the plant is checked against code it never touched.

**Kit change suggested:** a plant run asserts that the planted code ran (a counter or marker in the planted text), not only that rows changed, so an unreached plant fails even when its named reach is empty.

### 2026-10-11 — Sprint 060: a judge the writer had not seen, and how much of its first verdict was the judge's own

**What happened:** the frontier recorded the game's calls through UnityPlayer's and mono's imports, as the Windows capture records them, and compared them call by call with the game's Windows run. The first comparison reported 790 divergences. Before any relation was changed, each class was traced to its cause, and most were the capture's conditions, not the game's:
- **The window.** The capture hooks after the DLLs' start and opens at UnityMain.
- **Labels.** Its hooks label calls by the slot they pass through, and a pointer resolved before the hooks is never traced.
- **A thread of its own.** The capture's monitor thread makes only DllMain thread-attach calls.
- **The runner's paths.** The game was unpacked at `D:\w2m\game`, one character longer than `C:\w2m\app`.
- **A parser defect.** The last error is printed in decimal and was read as hex.

What remained was five defects in families that every row had accepted: FLS shared by every thread since threads arrived, empty time-zone names, a wrong EnumDisplayMonitors answer, LoadIconA succeeding where Windows fails, and CreateWindowExW returning NULL. A sixth difference was a value w2m chose, one processor, which changes how many worker threads the game starts.

**Finding (#50):** an independent judge finds what the writer's rows cannot, but its first verdict mixes the judge's own conditions with the subject's defects. Reproduce the judge's conditions (its window, its labels, its threads, its machine's paths) until each remaining divergence has one cause in the subject, and never widen a relation to absorb a divergence whose cause is unknown. Five real defects came out of 790 divergences only because no class was waved through.

**Kit change suggested:** a comparison against an external reference records, for each class of divergence it rules out, the cause in the reference's setup and the evidence for it (a line of the capture's source, a path in its strings), the way a plant names its reach.

### 2026-10-11 — Sprint 061: a reference whose process is not the program

**What happened:** the comparison with the game's Windows calls said w2m's LoadIconA(module, 103) answered an icon where Windows failed. The game's exe has icon 103. The reference process was not running the game's exe, though: the capture runs the game through its own launcher under the game's name, and the launcher had no icon. Two other reference conditions came to light the same way. The runner's paths and user differed from w2m's. The capture keeps a function's first 32 calls over all threads, so which threads' calls it holds depends on the schedule. Each was fixed at the reference or in the judge, and the remaining divergences were w2m's own: FLS shared by every thread, empty time-zone names, and one processor.

**Finding (#51):** an instrumented reference is a different program from the one it stands for wherever the instrument replaces part of it: its module, its paths, its sampling. Before a divergence is charged to the subject, ask what the instrument put in the subject's place, and make the instrument carry what it displaced (the exe's resources, the install path, the per-thread prefix).

**Kit change suggested:** a reference capture lists what its instrument substitutes for the program (the launcher, the hooks, the sampling cap) and, for each, how the comparison or the capture restores it.


### 2026-10-13 — Sprint 068 item B: probes that passed with the fault in place

**What happened:** a thread switch now moves XMM0-15, FPCR and FPSR through per-thread slots in shared memory instead of two text-pipe round trips. The first test ran the stored `threads` and `running_thread_context` probes and required Windows' rows. It passed. A planted fault that dropped the incoming thread's vector load also passed: neither probe holds a vector value across a switch, so a switch that loaded the wrong vectors changed no row. A second test, driving the swap inside a running VM with distinct values in two threads, failed under the plant and under a dropped save.

**Finding (#52):** a fixture tests only the state it carries across the event under test. The probes exercised switches by the dozen and never carried a vector through one. This is Addendum A3 (substrate-ui, the mirror-symmetric blinker) on a second project: a fixture symmetric in the bug's dimension cannot see the bug. Two projects now support it.

**Kit change suggested:** promote Addendum A3 into `TECHNIQUES.md` Section 1 under Testing: a test of a state-moving operation (a switch, a copy, a save) holds a distinct value in the moved state across the operation, and a planted fault that drops the move must fail it before the test counts.

### 2026-10-14 — Sprint 068 items L and D: a judge at the wrong grain, and plants no fixture reached

**What happened:** flag liveness drops the flags no later instruction reads, a decision made across instructions and
across blocks. The card asked the class runs to catch a live-flag plant. The class runs compile one instruction per
block, so the pass never acts there, and no plant on it could fail them. A random-program differential (the same
programs with the pass and without) judged it, and three plants failed it. Later the same day the wake keys (a signal
wakes only the threads waiting on what it signals) were judged by the thread families, and both plants on them passed
every family: no probe woke a wait on several objects by another thread's signal, the one case the plants broke. A row
for that case, run on the laptop, found a defect the plants had not been aimed at: Windows hands the event to the
waiting thread, and w2m let the setter take it back.

**Finding (#53):** a judge checks only transformations that act at its own grain. A per-instruction oracle cannot see a
pass across instructions, however many cases it holds, and a plant on that pass proves nothing there. Match the
judge's unit (one instruction, one block, one program) to the transformation's before naming the judge in a card.

**Finding (#54):** a plant that changes no row is first a question about the fixtures, and the answer can be a defect.
Both blind plants pointed at the same missing case; the row written for them failed on unplanted code. #52 said a
fixture tests only the state it carries; this is the same lesson found from the plant's side, and worth acting on the
day the plant comes back blind, not recording as a blind spot.

**Kit change suggested:** the sprint card's observation contract names each check's grain beside its plant
(per-instruction, per-block, whole-program), and a plant that comes back blind opens a fixture task in the same sprint.

### 2026-10-14 — Sprint 068: a clock that counted wrong, found by a comparison that changed

**What happened:** a commit of native lowerings (each judged against the Xeon's cases, each planted fault caught)
turned a conforming batch run into two divergences. The lowerings computed nothing wrong: a differential of 6,018
blocks agreed. They changed when the game's threads interleaved, because the model steps they replaced had been
counting part of their block twice and moving the instruction clock. Looking at how the count is kept turned up a
worse defect in code from the same day: the host computed the count from a limit the slice stub had since raised, so
refills vanished from it and the guest's clock could go back. The rates the day had reported since the stub were short
by about three times.

**Finding (#55):** when a change that is right by every per-instruction judge moves a whole-program comparison, the
change has moved a shared quantity the program reads (here time), and that quantity's bookkeeping is the next thing to
audit, before the comparison is tuned or the value excluded. A clock run at three nearby rates told value from timing
in three runs; the audit it prompted found two counting defects no judge of values could see.

**Kit change suggested:** an observation contract that compares whole-program behavior states the shared quantities
the run's values depend on (clock, schedule, address layout) and how a run varies each, so a moved comparison has a
named first question.

### 2026-10-14 — Sprint 069: a switch that never ran, shown by its own counter

**What happened:** thread switches moved into the VM, judged by running five threaded families with the switch on and
off and requiring equal rows and equal per-thread instruction counts. Every check passed, and the batch run conformed.
The batch run's progress line also printed a new counter, the switches the VM had made, and it read 0. The game
starts a second process (its crash handler), and the switch had been turned off for any run with two processes, a
guard copied from a rule that needed only the other process to be idle. Nothing failed: the host made every switch as
before, so the comparison had nothing to catch.

**Finding (#56):** a judge that requires the new path to equal the old one passes as well when the new path never
runs. An equivalence check needs a reach count beside it: how often the new path ran on the workload that matters. The
counter cost one line and was the only thing that showed the batch run had not changed.

**Kit change suggested:** a card that moves work from one path to another names, in its observation contract, the
counter that shows the new path taken on the target workload, and the census reports it.

### 2026-10-14 — Sprint 070: a speed census that compared unequal work

**What happened:** Sprints 069 and 070 each measured a change by the batch run's wall time before and after. The run
stops when its log holds the reference's 168 lines and its main thread its calls, and the frontier checks that only
when the progress line's 30-second wall-clock timer fires. A faster run therefore executes more instructions before
it stops: thread 4 ran 72.8 billion instructions in one run and 85.4 billion in the other. The exit counts and the
time by exit came from the progress line, which stops at its last tick. The two runs also differed in hand-overs and
switches, which the first write-up offered as a cold-against-warm difference in interleaving, unchecked. At a fixed
instruction count the interleaving was identical in every run and every configuration, and the exit counts matched
to the last exit.

**Finding (#57):** a speed comparison holds only at equal work. A stop that a wall-clock timer samples makes the work
depend on the speed being measured, and the figures read off that timer cover part of the run. Fix the work (a fixed
instruction count), read whole-run figures, run each side twice, and show the spread: here two identical runs
differed by up to 6.9 s.

**Kit change suggested:** an observation contract that measures speed names the fixed amount of work, the stop that
ends it, and the repeat count; a census that does not show its stop reason is not a census.

### 2026-10-14 — Sprint 076: a judge drawn from the same generator as the work

**What happened:** the stencil table derives each key's stencil from instances made by changing one recorded
instruction's fields, and the first judge was a second set of instances made the same way with another seed. That
judge passed 98.2% of its instances. The record's own instructions, rebuilt from the same table, differed on 133,748
of 1,186,511: wherever the instance maker never changed a field (a displacement whose class its moves always left, a
register in an opcode's low bits, equal registers it could only change one at a time), the argument became a
constant, and the judge's instances, sharing the same unchanged field, agreed with it. Eleven causes were then found
one by one from the record's failures, each fixed in the instance maker, until both judges showed 0.

**Finding (#58):** a held-out set is held out only from what made it differently. Instances drawn by the generator
that made the training set share its blind spots and confirm them. The judge needs data the generator did not make:
here the key's recorded calls, half kept out of the derivation.

**Kit change suggested:** a card whose output is learned or derived from generated samples names a judge drawn from
outside the generator (recorded data, a different generator), and reports the two judges' results side by side.

### 2026-10-15 — Sprint 079: a held-out judge held out only in name, twice

Sprint 076's lesson (#58) moved the judge from the instance generator to the record. Sprints 077 and 078 then judged
the C over that same record, which the table had been derived from in part, and reported 0 differing over 1,186,511
calls and 1,163,021 blocks. Both numbers were true and neither tested what mattered. A red-team pass measured
something else (how many flag variants collapse) and found seven groups disagreeing where they could not: refined
SSE-shift branches zeroing a fixed register, a CALL's return address and a JNZ's target written as constants. The
class case sets, which no step of the table's making had touched, then showed 144 wrong compiles in seven classes on
the committed table, and 0 once every value the samples never varied guarded the constants it could stand for.

**Finding (#59):** half the record kept out of the derivation is held out from the derivation, not from the record's
blind spots: a value the run never varied is unvaried in both halves. A judge is independent only if its data came
from a different source than everything that made the work: here, cases written to sweep operands, run on Intel.

**Kit change suggested:** a derived artifact's card names each judge's data source and the work's sources, and
requires one judge whose source is none of them; where a value never varied in the work's sources, the artifact
refuses inputs that vary it (a guard) rather than generalizing.

### 2026-10-15 — Sprint 082: a planted fault that could not fail

The card named its plant before the code existed: the scan reads an instruction's NZCV kind from the instruction
itself instead of the one before it. Both judges passed the planted C unchanged (4,000 record scans equal, 7,108
spec scans equal). The plant was not missed; it changed nothing a scan returns. A Jcc's flags differ by kind only when
it branches on its setter's NZCV, and every setter writes the flags its Jcc tests, so the scan has decided them before
it reaches the Jcc. The replacement plant (the scan steps past a jump instead of following it) fails both judges: 623
of 4,000 record scans and 203 spec scans short or differing.

**Finding (#60):** a plant chosen before the code is a guess about where the code can be wrong. A plant that the
judges pass is either a blind judge or an unobservable fault, and the two look the same until the fault's reach is
measured; here the tables showed the kind cannot change a result.

**Kit change suggested:** a card names the behavior its plant must change, not only the line it edits; a plant the
judges pass is reported with the measurement that tells a blind judge from a fault with no effect, and replaced by
one that changes the named behavior.

### 2026-10-16 — Sprint 084: a guard that checked the wrong witness

The reviewer asked that the runtime stack's invariant (no runtime frame live while guest code runs) be checked. The
check I wrote compared SP with the stack's top at every dispatch miss and stopped the run otherwise. It stopped the
translation-cache test, a correct run: an exception the runtime resolves resumes the guest through
`w2m_resume_context`, which never returns, so SP sits inside frames that are dead. SP is a witness of where frames were
pushed, not of whether any is live. Writing it did find a real gap: a run's vCPU had no SP at all before its first
bridge.

**Finding (#61):** a guard must check the property, not a value that coincides with it on the paths in mind. Before a
guard stops runs, list the paths that reach it (here: the run's start, a bridge's return, an exception resumed) and
measure the value on each.

**Kit change suggested:** a card that adds a run-stopping guard names the paths into the guarded point and the value
measured on each, beside the plant's reach (#60).

### 2026-10-16 — the frontier became the method: rediscovering what the files said

The user, 2026-10-16: "What is supposed to be a clever way of finding things at the frontier has become the way of
finding the whole thing. It's become a way of rediscovering things that can already be known just by asking." The
user had said the same on 2026-10-10, and a memory held it. Measured on 2026-10-16: Hail to the Rainbow's 7 x86-64
images declare 633 Windows imports in their import tables, which a script reads in seconds; w2m serves 238, written
across some forty sprints that took their work from where the last run stopped. The compiler inside the VM (Sprints
074-084) was inferred from one batch run and covers 484 of the decoder's 1,067 rows. BioShock 2's images are all
32-bit, which w2m cannot read, and no sprint had looked at them. A census of Sprint 044 had already listed
UnityPlayer's instruction classes, and it was not re-run.

**Finding (#62):** a search loop (the frontier) answers only "what stops next", and a project that plans from its
answers rebuilds by hand the inventory its inputs state. The remedy is not more care at each stop: it is a census of
every knowable surface, committed as data, before any loop runs, with the loop's stops counted as the census's misses.

**Kit change suggested:** a project's first architecture sprint, and every phase start, writes the knowable surface of
its inputs (formats, interfaces, instruction set, declared dependencies) as committed data with coverage beside it;
a sprint card names the census rows it closes; a sprint that found its work at a run's stop says why the census did
not show it.

### 2026-10-16 — Sprints 094-096: a speed measure that the program itself paced

Roadmap step 1 measured the batch run (seconds and instructions to the stop) and step 2 was judged by it. Sprint 095
cut the wake path's re-blocks from 11,086 to 58 and Sprint 096 cut the spinning that waited on other threads, PAUSEs
from 508-608 million to 62.7 million, and the instructions to the stop barely moved (81.3 to 80.4 billion). The rest
of the main thread's work was a busy-wait on QueryPerformanceCounter (62.9 million calls, one per PAUSE) with every
other thread blocked without a timeout. On the instruction clock the game waits for guest time, so the run covers about
57 guest seconds whatever w2m's speed, and Windows itself took 90 to 102 s to the same log line, against w2m's 38-39 s.

**Finding (#63):** before a run is made the gate for speed, ask what bounds it: the work, or the program's own clock.
A run that waits for time measures the clock, and no speed-up shows in it. The check is cheap: the share of the
dominant thread's instructions in a spin, and the clock calls beside its PAUSEs (one query of the end report here),
then the reference's own time to the same point.

**Kit change suggested:** a speed sprint's observation contract names what bounds the measured run (work, I/O, the
program's clock) with the measurement that shows it, before any optimization is judged by it.

### 2026-10-21 — the user's decision to rewrite in Rust: a design never tested against its goal

Sixteen sprints (107 to 122) built a host for several vCPUs in Python: one host thread per vCPU, a runtime lock, kicks
and wakes. Standard CPython 3.12 runs one thread's Python at a time, and the goal the plan states in its own first
section is about 37,600 system calls a second. No sprint measured the design against that number before building on
it. The user asked whether this was being done in Python, then directed a postmortem
(`docs/postmortem-2026-10-21.md`) and a rewrite in Rust only (BLACKBOARD, Decision 2026-10-21, user). Every
sprint's gates passed; each judged the sprint in front of it, and none judged the design against the goal.

**Finding (#64):** a gate that judges each sprint's work does not judge the design the sprints share. A design that
names a goal in numbers needs a feasibility check against that number before the first sprint builds on it, and the
check names the platform constraints the design depends on (here, one interpreter lock).

**Kit change suggested:** an architecture-band card that opens a chain states the chain's goal as a number and the
platform constraints it rests on, with a measurement or a cited source for each, before the chain's second card runs.

### 2026-10-25 — Sprints 145-149: a judge that passed on nothing, and a revisit that fell due unread

Sprint 145's case runner first passed with 0 cases judged: every compile was refused, and the only assertion was zero
difference classes. The sprint printed the counts and called the observation resolved, but the test still returned
passing when its fixture, its results or clang were missing, and fell back to a compiler measured to compile none of
the cases. CI, on Linux, never compiled the test, and every close recorded "CI run success" without saying so. The
link-3 review (reviewer-2, 2026-10-25) found it, with a second fault: the design's revisit condition for link 3 fell
due at Sprint 148's close and no sprint measured it. Sprint 149 measured it (235 of 249 rule modules reached).

**Finding (#65):** a differential test's zero is evidence only beside its denominator. A judge that can skip, refuse
or fall back must fail on each of those in the gates, and must assert the counts it judged, not only the differences
it found.

**Finding (#66):** the stencil table's key carries the code address's value class, so a compiler fed addresses from
one range (the cases at 0x2000_0000) can refuse everything another range (the game's, above 4 GB) taught it. A
design's assumption about a component's coverage is a number to measure at the design's own inputs.

**Finding (#67):** an unoptimized test build turned a per-case reset of 1,048,576 entries, one call a word, into over
an hour of gates; a release run took 33 s. A per-case loop over a large table is measured in the build the gates use.

**Finding (#68):** a design's revisit conditions fall due at named points; the close of the sprint at that point
states each condition's measure and the verdict, or the link does not close.

**Kit change suggested:** the close of a sprint that ends a design's link lists that link's revisit conditions with
their measured values.

### 2026-10-28 — Sprints 150-164: a step's gate met six sprints before its text, and lessons that recurred in prose

Step 3 of `docs/rust-port-plan-2026-10-24.md`, the walking skeleton, names a gate ("the guest's output and record
equal the stored ones") and three measurements ("an exit's time, the runtime word's hold times, whether a guest WFE
exits"). Sprint 158 met the gate: `w2m-vm` ran `sleeper` to ExitProcess(0) under substrate, and its 40 events equalled
the frozen Python's but for three exempt fields. The measurements stood where Sprint 142 left them, one exit sampled
at 3,667 ns, and no Rust code recorded a measurement. Sprints 159-161 lifted three more translator groups under link
5, whose purpose (letting `qpc` and then `sleeper` run further) had ended at Sprint 157. reviewer-2's review of
2026-10-27 named both; Decision 2026-10-27 closed link 5 and amended the step. Sprint 162 added the seven metrics
(vocabulary 0.11, P24); Sprint 163 measured an exit's time by kind (served exits' medians 584 to 1,042 ns, beside the C
monitor's 813-852 ns) and the WFx traps (WFE 0 of 1,000, WFI 1,000 of 1,000); Sprint 164 measured the runtime word.

Step 3's synthesis: the step met its gate at Sprint 158 and its text at Sprint 164, six sprints later. Its numbers do
not reopen the host design. The switch stub holds the word 83 ns at p50 and p99 over 3,018 takings
(`store/runs/w2m-vm-measurements-1793171817000000000`), inside the design's 200 ns; the long holds are the compile
site's (to 246 µs) and the bridges' (to 57.5 µs), which the design's step 6 changes remove; every wait but each run's
first is 4 ticks or less, and the first (163 and 179 ticks, no exit inside) is not explained. Sprint 164's own
instrument went through three forms. A 48-entry ring, sized without a count of takings, lost 2,914 of 3,002. Histograms
for all four sites in one, topping out at 5.3 µs below the design's 10 µs revisit point, with a lower bound carried in
prose and counter reads without an ISB, went to reviewer-2's second review. An entry per taking with its site and the
exits around it, in 1.3 MB the monitor region left unused, came third.

**Finding (#69):** a step's gate is one sentence of its text, and a step that meets its gate looks done. A close that
ends a step reads the step's whole text against the records, sentence by sentence, as #68 asks of a design's revisit
conditions.

**Finding (#70):** an instrument's capacity and range are numbers the sprint measures or derives before sizing it: the
ring needed the takings a run makes (3,002 on `sleeper`, which one counted run gives), and the histogram's range needed the design's own
threshold (10 µs, in the design's text). Both were in reach before the code was written.

**Finding (#71):** an order set by a Decision changed in a Surfaced disposition (the code review's of 2026-10-28 moved
the locked XCHG ahead of KIT_DIARY and the census card), and only the next review caught it. A disposition that
changes the order is a Decision, written as one.

**Hypothesis H5** (new): a lesson reaches new code only as a check in code; a lesson held in prose recurs. For it:
three rules held in prose recurred. The Emits rule ("name the plane's list") was written at 131 after 126 and broken at
162. Multi-line edits outside the Edit tool went on in 128, 131, 142, 143, 153, 157 and 158. A judge that skips (#65,
149) was written again at 154. Plants that could not fail or did not run recurred in 149, 155, 156, 162 and 163, and
zsh expansion that ran or counted nothing in 149 and 159. Two rules made into checks have not recurred: a gate has
failed a closed sprint without a Signal Report since 127 (reports had stopped at 122), and the gates have read the
planted samples' ProducerFailed event since 162. Against it: the checks are young (the Signal Report gate since 2026-10-24,
37 sprints), so their record is short. Verdict: supported.

**Kit change suggested:** a lesson the Rubber Duck Pass records as a class (`disposition: recorded`) names the check
that would catch its next instance, or says why none can be written; a second recurrence of a recorded class makes
the check the next card.

### 2026-10-28 — Sprints 178-183: a coverage claim in a card's prose, and a close status the check did not read

**Finding (#72):** H5 recurred as a coverage claim. Sprint 178's card said the translator lifts the 67 legacy classes
the model would step; it lifted 40, and the sprint closed partial. The routes were read before the card and the lift
was not. Sprint 180's card was drawn the same way, from the lowering's text, and held, but only because its count was
checked by `c_words` before the commit. Rule: a card that claims the translator lifts N classes quotes the lift's
per-class output (the C words judge's census, or the case runner's "to the model, not judged" line) before its
commit. Sprint 183's card followed it, and its result names by class the 5,114 cases the model still does not step.

**Finding (#73):** a partial close was recorded in prose only. Sprint 178's tail entry said "observation partial" under
a "closed" heading and a `status: done` card until reviewer-2's review of 178-181 caught it. Sprint 184 makes it a
check: `signal_report --check` takes a "closed-partial" heading only over a `closed-partial` card, and requires that
status of a sprint a Deferred entry tags "(Sprint N, partial)". H5 holds: the rule went into code once it recurred.

**Finding (#74):** a gate's line must name the tree it ran on. From Sprint 179 a closed entry's gates line matched a
kept run by its seconds alone, which another sprint's run of the same length could meet. From Sprint 184 `gates.sh`
keeps the sprint, HEAD and the diff's sha256 with each pass, and `--check` matches the sprint. Sprint 183 showed the
same class from the other side: its first four records were written by a `case_runs` binary that a planted fault's
test build had relinked, though the source was clean. A restore is done when the binaries the next step runs are
rebuilt, not when the source compares equal.

**Finding (#75):** a close's "CI does not run" line was written from memory six times and was wrong each time:
Sprints 176-184 said `c_words`'s C emitter is macOS only, though the test has run on CI's Linux since Sprint 154 and
says so in its module doc. The error surfaced only when Sprint 184 gave `c_words` a macOS-only call and CI failed.
The line names judges, so it is read from `.github/workflows/ci.yml` and the last CI log, not recalled; a judge that
calls a macOS-only tool is gated by `cfg` in the same edit.

**Finding (#76):** two of Sprint 184's checks held less than #73-74 say. The gates line records HEAD and the diff's
sha256, and `--check` compares neither, so #74's "a gate's line must name the tree it ran on" is written, not read; and
the pass file is untracked, so a fresh clone's gates fail their own check (reviewer-2, review of 182-184, Medium 2).
The zero-halfword list held a defect no runtime address can produce: `a64_mov_imm` never skips halfword 0, and the
image's halfwords 1 and 2 are never zero (Medium 1, correcting the reviewer's own earlier Medium 1, which this board
had accepted). In both, the check was built from the finding's words without reading the code the finding was about
(`a64_encoder.c` 25-32; the line that reads the columns back). A check is planted against the defect it names, at an
input the system can produce, before it is held.

### 2026-10-29 — Sprints 205-210: a reference older than the code it judged, and a pass graded past its card

**Finding (#77):** a reference agreeing with itself is not a reference agreeing with the oracle. Sprint 205 read the
mono cookie at row 924 as a Rust clock fault because the four frozen-host streams of 2026-10-17 agreed with each other;
they all predated Sprint 120's catch-up of a child at its start, and the frozen host's own run of 2026-10-21 gave the
Rust host's value. Sprint 205's "the Python makes no warp" was inferred from a formula and refuted by the exit ledger
(warp 609 on both). A reference is pinned to the oracle's commit, and a repeat is run against it before a difference is
read as the port's.

**Finding (#78):** an insert can split a Decision. Sprint 207's work commit placed the Decision "Step 4's reference
run" inside "Sequence after Sprint 205", leaving the first entry's decision and revisit clauses under the second;
reviewer-2 found it from the diff. Sprint 209 made it a check: `--check` fails an agent-resolved Decision from
2026-10-25 that holds other than one "Decision:" and one "Revisit if:". H5 again: the rule went into code at its first
recurrence the review could name.

**Finding (#79):** a grade written before the result was read. Sprint 209's tail said "observation pass" while its own
result said the arena's writes, an expected value, were logged nowhere, and its plant for that gap had been swapped
for one that reached a judge (reviewer-3, review of 205-209, High 1). `--check` reads a heading against a card's status
but not a grade against the result's departures. A departure that leaves an expected value unmet makes the close
partial, with its Deferred entry, before the grade is written.

**Finding (#80):** a rule's exit is checked, not its count. Sprint 210's first planted fault left EXECUTION_COMPLETED out,
and `check_records` printed the T3 violation and exited 0, since it dates a record by a nanosecond suffix the record's
directory lacked. A judge wired into the gates is planted against through the gates' own invocation, exit code read.

### 2026-10-29 — Sprints 210-211: an oracle's defect copied twice, a red run read as green

**Finding (#81):** the frozen host is the oracle where it agrees with Windows, not where it is wrong. Sprint 210's record
copied its way of naming an unserved request (the refusal's words as the "service"), and Sprint 211's first record
copied its way of finding the call (the word at the guest's stack top as a return address), which named
KERNEL32.dll!RtlUnwindEx at a stop no call made: the reference's thread never calls it. The user's rule of
2026-10-29: "if the python implementation is wrong, do not reproduce the wrong parts." A port step reads the frozen
host's code against Windows, the specs and the vocabulary before matching it, and names each departure.

**Finding (#82):** a result is read from the thing that holds it. A watch of CI run 40936387557 ended in `echo`, whose
exit 0 was read as the run's; the run had failed, and Sprint 210 stood closed on it while a reply said CI had passed.
WORKING_AGREEMENT already said a sprint closes on its own run's success; the tail wrote "closed" before that run
existed. H5 again: the next card puts the run's conclusion in a tracked file and the rule in `--check`.

**Finding (#83):** a substitution over a file that quotes substitutions rewrites the quotes. A `sed` that added a path
to a tail entry's Files line also matched the same text inside an earlier quoted command and changed the record of
what ran. Edits to BLACKBOARD use the Edit tool on a string unique to the target, or check the count of matches
before the write.

### 2026-10-29 — Sprints 216-218: judge work ahead of the stream, a default that classed errors, two plants one text

**Finding (#84):** work on the judges can crowd out the work they judge. Sprints 216 and 217 moved no row of the
batch run's stream, and the draft of 218 would have moved none; reviewer-3's review of 215-217 named it (High 2), and
the stream then went from 6,698 rows to all 10,505 of the reference in one card. Its review of 218 found the step-4
gate's other items (classes, 63 families, two guests, the oracle move) on no sequence at all. A sequence Decision
names the plan's gate items it leaves open, and a judge's refinement waits while its verdict already conforms.

**Finding (#85):** a conversion that defaults hides a classification. `impl From<String> for ModuleError` made every
`?` on a string a failure, so the full TLS array and the arena's refusals, which the frozen host raises as
Unimplemented, ended a run as faults with nothing at the site to say so. A typed error has no default: each site names
its kind, and the compiler lists the sites.

**Finding (#86):** two planted faults that reach one judge text do not show the judge tells them apart. Sprint 218's
wrong TLS index and its missing block both crashed the probe before it printed a row, and both read "stopped at
ExitProcess { code: 3221225477 }". Each plant names the judge text it expects before it runs, and a pair that shares
one is recorded as such, or given a judge that survives the fault.

**Finding (#87):** what the code already says is looked up, all of it at once, before a run is asked to find it.
HOST_FILETIME stopped Sprint 218's probes only after a run reached it, and the batch run's missing end showed only
after a 486.7 s run, while the frozen host's dispatch and `run_frontier.py` 284-310 named both. One pass over every
dispatch line of the two hosts then found three more unserved requests in seconds (HOST_CLOCK, HOST_UNIMPLEMENTED,
THREAD_WAKE). The user's rule of 2026-10-29: "no searching for already knowable answers that you can look up at
once". A lookup that can be a test becomes one (`host_requests.rs`); a run is kept for what only a run can show.

### 2026-10-29 — Sprints 221-222: a held run with no fingerprint, a refusal counted as a difference

**Finding (#88):** a held result needs its output held too, or it can change unseen while it waits. Sprint 221 held
80 ended family runs by the name of the row rule each waits on; reviewer-3 found nothing of what they printed was
kept. Each held row now carries the sha256 of its printed rows, and a one-byte plant was named.

**Finding (#89):** #85 again, in a list: one container for two kinds hides the classification. Sprint 222's class
judge pushed the cases it never ran into the list of measured differences, and a non-empty list meant
"discrepancy", so 19 classes nothing was compared on were reported as divergences and would have opened 19 ledger
entries (reviewer-3's High 1 of its review at 222). The vocabulary already held `inconclusive`. Two fields, and a
verdict that reads both, make the kind explicit at every reader. Reading the reader (`ledger.rs`) for the fix found
a second fault the same way: the unit it takes is the text before the first colon, and the judge's began with the
executor's name.

### 2026-10-29 — Sprint 223: a reviewer's fix path taken unchecked, an identity simpler than its facts

**Finding (#90):** a fix path a review proposes is a claim like any other, and the card checks it before it is
written in. Reviewer-3's Medium 1 at 222's close keyed the ledger's defect by its first case's program hash; card
223's draft took it, and the next review withdrew it: the first case is whichever one a judge reads first, so a new
reference run would split one defect. One lookup at drafting time, which runs each judge reads, would have shown it.

**Finding (#91):** an identity rule has to carry what makes two reports the same thing, no less. The first ledger by
defect closed an entry on any one resolution; a judge that runs one profile a session would then close another judge's
report from other runs. The interface ("a resolution closes the defect") was simpler than the facts it modelled,
judges and profiles; resolution per report, and an entry closed by its last report, keeps that complexity inside the
ledger (reviewer-3's Medium 1 of its review at 223).

### 2026-10-29 — Sprints 223-224: a trigger keyed to a directory, a gate that tested the machine

**Finding (#92):** a gate's trigger names what reads the change, not where the change sits. `scripts/gates.sh` ran the
frozen Python's 2,547 tests whenever a path under `store/` changed, so a Rust judge's ledger there cost Sprint 223 about
25 minutes of tests over Python nothing had touched; four exemptions, added one sprint at a time, had patched the same
rule. The user's answer went further, "there is no python running": a check over code that cannot change is no check.

**Finding (#93):** a correctness gate holds the code to a property the code controls. `exit_time.rs` bounded a wall-clock
figure, which the machine's load moved from 4,062 ns to 56,333 ns, and its failure stopped 152 tests behind it. A
measurement belongs in its own stage, kept with the conditions it was taken under, and judged against them; and a stage
that reads "did not run" as "slow" hides a broken test under a busy machine (reviewer-3's Low 1 of its review at 224).

### 2026-10-29 — Sprints 224-226: a dependency the file extension did not show

**Finding (#94):** a dependency is found by what reads or runs a thing, not by its file's extension. Removing the
Python, the project counted 467 `.py` files and two Rust readers of their text; it then found a third reader by
searching string literals (`case_inputs.rs`' `git log -- src/w2m`), CI's own count of 21 case files in `round_trip.rs`
after its run had failed, and, in reviewer-3's review at 226, the Python a YAML `run:` line executes and the generator
21 `probe.json` strings name. Before a component leaves, every consumer is listed whole: code that opens its files,
scripts and workflows that run it, configuration that names it, and tests that count what it produced.

### 2026-10-30 — Sprints 230-233: a judge weaker than its name, a run not started as its family starts

**Finding (#95):** a ported judge is ported whole before a count is taken by it. The Rust family judge from Sprint 196
keyed rows by name, ignored extra rows and dropped no excluded row, where the frozen `compare_exact_rows` it stood for
takes rows in order and refuses extras. Sprints 230 and 231 counted 249 conforming runs by it. Sprint 233 found the gap
by reading the frozen judge for another reason. Under the frozen rule all 249 still conformed, so no number was wrong,
but none had been shown. A judge's port needs its own plants that the weaker form passes, an extra row and a swapped
pair, before the first count.

**Finding (#96):** a run is the family's run only when started as the family starts it. 26 runs held since Sprint 221
as host stops, CreateProcessW's argument form and a temporary file, were runs given no start options: a probe run as its
own parent, a probe with no TEMP. Started as their families start, all 26 conformed. The lookup had listed the options
as a column, and no run used them. A column that names an input is a claim the runner reads it; a test should fail
when a runner ignores one.

**Finding (#97):** a count is read from structure, not from prose. `family_known` counted a contract's excluded rows by
splitting its reasons at "; ", and the reasons hold "; " too: 8 of 10 families were miscounted, `windows.file.attributes`
at 4 for 2. The names stood in the frozen source's dictionaries all along. Prose written for a reader is not a format.

### 2026-10-30 — Sprint 235: a test that gathered its faults and never asserted them

**Finding (#98):** a judge that gathers its faults must fail on them, and its plants must be run against it. The class
judge's test `the_rust_index_is_the_frozen_one_or_held` built a list of faults from Sprint 222 and ended with a
`println!`, no assertion; no class could fail it. Sprint 222's plants had reached other judges, the session and the
case-count test, so this one was never seen to fail. It was found when 17 held classes came to conform and the test
still passed. A gathering judge ends in one assertion over what it gathered, and each judge in a card's contract is
planted against itself.

### 2026-10-30 — Sprints 235-237: two cards split after their work began

**Finding (#99):** #94 applies to a card's scope, not only to a removal's code. Card 235 named "the 17 classes and the
6" and found three of the six needed a 121-line lowering; card 237 named the fixture's retirement and found the
fixture's header and sha256 in three binaries and every record's baseline. Both were split, honestly, after the work
began, on readers a search before the commit would have listed. A card that removes or retires a thing names that
thing's readers before it is committed (WORKING_AGREEMENT, 2026-10-30).
