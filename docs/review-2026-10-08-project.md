# w2m project review — 2026-10-08

Review of the w2m repository at commit 34b85f0 (Sprint 022 closed), against four criteria:

1. Does the project do what its plan, `windows_to_macos_research_direction_v0_2.md`, says it will do?
2. Does it follow the SDD discipline it adopted (`sdd-kit-3/`)?
3. Does it follow accepted engineering practice, with a named source for each standard applied?
4. Where has it gone wrong, and what is obviously missing?

## Method

What was read in full:

- **The kit:** every text file in `sdd-kit-3/`, including `AGENTS.md`, `foundations/01-04`, `grammar/`, `templates/`, `TECHNIQUES.md`, `ADDENDUMS.md`, `lib/sdd.py`, the research notes, and `example/`.
- **Project records:** the plan v0.2, `BLACKBOARD.md`, `WORKING_AGREEMENT.md`, `KIT_DIARY.md`, and `docs/review-2026-10-08-decoder-theory.md`.
- **Sprint cards:** 007, 013, 018, 020 and 022.
- **Hand-written modules under `src/w2m/`:** `contract.py`, `compare.py`, `trace.py`, `pe.py`, `x86ref.py`, `x86dec.py`, `x86ops.py`, `xedref.py`, `encgen.py`. The generated `events.py` and `x86dec_tables.py` were not read.
- **Scripts:** `check_vocab.py`, `judge_traces.py`, the three differential scripts, and `litmus_mp.py`.
- **Probes:** `probes/event_auto_reset/event_auto_reset.c` and `probes/mac_hv_litmus/mp_litmus.c`.
- **Contract and workflow:** the accepted event contract `0.2.0.json` and `.github/workflows/windows-probe.yml`.

What was run:

- Every gate in `WORKING_AGREEMENT.md`.
- A count of which vocabulary tags have ever fired in recorded data.
- 21 hand-picked edge-case encodings through w2m's decoder and the XED v2026.08.23 build in `third_party/`.
- A scan of all 38,976 generated encodings for exceptions and for prefix coverage.
- A scan of the 33 Python corpus binaries for delay-load import directories.

Not reviewed: the vocabulary rationale documents line by line, `specs/x86_decode.json` row by row, and the probes for Sprints 008, 011 and 012.

## 1. Verdict

The work that exists is mostly sound, and its gates pass:

- 125 tests pass in 3.45 s.
- `ruff check`, `mypy --strict`, `check_vocab.py`, `gen_events.py --check` and `gen_decoder.py --check` all exit 0.
- The measurement sprints caught three of their own false results before recording them.

Four problems outweigh that:

1. **The plan's central claim is untested.** After 23 sprints the project has not exercised the loop the plan rests on: contract, then generated implementation, then verification against the contract (v0.2 §1, §2, §11). No candidate has been generated, no model check has run, no discrepancy has been reduced, and no Windows instruction has executed on the Mac.
2. **The decoder became the project.** Ten sprints (013–022) went to a PE reader and an x86-64 decoder with no exit criterion. Two decisions the project recorded for that work were not carried out, and nothing was recorded to drop them.
3. **The SDD discipline lapsed from Sprint 013 on.** That work emits no signals, its cards have lost most required sections, and the append-only records were edited out of order.
4. **There are confirmed defects in code and evidence.** Section 4 lists eight, each reproduced or read at the cited line.

## 2. Does it do what the plan says?

The plan's first sentence of §1 states the goal: a Windows Steam game launched as a native Mac application from one command. Its working premise is that complete specifications can be turned into correct implementations by the project's own generation-and-verification system. The table maps each plan section to what exists.

