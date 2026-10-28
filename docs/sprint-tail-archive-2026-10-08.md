# Sprint tail archive

Entries rolled out of `BLACKBOARD.md` `## Sprint tail` on 2026-10-08 (Sprint 027) and later, verbatim, newest first. The tail keeps the 10 most recent (`sdd-kit-3/templates/BLACKBOARD.md`); each sprint's summary also stands in `## Built`.

### Sprint 035 (2026-10-09, closed)
- **Scope:** kernel32 for both probes as freestanding C in the VM; C candidates; translation on the signal plane.
- **Rubber Duck Pass:**
  - *Sequence narration:* the VM builds the runtime image with clang and `ld -preload`, copies it into the monitor area, writes the TEB and PEB at the spec's offsets, and points each import's dispatch entry at a bridge; a call saves the GPRs, enters the C service on the runtime stack, which reads or writes the TEB or asks the host through the transport page, and returns through RET's pop; the ring gives the host the call list; each compiled block is published on the signal plane.
  - *Observation 1 (a timer measured the recorder):* the first VM times doubled because the translation record was written inside the timed call. Disposition: resolved-here; the direct split matches Sprint 032.
  - *Observation 2 (a fixture caught store pollution):* re-judging under a new prefix doubled the reference executions. Disposition: resolved-here (ids reused for identical bodies; `--run-id` must be the GitHub run id).
  - *Observation 3 (C changes what admission must police):* the signature now carries the parameter rule; division moved from "may raise" to "refused", because an x86 host traps where AArch64 does not. Disposition: resolved-here, with tests.
- **Closed:** clean.

### Sprint 034 (2026-10-09, closed)
- **Scope:** CONTEXT, TEB, PEB and fault/suspend behavior measured on the runner; the guest state spec judged against it.
- **Rubber Duck Pass:**
  - *Sequence narration:* the runner built the probe, which printed header offsets, read its own TEB through gs: against the APIs, faulted under a first vectored handler with known registers and resumed it by rewriting CONTEXT, and suspended a spinning thread to read and change its registers; the checker compared the spec's 62 fields with three runs and recorded the verdict.
  - *Observation 1 (a clean check needs a failing control):* four planted spec faults are each caught with the exact field and value. Disposition: resolved-here (tests).
  - *Observation 2 (values written from memory):* SegCs 0x33 and SegGs 0x2B were not read by the probe. Disposition: marked unobserved in the spec; the next probe revision reads them.
- **Closed:** clean.

### Sprint 033 (2026-10-09, closed)
- **Scope:** Hail to the Rainbow's native and Mono-generated code, captured on the Windows runner and judged against XED.
- **Rubber Duck Pass:**
  - *Sequence narration:* the depot was packed into five zstd tar parts on a private release; the runner checked each part's hash, unpacked, checked all 297 files and that no other file was present, built the launcher, and started the game through UnityMain with Mono's jit_done and jit_code_buffer hooks; the hooks wrote each method's image, name and bytes, each trampoline's bytes, and the CPU's features; on the Mac the differential decoded every native position and every captured region, let XED decide where w2m's decoder refused, and wrote one substrate run.
  - *Observation 1 (the fixture was wrong twice):* BioShock 2 Remastered was 32-bit, and the decoder's "0 gaps" came from a Python corpus. Disposition: recorded (KIT_DIARY #17); criterion (d) re-pointed at this game.
  - *Observation 2 (the reader trusted its first record):* empty image names and data decoded as IN/OUT came from Mono reporting methods twice and trampolines carrying inline arguments. Disposition: resolved-here, with tests.
  - *Observation 3 (captures depend on the CPU):* run 1 had an Intel Xeon 8370C, run 2 an AMD EPYC 7763; Mono selects instructions by feature. Disposition: CPUID is recorded in every capture.
  - *Observation 4 (why rows are unconfirmed):* two missing validity rules (LEA mod=3; F2/F3 over 66). Disposition: deferred to Sprint 036 (decoder), first items.
- **Closed:** clean.

### Sprint 032 (2026-10-09, closed)
- **Scope:** last-error contract 0.2.0 judged on values chosen at run time.
- **Rubber Duck Pass:**
  - *Sequence narration:* the runner built last_error_values.exe; each run read QueryPerformanceCounter, chose three table entries, and made three SetLastError/GetLastError pairs, writing each SetLastError's true args_sha256 from the table. The comparator hashed `SetLastError(<returned value>)` for each GetLastError line and matched it against the preceding SetLastError line. Windows, the model and the VM conformed; a planted constant failed at the first read and the model repaired it.
  - *Observation 1 (the lockstep harness had a blind path):* a kernel32 return to an already-compiled block ran that block before the next check, so lockstep reported the VM one instruction ahead. The skeleton never returned to an address twice. Disposition: resolved-here; regression test fails at step 4,404 without the fix.
  - *Observation 2 (the loop rejected one candidate three times):* the guard's trigger read shared state that the runner had already filled for all three runs. Disposition: resolved-here; the trigger records each failure from its event.
  - *Observation 3 (what 0.2.0 still allows):* a candidate that ignores its argument passes one pair with probability at most 1/256; three pairs with consecutive values different make that negligible, and the table is not in the prompt. Disposition: recorded in the contract's open questions.
- **Closed:** clean.

### Sprint 031 (2026-10-08, closed)
- **Scope:** the candidate loop run for real on `windows.thread.last_error.set_get@0.1.0`.
- **Rubber Duck Pass:**
  - *Sequence narration:* the harness opened a session; the generator sent the contract's summary, documented quotes and the interface to qwen2.5-coder:7b and stored its reply; the runner admitted the code, installed it in kernel32 and ran the skeleton three times in the VM; the comparator judged each trace; the guard re-hashed the contract and checker, found them unchanged, and accepted the candidate citing three conforming comparisons. In the planted session the guard rejected attempt 1, the reducer reported and routed the discrepancy, and the model's repair was accepted and the discrepancy resolved.
  - *Observation 1 (the first real run failed silently in the record):* a missing httpx killed the generator and the session never closed. Disposition: resolved-here (dependency; abort path; test).
  - *Observation 2 (the record misnamed a model):* the planted wrapper hid real metering, so the record named the wrapper as the model. Disposition: resolved-here; session repeated.
  - *Observation 3 (the contract is weaker than the loop):* a constant passes contract 0.1.0. Disposition: open question; pinned by a test.
- **Closed:** clean, with one open contract defect.

### Sprint 030 (2026-10-08, closed)
- **Scope:** AArch64 back end in the Hypervisor.framework VM (profile_kind managed_vm).
- **Rubber Duck Pass:**
  - *Sequence narration:* the host loaded the runner-built .exe into a shared memory file, wrote stage-1 tables so each x86 address maps to itself, wrote the dispatcher and vector table, and started the monitor. The vCPU entered the dispatcher with x17 = the entry address; each miss exited to the host, which lifted and compiled one basic block, filled the dispatch table and resumed. Six times control reached a kernel32 thunk; the host serviced the call on the guest's registers and memory and resumed at the return address. ExitProcess(0) ended the run after 5,153 instructions in 43 blocks.
  - *Observation 1 (one real bug, found by the first run):* conditional branches were treated as ending control flow; the not-taken path ran into the next block. The failure was loud (undefined instruction at a named x86 address). Disposition: resolved-here.
  - *Observation 2 (lockstep is not enough):* a planted signed-carry fault passed lockstep over the whole program, because no SUB on the path mixes signs. Disposition: resolved-here; a random-state differential (8,680 cases) now runs before every VM judging and in the gates. It found both planted faults it had first missed only after its memory comparison covered the bytes around every effective address, not just the stack.
  - *Observation 3 (what is not yet isolated):* the monitor's tables and code share EL1 and the address space with translated guest code. Disposition: recorded for the next VM widening.
- **Closed:** clean.

### Sprint 029 (2026-10-08, closed)
- **Scope:** walking skeleton through every v0.2 §2 stage.
- **Rubber Duck Pass:**
  - *Sequence narration:* the runner compiled skeleton.c without a C runtime and ran it; the program wrote four trace lines to stdout and exited 0. On the Mac the loader mapped the same bytes at 0x140000000 and pointed six import slots at host thunks; the runtime decoded, lifted and executed 5,153 instructions, answered GetCurrentThreadId, SetLastError, GetLastError, GetStdHandle and WriteFile in the host, and stopped at ExitProcess(0). Both traces were wrapped, ingested, and judged conforms against the last-error contract.
  - *Observation 1 (worked first time; checked anyway):* the first Mac run succeeded. Three planted faults (kernel32, lifter, unliftable instruction) each changed the outcome, so the pass is not vacuous.
  - *Observation 2 (what the skeleton does not prove):* one thread, one image at its preferred base, 23 instruction classes, no VM. Each is the next widening, not a gap in the path.
  - *Observation 3 (a new fixture exposed an old oracle bug):* the LLVM reference parser in `test_x86ref.py` dropped the last instruction of `.text`; CRT probes hid it with trailing padding. Disposition: resolved-here; four CRT-only assertions scoped.
- **Closed:** clean.

### Sprint 028 (2026-10-08, closed)
- **Scope:** witness the pending-waiters precondition instead of assuming it.
- **Rubber Duck Pass:**
  - *Sequence narration:* on the runner, main started a and b, waited for both announcements, read the kernel's thread table once, found both in Wait/UserRequest, wrote two GUEST_THREAD_STATE_OBSERVED lines, then SetEvent; a returned 0, b timed out (258) after 2 s; main's final zero-timeout wait returned 258. The judge opened a session, imported 20 executions with observed exit codes and times, ingested 20 traces, found each witness ahead of SetEvent, and returned conforms ×20.
  - *Observation 1 (the defer was not needed):* the witness took one documented interface and one guest tag; the struct offsets derived from the documented layouts were right on the first runner build. Disposition: resolved-here.
  - *Observation 2 (third false gate):* part 1 was committed with a failing test because gates were chained with `;`. Disposition: resolved-here; `scripts/gates.sh` (fail-fast) now gates commits.
  - *Observation 3 (what the data now says):* waiter b was always pending, and a still won 20 of 20. Creation order and wait order coincide; separating them is the contract's open question.
- **Closed:** clean.

### Sprint 027 (2026-10-08, closed)
- **Scope:** records repair, CI with nothing skipped, property-based fuzzing; review §3, §5, §6.8.
- **Rubber Duck Pass:**
  - *Observation 1 (fuzzing found a defect at once):* one header-byte flip made an import DLL name non-ASCII and `pe.parse` raised `UnicodeDecodeError`, not `PeError`. Fixed in `_Reader.cstr`. The same property, pointed at the pre-023 decoder, found its `IndexError` within 5,000 examples, so the decoder property has power too. Disposition: resolved-here.
  - *Observation 2 (bridge mappings were written after the code):* Hypervisor.framework, Capstone and XED entries now exist; the agreement says they were added late. Disposition: recorded.
  - *Observation 3 (tail roll):* entries for Sprints 000-017 moved verbatim to `docs/sprint-tail-archive-2026-10-08.md`; nothing deleted. Disposition: resolved-here.
- **Closed:** clean (CI run 37851288883: 150 passed, 0 skipped).

### Sprint 026 (2026-10-08, closed)
- **Scope:** review §4.9 code defects: delay-load, relocation sites, non-returning calls, observed-evidence execution ids.
- **Rubber Duck Pass:**
  - *Observation 1 (a fetched summary was wrong):* the web-fetch summary of the PE Format delay-load table put Attributes at offset 24 and omitted the timestamp. LLVM `Object/COFF.h` and the 0x40-byte directory in `_wmi.pyd` agree on Attributes at 0 and 32-byte descriptors. Disposition: resolved-here; the code cites LLVM. KIT_DIARY finding 13.
  - *Observation 2 (a review question answered):* libcrypto's code-decoded strings do not come from non-returning calls. Disposition: recorded; the Deferred entry stands.
  - *Observation 3 (immutability kept):* 0.2.0 is not edited; the loader keeps a legacy rule for prose-cited executions and 0.2.1 carries ids. Disposition: resolved-here.
  - *Observation 4 (a gate that could not fail):* the close ran `uv run mypy | tail -1`; the pipe hid mypy's exit 1, and the Built entry claimed mypy 0. Fixed after commit 1c214cf; gates now run unpiped with each exit status printed. Disposition: resolved-here; same family as KIT_DIARY H4 (a check needs a way to fail).
- **Closed:** clean.

### Sprint 025 (2026-10-08, closed)
- **Scope:** decoder differential on the signal plane; judge session tags.
- **Rubber Duck Pass:**
  - *Sequence narration:* the differential opened a conformance session for `w2m-x86dec-spec-0.6.2`, recorded one exact comparison against XED with verdict discrepancy, then 300 discrepancy pairs, each naming its stored example and routed to contract revision, then closed with 300. The judge opened, imported 2 runner executions whose exit code and elapsed time were observed for the first time, ingested and compared both (conforms), and closed with 0.
  - *Observation 1 (a report written from memory):* the Sprint 024 report listed SESSION_INIT/COMPLETED and MEASUREMENT_RECORDED that its records do not contain. The defect it hid (judge runs without session tags, T1/T2) was real. Disposition: resolved-here; correction appended to that report; this report's trace is read from the records.
  - *Observation 2 (a latent collision):* run roots named by whole seconds; two judge runs in one second wrote one directory, and substrate's gap check caught it. Disposition: resolved-here in five scripts.
  - *Observation 3 (card invariant was wrong):* "classes the text report prints" counts 30, not 300. Disposition: corrected at close in the card.
- **Closed:** clean.

### Sprint 024 (2026-10-08, closed)
- **Scope:** vocabulary 0.4 and evidence defects; review items 4.5-4.8.
- **Rubber Duck Pass:**
  - *Sequence narration:* `judge_traces.py store:37738035459` opened a run (SESSION_INIT), imported 10 runner executions (EXECUTION_STARTED with `unobserved` naming timeout and grace; EXECUTION_COMPLETED with exit code and elapsed omitted and named), ingested each trace (TRACE_INGESTED ×10, hashes match the committed files), and compared each against `windows.event.auto_reset.single_signal@0.2.0` (COMPARISON_COMPLETED ×10, verdict conforms, released a). `probe_capabilities.py` fired CAPABILITY_PROBED ×5, `native_process_tso` not_established. `litmus_mp.py` fired CAPABILITY_PROBED guest_tsoen present with off 35 > 0.
  - *Observation 1 (a recorded claim was false):* the 2026-10-07 Decision said judge records regenerate from the store; the role maps they need were never committed. Recovered from GitHub artifacts days before nothing could have recovered them. Disposition: resolved-here; superseding Decision.
  - *Observation 2 (rates are not properties):* EnTSO-off forbidden rate was 4.5e-5 on 2026-10-07 and 7.0e-7 today, same binary, same host. Only power (off > 0, on = 0) is evidence. Disposition: recorded.
  - *Observation 3 (my own slip):* Sprint 023's Built entry went in after Sprint 022, not at the section end, an hour after the correction Decision said entries append at the end. Moved to the end in this sprint. Disposition: resolved-here.
- **Closed:** clean.

### Sprint 023 (2026-10-08, closed)
- **Scope:** decoder phase close; review items 4.3, 4.4, 4.2.
- **Rubber Duck Pass:**
  - *Observation 1 (a measuring tool that hid its own blind spot):* `encgen` never put a prefix before a VEX/XOP/EVEX escape, so 1,170 accepted-but-invalid cases were invisible to every differential since Sprint 020. The review found the class by reading SDM §2.3.2. Disposition: resolved-here; KIT_DIARY finding 10.
  - *Observation 2 (a review claim holds only in part):* zero-fill between raw and virtual size is the loader's behavior and stays; bytes past the section end were invented and are gone. Disposition: resolved-here.
  - *Observation 3 (hashing found a spec defect):* keying confirmations by row hash exposed three duplicate rows. Disposition: resolved-here; generator refuses duplicates.
- **Closed:** clean.

