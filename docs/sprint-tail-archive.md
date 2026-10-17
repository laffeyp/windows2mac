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
  - *Observation 3 (tail roll):* entries for Sprints 000-017 moved verbatim to `docs/sprint-tail-archive.md`; nothing deleted. Disposition: resolved-here.
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

### Sprint 101 (2026-10-17, closed)
- **Scope:** vocabulary 0.8 (P19), the VM compiler's counts as MEASUREMENT_RECORDED.
- **Rubber Duck Pass:**
  - *Sequence narration:* the batch run's runner emits 23 MEASUREMENT_RECORDED between its stop and EXECUTION_COMPLETED; read back through `read_record`, each equal to the printed count.
  - *Observation 1 (the vocabulary still holds names nothing emits):* 25 of 62 tags have no emit site (review 2026-10-17). Disposition: deferred to design sprint D1 (plan.md).
- **Closed:** CI pending at this entry.

### Sprint 100 (2026-10-17, closed)
- **Scope:** the open question of Sprint 089: a 0x67 address through R8D-R15D, 32 bits by the SDM.
- **Rubber Duck Pass:**
  - *Sequence narration:* no new tag; the address test, the equal-words judges, the 039 and 041 case sets, 456 class runs, 68 family runs, the batch run conforming.
  - *Observation 1 (one rule, four homes):* the rule lived in lift, symbolic_lift, the stencil table and the case generator's placement; each was found by its own failing judge, not by a search first. Disposition: resolved-here; the review of 2026-10-17 names the same spread (change amplification), plan.md D2.
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