| v0.2 section | What the plan requires | What exists at 34b85f0 | Status |
|---|---|---|---|
| §1, §4.1 Acquisition | `w2m steam install/run`; DepotDownloader path | Nothing. DepotDownloader not installed; neither chosen game's Windows depot fetched | Not started |
| §2 Two compilers joined by one contract | Specification compiler and application compiler | Neither exists | Not started |
| §3 Correctness relations | Six relations: exact, state, allowed set, partial order, numerical envelope, progress | `allowed_outcome_set` only (`src/w2m/compare.py:45`, `contract.py:63`) | 1 of 6 |
| §3.1 Clocks | Per-clock contracts | `GUEST_CLOCK_READ` tag; proposal P7 rejected; no clock contract | Not started |
| §4.2 Steam bridge | Versioned interface contract, generated adapters | Nothing | Not started |
| §4.3 Output bundle | Native Mac app bundle | Nothing | Not started |
| §5 Machine-semantics IR | Lift instructions to explicit semantics | Decoder (length, iclass, integer operands); no semantics | Front end only |
| §5.1 Code discovery and validity | Static plus run-time discovery, invalidation | Static recursive traversal (`x86ref.py`); no run-time path | Static half |
| §7 Win32/NT effects | State-and-effects contracts per API | One contract: `windows.event.auto_reset.single_signal@0.2.0` | 1 contract |
| §8 Page geometry | Guest page metadata separate from host backing | Probes on both backends (Sprint 012); no memory subsystem | Evidence only |
| §8.1–8.2 Backend capabilities | Capability profile per host | 4 KB IPA granule, guest TSO, 4 KB guest faults recorded; native TSO recorded as `absent` (see 4.6) | Done, one defect |
| §9 Concurrency | Generated litmus tests, model exploration | One MP litmus on the VM backend; herdtools7 coverage deferred | Partial |
| §10 Graphics | Shader, command, resource paths | Nothing; proposals P1–P3 open | Not started |
| §11.1 Evidence categories | Documented / observed / hypothesis | Enforced in `contract.py:83-125` | Done |
| §11.2 Contract compiler | Model tests, generators, checkers derived from contracts | None; the comparator is hand-written for one relation | Not started |
| §11.3 Repair vs revision | Two separate paths | Labels on Sprint 018–022 cards; vocabulary has `DISCREPANCY_ROUTED`; never fired | Labels only |
| §11.4 Substrate orchestration | Probes, model exploration, candidates, comparison, reduction on one record | Probe import, ingest and comparison topologies run (`judge_traces.py`) | Partial |
| §12 Test levels | Machine, system, composition, application, product | Machine (decoder only) and system (one contract) | 2 of 5, thin |
| §12.1 Application profiles | Game profile guides probes and coverage | BioShock 2 Remastered and The Blood of Dawnwalker chosen 2026-10-05; no file of either has been read | Not started |
| §14 Artifact identity | Hashes, provenance, invalidation | Content-addressed traces; contract versions as files; Mac run records not committed (see 4.7) | Partial |

The vocabulary tells the same story from the signal side. `signals/0.3.json` defines 60 tags. Across the ten committed guest traces and the eight local run records, 14 have ever fired:

> CAPABILITY_PROBED, COMPARISON_COMPLETED, EXECUTION_COMPLETED, EXECUTION_STARTED, GUEST_CALL_RETURNED, GUEST_CLOCK_READ, GUEST_THREAD_EXITED, GUEST_THREAD_STARTED, GUEST_TRACE_CLOSED, GUEST_TRACE_OPENED, MEASUREMENT_RECORDED, SESSION_COMPLETED, SESSION_INIT, TRACE_INGESTED.

None of the tags for generation (`CANDIDATE_*`), model checking (`MODEL_CHECK_*`), reduction, discrepancy handling, contract revision, translation or the product lifecycle has fired.

**Answer to criterion 1:** the project does part of what the plan's evidence sections ask (§8, §11.1). It does almost nothing yet of what the plan exists for (§1, §2, §11.2–11.4, §12). That would be expected at sprint 5. At sprint 23 it is a sequencing problem. The standard practice for a system of this shape is to build the thinnest path through every stage first and widen it afterwards. Cockburn calls this a walking skeleton (*Crystal Clear*, 2004); Hunt and Thomas call it tracer bullets (*The Pragmatic Programmer*, 1999). The riskiest assumption here is the §1 premise, and it is the one assumption no sprint has touched.

## 3. SDD compliance

### 3.1 The twelve hard rules (`sdd-kit-3/AGENTS.md`)