### Sprint 022 (2026-10-08, closed)
- **Scope:** explicit operands, w2m (SDM Appendix A templates) vs XED, corpus and generated encodings.
- **Rubber Duck Pass:**
  - *Observation 1 (zero is a claim that needs a test):* the first corpus run reported 0 differences in 2.26 M comparisons. A planted fault (REX byte-register names without REX) produced 224 differences, so the comparison has power. Disposition: resolved-here; the planted-fault result is in the card.
  - *Observation 2 (the literature's prediction held again):* both template errors (f64 on FF /2, /4; MOVSXD `Ez`) appeared only in generated encodings, never in 2.44 M compiler-emitted instructions. Disposition: resolved-here — spec 0.6.1.
  - *Observation 3 (a recorded number looked wrong):* this run reported 2,391,671 positions against the 2,444,456 recorded since Sprint 017. The recorded figure includes the 52,785 probe instructions; both are now reported separately. Disposition: resolved-here.
  - *Observation 4 (coverage):* 5.4% of corpus positions have no template (no-operand rows, SSE/AVX). Disposition: deferred — next decoder sprint, with the AVX-512 validity tail.
- **Closed:** clean.

### Sprint 021 (2026-10-08, closed)
- **Scope:** validity layer (#UD) and iclass edge rules from the generated differential.
- **Rubber Duck Pass:**
  - *Observation 1 (the agent's own test was wrong):* a test asserted `f0 01 03` is invalid; XED decodes it as `lock add dword ptr [rbx], eax` (opcode 01 has a memory destination). The intended case was `f0 03 03` (BAD_LOCK_PREFIX). Disposition: resolved-here — the test was checked against XED before being changed, and the decoder was right.
  - *Observation 2 (direction of error):* the strict-prefix rule turned silent misdecodes into 73 refusals (ours has no row where XED knows a newer instruction). Disposition: recorded — a refusal is honest; the rows are worklist items.
  - *Observation 3 (residue):* 11.38% accepted-but-invalid, mostly EVEX broadcast/zeroing/opmask rules and VEX/EVEX W constraints. Disposition: deferred — next decoder sprint; then operand comparison.
- **Closed:** clean.

### Sprint 020 (2026-10-08, closed)
- **Scope:** generated encodings per spec row vs XED.
- **Rubber Duck Pass:**
  - *Observation 1 (the literature's prediction held):* full agreement on 2.44 M corpus instructions coexisted with 31% disagreement on generated encodings. Disposition: recorded — corpus agreement is not decoder correctness.
  - *Observation 2 (vocabulary gap → spec schema):* the spec has no validity layer; invalid encodings decode instead of producing #UD, which v0.2 §5.1 requires ("or produces the fault prescribed by the guest contract"). Disposition: deferred to Sprint 021 — LOCK rule, VEX/EVEX L/W/vvvv constraints from the SDM opcode column.
  - *Observation 3 (structure held):* length_differs 0 across all generated cases.
- **Closed:** clean.

### Sprint 019 (2026-10-08, closed)
- **Scope:** EVEX and XOP prefix decoding; last corpus gaps.
- **Rubber Duck Pass:**
  - *Observation 1:* EVEX and XOP decoded correctly on the first run; one missing row (EVEX vpmaddwd) found by the differential and added. Disposition: resolved-here.
  - *Observation 2 (what this does not show):* full agreement on 2.44 M compiler-produced instructions. The literature (Paleari 2010; Woodruff et al. 2021) places decoder bugs in unusual encodings this corpus does not contain. Disposition: deferred to Sprint 020, generated encodings per spec row.
- **Files:** specs/x86_decode.json, scripts/gen_decoder.py, src/w2m/x86dec.py, src/w2m/x86dec_tables.py, tests/test_x86dec.py
- **Closed:** clean.

### Sprint 018 (2026-10-08, closed)
- **Scope:** XED-style iclass identity, corpus gaps closed except EVEX/XOP, CPU-feature dependence.
- **Rubber Duck Pass:**
  - *Observation 1 (a reference artifact that was a fact):* the four iclass differences on first run came from XED decoding for an unspecified chip. The bytes F3 0F BC are TZCNT on BMI1 CPUs and BSF on older ones; the results differ for a zero input. Disposition: recorded — Decision 2026-10-08 (decoder takes the CPU feature set); this is a semantic, not cosmetic, dependence.
  - *Observation 2 (independence held):* iclass names were assigned by rule before comparison; 99.997% matched XED on first run, and the residue was the chip effect, not naming errors.
  - *Observation 3 (remaining gaps):* 343 positions, 25 kinds, all EVEX or XOP. Disposition: deferred — EVEX/XOP prefix parsing is the next decoder sprint (implementation repair first, then rows).
  - *Observation 4 (vocabulary gap):* the generated-encoding test (mishegos-style, decision 2026-10-08 item 3) has not run; corpus agreement still says nothing about unusual encodings. Disposition: deferred — next after EVEX/XOP.
- **Files:** specs/x86_decode.json, scripts/gen_decoder.py, src/w2m/x86dec.py, src/w2m/x86dec_tables.py, tools/xedshim/xedshim.c, src/w2m/xedref.py, tests/test_x86dec.py, tests/test_xedref.py, scripts/decoder_differential.py
- **Closed:** clean.

### Sprint 017 (2026-10-08, closed)
- **Scope:** XED built from source and made the judge of length in the differential.
- **Rubber Duck Pass:**
  - *Observation 1 (what agreement means):* on 2.44 M compiler-produced instructions no decoder disagrees with XED on length. Disposition: recorded — consistent with the literature (decoder bugs live in unusual encodings); generated encodings are the next test, not more corpus.
  - *Observation 2 (traversal, not decoder):* the 15 Capstone failures are ASCII credit strings inside libcrypto's `.text`. Disposition: deferred — the traversal needs data detection for hand-written assembly (stop at bytes all decoders refuse; mark the region data); Deferred entry.
  - *Observation 3 (identity):* XED's iclass names separate what mnemonics conflate (MOVSD vs MOVSD_XMM). Disposition: deferred to Sprint 018 — the spec adopts XED iclass as each row's identity (a contract revision of `specs/x86_decode.json`).
- **Files:** tools/xedshim/xedshim.c, src/w2m/xedref.py, tests/test_xedref.py, scripts/decoder_differential.py
- **Closed:** clean.

### Sprint 016 (2026-10-08, closed)
- **Scope:** generalization test of the spec-generated decoder on 33 binaries it was not written from.
- **Rubber Duck Pass:**
  - *Observation 1 (vocabulary gap → worklist):* 203 gap kinds, all "no entry for opcode", none a structural failure; `specs/x86_decode_worklist.json` lists each with count and example bytes. Disposition: deferred — next sprint adds rows from the SDM tables, highest counts first, and reruns.
  - *Observation 2 (payload anomaly, real bug):* VEX.L is not modeled; 0F 77 with L=1 is vzeroall. Disposition: deferred to the same sprint (spec schema gains an `l` constraint).
  - *Observation 3 (reference uncertainty):* `repz ret` is Capstone's spelling of F3 C3; Intel's manuals describe REP with RET as having no defined effect. Disposition: recorded — mnemonic spelling for prefix-on-non-string instructions follows the reference; the spec gains a rule rather than a guess.
  - *Observation 4 (reference failure):* Capstone cannot decode 15 reachable locations. Disposition: deferred — check each with LLVM; where LLVM decodes them, the reference is wrong there.
- **Files:** scripts/decoder_differential.py, specs/x86_decode_worklist.json, corpus/python-3.14.8.manifest.json
- **Closed:** clean.

### Sprint 015 (2026-10-08, closed)
- **Scope:** spec-generated x86-64 decoder, compared with Capstone at every reachable instruction.
- **Rubber Duck Pass:**
  - *Observation 1 (the differential's first finding):* 94 instructions, one class: F2 0F 10/11 (SSE scalar-double move, mnemonic `movsd`) was printed `repne movsd` because `movsd` also names the string move A5, for which F2 is a repeat prefix. Disposition: resolved-here — REP fusion is limited to the one-byte map. The two-decoder plan found a name collision on its first run.
  - *Observation 2 (selection effect):* the spec was authored from the observed mnemonic list, so 100% here is not evidence of generality. Disposition: deferred — next sprint runs both decoders on binaries neither the spec nor its author has seen; DecodeError and disagreement counts become the worklist.
  - *Observation 3 (vocabulary gap):* operands are not compared yet; the translator needs operand semantics, not mnemonics. Disposition: deferred — operand comparison is a later layer of the same differential.
- **Files:** specs/x86_decode.json, scripts/gen_decoder.py, src/w2m/x86dec.py, src/w2m/x86dec_tables.py, tests/test_x86dec.py
- **Closed:** clean.

### Sprint 014 (2026-10-08, closed)
- **Scope:** Capstone reference traversal; Capstone graded against LLVM.
- **Rubber Duck Pass:**
  - *Observation 1 (payload anomaly, resolved-here):* the first Capstone/LLVM comparison showed 42-47 length disagreements per binary; every one was LLVM printing `lock` as a 1-byte line. A second pass mis-folded `rep` (LLVM prints `rep` with its instruction on one line). After normalizing the printing, agreement is total. Lesson recorded: disagreements are first checked for presentation before they count as decoder facts.
  - *Observation 2 (vocabulary gap → translator input):* two imports in event_auto_reset.exe are never called through their IAT slot: the compiler loads the slot into a callee-saved register (`mov r12, [rip+slot]` … `call r12` for CreateThread; `mov rsi, [rip+slot]` … `call rsi` for RegGetValueW) and calls through it. The traversal counted these as unresolved indirect calls. Disposition: recorded — import resolution needs one step of register data flow; the agent's first explanation ("passed as a value") was a guess and was replaced by the instruction listing.
  - *Observation 3:* 42 and 47 indirect transfers remain unresolved by static traversal (jump tables, function pointers). Disposition: deferred to the dynamic discovery path (v0.2 §5.1).
- **Files:** src/w2m/x86ref.py, tests/test_x86ref.py, src/w2m/pe.py, pyproject.toml
- **Closed:** clean.

### Sprint 013 (2026-10-08, closed)
- **Scope:** PE32+ reader graded against llvm-objdump on two runner-built binaries.
- **Rubber Duck Pass:**
  - *Observation 1 (vocabulary gap → contract inventory):* event_auto_reset.exe's source calls about a dozen Windows functions; the image imports 81 (KERNEL32 79, ADVAPI32 1, bcrypt 1). The statically linked C runtime contributes the rest. v0.2 §7: import scans are a coverage view; the first real inventory is dominated by CRT startup and I/O, not program logic. Disposition: recorded — the contract inventory sprint takes CRT entry points as first-class members.
  - *Observation 2 (resolved-here):* the first download loop fetched one run's artifact under both names; the probes were re-fetched by run id and their sha256 compared to the runner's.
  - *Observation 3 (resolved-here):* mypy flagged the data-directory list as variable-length tuples; typed explicitly.
- **Files:** src/w2m/pe.py, tests/test_pe.py, store/binaries/
- **Closed:** clean.

### Sprint 012 (2026-10-08, closed)
- **Scope:** v0.2 §8's two-page example on this Mac: correctness on both backends, then execution time per access.
- **Dual contract:** signal pass; artifact pass (see Built).
- **Observation contract:** pass.
- **Rubber Duck Pass:**
  - *Sequence narration:* SESSION_INIT (mac-Mac17,7-macos-25F80, capability). CAPABILITY_PROBED native_mprotect_4k absent (EINVAL); native_page_check_4k present; guest_page_protection_4k present (EC 0x25, FAR 0x20001000). 10 MEASUREMENT_RECORDED. SESSION_COMPLETED.
  - *Observation 1 (timing surprise, resolved-here):* the first check_page_cross ran at 1.9x — as much as checking every access — because its common path took a branch on 1023 of 1024 elements, doubling taken branches per iteration. Moving the page-entry check out of line made it 1.04x. The first number measured code layout, not the check.
  - *Observation 2 (vocabulary gap, deferred):* check_page_cross is valid only while no other thread changes protection during the loop. Windows lets any thread call VirtualProtect at any time, so a hoisted check needs an invalidation rule (a protection epoch the loop re-checks, or re-translation on change) — the v0.2 §5.1 code-validity machinery applied to data protection. Deferred entry added.
  - *Observation 3 (vocabulary, resolved-here):* the three capabilities had no enum values; P10 ratified into 0.3 within the sprint.
- **Files:** probes/page_geometry/page_geometry.c, scripts/page_geometry.py, signals/0.3.json, signals/0.3-rationale.md
- **Closed:** clean.

### Sprint 011 (2026-10-07, closed)
- **Scope:** execution time of x86-TSO ordering lowerings: native lowerings vs VM hardware TSO, L1 and DRAM-sized working sets.
- **Dual contract:** signal pass; artifact pass (see Built).
- **Observation contract:** pass, after two corrections the observation itself forced.
- **Rubber Duck Pass:**
  - *Observation 1 (payload anomaly, resolved-here):* first full run showed the VM 6.2x slower than native on the DRAM-sized set. The VM path had no warm pass while the native path did; the number included first-touch stage-2 faults on 128 MiB. With a warm pass the VM is 1.03-1.10x. Disposition: resolved-here; the 6.2x figure was never recorded as a measurement.
  - *Observation 2 (missing pair, resolved-here):* the native MP precondition passed with plain 0/10,000,000 — a check with no power. The driver now requires plain > 0 in the same attempt (attempt 7 had 9).
  - *Observation 3 (timing surprise):* absolute ns/access moved up to 50% between runs (L1 plain 0.087 then 0.131), ratios held within 0.01. Cause not established; likely core placement or clock state of an unpinned thread. Disposition: deferred — MEASUREMENT_RECORDED values are only comparable within one record; Drift watchlist entry.
  - *Observation 4 (vocabulary gap):* VM entso_on vs entso_off on the DRAM set differs by 7% at the median but only 1.4% at the minimum; not separable from noise with 5 repeats. Disposition: recorded — TSO hardware cost reported as "under 10% on this kernel", not as a number.
- **Files:** probes/ordering_cost/ordering_cost.c, scripts/ordering_cost.py
- **Closed:** clean.

### Sprint 010 (2026-10-07, closed)
- **Scope:** vocabulary 0.1 → 0.2 by proposal (P8, P9); version wiring through generator, gate, ingest.
- **Dual contract:** signal vacuous; artifact pass (see Built).
- **Rubber Duck Pass:**
  - *Observation 1 (order violation, hard rule 6):* the sprint touched 12 files. Disposition: recorded — a version bump must land whole, or the gate fails between sprints; stated in the card before execution.
  - *Observation 2 (payload anomaly):* TRACE_INGESTED.vocab_version previously echoed the harness version, not the trace's; under one version they coincided and the error was invisible. Disposition: resolved-here (records the trace's own version; tested).
- **Closed:** clean.

### Sprint 009 (2026-10-07, closed)
- **Scope:** demonstrate that ACTLR_EL1.EnTSO changes guest memory ordering; MP litmus on two vCPUs, rounds alternating off/on.
- **Dual contract:** signal pass; artifact pass (see Built).
- **Observation contract:** pass — the "effect demonstrated" branch.
- **Rubber Duck Pass:**
  - *Sequence narration:* SESSION_INIT (mac-Mac17,7-macos-25F80, capability). CAPABILITY_PROBED guest_tsoen present: off 2245/50,000,000, on 0/50,000,000, bound 6.0e-08, effect demonstrated. CAPABILITY_PROBED guest_tsoen_context_switch present: 200 exits, 0 mismatches. SESSION_COMPLETED.
  - *Observation 1 (timing surprise):* the off-mode rate was 1.2e-03 in a 4-round trial (233/200,000) and 4.5e-05 in the 100-round run. Disposition: recorded — the rate depends on timing and round length; the claim rests on off > 0 and on = 0 in alternating rounds, not on the rate.
  - *Observation 2 (vocabulary gap):* MP checks load-load and store-store order only. x86-TSO also *allows* store-load reordering (SB), and a backend must not be stronger than it claims to measure nor weaker than TSO elsewhere (LB, 2+2W, IRIW). Disposition: deferred — the herdtools7 x86-TSO suite on two to four vCPUs is a later sprint; Deferred entry added.
  - *Observation 3 (payload anomaly):* the guest MMU had to be on: with SCTLR_EL1.M=0, EL1 data accesses are Device-nGnRnE, which would have produced a false "TSO" result in the off mode. Disposition: resolved-here (identity map, Normal WB ISH).
- **Files:** probes/mac_hv_litmus/mp_litmus.c, scripts/litmus_mp.py
- **Closed:** clean.

### Sprint 008 (2026-10-07, closed)
- **Scope:** Hypervisor.framework capability probe on this Mac; CAPABILITY_PROBED on a substrate record.
- **Dual contract:** signal pass; artifact pass (see Built).
- **Observation contract:** pass.
- **Rubber Duck Pass:**
  - *Sequence narration:* SESSION_INIT (profile mac-Mac17,7-macos-25F80, purpose capability). CAPABILITY_PROBED ipa_granule_4k present; guest_tsoen present; guest_tsoen_context_switch present; host_page_16k present; native_process_tso absent. SESSION_COMPLETED (completed).
  - *Observation 1 (tone trace):* a detail string began with "ACTLR_EL1", against E12 (lowercase first word). Disposition: resolved-here.
  - *Observation 2 (vocabulary gap):* `guest_tsoen_context_switch` was observed across one vCPU exit, not across host thread migration or many exits; the capability name claims more than one exit shows. Disposition: deferred — the litmus sprint runs thousands of exits on two vCPUs and re-records it.
  - *Observation 3 (timing):* the probe and the driver ran first time; Hypervisor headers in the installed SDK were the bridge mapping (TECHNIQUES.md #46), no symbol was guessed.
- **Files:** probes/mac_hv_capability/hv_capability.c, probes/mac_hv_capability/entitlements.plist, scripts/probe_capabilities.py
- **Closed:** clean.

### Sprint 007 (2026-10-07, closed)
- **Scope:** the event contract's probe on real x64 Windows, 10 executions, judged on one substrate record.
- **Dual contract:** signal pass; artifact pass (see Built).
- **Observation contract:** pass. Judge record: 106 events, 21 producers started, 0 failed; 10 of 10 conforms. profile_id `gha-windows-2025-x64-10.0.26100.33438-Server`.
- **Rubber Duck Pass:**
  - *Sequence narration (evt-1):* main 6680 started; SetEvent returned 1; join of waiter 5880 returned 0 after 5880:2; join of 4328 returned 0 after 4328:2; WaitForSingleObject(event,0) returned 258. Waiter a's wait returned 0 citing main's SetEvent line; waiter b's returned 258.
  - *Observation 1 (timing surprise):* waiter a released in 10 of 10 executions; the profile looks FIFO. Disposition: recorded — documented evidence keeps {b} allowed (v0.2 §11.1: repeated observations are witnesses, not rules); open question added to 0.2.0 for a wait-order variant.
  - *Observation 2 (payload anomaly, resolved):* the Sprint 006 watchlist item: profile_id now carries UBR 33438 and `Server`. Disposition: resolved-here; watchlist item closed.
  - *Observation 3 (missing pair):* the imported executions carry `elapsed_seconds=0.0` on EXECUTION_COMPLETED and timeout 600 s (job limit) on EXECUTION_STARTED, not measured values, because the probe ran on the runner, not under the harness clock. Disposition: deferred — the probe writes its own elapsed time in a later probe-format revision.
- **Files:** probes/event_auto_reset/event_auto_reset.c, scripts/judge_traces.py, .github/workflows/windows-probe.yml, contracts/.../0.2.0.json
- **Closed:** clean.

### Sprint 006 (2026-10-07, closed)
- **Scope:** private repo; hello probe built and run on the `windows-2025` hosted runner; trace ingested on the Mac.
- **Dual contract:** signal pass; artifact pass (run 37718681787 conclusion success; ingest TRACE_INGESTED line_count=8 thread_count=2; `uv run pytest` 38 passed).
- **Observation contract:** pass. profile_id `gha-windows-2025-x64-10.0.26100`; runner Get-FileHash FD933867...555A equals Mac shasum.
- **Rubber Duck Pass:**
  - *Sequence narration (guest plane):* OPENED hello-001 on 10.0.26100. Thread 6588 started, read QPC 2267788232. Thread 8820 started after 6588:1, read QPC 2267788974, exited. Thread 6588 read QPC 2267789575 after 8820:2. CLOSED with 8 lines, 2 threads, 0 dropped.
  - *Observation 1 (order):* QPC values rise along both hb edges (…232 < …974 < …575), as the edges require. Disposition: resolved-here (no fault; first observed evidence that QPC is consistent across threads on this profile — one run, a witness, not a rule).
  - *Observation 2 (payload anomaly):* profile_id carries 10.0.26100 but not the update build revision (the runner README lists Build 33438) nor that the OS is Windows Server, not a client SKU. v0.2 §11.1 asks for the Windows build. Disposition: deferred — the next probe reads UBR and product type from the registry; Drift watchlist.
  - *Observation 3 (timing):* first dispatch (run 37718622857) failed at build: MSVC CRT deprecates `fopen` and `-Werror` promoted it. Disposition: resolved-here (`_CRT_SECURE_NO_WARNINGS`, commit after 052383a).
- **Files:** probes/hello/hello.c, .github/workflows/windows-probe.yml, .gitignore
- **Closed:** clean.

### Sprint 005 (2026-10-05, closed)
- **Scope:** comparator for allowed_outcome_set; substrate Producer triggered by TRACE_INGESTED.
- **Dual contract:** signal pass; artifact pass (`uv run pytest` 37 passed; ruff, mypy exit 0).
- **Observation contract:** pass. {b} trace → `COMPARISON_COMPLETED(verdict=conforms, relation=allowed_outcome_set)`; {a,b} trace → `verdict=discrepancy`, first_divergence "forbidden outcome both_waits_complete_from_the_single_signal: ['a', 'b'] at ['2:1', '3:1']".
- **Rubber Duck Pass:**
  - *Sequence narration:* EXECUTION_COMPLETED (e1, exited) → a probe trace exists. TRACE_INGESTED (same hash) → the trace is evidence-grade. COMPARISON_COMPLETED (cmp1, contract 0.1.0, verdict) → the trace was judged against the proposed contract.
  - *Observation 1 (payload anomaly):* `reference_ref="ref-e0"` names no reference execution; constraint E13 cannot resolve it. Disposition: deferred — no reference run exists until the Windows reference instance does; Drift watchlist entry added.
  - *Observation 2 (order):* the comparator re-decodes the trace; a malformed trace would raise inside the Producer. Disposition: resolved-here by construction — the comparator is triggered only by TRACE_INGESTED (S4 chain), so it only reads validated traces.
- **Files:** src/w2m/compare.py, tests/test_compare.py
- **Closed:** clean.

### Sprint 004 (2026-10-05, closed)
- **Scope:** contract format; first contract from v0.2 §11.2.
- **Dual contract:** signal vacuous; artifact pass (27 passed; ruff, mypy exit 0).
- **Rubber Duck Pass:** *Observation 1 (vocabulary gap):* the postcondition `signaled_after_consumption: false` is observable through GUEST_CALL_RETURNED (main's zero-timeout wait returning 258, WAIT_TIMEOUT per Microsoft Learn) — P4 not needed for this contract. Disposition: resolved-here. *Observation 2 (drift):* contract layout differs from the rationale's audit table. Disposition: recorded (Decision 2026-10-05).
- **Files:** src/w2m/contract.py, contracts/windows.event.auto_reset.single_signal/0.1.0.json, tests/test_contract.py
- **Closed:** clean.

### Sprint 003 (2026-10-05, closed)
- **Scope:** guest-trace ingest with hash, strict decode, T5, T6, S14, and hb_refs checks; substrate Producer wrapper.
- **Dual contract:** signal pass; artifact pass (`uv run pytest` 18 passed exit 0; ruff, mypy, `gen_events.py --check`, `check_vocab.py` all exit 0).
- **Observation contract:** pass. Record of `test_observation_ingested_on_the_record` narrated by `substrate narrate`: 12 events, 2 producers started, 2 completed, 0 failed.
- **Rubber Duck Pass:**
  - *Sequence narration:* seq 3 EXECUTION_STARTED (e1, windows_reference, timeout 60 s, grace 5 s) → reference_runner began a probe. seq 4 EXECUTION_COMPLETED (exited, 8 lines, trace sha256:fccec9fb...) → the probe ended and its trace exists. seq 5 trigger ingest-on-complete started trace_ingester. seq 8 TRACE_INGESTED (same hash, 8 lines, 2 threads, vocab 0.1) → T4 pairing satisfied.
  - *Observation 1 (payload anomaly):* `kill_grace_seconds` written as 5.0 is recorded as 5 (RFC 8785). Disposition: resolved-here — msgspec reads it back as 5.0; the round-trip is now asserted in the observation test.
  - *Observation 2 (timing surprise):* none; whole suite 0.13 s.
- **Files:** src/w2m/trace.py, tests/test_trace.py
- **Closed:** clean.

### Sprint 002 (2026-10-05, closed)
- **Scope:** generated Structs, `validate`, strict guest decoding.
- **Dual contract:** signal pass; artifact pass (7 tests; `--check`, ruff, mypy exit 0).
- **Rubber Duck Pass:**
  - *Observation 1 (payload anomaly):* msgspec cannot express a `le` above int64; the first `u64` alias crashed decoder construction. Disposition: resolved-here (u64 is `ge=0` only).
  - *Observation 2 (payload anomaly):* substrate refuses harness-plane integers beyond 2^53-1. Disposition: recorded — generated bound, Decision 2026-10-05, proposal P8.
  - *Observation 3 (vocabulary gap):* `types.version` text says major.minor.patch, but the vocabulary's own version is "0.1". Disposition: deferred — P8.
- **Files:** scripts/gen_events.py, src/w2m/events.py, tests/test_events.py
- **Closed:** clean.

### Sprint 001 (2026-10-05, closed)
- **Scope:** package scaffold and dependencies. **Dual contract:** signal vacuous; artifact pass. **Rubber Duck Pass:** vacuous (no runtime signals). **Closed:** clean.

### Sprint 000 (2026-10-05, closed)
- **Scope:** Vocabulary Session for w2m against `windows_to_macos_research_direction_v0_2.md`.
- **Dual contract:** signal vacuous (no runtime emission); artifact pass — `json.load` on `signals/0.1.json` exit 0, on `signals/proposals.json` exit 0; `python3 scripts/check_vocab.py` exit 0; assertions: SESSION_INIT carries vocab_version/profile/contract_version; DISCREPANCY_REPORTED is incident with contract_id/mismatch_class/minimized_program_sha256; mismatch_class enum is the six v0.2 §3 relations; rationale has `Dual-contract audit` and `Sign-off` sections.
- **Observation contract:** not applicable (architecture, content-only).
- **Rubber Duck Pass:**
  - *Sequence narration:* no runtime signals; the narrated trace is the session's own steps: Step 0 orientation, Layers 0-7 written, Step 9 audit, Steps 10-12 sign-off, gate run.
  - *Observation 1 (payload anomaly):* the rationale said "37 enums"; the file has 34. Disposition: resolved-here (rationale corrected; Built entry uses 34).
  - *Observation 2 (order violation, card scope):* `scripts/check_vocab.py` was authored though the card listed three files. The first gate run caught a false failure (READ as a non-past verb) that only a written allowlist fixes, and WORKING_AGREEMENT had no vocabulary gate command. Disposition: resolved-here — the file is the project's gate, registered in WORKING_AGREEMENT; one concept (the lock) kept.
  - *Observation 3 (vocabulary gap):* APPLICATION_LAUNCHED has no view-side counterpart. Disposition: deferred — proposal P2 WINDOW_PRESENTED; Drift watchlist.
  - *Observation 4 (citation slip):* the reference-machine Decision cited the wrong addendum item. Disposition: recorded — correction entry in `## Decisions`.
- **Files:** signals/0.1.json, signals/0.1-rationale.md, signals/proposals.json, scripts/check_vocab.py
- **Closed:** clean.

### Sprint 168 (2026-10-28, closed)
- **Scope:** the user's instruction of 2026-10-28: the gates run, by default, what the change since `origin/sprint-066` reaches, by established methods; `--all` keeps every gate.
- **Dual contract:** signal — none, as the card says (the gates write no record); artifact pass (`scripts/gates.sh` exit 0 on this sprint's change; `scripts/gates.sh --all` exit 0 in 844 s); observation pass (6 s for this sprint's change against 13 min 42 s for Sprint 167's full gates; plant 1 failed at its test in 4 s with 14 of 126 tests selected; plant 3 chose the Python gates).
- **CI does not run:** `scripts/gates.sh` in either mode; CI runs its own steps, unchanged.
- **Rubber Duck Pass:** *Sequence narration:* no record. *Observation 1 (a fault the new tool found in an old script):* nextest's first run failed at "creating test list failed"; `sign_and_run.sh` ran `codesign -f` from every process that ran a binary, and 11 of 20 concurrent runs failed; it now signs once under a lock, 20 of 20. Disposition: resolved-here. *Observation 2 (a card's invariant that was not so):* the card said CI runs `gates.sh`; CI runs its own steps. Disposition: recorded (the result says so). *Observation 3 (a plant changed from the card):* the card's plant 3 edited a Python test; the frozen Python is not edited, so a file under `tests/` showed the selection, and pytest's own failing stays pytest's. Disposition: recorded. *Observation 4 (the selection's limits, stated):* Rust tests read files at run time (`probes/`, `build/probe-runs/`, `signals/`) that no crate graph shows; any change outside one crate runs them all, and `build/` is outside git, so a new download is not a change; a step's close runs `--all`. Disposition: recorded. Edits outside the Edit tool: one-line `sed`s for WORKING_AGREEMENT's build line, the plants and the BLACKBOARD Decision's numbers.
- **Closed:** Sprint 167's CI run 40657214806 (success on 5dc5870db).
- **Files:** scripts/gates.sh, scripts/sign_and_run.sh, docs/sprint-tail-archive-2026-10-08.md (Sprint 158's entry), WORKING_AGREEMENT.md, BLACKBOARD.md, sprints/sprint-168-the-gates-nearest-the-change.md, signal-reports/sprint-168-report.md.

### Sprint 167 (2026-10-28, closed)
- **Scope:** reviewer-2's Medium 4 of 2026-10-27: a Rust case runner's discrepancy follows a REDUCTION_COMPLETED and names its case by program and initial state, so rules S5 and E1 hold.
- **Dual contract:** signal — SESSION_INIT 4, CANDIDATE_GENERATED, COMPARISON_COMPLETED and CANDIDATE_ACCEPTED 512 each, SESSION_COMPLETED 4 (completed) on four `case-runner-rust` records, every tag the card named for them; on the planted run's record (outside `store/runs/`) CANDIDATE_REJECTED, REDUCTION_COMPLETED, DISCREPANCY_REPORTED and DISCREPANCY_ROUTED 32 each, as the card named; artifact pass (fmt, clippy, the gates exit 0); observation pass (the hashes equal the frozen Python's on 13 committed sets, a YMM and an x87 case, and the planted record's first case; `check_records` 0 S5 and E1 violations on the planted record, 32 S5 on the misordered plant).
- **CI does not run:** the case runner, `case_runs` and the planted run (macOS only); CI runs `case_hashes`' test, whose sets are committed.
- **Rubber Duck Pass:** *Sequence narration:* each record opens with SESSION_INIT for one reference profile, then per class CANDIDATE_GENERATED, COMPARISON_COMPLETED and CANDIDATE_ACCEPTED, and closes with SESSION_COMPLETED completed; the planted record adds, for each of 32 CF classes, CANDIDATE_REJECTED, REDUCTION_COMPLETED (steps 0), DISCREPANCY_REPORTED (both hashes named) and DISCREPANCY_ROUTED. *Observation 1 (a check kept out on purpose):* the format-3 and format-4 sets are built, not committed, so their check ran once and is not a test; a test that skips where its file is absent is KIT_DIARY #65's fault. Disposition: recorded. *Observation 2 (a line the result first left open):* the result's draft said the planted record's hashes were not checked against the case that failed; the Python's hashes of `semantics_041` case 0, the case the report names, settled it before the close. Disposition: resolved-here. *Observation 3 (an order the next card needs changed):* P22 changes T4, which the frozen Python's `check_records.py` holds in code; Decision 2026-10-28 (the records check in Rust first) puts it before vocabulary 0.12. Disposition: resolved-here. Edits outside the Edit tool: one-line `sed`s for imports and a pattern; the plant was made with the Edit tool and undone by restoring the saved file.
- **Closed:** Sprint 166's CI run 40654664616 (success on ff613db5b).
- **Files:** crates/cpu_cases/ (lib.rs, tests/case_hashes.rs), crates/w2m_vm/ (cases.rs, bin/case_runs.rs), store/runs/case-runner-rust-1793175258000000000 and three more, docs/sprint-tail-archive-2026-10-08.md (Sprint 157's entry), WORKING_AGREEMENT.md, BLACKBOARD.md, sprints/sprint-167-a-discrepancy-names-its-case.md, signal-reports/sprint-167-report.md.

### Sprint 166 (2026-10-28, closed)
- **Scope:** the code review's finding 7, its two silent fallbacks: a condition `inverse` or `cond_flags` does not name is refused by name, as the Python's dicts raise KeyError.
- **Dual contract:** signal — SESSION_INIT 4, CANDIDATE_GENERATED, COMPARISON_COMPLETED and CANDIDATE_ACCEPTED 512 each (205, 174, 107, 26), SESSION_COMPLETED 4 (completed) on four `case-runner-rust` records, every tag the card named and none it did not; no discrepancy; artifact pass (fmt, clippy, the gates exit 0); observation pass (the C words judge's 1,184,313 calls compared with no refusal and no difference, as before; plant 1 named by the unit test at `inverse("xx")`; plant 2 named only by the unit test).
- **CI does not run:** the case runner and `case_runs` (macOS only); CI runs the C words judge and the two new unit tests.
- **Rubber Duck Pass:** *Sequence narration:* each record opens with SESSION_INIT for one reference profile, then per class CANDIDATE_GENERATED, COMPARISON_COMPLETED and CANDIDATE_ACCEPTED, and closes with SESSION_COMPLETED completed. *Observation 1 (a plant the card said a judge would name, which none did):* `cond_flags`'s `le` arm dropped passed the case runner, the C words judge and both guests; `compile_insn` reads the flag masks only for an instruction holding a flags op, which no Jcc, SETcc or CMOVcc does, so `cond_flags` is reached only through `assemble_block`'s and the run loop's `insn_flags`, and no judged block branches on `le`. The card's line was written without reading which path calls the function. Disposition: resolved-here (a test holds `lift::cc`'s 16 conditions to the table; the card's result says the batch run is the first end-to-end judge that may reach them). It is KIT_DIARY's H5 again: the plant's coverage was a claim in the card's prose. *Observation 2 (a fallback kept where a Result cannot go):* the run loop's look-ahead returns a mask, so on the error it counts every flag live, which keeps the code right; the instruction's compile refuses it. Disposition: recorded. Edits outside the Edit tool: one-line `sed`s for three `?`s and the plants.
- **Closed:** Sprint 165's CI run 40652354538 (success on d855236c9); Sprint 164's CI run 40650672275 (success on 605f0ce55).
- **Files:** crates/translator/src/lower.rs, crates/w2m_vm/src/compile.rs, store/runs/case-runner-rust-1793173959000000000 and three more, docs/sprint-tail-archive-2026-10-08.md (Sprint 156's entry), WORKING_AGREEMENT.md, BLACKBOARD.md, sprints/sprint-166-unknown-conditions-refused.md, signal-reports/sprint-166-report.md.

### Sprint 165 (2026-10-28, closed)
- **Scope:** KIT_DIARY's entry for Sprints 150-164, with step 3's synthesis and hypothesis H5.
- **Dual contract:** signal — none, as the card says (no producer runs; the Signal Report says the card names no tag and the sprint made no record); artifact pass (the gates exit 0); observation pass (every number in the entry read back against the records, the tail and the archive before the commit: 40 events and three exempt fields at 158; 3,667 ns at 142; 584-1,042 ns and 813-852 ns at 163; WFE 0 and WFI 1,000; 83 ns over 3,018 takings, 246 µs, 57.5 µs, 163 and 179 ticks, 2,914 of 3,002 at 164; the sprints of each recurring class).
- **CI does not run:** nothing this sprint adds; CI runs the workspace's tests as before.
- **Rubber Duck Pass:** *Sequence narration:* no record; the work is one KIT_DIARY entry and a hypothesis row. *Observation 1 (two numbers wrong in the first draft):* the entry said the Signal Report gate had stood "five weeks"; it is from 2026-10-24, four days and 37 sprints; and it called the ring's count "one count away", which said nothing. Both corrected before the commit by the read-back the card required. *Observation 2 (a class recorded under the wrong sprint):* the card's draft put a glob at 149; 149's fault was a command in a zsh variable, not word-split, and the glob was 159's. Corrected in the card before its commit. Disposition: resolved-here (both).
- **Closed:** Sprint 164's CI run 40650672275 on 605f0ce55, in progress at this close; its result is recorded in Sprint 166's tail entry.
- **Files:** KIT_DIARY.md, docs/sprint-tail-archive-2026-10-08.md (Sprint 155's entry), BLACKBOARD.md, sprints/sprint-165-kit-diary-and-step-3s-synthesis.md, signal-reports/sprint-165-report.md.

### Sprint 164 (2026-10-28, closed)
- **Scope:** step 3's third measurement on a record: how long the runtime word is held per taking, and how long a taker waits, by site.
- **Dual contract:** signal — SESSION_INIT 1 (purpose measurement, 0.11), MEASUREMENT_RECORDED 41, SESSION_COMPLETED 1 (completed) on `w2m-vm-measurements-1793171817000000000`, every tag the card named and none it did not; artifact pass (fmt, clippy, the gates exit 0); observation pass (each guest to ExitProcess, the timed `sleeper`'s output equal to the untimed; no time backward and no taking unwritten; the planted slot swap named "qpc: 15 takings, 15 backward").
- **CI does not run:** `run_measurements` and `w2m-vm` (macOS only, Hypervisor.framework).
- **Rubber Duck Pass:** *Sequence narration:* SESSION_INIT for w2m-vm-measurements, then per guest the exit kinds' `vm_exits`, `exit_ns_p50` and `exit_ns_p99`, then per site the runtime word's hold and wait p50 and p99 (switch, bridge, bridge across an exit, compile), then `wfe_exits` for wfe (0) and wfi (1,000), then SESSION_COMPLETED after 1.609 s. *Observation 1 (an instrument sized by guess, twice):* the card's 48-entry ring lost 2,914 of 3,002 takings; the histograms that replaced it mixed four sites, stopped below the design's 10 µs and carried a bound in prose. Disposition: resolved-here (an entry per taking, in the 1.3 MB the monitor region leaves free). *Observation 2 (a card's sentence the record contradicted):* "each wait is a few ticks" fails for each run's first taking, 163 and 179 ticks with no exit inside; a host write to the page first did not remove it, and the cause is not measured. Disposition: recorded (the result corrects the sentence). *Observation 3 (a counter read without its barrier):* `mrs cntvct_el0` had no ISB before it, as Linux puts one. Disposition: resolved-here. *Observation 4 (an order changed outside a Decision):* the code review's dispositions moved the locked XCHG to Sprint 165, against Decision 2026-10-27's order. Disposition: resolved-here (Decision 2026-10-28). Edits outside the Edit tool: one-line `sed`s for imports, call sites, the plants and their restoring, and the card's line citation.
- **Closed:** Sprint 163's CI run 40646226401 (success on 6045d76f8).
- **Files:** crates/w2m_vm/ (monitor_code.rs, guest.rs, process.rs, run.rs, monitor.rs, bin/w2m-vm.rs, bin/run_measurements.rs), crates/a64_encoder/src/lib.rs, harness/w2m-vm-measurements.toml, store/runs/w2m-vm-measurements-1793171817000000000, docs/design-rust-host-2026-10-24.md (a dated note), docs/sprint-tail-archive-2026-10-08.md (Sprint 154's entry), WORKING_AGREEMENT.md, BLACKBOARD.md, sprints/sprint-164-the-runtime-words-hold-times.md, signal-reports/sprint-164-report.md.

### Sprint 163 (2026-10-27, closed)
- **Scope:** two of step 3's three measurements on a record: an exit's host time by kind on `qpc` and `sleeper`, and whether a guest WFE exits.
- **Dual contract:** signal — SESSION_INIT 1 (purpose measurement, 0.11), MEASUREMENT_RECORDED 17, SESSION_COMPLETED 1 (completed) on `w2m-vm-measurements-1793169234000000000`, every tag the card named and none it did not; artifact pass (fmt, clippy, the gates exit 0); observation pass (each guest to ExitProcess, its served exits equal to its tally less one; p50 ≤ p99 for every kind; WFE 0 and WFI 1,000; the planted quantile named "p50 … left: 30, right: 20").
- **CI does not run:** `run_measurements` and `w2m-vm` (macOS only, Hypervisor.framework); CI runs `record.rs`'s quantile test.
- **Rubber Duck Pass:** *Sequence narration:* SESSION_INIT for the profile w2m-vm-measurements, then per guest and exit kind a `vm_exits` count and `exit_ns_p50` and `exit_ns_p99`, then `wfe_exits` for wfe (0) and wfi (1,000), then SESSION_COMPLETED after 0.918 s. *Observation 1 (a sample too small for its quantile):* `qpc` and `sleeper` serve 1 to 14 exits a kind, so p99 is the largest; the hvc 1 p99s are compiles. Disposition: recorded (stated in the card; step 4's batch run gives the distribution the host design needs). *Observation 2 (a plant the card set that could not fail):* the card named a 3-sample test, where both rules give the same median; the test uses four. Disposition: resolved-here. *Observation 3 (an answer the design waited for):* WFE does not exit; the design's alternative (a spin ending in WFE) is closed and its hvc stands. Disposition: recorded.
- **Closed:** Sprint 162's CI run 40644147884 (success on 74e8fe348).
- **Files:** crates/w2m_vm/ (run.rs, record.rs, bin/w2m-vm.rs, bin/run_measurements.rs), harness/w2m-vm-measurements.toml, store/runs/w2m-vm-measurements-1793169234000000000, docs/sprint-tail-archive-2026-10-08.md (Sprint 153's entry), WORKING_AGREEMENT.md, BLACKBOARD.md, sprints/sprint-163-an-exits-time-and-wfe.md, signal-reports/sprint-163-report.md.

### Sprint 162 (2026-10-27, closed)
- **Scope:** vocabulary 0.11: the seven `metric` values step 3's measurements need (P24), the first part of reviewer-2's High 1.
- **Dual contract:** signal — MEASUREMENT_RECORDED 30 times (each of the seven new values at least once) and SESSION_INIT 2 on the samples record, which replays; the run also fired every other harness kind twice, which the card's Emits did not name; artifact pass (`vocabulary_generator --check`, fmt, clippy, the gates exit 0 on the fourth run); observation pass (the planted `exit_ns_p95` refused by the Rust type and by substrate: "Invalid enum value 'exit_ns_p95' - at `$.metric`").
- **CI does not run:** the samples runs and their plants (they run substrate in the gates); CI runs the generator's check and the workspace's tests.
- **Rubber Duck Pass:** *Sequence narration:* the samples record writes every live harness kind twice at sample values, then a MEASUREMENT_RECORDED for each of 0.11's 28 `metric` values; it replays at level 2. *Observation 1 (an Emits that under-declared, a third time):* the card named MEASUREMENT_RECORDED and SESSION_INIT for a run that fires every harness kind; Sprint 131 recorded the rule (name the plane's list) after Sprint 126 missed it, and this card missed it again. The card stays as written, as Sprint 127 left 126's. Disposition: recorded (a lesson in prose, not in a check: KIT_DIARY's next entry takes it up, with reviewer-2's Medium 3). *Observation 2 (three gates runs failed):* two at tests that pin the vocabulary's version as a literal, found one at a time before the class was searched whole; one at `check_records`, which held the new samples record because its name carried a nanosecond suffix, while earlier samples records escape by a timestamp name. Disposition: resolved-here (the literals read 0.11; the record renamed before its first commit; the card says the escape is by name). *Observation 3 (a check that could not fail):* the gates' check of Sprint 126's plant matched `vocab_version` in RunStarted's config; this sprint's first draft matched its own producer's name. Disposition: resolved-here (both read the ProducerFailed event). Edits outside the Edit tool: one-line `sed`s for the six baselines and two test literals.
- **Closed:** Sprint 161's CI run 40640091798 (success on fe056440f).
- **Files:** signals/0.11.json, signals/0.11-rationale.md, signals/rust-current.json, signals/proposals.json, crates/events/src/generated.rs, crates/vocabulary_samples/, crates/vocabulary_generator/tests/planted.rs, crates/session_events/src/main.rs, harness/, scripts/gates.sh, store/runs/vocabulary-samples-20261028T055041, docs/sprint-tail-archive-2026-10-08.md (Sprint 152's entry), WORKING_AGREEMENT.md, BLACKBOARD.md, sprints/sprint-162-vocabulary-0-11.md, signal-reports/sprint-162-report.md.

### Sprint 161 (2026-10-27, closed)
- **Scope:** link 5's fifth group, the SSE2 integer block: the 77 classes `_lower_int` lowers, lifted and lowered in Rust.
- **Dual contract:** signal — SESSION_INIT 4, CANDIDATE_GENERATED, COMPARISON_COMPLETED and CANDIDATE_ACCEPTED 512 each, SESSION_COMPLETED 4 (completed) on four `case-runner-rust` records, every tag the card named and none it did not; no discrepancy; artifact pass (fmt, clippy, the gates exit 0); observation pass (11,518 calls equal to the C; 131,649 case instructions in 289 classes agree with Windows; 15,716 codes equal to the Python's; two plants named by both judges).
- **CI does not run:** the case runner, the Python words judge and `case_runs` (macOS only, the downloaded results, the fixture). CI runs the C words judge.
- **Rubber Duck Pass:** *Sequence narration:* each of the four records opens with SESSION_INIT for one reference profile, then per class CANDIDATE_GENERATED (the Rust translator's for 93, 152, 107 and 26 classes), COMPARISON_COMPLETED and CANDIDATE_ACCEPTED (205, 174, 107, 26), and closes with SESSION_COMPLETED completed. *Observation 1 (a lowering behind a symbol):* the C judge's first run named PCMPISTRI, RCPPS and RSQRTPS: `compile_insn` calls the runtime's C for them whenever the symbols hold it, which `_lower_int`'s model fallback hides; `RuntimeCalls` carries the symbols. Disposition: resolved-here. *Observation 2 (a fact recorded wrong):* Sprint 153's result called the fourth profile's set "x87 cases alone"; its 26 classes are SSE2 integer classes, now all the Rust translator's. Disposition: resolved-here (stated in the card). *Observation 3 (a run started outside the tools):* the gates were started with a shell `&`, so no notification came; a background task waited on their exit file. Disposition: resolved-here. *Observation 4 (the direction):* reviewer-2's review arrived during the close: step 3's measurements were never taken and link 5's purpose had ended at 157. Disposition: recorded (Decision 2026-10-27; `## Surfaced for review`; the plan's step 3 amended).
- **Closed:** Sprint 160's CI run 40637621294 (success on 66ce853e9).
- **Files:** crates/translator/, crates/w2m_vm/, store/runs/case-runner-rust-1793165537000000000 and three more, docs/rust-port-plan-2026-10-24.md (step 3 amended), docs/sprint-tail-archive-2026-10-08.md (Sprint 151's entry), WORKING_AGREEMENT.md, BLACKBOARD.md, sprints/sprint-161-the-sse2-integer-block.md, signal-reports/sprint-161-report.md.

### Sprint 160 (2026-10-27, closed)
- **Scope:** link 5's fourth group, the SSE float core: `_sse.rule`'s 52 classes lifted and lowered in Rust, chosen by the C words judge's new census of unlifted calls (MOVAPS 21,645 first).
- **Dual contract:** signal — SESSION_INIT 4, CANDIDATE_GENERATED, COMPARISON_COMPLETED and CANDIDATE_ACCEPTED 512 each, SESSION_COMPLETED 4 (completed) on four `case-runner-rust` records, every tag the card named and none it did not; no discrepancy; artifact pass (fmt, clippy, the gates exit 0); observation pass (75,873 calls equal to the C; 110,401 case instructions in 212 classes agree with Windows; 13,032 codes equal to the Python's; two plants named, each by two or three judges).
- **CI does not run:** the case runner, the Python words judge and `case_runs` (macOS only, the downloaded results, the fixture). CI runs the C words judge.
- **Rubber Duck Pass:** *Sequence narration:* each of the four records opens with SESSION_INIT for one reference profile, then per class CANDIDATE_GENERATED (the Rust translator's for 65, 129, 107 and 0 classes), COMPARISON_COMPLETED and CANDIDATE_ACCEPTED (205, 174, 107, 26), and closes with SESSION_COMPLETED completed. *Observation 1 (a choice made from a count):* the group was chosen from a census the judge did not print until this sprint; the judge prints it now, so the next group's choice reads the same count. Disposition: resolved-here. *Observation 2 (the C's own refusals grew):* the C emitter refuses 260 of this group's calls (MOVSS 258), so they are compared with no C words; the case runner and the Python words judge the classes. Disposition: recorded. *Observation 3 (a loop interval changed against the user's words):* the user asked for a 7-minute loop and the first job ran every 6 minutes; replaced by a 7-minute job at the user's word. Disposition: resolved-here. One one-line `sed` changed an import line.
- **Closed:** Sprint 159's CI run 40631983395 (success on 7abf01df6).
- **Files:** crates/translator/, store/runs/case-runner-rust-1793163955000000000 and three more, docs/sprint-tail-archive-2026-10-08.md (Sprint 150's entry), WORKING_AGREEMENT.md, BLACKBOARD.md, sprints/sprint-160-the-sse-float-core.md, signal-reports/sprint-160-report.md.

### Sprint 159 (2026-10-27, closed)
- **Scope:** link 5's third group, the forms Sprint 153 left by name: the `_LOCK` forms, CMPXCHG16B, the system ops and the model's `x86` steps, 54 classes, lifted and lowered in Rust.
- **Dual contract:** signal — SESSION_INIT 4, CANDIDATE_GENERATED, COMPARISON_COMPLETED and CANDIDATE_ACCEPTED 512 each, SESSION_COMPLETED 4 (completed) on four `case-runner-rust` records, every tag the card named and none it did not; no discrepancy; artifact pass (fmt, clippy, the gates exit 0); observation pass (768 group-3 calls equal to the C; 75,713 case instructions in 160 classes agree with Windows; 11,041 codes equal to the Python's; two plants named, the first by both judges).
- **CI does not run:** the case runner, the Python words judge and `case_runs` (macOS only, the downloaded results, the fixture). CI runs the C words judge and the translator's unit tests, the new offsets test among them.
- **Rubber Duck Pass:** *Sequence narration:* each of the four records opens with SESSION_INIT for one reference profile, then per class CANDIDATE_GENERATED (the Rust translator's for 65, 77, 55 and 0 classes), COMPARISON_COMPLETED and CANDIDATE_ACCEPTED (205, 174, 107, 26), and closes with SESSION_COMPLETED completed. *Observation 1 (a count that counted nothing):* the first count of the records read three of four and found no events, because a zsh assignment does not expand a glob and the name pattern missed the first record; the empty counts were nearly read as empty records. The class is Sprint 149's Observation 1, met again. Disposition: resolved-here (counted again, the files listed first). *Observation 2 (a silent default):* the run loop's first draft gave CPUID address 0 when the runtime lacked `w2m_cpuid`, where the Python always passes its symbols; it is an error now. Disposition: resolved-here. *Observation 3 (a count said in chat before it was taken):* chat first gave group 3 as 34 classes in 21 files; the registry holds 33 locked and system classes in 23 files and 21 model classes, as the card says. Disposition: resolved-here. Every multi-line edit this sprint went through the Edit tool or `cargo fmt`; one `sed` set the card's status line to done.
- **Closed:** Sprint 158's CI run 40614424475 (success on 591c43034); the BLACKBOARD update's run 40626118444 (success on 33917d5af).
- **Files:** crates/translator/, crates/w2m_vm/, store/runs/case-runner-rust-1793160290000000000 and three more, docs/sprint-tail-archive-2026-10-08.md (Sprint 149's entry), WORKING_AGREEMENT.md, BLACKBOARD.md, sprints/sprint-159-the-locked-and-system-forms.md, signal-reports/sprint-159-report.md.

### Sprint 158 (2026-10-27, closed)
- **Scope:** link 6, the skeleton's gate: `sleeper`'s output and record from the Rust host equal the stored ones, the frozen Python host's.
- **Dual contract:** signal — SESSION_INIT, CODE_TARGET_DISCOVERED, TRANSLATION_PUBLISHED, SESSION_COMPLETED from `w2m-vm` on `store/runs/w2m-vm-sleeper-1793149355000000000`, read back by the golden check; artifact pass (substrate run 0, fmt, clippy, the gates); observation pass (40 events equal to the Python's record, the output equal to the stored one; two planted faults, a swapped block and a dropped byte, each failed it at the event or the output named).
- **CI does not run:** `w2m-vm` and the run loop's tests (macOS only), and the output's check (its file is under `build/`); CI runs the record's check over the two committed records.
- **Rubber Duck Pass:** *Observation 1 (three fields each host sets):* `vocab_version` differs, 0.9 in the frozen Python and 0.10 in Rust, beside `harness_version` and `elapsed_seconds`; the check names the three in `EXEMPT` and compares every other field. Disposition: resolved-here. *Observation 2 (a baseline key with a meaning):* `judge` marks the records the judges' ledger reads; the skeleton's record names `host` instead. Disposition: resolved-here. *Observation 3 (edits outside the Edit tool):* six one-line `sed` changes and an `awk`-built planted record, each read back. Disposition: recorded.
- **Closed:** Sprint 157's CI run 40253743671 (success on f2693f415).
- **This close's CI:** run 40614424475, success on 591c43034 (read 2026-10-27 from `gh run watch --exit-status`, exit 0; written here at the BLACKBOARD update of 2026-10-27, since no Sprint 159 entry exists to carry it).
- **Files:** crates/w2m_vm/, harness/w2m-vm-sleeper.toml, store/runs/w2m-vm-sleeper-1793149355000000000, store/runs/translations-1793149239949363000, store/skeleton/sleeper-stdout.txt, WORKING_AGREEMENT.md, BLACKBOARD.md, sprints/sprint-158-the-skeletons-gate.md, signal-reports/sprint-158-report.md.

### Sprint 157 (2026-10-25, closed)
- **Scope:** link 5's run-loop judge, `sleeper` to its end: THREAD_EXIT_CODE, THREAD_EXIT_STUB, THREAD_END and the `hvc #12` hand-over at a thread's end served by the Rust host.
- **Dual contract:** signal — none, as the card says (the skeleton's record comes at link 6); artifact pass (fmt, clippy, the gates); observation pass (ExitProcess(0) with the Python's output; the thread requests, the detach stub's bytes and the hand-over equal to the Python host's; two planted faults each failed it, one at the stop and the requests, one at the exit code read back).
- **CI does not run:** the run loop's two tests and the hypervisor crate's vector register test (macOS only; the run loop's need the Python fixtures).
- **Rubber Duck Pass:** *Observation 1 (a declaration stable Rust refuses):* `hv_vcpu_set_simd_fp_reg` takes its 16 bytes by value as a short vector, which AAPCS64 passes in v0; stable Rust rejects `uint8x16_t` in an `extern "C"` signature (`simd_ffi`), so the call is made from inline assembly, and a test has the guest copy the set register and the host read the copy. Disposition: resolved-here. *Observation 2 (an edit outside the Edit tool):* WORKING_AGREEMENT's three changed lines were written by an inline script; its diff was read and holds only them. Disposition: recorded. *Observation 3 (two gates runs failed):* the first at `cargo fmt --check`, the second at clippy's `large_enum_variant` (`Switch::Go` carried 31 registers); after `cargo fmt` and a `Switch` that leaves the registers in `schedule`'s `x`, the third ran from the start. Disposition: resolved-here.
- **Closed:** Sprint 156's CI run 40188695646 (success on daa0bec17).
- **Files:** crates/hypervisor/, crates/w2m_vm/, store/run_loop/sleeper-end-python.sha256, WORKING_AGREEMENT.md, BLACKBOARD.md, sprints/sprint-157-a-thread-ends.md, signal-reports/sprint-157-report.md.

### Sprint 156 (2026-10-25, closed)
- **Scope:** link 5's run-loop judge, its second program: CreateThread served by the Rust host, and `sleeper` past its thread request with two threads.
- **Dual contract:** signal — none, as the card says (the skeleton's record comes at link 6); artifact pass (fmt, clippy, the gates); observation pass (the created thread equal to the Python host's, field for field; the stack's mapping as `_map_pages` leaves it; `woke` printed and a named stop; two planted faults each failed it at the field named).
- **CI does not run:** the run loop's two tests (macOS only, the Python fixtures).
- **Rubber Duck Pass:** *Observation 1 (a plant the fixture could not see):* the card's first plant, the guard pages left mapped, changes no recorded field; the judge gained a check of the stack pages' stage-1 mapping before the plant counted. Disposition: resolved-here. *Observation 2 (a check that ran nothing):* a build checked through `grep` showed no errors because `cargo` was not on the shell's PATH and the "command not found" line was filtered out; every command since reads its exit status. Disposition: resolved-here. *Observation 3 (a stop that named only its op):* `HostRequest { op: 30 }` named two different requests, CreateThread and THREAD_EXIT_CODE; the stop now carries the host's refusal text. Disposition: resolved-here.
- **Closed:** Sprint 155's CI run 40186644238 (success on f8db65215).
- **Files:** crates/w2m_vm/, crates/virtual_memory/src/lib.rs, store/run_loop/sleeper-python.sha256, WORKING_AGREEMENT.md, BLACKBOARD.md, sprints/sprint-156-a-second-thread.md, signal-reports/sprint-156-report.md.

### Sprint 155 (2026-10-25, closed)
- **Scope:** link 5's run-loop judge: the blocks the VM's C declines compiled in Rust inside the skeleton's run loop, so `qpc` runs past its first decline.
- **Dual contract:** signal — none, as the card says (the skeleton's record comes at link 6); artifact pass (fmt, clippy, the gates); observation pass (`qpc` to ExitProcess(0) with its counter in range; the blocks and the host's words equal the Python host's; a planted inverse condition failed it).
- **CI does not run:** the run loop's test (macOS only, the Python fixture); `c_words` runs on CI from Sprint 154.
- **Rubber Duck Pass:** *Observation 1 (two plants that did not plant):* `LIVENESS_SCAN` 0 and NZCV fusion off left these two blocks' words unchanged; a plant must change what the judged blocks use, measured, before it counts. Disposition: resolved-here (a wrong inverse condition). *Observation 2 (a card's stop overtaken):* the card expected a named stop short of the end; the first stop was `qpc`'s output, and two host requests more ran it to its end. Disposition: resolved-here, recorded as an amendment. *Observation 3 (the skeleton's guest finished):* `qpc` is the first guest the Rust host runs from entry to exit; its output varies with the clock, so its judge is the Python test's range, not a stored byte string. Disposition: recorded.
- **Closed:** Sprint 154's CI run 40185155755 (success on 89760257a); one single-test run in its log took 32.39 s, `c_words` on Linux, where it had skipped.
- **Files:** crates/w2m_vm/, crates/translator/, store/run_loop/qpc-python.sha256, WORKING_AGREEMENT.md, BLACKBOARD.md, sprints/sprint-155-the-run-loop-compiles.md, signal-reports/sprint-155-report.md.

### Sprint 154 (2026-10-25, closed)
- **Scope:** reviewer-2's review of Sprints 151-152: Medium 1 (a committed record deleted), Medium 2 (the C words judge skipping on CI), the ledger's three conditions, the lows, group 2's counts.
- **Dual contract:** signal — none: no producer's events change, the records stand as Sprint 153 made them; artifact pass (fmt, clippy, the gates with the ledger check); observation pass (the restored record replays; the two plants named; DIV 47 and IDIV 48 judged at a fault through Rust words). The CI run of the closing commit is the measure of `c_words` on Linux.
- **CI does not run:** the case runner's three tests, `case_runs` and its ledger check (macOS only). CI now runs `c_words`.
- **Rubber Duck Pass:** *Observation 1 (a deletion that hid an error's record):* Sprint 152 removed the record that showed its own error; restored, it stands outside the readers with a note. Disposition: resolved-here. *Observation 2 (link 3's Blocking 1, again):* a new judge skipped where its tool was absent, as the case runner once did; the lesson KIT_DIARY #65 holds was not applied to the next judge written. Disposition: resolved-here (fails on a missing compiler).
- **Closed:** Sprint 153's CI run 40183768136 (success on 0e667473e).
- **Files:** store/superseded/, crates/translator/tests/c_words.rs, crates/w2m_vm/, scripts/gates.sh, signals/proposals.json, docs/rust-port-plan-2026-10-24.md, WORKING_AGREEMENT.md, BLACKBOARD.md, sprints/sprint-151-the-lifter-and-the-integer-core.md, sprints/sprint-154-a-record-kept-and-a-judge-on-ci.md, signal-reports/sprint-154-report.md.

### Sprint 153 (2026-10-25, closed)
- **Scope:** link 5's second group in Rust: the rest of `_int.py`'s A64-lowered rules and `bmi.py`, with their lowering.
- **Dual contract:** signal pass (four `case-runner-rust` sessions, 512 classes accepted over them, the Rust translator the candidate for 161; no discrepancy; the ledger empty); artifact pass (fmt, clippy, the gates); observation pass (1,096,154 record calls equal to the C, 65,624 cases agree with Windows, 9,264 case codes equal to the Python's; the plant named).
- **CI does not run:** the case runner's three tests and `case_runs`; the C words judge returns early without clang.
- **Rubber Duck Pass:** *Observation 1 (perl for multi-line edits again):* three edits to `lower.rs` went through `perl -0pi` over several lines, the error class the standing rule names; the file's head was checked intact and the rest used the Edit tool. Disposition: recorded. *Observation 2 (a class count off by five):* the card said 75 classes; 70 is the count (38 named, 16 SETcc, 16 CMOVcc). Disposition: resolved-here. *Observation 3 (temporaries in the Python's order):* each rule draws its temporaries in the Python's order, since the register a temporary gets follows the order of writes; the equal words on a million calls bear it out. Disposition: recorded.
- **Closed:** Sprint 152's CI run 40181818440 (success on 9a76b1d51).
- **Files:** crates/translator/, store/runs/case-runner-rust-*, store/case_runner/discrepancy-ledger.json, WORKING_AGREEMENT.md, BLACKBOARD.md, sprints/sprint-153-the-integer-rest.md, signal-reports/sprint-153-report.md.

### Sprint 152 (2026-10-25, closed)
- **Scope:** the review of Sprints 149-150: High 1 (TRACE_INGESTED over verdict lines), High 2 (the judges' discrepancies outside any ledger), Medium 1 (three CPUs merged into one profile), Medium 2 (a race hit dismissed), the lows, and the review's third judge.
- **Dual contract:** signal pass (eight sessions, SESSION_INIT, CANDIDATE_GENERATED, COMPARISON_COMPLETED, CANDIDATE_ACCEPTED, SESSION_COMPLETED; in the planted chain DISCREPANCY_REPORTED, ROUTED and RESOLVED, 107 each); artifact pass (fmt, clippy, the gates; replay level 2 of each record); observation pass (the chain closed; a wrong `src` refused; the Rust words equal the Python's on 4,103 codes).
- **CI does not run:** the case runner's three tests and `case_runs` (macOS only, the downloaded results, the fixture); CI runs the ledger's unit tests.
- **Corrections:** Sprint 150 said its record met rule T4; it met it with TRACE_INGESTED over lines nothing validated. Sprint 151's tail did not say its record was withheld until this sprint (it does). Sprint 150's tail did not state its full gates exit: the second run exited 0. The review counted 248 rule modules; the registry map holds 249.
- **Rubber Duck Pass:** *Observation 1 (a profile is two facts):* the review named three CPUs; measured, the 8573C runs span two Windows images, so four profiles. Disposition: resolved-here. *Observation 2 (a ledger a Producer cannot finish):* a Producer cannot read its own unfinalised record, so the ledger file is written after a run by `case_runs --write-ledger`. Disposition: resolved-here. *Observation 3 (a stray command):* one edit command began with `python3 -` and waited on standard input until stopped; it changed nothing, checked with `git diff`. Disposition: resolved-here (the Edit tool made the change).
- **Closed:** Sprint 151's CI run 40179455651 (success on 28c7fca35).
- **Files:** crates/w2m_vm/, harness/, store/case_runner/, store/runs/case-runner-*, WORKING_AGREEMENT.md, BLACKBOARD.md, sprints/sprint-152-the-judges-records-made-true.md, signal-reports/sprint-152-report.md.

### Sprint 151 (2026-10-25, closed)
- **Scope:** link 5's first group in Rust, the lifter folded in as the link-3 review advised: lifted and lowered as the frozen Python does, judged word for word against the C emitter and against Windows through the case runner.
- **Dual contract:** signal deferred to Sprint 152 (the `case-runner-rust` record was made, 387 classes accepted with 38 Rust candidates, and withheld: reviewer-2 found its form states a validation that never happened); artifact pass (fmt, clippy, the gates); observation pass (1,060,791 record calls equal to the C; 36,939 cases agree with Windows and equal the Python's words; two plants named).
- **CI does not run:** the case runner's two tests and `case_runs` (macOS only, the downloaded results, the fixture). CI runs the C words judge only where clang is found; on ubuntu it returns early, said on stderr.
- **Rubber Duck Pass:** *Observation 1 (a million equal on the first run):* the C judge matched every compared call at once; both plants then showed the comparison names a difference to the word. Disposition: resolved-here. *Observation 2 (the assembler's home):* the translator needs `Assembler`, which lived in `w2m_vm`, which now depends on the translator; it moved to `a64_encoder` by `git mv`, re-exported where it was. Disposition: resolved-here. *Observation 3 (a record withheld):* the review of Sprints 149-150 arrived while this sprint's gates ran and found the record form false in one event and outside the ledger; this sprint's record has that form and is not committed. Disposition: deferred to Sprint 152. *Observation 4 (two VMs in one process):* the first gates run failed the second case-runner test at `hv_vm_create` (HV_BUSY): cargo runs a test binary's tests on parallel threads and a process holds one VM. A static mutex makes them take turns. Disposition: resolved-here.
- **Closed:** Sprint 150's CI run 40177021300 (success on c1c8ee44f).
- **Files:** crates/translator/, crates/a64_encoder/, crates/w2m_vm/, harness/case-runs-rust.toml, Cargo.toml, Cargo.lock, WORKING_AGREEMENT.md, BLACKBOARD.md, sprints/sprint-151-the-lifter-and-the-integer-core.md, signal-reports/sprint-151-report.md.

### Sprint 150 (2026-10-25, closed)
- **Scope:** the case runner's verdicts on a substrate record in the vocabulary's events, a unit per instruction class, as `scripts/vm_oracle.py --record` did through `differential.py`.
- **Dual contract:** signal pass (the record holds SESSION_INIT 1, EXECUTION_STARTED/COMPLETED and TRACE_INGESTED 22 each, CANDIDATE_GENERATED, COMPARISON_COMPLETED and CANDIDATE_ACCEPTED 387 each, SESSION_COMPLETED completed); artifact pass (fmt, clippy, the gates; replay level 2; check_records holds none); observation pass (the planted flip gave 107 discrepancies, the first naming ADD CF).
- **CI does not run:** the case runner's test and `case_runs` (macOS only, the downloaded results, the fixture); CI builds `case_runs`.
- **Rubber Duck Pass:** *Observation 1 (T4 on the first record):* the first record named the empty trace hash for all 22 runs and ingested none; the record check held all 22. Each run now has a trace of its verdicts, said in `unobserved` to be the runner's and not guest output. Disposition: resolved-here. *Observation 2 (one Producer for a causal chain):* `differential.py` chained its steps by predicate triggers; a run file cannot, so each event's cause is the Producer's start. The vocabulary's rules check order and payload, which hold. Disposition: recorded. *Observation 3 (the signer):* substrate starts the binary unsigned and `hv_vm_create` refused it (HV_DENIED); the run file calls it through `scripts/sign_and_run.sh`. Disposition: resolved-here. *Observation 4 (the store's conventions):* the first gates run failed four pytest tests over two conventions the sprint did not read first: `store/traces` holds guest traces only, and a baseline naming `differential` marks a causal differential record. The traces moved to `store/case_runner/traces`, the baseline names `judge`, and the record was made again. Disposition: resolved-here. A fifth, the timed-waits race hunt, failed under load as its own comment records (0 in 10 under the gates) and passed alone.
- **Closed:** Sprint 149's CI run 40174320596 (success on 83402c4b6); the ungated arena replay ran on Linux in it.
- **Files:** crates/w2m_vm/, harness/case-runs.toml, store/runs/case-runner-*, store/case_runner/traces/, Cargo.lock, WORKING_AGREEMENT.md, BLACKBOARD.md, sprints/sprint-150-the-case-runner-on-a-record.md, signal-reports/sprint-150-report.md.

### Sprint 149 (2026-10-25, closed)
- **Scope:** the large review of link 3 (reviewer-2, 2026-10-25): two blocking findings (a vacuous pass; the revisit unmeasured) and Mediums 1-4, fixed before link 4.
- **Dual contract:** signal — none fired; Sprint 150 binds the runner's verdicts to records (Medium 5); artifact pass (fmt, clippy, the gates with `W2M_REQUIRE_CASE_RUNS=1`); observation pass (counts equal per run, zero differences, three plants named, the revisit measured at 235 of 249).
- **CI does not run:** the case runner (`crates/w2m_vm/tests/case_runner.rs`: macOS only, downloaded results, the fixture). The arena's replay now does.
- **Corrections:** Sprint 148's observation 3 said the runner judges everything `scripts/vm_oracle.py` judges; it does not judge the 116,718 cases the model runs. Sprints 147 and 148 gave 173,776 judged; set 039 runs twice, and 136,153 distinct cases are judged of 250,840. Sprint 145's observation 1 was marked resolved-here while the vacuous path stayed in the code.
- **Rubber Duck Pass:** *Observation 1 (two plants that ran nothing):* the first runs of two plants passed a command through a zsh variable, which does not word-split, and exited 127 without running; the empty grep was nearly read as a pass. Disposition: resolved-here (rerun with the command written out; both named). *Observation 2 (the fixture's generator is inline Python):* regenerating the fixture runs the frozen Python inline, which the user's ruling allows as running; it is named in WORKING_AGREEMENT. Disposition: recorded.
- **Closed:** Sprint 148's CI run 40172473690 (success on 90d5b84b6).
- **Files:** crates/w2m_vm/, scripts/gates.sh, store/case_runner/, store/rules/registry.tsv, WORKING_AGREEMENT.md, KIT_DIARY.md, BLACKBOARD.md, sprints/sprint-149-the-case-runner-bound.md, signal-reports/sprint-149-report.md.

### Sprint 148 (2026-10-25, closed)
- **Scope:** the case runner's last judged fields: MXCSR, XMM, the YMM upper halves and the x87 state, at completion and at a fault.
- **Dual contract:** signal — none fired, as the card says; artifact pass (fmt, clippy, the gates); observation pass (zero differences; the fields' reach counted; the plant named).
- **Rubber Duck Pass:** *Observation 1 (a zero needs its denominator):* the first run showed zero differences with no count of the cases that carried YMM or x87 state; the tally now counts them (43,919 and 5,960), so a zero over an empty set would show. Disposition: resolved-here. *Observation 2 (FCOS and FSIN untested here):* the two classes the Python's class runs did not accept never reach the runner's judge; they leave to the model. Disposition: deferred to the model's port. *Observation 3 (link 3 complete):* with this sprint the case runner judges everything `scripts/vm_oracle.py` judges; the next link is the lifter. Disposition: recorded.
- **Closed:** Sprint 147's CI run 40171349437 (success on 926723fde).
- **Files:** crates/w2m_vm/, WORKING_AGREEMENT.md, BLACKBOARD.md, sprints/sprint-148-the-vector-and-x87-state.md, signal-reports/sprint-148-report.md.

### Sprint 147 (2026-10-25, closed)
- **Scope:** cases Windows ended in a fault run to the same fault in the Rust VM, judged on code, RIP, registers, flags, MXCSR, XMM and memory through the guest's exception records.
- **Dual contract:** signal — none fired, as the card says; artifact pass (fmt, clippy, two new modules, one new test, the gates); observation pass (25,162 faults and 148,614 completed cases, zero differences; the stub page equal to the Python's; the plant named).
- **Rubber Duck Pass:** *Observation 1 (the card's request list was short):* the card named three requests from reading `exceptions.c`; the run found `hvc #14` and `THREAD_STACK_GROW` before them, both in `runtime.c`'s path, which the card had read but not traced call by call. Disposition: resolved-here (both served, the card amended with the measurement). *Observation 2 (a branch served for one thread only):* `THREAD_STACK_GROW` answers 0 for the main thread as the Python does; a created thread's grow stops by name, since the Rust host has no threads. Disposition: recorded. *Observation 3 (faults in the model):* 8,006 Windows faults run in the model exits and are counted, not judged. Disposition: deferred to the model's port (the rules and lowering).
- **Closed:** Sprint 146's CI run 40169862922 (success on a8477bd01).
- **Files:** crates/w2m_vm/, Cargo.lock, WORKING_AGREEMENT.md, BLACKBOARD.md, sprints/sprint-147-the-fault-path.md, signal-reports/sprint-147-report.md.

### Sprint 146 (2026-10-24, closed)
- **Scope:** VirtualAlloc, VirtualFree, VirtualProtect and VirtualQuery answered in Rust as Windows answered them in the probes' rows; the arena the fault path's stubs page will live in.
- **Dual contract:** signal — none fired, as the card says; artifact pass (fmt, clippy, the gates); observation pass (38 virtual rows on two images, 6 stack rows; the plant named).
- **Rubber Duck Pass:** *Observation 1 (a replay is not a run):* the rows come from the probe's calls replayed against the crate, not from the probe running in the VM; the two rows that depend on the probe's own stores cannot be replayed honestly and are left to the VM run, named in the card. Disposition: recorded. *Observation 2 (an inline edit script):* one card edit ran through an inline Python command; running Python is allowed, writing it is not, and an edit script is too close. Disposition: resolved-here (edits go through the Edit tool). *Observation 3 (all passing on the first run):* the four tests passed at once; the planted fault showed the replay compares each field. Disposition: resolved-here.
- **Closed:** Sprint 145's CI run 40168395449 (success on 50963ff04).
- **Files:** crates/virtual_memory/, crates/w2m_vm/, Cargo.toml, Cargo.lock, WORKING_AGREEMENT.md, BLACKBOARD.md, sprints/sprint-146-the-virtual-memory-arena.md, signal-reports/sprint-146-report.md.

### Sprint 145 (2026-10-24, closed)
- **Scope:** Windows-recorded CPU cases run one instruction at a time in the Rust host's VM, every completed case judged against Windows.
- **Dual contract:** signal — none fired, as the card says; artifact pass (fmt, clippy, one new VM test, the gates); observation pass (148,614 cases, zero differences; the plant named).
- **Rubber Duck Pass:** *Observation 1 (a test that passed on nothing):* the first run judged 0 cases and passed, every compile refused; the zero difference classes meant nothing. Disposition: resolved-here (the refusal causes printed, the cause found, a compiler that covers the cases used; the result states the judged counts). *Observation 2 (a design assumption measured false):* the design's case runner rested on the C emitter compiling cases whose classes it stencils; at the cases' address it compiles none. Disposition: resolved-here (the design amended in place, dated, with the measurement). *Observation 3 (the Python's words as a fixture):* the runner was validated with words the frozen Python produced, a use the user's ruling allows (running Python to measure, not writing it); option E of the design was rejected for the skeleton because block layout varies there, and here the layout is fixed and checked. Disposition: recorded. *Observation 4 (an address read twice):* the fixture's first version added 8 to an address that already included it; the test's layout check refused it. Disposition: resolved-here. *Observation 5 (a gate hour lost to an unoptimized loop):* the first gates run sat 72 minutes in the case runner's test, which resets the 1,048,576 dispatch entries per case with one call per word; a release run had taken 33 s. Disposition: resolved-here (the empty table copied whole; 79 s unoptimized).
- **Closed:** Sprint 144's CI run 40162112879 (success on 05dceba04).
- **Files:** crates/w2m_vm/, crates/pe_loader/, crates/hypervisor/, docs/design-host-compile-path-2026-10-24.md, Cargo.lock, WORKING_AGREEMENT.md, BLACKBOARD.md, sprints/sprint-145-the-case-runner.md, signal-reports/sprint-145-report.md.

### Sprint 144 (2026-10-24, closed)
- **Scope:** every CPU case w2m ran on Windows, and every result Windows gave, read from Rust exactly as written; the operands of a decoded instruction as the VM's C gives them.
- **Dual contract:** signal — none fired, as the card says; artifact pass (fmt, clippy, 4 new tests and the extended differential, the gates); observation pass (the round trips, the hashes, the refusal counts; two plants named).
- **Rubber Duck Pass:** *Observation 1 (a dependency found on reading):* `undefined_memory` needs the operand layer, which no Rust crate had; the card was amended before code to carry `x86ops.py` with its C twin as judge. Disposition: resolved-here. *Observation 2 (a name that shadowed the language):* the Python's `Result` became `CaseResult`. Disposition: resolved-here. *Observation 3 (most downloads not judged):* of 1,713 entries under `build/probe-runs`, 145 hold a `result-1.bin` and 22 of those have both hashes recorded in `store/`; the other 123 cannot be checked against a recorded hash, so they are not read. Disposition: recorded.
- **Closed:** Sprint 143's CI run 40161085949 (success on 5a5048681).
- **Files:** crates/cpu_cases/, crates/x86_decoder/, Cargo.toml, Cargo.lock, WORKING_AGREEMENT.md, BLACKBOARD.md, sprints/sprint-144-the-case-formats-in-rust.md, signal-reports/sprint-144-report.md.

### Sprint 143 (2026-10-24, closed)
- **Scope:** x86 code decoded from Rust as the VM's C decoder decodes it, over every instruction the batch run compiled.
- **Dual contract:** signal — none fired, as the card says; artifact pass (fmt, clippy, 9 tests, the gates); observation pass (the record and the sweep equal; the coverage figures; the plant named at a `push`).
- **Rubber Duck Pass:** *Observation 1 (a generator that cannot go stale):* the card named a generator binary with a `--check`; tables generated by `build.rs` at every build need no check, so none was added. Disposition: resolved-here (stated as a deviation). *Observation 2 (a record older than its table):* five x87 symbols of the stencil table are absent from the record's header; the coverage count stands them at 0, which cannot change whether the C emits. Disposition: recorded. *Observation 3 (a status misread):* the first coverage count took a return of 0 as a refusal; `vmc_status` is negative, and 0 is an instruction with no words (12,020 calls). Disposition: resolved-here. *Observation 4 (a linker without its SDK):* clang called by its path found no libSystem; `xcrun clang` does. Disposition: resolved-here. *Observation 5 (multi-line perl again):* two multi-line edits to `lib.rs` were made with perl, not the Edit tool; they compiled and the file's header was checked, and later multi-line edits used Edit. Disposition: resolved-here.
- **Closed:** Sprint 142's CI run 40159508340 (success on c392a38cd).
- **Files:** crates/x86_decoder/, Cargo.toml, Cargo.lock, WORKING_AGREEMENT.md, BLACKBOARD.md, sprints/sprint-143-the-decoder-in-rust.md, signal-reports/sprint-143-report.md.

### Sprint 142 (2026-10-24, closed)
- **Scope:** the Rust host runs a Windows program's code in the VM until the first block that needs the host's compiler, and names it.
- **Dual contract:** signal — none fired, as the card says; artifact pass (fmt, clippy, one new VM test, the gates); observation pass (the stop named inside `.text`; the C's counts consistent with it; both exits `hvc #1`; the plant named).
- **Rubber Duck Pass:** *Observation 1 (an error class met again):* a perl substitution with `|` as delimiter and `\|` in its pattern spliced code into `guest.rs`'s licence header, the class recorded in an earlier session with the rule "the Edit tool for multi-line code changes"; the rule was not followed. Disposition: resolved-here (restored from Sprint 141's commit, redone with Edit, every changed file's first line checked). *Observation 2 (a gate that cannot yet name its oracle):* the stop is shown by the C's counts and the instruction, not by the Python host's first compile, which needs the frozen host run and the user's answer on inline Python. Disposition: recorded. *Observation 3 (a measurement of one):* one served exit (3,667 ns) is reported as one sample; the design's measurements wait for runs with many exits. Disposition: recorded. *Observation 4 (a count written, not counted):* the card's result first gave the COM vtables 45 methods; counted, 54. Disposition: resolved-here.
- **Closed:** Sprint 141's CI run 40158565613 (success on 2942d9c27).
- **Files:** crates/w2m_vm/, WORKING_AGREEMENT.md, BLACKBOARD.md, sprints/sprint-142-the-run-loop-to-the-first-decline.md, signal-reports/sprint-142-report.md.

### Sprint 141 (2026-10-24, closed)
- **Scope:** an import's call, in the VM, reaches its guest runtime service and returns through the dispatcher by the Microsoft x64 convention.
- **Dual contract:** signal — none fired, as the card says; artifact pass (fmt, clippy, one new VM test, the gates); observation pass (the three calls; one value from Windows' rows; the plant named RET's pop).
- **Rubber Duck Pass:** *Observation 1 (a split that kept the sprints small):* the bridges were split from the run loop when reading `_bridge` showed four pieces behind it; this sprint closed in one pass. Disposition: resolved-here (technique #17). *Observation 2 (pieces deferred, named):* call tracing in the bridge, the COM vtables and the VM compiler's symbol table are not carried; each is named in the card's result for Sprint 142. Disposition: recorded. *Observation 3 (a draft that wrote twice):* the first draft of `Guest::new` called `process::write` twice, a leftover; removed before it compiled. Disposition: resolved-here.
- **Closed:** Sprint 140's CI run 40157651012 (success on aead79d7b).
- **Files:** crates/w2m_vm/, WORKING_AGREEMENT.md, BLACKBOARD.md, sprints/sprint-141-the-bridges.md, signal-reports/sprint-141-report.md.

### Sprint 140 (2026-10-24, closed)
- **Scope:** the process area a program and the runtime read at start, written from Rust and held to Windows' rows.
- **Dual contract:** signal — none fired, as the card says; artifact pass (fmt, clippy, 5 new tests, the gates); observation pass (the parameters, loader-list and stack rows; one row named as unmet; the plant named `ldr.init_order`).
- **Rubber Duck Pass:** *Observation 1 (a Windows row the oracle never met):* `teb.reserve_is_header` fails for every guest whose header asks other than 1 MB, in the Python host as here; the probes the row was observed on were MSVC-built and asked 1 MB, so no run saw it. Disposition: recorded (open: the stack reserve follows the header when the stored records are made again; the test pins today's value so the change is seen). *Observation 2 (a plant that planted nothing):* the first planted test reversed `qpc`'s DLL dependency order, which is empty; it was removed before it ran, and the plant was made in the source. Disposition: resolved-here. *Observation 3 (the first link split before code):* reading `VM.__init__` and the process area showed the run loop's first link was two sprints; the split was made on the card before code. Disposition: resolved-here (technique #17).
- **Closed:** Sprint 139's CI run 40156124131 (success on 050d9aecf).
- **Files:** crates/w2m_vm/, Cargo.lock, WORKING_AGREEMENT.md, BLACKBOARD.md, sprints/sprint-140-the-process-area.md, signal-reports/sprint-140-report.md.

### Sprint 139 (2026-10-24, closed)
- **Scope:** the miss path the skeleton needs, decided against the goal and the systems that solved it, and the plan amended.
- **Dual contract:** signal — none fired, as the card says; artifact pass (`design_check`, the gates); observation pass (the design and the amended plan pass `design_check`; the Decision names the design; FEX's sources removed and named).
- **Rubber Duck Pass:** *Observation 1 (a decision that ran ahead of its options):* the first draft weighed three answers and chose the translator before any run loop, which would have kept every guest from running for five or more sprints; the reviewer named three thin answers it had not weighed, and the measurement of the other 11 guests ruled one out and left the run loop first standing. Disposition: resolved-here (F chosen). *Observation 2 (a judge that could not see its subject):* the C emitter cannot judge the blocks it declines, which are the blocks that forced the decision; the class cases can, through a case runner. Disposition: resolved-here (the case runner a link of its own). *Observation 3 (a figure quoted, not counted):* the reviewer's guarded-stencil figure came from an older table's tail; counted, it was 3,564 of 21,239 choices. Disposition: resolved-here. *Observation 4 (dates from the wrong clock):* entries from Sprint 132 to 138 carry 2026-10-25, the UTC date; the local date was 2026-10-24. Disposition: recorded (committed entries stand; the user asked which clock governs). *Observation 5 (placeholders in an append-only section):* each close filled the previous Built entry's `CIRUN` placeholder in place, an edit to an append-only section. Disposition: resolved-here (no placeholder from this entry on; the CI run goes in the next tail entry). *Observation 6 (Python written, though no file):* the measurement wrapped `VM._compile` in an inline `python -c`. Disposition: recorded (the question goes to the user; meanwhile existing entry points only).
- **Files:** docs/design-host-compile-path-2026-10-24.md, docs/rust-port-plan-2026-10-24.md, BLACKBOARD.md, sprints/sprint-139-the-host-compile-path.md, signal-reports/sprint-139-report.md.

### Sprint 138 (2026-10-25, closed)
- **Scope:** the program and its DLLs placed, relocated, bound and started from Rust as `loader.py` placed them.
- **Dual contract:** signal — none fired, as the card says; artifact pass (fmt, clippy, 6 tests, the gates); observation pass (the Python test's three checks; three Windows contract rows; the stub equal to clang's; the plant named RVA 0xe258).
- **Rubber Duck Pass:** *Observation 1 (the next link cannot run on what exists):* measured after this sprint's code, the Python host compiled 5 of `sleeper`'s blocks, 2 of `qpc`'s and 10 of `twoready`'s that the C in-VM compiler declined ("not a code page" 9, 4, 9; "no stencil" 4, 2, 10). The skeleton serves `hvc #1`, so it needs a host compile path, which the plan's step 4 ports after the skeleton. Disposition: recorded (Sprint 139 designs the miss path against the goal and the systems that solved it, before any code). *Observation 2 (a judge from Windows, not the Python):* the TLS and placement checks are Windows contract rows, so the Rust loader is held to what Windows did, not only to what the Python did. Disposition: recorded.
- **Files:** crates/pe_loader/, Cargo.toml, Cargo.lock, WORKING_AGREEMENT.md, BLACKBOARD.md, sprints/sprint-138-the-loader-in-rust.md, signal-reports/sprint-138-report.md.

### Sprint 137 (2026-10-25, closed)
- **Scope:** a PE32+ image read from Rust as `pe.py` read it, judged by llvm-objdump.
- **Dual contract:** signal — none fired, as the card says; artifact pass (fmt, clippy, 8 tests, the gates); observation pass (268 executables agree with llvm-objdump; the Python test's other seven checks; the plant named "KERNEL32.dll's imports").
- **Rubber Duck Pass:** *Observation 1 (a promise broken the day it was made):* Sprint 135's tail entry said Sprint 137's card would come before its code; the crate was drafted while Sprint 136's gates ran, and the card came after it and before its tests. Disposition: resolved-here (Sprint 138's card was written before its crate; the order is card, code, tests). *Observation 2 (a plant the judge never saw):* the first plant read the hint's bytes, which the reader's ASCII check refused before llvm-objdump was asked. A plant must reach the judge it tests. Disposition: resolved-here (the card amended to `+ 3`, and the first plant's refusal stated in the result). *Observation 3 (a count taken from a glob that was not run):* the card said 280 executables from a directory listing of all files; 268 are `.exe`. Disposition: resolved-here.
- **Files:** crates/pe_image/, Cargo.toml, Cargo.lock, WORKING_AGREEMENT.md, BLACKBOARD.md, sprints/sprint-137-the-pe-reader-in-rust.md, signal-reports/sprint-137-report.md.

### Sprint 136 (2026-10-25, closed)
- **Scope:** the monitor code the Python host wrote at a VM's start, written from Rust, dispatching, missing and taking faults as the Python's did.
- **Dual contract:** signal — none fired, as the card says; artifact pass (fmt, clippy, 5 unit tests and 4 VM test files, the gates); observation pass (the four behaviors; the planted shift named 0x7ff3456789a0, "left Ok(1), right Ok(32)").
- **Rubber Duck Pass:** *Observation 1 (a judge that could not see a term):* the planted test's first target had only bit 40 set above bit 39, so the shift changed from 40 to 41 moved bit 0, which the slot mask drops, and the two hashes agreed; the test's own precondition (`assert_ne!` on the two slots) refused it before the VM ran. Disposition: resolved-here (the target 0x7ff3456789a0 sets bits 42 to 46, and the main judge's first target is the same, so a hit exercises every term). *Observation 2 (a judge named by the card and not used):* the card named `llvm-objdump` for the Assembler's branches; its test compares them with `a64_encoder`'s words, which Sprint 135 checked against clang. Disposition: resolved-here (stated in the result). *Observation 3 (the return stub is a test aid):* `RETURN_STUB` at 0x2000 has no Python counterpart; it stands where the host's run loop will catch a return. Disposition: recorded (the skeleton's run loop replaces it).
- **Files:** crates/w2m_vm/, Cargo.lock, WORKING_AGREEMENT.md, BLACKBOARD.md, sprints/sprint-136-the-monitor-code-from-rust.md, signal-reports/sprint-136-report.md.

### Sprint 135 (2026-10-25, closed)
- **Scope:** every AArch64 word w2m emits, encoded in Rust as `src/w2m/a64_encoder.py` encodes it, checked against clang.
- **Dual contract:** signal — none fired, as the card says; artifact pass (fmt, clippy, 2 tests, the gates); observation pass (258 words equal clang's; the planted MOVK base named as "ours f2cfffc1, clang f2cfffc0").
- **Rubber Duck Pass:** *Observation 1 (a card written after its code):* the crate was ported and judged before `sprints/sprint-135-the-a64-encoder-in-rust.md` existed; the card says so in its result. The audit trail shows the work, but the card did not fix the judge ahead of it. Disposition: resolved-here (Sprint 136's card was written before its code, and so is Sprint 137's). *Observation 2 (defaults written in by pattern):* the Python test's calls leaned on default arguments Rust lacks; they were written in by pattern and the compiler named each call that lacked one, so no case passed with a value the Python did not give. Disposition: recorded.
- **Files:** crates/a64_encoder/, Cargo.toml, Cargo.lock, WORKING_AGREEMENT.md, BLACKBOARD.md, sprints/sprint-135-the-a64-encoder-in-rust.md, signal-reports/sprint-135-report.md.

### Sprint 134 (2026-10-25, closed)
- **Scope:** a function of the C guest runtime run in a Rust VM at its link address, answering as the Python host's VM answered.
- **Dual contract:** signal — none fired, as the card says; artifact pass (fmt, clippy, 6 tests, the gates); observation pass (26 inputs equal to the oracle; the abort vector with ESR 0x21 and ELR at the function; a planted wrong value named).
- **Rubber Duck Pass:** *Observation 1 (a stub where a dispatcher belongs):* the return stub sits at `DISPATCHER` (0x1800) and the vectors are bare `hvc`s, where the Python host writes its dispatcher and an exception entry into the runtime. Disposition: recorded (Sprint 136 writes the Python host's monitor code from Rust and replaces both). *Observation 2 (no code-cache coherence step yet):* the image and stubs are written by the host before the vCPU's first run, so no instruction cache holds stale code; code written while a vCPU runs will need the invalidation sequence the host design names. Disposition: recorded (design change 3; the sprint that writes code under a running vCPU carries it).
- **Files:** crates/w2m_vm/, Cargo.toml, Cargo.lock, WORKING_AGREEMENT.md, sprints/sprint-134-the-runtime-runs-in-a-rust-vm.md, signal-reports/sprint-134-report.md.

### Sprint 133 (2026-10-25, closed)
- **Scope:** the C guest runtime's image built from Rust, equal to the frozen Python host's, byte for byte.
- **Dual contract:** signal — none fired, as the card says; artifact pass (fmt, clippy, 9 tests, the gates); observation pass (the key names a Python image; the images equal; every service present; three plants caught).
- **Rubber Duck Pass:** *Observation 1 (a judge that leans on a local cache):* the byte-for-byte reference is the Python build's file under `build/guest_runtime/`, which is untracked and absent on CI's Linux runner (the test is macOS-only), and which step 4 retires with the Python host. Disposition: recorded (when the Python host moves under `oracle/`, the image's SHA-256 for the sources of that commit is written into the test as the reference, and a change to the C runtime updates it in the sprint that makes the change). *Observation 2 (a plant changed from the card):* the card named a header byte; a source byte tests the same key path. Disposition: resolved-here (named in the result).
- **Files:** crates/guest_runtime/, Cargo.toml, Cargo.lock, WORKING_AGREEMENT.md, sprints/sprint-133-the-guest-runtime-built-from-rust.md, signal-reports/sprint-133-report.md.

### Sprint 132 (2026-10-25, closed)
- **Scope:** a VM and one vCPU created, run and served from Rust, with no C monitor between.
- **Dual contract:** signal — none fired, as the card says; artifact pass (fmt, clippy, 3 tests and a compile-fail doc test, the gates); observation pass (the exits' class, immediate, x0 and PC as expected; CANCELED from another thread; the exit's time beside the C monitor's the same day; two plants refused).
- **Rubber Duck Pass:** *Observation 1 (a figure from another day):* the card compared Rust's exit with the C monitor's 0.72 µs of 2026-10-14; Rust's measured 832 to 873 ns read as 15% slower until `vm_monitor`'s `spin` ran the same loop the same day at 813 to 852 ns. Disposition: resolved-here (a speed is compared only with one measured beside it). *Observation 2 (a plant that tested the wrong thing):* stripping the signature made macOS kill the binary (SIGKILL) before it ran; the entitlement's refusal needs a binary signed without it. Disposition: resolved-here (signed ad hoc without the entitlement: HV_DENIED). *Observation 3 (headers by name, not content):* the bridge mapping named `hv_error.h` and `hv_types.h`, which hold x86 declarations only; arm64's codes are in `arm64/hv/hv_kern_types.h`. Disposition: resolved-here (WORKING_AGREEMENT corrected).
- **Files:** crates/hypervisor/, scripts/sign_and_run.sh, .cargo/config.toml, Cargo.toml, Cargo.lock, WORKING_AGREEMENT.md, sprints/sprint-132-a-vcpu-in-rust.md, signal-reports/sprint-132-report.md.

### Sprint 131 (2026-10-24, closed)
- **Scope:** the 24 tags no code fires deprecated in 0.10, kept readable on records and carried no further into Rust.
- **Dual contract:** signal — the samples run fired the 33 live harness kinds twice each and none of the 19 deprecated ones; the session check fired SESSION_INIT (vocab_version 0.10) and SESSION_COMPLETED; artifact pass (fmt, clippy, the workspace's tests, the generator's check, the gates, check_replay over 38,572 records, check_records 0 held); observation pass (33 kinds x 2, replay Level 2; three plants refused by name).
- **Rubber Duck Pass:** *Observation 1 (a card that under-declared, again):* the Emits said "each harness kind 0.10 keeps, twice" without naming the 33, so the report counts them as fired and not named, Sprint 126's fault repeated. Disposition: recorded (a card whose run fires every kind of a plane names the plane's tag list in Emits; the next such card does). *Observation 2 (a version pointer the frozen oracle reads):* moving `signals/current.json` to 0.10 would have changed the Python host's inputs. Disposition: recorded (Decision 2026-10-24, two pointers until step 4). *Observation 3 (a format rewritten by a tool):* jq reformatted all of `proposals.json`. Disposition: resolved-here (undone; P21 written in the file's format). *Observation 4 (the same edit fault twice in one day):* a perl substitution with `|` as its delimiter turned an escaped `\|` into alternation and spliced code into a licence header, in `design_check` (Sprint 128) and again in the generator here; and a sed without a line address stamped one sprint's CI run on two entries. Disposition: resolved-here (each repaired before any build or commit; multi-line code edits through the Edit tool, placeholders unique per sprint).
- **Files:** signals/0.10.json, signals/0.10-rationale.md, signals/rust-current.json, signals/proposals.json, crates/vocabulary_generator/, crates/events/, crates/vocabulary_samples/, crates/session_events/src/main.rs, crates/signal_report/src/main.rs, harness/*.toml, harness/schemas/harness-events.json, WORKING_AGREEMENT.md, BLACKBOARD.md, the two records, sprints/sprint-131-vocabulary-0-10.md, signal-reports/sprint-131-report.md.

### Sprint 130 (2026-10-24, closed)
- **Scope:** the Rust host's locks, so that no thread waits on any service path for a lock another thread's service holds.
- **Dual contract:** signal — none fired, as the card says; artifact pass (the design, the old one's pointer, the Decision, `design_check`, the gates); observation pass (2 documents pass; the planted design fails on QEMU).
- **Rubber Duck Pass:** *Observation 1 (the old shape kept, a second time):* the first draft released the runtime word only around `hvc #8`, while every service bridge (`vm_backend.py` 3762-3765) and every in-VM compile miss (4240-4251) takes it for the whole call; the global lock stayed on the hottest path, about 377,000 takings a second by three imports. Disposition: resolved-here (lock classes per service, the reviewer's first pass). *Observation 2 (a race the change itself would have made):* one transport page per process, served at 3809, serialized only by the word. Disposition: resolved-here (a page per vCPU). *Observation 3 (a budget in the wrong unit):* the draft budgeted NT system calls, not the kernel32 calls answered in the VM, which are the word's load. Disposition: resolved-here (two budgets). *Observation 4 (a lock kept, stated):* the run-table word for scheduling, about 31,000 takings a second in play. Disposition: recorded (Decision 2026-10-24, its revisit condition: busy over 5% or a p99 wait over 10 µs at step 6).
- **Files:** docs/design-rust-host-2026-10-24.md, docs/design-rust-host-2026-10-23.md, BLACKBOARD.md, sprints/sprint-130-the-host-redesigned.md, signal-reports/sprint-130-report.md.

### Sprint 129 (2026-10-24, closed)
- **Scope:** the order of work to milestone B and the first scene, each step held to the goal's figures and named against the systems that solved it.
- **Dual contract:** signal — none fired, as the card says; artifact pass (the plan, two pointer lines, the Decision, `design_check` 14 tests, the gates); observation pass (1 plan checked and passing; the planted plan failing on Rosetta 2).
- **Rubber Duck Pass:** *Observation 1 (a decision read against the wrong one):* the first draft put Microsoft's DLLs and an NT layer before milestone B on Decision 2026-10-16 user (BLACKBOARD 192), which the same day's later user decision (193) supersedes with "off the critical path"; its ground, DXMT's 131 missing imports, are "mostly the C runtime". Disposition: resolved-here (step 10; the reviewer's first pass). *Observation 2 (figures with no record):* the draft gave milestone A to Sprint 033 and a rate of 2.08 to 2.13 billion a second to Sprint 095; neither holds. Disposition: resolved-here (068; Sprint 108's 2,095,145,119). *Observation 3 (a missing step):* milestone B starts on the play path, whose calls the batch run never reaches. Disposition: resolved-here (step 7, gated to the first DXGI or Direct3D 11 call, the reviewer's second pass). *Observation 4 (a check that passed a plant):* one URL in a bullet sourced two systems. Disposition: resolved-here (`sourced`, a unit test, the plant rerun).
- **Files:** docs/rust-port-plan-2026-10-24.md, docs/rust-port-plan-2026-10-21.md, docs/plan-2026-10-09.md, BLACKBOARD.md, crates/design_check/src/main.rs, sprints/sprint-129-the-plan-reaches-a-frame.md, signal-reports/sprint-129-report.md.

### Sprint 128 (2026-10-24, closed)
- **Scope:** one check before every design or decision lands: the goal's figures and the systems that solved the same problem, with sources, or the gates fail.
- **Dual contract:** signal — none fired, as the card says; artifact pass (fmt, clippy, 13 tests, release build, `design_check` exit 0, the gates); observation pass (on the tree 0 documents, exit 0; the drafts of 129 and 130 pass in a scratch root; seven planted faults each exit 1 by name).
- **Rubber Duck Pass:** *Observation 1 (a check that failed good documents):* the first version matched a system and its source on one physical line, and both drafts, whose bullets wrap, failed. Disposition: resolved-here (items, a unit test). *Observation 2 (a check that could be walked around):* the reviewer showed two ways: the exemption trusted the date the author writes in a file's name, and only `design-` and `plan-` names were read while fix (a) says every design or decision. Disposition: resolved-here (a closed history list; three more prefixes; the `Architecture:` marker on Decisions, the marker rule in WORKING_AGREEMENT; a missing BLACKBOARD fails). The marker stays opt-in: an unmarked architecture decision is caught only by review. *Observation 3 (a gate doing its job on its author):* the card was set to done before its report existed, and `signal_report --check` failed the second gates run by name. Disposition: resolved-here (the card stays active until the closing commit, as 127's did). *Observation 4 (a careless edit):* a perl substitution whose pattern held its own delimiter wrote three lines into the licence header of `main.rs`; the build failed at once. Disposition: resolved-here (repaired, rebuilt, every test rerun).
- **Files:** crates/design_check/, Cargo.toml, Cargo.lock, scripts/gates.sh, .github/workflows/ci.yml, WORKING_AGREEMENT.md, BLACKBOARD.md (Decision 2026-10-24 user), sprints/sprint-128-designs-checked-against-the-goal.md.

### Sprint 127 (2026-10-24, closed)
- **Scope:** a sprint's Signal Report from its records, written in Rust, and a gate that fails a closed sprint without one.
- **Dual contract:** signal — none fired, as the card says; its report says so. Artifact pass (fmt, clippy, 25 tests, release build, `signal_report --check`); observation pass (twelve reports written again equal to the committed ones but for one known line each; two planted faults caught).
- **Rubber Duck Pass:** *Observation 1 (a difference the judge had to explain):* every report written again had one extra line, the commit that added the report, made after the Python wrote it. Disposition: resolved-here (each of the twelve checked against `git log --diff-filter=A`; no other line differs). *Observation 2 (a card that under-declared):* Sprint 126's report shows 50 tags fired that its Emits did not name, all from the samples run. Disposition: resolved-here (named in its report and here; the card stays as written, its record stands). *Observation 3 (records with no commit in their baseline):* `substrate run --file` records carry only the TOML's baseline, so the Python rule found no record for 125 or 126. Disposition: resolved-here (a record with no baseline commit belongs to the sprint whose commit added it). *Observation 4 (a commit after a red gate):* the gates failed on clippy's `manual_is_multiple_of`, and the next command committed without reading their status (KIT_DIARY #28's pattern, Sprint 028). Disposition: resolved-here (the commit amended before any push, after the gates passed; the gate and commit now run as separate steps, the commit only on a read exit 0).
- **Files:** crates/signal_report/, Cargo.toml, Cargo.lock, scripts/gates.sh, .github/workflows/ci.yml, WORKING_AGREEMENT.md, signal-reports/sprint-123-report.md to sprint-127-report.md, sprints/sprint-127-signal-reports-in-rust.md.

### Sprint 126 (2026-10-24, closed)
- **Scope:** the vocabulary's one source generated into the Rust types and substrate's schema file, nothing hand-written repeating it.
- **Dual contract:** signal — SESSION_INIT and SESSION_COMPLETED, now from the generated types, on the session-check record; artifact pass (fmt, clippy, 19 tests, release build, `vocabulary_generator --check`, the samples run and replay); observation pass (four planted faults refused by name; 104 sample events accepted and replayed). No Signal Report: none since Sprint 122 (hand-author log 2026-10-24); Sprint 127's Rust report writes 123 to 126.
- **Rubber Duck Pass:** *Observation 1 (a port that kept the original's fault):* the E7, E8 and cadence checks matched tag names as substrings, as `check_vocab.py` did; the reviewer showed `XSESSION_COMPLETEDX` passing both. Disposition: resolved-here (whole-word matching, a planted test). *Observation 2 (two checks of one type that disagreed):* substrate's patterns, matched by Python's `re`, took a trailing newline the Rust types refuse, so Rust would refuse a payload already on a record. Disposition: resolved-here (patterns end in `\Z`; a planted sample substrate must refuse, in the gates). *Observation 3 (a required report stopped and nobody saw):* Signal Reports ended at 122, the third such stop. Disposition: deferred-to Sprint 127 (the Rust report, and a gate that fails a closed sprint without one). *Observation 4 (a table padded to fit a declared length):* the first foreign-key table had four duplicate rows added to reach a typed array length of 37; Python's has 33. Disposition: resolved-here (slices, 33 rows).
- **Files:** Cargo.toml, Cargo.lock, crates/events/, crates/vocabulary_generator/, crates/vocabulary_samples/, crates/session_events/, harness/schemas/harness-events.json, harness/session-check.toml, harness/vocabulary-samples.toml, harness/vocabulary-samples-planted.toml, scripts/gates.sh, .github/workflows/ci.yml, substrate.commit, WORKING_AGREEMENT.md, sprints/sprint-126-the-vocabulary-in-rust.md; substrate K271 (55a6b667; its gate: py3.12 and py3.13 1,354 passed, 2 failed, py3.14 1,355 passed, 1 failed, every failure one K270 named on the clean HEAD; conformance 15 of 15 on all three).

### Sprint 125 (2026-10-23, closed)
- **Scope:** a Rust program's events on a substrate record through substrate's own code, with no Python in w2m (Decision 2026-10-23, how Rust w2m reaches substrate).
- **Dual contract:** signal — SESSION_INIT (`vocab_version` 0.9, `profile` w2m-session-check, `contract_version` 0.0, `harness_version` 0.1.0, `purpose` conformance) and SESSION_COMPLETED (`outcome` completed, both counts 0, `elapsed_seconds` the binary's) fired, on the record at seq 3 and 4; artifact pass (cargo fmt, clippy, 4 tests, release build; `substrate run --file` exit 0; replay Level 2 OK; substrate's 16 new tests); observation pass (the planted run: `ProducerFailed`, "line 2: SESSION_COMPLETED: Object missing required field `accepted_count`", `--strict` exit 1).
- **Rubber Duck Pass:** *Observation 1 (a gate red before the change):* substrate's `ci_local.sh` failed under py3.12 and py3.13 on its API-page test and a real-model test; both fail on its clean HEAD too (the page under 3.12; the model 2 of 3 runs), measured in a worktree without K270. Disposition: resolved-here (named in K270's card, not this sprint's). *Observation 2 (an expectation wrong, not the code):* an optional field left out is null on the record; the test expected it absent; w2m's Structs (no `omit_defaults`) record null today. Disposition: resolved-here (the test). *Observation 3 (Python reached for out of habit):* records were first read with inline Python; read since with `substrate tail --format jsonl` and `jq`. Disposition: resolved-here.
- **Files:** Cargo.toml, rust-toolchain.toml, rustfmt.toml, crates/session_events/, harness/session-check.toml, harness/schemas/session.json, scripts/gates.sh, .github/workflows/ci.yml, substrate.commit, .gitignore, WORKING_AGREEMENT.md, sprints/sprint-125-rust-reaches-substrate.md; substrate 3adb4afe.

### Sprint 124 (2026-10-23, closed)
- **Scope:** step 1 of the Rust port: every piece of the Python host's state placed in the Rust host, with its lock, before any host code.
- **Dual contract:** signal — none fired, stated in the card; artifact pass (the table: 202 rows, every attribute of the frozen host); observation pass (202 equals the frozen host's attribute count; the per-exit costs measured under 11 µs).
- **Rubber Duck Pass:** *Observation 1 (Python written for a Rust port):* the sprint first added `scripts/design_state_check.py`, a Python check that parsed the host's source, and committed it locally; the user stopped the push ("You're still using Python ... you shouldn't be"). Disposition: resolved-here (the commit undone before it left this Mac, the script moved out of the tree, the card's contracts rewritten without it; the user's Decision 2026-10-23 below). *Observation 1a (that tool's own fault, while it lived):* its first list counted `VM._process`, a method, read from inside a subscript target; 202 attributes, not 203. *Observation 2 (an assumption measured false):* the port plan's premise that Rust ends the GIL's starvation holds only for short holds: a first-fit pthread mutex held 10 µs of 11 starved an occasional taker for 7.5 to 13.4 ms at the median. Disposition: resolved-here (the hold rule and FAIRSHARE in the design). *Observation 3 (claims past the evidence):* five sentences of the first draft went past what was measured or read (the exit's 0.72 µs credited to vm_monitor.c, 18 shared counters, the round trip's maxima "once in 200,000", the runtime lock taken "on every call", a second design that was not one). Disposition: resolved-here (each corrected against its source).
- **Files:** docs/design-rust-host-2026-10-23.md, docs/rust-port-plan-2026-10-21.md (no step runs Python), sprints/sprint-124-the-rust-host-designed.md.

### Sprint 123 (2026-10-22, closed)
- **Scope:** step 0 of the Rust port: the Python w2m's verdicts as data, and a check of any commit's records against them.
- **Dual contract:** signal — none fired, stated in the card (the script reads SESSION_COMPLETED, COMPARISON_COMPLETED, CANDIDATE_GENERATED, EXECUTION_COMPLETED, TRACE_INGESTED from 530 records); artifact pass; observation pass (plant caught on its one key; f1838b96 equal to 4b36b5ad).
- **Rubber Duck Pass:** *Observation 1 (payload anomaly):* `comparison_id` embeds the candidate id, a hash of w2m's code, so the first check called 529 equal verdicts different. Disposition: resolved-here (comparisons named by what they compared against). *Observation 2 (order of facts):* the port plan named both differing batch runs as on 37784f1a; their baselines name f1838b96 and 4b36b5ad. Disposition: resolved-here (the plan and the Decision corrected; the traces differ only in line 1, the run's id).
- **Files:** scripts/golden.py, specs/golden/python-final.json, sprints/sprint-123-the-python-oracle-frozen.md.

### Sprint 122 (2026-10-21, closed)
- **Scope:** step 7 of the several-vCPU chain: the Machine driven from somewhere other than vCPU 0's wait, so a process's threads run on several vCPUs beside other processes.
- **Rubber Duck Pass:**
  - *Sequence narration* (`signal-reports/sprint-122-report.md`, 1,060 records over the sprint's commits): the batch runs on f1838b96 and 37784f1a (frontier-643a6283510c-1792622186569209000, -1792623423346201000) each SESSION_INIT, EXECUTION_STARTED, 104 MEASUREMENT_RECORDED, EXECUTION_COMPLETED (killed), TRACE_INGESTED, COMPARISON_COMPLETED with 0 divergences, SESSION_COMPLETED; two sweeps of 458 class and 68 family runs as Sprint 121's; three family runs on two vCPUs on each commit, baseline `vcpus` 2, conform. Emits check: each of the 12 named tags fired; none fired unnamed. The two scratch batch runs on the wall clock (two vCPUs and one) conform and are reported in the card.
  - *Observation 1 (a hang the sprint's own guest found on the old path):* busychild on one vCPU spun to its limit on 55795fc1: after another process's turn, the parent ran again the thread sliced out, and the readied main thread ran 111 instructions in 20 s. Disposition: resolved-here (the next ready thread runs; the plant of the old resume fails).
  - *Observation 2 (a mechanism kept without a failing plant, then a rule too narrow):* the first kick, for a process a host thread made ready, could not be made to matter (vCPU 1's slices end at the host while a child is ready); the second, only into another process than the requester's, hung handshake 30 runs of 60. Disposition: resolved-here (the first removed; the kept rule, any wake made on a host thread kicks the running process, 60 of 60; the narrow rule is a plant, 8 of 20).
  - *Observation 3 (guests that tested less than they claimed):* crosswake's first version ended under the no-kick plant 2 runs of 3, its child slow to start; statsize's first version used CreateFileW, which makes no stat request, and passed under its plant. Disposition: resolved-here (each measured against its plant before its test was written; the worker polls without blocking, statsize uses GetFileAttributesExW).
  - *Observation 4 (a design B fault the batch run reached first):* the host's view of guest memory used vCPU 0's pages for every vCPU; no family on two vCPUs made a request with a buffer on the runtime's stack from vCPU 1. Disposition: resolved-here (37784f1a; statsize and its plant).
  - *Observation 5 (a plant not caught):* the wake written at once into a process another thread runs; pingpong ended 20 runs of 20 under it. Disposition: resolved-here as measured (reported in the card; the pending wake rests on its stated reasoning, the window a few hundred instructions inside the runtime's C).
  - *Observation 6 (before-runs measured against moving source):* the first before-runs ran while the edits were being made, and two of their four sets were void. Disposition: resolved-here (before-runs repeated in a worktree at 55795fc1).
- **Closed:** CI pending at this entry; Sprint 121's CI run 39670499276 (success on 55795fc1).

### Sprint 121 (2026-10-21, closed)
- **Scope:** Sprint 118's Deferred entry, made scope by Sprint 120: a vCPU waiting in the host to take a thread learns of one the guest readies inside the VM.
- **Rubber Duck Pass:**
  - *Sequence narration* (`signal-reports/sprint-121-report.md`, 530 records): the batch run (frontier-643a6283510c-1792611641237977000) SESSION_INIT, EXECUTION_STARTED, 104 MEASUREMENT_RECORDED, EXECUTION_COMPLETED (killed), TRACE_INGESTED, COMPARISON_COMPLETED with 0 divergences, SESSION_COMPLETED; the sweep's 458 class and 68 family runs as Sprint 120's; family-threads-1792611776442439000, family-running_thread_context-1792611777227867000 and family-wait_handoff-1792611777811559000, baseline `vcpus` 2, conform. Emits check: each of the 12 named tags fired; none fired unnamed.
  - *Observation 1 (a decision the host overwrote):* the first draft had the guest claim waiting vCPUs and end the slice itself; a plant removing the claim changed nothing in 10 runs, because the host's resume after every request rewrites SLICE_UNTIL by its own rule. Disposition: resolved-here (the rule moved to the host's `_slice_until`, which counts the waiting vCPUs; the claim removed before the commit).
  - *Observation 2 (a plant that tested a different path):* a waiting vCPU counted one too many hangs twoready, but before any wake request: it starves a lone ready thread with no vCPU waiting. A doubled count, the case twoready was written for, does not hang it (5 of 5), since a vCPU that loads one worker while the other is ready gets a slice's end. Disposition: resolved-here (the test's docstring says which path its plant tests).
  - *Observation 3 (a judge the card set before measuring):* the card's draft judged by vCPU 1's total; in one run of the first probe the worker ran on vCPU 0 and the main thread on vCPU 1, and a probe whose worker never blocked between rounds made at most one wake request. Disposition: resolved-here (the main thread waits until the worker blocks; the judge is the worker's exact count on the waiting vCPU and no switch in the VM).
  - *Observation 4 (a claim not read from the source):* the card's first draft of the observed section said wait_handoff's threads run one at a time by design. Disposition: resolved-here (read from probes/wait_handoff/wait_handoff.c: its waiters run a few instructions between waits, about 10,000 in all).
  - *Observation 5 (a test that passed by luck, and a failure whose name was lost):* the close's first gate run failed 1 of 2,535 tests, and the command kept only pytest's last line. Five runs of the five files that race vCPUs named it: the several-process refusal's test ran spinchild at most 5 times, and spinchild reaches the refused wait in 11 runs of 20. Disposition: resolved-here (4a0f341b: up to 30 runs, 10 test runs of 10 pass; gate output now kept whole in build/).
- **Closed:** CI pending at this entry; Sprint 120's CI run 39322513903 (success on 2f16919b).

### Sprint 120 (2026-10-18, closed)
- **Scope:** a defect class, not a step: every timed wait ends, and no host thread waits with nothing able to end its wait (Decision 2026-10-18 user).
- **Rubber Duck Pass:**
  - *Sequence narration* (`signal-reports/sprint-120-report.md`, 530 records): the batch run (frontier-643a6283510c-1792347897080595000) SESSION_INIT, EXECUTION_STARTED, 104 MEASUREMENT_RECORDED, EXECUTION_COMPLETED (killed, 36.206 s), TRACE_INGESTED, COMPARISON_COMPLETED with 0 divergences, SESSION_COMPLETED; the sweep's 458 class and 68 family runs as Sprint 119's; family-threads-1792348019048745000, family-running_thread_context-1792348019878009000 and family-wait_handoff-1792348020413763000, baseline `vcpus` 2, conform. Emits check: each of the 12 named tags fired; none fired unnamed.
  - *Observation 1 (a hang recorded as a deferral):* Sprint 119 deferred "vCPU 0 keeps no deadline while it waits", which its own plant measured as a 60 s hang. Disposition: resolved-here (Decision 2026-10-18 user; the Deferred entry resolved).
  - *Observation 2 (a class larger than its member):* the deferred entry named one wait; the class held six: expiry on the wall clock, untimed host waits, a mutual suspend, a child's whole budget, per-process clocks, a context read under a held thread. Disposition: resolved-here (eight guests in tests/data/timed_waits, each hang measured before its fix and each plant measured to fail).
  - *Observation 3 (a judge claimed unchanged):* the card said the batch run would be unchanged; its exits moved (hvc 12 from 2 to 0, hvc 4 from 8 to 10) and its measurements from 107 to 104. Disposition: resolved-here (the card restated; the review of 2026-10-18 isolated the cause to the second process's slices and the catch-up together).
  - *Observation 4 (an Emits that named tags by reference, again):* the card named the sweep's tags "as on Sprint 119's sweep", so the check counted REDUCTION_COMPLETED, DISCREPANCY_REPORTED and DISCREPANCY_ROUTED unnamed, the slip of Sprint 114's Observation 1. Disposition: resolved-here (the card's Emits amended at the close, each tag by name).
  - *Observation 5 (tests that passed by luck):* five tests of races (the untimed plant, the counter on vCPU 1, the context retry, the no-wait plant, the lock plant) reached their path in only some runs, and the gates missed the lock plant once. Disposition: resolved-here (each runs until its path is reached, with a bound).
  - *Observation 6 (a file committed by mistake):* 25e57995's `git add -A` over docs committed `docs/research-paper-growing-a-runtime-from-observation.md`, kept uncommitted until then. Disposition: recorded (the user kept it committed, chat 2026-10-19).
- **Closed:** CI pending at this entry; Sprint 119's CI run 39113307657 (success on cbf4fc82).

### Sprint 119 (2026-10-18, closed)
- **Scope:** step 1, chain step 6: a thread suspended or read while another vCPU runs it; judged by the threads and running_thread_context families on two vCPUs.
- **Rubber Duck Pass:**
  - *Sequence narration* (`signal-reports/sprint-119-report.md`, 529 records): the batch run (frontier-643a6283510c-1792313027375176000) as Sprint 118's in kind and order, 0 divergences; the sweep's 458 class and 68 family runs as Sprint 118's; family-threads-1792313149099557000 and family-running_thread_context-1792313150102405000, baseline `vcpus` 2: SESSION_INIT, CANDIDATE_GENERATED, COMPARISON_COMPLETED (conforms), CANDIDATE_ACCEPTED, SESSION_COMPLETED. Emits check: each of the 12 named tags fired; none fired unnamed.
  - *Observation 1 (a save with no notify):* the first run hung, both host threads on the VM's condition; vCPU 0 put its thread down and waited without telling the suspender. Disposition: resolved-here (it notifies).
  - *Observation 2 (a thread run from stale registers):* 4 of 8 runs printed events.created_suspended with old events; vCPU 0, back from an idle turn, resumed main from the exit's registers while vCPU 1 had run it on. Disposition: resolved-here (`saved` through the idle turn; picks by deadline skip held threads; 0 of 10 since).
  - *Observation 3 (a plant the rows could not see):* without the flush 13 of 18 context reads read an unwritten slot, yet the rows conformed 10 of 10. Disposition: resolved-here (the test judges the pending-slot witness).
  - *Observation 4 (a vCPU that keeps no deadline while it waits):* the no-wait plant ran to the 60 s limit. Disposition: deferred (Deferred 2026-10-18, Sprint 119).
- **Closed:** CI pending at this entry; Sprint 118's CI run 39106709110 (success on 1ed6c4f6).

### Sprint 118 (2026-10-18, closed)
- **Scope:** step 1, chain step 5: one process's guest threads on several vCPUs, a host thread serving each vCPU past the first; judged by wait_handoff on two vCPUs.
- **Rubber Duck Pass:**
  - *Sequence narration* (`signal-reports/sprint-118-report.md`, 528 records): the batch run (frontier-643a6283510c-1792306704577911000) as Sprint 117's in kind and order, 0 divergences; the sweep's 458 class and 68 family runs as Sprint 117's; family-wait_handoff-1792306860114860000, baseline `vcpus` 2: SESSION_INIT, CANDIDATE_GENERATED, COMPARISON_COMPLETED (conforms), CANDIDATE_ACCEPTED, SESSION_COMPLETED. Emits check: each of the 12 named tags fired; none fired unnamed.
  - *Observation 1 (a word that named the wrong holder):* the first two-vCPU run hung; the runtime lock's exchange (Sprint 106) left the last waiter's value in the word, which RuntimeLock (Sprint 116) reads as the holder. Disposition: resolved-here (compare-and-swap, guest and host).
  - *Observation 2 (a wakeup lost between two vCPUs):* 1 of 3 runs stopped as a deadlock; a signal from vCPU 1 before vCPU 0's switch blocked the waiting thread woke nobody. Disposition: resolved-here (`w2m_block` blocks under the lock; 30 of 30 runs conform since).
  - *Observation 3 (a plant the rows could not see):* ignoring the mark let vCPU 1 run a stale copy of vCPU 0's thread, retiring nothing. Disposition: resolved-here (`_load_thread` refuses; the plant stops 5 of 5).
  - *Observation 4 (a second vCPU that idles):* vCPU 1 ran 144 of 10,207 instructions. Disposition: deferred (Deferred 2026-10-18, Sprint 118).
- **Closed:** CI pending at this entry; Sprint 117's CI run 39100682750 (success on 165d49c5).

### Sprint 117 (2026-10-17, closed)
- **Scope:** step 1, chain step 4: the kick; a host thread stops one vCPU on demand through its channel's kick word, and the monitor's kicker calls hv_vcpus_exit.
- **Rubber Duck Pass:**
  - *Sequence narration* (`signal-reports/sprint-117-report.md`, 527 records): the batch run (frontier-643a6283510c-1792300672353112000) as Sprint 116's in kind and order, COMPARISON_COMPLETED with 0 divergences; the sweep's 458 class and 68 family runs as Sprint 116's. Emits check: each of the 12 named tags fired; none fired unnamed. The kick fires no tag: it is a monitor exit kind, counted in `VM.kicks`, and no run on the record was kicked.
  - *Observation 1 (a wait that could miss a kick):* the channel's `wait_change` waits for exactly old + 1, so two kicks before the kicker looked would leave it waiting forever. Disposition: resolved-here (the kicker waits for any change; Decision 2026-10-17, Sprint 117).
  - *Observation 2 (a commit before its count was read):* the code commit went in on the test run's exit status before its pass count was read (188 passed, read after). Disposition: resolved-here (the count read; `pipefail` made the status pytest's).
- **Closed:** CI pending at this entry; Sprint 116's CI run 39099022393 (success on 6230f933).

### Sprint 116 (2026-10-17, closed)
- **Scope:** step 1, chain step 3: the guest runtime lock's host side as one object, `RuntimeLock`: one rule frees it, a shared lock is taken.
- **Rubber Duck Pass:**
  - *Sequence narration* (`signal-reports/sprint-116-report.md`, 527 records): the batch run (frontier-643a6283510c-1792299009808193000) as Sprint 115's in kind and order, COMPARISON_COMPLETED with 0 divergences; the sweep's 458 class and 68 family runs as Sprint 115's. Emits check: each of the 12 named tags fired; none fired unnamed.
  - *Observation 1 (a take that would deadlock):* the card's first draft took the lock whenever vcpu_count > 1; the litmus and runtime-lock tests drive vCPUs 1-4 from inside vCPU 0's on_dispatch, which a lock held across the hook would stop. Disposition: resolved-here (taken only when `shared`, chain step 5; Decision 2026-10-17, Sprint 116).
  - *Observation 2 (a check by place, not by frame):* the old release tested the exit's pc against the runtime image, which misses a bridge's words between `lock_runtime` and its call. Disposition: resolved-here (freed when the answer does not resume the vCPU at its exit pc).
- **Closed:** CI pending at this entry; Sprint 115's CI run 39096131917 (success on 7b73acb6).

### Sprint 115 (2026-10-17, closed)
- **Scope:** step 1, chain step 2: the `Vcpu` object, passed to every handler that reads a vCPU's state; no `VM` attribute names a served vCPU.
- **Rubber Duck Pass:**
  - *Sequence narration* (`signal-reports/sprint-115-report.md`, 527 records): the batch run (frontier-643a6283510c-1792295964556678000) as Sprint 114's in kind and order (SESSION_INIT, EXECUTION_STARTED, 107 MEASUREMENT_RECORDED, EXECUTION_COMPLETED, TRACE_INGESTED, COMPARISON_COMPLETED with 0 divergences, SESSION_COMPLETED); the sweep's 458 class and 68 family runs as Sprint 114's. Emits check: each of the 12 named tags fired; none fired unnamed.
  - *Observation 1 (a judge that cannot judge):* the card expected Sprint 114's instruction count; the batch run stops when its log holds 168 lines, which moves between runs (80.8, 81.7 and 82.5 billion on 114, 110 and 115). Disposition: resolved-here (the card amended: lines, divergences and exit kinds judge unchanged behaviour).
  - *Observation 2 (a failure an exit code hid):* the first test run failed one test, and a pipe to `tail` reported exit 0. Disposition: resolved-here (the class checked: four patched methods in `tests/test_instruction_count.py`; reruns keep pytest's status with `pipefail`).
- **Closed:** CI pending at this entry; Sprint 114's CI run 39073184106 (success on 36a9556a).

### Sprint 114 (2026-10-17, closed)
- **Scope:** step 1, chain step 1: one exit handler (`VM.serve_exit`), no change in behaviour.
- **Rubber Duck Pass:**
  - *Sequence narration* (`signal-reports/sprint-114-report.md`, 527 records): the batch run as Sprint 110's in kind and order (EXECUTION_STARTED, 107 MEASUREMENT_RECORDED, EXECUTION_COMPLETED, TRACE_INGESTED, COMPARISON_COMPLETED with 0 divergences); the sweep's item runs as Sprint 112's.
  - *Observation 1 (an Emits that named tags by reference):* the card wrote "their item runs' tags as on Sprint 112's sweep", so the check counts CANDIDATE_ACCEPTED, CANDIDATE_GENERATED, REDUCTION_COMPLETED and the discrepancy tags as unnamed. Disposition: resolved-here for cards from 115 on (each tag by name).
- **Closed:** CI pending at this entry; Sprint 112's CI run 39071634064 (success on 5ae905d4) and Sprint 113's 39072223099 (success on 58051c43).

### Sprint 113 (2026-10-17, closed)
- **Scope:** step 1 resumes: the host for several vCPUs designed twice (docs/design-several-vcpus.md).
- **Rubber Duck Pass:**
  - *Sequence narration:* `signal-reports/sprint-113-report.md`: no record; a design sprint.
  - *Observation 1 (polling that a documented call removes):* the parked attempt cut every run at 50 ms so a host thread could see a stop; hv_vcpu.h's hv_vcpus_exit, callable from any thread and sticky for a vCPU not running, stops one on demand. Disposition: resolved-here (the kick, chain step 4).
- **Closed:** CI pending at this entry; Sprint 112's CI run 39071634064 (see the next entry).

### Sprint 112 (2026-10-17, closed)
- **Scope:** in H4's place: discrepancy reports that name the program and state they failed from, and class runs' reports after a reduction.
- **Rubber Duck Pass:**
  - *Sequence narration* (`signal-reports/sprint-112-report.md`, 526 records of the sweep on b1ad0cfb): per class and family SESSION_INIT, CANDIDATE_GENERATED, COMPARISON_COMPLETED, then CANDIDATE_ACCEPTED (524) or, for FCOS and FSIN, REDUCTION_COMPLETED then DISCREPANCY_REPORTED then DISCREPANCY_ROUTED (8 each), SESSION_COMPLETED. Emits check: every named tag fired; CANDIDATE_ACCEPTED fired unnamed.
  - *Observation 1 (a fix the gate turned on old records):* removing the class runs from the known list held today's earlier FCOS and FSIN records. Disposition: resolved-here (a fix's commit time bounds what is known).
  - *Observation 2 (a reduction no one ran):* a family's probe has no reducer, so no REDUCTION_COMPLETED is claimed for it. Disposition: deferred (Deferred 2026-10-17, Sprint 112).
- **Closed:** CI pending at this entry; Sprint 111's CI run 39069254531 (success on a285ffc9).

### Sprint 111 (2026-10-17, closed)
- **Scope:** H3 of the SDD hardening order: Signal Reports made from the record, each card's Emits checked against what fired.
- **Rubber Duck Pass:**
  - *Sequence narration:* `signal-reports/sprint-111-report.md`: the sprint made no record; its card names no tag. Its reports for 108 to 110 narrate their records (108: 527 records; 110: EXECUTION_STARTED 0.207 s, EXECUTION_COMPLETED 33.965 s).
  - *Observation 1 (cards that name too little):* 108's card named MEASUREMENT_RECORDED alone while its records fired nine other tags; 110's named four of seven. Disposition: resolved-here for cards from 111 on (WORKING_AGREEMENT's rule: Emits name every tag the code fires).
  - *Observation 2 (a summary that hides one producer):* substrate's narration_summary of 110's batch run counts 5 producers started and 4 completed, with none failed or cancelled. Disposition: deferred (the next sprint that reads lifecycles).
- **Closed:** CI pending at this entry; Sprint 110's CI run 39068519125 (success on 2be498eb).

### Sprint 110 (2026-10-17, closed)
- **Scope:** H2's second half: the frontier's execution placed in time on its record, within a stated bound.
- **Rubber Duck Pass:**
  - *Sequence narration* (frontier-643a6283510c-1792269281973223000, by `t` from RunStarted): SESSION_INIT 0.001 s; EXECUTION_STARTED 0.207 s (timeout_seconds 10,800, kill_grace_seconds 60); the 107 MEASUREMENT_RECORDED; EXECUTION_COMPLETED 33.965 s (killed, elapsed_seconds 33.546); TRACE_INGESTED 33.978 s; COMPARISON_COMPLETED (0 divergences); SESSION_COMPLETED 34.265 s. Before, every event of a run carried its end's time.
  - *Observation 1 (an event loop held by a run):* `machine.run` on the loop's thread kept substrate's writer from stamping anything until the run ended. Disposition: resolved-here (`asyncio.to_thread`).
  - *Observation 2 (a stop that skipped its completion):* 41 of 43 frontier records with no EXECUTION_COMPLETED stopped at SERVICE_UNIMPLEMENTED. Disposition: resolved-here.
  - *Observation 3 (a plant that missed by chance):* the EnTSO-off litmus plant missed one time in 600, not one in a billion. Disposition: resolved-here (ten runs).
  - *Correction:* Sprint 109's table and Deferred entry count T3's 452 unchecked as frontier records; they are 394 laptop probe runs, 46 frontier runs and 12 judge records.
- **Closed:** CI pending at this entry; Sprint 109's CI run 39067108857 (success on a1060c55).

### Sprint 109 (2026-10-17, closed)
- **Scope:** H2 of the SDD hardening order, its first half: every record checked against the vocabulary's rules.
- **Rubber Duck Pass:**
  - *Sequence narration:* no record of its own; `scripts/check_records.py` read the application events of 32,755 records (SESSION_INIT through SESSION_COMPLETED) and found 1,962 violations, every one in a record made before 2026-10-17 or in the two known class- and family-run defects.
  - *Observation 1 (placeholders that pass a format check):* 789 sha256 fields are `sha256:` and 64 zeros, which E1's pattern accepts. Disposition: resolved-here (named as a placeholder); the payloads deferred.
  - *Observation 2 (a report with no reduction):* S5 fails 676 times in class and family runs, which have no reducer. Disposition: deferred (Deferred 2026-10-17).
  - *Observation 3 (a start emitted at the end):* the frontier emits EXECUTION_STARTED after its run, without timeout_seconds, so T3 is unchecked on 452 records. Disposition: deferred to the next sprint.
- **Closed:** CI pending at this entry; Sprint 108's CI run 39065947856 (success on 4a8f0eb1).

### Sprint 108 (2026-10-17, closed)
- **Scope:** H1 of the SDD hardening order: vocabulary 0.9 (P20) and the batch run's numbers on its record.
- **Rubber Duck Pass:**
  - *Sequence narration* (frontier-643a6283510c-1792267359516238000, read by `scripts/run_numbers.py`): SESSION_INIT; EXECUTION_STARTED; 23 MEASUREMENT_RECORDED `vm_compile_blocks`; 84 MEASUREMENT_RECORDED of P20 (instructions and PAUSEs for the process and 28 threads, the rate, seconds in the VM and by exit, exits by hvc, switches, wakes, the load); EXECUTION_COMPLETED (elapsed_seconds and exit_code still absent: H2's); TRACE_INGESTED; COMPARISON_COMPLETED (0 divergences); SESSION_COMPLETED.
  - *Observation 1 (a check that could not fail):* the process's PAUSEs were first the sum of the threads'. Disposition: resolved-here (`VM.pauses_executed`, the monitor's count).
  - *Observation 2 (an expectation the code cannot meet):* the card's 1% check against the runner's time around `machine.run`, which the runner does not take. Disposition: deferred to H2 (EXECUTION_COMPLETED's elapsed_seconds).
  - *Observation 3 (a push without its sweep):* CI run 39062977196 failed on 24853602, which carried Sprint 107's part 1 with the class and family indexes stale. Disposition: resolved-here (the sweep on e1cbc46f).
- **Closed:** CI pending at this entry; Sprint 106's CI run 39060474900 (success on 8398f1c8).

### Sprint 106 (2026-10-17, closed)
- **Scope:** roadmap step 1: one lock around the guest runtime, so guest code on several vCPUs calls it one vCPU at a time.
- **Rubber Duck Pass:**
  - *Sequence narration:* no new tag; two x86 threads on vCPUs 1 and 2 through the runtime's heap; 458 class runs, 68 family runs and the batch run at one vCPU.
  - *Observation 1 (a write that is two writes):* `struct.pack_into` zero-fills before it packs, and a vCPU read a zeroed dispatch slot and branched to 0. Disposition: resolved-here (one 64-bit store after a barrier; 0 faults in 60 runs).
  - *Observation 2 (a lock across a frame the host abandons):* on_exception resumed a vCPU at the dispatcher with w2m_exception's frame holding the lock; the gates hung 8 h 59 min. Disposition: resolved-here (the host frees the lock there and at each run's start).
  - *Observation 3 (a timeout that cannot end its test):* pytest-timeout's signal method raises a BaseException, which substrate's producer task does not catch, so the run waited forever. Disposition: resolved-here for w2m (`timeout_method = "thread"`); deferred to substrate (a BaseException in a producer leaves its run waiting).
- **Closed:** CI pending at this entry; Sprints 103-105's CI run 39020048181 (success on 2785934a).

### Sprint 105 (2026-10-17, closed)
- **Scope:** roadmap step 1's gate: the five litmus tests as x86 compiled by w2m, one thread a vCPU.
- **Rubber Duck Pass:**
  - *Sequence narration:* no new tag; the tests run inside a VM run's on_dispatch hook, through vCPUs 1-4's channels.
  - *Observation 1 (a judge with a chance of failing):* SB's relaxed outcome is rare (0 in 3 runs of 100). Disposition: resolved-here (measured, then up to five runs).
  - *Observation 2 (state copied, not set):* SLICE_UNTIL on a private page came from vCPU 0 as 0. Disposition: deferred to Sprint 106 (the scheduler sets it per vCPU).
- **Closed:** CI pending at this entry.

### Sprint 104 (2026-10-17, closed)
- **Scope:** N vCPUs on their own threads, each with its private translation chain (design B).
- **Rubber Duck Pass:**
  - *Sequence narration:* no new tag; 458 class runs, 68 family runs and the batch run at one vCPU as before.
  - *Observation 1 (an assumption never wired):* the backend decision rested on guest TSO, and no write of ACTLR_EL1.EnTSO existed. Disposition: resolved-here; Sprint 105's plant shows what it holds.
  - *Observation 2 (a name reused):* Channel's `base` held two meanings; one VM hid it. Disposition: resolved-here.
- **Closed:** CI pending at this entry.

### Sprint 103 (2026-10-17, closed)
- **Scope:** several vCPUs: the per-vCPU state measured and two designs.
- **Rubber Duck Pass:**
  - *Sequence narration:* no run; Apple's and Arm's documents quoted.
  - *Observation 1 (none):* the inventory's counts are reproducible from the card.
- **Closed:** CI pending at this entry; Sprints 098-102's CI run 39016421561 (success on ca5cb64b).

### Sprint 102 (2026-10-17, closed)
- **Scope:** Sprint 067's step E: each laptop game run proves the declared initial state (docs/windows-reference-machine-2026-10-12.md §5).
- **Rubber Duck Pass:**
  - *Sequence narration:* no new tag; the proof is a file of the run's output beside exec.json; three laptop jobs: one stopped by the check, the judge, the plant.
  - *Observation 1 (the check found a shared folder):* the first run stopped with 4 TEMP entries, one held open by another program in the session (Steam now runs on the laptop). Disposition: resolved-here (the game's TEMP is the job's own).
- **Closed:** CI pending at this entry.

### Sprint 101 (2026-10-17, closed)
- **Scope:** vocabulary 0.8 (P19), the VM compiler's counts as MEASUREMENT_RECORDED.
- **Rubber Duck Pass:**
  - *Sequence narration:* the batch run's runner emits 23 MEASUREMENT_RECORDED between its stop and EXECUTION_COMPLETED; read back through `read_record`, each equal to the printed count.
  - *Observation 1 (the vocabulary still holds names nothing emits):* 25 of 62 tags have no emit site (review 2026-10-17). Disposition: deferred to design sprint D1 (plan-2026-10-09.md).
- **Closed:** CI pending at this entry.

### Sprint 100 (2026-10-17, closed)
- **Scope:** the open question of Sprint 089: a 0x67 address through R8D-R15D, 32 bits by the SDM.
- **Rubber Duck Pass:**
  - *Sequence narration:* no new tag; the address test, the equal-words judges, the 039 and 041 case sets, 456 class runs, 68 family runs, the batch run conforming.
  - *Observation 1 (one rule, four homes):* the rule lived in lift, symbolic_lift, the stencil table and the case generator's placement; each was found by its own failing judge, not by a search first. Disposition: resolved-here; the review of 2026-10-17 names the same spread (change amplification), plan-2026-10-09.md D2.
  - *Observation 2 (a re-derivation regressed another sprint):* emptying HOST_LOWERED handed the x87 forms to stencils derived without their runtime symbols. Disposition: resolved in Sprint 098's 1d55c7da, with a judge.
- **Closed:** CI pending at this entry.

### Sprint 099 (2026-10-17, closed)
- **Scope:** a guest fault gives back the instructions its block counted and did not run.
- **Rubber Duck Pass:**
  - *Sequence narration:* no new tag; 4 count tests and the census test; the batch run gave back 167 faults, all stack growth (0 dispatched).
  - *Observation 1 (a test held the defect):* the census test asserted a count the defect produced. Disposition: resolved-here.
- **Closed:** CI pending at this entry.

### Sprint 098 (2026-10-17, closed)
- **Scope:** the batch run's model-stepped classes (hvc 10 and 11) made native.
- **Rubber Duck Pass:**
  - *Sequence narration:* 458 class runs (456 CANDIDATE_ACCEPTED, FSIN and FCOS pinned), 68 family runs to CANDIDATE_ACCEPTED, the batch run COMPARISON_COMPLETED `conforms` with 544 model exits.
  - *Observation 1 (the measure found what the judges could not):* every judge passed while the VM compiled the x87 forms to the model step; only the exit count showed it. Disposition: resolved-here (the derivation's symbols; a judge of them).
  - *Observation 2 (a plant blind in the case sets):* ties never occurred in the x87 cases. Disposition: resolved-here (the native differential).
- **Closed:** CI pending at this entry; Sprint 097's CI run 39004897894 (success on bc626dfc).

### Sprint 097 (2026-10-16, closed)
- **Scope:** roadmap step 2's last item: the GICv3 made before any vCPU.
- **Rubber Duck Pass:**
  - *Sequence narration:* no new tag; 68 family item runs to CANDIDATE_ACCEPTED; the frontier's SESSION_INIT, EXECUTION_STARTED, its stop, TRACE_INGESTED, COMPARISON_COMPLETED `conforms`, SESSION_COMPLETED.
  - *Observation 1 (none):* the GIC changed no row; nothing raises an interrupt yet.
- **Closed:** CI pending at this entry; Sprint 094's CI run 38999283011 (success on 293abc6a).

### Sprint 096 (2026-10-16, closed-partial)
- **Scope:** roadmap step 2: a PAUSE yields the slice when another thread is ready.
- **Rubber Duck Pass:**
  - *Sequence narration:* 458 class runs (456 CANDIDATE_ACCEPTED, FSIN and FCOS pinned) and 68 family runs to CANDIDATE_ACCEPTED; frontier runs to the stop, each COMPARISON_COMPLETED `conforms`.
  - *Observation 1 (timing surprise, of the measure):* PAUSEs fell 8 to 10 times and the instructions to the stop did not: the game's busy-wait on QueryPerformanceCounter paces the run. Disposition: recorded (Decision 2026-10-16, how steps 2-5 are measured; KIT_DIARY #63).
  - *Observation 2 (unused path):* a warp at a PAUSE with only timed waiters fired 0 times. Disposition: resolved-here (not kept).
- **Closed:** partial (timing deferred); CI pending at this entry.

### Sprint 095 (2026-10-16, closed-partial)
- **Scope:** roadmap step 2: a release readies the waiter it can satisfy, not every waiter.
- **Rubber Duck Pass:**
  - *Sequence narration:* frontier runs to the stop before and after (COMPARISON_COMPLETED `conforms` each); wait_handoff's item run CANDIDATE_REJECTED on the new wait-all row, then CANDIDATE_ACCEPTED after the fix; 68 family runs to CANDIDATE_ACCEPTED.
  - *Observation 1 (missing pair, of the judge):* two plants came back blind once the wake path moved, and the case they stood for had no row; the new row found a defect (KIT_DIARY #54 again). Disposition: resolved-here.
  - *Observation 2 (silent gap, of the records):* `## Built` stopped at Sprint 066, the tail held about 60 entries, and Sprint 094's frontier logs were never committed. Disposition: resolved-here (Built entries 065, 067-097; tail trimmed to 10; the logs committed in a87ec94c).
- **Closed:** partial (timing deferred); CI pending at this entry.

### Sprint 094 (2026-10-16, closed)
- **Scope:** roadmap step 1: the whole batch run measured on today's tree.
- **Rubber Duck Pass:**
  - *Sequence narration:* five runs on 3d703c02 (warm-up, two to the stop, two at 100 billion instructions), all conforming; 48.7 and 44.3 s at equal work against 070's 50.5 and 51.6 s; 2,055 to 2,259 M instructions a second, an fps-equivalent of 50 to 55.
  - *Observation 1 (the rate counts spinning):* the main thread retires 63.8% of the instructions with 233.6 million PAUSEs. Disposition: carried to roadmap step 2 (waits inside the VM).
  - *Observation 2 (facts tested that were already known):* in 093's last runs, documented facts (NTFS's lazy directory size, session 0 isolation, Win11's notepad hand-off) were found by tests. Disposition: resolved-here (rule kept: look up before measuring).
- **Closed:** CI pending at this entry; 093's CI run 38998451939 on 3d703c02 failed: test_reach's test of the copied Windows skipped without the `depot` mark, which W2M_REQUIRE_REFERENCES=1 turns into an error; marked in this commit.

### Sprint 093 (2026-10-16, closed)
- **Scope:** E2, the static reach of both games' imports through the copied Windows to system calls; closed once by the full review of 2026-10-16 (item 3).
- **Rubber Duck Pass:**
  - *Sequence narration:* the walk built (API set schema, forwarders, ordinals, both stub shapes, value tracking, the pointer tier); E3's misses went 11, 8, 4, 0, each a class searched whole; a person's 884 s session judged it at 303 of 305; the review then made the roadmap and the user stopped the E1 judge and the stutter replays.
  - *Observation 1 (the bound is loose):* the pointer tier reaches 1,336 of 1,486 win32k calls, so passing the judge says little. Disposition: resolved-by-decision (the review cuts static reach as an ordering tool; its loader fixes stay).
  - *Observation 2 (the syscall provider may count the kernel's own calls):* NtPlugPlayControl's user stack tops at NtDeviceIoControlFile's stub. Disposition: deferred (recorded in the card; a probe-name against stub-name test settles it if a trace's call list is used as a kernel's list in roadmap step 7).
- **Closed:** CI pending at this entry; 092's CI run 38977332489 (success on 0237a77a).

### Sprint 092 (2026-10-16, closed)
- **Scope:** step 1 of the solution, the laptop's recordings: DTrace installed; E1, a process's start; E3, calls per frame.
- **Rubber Duck Pass:**
  - *Sequence narration:* DTrace installed and booted; E1 traced probes/hello's 483 system calls (342 before main; one port, csrss's ApiPort); E3 ran the game four times, three failing on one DTrace defect each, then after a search of each defect's class, once with every check: 1,132 frames in 10.012 s, 314.1 NT and 18.4 win32k system calls a frame.
  - *Observation 1 (one error fixed at a time, of the method):* each failed run named one defect and the next run found the next. Disposition: resolved-here (each class searched whole, in the tool's issues and by short tests, before the next run: eight more found, among them three ntdll stubs with no syscall probe and frames from a second clock); recorded as the working rule.
  - *Observation 2 (a contract written from belief):* the card said E1's trace ends with NtTerminateProcess; 15 calls follow it. Disposition: resolved-here (corrected in the observed section from the trace).
- **Closed:** CI pending at this entry; 091's CI run 38965622909 (success on e1b41f1b).

### Sprint 091 (2026-10-16, closed)
- **Scope:** step 0 of the solution: the laptop's C:\Windows and its SOFTWARE and SYSTEM hives copied whole.
- **Rubber Duck Pass:**
  - *Sequence narration:* 183,280 files listed on the laptop; tar given that list copied 183,222 (68.3 GB); 40 named unreadable (files Windows holds open), 18 neither copied nor named (12 reparse points, 6 deleted after the listing), 26 written during the copy; the first copy's 5,683 files equal by sha256.
  - *Observation 1 (silent loss, of the tool):* the laptop's tar, walking the tree itself, left 25,796 readable files out with no error. Disposition: resolved-here (the file list given explicitly; the judge counts every listed file).
  - *Observation 2 (a judge written from memory):* the tar-error pattern did not match the real line, so the judge would have called every unreadable file missing. Disposition: resolved-here (pattern and test from the laptop's own line).
- **Closed:** CI pending at this entry.

### Sprint 090 (2026-10-16, closed-partial)
- **Scope:** compile_insn run on the symbolic templates gives stencils.
- **Rubber Duck Pass:**
  - *Sequence narration:* no run; the judge compared 1,000 instructions' stencils with compile_insn: 996 equal, 0 differing, 4 pinned SSE with no stencil; 363,546 stencils made.
  - *Observation 1 (silent bake, of the method):* a set compared by hash took the wrong path without a fork. Disposition: resolved-here (scale in the shape); the class of hazard (code that hashes or tests type instead of value) noted in the card.
  - *Observation 2 (the measure the Decision named):* forks multiply across ops. Disposition: recorded (Decision 2026-10-16, the back end's form: control in C, per-op stencils).
- **Closed:** partial (the stencils' count); CI pending at this entry.

### Sprint 089 (2026-10-16, closed)
- **Scope:** the compiler from the rules, first link: each rule's ops as a template, by running it on symbols.
- **Rubber Duck Pass:**
  - *Sequence narration:* no run; the template judge filled 5,504 templates for 1,441,089 instructions of three sources and found every one equal to lift().
  - *Observation 1 (blind judge, of the judge):* the game's record alone passed with a shape that missed a test lift() makes; the spec rows and class cases caught it. Disposition: resolved-here.
  - *Observation 2 (a suspected semantics defect):* 0x67 addressing through R8D-R15D in lift(). Disposition: open question, to be measured.
- **Closed:** CI run 38846969380 (success on 1a8351b0).

### Sprint 088 (2026-10-16, closed)
- **Scope:** the large review of 082-086 on the service census (H3, M1): Windows's own exports and COM registry from the laptop, DLL classes, P/Invoke from metadata, COM GUIDs, Steam interfaces.
- **Rubber Duck Pass:**
  - *Sequence narration:* no run; System32 and SysWOW64 copied from the laptop; `--windows` built the two tables; the census rewrote store/census/static with the judge holding (803 of 803; 322 of 331).
  - *Observation 1 (missed input, of the work):* the first copy of System32 missed 227 files, and SysWOW64's 233, by the glob's case. Disposition: resolved-here (counted against the laptop's own listing).
  - *Observation 2 (low precision, of the work):* candidate lookups are about 8% precise. Disposition: recorded (labelled as candidates).
- **Closed:** CI run 38843441901 (success on a41823fd).

### Sprint 087 (2026-10-16, closed)
- **Scope:** the large review of 082-086 on the instruction census (H2, H3 in part, M2): jump tables out of the sweep, a precision judge, the 32-bit census, one count per image, the module map in the frontier record.
- **Rubber Duck Pass:**
  - *Sequence narration:* no record run; the census rewrote store/census/static/instructions-*.json (6,201,183 instructions in Hail to the Rainbow; 848,970 in BioShock 2's 32-bit images); a scratch frontier wrote a module map.
  - *Observation 1 (blind judge, of the work):* 086's judge measured recall only, so jump-table data counted as code went unseen. Disposition: resolved-here (the precision judge, 0 after).
  - *Observation 2 (residual):* BioShock 2's count is a lower bound with no recall judge (no 32-bit run; no `.pdata`). Disposition: open, for the 32-bit work.
  - *Observation 3 (process, of the work):* I started one census with the shell's `&`, against the rule for background jobs; stopped it and reran it as a job.
- **Closed:** CI run 38840947861 (success on 4584edfb).

### Sprint 086 (2026-10-16, closed)
- **Scope:** the instruction census (recentering 2026-10-16, step 1): every x86-64 image's reachable code, by how w2m runs each class (native, model, not lifted); the walk joined with a `.pdata` sweep.
- **Rubber Duck Pass:**
  - *Sequence narration:* no run; `class_census.py --depots` wrote store/census/static/instructions-*.json; the judge placed 97.9-99.8% of the batch run's compiled instructions in their images' reachable sets.
  - *Observation 1 (blind method, of the work):* the census of Sprint 044 used the walk alone, which misses 32% of the mono code the game ran. Disposition: resolved-here (the `.pdata` sweep).
  - *Observation 2 (noise, of the work):* the sweep decodes bytes that are not code (CLI, STI, IN, OUT in a game). Disposition: recorded; not measured.
  - *Observation 3 (my judge, first version):* mapped addresses by preferred base and reported mono 10% reachable; three DLLs share 0x180000000 and were relocated. Disposition: resolved-here (bases from the record).
  - *Observation 4 (silent gate):* 085 closed with CI failing on its skipped depot tests. Disposition: resolved-here (`depot` marker).
- **Closed:** CI run 38837044927 (success on 066c69a4); Sprint 085 closes on it too.

### Sprint 085 (2026-10-16, closed)
- **Scope:** the static census (recentering 2026-10-16, step 1): every depot's images and the Windows functions they import or name, with w2m's coverage; PE32 read.
- **Rubber Duck Pass:**
  - *Sequence narration:* no run; `static_census.py` wrote store/census/static (4 depots); the judge found all 803 Windows-run imports and 322 of 331 run-time lookups (the 9 left: Unity plugin hooks nothing exports); PE32 imports equal objdump's on 16 images.
  - *Observation 1 (unmeasured claim, of the work):* I counted Hail to the Rainbow's 122 0x14C images as 32-bit code in chat and in the new recentering document; they are .NET assemblies. Disposition: resolved-here (the document and the card correct it; the user told in chat).
  - *Observation 2 (the gap the census exists for):* 396 of the game's 634 imported Windows functions and 927 of BioShock 2's 1,114 are not served. Disposition: open, the work list for the sprints after 086.
- **Closed:** not on its own CI: run 38834313370 failed (its two depot tests skipped in CI, where W2M_REQUIRE_REFERENCES fails a skip); 085 closes on Sprint 086's CI (run 38837044927, success on 066c69a4), whose commit carries the fix.

### Sprint 084 (2026-10-16, closed)
- **Scope:** the reviewer's small review of 083: a block's page protected in the VM; the PROCESS_VA plant observable; the runtime stack's invariant; layout constants in the contract; the counts on the record (moved to 085).
- **Rubber Duck Pass:**
  - *Sequence narration:* code committed (eb4dcd1c, then 48a1baf3 with the user's decision); classes, families and a judged frontier re-run on 48a1baf3 (456 of 458, 68, 87,731 VM compiles equal, 0 short; 527 records clean); a timed frontier (2.7 s for 10^9 instructions); contract 0.84.0.
  - *Observation 1 (blind guard, of the work):* a check on SP stopped a correct run: SP cannot tell an abandoned frame from a live one. Disposition: resolved-here (the check removed, the invariant stated); KIT_DIARY #61.
  - *Observation 2 (vocabulary gap):* MEASUREMENT_RECORDED has no metric for a count of blocks. Disposition: deferred to Sprint 085 (vocabulary 0.8 with the measure it serves).
  - *Observation 3 (unmeasured difference):* the judged and timed runs compiled 87,938 and 87,935 blocks with the same outcome. Disposition: recorded, cause not measured.
  - *Observation 4 (process):* a sweep started on an uncommitted BLACKBOARD, and 083's frontier side files were not committed. Disposition: resolved-here.
- **Closed:** CI run 38833133755 (success on b350f53a).

### Sprint 083 (2026-10-15, closed)
- **Scope:** a dispatch miss compiled inside the VM: `block_compiler.c` (`w2m_vm_compile`) over the code-page set, the runtime image above the stack, the host replaying the VM's log at every exit, `VM_COMPILE_JUDGE`; the review of 082's three asks.
- **Rubber Duck Pass:**
  - *Sequence narration:* code committed (1db10a26); 458 classes and 68 families re-run on it (456 accepted, FCOS and FSIN as before; 527 records clean); a judged frontier wrote its record (stop max_steps, 84,046 VM compiles equal, 0 short) and a timed one (3.8 s for 10^9 instructions); scratch runs with the compiler off and on gave the same calls and stop.
  - *Observation 1 (missing pair, of the work):* the stencil table holds the runtime stack's top as a constant, a layout fact no generator input names. Disposition: recorded (Decision 2026-10-15, the image above the stack; an assert pins the address).
  - *Observation 2 (blind judge, of the judge):* my VM judge compared a cut block with the scan of its whole extent and stopped a test run on a defect that was its own. Disposition: resolved-here (the log gives the scan only for an uncut block).
  - *Observation 3 (blind plant check, of the judge):* a plant on the scan's PROCESS_VA check cannot be seen by any judge. Disposition: recorded in the test file and the card (the second such case after KIT_DIARY #60).
  - *Observation 4 (residual, of the work):* 3,930 of 4,106 host misses are blocks on pages no block yet covers, the C's precondition. Disposition: open, for the next sprint's play-stretch measure (Decision 2026-10-15's revisit).
- **Closed:** CI run 38829826305 (success on 7d904142).

### Sprint 082 (2026-10-15, closed)
- **Scope:** the successor scan in C (`vmc_live_in`, `vmc_live_out`), the first link of putting the compiler in the VM: two form facts (the first branch's kind, its constant target's value position) and two judges against `VM._flags_live_in` and `_flags_live_out`.
- **Rubber Duck Pass:**
  - *Sequence narration:* the table regenerated on the new facts (0 disagreeing); code committed (d7a81658); `stencil_judge.py --judges scan,scan-spec` wrote one CANDIDATE_GENERATED, two COMPARISON_COMPLETED (conforms) and CANDIDATE_ACCEPTED (stencil-judge-1792108383330932000): 1,386,082 and 174,796 scans equal, 0 short, 0 differing; no frontier run, no sweep (vm_backend.py untouched).
  - *Observation 1 (blind plant check, of the judge):* the card's named plant (the NZCV kind from the instruction itself) passed both judges; measured, it can change no scan, since every setter writes the flags its Jcc tests. Disposition: resolved-here (a plant that steps past a jump fails both); KIT_DIARY #60.
  - *Observation 2 (residual, of the work):* where an instruction the scan reads has no form, the C's flags are wider than the host's (112 of the record's scans, 29,975 of the spec layouts'): correct code, other words than the host would compile. Disposition: open, for Sprint 083's VM judge, which compiles with the flags the C found and counts how often they differ from the host's.
- **Closed:** CI run 38826924336 (success on 2e45fd50).

### Sprint 081 (2026-10-15, closed: CI run 38810199578 success on 1753eca3)
- **Scope:** the reviewer's small review of Sprint 080: the acceptance count, the compiler's contract, the residuals named.
- **Rubber Duck Pass:**
  - *Sequence narration:* code and tests only; no run, no frontier, no sweep (no family or class identity touched).
  - *Observation 1 (payload anomaly, of the record):* every session written through `one_step_run` reported accepted_count 0, against the tag's note, including stencil-judge-1792098411625593000. Disposition: resolved-here; that record stays as written (records are not rewritten).
  - *Observation 2 (missing pair, of the work):* the judge cited a contract with no file. Disposition: resolved-here (`exact_words`, which requires a judge from outside the target's sources).
  - *Observation 3 (residual, of the work):* alternatives are sought at the chosen position only, and agreement on the probe values counts as equality. Disposition: open, named in `_hole` and the contract (Decisions are not edited).
  - *Observation 4 (trade-off, of the work):* `rules.COMPILER` takes all of vm_backend.py into every class identity. Disposition: kept; narrowed to compile_insn's sources if sweeps start costing.
- **Closed:** CI run 38810199578 (success on 1753eca3).

### Sprint 080 (2026-10-15, closed: CI run 38807493394 success on 0e760a2c)
- **Scope:** the large review of Sprints 073-079 (no blocking defect; H1-H3, M1-M4, L1-L2) and its experiments E1 and E2.
- **Rubber Duck Pass:**
  - *Sequence narration:* code committed (7c089ed8); every class re-run on it, 456 of 458 accepted, 458 stamped clean; the table derived on it from the committed portable record; the C table regenerated and checked byte for byte; `scripts/stencil_judge.py` wrote its record (four judges, CANDIDATE_ACCEPTED, 0 differing in each); no frontier run (the C is not in the VM).
  - *Observation 1 (blind judge, of the judge):* the guard rule of 079 covered one inference failure; measuring ambiguity found register holes fitting `min(v, 8)` and value holes fitting `v` and `low 5 bits`. Disposition: resolved-here (every unvaried value guarded; register holes through the register map; value alternatives checked at emit time); a plant that removes the guards fails the spec-row judge.
  - *Observation 2 (silent gate, of the work):* the class identity left out the compiler every verdict runs through, so Sprint 078's refactor was never re-judged by a class run. Disposition: resolved-here (`rules.COMPILER`; 458 re-run).
  - *Observation 3 (wrong destination, of the work):* the first class sweep wrote to `runs/`, class_run's default, and the index check caught it. Disposition: resolved-here (re-run with `--runs store/runs`).
  - *Observation 4 (provenance, of the record):* the table regenerated only from ignored pickles; the judges left no record. Disposition: resolved-here (store/stencil, `--check` in CI and gates.sh, the judge record); the record's own producing commit is inferred from file times (4394114c), not stamped.
  - *Observation 5 (process, of the work):* gates had been run one by one all epic while `scripts/gates.sh` runs them in order. Disposition: resolved-here.
- **Closed:** CI run 38807493394 (success on 0e760a2c).

### Sprint 079 (2026-10-15, closed: CI run 38803778005 success on c95b650d)
- **Scope:** a red-team pass over Sprints 076-078 (user request): guards on values the derivation never varied, canonical flag arguments in the key, a held-out judge over the class case sets, the table's inputs kept.
- **Rubber Duck Pass:**
  - *Sequence narration:* `stencil_table.py` rebuilt the table (21,182 canonical keys, none split; 3,365 stencils guarded); `gen_stencil_table.py` regenerated the C data; the held-out judge compiled the 250,840 class cases at two flag contexts on the old table and the new; the record judges reran; no frontier run, no family change (no host file touched).
  - *Observation 1 (blind judge, of the judge):* Sprints 077 and 078 judged the C over the record the table came from and reported 0 differing; the class cases found 144 wrong compiles on 078's table (seven SSE shift classes: a register, and in other keys a return address and a branch target, taken for constants). Disposition: resolved-here (guards; 0 differing of 31,311 on the new table); KIT_DIARY #59. Decision 2026-10-14 (copy-and-patch) had required the class runs as a judge; 077 and 078 left that half out.
  - *Observation 2 (missing pair, of the work):* flag arguments the instruction cannot read split keys: 38,154 became 21,182, and blocks the C could not compile fell from 17,842 to 5,661. Disposition: resolved-here.
  - *Observation 3 (residual, of the work):* 94% of held-out compiles have no stencil; the table covers forms the game record reached. Disposition: open; measured against the VM's own host-compile rate once the C runs there (Decision 2026-10-14's keys-from-runs clause is the revisit point).
  - *Observation 4 (provenance, of the record):* the table's inputs lived in /private/tmp. Disposition: resolved-here (`store/stencil-inputs/`, git-ignored, sha256 in the card).
- **Closed:** CI run 38803778005 (success on c95b650d). Large review of 073-079 requested from the reviewer.

### Sprint 078 (2026-10-15, closed: CI run 38796163028 success on 01cd4765)
- **Scope:** the block in C (`vmc_block`): extent, flag liveness inside the block, prologue, exits, the budget; the table keyed by form with each form's facts; `vm_backend.assemble_block` taken out of `Monitor._compile`.
- **Rubber Duck Pass:**
  - *Sequence narration:* `gen_stencil_table.py` re-keyed the table (17,328 forms, facts from every recorded instruction, 0 disagreeing); a scratch driver compiled 1,177,340 blocks made from the record in both compilers; every family re-run on 3f28863d to CANDIDATE_ACCEPTED (68); no frontier run (the C is not in the VM). 1,163,021 blocks equal in words and marks, 0 differing; C 1.3 µs a block natively against 78.9 µs.
  - *Observation 1 (missing pair, of the work):* 17,842 blocks reached a flag combination the record never compiled (14,319 at the first instruction, 3,523 later). Disposition: open, for the sprint that runs the C in the VM: measure the rate on the run's own block starts; a stencil for a superset of live flags is correct code but not `compile_insn`'s words.
  - *Observation 2 (blind plant check, of the judge):* three block faults planted one at a time (the liveness walk's NZCV kind, the budget's B.MI, `nzcv_out`) each failed the test.
  - *Observation 3 (silent gate, of the work):* the background suite reported exit 0 with one test failed, since the job's last command was an `echo`. Disposition: resolved-here (the status read from pytest's own line; the stale families re-run before the push).
  - *Observation 4 (unmeasured cause, of the record):* the card first named mid-block starts as the cause of Observation 1 without measuring it. Disposition: resolved-here (the card states only the measured status).
- **Closed:** CI run 38796163028 (success on 01cd4765).

### Sprint 077 (2026-10-14, closed: CI run 38691728944 success on ae831e71)
- **Scope:** the compiler in C for one instruction: the key from the decoder, Sprint 076's table as C data, the emitter (Decision 2026-10-14).
- **Rubber Duck Pass:**
  - *Sequence narration:* `scripts/gen_stencil_table.py` over Sprint 076's scratch table; a scratch driver compiled the record's 1,186,511 distinct calls in both compilers at random bases and exits; no run, runtime or Python compiler change, so no frontier record and no sweep. C key and words equal in all, 0 with no stencil; 208 ns per instruction natively against `compile_insn`'s 12.5 µs.
  - *Observation 1 (missing pair, of the judge):* the record reached one stencil of each of the 71 refined keys, so neither the record judge nor a per-key sample tests the other 75. Disposition: resolved-here (the samples add an instance made for each; 38,229 instructions in the CI test).
  - *Observation 2 (blind plant check, of the judge):* four faults planted one at a time (exit offset, a value-class bit, the implicit mask, the `-v` expression) each failed the test.
  - *Observation 3 (silent gate, of the work):* the CI wait loop passed `gh run list --commit` a short hash, found no run and reported exit 1 without watching anything. Disposition: resolved-here (the full hash).
  - *Observation 4 (residual, of the work):* Sprint 076's 506 unvaried values stay open: the record judge agrees, but class runs have not compiled through the C. Disposition: open, for the sprint that puts the C in the VM.
- **Closed:** CI run 38691728944 (success on ae831e71).

### Sprint 076 (2026-10-14, closed: CI run 38683458204 success on 1bb36655)
- **Scope:** the stencil table from instances made per key (Decision 2026-10-14, the stencil key): each key's encoder sequence, each argument a constant or one expression of a decoded value.
- **Rubber Duck Pass:**
  - *Sequence narration:* `scripts/stencil_table.py` over Sprint 074's scratch record; no run, compiler or runtime change, so no frontier record and no sweep. Final: 38,154 keys, a stencil each (71 refined by a value feature); 988,840 fresh instances and the record's 1,186,511 compile calls rebuilt from the table, 0 differing.
  - *Observation 1 (blind judge, of the judge):* the first judge drew its instances from the same generator as the derivation and passed 98.2% while the record differed on 133,748 calls. Disposition: resolved-here (the key's recorded calls split between derivation and judge; eleven causes fixed in the instance maker); KIT_DIARY #58.
  - *Observation 2 (revisit condition, of the work):* 58 keys gave two sequences over their instances (an SSE count at or past the lane width; CMPPS's predicate, a string argument the first signature left out). Disposition: resolved-here (a key is refined by one value feature, from a per-key entry); the Decision's form stands.
  - *Observation 3 (residual, of the work):* 506 values never varied in their keys' instances other than 0, 1 and -1 (TEST, CMP, ADD and OR immediates, Jcc displacements), where a constant may be an unshown hole. Disposition: open; the C compiler's differential over the record and the class runs is the next judge.
- **Closed:** CI run 38683458204 (success on 1bb36655).

### Sprint 075 (2026-10-14, closed: CI run 38679290552 success on 982bb22e)
- **Scope:** the decoder in C for the compiler inside the VM: `x86dec._decode` and `x86ops.operands` step for step over tables `gen_decoder.py` writes from the spec.
- **Rubber Duck Pass:**
  - *Sequence narration:* no run, compiler or runtime change, so no frontier record and no sweep; the differential compared 84,409 strings (every row, every outcome, truncations) and the record's 1,183,579 instructions, 0 differing.
  - *Observation 1 (payload anomaly, of the work):* the first run differed on 1,147 VEX, EVEX and XOP strings: their pp field orders F3 before F2, unlike the tables' prefix codes. Disposition: resolved-here (a mapping, as x86dec's `VEX_PFX`).
  - *Observation 2 (blind plant check, of the judge):* three faults planted one at a time were caught (754, 9 and 343 differing strings).
- **Closed:** CI run 38679290552 (success on 982bb22e).

### Sprint 074 (2026-10-14, closed: CI run 38678470845 success on 19f388a5)
- **Scope:** the stencil key from the decoder: does a key the compiler inside the VM can compute from the instruction's bytes pick one encoder sequence, and is every hole a function of decoded fields?
- **Rubber Duck Pass:**
  - *Sequence narration:* a scratch cold batch run recorded each compile call with its bytes (the same 1,186,511 distinct calls as Sprint 071's run); `scripts/stencil_census.py --decoder-key` over it; no frontier record, no sweep (no runtime or compiler change).
  - *Observation 1 (missing pair, of the work):* the first decoder key split one MUL row in two, because the decoded operands leave out MUL's implicit RAX and RDX. Disposition: resolved-here (each row's implicit registers enter the key: 38,154 keys, each one sequence).
  - *Observation 2 (payload anomaly, of the judge):* the census named holes by a vote per instruction, which can name a hole by coincidence. Disposition: resolved-here (one expression over every instruction of the form; 0 `other`); and the record leaves 17,276 keys' holes unshown, so Decision 2026-10-14 (the stencil key) takes holes from instances made per key.
  - *Observation 3 (silent gate, of the work):* a mypy error went into 7b185c3e because the gate's output was piped through `tail`, whose exit status is 0. Disposition: resolved-here (19f388a5; gates run unpiped before a commit).
- **Closed:** CI run 38678470845 (success on 19f388a5).

### Sprint 073 (2026-10-14, closed: CI run 38677583192 success on 4394114c)
- **Scope:** the arm64 encoders in C for the compiler inside the VM (Decision 2026-10-14's second build step), judged against `a64_encoder.py`.
- **Rubber Duck Pass:**
  - *Sequence narration:* no run, compiler or runtime change, so no frontier record and no sweep; the differential (`tests/test_a64_encoder_c.py`) compared 1,066,482 words and 953,795 rejections over 138 functions, 0 differing; the 394,453 distinct encoder calls of Sprint 071's record gave the same words in C, 0 differing.
  - *Observation 1 (blind plant check, of the judge):* a pass on the first run, in 3.3 s, could have meant a test that compared nothing. Disposition: resolved-here (three faults planted in a scratch copy, each caught: 8,413, 21 and 7,399 differing calls).
- **Closed:** CI run 38677583192 (success on 4394114c).

### Sprint 072 (2026-10-14, closed: CI run 38670636259 success on b7b20617)
- **Scope:** every compiled word from an encoder call (Decision 2026-10-14's first build step): six inline sites of `vm_backend.py` call new encoder forms, the words unchanged.
- **Rubber Duck Pass:**
  - *Sequence narration:* family sweeps of 68 families, each CANDIDATE_ACCEPTED (index b7b20617: 456 of 458 classes); no frontier record (the compiled words are unchanged, shown over the 1,186,511 recorded calls).
  - *Observation 1 (payload anomaly, of the judge):* the CI watch took the branch's latest run, the previous push's. Disposition: resolved-here (the run found by commit; 069's and 070's cited runs checked on their commits).
- **Closed:** CI run 38670636259 (success on b7b20617).

### Sprint 071 (2026-10-14, closed: a measurement; no records change)
- **Scope:** the stencil census for recentering step 6: does the compiler's output factor into copy-and-patch stencils?
- **Rubber Duck Pass:**
  - *Sequence narration:* a scratch batch run to 100 billion instructions, cold, its compile calls recorded; `scripts/stencil_census.py` over them; no frontier record, no sweep (no runtime or compiler change).
  - *Observation 1 (payload anomaly, of the judge):* the census's first two passes were wrong by its own instrument (a compile base out of branch range; 32-bit encoder words read as raw), shown by its rebuild check. Disposition: resolved-here (the rebuild exact for 1,313,377 of 1,313,377).
  - *Observation 2 (missing pair, of the work):* the Decision's grounds name two of the three rules the 70 `other` holes follow; all 70 checked afterwards (165 of 200 values 16 times an XMM number, 17 a width less a shift count, 18 a shift count less 1). Disposition: recorded in the card.
  - *Observation 3 (scope, of the work):* 394,155 words are built inline in `vm_backend.py`, outside the encoder. Disposition: open; the next sprint moves them into encoder functions with the words unchanged.
- **Closed:** census and Decision committed (7acafc15).

### Sprint 070 (2026-10-14, closed: CI run 38662524546 success on bc6c956c)
- **Scope:** the deadline warp inside the VM (recentering step 5): a wait that leaves no thread ready warps the instruction clock in the guest runtime by the host's rule.
- **Rubber Duck Pass:**
  - *Sequence narration:* family sweeps of 68 families, each CANDIDATE_ACCEPTED (index bc6c956c: 456 of 458 classes); frontier-643a6283510c-1792025575233476000: EXECUTION_STARTED, its stop, TRACE_INGESTED, COMPARISON_COMPLETED `conforms` (168 of 168 lines, 0 divergences), SESSION_COMPLETED.
  - *Observation 1 (blind plant, of the judge):* the plant that left the clock unwarped hung the probe rather than changing a census. Disposition: resolved-here (a reach check against the host's warp, per KIT_DIARY #56).
  - *Observation 2 (payload anomaly, of the judge):* the first census compared runs of unequal length (the frontier's stop is checked on a 30-second wall-clock tick, so a faster run goes further), and read as a cold-against-warm difference in interleaving. Disposition: resolved-here (d6edf5ca; the census at 100 billion instructions: identical thread censuses in all six runs; session 56.7-58.6 s with the warp in the host, 50.5-51.6 s in the VM; Sprint 069's table corrected in its card; KIT_DIARY #57).
- **Closed:** CI run 38662524546 (success on bc6c956c).

### Sprint 069 (2026-10-14, closed: CI run 38654917874 success on 773579d1)
- **Scope:** thread switches inside the VM (recentering step 5): the run table, the switch and the signal in the guest runtime by the host's rule.
- **Rubber Duck Pass:**
  - *Sequence narration:* family sweeps of 68 families, each CANDIDATE_ACCEPTED (indexes de414a3b, 773579d1: 456 of 458 classes); frontier-643a6283510c-1792020875595657000: EXECUTION_STARTED, its stop, TRACE_INGESTED, COMPARISON_COMPLETED `conforms` (168 of 168 lines, 0 divergences), SESSION_COMPLETED.
  - *Observation 1 (missing pair, of the judge):* the on-against-off equivalence passed while the new path never ran on the batch run (0 switches in the VM: a second process turned it off). Disposition: resolved-here (d33e4fb8; the progress line's switch counter; KIT_DIARY #56).
  - *Observation 2 (payload anomaly, of the work):* each slice switch had counted its block twice. Disposition: resolved-here (item D).
  - *Observation 3 (blind plant, of the judge):* SwitchToThread's answer inverted changes no row; no threaded probe reads it. Disposition: recorded in the card; a probe row for SwitchToThread's answer is open.
- **Closed:** CI run 38654917874 (success on 773579d1).

### Sprint 068 (2026-10-14, closed: CI run 38632198138 success on 4e98c57c)
- **Scope:** speed toward the playable slice: the time census's exits removed one cause at a time, each measured, the frontier conforming.
- **Rubber Duck Pass:**
  - *Sequence narration:* family sweeps of 68 families, each CANDIDATE_ACCEPTED, on every runtime change (last index a350c9f4: 456 of 458 classes, 68 families); the batch run to its own stop, frontier-643a6283510c-1791973386975499000 on d33b5bd7: EXECUTION_STARTED, its stop, TRACE_INGESTED, COMPARISON_COMPLETED `conforms` (unity.log 168 of 168 lines, 0 divergences), SESSION_COMPLETED.
  - *Observation 1 (payload anomaly, of the judge):* the instruction count dropped every slice the stub refilled in the VM and counted a model step's unrun block instructions again, so every rate since 76111810 was short (246 against 694 million a second). Disposition: resolved-here (3d052448; tests/test_instruction_count.py).
  - *Observation 2 (missing pair, of the judge):* the plants on the wake keys changed no family row; no probe woke a wait on several objects by a signal. Disposition: resolved-here (wait_handoff 0.2.0, row event_auto.multiple_waiter, which found a hand-over defect, fixed).
  - *Observation 3 (order violation, of the work):* mypy run through a pipe exited 0 on a type error that reached a push (2e9f6f5d), and background chains ran untracked. Disposition: resolved-here (scripts/gates.sh; tracked jobs only).
  - *Observation 4 (payload anomaly, of the work):* a thread switch at a slice's end resumes at the block's entry, whose SUBS counts the block a second time; found designing Sprint 069. Disposition: carried (Sprint 069 item D).
- **Closed:** CI run 38632198138 (success on 4e98c57c).

### Sprint 066 (2026-10-13, closed: CI run 38506623160 success on 7114da6e)
- **Scope:** thread stacks as Windows lays them out, a running thread's context, and the divergences threads sharing one vCPU left.
- **Observed:** the frontier conforms against its reference (0 defects; HeapReAlloc 3268 scheduling); thread_stacks (44 rows) and running_thread_context (4 rows) conform; all 68 families accepted (index 431bffa5).
- **Carried:** the main thread's stack still committed whole; InterlockedPopEntrySList against the laptop's runs is a timing-dependent value the classifier counts as a value (Sprint 067).

### Sprint 065 (2026-10-12, closed partial: CI run 38327467991 success on 91a8e77b)
- **Scope:** x87 and the MMX-register forms; past the FLD and seq 3426; the red team's findings; a clock the run controls.
- **Rubber Duck Pass:**
  - *Sequence narration:* class runs of the x87 and MMX classes to CANDIDATE_ACCEPTED (FSIN and FCOS discrepancy, pinned); family runs of 66 families, new: display 0.2.0, wait_handoff, code_rewrite, each CANDIDATE_ACCEPTED with plants caught; frontiers each with EXECUTION_STARTED, EXECUTION_COMPLETED `killed` at the reach rule, TRACE_INGESTED, COMPARISON_COMPLETED `discrepancy` (14 over 3,443) and SESSION_COMPLETED.
  - *Observation 1 (missing pair, of the judge):* the comparison never reported a Windows call w2m does not make. Disposition: resolved-here (`missing`).
  - *Observation 2 (payload anomaly, of the work):* a frontier stopped by the harness was recorded `faulted`, and run_frontier exited 0 on divergences. Disposition: resolved-here (`killed`; the exit status is the verdict).
  - *Observation 3 (order violation, of the work):* a run went two hours past its reference with no output; the clock experiment slowed 100 times stalled at 79 lines. Disposition: resolved-here (progress lines, the reach rule, the instruction clock).
  - *Observation 4 (vocabulary gap, checked):* PROBE_AUTHORED is in the card's Emits list and no 065 probe emitted it. Disposition: recorded; the probes' records carry their runs, and PROBE_AUTHORED's emission moves with 066's harness work.
- **Closed:** CI run 38327467991 (success on 91a8e77b), partial.

### Sprint 064 (2026-10-12, closed partial: CI run 38219168662 success on cc88048a)
- **Scope:** speed: the exits counted, their causes removed, the hot model-step classes native.
- **Rubber Duck Pass:**
  - *Sequence narration:* frontier runs of 1, 20 and 200 billion steps, each with EXECUTION_STARTED, its stop, TRACE_INGESTED, COMPARISON_COMPLETED `conforms` (2,393 to 2,508 calls) and SESSION_COMPLETED; class runs of the lowered classes to CANDIDATE_ACCEPTED, with plants rejected; two full class runs (436) and two family sweeps (60); a reference game run with thread names.
  - *Observation 1 (payload anomaly, of the judge):* the trace dropped 744,597 calls unseen, and the comparison still said `conforms`. Disposition: resolved-here (slot reuse; a lossy trace makes the comparison inconclusive).
  - *Observation 2 (missing pair, of the judge):* faster runs reordered threads, and pairing by agreement mispaired workers. Disposition: resolved-here (thread names from both sides).
  - *Observation 3 (order violation, of the work):* a plant restored with `git checkout` discarded uncommitted edits to the same file. Disposition: resolved-here (reapplied; plants are reverted by the reverse edit).
  - *Observation 4 (vocabulary gap, checked):* the frontier's stop at a lift refusal is P18's case (TRANSLATION_REFUSED, proposed); still reported as EXECUTION_COMPLETED `faulted`. Disposition: recorded; P18 stands for the next vocabulary version.
- **Closed:** CI run 38219168662 (success on cc88048a), partial.

### Sprint 063 (2026-10-11, closed partial: CI run 38215049553 success on 71c0d80a)
- **Scope:** kernel.appcore and the 20 unserved functions to seq 3426, five groups in parallel worktrees.
- **Rubber Duck Pass:**
  - *Sequence narration:* the frontier's SERVICE_UNIMPLEMENTED at EnumProcessModules; five groups' PROBE_AUTHORED, item runs to CANDIDATE_ACCEPTED and plants, each in its own worktree; five merges, each followed by its families' item runs; two full sweeps (60 accepted); the frontier to its step limit, TRACE_INGESTED, COMPARISON_COMPLETED `conforms`.
  - *Observation 1 (missing pair, of the harness):* the frontier's unity.log was never kept (copied after the drive was removed). Disposition: resolved-here.
  - *Observation 2 (order violation, of the parallel work):* three of five worktrees began at main, not the sprint's branch; the agents reset them onto sprint-063 before working. Disposition: recorded; a worktree's base is named in the next fan-out's instructions.
  - *Observation 3 (missing pair, of the criterion):* the game is slow, not stopped: 4 billion instructions stay in a deserialization loop. Disposition: deferred to 064 (Decision above).
  - *Observation 4 (vocabulary gap, checked):* the step limit is reported as EXECUTION_COMPLETED `faulted` with the limit in `unobserved`'s place in the stop text; P18's proposal covers a lift refusal, not a step limit. Disposition: recorded; a limit is the harness's, and EXECUTION_COMPLETED with `killed` may fit better — checked in 064.
- **Closed:** CI run 38215049553 (success on 71c0d80a), partial.

### Sprint 062 (2026-10-11, closed: CI run 38209558822 success on 61690d7d)
- **Scope:** the frontier's reruns: a profile, the quadratic sort, the snapshot and translation cache decided on the measurement.
- **Rubber Duck Pass:**
  - *Sequence narration:* a profiled scratch run (154 s, 98 in `list.sort`); the fix; the frontier's SERVICE_UNIMPLEMENTED at 57,445,097 instructions, TRACE_INGESTED (2,711 lines), COMPARISON_COMPLETED `conforms`, SESSION_COMPLETED at 33.1 s; a second profile.
  - *Observation 1 (payload anomaly, of the record):* two runs' trace digests differ although their calls agree; each guest trace names its own execution id, and registers past a function's parameters hold leftovers. Disposition: recorded (the comparator, run between the two streams, finds 0 divergences over 2,685 calls).
  - *Observation 2 (vocabulary gap, checked):* none.
- **Closed:** CI run 38209558822 (success on 61690d7d).

### Sprint 061 (2026-10-11, closed: CI run 38208477502 success on aee379d6)
- **Scope:** the 55 divergences of Sprint 060 closed by matching the reference (Decision 2026-10-11, user).
- **Rubber Duck Pass:**
  - *Sequence narration:* three probes on two runners each (time_and_error_mode, threads twice, processor_topology); their families' item runs to CANDIDATE_ACCEPTED with new plants; a reference game run with the game's resources, recorded as `game-run-38206263976`; the frontier's SERVICE_UNIMPLEMENTED, TRACE_INGESTED, COMPARISON_COMPLETED `conforms`; two plant runs, each caught; 57 item runs and the file run.
  - *Observation 1 (payload anomaly, of the reference):* LoadIconA's failure on Windows was the launcher's missing icon. Disposition: resolved-here (copy_resources.c; a new reference).
  - *Observation 2 (payload anomaly, of the judge):* the capture's count over all threads makes each thread's calls a prefix, which pairing by first call did not respect. Disposition: resolved-here (32 per thread; pairing by agreement, tested to keep a fault on every worker).
  - *Observation 3 (missing pair, of the probe):* reading FLS in the DLL's attach overwrote the thread's first last error, a row; caught by the row's change between rounds. Disposition: resolved-here (saved and restored).
  - *Observation 4 (vocabulary gap, checked):* none.
- **Closed:** CI run 38208477502 (success on aee379d6).

### Sprint 060 (2026-10-11, closed: CI run 38205517561 success on 104f1db8)
- **Scope:** the frontier's records on the record, and the game's own calls on Windows as a judge of w2m's (the ground-up review's G2, S4 and the missed finding).
- **Rubber Duck Pass:**
  - *Sequence narration:* the frontier's SESSION_INIT, EXECUTION_STARTED, SERVICE_UNIMPLEMENTED (the appmodel API set, 57,409,405 instructions), TRACE_INGESTED (1,769 lines, 22 threads), COMPARISON_COMPLETED against `game-run-38203364352` (55 divergences), SESSION_COMPLETED; the reference's SESSION_INIT, EXECUTION_STARTED (windows_reference), EXECUTION_COMPLETED (timed_out at 181 s); two plant runs, each caught at its call; 57 family item runs and their plants; the file run.
  - *Observation 1 (payload anomaly, of the judge):* the first comparison's 790 divergences were mostly the capture's conditions and one parser defect (the last error is decimal). Disposition: resolved-here, each cause traced before a relation changed; KIT_DIARY #50.
  - *Observation 2 (payload anomaly, of the reference):* the runner's paths and profile made path-derived values differ. Disposition: resolved-here for the install directory and temp (a new reference run); recorded for the profile (the laptop).
  - *Observation 3 (missing pair, of the product):* FLS values shared by every thread since Sprint 051, with every family accepted. Disposition: deferred to plan row 062 with the four other defects (Decision above).
  - *Observation 4 (vocabulary gap, checked):* the call stream needed no tag (GUEST_CALL_RETURNED, TRACE_INGESTED); the lift refusal's tag is proposed (P18), not used.
- **Closed:** CI run 38205517561 (success on 104f1db8).

### Sprint 059 (2026-10-11, closed: CI run 38195353233 success on 4a27afe9)
- **Scope:** PEB.ProcessParameters, which UnityPlayer's C runtime reads with no call; the 058 gap.
- **Rubber Duck Pass:**
  - *Sequence narration:* a scan of the game's modules for direct reads (four, all of Flags); the probe on two runners; the item run SESSION_INIT, CANDIDATE_GENERATED, two COMPARISON_COMPLETED, CANDIDATE_ACCEPTED, SESSION_COMPLETED; six plants; every family re-run; the frontier's EXECUTION_STARTED, then EXECUTION_COMPLETED faulted at the appmodel API set after 57,409,401 instructions, no fault dispatched.
  - *Observation 1 (missing pair, of the host):* the host wrote an empty environment as one NUL and environment.c counts two; the new EnvironmentSize row exposed it. Disposition: resolved-here.
  - *Observation 2 (payload anomaly, of the reference):* the parent's window values, title and environment version are the runner's start, not Windows's rule. Disposition: resolved-here (kept as relations, the reason in the derivation).
  - *Observation 3 (vocabulary gap, checked):* the frontier's stop is an API set with no observed host, reported as EXECUTION_COMPLETED faulted with the stop in `unobserved` (Sprint 049's form). Disposition: recorded, no new tag.

### Sprint 058 (2026-10-11, closed: CI run 38190593421 success on 56ffb04b)
- **Scope:** the seven defects a full read of Sprint 055's code found, each with a failing check first; the processes probe's new rows.
- **Rubber Duck Pass:**
  - *Sequence narration:* the checks run on d5ecffe8 and fail; the probe on two runners twice (the second round asks the large write again with 4000 bytes left); the processes item run SESSION_INIT, CANDIDATE_GENERATED, two COMPARISON_COMPLETED, CANDIDATE_ACCEPTED, SESSION_COMPLETED; its plants; 56 item runs to CANDIDATE_ACCEPTED, twice; the frontier's EXECUTION_STARTED, then SERVICE_UNIMPLEMENTED for GetCurrentDirectoryA after 57,411,545 instructions.
  - *Observation 1 (missing pair, of the judge):* the two plants on kernel.py changed nothing: the plant patched the module, and the run used the names family_run had imported from it. Disposition: resolved-here (family_run rebound; KIT_DIARY #49).
  - *Observation 2 (payload anomaly, of the Machine):* a deadlock raised out of Machine.run and the run's rows were lost. Disposition: resolved-here (a stop, tested).
  - *Observation 3 (order violation, of the reaches):* two reaches were named before the rows that their plants now also change existed. Disposition: resolved-here (amended with the reason, as in Sprint 048).
  - *Observation 4 (vocabulary gap, checked):* none.

### Sprint 057 (2026-10-11, closed: CI run 38188148154 success on f7b474c7)
- **Scope:** each family's candidate derived from the runtime's objects, its probe's imports and the requests its files make; every process a run starts checked against the candidate's build.
- **Rubber Duck Pass:**
  - *Sequence narration:* the objects read; 56 item runs SESSION_INIT, CANDIDATE_GENERATED, COMPARISON_COMPLETED per runner binary, CANDIDATE_ACCEPTED, SESSION_COMPLETED on the derived candidates; every family's plants run; five families' plant runs stopped on a lost anchor, re-anchored and run again.
  - *Observation 1 (missing pair, of the judge):* six plants named text no longer in their files, and no gate ran plants. Disposition: resolved-here (re-anchored; a gate checks every anchor; KIT_DIARY #48).
  - *Observation 2 (payload anomaly, of the reader):* `w2m_threads_blocked` is a common symbol, undefined with a size in its object, and the first derivation found no file defining it. Disposition: resolved-here (a common symbol counts as a definition, tested).
  - *Observation 3 (vocabulary gap, checked):* a run whose process ran another runtime stops with a named stop; no tag needed. Disposition: recorded (vocabulary 0.7).

### Sprint 056 (2026-10-11, closed: CI run 38183215347 success on 79f13466)
- **Scope:** the eight verified findings of the review of Sprint 054, the audit for their patterns, the claims.
- **Rubber Duck Pass:**
  - *Sequence narration:* the native checks fail on 243b8ad9; the registry, file_search and windows probes run on two runners twice (the class-name boundary asked again at 254 and 255); fixes; the three families' item runs SESSION_INIT, CANDIDATE_GENERATED, two COMPARISON_COMPLETED, CANDIDATE_ACCEPTED, SESSION_COMPLETED; 28 families re-run to CANDIDATE_ACCEPTED.
  - *Observation 1 (order violation, of the judge):* one native check failed first for a wrong expectation, not the defect. Disposition: resolved-here (each check's failure read before any fix).
  - *Observation 2 (missing pair, of the documentation):* Microsoft Learn's class-name maximum of 256 is off by one against Windows (256 answers 87). Disposition: resolved-here (the rows decide; the comment cites both).
  - *Observation 3 (vocabulary gap, checked):* none.
  - *Observation 4 (timing surprise, of CI):* the first closing commit (62dc17ce) failed CI run 38182897398: Linux's AddressSanitizer runs LeakSanitizer, which macOS's does not, and it reported the harnesses' own buffers and exited before stdout flushed. Disposition: resolved-here (leaks not judged; the guest heap is w2m's own).

### Sprint 055 (2026-10-11, closed: CI run 38180806466 success on 3e0dea6d)
- **Scope:** captures with names and the handler's calls; the processes and winrt families; a second process as a second VM.
- **Rubber Duck Pass:**
  - *Sequence narration:* game run 38176899195 (DISTRIBUTION and EVIDENCE as the capture's ingest gives them); the processes and winrt probes on two runners; their item runs SESSION_INIT, CANDIDATE_GENERATED, two COMPARISON_COMPLETED, CANDIDATE_ACCEPTED, SESSION_COMPLETED; every family re-run on a Machine to CANDIDATE_ACCEPTED (time_and_error_mode at 0.2.0); the frontier's EXECUTION_STARTED, then SERVICE_UNIMPLEMENTED for GetCurrentDirectoryA after 57,411,545 instructions, with one fault logged.
  - *Observation 1 (missing pair, of the judge):* the processes plants first changed no child row: the child VM ran the unplanted runtime. Disposition: resolved-here (the spawner takes the runtime sources; KIT_DIARY #46).
  - *Observation 2 (order violation, of the record):* a wait in progress logs no call, so the handler's counts could not show its main thread waiting. Disposition: resolved-here (the frontier prints each process's threads and the call each is in).
  - *Observation 3 (vocabulary gap, checked):* a child process's stop has no tag of its own; the frontier's SERVICE_UNIMPLEMENTED names it with the child's id in the stop. Disposition: recorded, no new tag (vocabulary 0.7).

### Sprint 054 (2026-10-11, closed: CI run 38175121068 success on 2b02046a)
- **Scope:** the version, display, windows, time, registry and file_search families; amended before the work to give the frontier the batch run's arguments, fix VirtualAlloc(NULL, MEM_COMMIT) and stop at the crash handler's start.
- **Rubber Duck Pass:**
  - *Sequence narration:* seven probes on two runners each (version, time, display and windows rerun after build or row fixes); each family's item run SESSION_INIT, CANDIDATE_GENERATED, two COMPARISON_COMPLETED, CANDIDATE_ACCEPTED, SESSION_COMPLETED; 27 other families re-run to CANDIDATE_ACCEPTED; the frontier's EXECUTION_STARTED, then SERVICE_UNIMPLEMENTED for CreateEventW (a named event) after 421,498 instructions.
  - *Observation 1 (order violation, of the record):* the frontier ran the play path while its criteria came from the batch trace. Disposition: resolved-here (the batch arguments; KIT_DIARY #44).
  - *Observation 2 (missing pair, of the reference):* the batch capture ran jit_capture.exe in the game exe's place, so its LoadIconA failure (1812) and its unmanifested version answers are the capture's, not the game's. Disposition: resolved-here (probes under the game's manifest and an icon judge; jit_capture linked with the manifest; KIT_DIARY #45).
  - *Observation 3 (vocabulary gap, checked):* none; every stop names its import.

### Sprint 053 (2026-10-11, closed partial: CI run 38166790366 success on 539c238c; this entry written at Sprint 054's close, 053 closed without one)
- **Scope:** structured exceptions: dispatch, RtlUnwindEx, faults dispatched, the Rtl* queries.
- **Rubber Duck Pass:**
  - *Sequence narration:* the exceptions probe on two runners after three failed builds and runs (link errors, STATUS_BAD_STACK, a fault in a __try body); the exceptions and vectored_handlers item runs to CANDIDATE_ACCEPTED; ten families re-run; the frontier's EXECUTION_STARTED, then SERVICE_UNIMPLEMENTED for VerSetConditionMask.
  - *Observation 1 (order violation, of the criterion):* the frontier criterion sat 20 unserved functions past the stop. Disposition: deferred to 054 (KIT_DIARY #43).

### Sprint 052 (2026-10-11, closed partial: CI run 38128014124 success on 6d522115)
- **Scope:** the com family, then (amended before the work) the network family's WSAStartup.
- **Rubber Duck Pass:**
  - *Sequence narration:* the com probe on two runners, then again with the notification rows; the com item run's SESSION_INIT, CANDIDATE_GENERATED, two COMPARISON_COMPLETED, CANDIDATE_ACCEPTED, SESSION_COMPLETED; the same for network; families re-run to CANDIDATE_ACCEPTED; the frontier's EXECUTION_STARTED, then SERVICE_UNIMPLEMENTED for WS2_32.dll!#115, then, with network served, for KERNEL32.dll!RaiseException.
  - *Observation 1 (order violation, of the card):* WSAStartup lay between the stop and the done criterion. Disposition: resolved-here (the card amended before the work, KIT_DIARY #40 applied).
  - *Observation 2 (vocabulary gap, checked):* none; the frontier's stop names the import.
  - *Observation 3 (missing pair):* the done criterion has no matching event; the sprint closes partial with it deferred to 053 (recorded).

### Sprint 051 (2026-10-11, closed: CI run 38126051832 success on d73e4ade)
- **Scope:** guest threads on one vCPU, from the work list's threads family.
- **Rubber Duck Pass:**
  - *Sequence narration:* the threads probe on two runners (PROBE_AUTHORED); the threads item run's SESSION_INIT, CANDIDATE_GENERATED, two COMPARISON_COMPLETED, CANDIDATE_ACCEPTED, SESSION_COMPLETED; 15 families re-run to CANDIDATE_ACCEPTED; the frontier's EXECUTION_STARTED, then SERVICE_UNIMPLEMENTED for ole32.dll!CoInitializeEx with two threads in the VM.
  - *Observation 1 (missing pair):* events.start in w2m held DllMain's attach without the TLS callback's before it. Disposition: resolved-here (the loader's start stub); KIT_DIARY #42.
  - *Observation 2 (order violation, of the test of every import's reach):* the walker did not reach the probe's ExitThread call. Disposition: resolved-here (address-taken roots), not a test exemption.
  - *Observation 3 (timing surprise):* time slices restarted the vCPU's interrupt timer, and the forward-rule test saw no stop. Disposition: resolved-here (slices only with two or more threads).

### Sprint 050 (2026-10-11, closed: CI run 38123808878 success on 3b709e65)
- **Scope:** set 050, the 64 classes the censuses list as missing that the guest profile reaches, in one case set.
- **Rubber Duck Pass:**
  - *Sequence narration:* per class, the class run's SESSION_INIT, CANDIDATE_GENERATED, the model's and the VM's COMPARISON_COMPLETED, CANDIDATE_ACCEPTED, SESSION_COMPLETED (64 classes); the frontier's EXECUTION_STARTED, then SERVICE_UNIMPLEMENTED naming KERNEL32.dll!CreateThread after 20,584,207 instructions.
  - *Observation 1 (payload anomaly):* the first judge found 18 integer classes unequal, among them SDM-undefined results the Xeon gives the same way in three runs. Disposition: resolved-here (fitted from the cases, the fit stated in integer_model.py); KIT_DIARY #41.
  - *Observation 2 (order violation, of the frozen-file rule):* a rule fix after the run changed set 050's case file; the judge's hash check stopped the comparison. Disposition: resolved-here (the builder keeps the old refusal for set 050).

### Sprint 049 (2026-10-11, closed: CI run 38121563657 success on d470850a)
- **Scope:** the work list from the game's own Windows run, loading DLLs at run time, and the families mono needs to start; widened mid-sprint by the user's AVX decision.
- **Rubber Duck Pass:**
  - *Sequence narration:* per family, PROBE_AUTHORED and the probe run on Server 2025 and 2022, then the item run's SESSION_INIT, CANDIDATE_GENERATED, two COMPARISON_COMPLETED, CANDIDATE_ACCEPTED, SESSION_COMPLETED; per frontier run, EXECUTION_STARTED then SERVICE_UNIMPLEMENTED naming the next unserved import, until the last run stopped on a lift refusal (EXECUTION_COMPLETED, faulted).
  - *Observation 1 (payload anomaly):* a family run whose candidate failed to build emitted ProducerFailed and RunFinalised stuck_quiescent and no SESSION_COMPLETED. Disposition: resolved-here (a build failure becomes a rejected comparison).
  - *Observation 2 (vocabulary gap, checked):* the final frontier stop is a lift refusal, which the frontier reports as EXECUTION_COMPLETED faulted with the refusal in `unobserved`; the vocabulary has no lift-refusal tag. Disposition: deferred to the Drift watchlist; the stop text names the class, and set 049c is planned from the census, not from the stop.
  - *Observation 3 (decision record):* BLACKBOARD carried the superseded v2 CPU entry as the last word for most of the sprint. Disposition: resolved-here (d470850a); KIT_DIARY #40.

### Sprint 048 (2026-10-10, closed: CI run 38099562363 success on 401deef)
- **Scope:** the guest's file namespace, then the families the frontier meets.
- **Rubber Duck Pass:**
  - *Sequence narration:* run the frontier, record the stopped call's arguments, read the caller's code, probe as the caller calls, contract, guest C, item run, plants, rerun every family when shared code moves.
  - *Observation 1 (a probe's own fault read as Windows's):* the temp-path probe's 64-name pass overflowed its buffer on a runner; a commit blamed GetEnvironmentStringsW first. Disposition: rows written as they end; KIT_DIARY #37.
  - *Observation 2 (a plant's reach mispredicted):* a row that prints a length cannot see a swap of two equal-length paths. Disposition: the list corrected with its reason; KIT_DIARY #38.
  - *Observation 3 (runner state is not a user's):* elevated runners, more than 64 variables. Disposition: compare what the caller asks; clear before setting.

### Sprint 047 (2026-10-10, closed: CI run 38088432347 success on 0f889e5)
- **Scope:** UnityMain's start-up, stop by stop.
- **Rubber Duck Pass:**
  - *Sequence narration:* look ahead, take each need in order, probe, contract, guest code, item run, plants, frontier.
  - *Observation 1 (tools that reported success the record did not hold):* class_run and family_run. Disposition: both read the record; KIT_DIARY #34, #35.
  - *Observation 2 (host resolution):* the Mac's float birth time. Disposition: exact from libc; KIT_DIARY #36.
  - *Observation 3 (OS versions):* Server 2025 and 2022 differ on two rows. Disposition: excluded; w2m answers as 2025.

### Sprint 046 (2026-10-10, closed: CI run 38084406263 success on dab6274)
- **Scope:** load UnityPlayer.dll, run its DllMain, enter UnityMain.
- **Rubber Duck Pass:**
  - *Sequence narration:* stop, stub-list what follows, probe on the runners, answer the rows, plant, accept, move.
  - *Observation 1 (SDK against mingw):* five runner rounds lost to header and link differences. Disposition: the gate defines `small` and `hyper`; KIT_DIARY #31.
  - *Observation 2 (results per CPU model):* RCPPS and RSQRTPS differ by vendor. Disposition: Intel's tables measured on two generations; KIT_DIARY #32.
  - *Observation 3 (two wrong reaches):* recorded on the card.
  - *Observation 4 (closed on red CI):* 40 CI runs failed on depot-reading tests while local gates passed. Disposition: tests moved to stored probe binaries; KIT_DIARY #33; close waits on green CI, which GitHub's billing block stops.
- **Closed:** clean.

### Sprint 044 (2026-10-10, closed)
- **Scope:** the launcher's C runtime start-up, family by family, until the stop is `UnityPlayer.dll!UnityMain`.
- **Rubber Duck Pass:**
  - *Sequence narration:* the frontier stops at a service; a scratch run with logged refusals shows the forms start-up uses; a no-CRT probe records Windows's answers on 4 to 16 runners; the rows become a contract; guest C answers them; planted faults with named reaches; the item run accepts on every runner binary; the frontier moves to the next stop.
  - *Observation 1 (three wrong reaches and a blind spot):* two plants stopped the run, one changed 4 rows for 2 named; HEAP_ZERO_MEMORY ignored on a zero arena changed nothing. Disposition: recorded on the card; the probe covers a dirty block.
  - *Observation 2 (silent counters and statuses):* ids above 63 went uncounted; argument-form stops read as faults. Disposition: resolved-here, each with a test that fails without the fix.
  - *Observation 3 (the model's file):* accepted, and different from the agent's wherever the rows are silent. Disposition: recorded; KIT_DIARY #29.
- **Closed:** clean.

### Sprint 043 (2026-10-10, closed)
- **Scope:** every differential as a causal topology; discrepancies resolved across runs.
- **Rubber Duck Pass:**
  - *Sequence narration:* per unit, candidate → comparison → decision; report → route; acceptance → resolution of what the ledger held open; the ledger folds the records.
  - *Observation 1 (the ledger is derived):* nothing writes it but the fold. Disposition: tested.
  - *Observation 2 (no unit field):* the unit rides in the first-divergence prefix. Disposition: recorded.
  - *Observation 3 (CI red underneath):* six red pushes found by the review. Disposition: resolved-here; KIT_DIARY #28.
- **Closed:** clean.

### Sprint 042 (2026-10-10, closed)
- **Scope:** one Windows function end to end: probe, contract from Windows's rows, file-system bridge, runtime implementation, judge; a cloud model measured inside it.
- **Rubber Duck Pass:**
  - *Sequence narration:* the runners printed 188 rows that became the contract; the item run's two comparisons ran each runner binary in the VM over file.c and the decision followed both; in the session, each model call caused one tool call and the seventh judge's item run accepted.
  - *Observation 1 (documentation as hypothesis):* POSIX delete, trailing-name stripping, error 131 came from the rows. Disposition: resolved-here; KIT_DIARY #27.
  - *Observation 2 (decided twice):* every 041 class record held two CANDIDATE_ACCEPTED. Disposition: resolved-here (`Once()`, counts tested); KIT_DIARY #26.
  - *Observation 3 (the model as a tester):* a missing service and a NUL in a path crashed the judge. Disposition: resolved-here, with tests.
- **Closed:** clean.

### Sprint 041 (2026-10-10, closed)
- **Scope:** the instruction class as the unit of work: rules per class, item runs, the VM's SSE, the next block by frequency.
- **Rubber Duck Pass:**
  - *Sequence narration:* the VM ran both case sets against Intel; each class's item run caused its two comparisons and its decision; the runner ran set 041 on four CPUs; both executors agreed with all of them after three fixes.
  - *Observation 1 (CPU over the SDM's wording):* CMPXCHG register destination unwritten on mismatch. Disposition: resolved-here; KIT_DIARY #25.
  - *Observation 2 (a helper's scratch register):* `_mark_defined` clobbered the shift count. Disposition: resolved-here; KIT_DIARY #24.
  - *Observation 3 (scope left for later):* model-drafted rules (042); LOCK atomicity across vCPUs (047). Disposition: recorded.
- **Closed:** clean.

### Sprint 040 (2026-10-09, closed)
- **Scope:** the guest state at a stop, in the VM, judged on Sprint 039's Windows fault records.
- **Rubber Duck Pass:**
  - *Sequence narration:* the comparator rebuilt 039's cases and checked their hash against the runner's, ran each compilable case in one VM, and read either the state at the next dispatch or the CONTEXT and exception record the guest runtime built after the EL1 vector saved the registers; both Intel CPUs conformed.
  - *Observation 1 (a plant outside the reference, again):* no Windows record can show a PUSH that moves RSP early. Disposition: resolved-here (POP [m] planted); KIT_DIARY #22.
  - *Observation 2 (reference inputs depended on the lifter):* lifting CPUID changed 039's case set. Disposition: resolved-here (`CLASSES_039`); KIT_DIARY #23.
  - *Observation 3 (a static check beat the CPU):* `commit_order` found POP [m]'s order defect from Sprint 029. Disposition: resolved-here.
- **Closed:** clean.

### Sprint 039 (2026-10-09, closed)
- **Scope:** the semantics oracle extended to XMM, MXCSR, AF, DF and faults; SSE floating-point semantics in the model; FPCR.AH in a guest.
- **Rubber Duck Pass:**
  - *Sequence narration:* the model gained exact SSE arithmetic and the probe gained full state; the runner built 39,708 cases from the seed and ran them three times per job; AMD runs taught three rules; two Intel runs then agreed on every case; a guest at EL1 ran FMIN, FMUL and FCVTZS under FPCR.AH 0 and 1.
  - *Observation 1 (the CPU teaches what the manual leaves vague):* Windows's #XM codes, flags at an unmasked fault, and PE under trapped overflow came from the oracle, one guess refuted. Disposition: rules cite their run; Sprint 041 accepts classes on Intel verdicts.
  - *Observation 2 (a plant outside the judge):* AF-undefined could not be caught. Disposition: KIT_DIARY #21; undefined-flag sets checked per class in 041.
  - *Observation 3 (the VM had no display):* the Windows VM was created with no display device and sat at the boot prompt with an empty disk. Disposition: virtio-ramfb added, keystrokes sent through UTM; Setup now writes its image.
- **Closed:** clean.

### Sprint 038 (2026-10-09, closed)
- **Scope:** the frontier as a topology; the game's Windows calls, threads and play-time code from the runner; acquisition, contract and probe tags on real runs.
- **Rubber Duck Pass:**
  - *Sequence narration:* the frontier ran the game's executable in the VM and recorded the service it lacked; the acquisition, contract and probe runs read the depots, contracts and binaries and emitted their lifecycle; on the runner the launcher counted every call through UnityPlayer's and Mono's imports and logged names resolved at run time and thread counts, in batch and play modes.
  - *Observation 1 (definitions over names):* BRIDGE_CALL_UNMAPPED, a regex over Steam's format, and a release used as a folder each misled once. Disposition: resolved-here; KIT_DIARY #20.
  - *Observation 2 (the hot path decides the design):* three functions take 68 million calls in 180 s. Disposition: Decision 2026-10-09; Sprint 040.
  - *Observation 3 (the runner can render):* play mode created a D3D11 device on Microsoft Basic Render Driver. Disposition: recorded; the local VM is the place for interactive play.
- **Closed:** clean.

### Sprint 037 (2026-10-09, closed)
- **Scope:** single instructions on the runner's CPU judge the lifter, sharing no code with it.
- **Rubber Duck Pass:**
  - *Sequence narration:* the generator wrote 20,116 cases with placed addresses; the runner loaded each state, jumped to the instruction and caught both exits; the judge stepped the model once from the same state and compared path, GPRs, defined flags and windows; two defects were fixed and the runs repeated on three CPU models.
  - *Observation 1 (agreement between two implementations is not evidence):* 16-bit PUSH was wrong in lift.py since Sprint 029, and lockstep and the state differential agreed because both execute lift.py. Disposition: resolved-here; KIT_DIARY #18.
  - *Observation 2 (the self-check found a defect before the CPU did):* placing addresses exposed that FS/GS overrides were dropped. Disposition: resolved-here (LiftError until segment bases exist).
  - *Observation 3 (the reference is a specific CPU):* Intel and AMD disagree on 0F 0D register forms. Disposition: Decision recorded; CPU name in every run.
- **Closed:** clean.

### Sprint 036 (2026-10-09, closed)
- **Scope:** the decoder for Hail to the Rainbow — validity rules and the game's rows; criterion (d).
- **Rubber Duck Pass:**
  - *Sequence narration:* two validity rules went in; a script read the SDM's opcode lines for every iclass the game needs and proposed rows; a merge replaced hand-written vector rows and kept general-purpose ones; the generated differential named each wrong row and the cause was fixed in the generator or the merge, not in the row; the game differential then found every used row confirmed.
  - *Observation 1 (the fallback hid missing rows):* a decoder that falls back to a lower prefix's row decodes bytes it has no row for. Disposition: resolved-here (strict prefix with per-opcode exceptions taken from XED).
  - *Observation 2 (a quiet merge was a lying merge):* "already present" counted rows with the same bytes and a different iclass; making that a conflict exposed four generator faults at once. Disposition: resolved-here.
  - *Observation 3 (a script must reproduce the session):* the committed rebuild first differed from the step-by-step spec by one row, because request order decided which row won a key. Disposition: resolved-here; the rebuild now equals it row for row.
- **Closed:** clean.