| Rule | Requirement | Project state | Verdict |
|---|---|---|---|
| 1 Never edit foundations | Kit read-only | `git diff 052383a HEAD -- sdd-kit-3` is empty | Pass |
| 2 Vocabulary is the contract | Changes by typed proposal and version bump | 0.1 → 0.2 (P8, P9) and 0.2 → 0.3 (P10) went through `signals/proposals.json`; P11, required by the 2026-10-08 user decision, is still open | Pass, one open item |
| 3 Dual contract | Signal leg and artifact leg both graded | Sprints 013–022 graded "signal vacuous (library)" although every one ran a comparison and produced results | Fail from Sprint 013 |
| 4 Resolve-and-record | No silent decisions | 26 Decisions entries with grounds; two decisions dropped without a superseding entry (4.2) | Partial |
| 5 Comprehension affirmation | First session, post-scope-change, post-vocabulary-bump | Written 2026-10-03 and after the v0.2 change; not re-articulated after the 0.2 and 0.3 bumps | Minor fail |
| 6 ≤2 files, one concept | Split larger work, or record the exception | Sprint 010 recorded its 12-file exception. Sprint 018 bundles six concepts across nine files and Sprint 022 about eight files; neither records an exception | Fail from Sprint 018 |
| 7 Canonical home registry | Registry maintained | Present and extended through Sprint 018; vocabulary row still names `signals/0.1.json` | Pass, stale row |
| 8 UI design context | UI sprints include the design canon | No UI yet | Not applicable |
| 9 Observation contract | Declared before execution for behavior-touching sprints | Sprints 003–012 declare expected signals in advance. From Sprint 016 the cards carry an "observation" section written after the run, recording results, not expectations | Fail from Sprint 016 |
| 10 Bypass logged | Hand-author log in `WORKING_AGREEMENT.md` | Log empty. Sprint 000 authored `scripts/check_vocab.py` outside its card; that was recorded in the Sprint tail, not the log | Minor fail |
| 11 Originals over summaries | Transmit originals | No transmission occurred | Not tested |
| 12 Vocabulary first | No implementation before 0.1 lock | 0.1 locked in Sprint 000 before any code | Pass |

### 3.2 The loop beyond the hard rules

**Signal plane.** The decoder phase runs outside the vocabulary. `scripts/decoder_differential.py`, `generated_differential.py` and `operand_differential.py` emit no signal and write no substrate record. Their findings live in two JSON worklists under `specs/` and in BLACKBOARD prose.

The vocabulary already holds the tags this work needs: `COMPARISON_COMPLETED` with a `verdict`, `DISCREPANCY_REPORTED` with a `mismatch_class`, and `DISCREPANCY_ROUTED` with a `repair_path` that separates implementation repair from contract revision. The user decision of 2026-10-08 calls decoder disagreements the data that moves the project. Under SDD that data should be signals. Foundation 02 states the distinction: "A log tells you what happened. A signal tells you what *meant* something happened."

**Rubber Duck Pass.** Step 1 (sequence narration) last appears in Sprint 012. Sprints 013–022 list observations with dispositions but narrate no trace, because none exists. `AGENTS.md` describes a pass without its external check surface as "comprehension theater". In fairness, the observations in 020–022 are grounded in an external check, XED's output. That check surface is real but sits outside the vocabulary.

**Sprint cards.** Compare Sprint 007 with Sprint 018. Sprint 007 carries `prerequisites`, `context_files`, a signal contract with `Emits` and invariants, an artifact contract with files and exit codes, an observation contract, done criteria and a self-review checklist. Sprint 018 carries scope, repair-path labels, exit codes and post-hoc observations. Six required sections are gone. `templates/SPRINT_CARD.md` marks each of them required.

**Signal Reports.** `signal-reports/` holds one file, `sprint-000-report.md`. `templates/SIGNAL_REPORT.md` allows the report to live in chat. Chat is not in the repository, though, so 22 of 23 sprint reports cannot be audited.

**BLACKBOARD append-only discipline.**

- `## Decisions` lines 46–53 (dated 2026-10-08) sit above lines 54–56 (dated 2026-10-07). Commit e45db7b shows the 10-07 entries existed before the 10-08 entries were written, so the new entries were inserted, not appended.
- `## Built` is out of order: Sprint 007 is last, and Sprint 008 sits between Sprints 002 and 003.
- The Sprint 011 Built entry was retitled from "TSO ordering cost, native vs VM" to "TSO ordering lowerings, execution time". The user ordered the rewording (Decision 2026-10-08, "Records ... were reworded"). `templates/BLACKBOARD.md` marks `## Built` append-only, so a correcting entry was the compliant form.
- `## Sprint tail` holds 23 entries; the template's limit is 10, with older entries rolled into `## Built`.

**`WORKING_AGREEMENT.md` contradicts the current state in six places:**

- line 30 and line 68 name `signals/0.1.json` as the vocabulary; the version in force is 0.3 (`signals/current.json`).
- line 92 says "Windows oracle: pending user answer (Q2)"; the user answered on 2026-10-05.
- line 59 marks the Hypervisor.framework bridge mapping "pending"; Sprints 008–012 were written against that framework. The mapping lives only in Sprint 008's Rubber Duck note ("Hypervisor headers in the installed SDK were the bridge mapping").
- Capstone (Sprint 014) and XED (Sprint 017) have no bridge-mapping entries. TECHNIQUES.md #46 requires one before the first sprint that uses an SDK.
- line 132, the custom technique "Oracle-first contract mining", requires that "every clause cites an oracle run". The user decision of 2026-10-07 made reference runs optional, and `src/w2m/contract.py:121-125` implements that decision.
- the sprint cadence policy (lines 108–113) has no decoder phase. Sprints 013–022 are filed under Phase 3, "Application profile and product slice", which none of them touches.

**KIT_DIARY.** This record is maintained well. Nine numbered findings and four hypotheses (H4 confirmed with evidence) meet `templates/KIT_DIARY.md`. Its last entry is the 2026-10-08 cost correction; the ten decoder sprints have no diary entry.

**Answer to criterion 2:** Sprints 000–012 follow the kit closely and improve on it in places: generated Structs with a `--check` gate, and same-run controls. Sprints 013–022 keep the gates and the observations but drop the vocabulary, the card structure, pre-declared observation contracts and the record order.

## 4. Where it went wrong: confirmed defects

Each item names the file and line, the evidence, and its consequence.

### 4.1 The decoder chain has no exit criterion

Sprints 013–022 ran on 2026-10-08. Their results, in order:

- PE reader graded against `llvm-objdump`.
- Capstone traversal.
- A spec-generated decoder.
- An unseen-corpus differential.
- XED as primary reference.
- Instruction-class identity.
- EVEX and XOP decoding.
- Generated encodings.
- A validity layer.
- Operands.

Each close names the next decoder sprint: Sprint 021 defers the "AVX-512 validity tail", and Sprint 022 defers the "5.4% of corpus positions with no template". No card or Decision states the agreement level at which the decoder is good enough for the next stage, the lifter. Work without a stated finish line keeps expanding: every differential produces a new residue, and the residue becomes the next sprint.

The worklists rank fixes by frequency in Python 3.14.8's binaries, not in either chosen game. v0.2 §12.1 says the selected game's profile "guides probes and contract coverage".

### 4.2 Two recorded decisions were not carried out and not superseded

The Decision of 2026-10-08 (BLACKBOARD line 50) orders adjudication "CPU (windows-2025 runner) first, then XED and the SDM text". No CPU oracle probe exists. Since Sprint 017, XED alone judges every disagreement.

`docs/review-2026-10-08-decoder-theory.md` ("Against v0.2") says the spec "should record which source confirmed each row". The rows carry a `src` citation (for example "Intel SDM Vol 2D Appendix A, Table A-2"), but no field records a confirmation or an evidence category. Under v0.2 §11.1 and the project's own reading, the 812 rows remain hypotheses. They agree with XED, but agreement with XED is not recorded per row as evidence.

`AGENTS.md` hard rule 4 says "Never decide silently". Abandoning a recorded decision without a superseding entry is a silent decision.

### 4.3 The decoder accepts VEX instructions after 66, F2 or F3 prefixes; the generator cannot find this

`src/w2m/x86dec.py:104` enters the VEX branch whenever the byte after the prefixes is C4 or C5 and no REX was seen. It does not check for a preceding 66, F2, F3 or F0. Intel SDM Vol 2A §2.3.2 makes those prefixes before VEX cause #UD, and XED agrees:

| Bytes | w2m | XED v2026.08.23 |
|---|---|---|
| `66 c5 f8 77` | length 4, VZEROUPPER | invalid, BAD_LEGACY_PREFIX |
| `f3 c5 f8 77` | length 4, VZEROUPPER | invalid, BAD_LEGACY_PREFIX |
| `66 62 f1 7c 48 58 c0` | DecodeError (spec gap) | invalid, BAD_LEGACY_PREFIX |
| `40 c5 f8 77` | DecodeError (spec gap) | invalid, BAD_REX_PREFIX |

The first two decode as valid instructions where the CPU raises #UD. The last two are reported as gaps in the spec ("no entry") when they are invalid encodings. That conflates "w2m does not know" with "the guest observes #UD", which v0.2 §5.1 keeps apart.

The generated differential never saw this class. `src/w2m/encgen.py:59-67` adds the extra legacy prefixes (`LEGACY_EXTRA`) only to legacy rows. Lines 68–89 build VEX, XOP and EVEX cases with no prefix before them. Of 38,976 generated cases, 0 put a legacy prefix before a VEX, XOP or EVEX escape. Sprint 021's figure of "11.38% accepted-but-invalid" therefore understates the validity gap by at least this class.

### 4.4 The decoder raises IndexError on short input; the differential would count crashes as refusals

`x86dec.decode` indexes `b[i]` at lines 77, 88, 96, 104 and 119 without a length check. `decode(bytes.fromhex("66"))` raises `IndexError`, and so does `decode(bytes.fromhex("66" * 15))`. XED returns BUFFER_TOO_SHORT and INSTR_TOO_LONG. The docstring promises `DecodeError`.

Every caller pads input to 15 bytes with zeros (`pe.py:69`, `generated_differential.py:48`). That hides the crash but creates a second problem. An instruction cut off by the end of a section is decoded with invented zero bytes, where a real CPU would fault on the next page.

`scripts/generated_differential.py:51` catches `IndexError` and `KeyError` and counts them as "ours rejects". A crash in the decoder is therefore scored as a correct refusal (`both_reject`) whenever XED also refuses. The current case set raises neither exception (all 38,976 cases measured), so this is a latent defect in the measuring tool, not a present wrong number.

### 4.5 The litmus driver records guest TSO as "present" on a test that cannot fail

`scripts/litmus_mp.py:63` sets `outcome="present" if on["forbidden_outcomes"] == 0`, and line 102 exits 0 on the same condition. Neither requires the EnTSO-off rounds to show any forbidden outcome. A run in which the test has no power (off = 0, on = 0) records `guest_tsoen present`. The words "effect demonstrated" appear only inside the free-text `detail`.

KIT_DIARY hypothesis H4 ("a correctness check that passes on zero observed failures needs a same-run control that fails") was confirmed in Sprint 011, and `scripts/ordering_cost.py:69` applies it. It was not carried back to `litmus_mp.py`. The recorded run (off 2245/50,000,000, on 0/50,000,000) did have power, so the recorded fact stands. The driver would record the same fact without power.

### 4.6 A typed field asserts what the plan says is open

`scripts/probe_capabilities.py:74` records `CAPABILITY_PROBED(capability="native_process_tso", outcome="absent")` because a search of the SDK headers for "EnTSO", "TSOEN" and "total store order" found nothing. The free-text `detail` says "an SDK fact, not a hardware claim". The typed field says absent.

v0.2 §8.1 says the opposite: "that remains an open destination-interface question rather than an asserted absence". The vocabulary's `capability_outcome` enum offers only `present`, `absent` and `error`, so the code had no honest value to emit. Any later consumer that reads the typed field, such as `BACKEND_EVALUATED`, will read an absence the plan forbids asserting. This is a vocabulary gap: `capability_outcome` needs a value such as `not_established`, proposed as `PAYLOAD_FIELD_PROPOSED`.

### 4.7 Imported executions carry made-up payload values

`scripts/judge_traces.py:64-68` emits `EXECUTION_STARTED(timeout_seconds=600.0, kill_grace_seconds=0.0)` and `EXECUTION_COMPLETED(exit_status="exited", exit_code=0, elapsed_seconds=0.0)` for every runner execution. These are constants, not observations:

- **`exit_code=0`** follows indirectly from `.github/workflows/windows-probe.yml`, which fails the job on a non-zero exit. The record does not say it was inferred.
- **`elapsed_seconds` and the timeout** are placeholders. Sprint 007's Rubber Duck observation 3 records this and defers it.

TECHNIQUES.md #47 ("substrate-gap-honest events ... Don't fake-emit") names exactly this: when the data does not exist, say so; do not emit a plausible value.

In the same probe, `probes/event_auto_reset/event_auto_reset.c:58` fills `args_sha256` with the hash of a hand-written label such as `"WaitForSingleObject(event,2000)"`. The field's name promises a hash of the arguments.

The probe's precondition, that both waiters are pending when `SetEvent` runs, comes from `Sleep(200)` (line 124) and is never observed. The contract's `scenario.initial.pending_waiters: [a, b]` is assumed rather than witnessed. The 10-of-10 result for waiter a fits waiter b sometimes not yet waiting as well as it fits FIFO selection. The contract's third open question raises the FIFO reading but not this one.

### 4.8 Phase 2 evidence is not in the repository

`.gitignore` excludes `/runs/`, on the grounds (Decision 2026-10-07) that `scripts/judge_traces.py` regenerates judge records from the committed traces. That holds for the two `judge-*` records. The Built entries for Sprints 008–012 cite `runs/capability-1791441341`, `runs/litmus-1791441621`, `runs/ordering-1791442786` and `runs/pagegeom-1791443163`. Those are hardware measurements on one Mac, and nothing committed regenerates them.

The evidence behind every Phase 2 conclusion exists only on this machine. v0.2 §14 requires provenance from accepted results back to their evidence.

### 4.9 Smaller defects

- **`src/w2m/pe.py` ignores the delay-load import directory (index 13).** One of the 33 Python corpus binaries (`_wmi.pyd`) has one. Calls through delay-load slots are counted as unresolved indirect transfers by `x86ref.walk`, and the delay-loaded DLLs are missing from the import inventory that v0.2 §7 treats as the first draft of the contract inventory. Microsoft's "PE Format" documents the table under "Delay-Load Import Tables".
- **`pe.py:176-187` counts base relocations by type but keeps no target addresses.** A loader cannot apply relocations from this data.
- **`x86ref.walk` treats every `call` as returning.** Calls to non-returning functions (ExitProcess, `__fastfail` paths) let the traversal walk into whatever follows. Andriesse et al. (USENIX Security 2016) measure non-returning functions as a main source of error in recursive disassembly of real binaries. This review did not test whether the libcrypto strings in the Deferred list (2026-10-08) come from that cause or, as recorded, from data embedded in hand-written assembly.
- **`contract.py:101-102` claims to require an execution id for observed evidence but only checks that `source_ref` is non-empty.**
- **`.github/workflows/windows-probe.yml` pins `actions/checkout@v4` and `actions/upload-artifact@v4` by tag.** GitHub's "Security hardening for GitHub Actions" guide recommends pinning to a full commit SHA. The risk is small for first-party actions.

## 5. Accepted engineering practice

| Practice | Source | Project state |
|---|---|---|
| Walking skeleton before widening | Cockburn, *Crystal Clear* (2004); Hunt and Thomas, *The Pragmatic Programmer* (1999) | Absent (Section 2) |
| Continuous integration on every change | Fowler, "Continuous Integration" (2006) | Absent. The only workflow is the manually dispatched Windows probe. On a clean clone the XED-dependent tests skip (`tests/test_xedref.py:13`, `test_x86dec.py:107`, `test_x86ops.py:45`), so the suite passes while checking less |
| Fuzz parsers of untrusted input | libFuzzer/OSS-Fuzz practice; Atheris for Python; Hypothesis for property tests | `pe.py` and `x86dec.py` parse attacker-shaped bytes (shipped game binaries) and have hand-picked malformed cases only. Item 4.4 is the kind of defect a fuzzer finds in seconds |
| Differential testing with generated inputs | Paleari et al., ISSTA 2010; Woodruff et al., LangSec 2021 | Adopted (Sprint 020); prefix coverage incomplete (4.3) |
| Independent versions fail together | Knight and Leveson, IEEE TSE 1986 | Cited and acted on (XED by authority, not by vote) |
| Same-run controls; zero failures in n trials bounds the rate at 3/n | Hanley and Lippman-Hand, JAMA 1983 | Applied in `ordering_cost.py` and in the litmus bound; missing from the litmus outcome (4.5) |
| Code layout distorts benchmarks | Mytkowicz et al., "Producing Wrong Data Without Doing Anything Obviously Wrong!", ASPLOS 2009 | Found independently in Sprint 012 (1.9x became 1.04x after moving a branch out of line) |
| Reproducible evidence | v0.2 §14; content-addressed storage | Traces and binaries content-addressed and committed; Mac run records not (4.8) |
| Pinned dependencies | uv lockfile practice | `uv.lock` committed; `substrate-kernel==1.1.1`, `capstone==5.0.9`, ruff pinned; XED pinned by commit in `WORKING_AGREEMENT.md:86` |
| Typed validation at the boundary | msgspec strict decoding; foundation 02 | Strong: strict Structs, `forbid_unknown_fields`, generated from the vocabulary |

## 6. What is obviously missing

1. **An end-to-end path.** Nothing turns a Windows PE into execution on the Mac in either backend. The VM probes run hand-written AArch64 guest code (`mp_litmus.c:29-51`).
2. **The machine-semantics layer (v0.2 §5).** Decoding stops at operands for 303 integer rows. No instruction has defined semantics, so "compare instruction semantics by execution", the decoder plan's item 5, has nothing to compare.
3. **A loader and guest runtime.** No mapping of sections, relocation, import binding or thread start exists.
4. **A Win32 contract inventory.** Sprint 013 found 81 imports in one probe (KERNEL32 79). One contract exists. No sprint has listed which contracts a chosen game needs.
5. **The contract compiler (v0.2 §11.2).** Contracts do not yet generate tests, generators or checkers; one relation is hand-coded.
6. **Generation of implementations (v0.2 §1, §11.4).** The `CANDIDATE_*` tags and the generator/checker topology the plan names have never run.
7. **Contact with the chosen games.** BioShock 2 Remastered (app 409720) was chosen on 2026-10-05 as the test profile. Its Windows binaries have not been downloaded, parsed or traversed, though the Mac is logged into Steam (Decision 2026-10-05).
8. **Phase acceptance criteria.** `WORKING_AGREEMENT.md` names phases without the evidence that closes each.
9. **CI for the harness suite**, and a fuzz target for `pe.parse` and `x86dec.decode`.

## 7. What holds up and should be kept

- **The evidence model.** `contract.py` requires a quote and a retrieval date for documented evidence and a distinguishing check for a hypothesis, and refuses acceptance on hypothesis alone. The event contract keeps outcome {b} allowed after ten FIFO-looking runs, as v0.2 §3 requires ("Capturing one schedule does not make that schedule the only acceptable result").
- **Typed signals generated from the vocabulary**, with `gen_events.py --check` as a gate. This turns vocabulary drift into a failing command; TECHNIQUES.md Section 3 lists it as a facility the kit does not ship.
- **Trace ingest (`trace.py`)** checks the content hash, strict decoding, the open-first/close-last ordering, per-thread sequence numbers and happens-before references. It reports the first bad line, and it never relies on the order of lines in the file.
- **Measurement hygiene in Phase 2.** Three wrong first-pass numbers were caught before they were recorded: a warm-up asymmetry (6.2x), a litmus with no power, and a code-layout effect (1.9x). KIT_DIARY records each.
- **The decoder differential itself.** It used three references (XED, Capstone, LLVM). A planted fault produced 224 disagreements, which shows the comparison can detect a one-table error. Generated encodings found two operand-template errors that 2.44 million compiled instructions never hit.
- **Recorded decisions with grounds, and the user's redirections absorbed** as appended user decisions (v0.2 adoption, no physical machine, optional reference runs, no "cost" framing).

## 8. Recommendations, in order

1. **Close the decoder phase.** Write its exit criteria into `WORKING_AGREEMENT.md`: for example, length and instruction class equal XED's on generated cases for every row a chosen game uses, and validity refusals equal XED's, including prefixes before VEX/EVEX. Fix 4.3 and 4.4 inside that close. Record a superseding Decision for the CPU oracle and per-row confirmation (4.2), either doing them or dropping them with grounds.
2. **Build the walking skeleton from artifacts that already exist.** Write a minimal Windows probe with no C runtime that calls two kernel32 functions. Run it on the `windows-2025` runner for a reference trace. On the Mac, load it, lift its few instructions, run them in the Hypervisor.framework VM with the guest TSO setting, emit the guest trace, and judge it with the existing `compare.py` path. Each stage starts as narrow as possible. The point is that every stage of v0.2 §2 exists once.
3. **Run one generated implementation against one contract.** Generate the auto-reset event's runtime implementation through a substrate `best_of_n_verified`-shaped topology. Check it against `windows.event.auto_reset.single_signal@0.2.0`, letting `CANDIDATE_*` and `DISCREPANCY_*` fire. This tests the §1 premise directly.
4. **Put the decoder differential on the record.** Emit `COMPARISON_COMPLETED` per run and `DISCREPANCY_REPORTED` / `DISCREPANCY_ROUTED` per disagreement class, with `repair_path` set from the card labels. Otherwise, record a Decision that decoder work sits outside the signal plane, with grounds.
5. **Repair the records.**
   - Append correcting entries for the out-of-order Decisions and Built entries.
   - Roll the Sprint tail to 10.
   - Update the six stale `WORKING_AGREEMENT.md` lines.
   - Add bridge mappings for Hypervisor.framework, Capstone and XED.
   - Restore the full card template.
   - Write Signal Reports to `signal-reports/`.
6. **Fix the evidence defects.**
   - Require off > 0 in `litmus_mp.py` before recording "present".
   - Propose a `not_established` capability outcome and re-record `native_process_tso`.
   - Mark imported execution fields as inferred or leave them out.
   - Commit, or archive by hash, the Mac run records that Built entries cite.
7. **Add CI and fuzzing.** A GitHub workflow that runs the full gate list, with XED built in the job. An Atheris or Hypothesis target for `pe.parse` and `x86dec.decode`.
8. **Fetch BioShock 2 Remastered's Windows depot** and run `pe.py`, `x86ref.walk` and the differential on it, so the chosen profile, not the Python corpus, ranks the work.

## Sources

- Intel, *64 and IA-32 Architectures Software Developer's Manual*, Vol 2A §2.3.2 (VEX prefix: preceding 66/F2/F3/F0/REX cause #UD).
- Intel XED v2026.08.23, commit 0bcb6237345c, built in `third_party/xed`; outputs quoted in 4.3 and 4.4 were produced on 2026-10-08.
- A. Cockburn, *Crystal Clear: A Human-Powered Methodology for Small Teams*, Addison-Wesley, 2004 (walking skeleton).
- A. Hunt, D. Thomas, *The Pragmatic Programmer*, Addison-Wesley, 1999 (tracer bullets).
- M. Fowler, "Continuous Integration", martinfowler.com, 2006.
- R. Paleari, L. Martignoni, G. Fresi Roglia, D. Bruschi, "N-version disassembly: differential testing of x86 disassemblers", ISSTA 2010.
- W. Woodruff, N. Carroll, S. Peters, "Differential Analysis of x86-64 Instruction Decoders", LangSec 2021.
- J. C. Knight, N. G. Leveson, "An Experimental Evaluation of the Assumption of Independence in Multiversion Programming", IEEE TSE 12(1), 1986.
- D. Andriesse, X. Chen, V. van der Veen, A. Slowinska, H. Bos, "An In-Depth Analysis of Disassembly on Full-Scale x86/x64 Binaries", USENIX Security 2016.
- T. Mytkowicz, A. Diwan, M. Hauswirth, P. F. Sweeney, "Producing Wrong Data Without Doing Anything Obviously Wrong!", ASPLOS 2009.
- J. A. Hanley, A. Lippman-Hand, "If Nothing Goes Wrong, Is Everything All Right?", JAMA 249(13), 1983.
- Microsoft, "PE Format", learn.microsoft.com (Delay-Load Import Tables; base relocations).
- GitHub, "Security hardening for GitHub Actions", docs.github.com (pin actions to a full-length commit SHA).
- `sdd-kit-3/AGENTS.md`, `templates/SPRINT_CARD.md`, `templates/BLACKBOARD.md`, `templates/SIGNAL_REPORT.md`, `TECHNIQUES.md` #46, #47; `foundations/02-sdd-practice.md`.
